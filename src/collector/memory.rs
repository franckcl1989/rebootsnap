use serde::Serialize;
use std::time::Instant;

use procfs::Current;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
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
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all memory files missing")
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

        fn read_raw(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path)).ok()
        }

        let pmi = procfs::Meminfo::current().ok();
        let mem_total_kb = pmi.as_ref().map(|m| (m.mem_total / 1024) as i64);
        let mem_available_kb = pmi
            .as_ref()
            .and_then(|m| m.mem_available.map(|v| (v / 1024) as i64));

        let mut entries: Vec<MeminfoEntry> = Vec::new();
        if let Some(raw) = read_raw(&probe.roots, FILES[0]) {
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
                entries.push(MeminfoEntry { key, value_kb });
            }
        }
        entries.sort_by(|a, b| a.key.cmp(&b.key));

        let record = MemoryRecord {
            collection: "RT-06",
            meminfo: entries,
            pressure_memory: read_raw(&probe.roots, FILES[1]),
            vmstat: read_raw(&probe.roots, FILES[2]),
            zoneinfo: read_raw(&probe.roots, FILES[3]),
        };

        let writer = match output.json_writer("memory.json") {
            Ok(w) => w,
            Err(e) => {
                return CollectionOutcome {
                    status: CollectionStatus::Failed {
                        reason: e.to_string(),
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
            }
        };
        let (size, _) = match writer.commit(&record).await {
            Ok(v) => v,
            Err(e) => {
                return CollectionOutcome {
                    status: CollectionStatus::Failed {
                        reason: e.to_string(),
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
            }
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
