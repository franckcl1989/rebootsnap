use serde::Serialize;
use std::time::Instant;

use procfs::Current;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Memory;

#[derive(Serialize)]
struct MeminfoEntry {
    key: String,
    value_kb: i64,
}

#[derive(Serialize)]
struct MemoryRecord {
    schema_version: &'static str,
    collection: &'static str,
    meminfo: Vec<MeminfoEntry>,
    pressure_memory: Option<String>,
    vmstat: Option<String>,
    zoneinfo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    swaps: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    swappiness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oom_kill_allocating_task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    panic_on_oom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overcommit_memory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overcommit_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_free_kbytes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dirty_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dirty_background_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vfs_cache_pressure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zone_reclaim_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zram_stats: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zswap_stats: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thp_enabled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thp_defrag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    khugepaged_defrag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    numa_stats: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ksm_run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ksm_pages_shared: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ksm_pages_sharing: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ksm_pages_unshared: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ksm_full_scans: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nr_hugepages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nr_overcommit_hugepages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hugetlb_shm_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    buddyinfo: Option<String>,
}

const FILES: &[&str] = &[
    "/proc/meminfo",
    "/proc/pressure/memory",
    "/proc/vmstat",
    "/proc/zoneinfo",
    "/proc/swaps",
    "/proc/sys/vm/swappiness",
    "/proc/sys/vm/oom_kill_allocating_task",
    "/proc/sys/vm/panic_on_oom",
    "/proc/sys/vm/overcommit_memory",
    "/proc/sys/vm/overcommit_ratio",
    "/proc/sys/vm/min_free_kbytes",
    "/proc/sys/vm/dirty_ratio",
    "/proc/sys/vm/dirty_background_ratio",
    "/proc/sys/vm/vfs_cache_pressure",
    "/proc/sys/vm/zone_reclaim_mode",
    "/sys/kernel/mm/ksm/run",
    "/sys/kernel/mm/ksm/pages_shared",
    "/sys/kernel/mm/ksm/pages_sharing",
    "/sys/kernel/mm/ksm/pages_unshared",
    "/sys/kernel/mm/ksm/full_scans",
    "/proc/sys/vm/nr_hugepages",
    "/proc/sys/vm/nr_overcommit_hugepages",
    "/proc/sys/vm/hugetlb_shm_group",
    "/proc/buddyinfo",
];

const UNSUPPORTED_IF_MISSING: &[&str] = &[
    "/proc/pressure/memory",
];

fn read_zram_stats(roots: &FsRoots) -> Option<String> {
    let block_dir = std::fs::read_dir(roots.resolve("/sys/block")).ok()?;
    let names: Vec<String> = block_dir
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name();
            let s = name.to_str()?;
            if s.starts_with("zram") {
                Some(s.to_string())
            } else {
                None
            }
        })
        .collect();
    if names.is_empty() {
        return None;
    }
    let mut out = String::new();
    for name in &names {
        out.push_str(&format!("{}:\n", name));
        for f in &["disksize", "comp_algorithm", "mm_stat"] {
            let path = format!("/sys/block/{}/{}", name, f);
            let content = std::fs::read_to_string(roots.resolve(&path)).unwrap_or_default();
            out.push_str(&format!("{}: {}\n", f, content.trim()));
        }
    }
    Some(out)
}

fn read_zswap_stats(roots: &FsRoots) -> Option<String> {
    let pool =
        std::fs::read_to_string(roots.resolve("/sys/kernel/mm/zswap/pool_total_size")).ok()?;
    let stored =
        std::fs::read_to_string(roots.resolve("/sys/kernel/mm/zswap/stored_pages"))
            .unwrap_or_default();
    Some(format!(
        "pool_total_size: {}\nstored_pages: {}",
        pool.trim(),
        stored.trim()
    ))
}

fn read_numa_stats(roots: &FsRoots) -> Option<String> {
    let node_dir = std::fs::read_dir(roots.resolve("/sys/devices/system/node")).ok()?;
    let mut names: Vec<String> = node_dir
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name();
            let s = name.to_str()?;
            if s.starts_with("node") {
                Some(s.to_string())
            } else {
                None
            }
        })
        .collect();
    if names.is_empty() {
        return None;
    }
    names.sort();
    let mut out = String::new();
    for name in &names {
        out.push_str(&format!("{}:\n", name));
        for f in &["meminfo", "numastat"] {
            let path = format!("/sys/devices/system/node/{}/{}", name, f);
            let content = std::fs::read_to_string(roots.resolve(&path)).unwrap_or_default();
            out.push_str(&format!("{}:\n{}\n", f, content.trim()));
        }
    }
    Some(out)
}

impl Memory {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all memory files missing")
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

        let pmi = procfs::Meminfo::current().ok();
        let mem_total_kb = pmi.as_ref().map(|m| (m.mem_total / 1024) as i64);
        let mem_available_kb = pmi
            .as_ref()
            .and_then(|m| m.mem_available.map(|v| (v / 1024) as i64));

        let mut entries: Vec<MeminfoEntry> = Vec::new();
        if let Some(raw) = read_trimmed(&probe.roots, FILES[0]) {
            for line in raw.lines() {
                let Some((key, val)) = line.split_once(':') else {
                    continue;
                };
                let key = key.trim().to_string();
                let val_str = val.trim();
                let value_kb = val_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                entries.push(MeminfoEntry { key, value_kb });
            }
        }
        entries.sort_by(|a, b| a.key.cmp(&b.key));

        let record = MemoryRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-06",
            meminfo: entries,
            pressure_memory: read_trimmed(&probe.roots, FILES[1]),
            vmstat: read_trimmed(&probe.roots, FILES[2]),
            zoneinfo: read_trimmed(&probe.roots, FILES[3]),
            swaps: read_trimmed(&probe.roots, FILES[4]),
            swappiness: read_trimmed(&probe.roots, FILES[5]),
            oom_kill_allocating_task: read_trimmed(&probe.roots, FILES[6]),
            panic_on_oom: read_trimmed(&probe.roots, FILES[7]),
            overcommit_memory: read_trimmed(&probe.roots, FILES[8]),
            overcommit_ratio: read_trimmed(&probe.roots, FILES[9]),
            min_free_kbytes: read_trimmed(&probe.roots, FILES[10]),
            dirty_ratio: read_trimmed(&probe.roots, FILES[11]),
            dirty_background_ratio: read_trimmed(&probe.roots, FILES[12]),
            vfs_cache_pressure: read_trimmed(&probe.roots, FILES[13]),
            zone_reclaim_mode: read_trimmed(&probe.roots, FILES[14]),
            zram_stats: read_zram_stats(&probe.roots),
            zswap_stats: read_zswap_stats(&probe.roots),
            thp_enabled: read_trimmed(
                &probe.roots,
                "/sys/kernel/mm/transparent_hugepage/enabled",
            ),
            thp_defrag: read_trimmed(
                &probe.roots,
                "/sys/kernel/mm/transparent_hugepage/defrag",
            ),
            khugepaged_defrag: read_trimmed(
                &probe.roots,
                "/sys/kernel/mm/transparent_hugepage/khugepaged/defrag",
            ),
            numa_stats: read_numa_stats(&probe.roots),
            ksm_run: read_trimmed(&probe.roots, FILES[15]),
            ksm_pages_shared: read_trimmed(&probe.roots, FILES[16]),
            ksm_pages_sharing: read_trimmed(&probe.roots, FILES[17]),
            ksm_pages_unshared: read_trimmed(&probe.roots, FILES[18]),
            ksm_full_scans: read_trimmed(&probe.roots, FILES[19]),
            nr_hugepages: read_trimmed(&probe.roots, FILES[20]),
            nr_overcommit_hugepages: read_trimmed(&probe.roots, FILES[21]),
            hugetlb_shm_group: read_trimmed(&probe.roots, FILES[22]),
            buddyinfo: read_trimmed(&probe.roots, FILES[23]),
        };

        let writer = match output.json_writer("memory.json") {
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
                    mem_total_kb,
                    mem_available_kb,
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
                    mem_total_kb,
                    mem_available_kb,
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
            mem_total_kb,
            mem_available_kb,
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
        let probe = Memory.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Memory probe unavailable in mock - skipping");
            return;
        }
        let outcome = Memory.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }

    #[tokio::test]
    async fn test_collect_content() {
        let ctx = setup_test("normal");
        let probe = Memory.probe(&ctx.roots).await;
        if !probe.available {
            return;
        }
        let outcome = Memory.collect(&ctx.output, &probe).await;
        assert!(outcome.file_size > 0);

        let output_dir = ctx.output.root();
        let entries: Vec<_> = std::fs::read_dir(output_dir).expect("read_dir")
            .filter_map(|e| e.ok()).collect();
        assert!(!entries.is_empty(), "should have output files");

        let content = std::fs::read_to_string(entries[0].path()).expect("read output");
        let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(json["collection"], "RT-06");
        assert!(json["meminfo"].is_array(), "meminfo should be present");
    }
}
