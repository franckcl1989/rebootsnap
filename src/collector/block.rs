use serde::Serialize;
use std::time::Instant;

use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Block;

#[derive(Serialize)]
struct BlockRecord {
    collection: &'static str,
    diskstats: Option<String>,
    partitions: Option<String>,
    pressure_io: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block_device_info: Option<String>,
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
    dm_name: Option<String>,
    dm_uuid: Option<String>,
    dm_suspended: Option<String>,
    loop_backing_file: Option<String>,
    zram_disksize: Option<String>,
    zram_comp_algorithm: Option<String>,
    zram_mm_stat: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/diskstats",
    "/proc/partitions",
    "/proc/pressure/io",
    "/proc/sys/fs/aio-max-nr",
    "/proc/sys/fs/aio-nr",
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

        fn read_raw(roots: &FsRoots, path: &str) -> Option<String> {
            std::fs::read_to_string(roots.resolve(path)).ok()
        }

        let block_device_info = enumerate_block_devices(&probe.roots);

        let record = BlockRecord {
            collection: "RT-10",
            diskstats: read_raw(&probe.roots, FILES[0]),
            partitions: read_raw(&probe.roots, FILES[1]),
            pressure_io: read_raw(&probe.roots, FILES[2]),
            block_device_info,
            aio_max_nr: read_raw(&probe.roots, FILES[3]),
            aio_nr: read_raw(&probe.roots, FILES[4]),
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

fn enumerate_block_devices(roots: &FsRoots) -> Option<String> {
    let block_dir = roots.resolve("/sys/block");
    let dir = std::fs::read_dir(&block_dir).ok()?;
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
            dm_name,
            dm_uuid,
            dm_suspended,
            loop_backing_file,
            zram_disksize,
            zram_comp_algorithm,
            zram_mm_stat,
        });
    }

    if devices.is_empty() {
        None
    } else {
        serde_json::to_string(&devices).ok()
    }
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
        match &outcome.status {
            crate::collector::CollectionStatus::Failed { reason } => {
                panic!("collect failed: {reason}");
            }
            _ => {}
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
