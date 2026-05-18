use serde::Serialize;
use std::time::{Duration, Instant};
use zbus::Connection;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Time;

#[derive(Serialize)]
struct TimeRecord {
    collection: &'static str,
    timer_list: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rtc_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rtc_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_clocksource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    available_clocksource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_sync: Option<String>,
}

async fn query_timedated() -> Option<String> {
    let connection = Connection::system().await.ok()?;

    let timedated = zbus::Proxy::new(
        &connection,
        "org.freedesktop.timedate1",
        "/org/freedesktop/timedate1",
        "org.freedesktop.timedate1",
    )
    .await
    .ok()?;

    let ntp: bool = timedated.get_property("NTP").await.ok().unwrap_or(false);
    let sync: bool = timedated.get_property("NTPSynchronized").await.ok().unwrap_or(false);
    let tz: String = timedated.get_property("Timezone").await.ok().unwrap_or_default();
    let can_ntp: bool = timedated.get_property("CanNTP").await.ok().unwrap_or(false);

    let result = serde_json::json!({
        "ntp_enabled": ntp,
        "ntp_synchronized": sync,
        "timezone": tz,
        "can_ntp": can_ntp,
    });

    Some(serde_json::to_string(&result).unwrap_or_default())
}

const FILES: &[&str] = &[
    "/proc/timer_list",
    "/proc/driver/rtc",
    "/etc/localtime",
    "/etc/adjtime",
    "/etc/crontab",
    "/sys/class/rtc/rtc0/date",
    "/sys/class/rtc/rtc0/time",
    "/sys/devices/system/clocksource/clocksource0/current_clocksource",
    "/sys/devices/system/clocksource/clocksource0/available_clocksource",
];

impl Time {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all time files missing")
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

        let time_sync: Option<String> = tokio::time::timeout(Duration::from_secs(3), query_timedated()).await.unwrap_or_default();

        let record = TimeRecord {
            collection: "RT-19",
            timer_list: read_raw(&probe.roots, FILES[0]),
            rtc_date: read_raw(&probe.roots, FILES[5]),
            rtc_time: read_raw(&probe.roots, FILES[6]),
            current_clocksource: read_raw(&probe.roots, FILES[7]),
            available_clocksource: read_raw(&probe.roots, FILES[8]),
            time_sync,
        };

        let writer = match output.json_writer("time.json") {
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
        let probe = Time.probe(&ctx.roots).await;
        if !probe.available {
            println!("Time probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Time.collect(&ctx.output, &probe).await;
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
