use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Kernel;

#[derive(Serialize)]
struct KernelRecord {
    collection: &'static str,
    ostype: Option<String>,
    osrelease: Option<String>,
    modules: Option<String>,
    tainted: Option<String>,
    core_pattern: Option<String>,
    panic: Option<String>,
    printk: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/kernel/ostype",
    "/proc/sys/kernel/osrelease",
    "/proc/modules",
    "/proc/sys/kernel/tainted",
    "/proc/sys/kernel/core_pattern",
    "/proc/sys/kernel/panic",
    "/proc/sys/kernel/printk",
];

impl Kernel {
    pub async fn probe(&self) -> ProbeOutcome {
        probe_files(FILES, "all kernel files missing")
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

        let record = KernelRecord {
            collection: "RT-02",
            ostype: read_raw(FILES[0]),
            osrelease: read_raw(FILES[1]),
            modules: read_raw(FILES[2]),
            tainted: read_raw(FILES[3]),
            core_pattern: read_raw(FILES[4]),
            panic: read_raw(FILES[5]),
            printk: read_raw(FILES[6]),
        };

        let writer = match output.json_writer("kernel.json") {
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
