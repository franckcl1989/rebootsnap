use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Tmpfs;

#[derive(Serialize)]
struct TmpfsRecord {
    collection: &'static str,
    mounts: Option<String>,
    run_entries: Option<u64>,
    dev_shm_entries: Option<u64>,
    tmp_entries: Option<u64>,
}

const MOUNTS_FILE: &str = "/proc/mounts";
const RUNTIME_DIRS: &[&str] = &["/run", "/dev/shm", "/tmp"];

fn count_dir(path: &str) -> Option<u64> {
    let entries = std::fs::read_dir(path).ok()?;
    Some(entries.flatten().count() as u64)
}

impl Tmpfs {
    pub async fn probe(&self) -> ProbeOutcome {
        let mounts_exists = std::path::Path::new(MOUNTS_FILE).exists();
        if !mounts_exists {
            return ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("all tmpfs files missing".into()),
            };
        }
        let degraded: Vec<String> = RUNTIME_DIRS
            .iter()
            .filter(|d| !std::path::Path::new(d).exists())
            .map(|s| s.to_string())
            .collect();
        if degraded.len() == RUNTIME_DIRS.len() {
            ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("all tmpfs runtime dirs missing".into()),
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

        let record = TmpfsRecord {
            collection: "RT-08",
            mounts: read_raw(MOUNTS_FILE),
            run_entries: count_dir(RUNTIME_DIRS[0]),
            dev_shm_entries: count_dir(RUNTIME_DIRS[1]),
            tmp_entries: count_dir(RUNTIME_DIRS[2]),
        };

        let writer = match output.json_writer("tmpfs.json") {
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
                    mem_total_kb: None,
                    mem_available_kb: None,
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
                    mem_total_kb: None,
                    mem_available_kb: None,
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
            mem_total_kb: None,
            mem_available_kb: None,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
