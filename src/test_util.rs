use std::path::PathBuf;

use crate::fs::FsRoots;
use crate::output::OutputDir;

pub struct TestContext {
    pub roots: FsRoots,
    pub output: OutputDir,
    #[allow(dead_code)]
    pub tmp: tempfile::TempDir,
}

pub fn setup_test(fixture: &str) -> TestContext {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let roots = FsRoots {
        proc: base.join("proc"),
        sys: base.join("sys"),
        dev: base.join("dev"),
        etc: base.join("etc"),
        run: base.join("run"),
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let output = OutputDir::from_existing(tmp.path().to_path_buf());
    TestContext { roots, output, tmp }
}
