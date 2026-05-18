use std::path::PathBuf;
use std::time::Instant;

use rebootsnap::collector::{all_tasks, CollectionStatus};
use rebootsnap::fs::FsRoots;
use rebootsnap::output::OutputDir;

fn fixture_roots(name: &str) -> FsRoots {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    FsRoots {
        proc: base.join("proc"),
        sys: base.join("sys"),
        dev: base.join("dev"),
        etc: base.join("etc"),
        run: base.join("run"),
    }
}

async fn run_collectors(roots: &FsRoots) -> Vec<(String, bool, String)> {
    let output_dir = tempfile::tempdir().expect("tempdir");
    let output = OutputDir::from_existing(output_dir.path().to_path_buf());

    let tasks = all_tasks();
    let mut results = Vec::new();

    for task in &tasks {
        let probe = task.probe(roots).await;
        let outcome = task.collect(&output, &probe).await;
        let status = match &outcome.status {
            CollectionStatus::Ok => "ok",
            CollectionStatus::Truncated { .. } => "truncated",
            CollectionStatus::Degraded { .. } => "degraded",
            CollectionStatus::Failed { .. } => "failed",
            CollectionStatus::TimedOut => "timed_out",
        };
        results.push((task.id().to_string(), outcome.file_size > 0, status.to_string()));
    }

    results
}

#[tokio::test]
async fn test_normal_scenario_all_collectors_produce_output() {
    let roots = fixture_roots("normal");
    let results = run_collectors(&roots).await;

    let mut failed: Vec<String> = Vec::new();
    for (id, has_output, status) in &results {
        if !has_output && status != "degraded" && status != "failed" {
            failed.push(format!("{id}: no output, status={status}"));
        }
    }

    assert!(
        failed.is_empty(),
        "collectors without output (non-degraded): {failed:?}"
    );

    let ok_count = results.iter().filter(|(_, _, s)| s == "ok").count();
    let degraded_count = results.iter().filter(|(_, _, s)| s == "degraded").count();
    let total = results.len();
    assert_eq!(
        total, 21,
        "expected 21 collectors, got {total}"
    );
    assert!(
        ok_count + degraded_count >= 18,
        "too many failures: ok={ok_count} degraded={degraded_count}"
    );
}

#[tokio::test]
async fn test_partial_scenario_degradation_marks_correctly() {
    let roots = fixture_roots("partial");
    let results = run_collectors(&roots).await;

    let mut not_ok: Vec<String> = Vec::new();
    for (id, _, status) in &results {
        if status != "ok" {
            not_ok.push(format!("{id}:{status}"));
        }
    }

    assert!(
        not_ok.iter().any(|s| s.starts_with("RT-03:")),
        "RT-03 (systemd) must not be ok without D-Bus socket, got: {not_ok:?}"
    );
    assert!(
        not_ok.iter().any(|s| s.starts_with("RT-13:")),
        "RT-13 (netfilter) must not be ok with missing conntrack files, got: {not_ok:?}"
    );

    assert!(
        not_ok.len() >= 2,
        "expected at least 2 non-ok collectors, got {not_ok:?}"
    );

    for (id, _has_output, status) in &results {
        assert_ne!(
            status, "timed_out",
            "{id} timed out unexpectedly"
        );
    }
}

#[tokio::test]
async fn test_malformed_scenario_no_crash() {
    let roots = fixture_roots("malformed");
    let results = run_collectors(&roots).await;

    for (id, _, status) in &results {
        assert_ne!(
            status, "timed_out",
            "{id} timed out with malformed data"
        );
    }

    let ok_count = results.iter().filter(|(_, _, s)| s == "ok").count();
    let degraded_count = results.iter().filter(|(_, _, s)| s == "degraded").count();
    assert!(
        ok_count + degraded_count >= 10,
        "too many failures with malformed data: ok={ok_count} degraded={degraded_count}"
    );
}

#[tokio::test]
async fn test_global_timeout_graceful_exit() {
    let roots = fixture_roots("normal");
    let output_dir = tempfile::tempdir().expect("tempdir");
    let output = OutputDir::from_existing(output_dir.path().to_path_buf());

    let tasks = all_tasks();
    let mut probes = Vec::new();
    for task in tasks.iter() {
        probes.push(task.probe(&roots).await);
    }

    let start = Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            let mut set = tokio::task::JoinSet::new();
            for (task, probe) in tasks.into_iter().zip(probes) {
                let root = output.root().to_path_buf();
                set.spawn(async move {
                    let o = OutputDir::from_existing(root);
                    tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        task.collect(&o, &probe),
                    )
                    .await
                    .unwrap_or_else(|_| rebootsnap::collector::CollectionOutcome {
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
                    })
                });
            }
            while let Some(r) = set.join_next().await {
                let _ = r;
            }
        },
    )
    .await;

    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "collectors should complete within global timeout"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(25),
        "should finish well under 25s, took {elapsed:?}"
    );
}

#[tokio::test]
async fn test_constrained_scenario_degrades_gracefully() {
    let roots = fixture_roots("constrained");
    let results = run_collectors(&roots).await;

    for (_id, _has_output, status) in &results {
        assert_ne!(status, "timed_out", "collector timed out in constrained scenario");
    }

    let failed_count = results.iter().filter(|(_, _, s)| s == "failed").count();
    assert!(failed_count <= 3, "too many failures in constrained scenario: {failed_count}");
}

#[tokio::test]
async fn test_denied_scenario_handles_permission_errors() {
    let roots = fixture_roots("denied");
    let results = run_collectors(&roots).await;

    let events_status = results.iter().find(|(id, _, _)| id == "RT-20").map(|(_, _, s)| s.clone());
    assert!(events_status.is_some(), "RT-20 missing from results");
    let es = events_status.unwrap();
    assert!(es == "degraded" || es == "failed", "RT-20 should be degraded/failed when kmsg unreadable, got {es}");

    let systemd_status = results.iter().find(|(id, _, _)| id == "RT-03").map(|(_, _, s)| s.clone());
    assert!(systemd_status.is_some_and(|s| s != "ok"), "RT-03 should not be ok without D-Bus");

    for (_id, _has_output, status) in &results {
        assert_ne!(status, "timed_out", "collector timed out");
    }
}
