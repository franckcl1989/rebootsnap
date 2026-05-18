use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Power;

#[derive(Serialize)]
struct PowerRecord {
    collection: &'static str,
    suspend_state: Option<String>,
    mem_sleep: Option<String>,
    acpi_wakeup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_freq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_throttle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thermal_zones: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edac_errors: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpuidle_states: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_boost: Option<String>,
}

const FILES: &[&str] = &[
    "/sys/power/state",
    "/sys/power/mem_sleep",
    "/proc/acpi/wakeup",
];

fn read_cpu_freq(roots: &FsRoots) -> Option<String> {
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
        entries.push((id, cpufreq));
    }
    if entries.is_empty() {
        return None;
    }
    entries.sort_by_key(|(id, _)| *id);
    let mut out = String::new();
    for (id, path) in &entries {
        out.push_str(&format!("cpu{}:\n", id));
        for f in &[
            "scaling_cur_freq",
            "scaling_governor",
            "scaling_max_freq",
            "scaling_min_freq",
            "cpuinfo_max_freq",
            "cpuinfo_min_freq",
        ] {
            let content = std::fs::read_to_string(path.join(f)).unwrap_or_default();
            out.push_str(&format!("{}: {}\n", f, content.trim()));
        }
    }
    Some(out)
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

fn read_thermal_zones(roots: &FsRoots) -> Option<String> {
    let thermal_dir = match std::fs::read_dir(roots.resolve("/sys/class/thermal")) {
        Ok(d) => d,
        Err(_) => return None,
    };
    let mut zone_entries = Vec::new();
    let mut cd_entries = Vec::new();
    for entry in thermal_dir.flatten() {
        if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("thermal_zone") {
            zone_entries.push((name, entry.path()));
        } else if name.starts_with("cooling_device") {
            cd_entries.push((name, entry.path()));
        }
    }
    if zone_entries.is_empty() && cd_entries.is_empty() {
        return None;
    }
    zone_entries.sort();
    cd_entries.sort();
    let mut out = String::new();
    for (name, path) in &zone_entries {
        out.push_str(&format!("{}:\n", name));
        for f in &["type", "temp", "mode", "policy"] {
            let content = std::fs::read_to_string(path.join(f)).unwrap_or_default();
            out.push_str(&format!("{}: {}\n", f, content.trim()));
        }
    }
    for (name, path) in &cd_entries {
        out.push_str(&format!("{}:\n", name));
        for f in &["type", "cur_state", "max_state"] {
            let content = std::fs::read_to_string(path.join(f)).unwrap_or_default();
            out.push_str(&format!("{}: {}\n", f, content.trim()));
        }
    }
    Some(out)
}

fn read_edac_errors(roots: &FsRoots) -> Option<String> {
    let edac_dir = std::fs::read_dir(roots.resolve("/sys/devices/system/edac/mc")).ok()?;
    let mut entries = Vec::new();
    for entry in edac_dir.flatten() {
        if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("mc") {
            entries.push((name, entry.path()));
        }
    }
    if entries.is_empty() {
        return None;
    }
    entries.sort();
    let mut out = String::new();
    for (name, path) in &entries {
        out.push_str(&format!("{}:\n", name));
        for f in &[
            "ce_count",
            "ue_count",
            "ce_noinfo_count",
            "ue_noinfo_count",
            "size_mb",
            "mc_name",
        ] {
            let content = std::fs::read_to_string(path.join(f)).unwrap_or_default();
            out.push_str(&format!("{}: {}\n", f, content.trim()));
        }
    }
    Some(out)
}

fn read_cpuidle(roots: &FsRoots) -> Option<String> {
    let cpu_dir = std::fs::read_dir(roots.resolve("/sys/devices/system/cpu")).ok()?;
    let mut cpu_entries: Vec<(u32, std::path::PathBuf)> = Vec::new();
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
        cpu_entries.push((id, cpuidle));
    }
    if cpu_entries.is_empty() {
        return None;
    }
    cpu_entries.sort_by_key(|(id, _)| *id);
    let mut out = String::new();
    for (cpu_id, path) in &cpu_entries {
        let state_dir = match std::fs::read_dir(path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let mut states: Vec<(String, std::path::PathBuf)> = Vec::new();
        for entry in state_dir.flatten() {
            if !entry.file_type().ok().is_some_and(|ft| ft.is_dir()) {
                continue;
            }
            let sname = entry.file_name().to_string_lossy().into_owned();
            if !sname.starts_with("state") {
                continue;
            }
            states.push((sname, entry.path()));
        }
        if states.is_empty() {
            continue;
        }
        states.sort();
        for (state_name, state_path) in &states {
            out.push_str(&format!("cpu{}/{}:\n", cpu_id, state_name));
            for f in &["name", "time", "usage", "above", "below"] {
                let content =
                    std::fs::read_to_string(state_path.join(f)).unwrap_or_default();
                out.push_str(&format!("{}: {}\n", f, content.trim()));
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
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

        fn read_raw(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path)).ok()
        }

        let record = PowerRecord {
            collection: "RT-18",
            suspend_state: read_raw(&probe.roots, FILES[0]),
            mem_sleep: read_raw(&probe.roots, FILES[1]),
            acpi_wakeup: read_raw(&probe.roots, FILES[2]),
            cpu_freq: read_cpu_freq(&probe.roots),
            cpu_throttle: read_cpu_throttle(&probe.roots),
            thermal_zones: read_thermal_zones(&probe.roots),
            edac_errors: read_edac_errors(&probe.roots),
            cpuidle_states: read_cpuidle(&probe.roots),
            cpu_boost: read_raw(&probe.roots, "/sys/devices/system/cpu/cpufreq/boost"),
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
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
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
