use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
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

impl Cpu {
    pub async fn probe(&self) -> ProbeOutcome {
        let files = [
            "/proc/stat",
            "/proc/loadavg",
            "/proc/pressure/cpu",
            "/proc/interrupts",
            "/proc/softirqs",
        ];
        let degraded: Vec<String> = files
            .iter()
            .filter(|f| !std::path::Path::new(f).exists())
            .map(|s| s.to_string())
            .collect();

        if degraded.len() == files.len() {
            ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("all cpu files missing".into()),
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
        _probe: &ProbeOutcome,
    ) -> CollectionOutcome {
        let start = Instant::now();

        fn read_raw(path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }

        let record = CpuRecord {
            collection: "RT-05",
            stat: read_raw("/proc/stat"),
            loadavg: read_raw("/proc/loadavg"),
            pressure_cpu: read_raw("/proc/pressure/cpu"),
            interrupts: read_raw("/proc/interrupts"),
            softirqs: read_raw("/proc/softirqs"),
        };

        let outcome = |status: CollectionStatus| CollectionOutcome {
            status,
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

        let Ok(writer) = output.json_writer("cpu.json") else {
            return outcome(CollectionStatus::Failed {
                reason: "json writer creation failed".into(),
            });
        };
        let Ok((size, _)) = writer.commit(&record).await else {
            return outcome(CollectionStatus::Failed {
                reason: "json write failed".into(),
            });
        };

        let mut o = outcome(CollectionStatus::Ok);
        o.file_size = size;
        o
    }
}
