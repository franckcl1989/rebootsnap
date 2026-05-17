use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Fd;

#[derive(Serialize)]
struct FdRecord {
    collection: &'static str,
    file_nr: Option<String>,
    file_max: Option<String>,
    inode_nr: Option<String>,
    inode_max: Option<String>,
    locks: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/fs/file-nr",
    "/proc/sys/fs/file-max",
    "/proc/sys/fs/inode-nr",
    "/proc/sys/fs/inode-max",
    "/proc/locks",
];

impl Fd {
    pub async fn probe(&self) -> ProbeOutcome {
        probe_files(FILES, "all fd files missing")
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

        let record = FdRecord {
            collection: "RT-07",
            file_nr: read_raw(FILES[0]),
            file_max: read_raw(FILES[1]),
            inode_nr: read_raw(FILES[2]),
            inode_max: read_raw(FILES[3]),
            locks: read_raw(FILES[4]),
        };

        let writer = match output.json_writer("fds.json") {
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
