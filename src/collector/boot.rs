use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Boot;

#[derive(Serialize)]
struct BootRecord {
    schema_version: &'static str,
    collection: &'static str,
    boot_id: Option<String>,
    uptime_seconds: Option<f64>,
    kernel_version: Option<String>,
    cmdline: Option<String>,
    hostname: Option<String>,
    domainname: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/kernel/random/boot_id",
    "/proc/uptime",
    "/proc/version",
    "/proc/cmdline",
    "/proc/sys/kernel/hostname",
    "/proc/sys/kernel/domainname",
];

impl Boot {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all boot identity files missing")
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

        let boot_id = read_trimmed(&probe.roots, FILES[0]);
        let uptime = read_trimmed(&probe.roots, FILES[1])
            .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok());
        let kernel_version = read_trimmed(&probe.roots, FILES[2]);
        let cmdline = read_trimmed(&probe.roots, FILES[3]);
        let hostname = read_trimmed(&probe.roots, FILES[4]);
        let domainname = read_trimmed(&probe.roots, FILES[5]);

        let record = BootRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-01",
            boot_id: boot_id.clone(),
            uptime_seconds: uptime,
            kernel_version: kernel_version.clone(),
            cmdline,
            hostname: hostname.clone(),
            domainname,
        };

        let writer = match output.json_writer("boot.json") {
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
                    hostname,
                    kernel_version,
                    boot_id,
                    uptime_seconds: uptime.map(|u| u as u64),
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
                    hostname,
                    kernel_version,
                    boot_id,
                    uptime_seconds: uptime.map(|u| u as u64),
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
            hostname,
            kernel_version,
            boot_id,
            uptime_seconds: uptime.map(|u| u as u64),
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
        let probe = Boot.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Boot probe unavailable in mock - skipping");
            return;
        }
        let outcome = Boot.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }

    #[tokio::test]
    async fn test_collect_content() {
        let ctx = setup_test("normal");
        let probe = Boot.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Boot.collect(&ctx.output, &probe).await;
        assert!(outcome.file_size > 0);

        let output_dir = ctx.output.root();
        let entries: Vec<_> = std::fs::read_dir(output_dir).expect("read_dir")
            .filter_map(|e| e.ok()).collect();
        assert!(!entries.is_empty(), "should have output files");

        let content = std::fs::read_to_string(entries[0].path()).expect("read output");
        let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(json["collection"], "RT-01");
        assert!(json["hostname"].is_string(), "hostname should be present");
    }
}
