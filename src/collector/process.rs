use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

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
    pub async fn probe(&self) -> ProbeOutcome {
        if !std::path::Path::new("/proc/1/status").exists() {
            return ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("/proc not accessible".into()),
            };
        }
        ProbeOutcome {
            available: true,
            degraded: Vec::new(),
            reason: None,
        }
    }

    pub async fn collect(
        &self,
        output: &OutputDir,
        _probe: &ProbeOutcome,
    ) -> CollectionOutcome {
        let start = Instant::now();

        let entries: Vec<_> = procfs::process::all_processes()
            .map(|iter| iter.flatten().collect())
            .unwrap_or_default();
        let mut total: u64 = entries.len() as u64;

        let Ok(mut writer) = output.jsonl_writer("processes.jsonl") else {
            return CollectionOutcome {
                status: CollectionStatus::Failed {
                    reason: "jsonl writer creation failed".into(),
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

        for proc in &entries {
            let pid = Some(proc.pid);
            let stat = proc.stat().ok();
            let status = proc.status().ok();
            let ppid = stat.as_ref().map(|s| s.ppid);
            let name = stat.as_ref().map(|s| s.comm.clone()).filter(|s| !s.is_empty());
            let state = stat.as_ref().map(|s| format!("{}", s.state));
            let threads = stat.as_ref().map(|s| s.num_threads);
            let uid = status.as_ref().map(|s| s.euid as u32);
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

            if writer.write_line(&record).await.is_err() {
                total = writer.item_count();
                break;
            }
        }

        let is_truncated = writer.is_truncated();
        let items = writer.item_count();

        let (size, _items) = writer.finish().await;

        let status = if is_truncated {
            let reason = if size >= 64 * 1024 * 1024 {
                "size_limit"
            } else {
                "item_count_limit"
            };
            CollectionStatus::Truncated {
                reason: reason.into(),
            }
        } else {
            CollectionStatus::Ok
        };

        CollectionOutcome {
            status,
            duration: start.elapsed(),
            file_size: size,
            items_total: Some(total),
            items_collected: Some(items),
            mem_total_kb: None,
            mem_available_kb: None,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
