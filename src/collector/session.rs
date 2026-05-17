use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

pub struct Session;

#[derive(Serialize)]
struct SessionRecord {
    collection: &'static str,
    utmp_size: Option<u64>,
    passwd_mtime: Option<String>,
    group_mtime: Option<String>,
}

const FILES: &[&str] = &["/var/run/utmp", "/etc/passwd", "/etc/group"];

impl Session {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all session files missing")
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

        fn read_size(roots: &FsRoots, path: &str) -> Option<u64> {
            std::fs::metadata(roots.resolve(path)).ok().map(|m| m.len())
        }

        fn read_mtime(roots: &FsRoots, path: &str) -> Option<String> {
            let m = std::fs::metadata(roots.resolve(path)).ok()?;
            let t = m.modified().ok()?;
            let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?;
            Some(secs.as_secs().to_string())
        }

        let record = SessionRecord {
            collection: "RT-15",
            utmp_size: read_size(&probe.roots, FILES[0]),
            passwd_mtime: read_mtime(&probe.roots, FILES[1]),
            group_mtime: read_mtime(&probe.roots, FILES[2]),
        };

        let writer = match output.json_writer("sessions.json") {
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
