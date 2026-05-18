use serde::Serialize;
use std::time::Instant;

use crate::collector::util::SCHEMA_VERSION;
use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::{OutputDir, SIZE_LIMIT};

#[derive(Clone)]
pub struct Process;

#[derive(Serialize)]
struct NsInodes {
    mnt: Option<u64>,
    net: Option<u64>,
    pid: Option<u64>,
    ipc: Option<u64>,
    uts: Option<u64>,
    user: Option<u64>,
    cgroup: Option<u64>,
    time: Option<u64>,
}

#[derive(Serialize)]
struct CgroupEntry {
    hierarchy: Option<u32>,
    controllers: Option<String>,
    path: Option<String>,
}

#[derive(Serialize)]
struct ProcessRecord {
    schema_version: &'static str,
    collection: &'static str,
    pid: Option<i32>,
    ppid: Option<i32>,
    name: Option<String>,
    state: Option<String>,
    uid: Option<u32>,
    threads: Option<i64>,
    cmdline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wchan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sched: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedstat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oom_score: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oom_score_adj: Option<i64>,
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
    ns_inodes: Option<NsInodes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cgroup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cgroup_parsed: Option<Vec<CgroupEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    loginuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_vmsize: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_vmrss: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_vmswap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_vmdata: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_seccomp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cap_effective: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cap_permitted: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cap_bounding: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cap_bits: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_nonvoluntary_ctxt_switches: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    io_read_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    io_write_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    io_cancelled_write_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fd_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    statm_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    statm_resident: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    statm_shared: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    statm_data: Option<u64>,
}

fn parse_ns_inode(raw: &str) -> Option<u64> {
    raw.split(':').nth(1)?.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok()
}

fn parse_cgroup_line(line: &str) -> Option<CgroupEntry> {
    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() < 3 {
        return None;
    }
    Some(CgroupEntry {
        hierarchy: parts[0].parse().ok(),
        controllers: if parts[1].is_empty() { None } else { Some(parts[1].to_string()) },
        path: if parts[2].is_empty() { None } else { Some(parts[2].to_string()) },
    })
}

fn decode_caps(cap: Option<u64>) -> Option<Vec<String>> {
    let bits = cap?;
    let mut names = Vec::new();
    for bit in 0..41 {
        if bits & (1u64 << bit) != 0 {
            names.push(format!("cap_{}", bit));
        }
    }
    if names.is_empty() { None } else { Some(names) }
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

            let start_time = stat.as_ref().map(|s| s.starttime);

            let wchan = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/wchan", proc.pid)),
            )
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "0");

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
            .ok()
            .and_then(|s| s.trim().parse().ok());
            let oom_score_adj = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/oom_score_adj", proc.pid)),
            )
            .ok()
            .and_then(|s| s.trim().parse().ok());
            let cgroup = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/cgroup", proc.pid)),
            )
            .ok();
            let cgroup_parsed = cgroup.as_ref().map(|raw| {
                raw.lines().filter_map(parse_cgroup_line).collect()
            });
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

            let ns_inodes = if ns_mnt.is_some() || ns_net.is_some() {
                Some(NsInodes {
                    mnt: ns_mnt.as_deref().and_then(parse_ns_inode),
                    net: ns_net.as_deref().and_then(parse_ns_inode),
                    pid: ns_pid.as_deref().and_then(parse_ns_inode),
                    ipc: ns_ipc.as_deref().and_then(parse_ns_inode),
                    uts: ns_uts.as_deref().and_then(parse_ns_inode),
                    user: ns_user.as_deref().and_then(parse_ns_inode),
                    cgroup: ns_cgroup.as_deref().and_then(parse_ns_inode),
                    time: ns_time.as_deref().and_then(parse_ns_inode),
                })
            } else {
                None
            };

            let status_vmsize = status.as_ref().and_then(|s| s.vmsize);
            let status_vmrss = status.as_ref().and_then(|s| s.vmrss);
            let status_vmswap = status.as_ref().and_then(|s| s.vmswap);
            let status_vmdata = status.as_ref().and_then(|s| s.vmdata);
            let status_threads = status.as_ref().map(|s| s.threads as i64);
            let status_seccomp = status.as_ref().and_then(|s| s.seccomp);
            let cap_effective = status.as_ref().map(|s| s.capeff);
            let cap_permitted = status.as_ref().map(|s| s.capprm);
            let cap_bounding = status.as_ref().and_then(|s| s.capbnd);
            let cap_bits = decode_caps(status.as_ref().and_then(|s| s.capbnd));
            let status_nonvoluntary_ctxt_switches = status
                .as_ref()
                .and_then(|s| s.nonvoluntary_ctxt_switches);

            let io_read_bytes = proc.io().ok().map(|i| i.read_bytes);
            let io_write_bytes = proc.io().ok().map(|i| i.write_bytes);
            let io_cancelled_write_bytes = proc.io().ok().map(|i| i.cancelled_write_bytes);

            let fd_count = std::fs::read_dir(
                probe.roots.resolve(&format!("/proc/{}/fd", proc.pid)),
            )
            .ok()
            .map(|dir| dir.filter_map(|e| e.ok()).count() as u64);

            let stack = std::fs::read_to_string(
                probe.roots.resolve(&format!("/proc/{}/stack", proc.pid)),
            )
            .ok()
            .map(|s| s.chars().take(2048).collect::<String>());

            let statm = proc.statm().ok();
            let statm_size = statm.as_ref().map(|s| s.size);
            let statm_resident = statm.as_ref().map(|s| s.resident);
            let statm_shared = statm.as_ref().map(|s| s.shared);
            let statm_data = statm.as_ref().map(|s| s.data);

            let record = ProcessRecord {
                schema_version: SCHEMA_VERSION,
                collection: "RT-04",
                pid,
                ppid,
                name,
                state,
                uid,
                threads,
                cmdline,
                start_time,
                wchan,
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
                ns_inodes,
                cgroup,
                cgroup_parsed,
                loginuid,
                exe,
                status_vmsize,
                status_vmrss,
                status_vmswap,
                status_vmdata,
                status_threads,
                status_seccomp,
                cap_effective,
                cap_permitted,
                cap_bounding,
                cap_bits,
                status_nonvoluntary_ctxt_switches,
                io_read_bytes,
                io_write_bytes,
                io_cancelled_write_bytes,
                fd_count,
                stack,
                statm_size,
                statm_resident,
                statm_shared,
                statm_data,
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
            CollectionStatus::Partial {
                degrading: probe.degraded.clone(),
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
            eprintln!("Process probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Process.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
