use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CollectError {
    #[error("interface unavailable: {0}")]
    Unavailable(String),

    #[error("partially degraded: {0:?}")]
    Degraded(Vec<String>),

    #[error("output limit reached: {0}")]
    Truncated(String),

    #[error("collection timed out")]
    TimedOut,

    #[error("collection failed: {0}")]
    Failed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum OutputError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("output exceeds 64 MiB limit")]
    SizeLimit,

    #[error("item count exceeds 50000 limit")]
    ItemCountLimit,
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ArchiveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tar error: {0}")]
    Tar(String),

    #[error("gzip error: {0}")]
    Gzip(String),
}
