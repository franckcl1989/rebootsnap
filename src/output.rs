use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;
use serde::Serialize;

const SIZE_LIMIT: u64 = 64 * 1024 * 1024;
const ITEM_COUNT_LIMIT: u64 = 50_000;

pub struct OutputDir {
    pub root: PathBuf,
}

impl OutputDir {
    pub async fn create(base_dir: &Path) -> Result<Self, String> {
        let ts = Local::now().format("%Y%m%d-%H%M%S");
        let dirname = format!("rebootsnap-{}", ts);
        let root = base_dir.join(dirname);
        fs::create_dir_all(&root)
            .map_err(|e| format!("cannot create output directory {}: {}", root.display(), e))?;
        Ok(OutputDir { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn json_writer(&self, filename: &str) -> Result<JsonWriter, String> {
        let tmp_path = self.root.join(format!("{}.tmp", filename));
        let final_path = self.root.join(filename);
        Ok(JsonWriter {
            tmp_path,
            final_path,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
        })
    }

    pub async fn jsonl_writer(&self, filename: &str) -> Result<JsonlWriter, String> {
        let tmp_path = self.root.join(format!("{}.tmp", filename));
        let final_path = self.root.join(filename);
        Ok(JsonlWriter {
            tmp_path,
            final_path,
            byte_count: 0,
            item_count: 0,
            truncated: false,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
        })
    }

    pub async fn text_writer(&self, filename: &str) -> Result<TextWriter, String> {
        let tmp_path = self.root.join(format!("{}.tmp", filename));
        let final_path = self.root.join(filename);
        Ok(TextWriter {
            tmp_path,
            final_path,
            byte_count: 0,
            truncated: false,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
        })
    }

    pub fn cleanup_tmp(&self) {
        if let Ok(dir) = fs::read_dir(&self.root) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "tmp") {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    pub fn directory_name(&self) -> Option<&str> {
        self.root.file_name()?.to_str()
    }
}

pub struct JsonWriter {
    tmp_path: PathBuf,
    final_path: PathBuf,
    started_at: std::time::Duration,
}

impl JsonWriter {
    pub async fn commit<T: Serialize>(self, value: &T) -> Result<(u64, Option<u64>), String> {
        let json_bytes =
            serde_json::to_vec(value).map_err(|e| format!("json serialization failed: {}", e))?;

        if json_bytes.len() as u64 > SIZE_LIMIT {
            return Err("json output exceeds 64 MiB limit".to_string());
        }

        let len = json_bytes.len() as u64;
        fs::write(&self.tmp_path, &json_bytes)
            .map_err(|e| format!("cannot write temp file {}: {}", self.tmp_path.display(), e))?;

        fs::rename(&self.tmp_path, &self.final_path)
            .map_err(|e| format!("cannot rename {} to {}: {}", self.tmp_path.display(), self.final_path.display(), e))?;

        Ok((len, None))
    }
}

pub struct JsonlWriter {
    tmp_path: PathBuf,
    final_path: PathBuf,
    byte_count: u64,
    item_count: u64,
    truncated: bool,
    started_at: std::time::Duration,
}

impl JsonlWriter {
    pub async fn write_line<T: Serialize>(&mut self, value: &T) -> Result<(), String> {
        if self.truncated {
            return Ok(());
        }

        let mut line_bytes =
            serde_json::to_vec(value).map_err(|e| format!("jsonl serialization failed: {}", e))?;
        line_bytes.push(b'\n');

        let line_len = line_bytes.len() as u64;

        if self.byte_count + line_len > SIZE_LIMIT {
            let marker = serde_json::json!({"truncated": true, "reason": "size_limit"});
            let marker_bytes = serde_json::to_vec(&marker)
                .map_err(|e| format!("truncation marker serialization failed: {}", e))?;
            let mut marker_line = marker_bytes.clone();
            marker_line.push(b'\n');

            if self.byte_count + marker_line.len() as u64 <= SIZE_LIMIT {
                if let Ok(mut f) = fs::OpenOptions::new().append(true).open(&self.tmp_path) {
                    let _ = f.write_all(&marker_line);
                }
            }
            self.truncated = true;
            return Ok(());
        }

        if self.item_count >= ITEM_COUNT_LIMIT {
            let marker = serde_json::json!({"truncated": true, "reason": "item_count_limit"});
            let marker_bytes = serde_json::to_vec(&marker)
                .map_err(|e| format!("truncation marker serialization failed: {}", e))?;
            let mut marker_line = marker_bytes.clone();
            marker_line.push(b'\n');

            if self.byte_count + marker_line.len() as u64 <= SIZE_LIMIT {
                if let Ok(mut f) = fs::OpenOptions::new().append(true).open(&self.tmp_path) {
                    let _ = f.write_all(&marker_line);
                }
            }
            self.truncated = true;
            return Ok(());
        }

        if !self.tmp_path.exists() {
            fs::write(&self.tmp_path, &line_bytes)
                .map_err(|e| format!("cannot write temp file {}: {}", self.tmp_path.display(), e))?;
        } else {
            let mut f = fs::OpenOptions::new()
                .append(true)
                .open(&self.tmp_path)
                .map_err(|e| {
                    format!("cannot open temp file {}: {}", self.tmp_path.display(), e)
                })?;
            f.write_all(&line_bytes)
                .map_err(|e| format!("cannot append to {}: {}", self.tmp_path.display(), e))?;
        }

        self.byte_count += line_len;
        self.item_count += 1;
        Ok(())
    }

    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn byte_count(&self) -> u64 {
        self.byte_count
    }

    pub fn item_count(&self) -> u64 {
        self.item_count
    }

    pub async fn finish(self) -> Result<(u64, u64), String> {
        if self.tmp_path.exists() {
            fs::rename(&self.tmp_path, &self.final_path)
                .map_err(|e| {
                    format!(
                        "cannot rename {} to {}: {}",
                        self.tmp_path.display(),
                        self.final_path.display(),
                        e
                    )
                })?;
        }
        Ok((self.byte_count, self.item_count))
    }
}

pub struct TextWriter {
    tmp_path: PathBuf,
    final_path: PathBuf,
    byte_count: u64,
    truncated: bool,
    started_at: std::time::Duration,
}

impl TextWriter {
    pub async fn write(&mut self, data: &[u8]) -> Result<(), String> {
        if self.truncated {
            return Ok(());
        }

        let data_len = data.len() as u64;

        if self.byte_count + data_len > SIZE_LIMIT {
            let remaining = (SIZE_LIMIT - self.byte_count) as usize;
            if remaining > 0 {
                let truncated_data = &data[..remaining.min(data.len())];
                if !self.tmp_path.exists() {
                    fs::write(&self.tmp_path, truncated_data)
                        .map_err(|e| format!("cannot write temp file {}: {}", self.tmp_path.display(), e))?;
                } else {
                    let mut f = fs::OpenOptions::new()
                        .append(true)
                        .open(&self.tmp_path)
                        .map_err(|e| {
                            format!("cannot open temp file {}: {}", self.tmp_path.display(), e)
                        })?;
                    f.write_all(truncated_data)
                        .map_err(|e| format!("cannot append to {}: {}", self.tmp_path.display(), e))?;
                }
                self.byte_count = SIZE_LIMIT;
            }
            self.truncated = true;
            return Ok(());
        }

        if !self.tmp_path.exists() {
            fs::write(&self.tmp_path, data)
                .map_err(|e| format!("cannot write temp file {}: {}", self.tmp_path.display(), e))?;
        } else {
            let mut f = fs::OpenOptions::new()
                .append(true)
                .open(&self.tmp_path)
                .map_err(|e| {
                    format!("cannot open temp file {}: {}", self.tmp_path.display(), e)
                })?;
            f.write_all(data)
                .map_err(|e| format!("cannot append to {}: {}", self.tmp_path.display(), e))?;
        }

        self.byte_count += data_len;
        Ok(())
    }

    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn byte_count(&self) -> u64 {
        self.byte_count
    }

    pub async fn finish(self) -> Result<u64, String> {
        if self.tmp_path.exists() {
            fs::rename(&self.tmp_path, &self.final_path)
                .map_err(|e| {
                    format!(
                        "cannot rename {} to {}: {}",
                        self.tmp_path.display(),
                        self.final_path.display(),
                        e
                    )
                })?;
        }
        Ok(self.byte_count)
    }
}
