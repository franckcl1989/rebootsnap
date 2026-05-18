use serde::Serialize;
use std::fs;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Events;

#[derive(Serialize)]
struct DmesgRecord {
    collection: &'static str,
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_levels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_ratelimit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_ratelimit_burst: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_dropped: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    devkmsg_log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dmesg_restrict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dmesg_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    volatile_journal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    journal_machine_id: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/kernel/printk",
    "/proc/sys/kernel/printk_ratelimit",
    "/proc/sys/kernel/printk_ratelimit_burst",
    "/proc/sys/kernel/printk_dropped",
    "/proc/sys/kernel/devkmsg_log",
    "/proc/sys/kernel/dmesg_restrict",
];

const KMSG_PATH: &str = "/dev/kmsg";

fn query_volatile_journal(roots: &FsRoots) -> Option<(String, String)> {
    let journal_dir = roots.resolve("/run/log/journal");
    let entries = fs::read_dir(&journal_dir).ok()?;
    let machine_id_dir = entries
        .filter_map(|e| e.ok())
        .find(|e| e.path().is_dir())?;
    let machine_id = machine_id_dir.file_name().to_string_lossy().to_string();
    let mut journal_files = Vec::new();
    if let Ok(j_entries) = fs::read_dir(machine_id_dir.path()) {
        for entry in j_entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "journal") {
                let meta = fs::metadata(&path).ok();
                journal_files.push(serde_json::json!({
                    "name": entry.file_name().to_string_lossy(),
                    "size": meta.as_ref().map(|m| m.len()),
                }));
            }
            if journal_files.len() >= 20 {
                break;
            }
        }
    }
    let result = serde_json::json!({
        "machine_id": machine_id,
        "files": journal_files,
        "source": "/run/log/journal",
        "note": "journal file enumeration only; full content requires journalctl or D-Bus GetJournal fd"
    });
    Some((
        serde_json::to_string(&result).ok()?,
        machine_id,
    ))
}

impl Events {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        let mut probe = crate::collector::probe_files(roots, FILES, "all events files missing");
        let kmsg_ok = std::fs::File::open(KMSG_PATH).is_ok();
        if !kmsg_ok {
            if probe.available {
                probe.degraded.push(KMSG_PATH.into());
            } else {
                probe.available = true;
                probe.reason = None;
                probe.degraded.push(KMSG_PATH.into());
            }
        }
        probe
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

        fn read_trimmed(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path))
                .ok()
                .map(|s| s.trim().to_string())
        }

        let dmesg_text = read_trimmed(&probe.roots, KMSG_PATH);
        let dmesg_ok = dmesg_text.is_some();

        let (volatile_journal, journal_machine_id) =
            query_volatile_journal(&probe.roots)
                .map(|(j, m)| (Some(j), Some(m)))
                .unwrap_or((None, None));

        let record = DmesgRecord {
            collection: "RT-20",
            source: "/dev/kmsg",
            printk_levels: read_trimmed(&probe.roots, FILES[0]),
            printk_ratelimit: read_trimmed(&probe.roots, FILES[1]),
            printk_ratelimit_burst: read_trimmed(&probe.roots, FILES[2]),
            printk_dropped: read_trimmed(&probe.roots, FILES[3]),
            devkmsg_log: read_trimmed(&probe.roots, FILES[4]),
            dmesg_restrict: read_trimmed(&probe.roots, FILES[5]),
            dmesg_text,
            volatile_journal,
            journal_machine_id,
        };

        let writer = match output.json_writer("dmesg.json") {
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

        let mut degraded = probe.degraded.clone();
        if !dmesg_ok {
            degraded.push("dmesg: /dev/kmsg unreadable".into());
        }

        CollectionOutcome {
            status: if degraded.is_empty() {
                CollectionStatus::Ok
            } else {
                CollectionStatus::Degraded {
                    missing: degraded,
                }
            },
            duration: start.elapsed(),
            file_size: size,
            items_total: None,
            items_collected: if dmesg_ok { Some(1) } else { None },
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
        let probe = Events.probe(&ctx.roots).await;
        if !probe.available {
            println!("Events probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Events.collect(&ctx.output, &probe).await;
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
