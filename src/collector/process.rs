use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;

use crate::collector::{CollectResult, CollectStatus, Collector, ProbeResult, ProbeStatus};
use crate::output::OutputDir;

pub struct ProcessCollector;

impl ProcessCollector {
    pub fn new() -> Self {
        ProcessCollector
    }
}

#[derive(Serialize)]
struct ProcessRecord {
    collection: String,
    pid: i32,
    ppid: Option<i32>,
    name: String,
    state: Option<String>,
    uid: Option<u32>,
    threads: Option<i64>,
    cmdline: Option<String>,
}

#[async_trait]
impl Collector for ProcessCollector {
    fn id(&self) -> &'static str {
        "RT-04"
    }

    fn filename(&self) -> &'static str {
        "processes.jsonl"
    }

    fn item_timeout(&self) -> Duration {
        Duration::from_secs(10)
    }

    async fn probe(&self) -> ProbeResult {
        let mut detail = HashMap::new();

        if std::path::Path::new("/proc/1/status").exists() {
            if let Ok(p) = std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
                detail.insert("perf_event_paranoid".to_string(), p.trim().to_string());
            }
            ProbeResult {
                status: ProbeStatus::Available,
                detail,
            }
        } else {
            ProbeResult {
                status: ProbeStatus::Unavailable("/proc not accessible".to_string()),
                detail,
            }
        }
    }

    async fn collect(
        &self,
        output: &OutputDir,
        _probe: &ProbeResult,
    ) -> CollectResult {
        let start = tokio::time::Instant::now();

        let mut writer = match output.jsonl_writer(ProcessCollector.filename()).await {
            Ok(w) => w,
            Err(e) => {
                return CollectResult {
                    status: CollectStatus::Failed(e),
                    duration: start.elapsed(),
                    file_size: 0,
                    items_total: None,
                    items_collected: None,
                    error_reason: None,
                };
            }
        };

        let mut total: u64 = 0;
        let entries = match procfs::process::all_processes() {
            Ok(entries) => entries,
            Err(e) => {
                return CollectResult {
                    status: CollectStatus::Failed(format!("cannot enumerate processes: {}", e)),
                    duration: start.elapsed(),
                    file_size: 0,
                    items_total: None,
                    items_collected: None,
                    error_reason: None,
                };
            }
        };

        for entry in entries {
            total += 1;

            let proc_entry = match entry {
                Ok(p) => Some(p),
                Err(_) => None,
            };
            let pid = proc_entry.as_ref().map(|p| p.pid).unwrap_or(-1);
            let stat = proc_entry.as_ref().and_then(|p| p.stat().ok());
            let status = proc_entry.as_ref().and_then(|p| p.status().ok());
            let ppid = stat.as_ref().map(|s| s.ppid);
            let name = stat
                .as_ref()
                .map(|s| s.comm.clone())
                .unwrap_or_else(|| "?".to_string());
            let state = stat.as_ref().map(|s| format!("{}", s.state));
            let threads = stat.as_ref().map(|s| s.num_threads);
            let uid = status.as_ref().map(|s| s.euid as u32);

            let cmdline = proc_entry
                .as_ref()
                .and_then(|p| p.cmdline().ok())
                .map(|c| c.join(" "))
                .and_then(|s| if s.is_empty() { None } else { Some(s) });

            let record = ProcessRecord {
                collection: "RT-04".to_string(),
                pid,
                ppid,
                name,
                state,
                uid,
                threads,
                cmdline,
            };

            if writer.write_line(&record).await.is_err() {
                return CollectResult {
                    status: CollectStatus::Failed("jsonl write failed".to_string()),
                    duration: start.elapsed(),
                    file_size: writer.byte_count(),
                    items_total: Some(total),
                    items_collected: Some(writer.item_count()),
                    error_reason: None,
                };
            }
        }

        let is_truncated = writer.is_truncated();
        let collected = writer.item_count();
        let byte_count = writer.byte_count();

        match writer.finish().await {
            Ok((size, items)) => {
                let duration = start.elapsed();
                CollectResult {
                    status: if is_truncated {
                        CollectStatus::Truncated(
                            if size >= 64 * 1024 * 1024 {
                                "size_limit".to_string()
                            } else {
                                "item_count_limit".to_string()
                            },
                        )
                    } else {
                        CollectStatus::Ok
                    },
                    duration,
                    file_size: size,
                    items_total: Some(total),
                    items_collected: Some(items),
                    error_reason: None,
                }
            }
            Err(e) => CollectResult {
                status: CollectStatus::Failed(e),
                duration: start.elapsed(),
                file_size: byte_count,
                items_total: Some(total),
                items_collected: Some(collected),
                error_reason: None,
            },
        }
    }
}
