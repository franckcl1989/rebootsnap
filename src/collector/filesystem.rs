use serde::Serialize;
use std::ffi::CString;
use std::time::Instant;

use crate::collector::util::SCHEMA_VERSION;
use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Filesystem;

#[derive(Serialize)]
struct FilesystemRecord {
    schema_version: &'static str,
    collection: &'static str,
    mounts_capacity: Vec<MountCapacity>,
    critical_dirs: Vec<DirCapacity>,
    block_devices: Vec<BlockDevEntry>,
}

#[derive(Serialize)]
struct MountCapacity {
    mount: String,
    fs_type: String,
    device: String,
    total_bytes: u64,
    used_bytes: u64,
    avail_bytes: u64,
    total_inodes: u64,
    used_inodes: u64,
}

#[derive(Serialize)]
struct DirCapacity {
    path: String,
    total_bytes: u64,
    used_bytes: u64,
    avail_bytes: u64,
}

#[derive(Serialize)]
struct BlockDevEntry {
    name: String,
    size_sectors: Option<String>,
    major_minor: Option<String>,
    fs_uuid: Option<String>,
    fs_label: Option<String>,
}

fn statvfs(path: &str) -> Option<(u64, u64, u64, u64, u64)> {
    let c_path = CString::new(path).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }
    let total = stat.f_blocks.saturating_mul(stat.f_bsize);
    let avail = stat.f_bavail.saturating_mul(stat.f_bsize);
    let used = total.saturating_sub(avail);
    let total_inodes = stat.f_files;
    let used_inodes = stat.f_files.saturating_sub(stat.f_ffree);
    Some((total, used, avail, total_inodes, used_inodes))
}

impl Filesystem {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        ProbeOutcome {
            available: true,
            degraded: Vec::new(),
            reason: None,
            roots: roots.clone(),
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

        let mounts_capacity = {
            let mut caps = Vec::new();
            let mounts_raw = std::fs::read_to_string(
                probe.roots.resolve("/proc/mounts"),
            )
            .unwrap_or_default();
            let skip_fs = [
                "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "cgroup", "cgroup2",
                "pstore", "bpf", "debugfs", "tracefs", "securityfs", "configfs",
                "hugetlbfs", "mqueue", "fusectl", "selinuxfs",
            ];
            for line in mounts_raw.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 3 {
                    continue;
                }
                let device = parts[0].to_string();
                let mount = parts[1].to_string();
                let fs_type = parts[2].to_string();
                if skip_fs.contains(&fs_type.as_str()) {
                    continue;
                }
                if let Some((total, used, avail, total_inodes, used_inodes)) = statvfs(&mount) {
                    caps.push(MountCapacity {
                        mount,
                        fs_type,
                        device,
                        total_bytes: total,
                        used_bytes: used,
                        avail_bytes: avail,
                        total_inodes,
                        used_inodes,
                    });
                }
            }
            caps
        };

        let critical_dirs = {
            let dirs = [
                ("/var/log", "/var/log"),
                ("/var/lib/containerd", "/var/lib/containerd"),
                ("/var/lib/docker", "/var/lib/docker"),
                ("/var/lib/kubelet", "/var/lib/kubelet"),
            ];
            let mut out = Vec::new();
            for &(label, path) in &dirs {
                let resolved = probe.roots.resolve(path);
                let path_str = resolved.to_string_lossy().to_string();
                if let Some((total, used, avail, _, _)) = statvfs(&path_str) {
                    out.push(DirCapacity {
                        path: label.to_string(),
                        total_bytes: total,
                        used_bytes: used,
                        avail_bytes: avail,
                    });
                }
            }
            out
        };

        let block_devices = {
            let block_dir = probe.roots.resolve("/sys/class/block");
            let mut devs = Vec::new();
            if let Ok(dir) = std::fs::read_dir(&block_dir) {
                for entry in dir.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let base = entry.path();
                    let size_sectors = std::fs::read_to_string(base.join("size"))
                        .ok()
                        .map(|s| s.trim().to_string());
                    let major_minor = std::fs::read_to_string(base.join("dev"))
                        .ok()
                        .map(|s| s.trim().to_string());
                    let (fs_uuid, fs_label) = std::fs::read_to_string(base.join("device/uevent"))
                        .ok()
                        .map(|s| {
                            let mut uuid = None;
                            let mut label = None;
                            for line in s.lines() {
                                if line.starts_with("ID_FS_UUID=") {
                                    uuid = Some(line.trim_start_matches("ID_FS_UUID=").to_string());
                                } else if line.starts_with("ID_FS_LABEL=") {
                                    label = Some(line.trim_start_matches("ID_FS_LABEL=").to_string());
                                }
                            }
                            (uuid, label)
                        })
                        .unwrap_or((None, None));
                    devs.push(BlockDevEntry {
                        name,
                        size_sectors,
                        major_minor,
                        fs_uuid,
                        fs_label,
                    });
                    if devs.len() >= 50 {
                        break;
                    }
                }
            }
            devs
        };

        let mounts_count = mounts_capacity.len() as u64;

        let record = FilesystemRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-22",
            mounts_capacity,
            critical_dirs,
            block_devices,
        };

        let writer = match output.json_writer("filesystem.json") {
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
            items_collected: Some(mounts_count),
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
        let probe = Filesystem.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Filesystem.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
