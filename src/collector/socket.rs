use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Socket;

#[derive(Serialize)]
struct SocketRecord {
    collection: &'static str,
    tcp: Option<String>,
    tcp6: Option<String>,
    udp: Option<String>,
    udp6: Option<String>,
    unix: Option<String>,
    raw: Option<String>,
    raw6: Option<String>,
    snmp: Option<String>,
    snmp6: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/net/tcp",
    "/proc/net/tcp6",
    "/proc/net/udp",
    "/proc/net/udp6",
    "/proc/net/unix",
    "/proc/net/raw",
    "/proc/net/raw6",
    "/proc/net/snmp",
    "/proc/net/snmp6",
];

impl Socket {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all net socket files missing")
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

        let record = SocketRecord {
            collection: "RT-12",
            tcp: read_raw(&probe.roots, FILES[0]),
            tcp6: read_raw(&probe.roots, FILES[1]),
            udp: read_raw(&probe.roots, FILES[2]),
            udp6: read_raw(&probe.roots, FILES[3]),
            unix: read_raw(&probe.roots, FILES[4]),
            raw: read_raw(&probe.roots, FILES[5]),
            raw6: read_raw(&probe.roots, FILES[6]),
            snmp: read_raw(&probe.roots, FILES[7]),
            snmp6: read_raw(&probe.roots, FILES[8]),
        };

        let writer = match output.json_writer("sockets.json") {
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
