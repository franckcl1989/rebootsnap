use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Tmpfs;

#[derive(Serialize)]
struct TmpfsRecord {
    schema_version: &'static str,
    collection: &'static str,
    mounts: Option<String>,
    run_entries: Option<u64>,
    dev_shm_entries: Option<u64>,
    tmp_entries: Option<u64>,
    tmpfs_total_entries: Option<u64>,
}

const MOUNTS_FILE: &str = "/proc/mounts";
const RUNTIME_DIRS: &[&str] = &["/run", "/dev/shm", "/tmp"];

impl Tmpfs {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        let mounts_exists = roots.exists(MOUNTS_FILE);
        if !mounts_exists {
            return ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("all tmpfs files missing".into()),
                roots: roots.clone(),
            };
        }
        let degraded: Vec<String> = RUNTIME_DIRS
            .iter()
            .filter(|d| !roots.exists(d))
            .map(|s| s.to_string())
            .collect();
        if degraded.len() == RUNTIME_DIRS.len() {
            ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("all tmpfs runtime dirs missing".into()),
                roots: roots.clone(),
            }
        } else if degraded.is_empty() {
            ProbeOutcome {
                available: true,
                degraded: Vec::new(),
                reason: None,
                roots: roots.clone(),
            }
        } else {
            ProbeOutcome {
                available: true,
                degraded,
                reason: None,
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

        fn count_dir(roots: &FsRoots, path: &str) -> Option<u64> {
            let entries = std::fs::read_dir(roots.resolve(path)).ok()?;
            Some(entries.flatten().count() as u64)
        }

        let run_entries_val = count_dir(&probe.roots, RUNTIME_DIRS[0]);
        let dev_shm_entries_val = count_dir(&probe.roots, RUNTIME_DIRS[1]);
        let tmp_entries_val = count_dir(&probe.roots, RUNTIME_DIRS[2]);
        let total_entries = match (&run_entries_val, &dev_shm_entries_val, &tmp_entries_val) {
            (None, None, None) => None,
            _ => Some(
                run_entries_val.unwrap_or(0)
                    + dev_shm_entries_val.unwrap_or(0)
                    + tmp_entries_val.unwrap_or(0),
            ),
        };

        let record = TmpfsRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-08",
            mounts: read_trimmed(&probe.roots, MOUNTS_FILE),
            run_entries: run_entries_val,
            dev_shm_entries: dev_shm_entries_val,
            tmp_entries: tmp_entries_val,
            tmpfs_total_entries: total_entries,
        };

        let writer = match output.json_writer("tmpfs.json") {
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
        let probe = Tmpfs.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Tmpfs probe unavailable in mock - skipping");
            return;
        }
        let outcome = Tmpfs.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
