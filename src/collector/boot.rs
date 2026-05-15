use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;

use crate::collector::{CollectResult, CollectStatus, Collector, ProbeResult, ProbeStatus};
use crate::output::OutputDir;

pub struct BootCollector;

impl BootCollector {
    pub fn new() -> Self {
        BootCollector
    }
}

#[derive(Serialize)]
struct BootRecord {
    collection: String,
    boot_id: Option<String>,
    uptime_seconds: Option<f64>,
    kernel_version: Option<String>,
    cmdline: Option<String>,
    hostname: Option<String>,
}

#[async_trait]
impl Collector for BootCollector {
    fn id(&self) -> &'static str {
        "RT-01"
    }

    fn filename(&self) -> &'static str {
        "boot.json"
    }

    async fn probe(&self) -> ProbeResult {
        let files = [
            "/proc/sys/kernel/random/boot_id",
            "/proc/uptime",
            "/proc/version",
            "/proc/cmdline",
            "/proc/sys/kernel/hostname",
        ];
        let mut degraded = Vec::new();
        for f in &files {
            if std::path::Path::new(f).exists() {
                if std::fs::read_to_string(f).is_err() {
                    degraded.push(f.to_string());
                }
            }
        }
        if degraded.len() == files.len() {
            ProbeResult {
                status: ProbeStatus::Unavailable("all boot identity files unreadable".to_string()),
                detail: HashMap::new(),
            }
        } else if degraded.is_empty() {
            ProbeResult {
                status: ProbeStatus::Available,
                detail: HashMap::new(),
            }
        } else {
            ProbeResult {
                status: ProbeStatus::Degraded(degraded),
                detail: HashMap::new(),
            }
        }
    }

    async fn collect(
        &self,
        output: &OutputDir,
        _probe: &ProbeResult,
    ) -> CollectResult {
        let start = tokio::time::Instant::now();

        fn read_trimmed(path: &str) -> Option<String> {
            std::fs::read_to_string(path)
                .ok()
                .map(|s| s.trim().to_string())
        }

        let boot_id = read_trimmed("/proc/sys/kernel/random/boot_id");
        let uptime = read_trimmed("/proc/uptime").and_then(|s| {
            s.split_whitespace()
                .next()?
                .parse::<f64>()
                .ok()
        });
        let kernel_version = read_trimmed("/proc/version");
        let cmdline = read_trimmed("/proc/cmdline");
        let hostname = read_trimmed("/proc/sys/kernel/hostname");

        let record = BootRecord {
            collection: "RT-01".to_string(),
            boot_id,
            uptime_seconds: uptime,
            kernel_version,
            cmdline,
            hostname,
        };

        match output.json_writer(BootCollector.filename()).await {
            Ok(writer) => match writer.commit(&record).await {
                Ok((size, _)) => {
                    let duration = start.elapsed();
                    CollectResult {
                        status: CollectStatus::Ok,
                        duration,
                        file_size: size,
                        items_total: None,
                        items_collected: None,
                        error_reason: None,
                    }
                }
                Err(e) => CollectResult {
                    status: CollectStatus::Failed(e),
                    duration: start.elapsed(),
                    file_size: 0,
                    items_total: None,
                    items_collected: None,
                    error_reason: None,
                },
            },
            Err(e) => CollectResult {
                status: CollectStatus::Failed(e),
                duration: start.elapsed(),
                file_size: 0,
                items_total: None,
                items_collected: None,
                error_reason: None,
            },
        }
    }
}
