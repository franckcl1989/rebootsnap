use serde::Serialize;
use std::time::{Duration, Instant};
use zbus::Connection;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Time;

#[derive(Serialize)]
struct TimeSyncInfo {
    ntp_enabled: bool,
    ntp_synchronized: bool,
    timezone: String,
    can_ntp: bool,
}

#[derive(Serialize)]
struct TimeRecord {
    schema_version: &'static str,
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
    time_sync: Option<TimeSyncInfo>,
}

async fn query_timedated() -> Option<TimeSyncInfo> {
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

    Some(TimeSyncInfo {
        ntp_enabled: ntp,
        ntp_synchronized: sync,
        timezone: tz,
        can_ntp,
    })
}

const FILES: &[&str] = &[
    "/proc/timer_list",
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

        let time_sync: Option<TimeSyncInfo> = tokio::time::timeout(Duration::from_secs(3), query_timedated()).await.unwrap_or_default();

        let record = TimeRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-19",
            timer_list: read_trimmed(&probe.roots, FILES[0]),
            rtc_date: read_trimmed(&probe.roots, FILES[1]),
            rtc_time: read_trimmed(&probe.roots, FILES[2]),
            current_clocksource: read_trimmed(&probe.roots, FILES[3]),
            available_clocksource: read_trimmed(&probe.roots, FILES[4]),
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
        let probe = Time.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Time probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Time.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
