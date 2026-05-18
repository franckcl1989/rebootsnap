use serde::Serialize;
use std::path::Path;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Device;

#[derive(Serialize)]
struct DeviceRecord {
    collection: &'static str,
    devices: Option<String>,
    misc: Option<String>,
    kernel_debug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sysfs_devices: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_devices: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interrupts: Option<String>,
}

#[derive(Serialize)]
struct SysfsDevice {
    path: String,
    uevent: Option<String>,
    driver: Option<String>,
    modalias: Option<String>,
    device_state: Option<String>,
    errors: Option<String>,
    removable: Option<String>,
    reset: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/devices",
    "/proc/misc",
    "/sys/kernel/debug",
    "/proc/bus/input/devices",
    "/proc/interrupts",
];

fn read_raw(roots: &FsRoots, path: &str) -> Option<String> {
    std::fs::read_to_string(roots.resolve(path)).ok()
}

fn walk_devices(dir: &Path, depth: u32, max_depth: u32, results: &mut Vec<SysfsDevice>) {
    if depth > max_depth || results.len() >= 100 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let info = SysfsDevice {
                path: path.display().to_string(),
                uevent: std::fs::read_to_string(path.join("uevent")).ok(),
                driver: std::fs::read_link(path.join("driver"))
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
                modalias: std::fs::read_to_string(path.join("modalias")).ok(),
                device_state: std::fs::read_to_string(path.join("state")).ok(),
                errors: std::fs::read_to_string(path.join("errors")).ok(),
                removable: std::fs::read_to_string(path.join("removable")).ok(),
                reset: std::fs::read_to_string(path.join("reset")).ok(),
            };
            if info.uevent.is_some() || info.driver.is_some() {
                results.push(info);
            }
            walk_devices(&path, depth + 1, max_depth, results);
        }
    }
}

fn enumerate_sysfs_devices(roots: &FsRoots) -> Option<String> {
    let devices_dir = roots.resolve("/sys/devices");
    let mut results: Vec<SysfsDevice> = Vec::new();

    let entries = match std::fs::read_dir(&devices_dir) {
        Ok(e) => e,
        Err(_) => return None,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_devices(&path, 0, 3, &mut results);
            if results.len() >= 100 {
                break;
            }
        }
    }

    if results.is_empty() {
        return None;
    }

    serde_json::to_string(&results).ok()
}

impl Device {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all device files missing")
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

        let record = DeviceRecord {
            collection: "RT-17",
            devices: read_raw(&probe.roots, FILES[0]),
            misc: read_raw(&probe.roots, FILES[1]),
            kernel_debug: if std::fs::read_dir(probe.roots.resolve(FILES[2]))
                .ok()
                .is_some_and(|mut d| d.next().is_some())
            {
                Some("mounted".to_string())
            } else {
                None
            },
            sysfs_devices: enumerate_sysfs_devices(&probe.roots),
            input_devices: read_raw(&probe.roots, FILES[3]),
            interrupts: read_raw(&probe.roots, FILES[4]),
        };

        let writer = match output.json_writer("devices.json") {
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
        let probe = Device.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Device probe unavailable in mock - skipping");
            return;
        }
        let outcome = Device.collect(&ctx.output, &probe).await;
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
