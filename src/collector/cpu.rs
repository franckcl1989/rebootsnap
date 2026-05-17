use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Cpu;

#[derive(Serialize)]
struct CpuRecord {
    collection: &'static str,
    stat: Option<String>,
    loadavg: Option<String>,
    pressure_cpu: Option<String>,
    interrupts: Option<String>,
    softirqs: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/stat",
    "/proc/loadavg",
    "/proc/pressure/cpu",
    "/proc/interrupts",
    "/proc/softirqs",
];

impl Cpu {
    pub async fn probe(&self) -> ProbeOutcome {
        probe_files(FILES, "all cpu files missing")
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

        let record = CpuRecord {
            collection: "RT-05",
            stat: read_raw(FILES[0]),
            loadavg: read_raw(FILES[1]),
            pressure_cpu: read_raw(FILES[2]),
            interrupts: read_raw(FILES[3]),
            softirqs: read_raw(FILES[4]),
        };

        let Ok(writer) = output.json_writer("cpu.json") else {
            return CollectionOutcome {
                status: CollectionStatus::Failed {
                    reason: "json writer creation failed".into(),
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
                mem_total_kb: None,
                mem_available_kb: None,
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
            mem_total_kb: None,
            mem_available_kb: None,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
