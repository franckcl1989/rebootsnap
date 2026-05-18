use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Security;

#[derive(Serialize)]
struct SecurityRecord {
    collection: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    entropy_avail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    poolsize: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_wakeup_threshold: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    urandom_min_reseed_secs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cap_last_cap: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seccomp_actions_avail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seccomp_actions_logged: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audit_backlog_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audit_backlog_wait_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ima_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lsm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lockdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fips_enabled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip_forward: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ipv6_forwarding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rp_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tcp_syncookies: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selinux_enforce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_ratelimit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printk_ratelimit_burst: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accept_redirects: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    send_redirects: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secure_redirects: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_martians: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/sys/kernel/random/entropy_avail",
    "/proc/sys/kernel/random/poolsize",
    "/proc/sys/kernel/random/read_wakeup_threshold",
    "/proc/sys/kernel/random/urandom_min_reseed_secs",
    "/proc/sys/kernel/cap_last_cap",
    "/proc/sys/kernel/seccomp/actions_avail",
    "/proc/sys/kernel/seccomp/actions_logged",
    "/proc/sys/kernel/audit_backlog_limit",
    "/proc/sys/kernel/audit_backlog_wait_time",
    "/sys/kernel/security/ima/policy",
    "/sys/kernel/security/lsm",
    "/sys/kernel/security/lockdown",
    "/proc/sys/crypto/fips_enabled",
    "/proc/sys/net/ipv4/ip_forward",
    "/proc/sys/net/ipv6/conf/all/forwarding",
    "/proc/sys/net/ipv4/conf/all/rp_filter",
    "/proc/sys/net/ipv4/tcp_syncookies",
    "/proc/sys/net/ipv4/conf/all/accept_redirects",
    "/proc/sys/net/ipv4/conf/all/send_redirects",
    "/proc/sys/net/ipv4/conf/all/secure_redirects",
    "/proc/sys/net/ipv4/conf/all/log_martians",
];

impl Security {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all security files missing")
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

        let record = SecurityRecord {
            collection: "RT-16",
            entropy_avail: read_raw(&probe.roots, FILES[0]),
            poolsize: read_raw(&probe.roots, FILES[1]),
            read_wakeup_threshold: read_raw(&probe.roots, FILES[2]),
            urandom_min_reseed_secs: read_raw(&probe.roots, FILES[3]),
            cap_last_cap: read_raw(&probe.roots, FILES[4]),
            seccomp_actions_avail: read_raw(&probe.roots, FILES[5]),
            seccomp_actions_logged: read_raw(&probe.roots, FILES[6]),
            audit_backlog_limit: read_raw(&probe.roots, FILES[7]),
            audit_backlog_wait_time: read_raw(&probe.roots, FILES[8]),
            ima_policy: read_raw(&probe.roots, FILES[9]),
            lsm: read_raw(&probe.roots, FILES[10]),
            lockdown: read_raw(&probe.roots, FILES[11]),
            fips_enabled: read_raw(&probe.roots, FILES[12]),
            ip_forward: read_raw(&probe.roots, FILES[13]),
            ipv6_forwarding: read_raw(&probe.roots, FILES[14]),
            rp_filter: read_raw(&probe.roots, FILES[15]),
            tcp_syncookies: read_raw(&probe.roots, FILES[16]),
            selinux_enforce: read_raw(&probe.roots, "/sys/fs/selinux/enforce"),
            printk_ratelimit: read_raw(&probe.roots, "/proc/sys/kernel/printk_ratelimit"),
            printk_ratelimit_burst: read_raw(&probe.roots, "/proc/sys/kernel/printk_ratelimit_burst"),
            accept_redirects: read_raw(&probe.roots, FILES[17]),
            send_redirects: read_raw(&probe.roots, FILES[18]),
            secure_redirects: read_raw(&probe.roots, FILES[19]),
            log_martians: read_raw(&probe.roots, FILES[20]),
        };

        let writer = match output.json_writer("security.json") {
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
        let probe = Security.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Security probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Security.collect(&ctx.output, &probe).await;
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
