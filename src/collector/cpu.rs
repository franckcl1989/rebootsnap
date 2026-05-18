use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Cpu;

#[derive(Serialize)]
struct CpuTopologyEntry {
    id: u32,
    online: bool,
    core_id: Option<String>,
    thread_siblings: Option<String>,
}

#[derive(Serialize)]
struct CpuRecord {
    collection: &'static str,
    stat: Option<String>,
    loadavg: Option<String>,
    pressure_cpu: Option<String>,
    interrupts: Option<String>,
    softirqs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_topology: Option<Vec<CpuTopologyEntry>>,
    cpu_isolated: Option<String>,
    smt_control: Option<String>,
    nohz_full: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/stat",
    "/proc/loadavg",
    "/proc/pressure/cpu",
    "/proc/interrupts",
    "/proc/softirqs",
    "/sys/devices/system/cpu/isolated",
    "/sys/devices/system/cpu/smt/control",
    "/sys/devices/system/cpu/nohz_full",
];

impl Cpu {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all cpu files missing")
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

        let cpu_topology = {
            let cpu_base = "/sys/devices/system/cpu";
            let cpu_dir = probe.roots.resolve(cpu_base);
            let mut entries = Vec::new();
            if let Ok(dir) = std::fs::read_dir(&cpu_dir) {
                for entry in dir.flatten() {
                    let name_str = entry.file_name().to_string_lossy().into_owned();
                    if !name_str.starts_with("cpu") {
                        continue;
                    }
                    let id: u32 = match name_str[3..].parse() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let online = read_raw(
                        &probe.roots,
                        &format!("{}/cpu{}/online", cpu_base, id),
                    )
                    .map(|s| s.trim() == "1")
                    .unwrap_or(false);
                    let core_id = read_raw(
                        &probe.roots,
                        &format!("{}/cpu{}/topology/core_id", cpu_base, id),
                    )
                    .map(|s| s.trim().to_string());
                    let thread_siblings = read_raw(
                        &probe.roots,
                        &format!("{}/cpu{}/topology/thread_siblings_list", cpu_base, id),
                    )
                    .map(|s| s.trim().to_string());
                    entries.push(CpuTopologyEntry {
                        id,
                        online,
                        core_id,
                        thread_siblings,
                    });
                }
            }
            if entries.is_empty() {
                None
            } else {
                Some(entries)
            }
        };

        let record = CpuRecord {
            collection: "RT-05",
            stat: read_raw(&probe.roots, FILES[0]),
            loadavg: read_raw(&probe.roots, FILES[1]),
            pressure_cpu: read_raw(&probe.roots, FILES[2]),
            interrupts: read_raw(&probe.roots, FILES[3]),
            softirqs: read_raw(&probe.roots, FILES[4]),
            cpu_topology,
            cpu_isolated: read_raw(&probe.roots, FILES[5]),
            smt_control: read_raw(&probe.roots, FILES[6]),
            nohz_full: read_raw(&probe.roots, FILES[7]),
        };

        let writer = match output.json_writer("cpu.json") {
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
