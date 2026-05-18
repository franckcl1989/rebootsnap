use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Fd;

#[derive(Serialize)]
struct FdRecord {
    schema_version: &'static str,
    collection: &'static str,
    file_nr: Option<String>,
    file_max: Option<String>,
    inode_nr: Option<String>,
    inode_max: Option<String>,
    nr_open: Option<String>,
    pid_max: Option<String>,
    locks: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/fs/file-nr",
    "/proc/sys/fs/file-max",
    "/proc/sys/fs/inode-nr",
    "/proc/sys/fs/inode-max",
    "/proc/sys/fs/nr_open",
    "/proc/sys/kernel/pid_max",
    "/proc/locks",
];

const UNSUPPORTED_IF_MISSING: &[&str] = &[
    "/proc/sys/fs/inode-max",
];

impl Fd {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all fd files missing")
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

        let record = FdRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-07",
            file_nr: read_trimmed(&probe.roots, FILES[0]),
            file_max: read_trimmed(&probe.roots, FILES[1]),
            inode_nr: read_trimmed(&probe.roots, FILES[2]),
            inode_max: read_trimmed(&probe.roots, FILES[3]),
            nr_open: read_trimmed(&probe.roots, FILES[4]),
            pid_max: read_trimmed(&probe.roots, FILES[5]),
            locks: read_trimmed(&probe.roots, FILES[6]),
        };

        let writer = match output.json_writer("fds.json") {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::setup_test;

    #[tokio::test]
    async fn test_collect_normal() {
        let ctx = setup_test("normal");
        let probe = Fd.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Fd probe unavailable in mock - skipping");
            return;
        }
        let outcome = Fd.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }

    #[tokio::test]
    async fn test_collect_content() {
        let ctx = setup_test("normal");
        let probe = Fd.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Fd.collect(&ctx.output, &probe).await;
        assert!(outcome.file_size > 0);

        let output_dir = ctx.output.root();
        let entries: Vec<_> = std::fs::read_dir(output_dir).expect("read_dir")
            .filter_map(|e| e.ok()).collect();
        assert!(!entries.is_empty(), "should have output files");

        let content = std::fs::read_to_string(entries[0].path()).expect("read output");
        let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(json["collection"], "RT-07");
        assert!(json["file_nr"].is_string(), "file_nr should be present");
    }
}
