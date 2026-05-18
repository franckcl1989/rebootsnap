use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Mount;

#[derive(Serialize)]
struct MountRecord {
    collection: &'static str,
    mountinfo: Option<String>,
    mounts: Option<String>,
    mountstats: Option<String>,
    filesystems: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ext4_features: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suid_dumpable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fs_debug_types: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/self/mountinfo",
    "/proc/self/mounts",
    "/proc/self/mountstats",
    "/proc/filesystems",
    "/sys/fs/ext4/features",
    "/proc/sys/fs/suid_dumpable",
];

impl Mount {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all mount files missing")
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

        fn read_raw(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path)).ok()
        }

        let record = MountRecord {
            collection: "RT-09",
            mountinfo: read_raw(&probe.roots, FILES[0]),
            mounts: read_raw(&probe.roots, FILES[1]),
            mountstats: read_raw(&probe.roots, FILES[2]),
            filesystems: read_raw(&probe.roots, FILES[3]),
            ext4_features: read_raw(&probe.roots, FILES[4]),
            suid_dumpable: read_raw(&probe.roots, FILES[5]),
            fs_debug_types: enumerate_fs_debug(&probe.roots),
        };

        let writer = match output.json_writer("mounts.json") {
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
                CollectionStatus::Degraded {
                    missing: probe.degraded.clone(),
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
        let probe = Mount.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Mount probe unavailable in mock - skipping");
            return;
        }
        let outcome = Mount.collect(&ctx.output, &probe).await;
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}

fn enumerate_fs_debug(roots: &FsRoots) -> Option<String> {
    let dir = roots.resolve("/sys/fs");
    let entries = std::fs::read_dir(&dir).ok()?;
    let fs_types: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    if fs_types.is_empty() {
        None
    } else {
        serde_json::to_string(&fs_types).ok()
    }
}
