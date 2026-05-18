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
    #[serde(skip_serializing_if = "Option::is_none")]
    limits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sched: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedstat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oom_score: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oom_score_adj: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_mnt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_pid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_ipc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_uts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_cgroup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ns_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cgroup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    loginuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exe: Option<String>,
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

            let limits = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/limits", proc.pid)),
            )
            .ok()
            .map(|s| s.chars().take(4096).collect::<String>());
            let sched = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/sched", proc.pid)),
            )
            .ok()
            .map(|s| s.chars().take(2048).collect::<String>());
            let schedstat = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/schedstat", proc.pid)),
            )
            .ok();
            let oom_score = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/oom_score", proc.pid)),
            )
            .ok();
            let oom_score_adj = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/oom_score_adj", proc.pid)),
            )
            .ok();
            let cgroup = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/cgroup", proc.pid)),
            )
            .ok();
            let loginuid = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/loginuid", proc.pid)),
            )
            .ok();
            let exe = std::fs::read_link(
                probe.roots.resolve(&format!("/proc/{}/exe", proc.pid)),
            )
            .ok()
            .map(|p| p.to_string_lossy().to_string());

            fn read_ns(roots: &FsRoots, pid: i32, ns: &str) -> Option<String> {
                std::fs::read_to_string(
                    roots.resolve(&format!("/proc/{}/ns/{}", pid, ns)),
                )
                .ok()
            }
            let ns_mnt = read_ns(&probe.roots, proc.pid, "mnt");
            let ns_net = read_ns(&probe.roots, proc.pid, "net");
            let ns_pid = read_ns(&probe.roots, proc.pid, "pid");
            let ns_ipc = read_ns(&probe.roots, proc.pid, "ipc");
            let ns_uts = read_ns(&probe.roots, proc.pid, "uts");
            let ns_user = read_ns(&probe.roots, proc.pid, "user");
            let ns_cgroup = read_ns(&probe.roots, proc.pid, "cgroup");
            let ns_time = read_ns(&probe.roots, proc.pid, "time");

            let record = ProcessRecord {
                collection: "RT-04",
                pid,
                ppid,
                name,
                state,
                uid,
                threads,
                cmdline,
                limits,
                sched,
                schedstat,
                oom_score,
                oom_score_adj,
                ns_mnt,
                ns_net,
                ns_pid,
                ns_ipc,
                ns_uts,
                ns_user,
                ns_cgroup,
                ns_time,
                cgroup,
                loginuid,
                exe,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::setup_test;

    #[tokio::test]
    async fn test_collect_normal() {
        let ctx = setup_test("normal");
        let probe = Process.probe(&ctx.roots).await;
        if !probe.available {
            println!("Process probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Process.collect(&ctx.output, &probe).await;
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
