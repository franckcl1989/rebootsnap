use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Mount;

#[derive(Serialize)]
struct MountRecord {
    schema_version: &'static str,
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
    fs_debug_types: Option<Vec<String>>,
}

const FILES: &[&str] = &[
    "/proc/self/mountinfo",
    "/proc/self/mounts",
    "/proc/self/mountstats",
    "/proc/filesystems",
    "/sys/fs/ext4/features",
    "/proc/sys/fs/suid_dumpable",
];

const UNSUPPORTED_IF_MISSING: &[&str] = &[
    "/sys/fs/ext4/features",
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

        let record = MountRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-09",
            mountinfo: read_trimmed(&probe.roots, FILES[0]),
            mounts: read_trimmed(&probe.roots, FILES[1]),
            mountstats: read_trimmed(&probe.roots, FILES[2]),
            filesystems: read_trimmed(&probe.roots, FILES[3]),
            ext4_features: read_trimmed(&probe.roots, FILES[4]),
            suid_dumpable: read_trimmed(&probe.roots, FILES[5]),
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

        let (unsupported, degraded): (Vec<_>, Vec<_>) = probe.degraded.iter()
            .cloned()
            .partition(|f| UNSUPPORTED_IF_MISSING.contains(&f.as_str()));

        CollectionOutcome {
            status: if !degraded.is_empty() {
                CollectionStatus::Partial { degrading: degraded }
            } else if !unsupported.is_empty() {
                CollectionStatus::Unsupported { reason: unsupported.join(", ") }
            } else {
                CollectionStatus::Ok
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

fn enumerate_fs_debug(roots: &FsRoots) -> Option<Vec<String>> {
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
        Some(fs_types)
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
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
