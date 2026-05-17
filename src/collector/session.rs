use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Session;

const UTMP_RECORD_SIZE: usize = 384;

#[derive(Serialize)]
struct UtmpEntry {
    ut_type: i16,
    ut_pid: i32,
    ut_line: String,
    ut_user: String,
    ut_host: String,
    ut_tv_sec: i32,
    ut_tv_usec: i32,
}

#[derive(Serialize)]
struct SessionRecord {
    collection: &'static str,
    utmp_size: Option<u64>,
    wtmp_size: Option<u64>,
    btmp_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    utmp_entries: Option<Vec<UtmpEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wtmp_entries: Option<Vec<UtmpEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    btmp_entries: Option<Vec<UtmpEntry>>,
}

const FILES: &[&str] = &["/var/run/utmp", "/var/log/wtmp", "/var/log/btmp"];

fn parse_utmp_record(data: &[u8]) -> UtmpEntry {
    let ut_type = i32::from_ne_bytes(data[0..4].try_into().unwrap_or_default()) as i16;
    let ut_pid = i32::from_ne_bytes(data[4..8].try_into().unwrap_or_default());
    let ut_line = String::from_utf8_lossy(&data[8..40]).trim_end_matches('\0').to_string();
    let ut_user = String::from_utf8_lossy(&data[44..76]).trim_end_matches('\0').to_string();
    let ut_host = String::from_utf8_lossy(&data[76..332]).trim_end_matches('\0').to_string();
    let ut_tv_sec = i32::from_ne_bytes(data[340..344].try_into().unwrap_or_default());
    let ut_tv_usec = i32::from_ne_bytes(data[344..348].try_into().unwrap_or_default());
    UtmpEntry {
        ut_type,
        ut_pid,
        ut_line,
        ut_user,
        ut_host,
        ut_tv_sec,
        ut_tv_usec,
    }
}

fn read_utmp_entries(roots: &FsRoots, path: &str) -> Option<Vec<UtmpEntry>> {
    let data = std::fs::read(roots.resolve(path)).ok()?;
    let mut entries = Vec::new();
    for chunk in data.chunks(UTMP_RECORD_SIZE) {
        if chunk.len() < 48 {
            break;
        }
        let entry = parse_utmp_record(chunk);
        if entry.ut_user.is_empty() && entry.ut_line.is_empty() {
            continue;
        }
        entries.push(entry);
        if entries.len() >= 1000 {
            break;
        }
    }
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

impl Session {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all session files missing")
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

        fn read_size(roots: &FsRoots, path: &str) -> Option<u64> {
            std::fs::metadata(roots.resolve(path)).ok().map(|m| m.len())
        }

        let utmp_entries = read_utmp_entries(&probe.roots, FILES[0]);
        let wtmp_entries = read_utmp_entries(&probe.roots, FILES[1]);
        let btmp_entries = read_utmp_entries(&probe.roots, FILES[2]);

        let items_collected = utmp_entries.as_ref().map_or(0, |v| v.len())
            + wtmp_entries.as_ref().map_or(0, |v| v.len())
            + btmp_entries.as_ref().map_or(0, |v| v.len());

        let record = SessionRecord {
            collection: "RT-15",
            utmp_size: read_size(&probe.roots, FILES[0]),
            wtmp_size: read_size(&probe.roots, FILES[1]),
            btmp_size: read_size(&probe.roots, FILES[2]),
            utmp_entries,
            wtmp_entries,
            btmp_entries,
        };

        let writer = match output.json_writer("sessions.json") {
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
            items_collected: Some(items_collected as u64),
            mem_total_kb: None,
            mem_available_kb: None,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
