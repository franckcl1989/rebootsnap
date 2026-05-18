use futures_util::StreamExt;
use serde::Serialize;
use std::time::{Duration, Instant};

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Netfilter;

#[derive(Serialize)]
struct NetfilterRecord {
    schema_version: &'static str,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    nftables_probe: Option<NftablesProbe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xfrm_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xfrm_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    qdisc_info: Option<Vec<QdiscEntry>>,
}

#[derive(Serialize)]
struct NftablesProbe {
    source: String,
    nl_family: String,
    status: String,
    note: String,
}

#[derive(Serialize)]
struct QdiscEntry {
    index: i32,
    handle: String,
    parent: String,
    info: u32,
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

async fn query_nftables_probe() -> Option<NftablesProbe> {
    use neli::consts::socket::NlFamily;
    use neli::socket::asynchronous::NlSocketHandle;
    use neli::utils::Groups;

    let _sock = NlSocketHandle::connect(NlFamily::Netfilter, None, Groups::empty()).ok()?;
    Some(NftablesProbe {
        source: "netlink".into(),
        nl_family: "NETLINK_NETFILTER".into(),
        status: "not_implemented".into(),
        note: "nftables netlink query connected; full rule dump pending neli 0.7 nftnl integration".into(),
    })
}

async fn query_xfrm() -> Option<String> {
    None
}

async fn query_qdisc() -> Option<Vec<QdiscEntry>> {
    let (conn, handle, _) = rtnetlink::new_connection().ok()?;
    tokio::spawn(conn);

    let mut entries = Vec::new();
    let mut stream = handle.qdisc().get().execute();
    while let Some(Ok(msg)) = stream.next().await {
        entries.push(QdiscEntry {
            index: msg.header.index,
            handle: format!("{}", msg.header.handle),
            parent: format!("{}", msg.header.parent),
            info: msg.header.info,
        });
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

        let nftables_probe = tokio::time::timeout(
            Duration::from_secs(3),
            query_nftables_probe(),
        )
        .await
        .ok()
        .flatten();

        let xfrm_state = tokio::time::timeout(
            Duration::from_secs(3),
            query_xfrm(),
        )
        .await
        .ok()
        .flatten();

        let xfrm_policy = tokio::time::timeout(
            Duration::from_secs(3),
            query_xfrm(),
        )
        .await
        .ok()
        .flatten();

        let qdisc_info = tokio::time::timeout(
            Duration::from_secs(3),
            query_qdisc(),
        )
        .await
        .ok()
        .flatten();

        let record = NetfilterRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-13",
            conntrack: read_trimmed(&probe.roots, FILES[0]),
            conntrack_stats: read_trimmed(&probe.roots, FILES[1]),
            nf_conntrack_max: read_trimmed(&probe.roots, FILES[2]),
            nf_tables_names: read_trimmed(&probe.roots, FILES[3]),
            xfrm_stat: read_trimmed(&probe.roots, FILES[4]),
            ip_tables_names: read_trimmed(&probe.roots, FILES[5]),
            ip6_tables_names: read_trimmed(&probe.roots, FILES[6]),
            arp_tables_names: read_trimmed(&probe.roots, FILES[7]),
            ip_tables_matches: read_trimmed(&probe.roots, FILES[8]),
            ip_tables_targets: read_trimmed(&probe.roots, FILES[9]),
            ebtables_names: read_trimmed(&probe.roots, "/proc/net/ebtables_names"),
            nftables_probe,
            xfrm_state,
            xfrm_policy,
            qdisc_info,
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
                CollectionStatus::Partial {
                    degrading: probe.degraded.clone(),
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
        let probe = Netfilter.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Netfilter probe unavailable in mock - skipping");
            return;
        }
        let outcome = Netfilter.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
