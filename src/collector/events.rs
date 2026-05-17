use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Events;

#[derive(Serialize)]
struct EventsRecord {
    collection: &'static str,
    dmesg: Option<String>,
    printk: Option<String>,
}

const FILES: &[&str] = &["/proc/sys/kernel/printk"];

const DMESG_MAX: usize = 128 * 1024;

impl Events {
    pub async fn probe(&self) -> ProbeOutcome {
        probe_files(FILES, "all events files missing")
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

        let raw_dmesg = read_raw("/dev/kmsg");
        let dmesg = raw_dmesg.map(|s| {
            if s.len() > DMESG_MAX {
                let truncated = &s[..DMESG_MAX];
                format!(
                    "[RebootSnap: dmesg truncated from {} to {} bytes]\n{}",
                    s.len(),
                    DMESG_MAX,
                    truncated
                )
            } else {
                s
            }
        });

        let record = EventsRecord {
            collection: "RT-20",
            dmesg,
            printk: read_raw(FILES[0]),
        };

        let writer = match output.json_writer("events.json") {
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
