use serde::Serialize;
use std::time::{Duration, Instant};
use zbus::zvariant::OwnedObjectPath;

use crate::collector::util::SCHEMA_VERSION;
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
struct LogindSession {
    id: String,
    uid: u32,
    user: String,
    seat: String,
    session_type: String,
    state: String,
    tty: String,
    remote: bool,
    display: String,
}

#[derive(Serialize)]
struct LogindSeat {
    id: String,
    active_session: String,
    can_graphical: bool,
}

#[derive(Serialize)]
struct SessionRecord {
    schema_version: &'static str,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    logind_sessions: Option<Vec<LogindSession>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logind_seats: Option<Vec<LogindSeat>>,
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

type LogindSessionInfo = (String, u32, String, String, OwnedObjectPath);
type LogindSeatInfo = (String, OwnedObjectPath);

async fn get_logind_property_string(
    conn: &zbus::Connection,
    dest: &str,
    obj_path: &str,
    iface: &str,
    name: &str,
) -> Option<String> {
    let reply = conn
        .call_method(
            Some(dest),
            obj_path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(iface, name),
        )
        .await
        .ok()?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    match &*value {
        zbus::zvariant::Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

async fn get_logind_property_bool(
    conn: &zbus::Connection,
    dest: &str,
    obj_path: &str,
    iface: &str,
    name: &str,
) -> Option<bool> {
    let reply = conn
        .call_method(
            Some(dest),
            obj_path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(iface, name),
        )
        .await
        .ok()?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    match &*value {
        zbus::zvariant::Value::Bool(b) => Some(*b),
        _ => None,
    }
}

async fn query_logind_sessions(_roots: &FsRoots) -> Option<Vec<LogindSession>> {
    let connection = zbus::Connection::system().await.ok()?;

    let reply = connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "ListSessions",
            &(),
        )
        .await
        .ok()?;
    let sessions: Vec<LogindSessionInfo> = reply.body().deserialize().ok()?;

    let dest = "org.freedesktop.login1";
    let iface = "org.freedesktop.login1.Session";
    let mut details = Vec::new();
    for (id, uid, user, seat, obj_path) in sessions {
        let obj_path_str = obj_path.to_string();
        let session_type = get_logind_property_string(&connection, dest, &obj_path_str, iface, "Type")
            .await
            .unwrap_or_default();
        let state = get_logind_property_string(&connection, dest, &obj_path_str, iface, "State")
            .await
            .unwrap_or_default();
        let tty = get_logind_property_string(&connection, dest, &obj_path_str, iface, "TTY")
            .await
            .unwrap_or_default();
        let remote = get_logind_property_bool(&connection, dest, &obj_path_str, iface, "Remote")
            .await
            .unwrap_or(false);
        let display = get_logind_property_string(&connection, dest, &obj_path_str, iface, "Display")
            .await
            .unwrap_or_default();

        details.push(LogindSession {
            id,
            uid,
            user,
            seat,
            session_type,
            state,
            tty,
            remote,
            display,
        });
        if details.len() >= 50 {
            break;
        }
    }

    if details.is_empty() {
        None
    } else {
        Some(details)
    }
}

async fn query_logind_seats(_roots: &FsRoots) -> Option<Vec<LogindSeat>> {
    let connection = zbus::Connection::system().await.ok()?;

    let reply = connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "ListSeats",
            &(),
        )
        .await
        .ok()?;
    let seats: Vec<LogindSeatInfo> = reply.body().deserialize().ok()?;

    let dest = "org.freedesktop.login1";
    let iface = "org.freedesktop.login1.Seat";
    let mut details = Vec::new();
    for (id, obj_path) in seats {
        let obj_path_str = obj_path.to_string();
        let active_session = get_logind_property_string(&connection, dest, &obj_path_str, iface, "ActiveSession")
            .await
            .unwrap_or_default();
        let can_graphical = get_logind_property_bool(&connection, dest, &obj_path_str, iface, "CanGraphical")
            .await
            .unwrap_or(false);

        details.push(LogindSeat {
            id,
            active_session,
            can_graphical,
        });
        if details.len() >= 20 {
            break;
        }
    }

    if details.is_empty() {
        None
    } else {
        Some(details)
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

        let logind_sessions = tokio::time::timeout(
            Duration::from_secs(3),
            query_logind_sessions(&probe.roots),
        )
        .await
        .ok()
        .and_then(|r| r);
        let logind_seats = tokio::time::timeout(
            Duration::from_secs(3),
            query_logind_seats(&probe.roots),
        )
        .await
        .ok()
        .and_then(|r| r);

        let record = SessionRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-15",
            utmp_size: read_size(&probe.roots, FILES[0]),
            wtmp_size: read_size(&probe.roots, FILES[1]),
            btmp_size: read_size(&probe.roots, FILES[2]),
            utmp_entries,
            wtmp_entries,
            btmp_entries,
            logind_sessions,
            logind_seats,
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
                CollectionStatus::Partial {
                    degrading: probe.degraded.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::setup_test;

    #[tokio::test]
    async fn test_collect_normal() {
        let ctx = setup_test("normal");
        let probe = Session.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Session probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Session.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
