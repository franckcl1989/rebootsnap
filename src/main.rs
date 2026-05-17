use std::fs as std_fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use chrono::Local;
use rebootsnap::collector::{
    all_tasks, CollectionOutcome, CollectionStatus, ProbeOutcome,
};
use rebootsnap::error::ArchiveError;
use rebootsnap::fs::FsRoots;
use rebootsnap::output::OutputDir;
use serde::Serialize;

const GLOBAL_TIMEOUT_SECS: u64 = 300;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct ManifestItem {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collected_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_reason: Option<String>,
}

#[derive(Serialize)]
struct Manifest {
    rebootsnap_version: String,
    collected_at: String,
    host: ManifestHost,
    probe: ManifestProbe,
    items: Vec<ManifestItem>,
    global_duration_ms: u64,
    exit_reason: String,
}

#[derive(Serialize)]
struct ManifestHost {
    hostname: String,
    kernel: String,
    boot_id: String,
    uptime_seconds: u64,
}

#[derive(Serialize)]
struct ManifestProbe {
    systemd: String,
    netlink: String,
    conntrack: String,
    hidepid: String,
    capabilities: Vec<String>,
}

#[derive(Serialize)]
struct Summary {
    collection: String,
    hostname: String,
    uptime_seconds: u64,
    procs_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    procs_total_truncated: Option<bool>,
    mem_total_kb: Option<i64>,
    mem_available_kb: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    load1: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    load5: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    load15: Option<f64>,
}

fn outcome_status(status: &CollectionStatus) -> &'static str {
    match status {
        CollectionStatus::Ok => "ok",
        CollectionStatus::Truncated { .. } => "truncated",
        CollectionStatus::Degraded { .. } => "degraded",
        CollectionStatus::Failed { .. } => "failed",
        CollectionStatus::TimedOut => "timed_out",
    }
}

fn loadavg() -> (Option<f64>, Option<f64>, Option<f64>) {
    let raw = std_fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let parts: Vec<&str> = raw.split_whitespace().collect();
    (
        parts.first().and_then(|s| s.parse().ok()),
        parts.get(1).and_then(|s| s.parse().ok()),
        parts.get(2).and_then(|s| s.parse().ok()),
    )
}

fn create_tar_gz(
    src_dir: &Path,
    dir_name: &str,
    dest_path: &Path,
) -> Result<(), ArchiveError> {
    let gz_file = std_fs::File::create(dest_path).map_err(ArchiveError::Io)?;
    let gz_encoder = flate2::write::GzEncoder::new(gz_file, flate2::Compression::default());
    let mut tar_builder = tar::Builder::new(gz_encoder);

    for entry in std_fs::read_dir(src_dir).map_err(ArchiveError::Io)? {
        let entry = entry.map_err(ArchiveError::Io)?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .ok_or_else(|| ArchiveError::Tar(format!("no file name for {}", path.display())))?;
        let tar_path = Path::new(dir_name).join(file_name);
        tar_builder
            .append_path_with_name(&path, &tar_path)
            .map_err(|e| ArchiveError::Tar(format!("tar append error for {}: {}", path.display(), e)))?;
    }

    let gz_encoder = tar_builder
        .into_inner()
        .map_err(|e| ArchiveError::Tar(format!("tar finish error: {}", e)))?;
    gz_encoder
        .finish()
        .map_err(|e| ArchiveError::Gzip(format!("gzip finish error: {}", e)))?;

    Ok(())
}

fn timed_out_outcome() -> CollectionOutcome {
    CollectionOutcome {
        status: CollectionStatus::TimedOut,
        duration: std::time::Duration::ZERO,
        file_size: 0,
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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let base_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let output = match OutputDir::create(&base_dir).await {
        Ok(o) => o,
        Err(e) => {
            tracing::error!("{}", e);
            process::exit(1);
        }
    };
    let manifest_dir = output.root().to_path_buf();

    let global_start = Instant::now();

    let tasks = all_tasks();

    let roots = FsRoots::default();

    let task_infos: Vec<(&str, &str, std::time::Duration)> = tasks
        .iter()
        .map(|t| (t.id(), t.filename(), t.item_timeout()))
        .collect();

    let mut probe_pairs: Vec<(&str, ProbeOutcome)> = Vec::with_capacity(tasks.len());
    for task in &tasks {
        probe_pairs.push((task.id(), task.probe(&roots).await));
    }
    let probe_map: std::collections::HashMap<&str, &ProbeOutcome> = probe_pairs
        .iter()
        .map(|(id, p)| (*id, p))
        .collect();

    let mut set = tokio::task::JoinSet::new();

    for (i, task) in tasks.into_iter().enumerate() {
        let root = manifest_dir.clone();
        let id = task.id().to_string();
        let filename = task.filename().to_string();
        let timeout_secs = task.item_timeout();
        let probe = probe_pairs[i].1.clone();
        set.spawn(async move {
            let output = OutputDir::from_existing(root);
            let outcome = tokio::time::timeout(timeout_secs, task.collect(&output, &probe))
                .await
                .unwrap_or_else(|_| timed_out_outcome());
            (id, filename, outcome)
        });
    }

    let global_result = tokio::time::timeout(
        std::time::Duration::from_secs(GLOBAL_TIMEOUT_SECS),
        async {
            let mut results: Vec<(String, String, CollectionOutcome)> = Vec::new();
            while let Some(join_result) = set.join_next().await {
                match join_result {
                    Ok(tuple) => results.push(tuple),
                    Err(e) => {
                        tracing::warn!("task join error: {}", e);
                    }
                }
            }
            results
        },
    )
    .await;

    let (results, exit_reason) = match global_result {
        Ok(results) => (results, "completed".to_string()),
        Err(_) => (Vec::new(), "global_timeout".to_string()),
    };

    let mut manifest_items: Vec<ManifestItem> = Vec::new();
    for &(id, filename, _) in &task_infos {
        let result = results.iter().find(|(rid, _, _)| rid == id);
        let item = match result {
            Some((_, _, outcome)) => ManifestItem {
                id: id.to_string(),
                status: outcome_status(&outcome.status).into(),
                file: Some(filename.to_string()),
                size_bytes: (outcome.file_size > 0).then_some(outcome.file_size),
                duration_ms: Some(outcome.duration.as_millis() as u64),
                total_items: outcome.items_total,
                collected_items: outcome.items_collected,
                truncation: match &outcome.status {
                    CollectionStatus::Truncated { reason } => Some(reason.clone()),
                    _ => None,
                },
                missing: match &outcome.status {
                    CollectionStatus::Degraded { missing } => Some(missing.clone()),
                    _ => None,
                },
                error_reason: match &outcome.status {
                    CollectionStatus::Failed { reason } => Some(reason.clone()),
                    _ => None,
                },
            },
            None => ManifestItem {
                id: id.to_string(),
                status: "timed_out".into(),
                file: Some(filename.to_string()),
                size_bytes: None,
                duration_ms: None,
                total_items: None,
                collected_items: None,
                truncation: None,
                missing: None,
                error_reason: None,
            },
        };
        manifest_items.push(item);
    }

    let global_duration = global_start.elapsed();

    let boot_outcome = results.iter().find(|(id, _, _)| id == "RT-01").map(|(_, _, o)| o);
    let mem_outcome = results.iter().find(|(id, _, _)| id == "RT-06").map(|(_, _, o)| o);
    let proc_item = manifest_items.iter().find(|i| i.id == "RT-04");

    let hostname = boot_outcome.and_then(|o| o.hostname.clone()).unwrap_or_default();
    let kernel = boot_outcome.and_then(|o| o.kernel_version.clone()).unwrap_or_default();
    let boot_id = boot_outcome.and_then(|o| o.boot_id.clone()).unwrap_or_default();
    let uptime = boot_outcome.and_then(|o| o.uptime_seconds).unwrap_or(0);

    let summary = Summary {
        collection: "summary".into(),
        hostname: hostname.clone(),
        uptime_seconds: uptime,
        procs_total: proc_item.and_then(|p| p.collected_items),
        procs_total_truncated: proc_item
            .filter(|p| p.status == "truncated")
            .map(|_| true),
        mem_total_kb: mem_outcome.and_then(|o| o.mem_total_kb),
        mem_available_kb: mem_outcome.and_then(|o| o.mem_available_kb),
        load1: loadavg().0,
        load5: loadavg().1,
        load15: loadavg().2,
    };

    let summary_bytes = serde_json::to_vec_pretty(&summary).unwrap_or_default();
    let summary_path = manifest_dir.join("summary.json");
    if let Err(e) = std_fs::write(&summary_path, &summary_bytes) {
        tracing::error!("cannot write summary.json: {}", e);
    }

    let systemd_probe = probe_map.get("RT-03").copied();
    let netlink_probe = probe_map.get("RT-11").copied();
    let conntrack_probe = probe_map.get("RT-13").copied();

    let manifest = Manifest {
        rebootsnap_version: VERSION.into(),
        collected_at: Local::now().to_rfc3339(),
        host: ManifestHost {
            hostname,
            kernel,
            boot_id,
            uptime_seconds: uptime,
        },
        probe: ManifestProbe {
            systemd: systemd_probe
                .map(|p| {
                    if p.available {
                        "available".into()
                    } else {
                        p.reason.clone().unwrap_or_else(|| "unavailable".into())
                    }
                })
                .unwrap_or_else(|| "unavailable".into()),
            netlink: netlink_probe
                .map(|p| {
                    if p.available {
                        "available".into()
                    } else {
                        p.reason.clone().unwrap_or_else(|| "unavailable".into())
                    }
                })
                .unwrap_or_else(|| "unavailable".into()),
            conntrack: conntrack_probe
                .map(|p| {
                    if p.available {
                        "available".into()
                    } else {
                        p.reason.clone().unwrap_or_else(|| "unavailable".into())
                    }
                })
                .unwrap_or_else(|| "unavailable".into()),
            hidepid: String::new(),
            capabilities: Vec::new(),
        },
        items: manifest_items,
        global_duration_ms: global_duration.as_millis() as u64,
        exit_reason,
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap_or_default();
    let manifest_path = manifest_dir.join("manifest.json");
    if let Err(e) = std_fs::write(&manifest_path, &manifest_bytes) {
        tracing::error!("cannot write manifest.json: {}", e);
    }

    output.cleanup_tmp();

    let dir_name = output
        .directory_name()
        .unwrap_or("rebootsnap-unknown")
        .to_string();
    let tar_name = format!("{}.tar.gz", dir_name);
    let tar_path = base_dir.join(&tar_name);

    let manifest_dir_clone = manifest_dir.clone();
    let tar_path_clone = tar_path.clone();
    let tar_result = tokio::task::spawn_blocking(move || {
        create_tar_gz(&manifest_dir_clone, &dir_name, &tar_path_clone)
    })
    .await;

    match tar_result {
        Ok(Ok(())) => {
            let _ = std_fs::remove_dir_all(&manifest_dir);
            tracing::info!("{}", tar_path.display());
        }
        Ok(Err(e)) => {
            tracing::error!("cannot create tar.gz: {}", e);
            tracing::info!(
                "output directory preserved at: {}",
                manifest_dir.display()
            );
            process::exit(1);
        }
        Err(e) => {
            tracing::error!("spawn_blocking failed: {}", e);
            tracing::info!(
                "output directory preserved at: {}",
                manifest_dir.display()
            );
            process::exit(1);
        }
    }
}
