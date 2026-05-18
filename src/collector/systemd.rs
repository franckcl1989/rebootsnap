use serde::Serialize;
use std::time::Instant;
use zbus::zvariant::OwnedObjectPath;

use crate::collector::{CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Systemd;

#[derive(Serialize)]
struct SystemdRecord {
    collection: &'static str,
    manager_version: Option<String>,
    architecture: Option<String>,
    features: Option<String>,
    virtualization: Option<String>,
    units: Vec<UnitEntry>,
    jobs: Vec<JobEntry>,
    inhibitors: Vec<InhibitorEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed_units_detail: Option<String>,
}

#[derive(Serialize)]
struct UnitEntry {
    name: String,
    description: String,
    load: String,
    active: String,
    sub: String,
}

#[derive(Serialize)]
struct JobEntry {
    id: u32,
    unit: String,
    #[serde(rename = "type")]
    kind: String,
    state: String,
}

#[derive(Serialize)]
struct InhibitorEntry {
    what: String,
    who: String,
    why: String,
    mode: String,
    uid: u32,
    pid: u32,
}

type UnitRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    OwnedObjectPath,
    u32,
    String,
    OwnedObjectPath,
);

type JobRow = (
    u32,
    String,
    String,
    String,
    OwnedObjectPath,
    OwnedObjectPath,
);

type InhibitorRow = (String, String, String, String, u32, u32);

impl Systemd {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                return ProbeOutcome {
                    available: false,
                    degraded: Vec::new(),
                    reason: Some(format!("D-Bus system bus unavailable: {}", e)),
                    roots: roots.clone(),
                };
            }
        };
        match conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .await
        {
            Ok(reply) => {
                let names: Result<Vec<String>, _> = reply.body().deserialize();
                let has_systemd = names.is_ok_and(|n| {
                    n.contains(&"org.freedesktop.systemd1".to_string())
                });
                if has_systemd {
                    ProbeOutcome {
                        available: true,
                        degraded: Vec::new(),
                        reason: None,
                        roots: roots.clone(),
                    }
                } else {
                    ProbeOutcome {
                        available: false,
                        degraded: Vec::new(),
                        reason: Some("org.freedesktop.systemd1 not on system bus".into()),
                        roots: roots.clone(),
                    }
                }
            }
            Err(e) => ProbeOutcome {
                available: false,
                degraded: Vec::new(),
                reason: Some(format!("D-Bus ListNames failed: {}", e)),
                roots: roots.clone(),
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

        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
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

        let manager_version = get_manager_property_string(&conn, "Version").await;
        let architecture = get_manager_property_string(&conn, "Architecture").await;
        let features = get_manager_property_string(&conn, "Features").await;
        let virtualization = get_manager_property_string(&conn, "Virtualization").await;

        let (units, unit_err) = list_units(&conn).await;
        if let Some(e) = unit_err {
            degraded.push(format!("ListUnits: {}", e));
        }

        let (jobs, job_err) = list_jobs(&conn).await;
        if let Some(e) = job_err {
            degraded.push(format!("ListJobs: {}", e));
        }

        let (inhibitors, inh_err) = list_inhibitors(&conn).await;
        if let Some(e) = inh_err {
            degraded.push(format!("ListInhibitors: {}", e));
        }

        let failed_detail = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            query_failed_units(&conn),
        )
        .await
        .ok()
        .flatten();

        let all_empty = units.is_empty() && jobs.is_empty() && inhibitors.is_empty();
        if all_empty && !degraded.is_empty() {
            return CollectionOutcome {
                status: CollectionStatus::Failed {
                    reason: degraded.join("; "),
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

        let record = SystemdRecord {
            collection: "RT-03",
            manager_version,
            architecture,
            features,
            virtualization,
            units: units.into_iter().map(|(name, desc, load, active, sub, ..)| UnitEntry {
                name,
                description: desc,
                load,
                active,
                sub,
            }).collect(),
            jobs: jobs.into_iter().map(|(id, unit, kind, state, ..)| JobEntry { id, unit, kind, state }).collect(),
            inhibitors: inhibitors.into_iter().map(|(what, who, why, mode, uid, pid)| InhibitorEntry {
                what, who, why, mode, uid, pid,
            }).collect(),
            failed_units_detail: failed_detail,
        };

        let writer = match output.json_writer("systemd.json") {
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
            status: if degraded.is_empty() {
                CollectionStatus::Ok
            } else {
                CollectionStatus::Degraded { missing: degraded }
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

async fn get_manager_property_string(
    conn: &zbus::Connection,
    name: &str,
) -> Option<String> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.freedesktop.systemd1.Manager", name),
        )
        .await
        .ok()?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    match &*value {
        zbus::zvariant::Value::Str(s) => Some(s.to_string()),
        zbus::zvariant::Value::Array(_) => {
            let strs: Vec<String> = value.try_into().ok()?;
            Some(strs.join(" "))
        }
        _ => None,
    }
}

async fn list_units(conn: &zbus::Connection) -> (Vec<UnitRow>, Option<String>) {
    match conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "ListUnits",
            &(),
        )
        .await
    {
        Ok(reply) => match reply.body().deserialize::<Vec<UnitRow>>() {
            Ok(units) => (units, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        },
        Err(e) => (Vec::new(), Some(e.to_string())),
    }
}

async fn list_jobs(conn: &zbus::Connection) -> (Vec<JobRow>, Option<String>) {
    match conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "ListJobs",
            &(),
        )
        .await
    {
        Ok(reply) => match reply.body().deserialize::<Vec<JobRow>>() {
            Ok(jobs) => (jobs, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        },
        Err(e) => (Vec::new(), Some(e.to_string())),
    }
}

async fn list_inhibitors(conn: &zbus::Connection) -> (Vec<InhibitorRow>, Option<String>) {
    match conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "ListInhibitors",
            &(),
        )
        .await
    {
        Ok(reply) => match reply.body().deserialize::<Vec<InhibitorRow>>() {
            Ok(inh) => (inh, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        },
        Err(e) => (Vec::new(), Some(e.to_string())),
    }
}

async fn query_failed_units(
    conn: &zbus::Connection,
) -> Option<String> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "ListUnits",
            &(),
        )
        .await
        .ok()?;
    let units: Vec<UnitRow> = reply.body().deserialize().ok()?;

    let mut details = Vec::new();
    for (name, _desc, _load, active, _sub, _follow, _upath, _jid, _jtype, _jpath) in units {
        if active != "failed" && active != "auto-restart" {
            continue;
        }
        if details.len() >= 20 {
            break;
        }

        let get_reply = match conn
            .call_method(
                Some("org.freedesktop.systemd1"),
                "/org/freedesktop/systemd1",
                Some("org.freedesktop.systemd1.Manager"),
                "GetUnit",
                &(&name),
            )
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };
        let unit_path: OwnedObjectPath = match get_reply.body().deserialize() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let active_state = get_unit_property_string(conn, &unit_path, "ActiveState").await;
        let sub_state = get_unit_property_string(conn, &unit_path, "SubState").await;
        let load_state = get_unit_property_string(conn, &unit_path, "LoadState").await;
        let description = get_unit_property_string(conn, &unit_path, "Description").await;
        let main_pid = get_unit_property_u32(conn, &unit_path, "MainPID").await;

        details.push(serde_json::json!({
            "name": name,
            "load_state": load_state,
            "active_state": active_state,
            "sub_state": sub_state,
            "main_pid": main_pid,
            "description": description,
        }));
    }

    if details.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&details).unwrap_or_default())
    }
}

async fn get_unit_property_string(
    conn: &zbus::Connection,
    unit_path: &OwnedObjectPath,
    property: &str,
) -> Option<String> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            unit_path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.freedesktop.systemd1.Unit", property),
        )
        .await
        .ok()?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    match &*value {
        zbus::zvariant::Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

async fn get_unit_property_u32(
    conn: &zbus::Connection,
    unit_path: &OwnedObjectPath,
    property: &str,
) -> Option<u32> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            unit_path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.freedesktop.systemd1.Unit", property),
        )
        .await
        .ok()?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    match &*value {
        zbus::zvariant::Value::U32(v) => Some(*v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::setup_test;

    #[tokio::test]
    async fn test_collect_normal() {
        let ctx = setup_test("normal");
        let probe = Systemd.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Systemd probe unavailable in mock (expected without D-Bus/netlink)");
            return;
        }
        let outcome = Systemd.collect(&ctx.output, &probe).await;
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
