use crate::fs::FsRoots;

pub const SCHEMA_VERSION: &str = "0.1";

pub fn read_trimmed(roots: &FsRoots, path: &str) -> Option<String> {
    let s = std::fs::read_to_string(roots.resolve(path)).ok()?;
    Some(s.trim().to_string())
}
