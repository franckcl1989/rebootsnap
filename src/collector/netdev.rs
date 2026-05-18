use futures_util::StreamExt;
use netlink_packet_route::link::{LinkAttribute, LinkInfo};
use rtnetlink::new_connection;
use serde::Serialize;
use std::time::{Duration, Instant};

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Netdev;

#[derive(Serialize)]
struct NetdevRecord {
    schema_version: &'static str,
    collection: &'static str,
    interfaces: Vec<InterfaceInfo>,
    routes_v4: Option<String>,
    routes_v6: Option<String>,
    arp: Option<String>,
    netstat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip_addresses: Option<Vec<IpAddrEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tunnel_info: Option<Vec<TunnelEntry>>,
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

#[derive(Serialize)]
struct IpAddrEntry {
    ifindex: u32,
    family: u8,
    prefix_len: u8,
    scope: u8,
    flags: u8,
}

#[derive(Serialize)]
struct TunnelEntry {
    ifindex: u32,
    name: String,
    kind: String,
}

const PROBE_DIR: &str = "/sys/class/net";

const PROBE_FILES: &[&str] = &[
    "/proc/net/route",
    "/proc/net/ipv6_route",
    "/proc/net/arp",
    "/proc/net/netstat",
];

async fn query_ip_addresses() -> Option<Vec<IpAddrEntry>> {
    let (conn, handle, _) = new_connection().ok()?;
    tokio::spawn(conn);

    let mut entries = Vec::new();
    let mut stream = handle.address().get().execute();
    while let Some(Ok(msg)) = stream.next().await {
        let header = &msg.header;
        entries.push(IpAddrEntry {
            ifindex: header.index,
            family: u8::from(header.family),
            prefix_len: header.prefix_len,
            scope: u8::from(header.scope),
            flags: header.flags.bits(),
        });
        if entries.len() >= 200 {
            break;
        }
    }

    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

async fn query_tunnel_links() -> Option<Vec<TunnelEntry>> {
    let (conn, handle, _) = new_connection().ok()?;
    tokio::spawn(conn);

    let mut entries = Vec::new();
    let mut stream = handle.link().get().execute();
    while let Some(Ok(msg)) = stream.next().await {
        let name = msg
            .attributes
            .iter()
            .find_map(|attr| {
                if let LinkAttribute::IfName(n) = attr {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let kind: Option<String> = msg.attributes.iter().find_map(|attr| {
            if let LinkAttribute::LinkInfo(infos) = attr {
                infos.iter().find_map(|info| {
                    if let LinkInfo::Kind(k) = info {
                        Some(k.to_string())
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        });
        if let Some(ref k) = kind {
            let is_tunnel = matches!(
                k.as_str(),
                "sit" | "ipip" | "gre" | "ip6gre" | "ip6tnl" | "vti" | "xfrm" | "gretap" | "ip6gretap" | "vxlan" | "geneve"
            );
            if is_tunnel {
                entries.push(TunnelEntry {
                    ifindex: msg.header.index,
                    name,
                    kind: k.clone(),
                });
            }
        }
        if entries.len() >= 100 {
            break;
        }
    }

    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

impl Netdev {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        let has_ifs = match std::fs::read_dir(roots.resolve(PROBE_DIR)) {
            Ok(mut entries) => entries.next().is_some(),
            Err(_) => false,
        };
        let file_result = crate::collector::probe_files(roots, PROBE_FILES, "all net proc files missing");
        if !has_ifs && !file_result.available {
            return ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some("net sysfs and proc files unavailable".into()),
                roots: roots.clone(),
            };
        }
        let mut degraded: Vec<String> = if !has_ifs {
            vec!["sysfs interface dir empty/missing".into()]
        } else {
            Vec::new()
        };
        degraded.extend(file_result.degraded);
        ProbeOutcome {
            available: true,
            degraded,
            reason: None,
            roots: roots.clone(),
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
        if let Ok(dir) = std::fs::read_dir(probe.roots.resolve(PROBE_DIR)) {
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

        let iface_count = interfaces.len() as u64;

        let (ip_addrs, tunnels) = tokio::join!(
            tokio::time::timeout(Duration::from_secs(3), query_ip_addresses()),
            tokio::time::timeout(Duration::from_secs(3), query_tunnel_links()),
        );

        let record = NetdevRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-11",
            interfaces,
            routes_v4: read_trimmed(&probe.roots, "/proc/net/route"),
            routes_v6: read_trimmed(&probe.roots, "/proc/net/ipv6_route"),
            arp: read_trimmed(&probe.roots, "/proc/net/arp"),
            netstat: read_trimmed(&probe.roots, "/proc/net/netstat"),
            ip_addresses: ip_addrs.ok().flatten(),
            tunnel_info: tunnels.ok().flatten(),
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
                CollectionStatus::Partial {
                    degrading: degraded,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::setup_test;

    #[tokio::test]
    async fn test_collect_normal() {
        let ctx = setup_test("normal");
        let probe = Netdev.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Netdev probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Netdev.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
