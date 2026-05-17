use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Netfilter;

#[derive(Serialize)]
struct NetfilterRecord {
    collection: &'static str,
    conntrack: Option<String>,
    conntrack_stats: Option<String>,
    nf_conntrack_max: Option<String>,
    nf_tables_names: Option<String>,
    xfrm_stat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip_tables_names: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip6_tables_names: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arp_tables_names: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip_tables_matches: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip_tables_targets: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ebtables_names: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/net/nf_conntrack",
    "/proc/net/stat/nf_conntrack",
    "/proc/sys/net/nf_conntrack_max",
    "/proc/net/nf_tables_names",
    "/proc/net/xfrm_stat",
    "/proc/net/ip_tables_names",
    "/proc/net/ip6_tables_names",
    "/proc/net/arp_tables_names",
    "/proc/net/ip_tables_matches",
    "/proc/net/ip_tables_targets",
];

impl Netfilter {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all netfilter files missing")
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

        let record = NetfilterRecord {
            collection: "RT-13",
            conntrack: read_raw(&probe.roots, FILES[0]),
            conntrack_stats: read_raw(&probe.roots, FILES[1]),
            nf_conntrack_max: read_raw(&probe.roots, FILES[2]),
            nf_tables_names: read_raw(&probe.roots, FILES[3]),
            xfrm_stat: read_raw(&probe.roots, FILES[4]),
            ip_tables_names: read_raw(&probe.roots, FILES[5]),
            ip6_tables_names: read_raw(&probe.roots, FILES[6]),
            arp_tables_names: read_raw(&probe.roots, FILES[7]),
            ip_tables_matches: read_raw(&probe.roots, FILES[8]),
            ip_tables_targets: read_raw(&probe.roots, FILES[9]),
            ebtables_names: read_raw(&probe.roots, "/proc/net/ebtables_names"),
        };

        let writer = match output.json_writer("netfilter.json") {
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
