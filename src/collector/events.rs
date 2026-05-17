use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Events;

#[derive(Serialize)]
struct DmesgRecord {
    collection: &'static str,
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_levels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_ratelimit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_ratelimit_burst: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_dropped: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    devkmsg_log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dmesg_restrict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dmesg_text: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/kernel/printk",
    "/proc/sys/kernel/printk_ratelimit",
    "/proc/sys/kernel/printk_ratelimit_burst",
    "/proc/sys/kernel/printk_dropped",
    "/proc/sys/kernel/devkmsg_log",
    "/proc/sys/kernel/dmesg_restrict",
];

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

        fn read_trimmed(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path))
                .ok()
                .map(|s| s.trim().to_string())
        }

        let dmesg_text = read_trimmed(&probe.roots, KMSG_PATH);
        let dmesg_ok = dmesg_text.is_some();

        let record = DmesgRecord {
            collection: "RT-20",
            source: "/dev/kmsg",
            printk_levels: read_trimmed(&probe.roots, FILES[0]),
            printk_ratelimit: read_trimmed(&probe.roots, FILES[1]),
            printk_ratelimit_burst: read_trimmed(&probe.roots, FILES[2]),
            printk_dropped: read_trimmed(&probe.roots, FILES[3]),
            devkmsg_log: read_trimmed(&probe.roots, FILES[4]),
            dmesg_restrict: read_trimmed(&probe.roots, FILES[5]),
            dmesg_text,
        };

        let writer = match output.json_writer("dmesg.json") {
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
            file_size: size,
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
