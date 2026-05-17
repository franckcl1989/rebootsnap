use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

pub struct Boot;

#[derive(Serialize)]
struct BootRecord {
    collection: &'static str,
    boot_id: Option<String>,
    uptime_seconds: Option<f64>,
    kernel_version: Option<String>,
    cmdline: Option<String>,
    hostname: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/kernel/random/boot_id",
    "/proc/uptime",
    "/proc/version",
    "/proc/cmdline",
    "/proc/sys/kernel/hostname",
];

impl Boot {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all boot identity files missing")
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
            std::fs::read_to_string(roots.resolve(path)).ok().map(|s| s.trim().to_string())
        }

        let boot_id = read_trimmed(&probe.roots, FILES[0]);
        let uptime = read_trimmed(&probe.roots, FILES[1])
            .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok());
        let kernel_version = read_trimmed(&probe.roots, FILES[2]);
        let cmdline = read_trimmed(&probe.roots, FILES[3]);
        let hostname = read_trimmed(&probe.roots, FILES[4]);

        let record = BootRecord {
            collection: "RT-01",
            boot_id: boot_id.clone(),
            uptime_seconds: uptime,
            kernel_version: kernel_version.clone(),
            cmdline,
            hostname: hostname.clone(),
        };

        let writer = match output.json_writer("boot.json") {
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
                    hostname,
                    kernel_version,
                    boot_id,
                    uptime_seconds: uptime.map(|u| u as u64),
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
                    hostname,
                    kernel_version,
                    boot_id,
                    uptime_seconds: uptime.map(|u| u as u64),
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
            hostname,
            kernel_version,
            boot_id,
            uptime_seconds: uptime.map(|u| u as u64),
        }
    }
}
