use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;

use crate::collector::{CollectResult, CollectStatus, Collector, ProbeResult, ProbeStatus};
use crate::output::OutputDir;

pub struct MemoryCollector;

impl MemoryCollector {
    pub fn new() -> Self {
        MemoryCollector
    }
}

#[derive(Serialize)]
struct MeminfoValue {
    key: String,
    value_kb: Option<i64>,
    value: Option<String>,
}

#[derive(Serialize)]
struct MemoryRecord {
    collection: String,
    meminfo: Vec<MeminfoValue>,
    pressure_memory: Option<String>,
    vmstat: Option<String>,
    zoneinfo: Option<String>,
}

#[async_trait]
impl Collector for MemoryCollector {
    fn id(&self) -> &'static str {
        "RT-06"
    }

    fn filename(&self) -> &'static str {
        "memory.json"
    }

    async fn probe(&self) -> ProbeResult {
        let files = [
            "/proc/meminfo",
            "/proc/pressure/memory",
            "/proc/vmstat",
            "/proc/zoneinfo",
        ];
        let mut degraded = Vec::new();
        for f in &files {
            if !std::path::Path::new(f).exists() {
                degraded.push(f.to_string());
            }
        }
        if degraded.len() == files.len() {
            ProbeResult {
                status: ProbeStatus::Unavailable("all memory files missing".to_string()),
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

        fn parse_meminfo(raw: &str) -> Vec<MeminfoValue> {
            raw.lines()
                .filter_map(|line| {
                    let mut parts = line.splitn(2, ':');
                    let key = parts.next()?.trim().to_string();
                    let val_str = parts.next()?.trim();
                    let value_kb = val_str
                        .split_whitespace()
                        .next()?
                        .parse::<i64>()
                        .ok();
                    Some(MeminfoValue {
                        key,
                        value_kb,
                        value: Some(val_str.to_string()),
                    })
                })
                .collect()
        }

        let meminfo_raw = read_raw("/proc/meminfo");
        let meminfo = meminfo_raw
            .as_deref()
            .map(parse_meminfo)
            .unwrap_or_default();
        let pressure_memory = read_raw("/proc/pressure/memory");
        let vmstat = read_raw("/proc/vmstat");
        let zoneinfo = read_raw("/proc/zoneinfo");

        let record = MemoryRecord {
            collection: "RT-06".to_string(),
            meminfo,
            pressure_memory,
            vmstat,
            zoneinfo,
        };

        match output.json_writer(MemoryCollector.filename()).await {
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
