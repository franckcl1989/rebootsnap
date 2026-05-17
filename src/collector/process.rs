use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::{OutputDir, SIZE_LIMIT};

#[derive(Clone)]
pub struct Process;

#[derive(Serialize)]
struct ProcessRecord {
    collection: &'static str,
    pid: Option<i32>,
    ppid: Option<i32>,
    name: Option<String>,
    state: Option<String>,
    uid: Option<u32>,
    threads: Option<i64>,
    cmdline: Option<String>,
}

impl Process {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        if std::path::Path::new("/proc/1/status").exists() {
            ProbeOutcome {
                available: true,
                degraded: Vec::new(),
                reason: None,
                roots: roots.clone(),
            }
        } else {
            ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("/proc not accessible".into()),
                roots: roots.clone(),
            }
        }
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

        let entries: Vec<_> = procfs::process::all_processes()
            .map(|iter| iter.flatten().collect())
            .unwrap_or_default();
        let total: u64 = entries.len() as u64;

        let mut writer = match output.jsonl_writer("processes.jsonl") {
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

        let mut write_error: Option<String> = None;
        for proc in &entries {
            let pid = Some(proc.pid);
            let stat = proc.stat().ok();
            let status = proc.status().ok();
            let ppid = stat.as_ref().map(|s| s.ppid);
            let name = stat.as_ref().map(|s| s.comm.clone()).filter(|s| !s.is_empty());
            let state = stat.as_ref().map(|s| format!("{}", s.state));
            let threads = stat.as_ref().map(|s| s.num_threads);
            let uid = status.as_ref().map(|s| s.euid);
            let cmdline = proc
                .cmdline()
                .ok()
                .map(|c| c.join(" "))
                .filter(|s| !s.is_empty());

            let record = ProcessRecord {
                collection: "RT-04",
                pid,
                ppid,
                name,
                state,
                uid,
                threads,
                cmdline,
            };

            if let Err(e) = writer.write_line(&record).await {
                write_error = Some(e.to_string());
                break;
            }
        }

        let is_truncated = writer.is_truncated();
        let collected = writer.item_count();

        let (size, _) = writer.finish().await;

        let status = if let Some(e) = write_error {
            CollectionStatus::Failed { reason: e }
        } else if is_truncated {
            let reason = if size >= SIZE_LIMIT {
                "size_limit"
            } else {
                "item_count_limit"
            };
            CollectionStatus::Truncated {
                reason: reason.into(),
            }
        } else if probe.degraded.is_empty() {
            CollectionStatus::Ok
        } else {
            CollectionStatus::Degraded {
                missing: probe.degraded.clone(),
            }
        };

        CollectionOutcome {
            status,
            duration: start.elapsed(),
            file_size: size,
            items_total: Some(total),
            items_collected: Some(collected),
            mem_total_kb: None,
            mem_available_kb: None,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
