use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Local;
use serde::Serialize;
use tempfile::NamedTempFile;

const SIZE_LIMIT: u64 = 64 * 1024 * 1024;
const ITEM_COUNT_LIMIT: u64 = 50_000;

pub struct OutputDir {
    pub root: PathBuf,
}

impl OutputDir {
    pub async fn create(base_dir: &Path) -> Result<Self, String> {
        let ts = Local::now().format("%Y%m%d-%H%M%S");
        let dirname = format!("rebootsnap-{}", ts);
        let root = base_dir.join(&dirname);
        fs::create_dir_all(&root)
            .map_err(|e| format!("cannot create output directory {}: {}", root.display(), e))?;
        tracing::info!("output directory: {}", root.display());
        Ok(OutputDir { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn directory_name(&self) -> Option<&str> {
        self.root.file_name()?.to_str()
    }

    pub fn json_writer(&self, filename: &str) -> Result<JsonWriter, String> {
        let tf = NamedTempFile::new_in(&self.root)
            .map_err(|e| format!("cannot create temp file: {}", e))?;
        Ok(JsonWriter {
            tf,
            final_path: self.root.join(filename),
        })
    }

    pub fn jsonl_writer(&self, filename: &str) -> Result<JsonlWriter, String> {
        let tf = NamedTempFile::new_in(&self.root)
            .map_err(|e| format!("cannot create temp file: {}", e))?;
        Ok(JsonlWriter {
            tf: Some(tf),
            byte_count: 0,
            item_count: 0,
            truncated: false,
            final_path: self.root.join(filename),
        })
    }

    #[allow(dead_code)]
    pub fn text_writer(&self, filename: &str) -> Result<TextWriter, String> {
        let tf = NamedTempFile::new_in(&self.root)
            .map_err(|e| format!("cannot create temp file: {}", e))?;
        Ok(TextWriter {
            tf,
            byte_count: 0,
            truncated: false,
            final_path: self.root.join(filename),
        })
    }

    pub fn cleanup_tmp(&self) {
        match fs::read_dir(&self.root) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "tmp") {
                        if let Err(e) = fs::remove_file(&path) {
                            tracing::warn!("cannot remove temp file {}: {}", path.display(), e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("cannot read output directory: {}", e);
            }
        }
    }
}

pub struct JsonWriter {
    tf: NamedTempFile,
    final_path: PathBuf,
}

impl JsonWriter {
    pub async fn commit<T: Serialize>(mut self, value: &T) -> Result<(u64, Option<u64>), String> {
        let json_bytes =
            serde_json::to_vec(value).map_err(|e| format!("json serialization failed: {}", e))?;

        if json_bytes.len() as u64 > SIZE_LIMIT {
            return Err("json output exceeds 64 MiB limit".to_string());
        }

        self.tf
            .write_all(&json_bytes)
            .map_err(|e| format!("cannot write temp file: {}", e))?;

        let size = json_bytes.len() as u64;
        self.tf
            .persist(&self.final_path)
            .map_err(|e| format!("cannot persist {}: {}", self.final_path.display(), e))?;

        Ok((size, None))
    }
}

pub struct JsonlWriter {
    tf: Option<NamedTempFile>,
    byte_count: u64,
    item_count: u64,
    truncated: bool,
    final_path: PathBuf,
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

        let tf = self.tf.as_mut().expect("JsonlWriter already finished");

        if self.byte_count + line_len > SIZE_LIMIT {
            if self.byte_count < SIZE_LIMIT {
                let marker = serde_json::json!({"truncated": true, "reason": "size_limit"});
                let mut marker_bytes = serde_json::to_vec(&marker)
                    .map_err(|e| format!("truncation marker failed: {}", e))?;
                marker_bytes.push(b'\n');
                if self.byte_count + marker_bytes.len() as u64 <= SIZE_LIMIT {
                    tf.write_all(&marker_bytes).map_err(|e| format!("write error: {}", e))?;
                }
            }
            self.truncated = true;
            return Ok(());
        }

        if self.item_count >= ITEM_COUNT_LIMIT {
            if self.byte_count < SIZE_LIMIT {
                let marker = serde_json::json!({"truncated": true, "reason": "item_count_limit"});
                let mut marker_bytes = serde_json::to_vec(&marker)
                    .map_err(|e| format!("truncation marker failed: {}", e))?;
                marker_bytes.push(b'\n');
                if self.byte_count + marker_bytes.len() as u64 <= SIZE_LIMIT {
                    tf.write_all(&marker_bytes).map_err(|e| format!("write error: {}", e))?;
                }
            }
            self.truncated = true;
            return Ok(());
        }

        tf.write_all(&line_bytes)
            .map_err(|e| format!("write error: {}", e))?;

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

    pub async fn finish(mut self) -> (u64, u64) {
        if self.byte_count > 0 {
            if let Some(tf) = self.tf.take() {
                if let Err(e) = tf.persist(&self.final_path) {
                    tracing::error!("cannot persist {}: {}", self.final_path.display(), e);
                }
            }
        }
        (self.byte_count, self.item_count)
    }
}

#[allow(dead_code)]
pub struct TextWriter {
    tf: NamedTempFile,
    byte_count: u64,
    truncated: bool,
    final_path: PathBuf,
}

#[allow(dead_code)]
impl TextWriter {
    pub async fn write(&mut self, data: &[u8]) -> Result<(), String> {
        if self.truncated {
            return Ok(());
        }
        let data_len = data.len() as u64;

        if self.byte_count + data_len > SIZE_LIMIT {
            let remaining = (SIZE_LIMIT - self.byte_count) as usize;
            if remaining > 0 {
                self.tf
                    .write_all(&data[..remaining.min(data.len())])
                    .map_err(|e| format!("write error: {}", e))?;
                self.byte_count = SIZE_LIMIT;
            }
            self.truncated = true;
            return Ok(());
        }

        self.tf
            .write_all(data)
            .map_err(|e| format!("write error: {}", e))?;
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
        if self.byte_count > 0 {
            self.tf
                .persist(&self.final_path)
                .map_err(|e| format!("cannot persist {}: {}", self.final_path.display(), e))?;
        }
        Ok(self.byte_count)
    }
}
