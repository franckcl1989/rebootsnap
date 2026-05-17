use serde::Serialize;
use std::time::Instant;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::output::OutputDir;

pub struct Netdev;

#[derive(Serialize)]
struct NetdevRecord {
    collection: &'static str,
    interfaces: Vec<InterfaceInfo>,
    routes_v4: Option<String>,
    routes_v6: Option<String>,
    arp: Option<String>,
    netstat: Option<String>,
}

#[derive(Serialize, Clone)]
struct InterfaceInfo {
    name: String,
    ifindex: Option<i32>,
    address: Option<String>,
    mtu: Option<String>,
    operstate: Option<String>,
    carrier: Option<String>,
    flags: Option<String>,
    stats_rx_bytes: Option<u64>,
    stats_tx_bytes: Option<u64>,
    stats_rx_packets: Option<u64>,
    stats_tx_packets: Option<u64>,
    stats_rx_errors: Option<u64>,
    stats_tx_errors: Option<u64>,
    stats_rx_dropped: Option<u64>,
    stats_tx_dropped: Option<u64>,
}

const PROBE_DIR: &str = "/sys/class/net";

impl Netdev {
    pub async fn probe(&self) -> ProbeOutcome {
        match std::fs::read_dir(PROBE_DIR) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    ProbeOutcome {
                        available: true,
                        degraded: Vec::new(),
                        reason: None,
                    }
                } else {
                    ProbeOutcome {
                        available: false,
                        degraded: Vec::new(),
                        reason: Some("no network interfaces found".into()),
                    }
                }
            }
            Err(_) => ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("net sysfs unavailable".into()),
            },
        }
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

        fn read_file(path: &std::path::Path) -> Option<String> {
            std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
        }

        fn read_u64(path: &std::path::Path) -> Option<u64> {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|s| s.trim().parse().ok())
        }

        let mut interfaces = Vec::new();
        if let Ok(dir) = std::fs::read_dir(PROBE_DIR) {
            for entry in dir.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                if name.is_empty() {
                    continue;
                }
                interfaces.push(InterfaceInfo {
                    name,
                    ifindex: read_file(&path.join("ifindex"))
                        .and_then(|s| s.parse().ok()),
                    address: read_file(&path.join("address")),
                    mtu: read_file(&path.join("mtu")),
                    operstate: read_file(&path.join("operstate")),
                    carrier: read_file(&path.join("carrier")),
                    flags: read_file(&path.join("flags")),
                    stats_rx_bytes: read_u64(&path.join("statistics/rx_bytes")),
                    stats_tx_bytes: read_u64(&path.join("statistics/tx_bytes")),
                    stats_rx_packets: read_u64(&path.join("statistics/rx_packets")),
                    stats_tx_packets: read_u64(&path.join("statistics/tx_packets")),
                    stats_rx_errors: read_u64(&path.join("statistics/rx_errors")),
                    stats_tx_errors: read_u64(&path.join("statistics/tx_errors")),
                    stats_rx_dropped: read_u64(&path.join("statistics/rx_dropped")),
                    stats_tx_dropped: read_u64(&path.join("statistics/tx_dropped")),
                });
            }
        }

        fn raw(path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }

        let iface_count = interfaces.len() as u64;

        let record = NetdevRecord {
            collection: "RT-11",
            interfaces,
            routes_v4: raw("/proc/net/route"),
            routes_v6: raw("/proc/net/ipv6_route"),
            arp: raw("/proc/net/arp"),
            netstat: raw("/proc/net/netstat"),
        };

        let writer = match output.json_writer("netdev.json") {
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
        if iface_count == 0 {
            degraded.push("no interfaces read from sysfs".into());
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
            items_collected: Some(iface_count),
            mem_total_kb: None,
            mem_available_kb: None,
            hostname: None,
            kernel_version: None,
            boot_id: None,
            uptime_seconds: None,
        }
    }
}
