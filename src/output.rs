use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use chrono::Local;
use serde::Serialize;
use tempfile::NamedTempFile;

use crate::error::OutputError;

pub const SIZE_LIMIT: u64 = 64 * 1024 * 1024;
const ITEM_COUNT_LIMIT: u64 = 50_000;

pub struct OutputDir {
    pub root: PathBuf,
}

impl OutputDir {
    pub async fn create(base_dir: &Path) -> Result<Self, OutputError> {
        let ts = Local::now().format("%Y%m%d-%H%M%S");
        let dirname = format!("rebootsnap-{}", ts);
        let root = base_dir.join(&dirname);
        std::fs::create_dir_all(&root).map_err(OutputError::Io)?;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).ok();
        tracing::info!("output directory: {}", root.display());
        Ok(OutputDir { root })
    }

    pub fn from_existing(root: PathBuf) -> Self {
        OutputDir { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn directory_name(&self) -> Option<&str> {
        self.root.file_name()?.to_str()
    }

    pub fn json_writer(&self, filename: &str) -> Result<JsonWriter, OutputError> {
        let tf = NamedTempFile::new_in(&self.root).map_err(OutputError::Io)?;
        Ok(JsonWriter {
            tf,
            final_path: self.root.join(filename),
        })
    }

    pub fn jsonl_writer(&self, filename: &str) -> Result<JsonlWriter, OutputError> {
        let tf = NamedTempFile::new_in(&self.root).map_err(OutputError::Io)?;
        Ok(JsonlWriter {
            tf: Some(tf),
            byte_count: 0,
            item_count: 0,
            truncated: false,
            final_path: self.root.join(filename),
        })
    }

    pub fn cleanup_tmp(&self) {
        match std::fs::read_dir(&self.root) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_temp = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(".tmp"));
                    if is_temp
                        && let Err(e) = std::fs::remove_file(&path)
                    {
                        tracing::warn!("cannot remove temp file {}: {}", path.display(), e);
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
    pub async fn commit<T: Serialize>(mut self, value: &T) -> Result<(u64, Option<u64>), OutputError> {
        let json_bytes = serde_json::to_vec(value).map_err(OutputError::Serialize)?;

        if json_bytes.len() as u64 > SIZE_LIMIT {
            return Err(OutputError::SizeLimit);
        }

        self.tf.write_all(&json_bytes).map_err(OutputError::Io)?;

        let size = json_bytes.len() as u64;
        self.tf
            .persist(&self.final_path)
            .map_err(|e| OutputError::Io(e.error))?;
        std::fs::set_permissions(&self.final_path, std::fs::Permissions::from_mode(0o600)).ok();

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
    pub async fn write_line<T: Serialize>(&mut self, value: &T) -> Result<(), OutputError> {
        if self.truncated {
            return Ok(());
        }

        let mut line_bytes = serde_json::to_vec(value).map_err(OutputError::Serialize)?;
        line_bytes.push(b'\n');
        let line_len = line_bytes.len() as u64;

        let tf = match self.tf.as_mut() {
            Some(t) => t,
            None => {
                return Err(OutputError::Io(std::io::Error::other(
                    "JsonlWriter already finished",
                )));
            }
        };

        if self.byte_count + line_len > SIZE_LIMIT {
            if self.byte_count < SIZE_LIMIT {
                let marker = serde_json::json!({"truncated": true, "reason": "size_limit"});
                let mut marker_bytes = serde_json::to_vec(&marker)
                    .map_err(OutputError::Serialize)?;
                marker_bytes.push(b'\n');
                if self.byte_count + marker_bytes.len() as u64 <= SIZE_LIMIT {
                    tf.write_all(&marker_bytes).map_err(OutputError::Io)?;
                }
            }
            self.truncated = true;
            return Ok(());
        }

        if self.item_count >= ITEM_COUNT_LIMIT {
            if self.byte_count < SIZE_LIMIT {
                let marker = serde_json::json!({"truncated": true, "reason": "item_count_limit"});
                let mut marker_bytes = serde_json::to_vec(&marker)
                    .map_err(OutputError::Serialize)?;
                marker_bytes.push(b'\n');
                if self.byte_count + marker_bytes.len() as u64 <= SIZE_LIMIT {
                    tf.write_all(&marker_bytes).map_err(OutputError::Io)?;
                }
            }
            self.truncated = true;
            return Ok(());
        }

        tf.write_all(&line_bytes).map_err(OutputError::Io)?;

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
        if self.byte_count > 0
            && let Some(tf) = self.tf.take()
        {
            if let Err(e) = tf.persist(&self.final_path) {
                tracing::error!("cannot persist {}: {}", self.final_path.display(), e);
            } else {
                std::fs::set_permissions(&self.final_path, std::fs::Permissions::from_mode(0o600)).ok();
            }
        }
        (self.byte_count, self.item_count)
    }
}
