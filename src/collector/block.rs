use serde::Serialize;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Block;

#[derive(Serialize)]
struct BlockRecord {
    schema_version: &'static str,
    collection: &'static str,
    diskstats: Option<String>,
    partitions: Option<String>,
    pressure_io: Option<String>,
    block_device_info: Vec<BlockDeviceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aio_max_nr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aio_nr: Option<String>,
}

#[derive(Serialize)]
struct BlockDeviceInfo {
    name: String,
    scheduler: Option<String>,
    nr_requests: Option<String>,
    read_ahead_kb: Option<String>,
    rotational: Option<String>,
    max_sectors_kb: Option<String>,
    stat: Option<String>,
    io_stat: Option<IoStatFields>,
    dm_name: Option<String>,
    dm_uuid: Option<String>,
    dm_suspended: Option<String>,
    loop_backing_file: Option<String>,
    zram_disksize: Option<String>,
    zram_comp_algorithm: Option<String>,
    zram_mm_stat: Option<String>,
}
#[derive(Serialize)]
struct IoStatFields {
    rd_ios: u64,
    rd_merges: u64,
    rd_sectors: u64,
    rd_ticks: u64,
    wr_ios: u64,
    wr_merges: u64,
    wr_sectors: u64,
    wr_ticks: u64,
    ios_in_progress: u64,
    io_ticks: u64,
    io_aveq: u64,
}

fn parse_io_stat(stat_str: &str) -> Option<IoStatFields> {
    let parts: Vec<u64> = stat_str.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    if parts.len() < 11 { return None; }
    Some(IoStatFields {
        rd_ios: parts[0], rd_merges: parts[1], rd_sectors: parts[2], rd_ticks: parts[3],
        wr_ios: parts[4], wr_merges: parts[5], wr_sectors: parts[6], wr_ticks: parts[7],
        ios_in_progress: parts[8], io_ticks: parts[9], io_aveq: parts[10],
    })
}

const FILES: &[&str] = &[
    "/proc/diskstats",
    "/proc/partitions",
    "/proc/pressure/io",
    "/proc/sys/fs/aio-max-nr",
    "/proc/sys/fs/aio-nr",
];

const UNSUPPORTED_IF_MISSING: &[&str] = &[
    "/proc/pressure/io",
];

impl Block {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all block files missing")
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

        let block_device_info = enumerate_block_devices(&probe.roots);

        let record = BlockRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-10",
            diskstats: read_trimmed(&probe.roots, FILES[0]),
            partitions: read_trimmed(&probe.roots, FILES[1]),
            pressure_io: read_trimmed(&probe.roots, FILES[2]),
            block_device_info,
            aio_max_nr: read_trimmed(&probe.roots, FILES[3]),
            aio_nr: read_trimmed(&probe.roots, FILES[4]),
        };

        let writer = match output.json_writer("block.json") {
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

        let (unsupported, degraded): (Vec<_>, Vec<_>) = probe.degraded.iter()
            .cloned()
            .partition(|f| UNSUPPORTED_IF_MISSING.contains(&f.as_str()));

        CollectionOutcome {
            status: if !degraded.is_empty() {
                CollectionStatus::Partial { degrading: degraded }
            } else if !unsupported.is_empty() {
                CollectionStatus::Unsupported { reason: unsupported.join(", ") }
            } else {
                CollectionStatus::Ok
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

fn enumerate_block_devices(roots: &FsRoots) -> Vec<BlockDeviceInfo> {
    let block_dir = roots.resolve("/sys/block");
    let dir = match std::fs::read_dir(&block_dir) { Ok(d) => d, Err(_) => return Vec::new() };
    let mut devices: Vec<BlockDeviceInfo> = Vec::new();

    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        let base = entry.path();

        let read_attr = |rel: &str| -> Option<String> {
            std::fs::read_to_string(base.join(rel)).ok()
        };

        let scheduler = read_attr("queue/scheduler");
        let nr_requests = read_attr("queue/nr_requests");
        let read_ahead_kb = read_attr("queue/read_ahead_kb");
        let rotational = read_attr("queue/rotational");
        let max_sectors_kb = read_attr("queue/max_sectors_kb");
        let stat = read_attr("stat");
        let io_stat = stat.as_deref().and_then(parse_io_stat);

        let dm = name.starts_with("dm-");
        let dm_name = if dm { read_attr("dm/name") } else { None };
        let dm_uuid = if dm { read_attr("dm/uuid") } else { None };
        let dm_suspended = if dm { read_attr("dm/suspended") } else { None };

        let is_loop = name.starts_with("loop");
        let loop_backing_file = if is_loop {
            std::fs::read_link(base.join("loop/backing_file"))
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        } else {
            None
        };

        let is_zram = name.starts_with("zram");
        let zram_disksize = if is_zram { read_attr("disksize") } else { None };
        let zram_comp_algorithm = if is_zram { read_attr("comp_algorithm") } else { None };
        let zram_mm_stat = if is_zram { read_attr("mm_stat") } else { None };

        devices.push(BlockDeviceInfo {
            name,
            scheduler,
            nr_requests,
            read_ahead_kb,
            rotational,
            max_sectors_kb,
            stat,
            io_stat,
            dm_name,
            dm_uuid,
            dm_suspended,
            loop_backing_file,
            zram_disksize,
            zram_comp_algorithm,
            zram_mm_stat,
        });
    }

    devices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::setup_test;

    #[tokio::test]
    async fn test_collect_normal() {
        let ctx = setup_test("normal");
        let probe = Block.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Block probe unavailable in mock - skipping");
            return;
        }
        let outcome = Block.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }

    #[tokio::test]
    async fn test_collect_content() {
        let ctx = setup_test("normal");
        let probe = Block.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Block.collect(&ctx.output, &probe).await;
        assert!(outcome.file_size > 0);

        let output_dir = ctx.output.root();
        let entries: Vec<_> = std::fs::read_dir(output_dir).expect("read_dir")
            .filter_map(|e| e.ok()).collect();
        assert!(!entries.is_empty(), "should have output files");

        let content = std::fs::read_to_string(entries[0].path()).expect("read output");
        let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(json["collection"], "RT-10");
        assert!(json["diskstats"].is_string(), "diskstats should be present");
    }
}
