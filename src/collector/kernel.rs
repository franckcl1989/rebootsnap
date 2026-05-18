use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Kernel;

#[derive(Serialize)]
struct MceBankEntry {
    cpu: String,
    bank: String,
    value: String,
}

#[derive(Serialize)]
struct KernelRecord {
    schema_version: &'static str,
    collection: &'static str,
    ostype: Option<String>,
    osrelease: Option<String>,
    modules: Option<String>,
    tainted: Option<String>,
    core_pattern: Option<String>,
    panic: Option<String>,
    printk: Option<String>,
    watchdog: Option<String>,
    soft_watchdog: Option<String>,
    nmi_watchdog: Option<String>,
    kexec_load_disabled: Option<String>,
    hung_task_panic: Option<String>,
    hung_task_timeout_secs: Option<String>,
    hung_task_check_interval_secs: Option<String>,
    sysrq: Option<String>,
    panic_on_oops: Option<String>,
    unknown_nmi_panic: Option<String>,
    kexec_crash_loaded: Option<String>,
    kexec_crash_size: Option<String>,
    livepatch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mce_banks: Option<Vec<MceBankEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kernel_counters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    irq_affinity: Option<Vec<serde_json::Value>>,
}

const FILES: &[&str] = &[
    "/proc/sys/kernel/ostype",
    "/proc/sys/kernel/osrelease",
    "/proc/modules",
    "/proc/sys/kernel/tainted",
    "/proc/sys/kernel/core_pattern",
    "/proc/sys/kernel/panic",
    "/proc/sys/kernel/printk",
    "/proc/sys/kernel/watchdog",
    "/proc/sys/kernel/soft_watchdog",
    "/proc/sys/kernel/nmi_watchdog",
    "/proc/sys/kernel/kexec_load_disabled",
    "/proc/sys/kernel/hung_task_panic",
    "/proc/sys/kernel/hung_task_timeout_secs",
    "/proc/sys/kernel/hung_task_check_interval_secs",
    "/proc/sys/kernel/sysrq",
    "/proc/sys/kernel/panic_on_oops",
    "/proc/sys/kernel/unknown_nmi_panic",
    "/sys/kernel/kexec_crash_loaded",
    "/sys/kernel/kexec_crash_size",
    "/sys/kernel/livepatch",
];

const UNSUPPORTED_IF_MISSING: &[&str] = &[
    "/proc/sys/kernel/hung_task_check_interval_secs",
];

impl Kernel {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all kernel files missing")
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

        let record = KernelRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-02",
            ostype: read_trimmed(&probe.roots, FILES[0]),
            osrelease: read_trimmed(&probe.roots, FILES[1]),
            modules: read_trimmed(&probe.roots, FILES[2]),
            tainted: read_trimmed(&probe.roots, FILES[3]),
            core_pattern: read_trimmed(&probe.roots, FILES[4]),
            panic: read_trimmed(&probe.roots, FILES[5]),
            printk: read_trimmed(&probe.roots, FILES[6]),
            watchdog: read_trimmed(&probe.roots, FILES[7]),
            soft_watchdog: read_trimmed(&probe.roots, FILES[8]),
            nmi_watchdog: read_trimmed(&probe.roots, FILES[9]),
            kexec_load_disabled: read_trimmed(&probe.roots, FILES[10]),
            hung_task_panic: read_trimmed(&probe.roots, FILES[11]),
            hung_task_timeout_secs: read_trimmed(&probe.roots, FILES[12]),
            hung_task_check_interval_secs: read_trimmed(&probe.roots, FILES[13]),
            sysrq: read_trimmed(&probe.roots, FILES[14]),
            panic_on_oops: read_trimmed(&probe.roots, FILES[15]),
            unknown_nmi_panic: read_trimmed(&probe.roots, FILES[16]),
            kexec_crash_loaded: read_trimmed(&probe.roots, FILES[17]),
            kexec_crash_size: read_trimmed(&probe.roots, FILES[18]),
            livepatch: {
                std::fs::read_dir(probe.roots.resolve(FILES[19]))
                    .ok()
                    .map(|dir| {
                        let count = dir.filter_map(|e| e.ok()).count();
                        format!("present:{}", count)
                    })
                    .or_else(|| Some("absent".to_string()))
            },
            mce_banks: None,
            kernel_counters: None,
            irq_affinity: None,
        };

        let writer = match output.json_writer("kernel.json") {
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
        let probe = Kernel.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Kernel probe unavailable in mock - skipping");
            return;
        }
        let outcome = Kernel.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }

    #[tokio::test]
    async fn test_collect_content() {
        let ctx = setup_test("normal");
        let probe = Kernel.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Kernel.collect(&ctx.output, &probe).await;
        assert!(outcome.file_size > 0);

        let output_dir = ctx.output.root();
        let entries: Vec<_> = std::fs::read_dir(output_dir).expect("read_dir")
            .filter_map(|e| e.ok()).collect();
        assert!(!entries.is_empty(), "should have output files");

        let content = std::fs::read_to_string(entries[0].path()).expect("read output");
        let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(json["collection"], "RT-02");
        assert!(json["ostype"].is_string(), "ostype should be present");
    }
}
