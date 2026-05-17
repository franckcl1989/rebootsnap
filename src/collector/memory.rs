use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
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

const FILES: &[&str] = &[
    "/proc/meminfo",
    "/proc/pressure/memory",
    "/proc/vmstat",
    "/proc/zoneinfo",
];

impl Memory {
    pub async fn probe(&self) -> ProbeOutcome {
        probe_files(FILES, "all memory files missing")
    }

    pub async fn collect(
        &self,
        output: &OutputDir,
        probe: &ProbeOutcome,
    ) -> CollectionOutcome {
        if !probe.available {
            return CollectionOutcome {
                status: CollectionStatus::Failed {
                    reason: probe.reason.clone().unwrap_or_default(),
                },
                duration: Default::default(),
                file_size: 0,
                items_total: None,
                items_collected: None,
                mem_total_kb: None,
                mem_available_kb: None,
                hostname: None,
                kernel_version: None,
                boot_id: None,
                uptime_seconds: None,
            };
        }
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

        let Ok(writer) = output.json_writer("memory.json") else {
            return CollectionOutcome {
                status: CollectionStatus::Failed {
                    reason: "json writer creation failed".into(),
                },
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
        };
        let Ok((size, _)) = writer.commit(&record).await else {
            return CollectionOutcome {
                status: CollectionStatus::Failed {
                    reason: "json write failed".into(),
                },
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
        };

        CollectionOutcome {
            status: if probe.degraded.is_empty() {
                CollectionStatus::Ok
            } else {
                CollectionStatus::Degraded {
                    missing: probe.degraded.clone(),
                }
            },
            duration: start.elapsed(),
            file_size: size,
            items_total: None,
            items_collected: None,
            mem_total_kb,
            mem_available_kb,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
