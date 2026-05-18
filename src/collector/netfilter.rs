use futures_util::StreamExt;
use serde::Serialize;
use std::time::{Duration, Instant};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    nftables_rules: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xfrm_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xfrm_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    qdisc_info: Option<String>,
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

async fn query_nftables_rules() -> Option<String> {
    use neli::consts::socket::NlFamily;
    use neli::socket::asynchronous::NlSocketHandle;
    use neli::utils::Groups;

    let _sock = NlSocketHandle::connect(NlFamily::Netfilter, None, Groups::empty()).ok()?;
    let result = serde_json::json!({
        "source": "netlink",
        "nl_family": "NETLINK_NETFILTER",
        "note": "nftables netlink query connected; full rule dump pending neli 0.7 nftnl integration"
    });
    serde_json::to_string(&result).ok()
}

async fn query_xfrm() -> Option<String> {
    None
}

async fn query_qdisc() -> Option<String> {
    let (conn, handle, _) = rtnetlink::new_connection().ok()?;
    tokio::spawn(conn);

    let mut entries = Vec::new();
    let mut stream = handle.qdisc().get().execute();
    while let Some(Ok(msg)) = stream.next().await {
        entries.push(serde_json::json!({
            "index": msg.header.index,
            "handle": format!("{}", msg.header.handle),
            "parent": format!("{}", msg.header.parent),
            "info": msg.header.info,
        }));
        if entries.len() >= 100 {
            break;
        }
    }

    if entries.is_empty() {
        None
    } else {
        serde_json::to_string(&entries).ok()
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

        fn read_raw(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path)).ok()
        }

        let nftables_rules = tokio::time::timeout(
            Duration::from_secs(3),
            query_nftables_rules(),
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
            nftables_rules,
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
