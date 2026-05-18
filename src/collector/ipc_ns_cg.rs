use serde::Serialize;
use std::path::Path;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct IpcNsCg;

#[derive(Serialize)]
struct IpcNsCgRecord {
    schema_version: &'static str,
    collection: &'static str,
    cgroups: Option<String>,
    sysv_ipc_msg: Option<String>,
    sysv_ipc_sem: Option<String>,
    sysv_ipc_shm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    init_ns_ipc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    init_cgroup: Option<String>,
    cgroup_tree_v1: Vec<CgroupV1Entry>,
    cgroup_tree_v2: Vec<CgroupV2Entry>,
    posix_mq_queues: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mqueue_queues_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mqueue_msg_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mqueue_msgsize_max: Option<String>,
}

#[derive(Serialize)]
struct CgroupV1Entry {
    path: String,
    controller: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tasks_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    procs_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_limit_in_bytes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_usage_in_bytes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_max_usage_in_bytes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_failcnt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_cfs_period_us: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_cfs_quota_us: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_shares: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_stat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blkio_throttle_read_bps_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blkio_throttle_write_bps_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pids_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pids_current: Option<String>,
}

#[derive(Serialize)]
struct CgroupV2Entry {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    controllers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtree_control: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    procs_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    threads_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_current: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_high: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_low: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_weight_nice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    io_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    io_weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pids_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pids_current: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_pressure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_pressure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    io_pressure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_events: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_events_local: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/cgroups",
    "/proc/sysvipc/msg",
    "/proc/sysvipc/sem",
    "/proc/sysvipc/shm",
    "/proc/sys/fs/mqueue/queues_max",
    "/proc/sys/fs/mqueue/msg_max",
    "/proc/sys/fs/mqueue/msgsize_max",
];

fn read_init_ns_ipc(roots: &FsRoots) -> Option<String> {
    std::fs::read_link(roots.resolve("/proc/1/ns/ipc"))
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

fn read_init_cgroup(roots: &FsRoots) -> Option<String> {
    std::fs::read_to_string(roots.resolve("/proc/1/cgroup")).ok()
}

fn query_posix_mq(roots: &FsRoots) -> Vec<String> {
    let dir = roots.resolve("/dev/mqueue");
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect()
}

fn read_cgroup_v1(roots: &FsRoots) -> Vec<CgroupV1Entry> {
    let cgroup_root = roots.resolve("/sys/fs/cgroup");
    if !cgroup_root.join("memory").is_dir() { return Vec::new() }
    let Ok(dir_entries) = std::fs::read_dir(&cgroup_root) else { return Vec::new() };
    let mut result: Vec<CgroupV1Entry> = Vec::new();
    for entry in dir_entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue }
        let Some(controller) = path.file_name().and_then(|n| n.to_str()) else { continue };
        walk_cgroup_v1(&path, &cgroup_root, controller, 0, &mut result);
        if result.len() >= 50 { break }
    }
    result.truncate(50);
    result
}

fn walk_cgroup_v1(
    dir: &Path,
    base: &Path,
    controller: &str,
    depth: u32,
    result: &mut Vec<CgroupV1Entry>,
) {
    if depth > 2 || result.len() >= 50 {
        return;
    }
    let rel = dir.strip_prefix(base).unwrap_or(dir);
    let path_str = format!("/{}", rel.display());
    result.push(collect_cgroup_v1_entry(dir, controller, &path_str));
    if let Ok(children) = std::fs::read_dir(dir) {
        for child in children.flatten() {
            let child_path = child.path();
            if child_path.is_dir() {
                walk_cgroup_v1(&child_path, base, controller, depth + 1, result);
            }
        }
    }
}

fn collect_cgroup_v1_entry(dir: &Path, controller: &str, path: &str) -> CgroupV1Entry {
    fn read_file(dir: &Path, name: &str) -> Option<String> {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .map(|s| s.trim().to_string())
    }
    fn count_lines(dir: &Path, name: &str) -> Option<usize> {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .map(|s| s.lines().count())
    }
    CgroupV1Entry {
        path: path.to_string(),
        controller: controller.to_string(),
        tasks_count: count_lines(dir, "tasks"),
        procs_count: count_lines(dir, "cgroup.procs"),
        memory_limit_in_bytes: read_file(dir, "memory.limit_in_bytes"),
        memory_usage_in_bytes: read_file(dir, "memory.usage_in_bytes"),
        memory_max_usage_in_bytes: read_file(dir, "memory.max_usage_in_bytes"),
        memory_failcnt: read_file(dir, "memory.failcnt"),
        cpu_cfs_period_us: read_file(dir, "cpu.cfs_period_us"),
        cpu_cfs_quota_us: read_file(dir, "cpu.cfs_quota_us"),
        cpu_shares: read_file(dir, "cpu.shares"),
        cpu_stat: read_file(dir, "cpu.stat"),
        blkio_throttle_read_bps_device: read_file(dir, "blkio.throttle.read_bps_device"),
        blkio_throttle_write_bps_device: read_file(dir, "blkio.throttle.write_bps_device"),
        pids_max: read_file(dir, "pids.max"),
        pids_current: read_file(dir, "pids.current"),
    }
}

fn read_cgroup_v2(roots: &FsRoots) -> Vec<CgroupV2Entry> {
    let cgroup_root = roots.resolve("/sys/fs/cgroup");
    if !cgroup_root.join("cgroup.controllers").is_file() { return Vec::new() }
    let mut result: Vec<CgroupV2Entry> = Vec::new();
    walk_cgroup_v2(&cgroup_root, "", 0, &mut result);
    result.truncate(50);
    result
}

fn walk_cgroup_v2(dir: &Path, rel_path: &str, depth: u32, result: &mut Vec<CgroupV2Entry>) {
    if depth > 2 || result.len() >= 50 {
        return;
    }
    let path_str = if rel_path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", rel_path)
    };
    result.push(collect_cgroup_v2_entry(dir, &path_str));
    if let Ok(children) = std::fs::read_dir(dir) {
        for child in children.flatten() {
            let child_path = child.path();
            if child_path.is_dir() {
                let Some(child_name) = child_path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let child_rel = if rel_path.is_empty() {
                    child_name.to_string()
                } else {
                    format!("{}/{}", rel_path, child_name)
                };
                walk_cgroup_v2(&child_path, &child_rel, depth + 1, result);
            }
        }
    }
}

fn collect_cgroup_v2_entry(dir: &Path, path: &str) -> CgroupV2Entry {
    fn read_file(dir: &Path, name: &str) -> Option<String> {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .map(|s| s.trim().to_string())
    }
    fn count_lines(dir: &Path, name: &str) -> Option<usize> {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .map(|s| s.lines().count())
    }
    fn first_line(dir: &Path, name: &str) -> Option<String> {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .map(|s| s.lines().next().unwrap_or("").to_string())
    }
    fn first_n_lines(dir: &Path, name: &str, n: usize) -> Option<String> {
        std::fs::read_to_string(dir.join(name)).ok().map(|s| {
            s.lines()
                .take(n)
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
    CgroupV2Entry {
        path: path.to_string(),
        controllers: read_file(dir, "cgroup.controllers"),
        subtree_control: read_file(dir, "cgroup.subtree_control"),
        procs_count: count_lines(dir, "cgroup.procs"),
        threads_count: count_lines(dir, "cgroup.threads"),
        memory_max: read_file(dir, "memory.max"),
        memory_current: read_file(dir, "memory.current"),
        memory_high: read_file(dir, "memory.high"),
        memory_low: read_file(dir, "memory.low"),
        cpu_max: read_file(dir, "cpu.max"),
        cpu_weight: read_file(dir, "cpu.weight"),
        cpu_weight_nice: read_file(dir, "cpu.weight.nice"),
        io_max: read_file(dir, "io.max"),
        io_weight: read_file(dir, "io.weight"),
        pids_max: read_file(dir, "pids.max"),
        pids_current: read_file(dir, "pids.current"),
        cpu_pressure: first_line(dir, "cpu.pressure"),
        memory_pressure: first_line(dir, "memory.pressure"),
        io_pressure: first_line(dir, "io.pressure"),
        memory_events: first_n_lines(dir, "memory.events", 3),
        memory_events_local: first_n_lines(dir, "memory.events.local", 3),
    }
}

impl IpcNsCg {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all ipc/ns/cg files missing")
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

        let record = IpcNsCgRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-14",
            cgroups: read_trimmed(&probe.roots, FILES[0]),
            sysv_ipc_msg: read_trimmed(&probe.roots, FILES[1]),
            sysv_ipc_sem: read_trimmed(&probe.roots, FILES[2]),
            sysv_ipc_shm: read_trimmed(&probe.roots, FILES[3]),
            init_ns_ipc: read_init_ns_ipc(&probe.roots),
            init_cgroup: read_init_cgroup(&probe.roots),
            cgroup_tree_v1: read_cgroup_v1(&probe.roots),
            cgroup_tree_v2: read_cgroup_v2(&probe.roots),
            posix_mq_queues: query_posix_mq(&probe.roots),
            mqueue_queues_max: read_trimmed(&probe.roots, FILES[4]),
            mqueue_msg_max: read_trimmed(&probe.roots, FILES[5]),
            mqueue_msgsize_max: read_trimmed(&probe.roots, FILES[6]),
        };

        let writer = match output.json_writer("ipc_ns_cg.json") {
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
        let (size, _) = match writer.commit(&record).await {
            Ok(v) => v,
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

        CollectionOutcome {
            status: if probe.degraded.is_empty() {
                CollectionStatus::Ok
            } else {
                CollectionStatus::Partial {
                    degrading: probe.degraded.clone(),
                }
            },
            duration: start.elapsed(),
            file_size: size,
            items_total: None,
            items_collected: None,
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
        let probe = IpcNsCg.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("IpcNsCg probe unavailable in mock - skipping");
            return;
        }
        let outcome = IpcNsCg.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
