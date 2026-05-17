use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Memory;

#[derive(Serialize)]
struct MeminfoEntry {
    key: String,
    value_kb: i64,
}

#[derive(Serialize)]
struct MemoryRecord {
    collection: &'static str,
    meminfo: Vec<MeminfoEntry>,
    pressure_memory: Option<String>,
    vmstat: Option<String>,
    zoneinfo: Option<String>,
}

impl Memory {
    pub async fn probe(&self) -> ProbeOutcome {
        let files = [
            "/proc/meminfo",
            "/proc/pressure/memory",
            "/proc/vmstat",
            "/proc/zoneinfo",
        ];
        let degraded: Vec<String> = files
            .iter()
            .filter(|f| !std::path::Path::new(f).exists())
            .map(|s| s.to_string())
            .collect();

        if degraded.len() == files.len() {
            ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("all memory files missing".into()),
            }
        } else if degraded.is_empty() {
            ProbeOutcome {
                available: true,
                degraded: Vec::new(),
                reason: None,
            }
        } else {
            ProbeOutcome {
                available: true,
                degraded,
                reason: None,
            }
        }
    }

    pub async fn collect(
        &self,
        output: &OutputDir,
        _probe: &ProbeOutcome,
    ) -> CollectionOutcome {
        let start = Instant::now();

        fn read_raw(path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }

        let mut entries: Vec<MeminfoEntry> = Vec::new();
        let mut mem_total_kb: Option<i64> = None;
        let mut mem_available_kb: Option<i64> = None;

        if let Some(raw) = read_raw("/proc/meminfo") {
            for line in raw.lines() {
                let Some((key, val)) = line.split_once(':') else {
                    continue;
                };
                let key = key.trim().to_string();
                let val_str = val.trim();
                let value_kb = val_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                if key == "MemTotal" {
                    mem_total_kb = Some(value_kb);
                }
                if key == "MemAvailable" {
                    mem_available_kb = Some(value_kb);
                }
                entries.push(MeminfoEntry { key, value_kb });
            }
        }
        entries.sort_by(|a, b| a.key.cmp(&b.key));

        let record = MemoryRecord {
            collection: "RT-06",
            meminfo: entries,
            pressure_memory: read_raw("/proc/pressure/memory"),
            vmstat: read_raw("/proc/vmstat"),
            zoneinfo: read_raw("/proc/zoneinfo"),
        };

        let outcome = |status: CollectionStatus| CollectionOutcome {
            status,
            duration: start.elapsed(),
            file_size: 0,
            items_total: None,
            items_collected: None,
            mem_total_kb,
            mem_available_kb,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        };

        let Ok(writer) = output.json_writer("memory.json") else {
            return outcome(CollectionStatus::Failed {
                reason: "json writer creation failed".into(),
            });
        };
        let Ok((size, _)) = writer.commit(&record).await else {
            return outcome(CollectionStatus::Failed {
                reason: "json write failed".into(),
            });
        };

        let mut o = outcome(CollectionStatus::Ok);
        o.file_size = size;
        o
    }
}
