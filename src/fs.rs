use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FsRoots {
    pub proc: PathBuf,
    pub sys: PathBuf,
    pub dev: PathBuf,
    pub etc: PathBuf,
    pub run: PathBuf,
}

impl Default for FsRoots {
    fn default() -> Self {
        Self {
            proc: PathBuf::from("/proc"),
            sys: PathBuf::from("/sys"),
            dev: PathBuf::from("/dev"),
            etc: PathBuf::from("/etc"),
            run: PathBuf::from("/run"),
        }
    }
}

impl FsRoots {
    pub fn resolve(&self, path: &str) -> PathBuf {
        if let Some(rest) = path.strip_prefix("/proc") {
            self.proc.join(rest.trim_start_matches('/'))
        } else if let Some(rest) = path.strip_prefix("/sys") {
            self.sys.join(rest.trim_start_matches('/'))
        } else if let Some(rest) = path.strip_prefix("/dev") {
            self.dev.join(rest.trim_start_matches('/'))
        } else if let Some(rest) = path.strip_prefix("/etc") {
            self.etc.join(rest.trim_start_matches('/'))
        } else if let Some(rest) = path.strip_prefix("/run") {
            self.run.join(rest.trim_start_matches('/'))
        } else {
            PathBuf::from(path)
        }
    }

    pub fn exists(&self, path: &str) -> bool {
        Path::new(&self.resolve(path)).exists()
    }
}
