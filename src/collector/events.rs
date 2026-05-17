use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

pub struct Events;

#[derive(Serialize)]
struct EventsMarker {
    collection: &'static str,
    source: &'static str,
}

const FILES: &[&str] = &["/proc/sys/kernel/printk"];

const KMSG_PATH: &str = "/dev/kmsg";

impl Events {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        let mut probe = crate::collector::probe_files(roots, FILES, "all events files missing");
        let kmsg_ok = std::fs::File::open(KMSG_PATH).is_ok();
        if !kmsg_ok {
            if probe.available {
                probe.degraded.push(KMSG_PATH.into());
            } else {
                probe.available = true;
                probe.reason = None;
                probe.degraded.push(KMSG_PATH.into());
            }
        }
        probe
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

        let dmesg_raw = std::fs::read_to_string(probe.roots.resolve(KMSG_PATH)).ok();
        let dmesg_bytes: Vec<u8> = {
            let marker = EventsMarker {
                collection: "RT-20",
                source: "/dev/kmsg",
            };
            let mut prefix = serde_json::to_vec(&marker).unwrap_or_default();
            prefix.push(b'\n');
            if let Some(ref s) = dmesg_raw {
                prefix.extend_from_slice(s.as_bytes());
            }
            prefix
        };
        let dmesg_ok = dmesg_raw.is_some();

        let mut writer = match output.text_writer("dmesg.txt") {
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

        if let Err(e) = writer.write(&dmesg_bytes).await {
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

        let file_size = match writer.finish().await {
            Ok(s) => s,
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

        let mut degraded = probe.degraded.clone();
        if !dmesg_ok {
            degraded.push("dmesg: /dev/kmsg unreadable".into());
        }

        CollectionOutcome {
            status: if degraded.is_empty() {
                CollectionStatus::Ok
            } else {
                CollectionStatus::Degraded {
                    missing: degraded,
                }
            },
            duration: start.elapsed(),
            file_size,
            items_total: None,
            items_collected: if dmesg_ok { Some(1) } else { None },
            mem_total_kb: None,
            mem_available_kb: None,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
