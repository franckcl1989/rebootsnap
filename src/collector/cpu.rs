use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;

use crate::collector::{CollectResult, CollectStatus, Collector, ProbeResult, ProbeStatus};
use crate::output::OutputDir;

pub struct CpuCollector;

impl CpuCollector {
    pub fn new() -> Self {
        CpuCollector
    }
}

#[derive(Serialize)]
struct CpuRecord {
    collection: String,
    stat: Option<String>,
    loadavg: Option<String>,
    pressure_cpu: Option<String>,
    interrupts: Option<String>,
    softirqs: Option<String>,
}

#[async_trait]
impl Collector for CpuCollector {
    fn id(&self) -> &'static str {
        "RT-05"
    }

    fn filename(&self) -> &'static str {
        "cpu.json"
    }

    async fn probe(&self) -> ProbeResult {
        let files = [
            "/proc/stat",
            "/proc/loadavg",
            "/proc/pressure/cpu",
            "/proc/interrupts",
            "/proc/softirqs",
        ];
        let mut degraded = Vec::new();
        for f in &files {
            if !std::path::Path::new(f).exists() {
                degraded.push(f.to_string());
            }
        }
        if degraded.len() == files.len() {
            ProbeResult {
                status: ProbeStatus::Unavailable("all cpu files missing".to_string()),
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

        fn read_raw(path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }

        let record = CpuRecord {
            collection: "RT-05".to_string(),
            stat: read_raw("/proc/stat"),
            loadavg: read_raw("/proc/loadavg"),
            pressure_cpu: read_raw("/proc/pressure/cpu"),
            interrupts: read_raw("/proc/interrupts"),
            softirqs: read_raw("/proc/softirqs"),
        };

        match output.json_writer(CpuCollector.filename()).await {
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
