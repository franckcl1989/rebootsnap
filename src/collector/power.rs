use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Power;

#[derive(Serialize)]
struct PowerRecord {
    collection: &'static str,
    state: Option<String>,
    disk: Option<String>,
}

const FILES: &[&str] = &["/sys/power/state", "/sys/power/disk"];

impl Power {
    pub async fn probe(&self) -> ProbeOutcome {
        probe_files(FILES, "all power files missing")
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

        let record = PowerRecord {
            collection: "RT-18",
            state: read_raw(FILES[0]),
            disk: read_raw(FILES[1]),
        };

        let writer = match output.json_writer("power.json") {
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
