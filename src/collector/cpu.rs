use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Cpu;

#[derive(Serialize)]
struct VulnEntry {
    name: String,
    status: String,
}

#[derive(Serialize)]
struct CpuTopologyEntry {
    id: u32,
    online: bool,
    core_id: Option<String>,
    thread_siblings: Option<String>,
}

#[derive(Serialize)]
struct CpuRecord {
    schema_version: &'static str,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    vulnerabilities: Option<Vec<VulnEntry>>,
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

const UNSUPPORTED_IF_MISSING: &[&str] = &[
    "/proc/pressure/cpu",
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
                    let online = read_trimmed(
                        &probe.roots,
                        &format!("{}/cpu{}/online", cpu_base, id),
                    )
                    .map(|s| s == "1")
                    .unwrap_or(false);
                    let core_id = read_trimmed(
                        &probe.roots,
                        &format!("{}/cpu{}/topology/core_id", cpu_base, id),
                    );
                    let thread_siblings = read_trimmed(
                        &probe.roots,
                        &format!("{}/cpu{}/topology/thread_siblings_list", cpu_base, id),
                    );
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
            schema_version: SCHEMA_VERSION,
            collection: "RT-05",
            stat: read_trimmed(&probe.roots, FILES[0]),
            loadavg: read_trimmed(&probe.roots, FILES[1]),
            pressure_cpu: read_trimmed(&probe.roots, FILES[2]),
            interrupts: read_trimmed(&probe.roots, FILES[3]),
            softirqs: read_trimmed(&probe.roots, FILES[4]),
            cpu_topology,
            cpu_isolated: read_trimmed(&probe.roots, FILES[5]),
            smt_control: read_trimmed(&probe.roots, FILES[6]),
            nohz_full: read_trimmed(&probe.roots, FILES[7]),
            vulnerabilities: {
                let vuln_dir = probe.roots.resolve("/sys/devices/system/cpu/vulnerabilities");
                let mut vulns = Vec::new();
                if let Ok(dir) = std::fs::read_dir(&vuln_dir) {
                    for entry in dir.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let status = std::fs::read_to_string(entry.path())
                            .ok()
                            .map(|s| s.trim().to_string());
                        if let Some(status) = status {
                            vulns.push(VulnEntry { name, status });
                        }
                    }
                }
                if vulns.is_empty() { None } else { Some(vulns) }
            },
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
        let probe = Cpu.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Cpu probe unavailable in mock - skipping");
            return;
        }
        let outcome = Cpu.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }

    #[tokio::test]
    async fn test_collect_content() {
        let ctx = setup_test("normal");
        let probe = Cpu.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Cpu.collect(&ctx.output, &probe).await;
        assert!(outcome.file_size > 0);

        let output_dir = ctx.output.root();
        let entries: Vec<_> = std::fs::read_dir(output_dir).expect("read_dir")
            .filter_map(|e| e.ok()).collect();
        assert!(!entries.is_empty(), "should have output files");

        let content = std::fs::read_to_string(entries[0].path()).expect("read output");
        let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(json["collection"], "RT-05");
        assert!(json["stat"].is_string(), "stat should be present");
    }
}
