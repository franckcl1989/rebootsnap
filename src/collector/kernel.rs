use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Kernel;

#[derive(Serialize)]
struct KernelRecord {
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

        fn read_raw(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path)).ok()
        }

        let record = KernelRecord {
            collection: "RT-02",
            ostype: read_raw(&probe.roots, FILES[0]),
            osrelease: read_raw(&probe.roots, FILES[1]),
            modules: read_raw(&probe.roots, FILES[2]),
            tainted: read_raw(&probe.roots, FILES[3]),
            core_pattern: read_raw(&probe.roots, FILES[4]),
            panic: read_raw(&probe.roots, FILES[5]),
            printk: read_raw(&probe.roots, FILES[6]),
            watchdog: read_raw(&probe.roots, FILES[7]),
            soft_watchdog: read_raw(&probe.roots, FILES[8]),
            nmi_watchdog: read_raw(&probe.roots, FILES[9]),
            kexec_load_disabled: read_raw(&probe.roots, FILES[10]),
            hung_task_panic: read_raw(&probe.roots, FILES[11]),
            hung_task_timeout_secs: read_raw(&probe.roots, FILES[12]),
            hung_task_check_interval_secs: read_raw(&probe.roots, FILES[13]),
            sysrq: read_raw(&probe.roots, FILES[14]),
            panic_on_oops: read_raw(&probe.roots, FILES[15]),
            unknown_nmi_panic: read_raw(&probe.roots, FILES[16]),
            kexec_crash_loaded: read_raw(&probe.roots, FILES[17]),
            kexec_crash_size: read_raw(&probe.roots, FILES[18]),
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
