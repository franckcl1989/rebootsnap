use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Power;

#[derive(Serialize)]
struct CpuFreqEntry {
    id: u32,
    scaling_cur_freq: Option<String>,
    scaling_governor: Option<String>,
    scaling_max_freq: Option<String>,
    scaling_min_freq: Option<String>,
    cpuinfo_max_freq: Option<String>,
    cpuinfo_min_freq: Option<String>,
}

#[derive(Serialize)]
struct ThermalZoneEntry {
    name: String,
    zone_type: Option<String>,
    temp: Option<String>,
    mode: Option<String>,
    policy: Option<String>,
}

#[derive(Serialize)]
struct CoolingDeviceEntry {
    name: String,
    device_type: Option<String>,
    cur_state: Option<String>,
    max_state: Option<String>,
}

#[derive(Serialize)]
struct EdacEntry {
    name: String,
    ce_count: Option<String>,
    ue_count: Option<String>,
    ce_noinfo_count: Option<String>,
    ue_noinfo_count: Option<String>,
    size_mb: Option<String>,
    mc_name: Option<String>,
}

#[derive(Serialize)]
struct CpuIdleState {
    name: String,
    desc: Option<String>,
    time: Option<String>,
    usage: Option<String>,
}

#[derive(Serialize)]
struct CpuIdleEntry {
    cpu_id: u32,
    states: Vec<CpuIdleState>,
}

#[derive(Serialize)]
struct PowerRecord {
    schema_version: &'static str,
    collection: &'static str,
    suspend_state: Option<String>,
    mem_sleep: Option<String>,
    acpi_wakeup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_freq: Option<Vec<CpuFreqEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_throttle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thermal_zones: Option<Vec<ThermalZoneEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cooling_devices: Option<Vec<CoolingDeviceEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edac_errors: Option<Vec<EdacEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpuidle_states: Option<Vec<CpuIdleEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_boost: Option<String>,
}

const FILES: &[&str] = &[
    "/sys/power/state",
    "/sys/power/mem_sleep",
    "/proc/acpi/wakeup",
];

const UNSUPPORTED_IF_MISSING: &[&str] = &[];

fn read_cpu_freq(roots: &FsRoots) -> Option<Vec<CpuFreqEntry>> {
    let cpu_dir = std::fs::read_dir(roots.resolve("/sys/devices/system/cpu")).ok()?;
    let mut entries = Vec::new();
    for entry in cpu_dir.flatten() {
        if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("cpu") {
            continue;
        }
        let id: u32 = match name[3..].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let cpufreq = entry.path().join("cpufreq");
        if !cpufreq.is_dir() {
            continue;
        }
        entries.push(CpuFreqEntry {
            id,
            scaling_cur_freq: std::fs::read_to_string(cpufreq.join("scaling_cur_freq")).ok().map(|s| s.trim().to_string()),
            scaling_governor: std::fs::read_to_string(cpufreq.join("scaling_governor")).ok().map(|s| s.trim().to_string()),
            scaling_max_freq: std::fs::read_to_string(cpufreq.join("scaling_max_freq")).ok().map(|s| s.trim().to_string()),
            scaling_min_freq: std::fs::read_to_string(cpufreq.join("scaling_min_freq")).ok().map(|s| s.trim().to_string()),
            cpuinfo_max_freq: std::fs::read_to_string(cpufreq.join("cpuinfo_max_freq")).ok().map(|s| s.trim().to_string()),
            cpuinfo_min_freq: std::fs::read_to_string(cpufreq.join("cpuinfo_min_freq")).ok().map(|s| s.trim().to_string()),
        });
    }
    if entries.is_empty() { None } else { Some(entries) }
}

fn read_cpu_throttle(roots: &FsRoots) -> Option<String> {
    let cpu_dir = std::fs::read_dir(roots.resolve("/sys/devices/system/cpu")).ok()?;
    let mut entries = Vec::new();
    for entry in cpu_dir.flatten() {
        if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("cpu") {
            continue;
        }
        let id: u32 = match name[3..].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let throttle = entry.path().join("thermal_throttle");
        if !throttle.is_dir() {
            continue;
        }
        entries.push((id, throttle));
    }
    if entries.is_empty() {
        return None;
    }
    entries.sort_by_key(|(id, _)| *id);
    let mut out = String::new();
    for (id, path) in &entries {
        out.push_str(&format!("cpu{}:\n", id));
        for f in &["core_throttle_count", "package_throttle_count"] {
            let content = std::fs::read_to_string(path.join(f)).unwrap_or_default();
            out.push_str(&format!("{}: {}\n", f, content.trim()));
        }
    }
    Some(out)
}

fn read_thermal_zones(roots: &FsRoots) -> (Option<Vec<ThermalZoneEntry>>, Option<Vec<CoolingDeviceEntry>>) {
    let thermal_dir = match std::fs::read_dir(roots.resolve("/sys/class/thermal")) {
        Ok(d) => d,
        Err(_) => return (None, None),
    };
    let mut zones = Vec::new();
    let mut cds = Vec::new();
    for entry in thermal_dir.flatten() {
        if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("thermal_zone") {
            zones.push(ThermalZoneEntry {
                name,
                zone_type: std::fs::read_to_string(entry.path().join("type")).ok().map(|s| s.trim().to_string()),
                temp: std::fs::read_to_string(entry.path().join("temp")).ok().map(|s| s.trim().to_string()),
                mode: std::fs::read_to_string(entry.path().join("mode")).ok().map(|s| s.trim().to_string()),
                policy: std::fs::read_to_string(entry.path().join("policy")).ok().map(|s| s.trim().to_string()),
            });
        } else if name.starts_with("cooling_device") {
            cds.push(CoolingDeviceEntry {
                name,
                device_type: std::fs::read_to_string(entry.path().join("type")).ok().map(|s| s.trim().to_string()),
                cur_state: std::fs::read_to_string(entry.path().join("cur_state")).ok().map(|s| s.trim().to_string()),
                max_state: std::fs::read_to_string(entry.path().join("max_state")).ok().map(|s| s.trim().to_string()),
            });
        }
    }
    (
        if zones.is_empty() { None } else { Some(zones) },
        if cds.is_empty() { None } else { Some(cds) },
    )
}

fn read_edac_errors(roots: &FsRoots) -> Option<Vec<EdacEntry>> {
    let edac_dir = std::fs::read_dir(roots.resolve("/sys/devices/system/edac/mc")).ok()?;
    let mut entries = Vec::new();
    for entry in edac_dir.flatten() {
        if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("mc") {
            entries.push(EdacEntry {
                name: name.clone(),
                ce_count: std::fs::read_to_string(entry.path().join("ce_count")).ok().map(|s| s.trim().to_string()),
                ue_count: std::fs::read_to_string(entry.path().join("ue_count")).ok().map(|s| s.trim().to_string()),
                ce_noinfo_count: std::fs::read_to_string(entry.path().join("ce_noinfo_count")).ok().map(|s| s.trim().to_string()),
                ue_noinfo_count: std::fs::read_to_string(entry.path().join("ue_noinfo_count")).ok().map(|s| s.trim().to_string()),
                size_mb: std::fs::read_to_string(entry.path().join("size_mb")).ok().map(|s| s.trim().to_string()),
                mc_name: std::fs::read_to_string(entry.path().join("mc_name")).ok().map(|s| s.trim().to_string()),
            });
        }
    }
    if entries.is_empty() { None } else { Some(entries) }
}

fn read_cpuidle(roots: &FsRoots) -> Option<Vec<CpuIdleEntry>> {
    let cpu_dir = std::fs::read_dir(roots.resolve("/sys/devices/system/cpu")).ok()?;
    let mut cpu_entries: Vec<CpuIdleEntry> = Vec::new();
    for entry in cpu_dir.flatten() {
        if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("cpu") {
            continue;
        }
        let id: u32 = match name[3..].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let cpuidle = entry.path().join("cpuidle");
        if !cpuidle.is_dir() {
            continue;
        }
        let state_dir = match std::fs::read_dir(&cpuidle) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let mut states = Vec::new();
        for st_entry in state_dir.flatten() {
            if !st_entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
                continue;
            }
            let sname = st_entry.file_name().to_string_lossy().into_owned();
            if !sname.starts_with("state") {
                continue;
            }
            states.push(CpuIdleState {
                name: sname,
                desc: std::fs::read_to_string(st_entry.path().join("name")).ok().map(|s| s.trim().to_string()),
                time: std::fs::read_to_string(st_entry.path().join("time")).ok().map(|s| s.trim().to_string()),
                usage: std::fs::read_to_string(st_entry.path().join("usage")).ok().map(|s| s.trim().to_string()),
            });
        }
        if !states.is_empty() {
            cpu_entries.push(CpuIdleEntry { cpu_id: id, states });
        }
    }
    if cpu_entries.is_empty() { None } else { Some(cpu_entries) }
}

impl Power {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all power files missing")
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

        let (thermal_zones, cooling_devices) = read_thermal_zones(&probe.roots);

        let record = PowerRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-18",
            suspend_state: read_trimmed(&probe.roots, FILES[0]),
            mem_sleep: read_trimmed(&probe.roots, FILES[1]),
            acpi_wakeup: read_trimmed(&probe.roots, FILES[2]),
            cpu_freq: read_cpu_freq(&probe.roots),
            cpu_throttle: read_cpu_throttle(&probe.roots),
            thermal_zones,
            cooling_devices,
            edac_errors: read_edac_errors(&probe.roots),
            cpuidle_states: read_cpuidle(&probe.roots),
            cpu_boost: read_trimmed(&probe.roots, "/sys/devices/system/cpu/cpufreq/boost"),
        };

        let writer = match output.json_writer("power.json") {
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
                CollectionStatus::Partial {
                    degrading: degraded,
                }
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
        let probe = Power.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Power probe unavailable in mock - skipping");
            return;
        }
        let outcome = Power.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }

    #[tokio::test]
    async fn test_collect_content() {
        let ctx = setup_test("normal");
        let probe = Power.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Power.collect(&ctx.output, &probe).await;
        assert!(outcome.file_size > 0);

        let output_dir = ctx.output.root();
        let entries: Vec<_> = std::fs::read_dir(output_dir).expect("read_dir")
            .filter_map(|e| e.ok()).collect();
        assert!(!entries.is_empty(), "should have output files");

        let content = std::fs::read_to_string(entries[0].path()).expect("read output");
        let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(json["collection"], "RT-18");
        assert!(json["suspend_state"].is_string(), "suspend_state should be present");
    }
}
