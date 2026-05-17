use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
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

impl Boot {
    pub async fn probe(&self) -> ProbeOutcome {
        let files = [
            "/proc/sys/kernel/random/boot_id",
            "/proc/uptime",
            "/proc/version",
            "/proc/cmdline",
            "/proc/sys/kernel/hostname",
        ];
        let degraded: Vec<String> = files
            .iter()
            .filter(|f| std::path::Path::new(f).exists() && std::fs::read_to_string(f).is_err())
            .map(|s| s.to_string())
            .collect();

        if degraded.len() == files.len() {
            ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("all boot identity files unreadable".into()),
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

        fn read_trimmed(path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
        }

        let boot_id = read_trimmed("/proc/sys/kernel/random/boot_id");
        let uptime = read_trimmed("/proc/uptime")
            .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok());
        let kernel_version = read_trimmed("/proc/version");
        let cmdline = read_trimmed("/proc/cmdline");
        let hostname = read_trimmed("/proc/sys/kernel/hostname");

        let record = BootRecord {
            collection: "RT-01",
            boot_id: boot_id.clone(),
            uptime_seconds: uptime,
            kernel_version: kernel_version.clone(),
            cmdline,
            hostname: hostname.clone(),
        };

        let outcome = |status: CollectionStatus| CollectionOutcome {
            status,
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

        let Ok(writer) = output.json_writer("boot.json") else {
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
