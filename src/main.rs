use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use chrono::Local;
use serde::Serialize;

mod collector;
mod output;

use collector::{
    boot::BootCollector, cpu::CpuCollector, memory::MemoryCollector, process::ProcessCollector,
    CollectResult, CollectStatus, Collector, ProbeStatus,
};
use output::OutputDir;

const GLOBAL_TIMEOUT_SECS: u64 = 300;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct ManifestItem {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<String>>,
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
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_reason: Option<String>,
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

fn manifest_status(cs: &CollectStatus) -> &'static str {
    match cs {
        CollectStatus::Ok => "ok",
        CollectStatus::Truncated(_) => "truncated",
        CollectStatus::Degraded(_) => "degraded",
        CollectStatus::Failed(_) => "failed",
        CollectStatus::TimedOut => "timed_out",
    }
}

fn read_hostname() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_kernel() -> String {
    fs::read_to_string("/proc/version")
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

fn read_boot_id() -> String {
    fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_uptime() -> u64 {
    fs::read_to_string("/proc/uptime")
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64
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

fn build_summary(items: &[ManifestItem]) -> Summary {
    let hostname = read_hostname();
    let uptime = read_uptime();

    let procs = items.iter().find(|i| i.id == "RT-04");
    let procs_total = procs.and_then(|p| p.collected_items);
    let procs_total_truncated = procs
        .filter(|p| p.status == "truncated")
        .map(|_| true);

    let mut mem_total_kb: Option<i64> = None;
    let mut mem_available_kb: Option<i64> = None;
    if let Some(mem_item) = items.iter().find(|i| i.id == "RT-06") {
        if let Some(ref filename) = mem_item.file {
            if let Ok(raw) = fs::read_to_string(filename) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(arr) = val["meminfo"].as_array() {
                        for item in arr {
                            match item["key"].as_str() {
                                Some("MemTotal") => mem_total_kb = item["value_kb"].as_i64(),
                                Some("MemAvailable") => mem_available_kb = item["value_kb"].as_i64(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    let (load1, load5, load15) = {
        let raw = fs::read_to_string("/proc/loadavg").unwrap_or_default();
        let parts: Vec<&str> = raw.split_whitespace().collect();
        (
            parts.first().and_then(|s| s.parse::<f64>().ok()),
            parts.get(1).and_then(|s| s.parse::<f64>().ok()),
            parts.get(2).and_then(|s| s.parse::<f64>().ok()),
        )
    };

    Summary {
        collection: "summary".to_string(),
        hostname,
        uptime_seconds: uptime,
        procs_total,
        procs_total_truncated,
        mem_total_kb,
        mem_available_kb,
        load1,
        load5,
        load15,
    }
}

#[derive(Default)]
struct ProbeSummary {
    systemd_available: bool,
    netlink_available: bool,
    conntrack_available: bool,
    hidepid: String,
    capabilities: Vec<String>,
}

impl ProbeSummary {
    fn update(&mut self, id: &str, status: &ProbeStatus) {
        match status {
            ProbeStatus::Available => match id {
                "RT-03" => self.systemd_available = true,
                "RT-11" | "RT-12" => self.netlink_available = true,
                "RT-13" => {
                    self.netlink_available = true;
                    self.conntrack_available = true;
                }
                _ => {}
            },
            ProbeStatus::Degraded(_) => match id {
                "RT-11" | "RT-12" | "RT-13" => self.netlink_available = true,
                _ => {}
            },
            _ => {}
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let base_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let output = match OutputDir::create(&base_dir).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let manifest_dir = output.root().to_path_buf();

    let collectors: Vec<Box<dyn Collector>> = vec![
        Box::new(BootCollector::new()),
        Box::new(ProcessCollector::new()),
        Box::new(CpuCollector::new()),
        Box::new(MemoryCollector::new()),
    ];

    let global_start = Instant::now();

    let mut probes = Vec::with_capacity(collectors.len());
    let mut probe_summary = ProbeSummary::default();
    for c in &collectors {
        let result = c.probe().await;
        probe_summary.update(c.id(), &result.status);
        probes.push(result);
    }

    let infos: Vec<(String, String)> = collectors
        .iter()
        .map(|c| (c.id().to_string(), c.filename().to_string()))
        .collect();

    let mut set = tokio::task::JoinSet::new();
    for (i, (c, probe)) in collectors.into_iter().zip(probes.into_iter()).enumerate() {
        if !c.required() {
            continue;
        }
        let output = OutputDir {
            root: manifest_dir.clone(),
        };
        let timeout = c.item_timeout();
        let id = if i < infos.len() {
            infos[i].0.clone()
        } else {
            "?".to_string()
        };
        set.spawn(async move {
            let result = tokio::time::timeout(timeout, c.collect(&output, &probe)).await;
            (id, result)
        });
    }

    let global_result = tokio::time::timeout(Duration::from_secs(GLOBAL_TIMEOUT_SECS), async {
        let mut results: Vec<(String, Result<CollectResult, tokio::time::error::Elapsed>)> =
            Vec::new();
        while let Some(join_result) = set.join_next().await {
            match join_result {
                Ok((id, result)) => results.push((id, result)),
                Err(e) => {
                    eprintln!("Task join error: {}", e);
                }
            }
        }
        results
    })
    .await;

    let (results, exit_reason) = match global_result {
        Ok(results) => (results, "completed".to_string()),
        Err(_) => (Vec::new(), "global_timeout".to_string()),
    };

    let mut manifest_items: Vec<ManifestItem> = Vec::new();

    for info in &infos {
        let task_result = results.iter().find(|(id, _)| id == &info.0);
        let item = match task_result {
            Some((_, Ok(result))) => ManifestItem {
                id: info.0.clone(),
                status: manifest_status(&result.status).to_string(),
                file: Some(info.1.clone()),
                files: None,
                size_bytes: if result.file_size > 0 {
                    Some(result.file_size)
                } else {
                    None
                },
                duration_ms: Some(result.duration.as_millis() as u64),
                total_items: result.items_total,
                collected_items: result.items_collected,
                truncation: match &result.status {
                    CollectStatus::Truncated(reason) => Some(reason.clone()),
                    _ => None,
                },
                missing: match &result.status {
                    CollectStatus::Degraded(items) => Some(items.clone()),
                    _ => None,
                },
                reason: None,
                error_reason: match &result.status {
                    CollectStatus::Failed(reason) => Some(reason.clone()),
                    _ => None,
                },
            },
            Some((_, Err(_elapsed))) => ManifestItem {
                id: info.0.clone(),
                status: "timed_out".to_string(),
                file: Some(info.1.clone()),
                files: None,
                size_bytes: None,
                duration_ms: None,
                total_items: None,
                collected_items: None,
                truncation: None,
                missing: None,
                reason: None,
                error_reason: None,
            },
            None => ManifestItem {
                id: info.0.clone(),
                status: "timed_out".to_string(),
                file: Some(info.1.clone()),
                files: None,
                size_bytes: None,
                duration_ms: None,
                total_items: None,
                collected_items: None,
                truncation: None,
                missing: None,
                reason: None,
                error_reason: None,
            },
        };
        manifest_items.push(item);
    }

    let global_duration = global_start.elapsed();

    let summary = build_summary(&manifest_items);
    let summary_bytes = serde_json::to_vec_pretty(&summary).unwrap_or_default();
    let summary_path = manifest_dir.join("summary.json");
    if let Err(e) = fs::write(&summary_path, &summary_bytes) {
        eprintln!("Error writing summary.json: {}", e);
    }

    let manifest = Manifest {
        rebootsnap_version: VERSION.to_string(),
        collected_at: Local::now().to_rfc3339(),
        host: ManifestHost {
            hostname: read_hostname(),
            kernel: read_kernel(),
            boot_id: read_boot_id(),
            uptime_seconds: read_uptime(),
        },
        probe: ManifestProbe {
            systemd: if probe_summary.systemd_available {
                "available".to_string()
            } else {
                "unavailable".to_string()
            },
            netlink: if probe_summary.netlink_available {
                "available".to_string()
            } else {
                "unavailable".to_string()
            },
            conntrack: if probe_summary.conntrack_available {
                "available".to_string()
            } else {
                "unavailable".to_string()
            },
            hidepid: probe_summary.hidepid,
            capabilities: probe_summary.capabilities,
        },
        items: manifest_items,
        global_duration_ms: global_duration.as_millis() as u64,
        exit_reason,
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap_or_default();
    let manifest_path = manifest_dir.join("manifest.json");
    if let Err(e) = fs::write(&manifest_path, &manifest_bytes) {
        eprintln!("Error writing manifest.json: {}", e);
    }

    output.cleanup_tmp();

    let dir_name = output
        .directory_name()
        .unwrap_or("rebootsnap-unknown");
    let tar_name = format!("{}.tar.gz", dir_name);
    let tar_path = base_dir.join(&tar_name);

    match create_tar_gz(&manifest_dir, dir_name, &tar_path) {
        Ok(()) => {
            let _ = fs::remove_dir_all(&manifest_dir);
            println!("{}", tar_path.display());
        }
        Err(e) => {
            eprintln!("Error creating tar.gz: {}", e);
            eprintln!("Output directory preserved at: {}", manifest_dir.display());
            process::exit(1);
        }
    }
}

fn create_tar_gz(
    src_dir: &std::path::Path,
    dir_name: &str,
    dest_path: &std::path::Path,
) -> Result<(), String> {
    let gz_file = fs::File::create(dest_path)
        .map_err(|e| format!("cannot create {}: {}", dest_path.display(), e))?;
    let gz_encoder = flate2::write::GzEncoder::new(gz_file, flate2::Compression::default());
    let mut tar_builder = tar::Builder::new(gz_encoder);

    for entry in
        fs::read_dir(src_dir).map_err(|e| format!("cannot read dir {}: {}", src_dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("dir entry error: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path
                .file_name()
                .ok_or_else(|| format!("no file name for {}", path.display()))?;
            let tar_path = std::path::Path::new(dir_name).join(file_name);
            tar_builder
                .append_path_with_name(&path, &tar_path)
                .map_err(|e| format!("tar append error for {}: {}", path.display(), e))?;
        }
    }

    let gz_encoder = tar_builder
        .into_inner()
        .map_err(|e| format!("tar finish error: {}", e))?;
    gz_encoder
        .finish()
        .map_err(|e| format!("gzip finish error: {}", e))?;

    Ok(())
}
