use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutputError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("output exceeds 64 MiB limit")]
    SizeLimit,

}

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tar error: {0}")]
    Tar(String),

    #[error("gzip error: {0}")]
    Gzip(String),
}
