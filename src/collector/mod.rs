use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;

use crate::output::OutputDir;

pub mod boot;
pub mod cpu;
pub mod memory;
pub mod process;

pub struct ProbeResult {
    pub status: ProbeStatus,
    pub detail: HashMap<String, String>,
}

pub enum ProbeStatus {
    Available,
    Degraded(Vec<String>),
    Unavailable(String),
}

pub struct CollectResult {
    pub status: CollectStatus,
    pub duration: Duration,
    pub file_size: u64,
    pub items_total: Option<u64>,
    pub items_collected: Option<u64>,
    pub error_reason: Option<String>,
}

pub enum CollectStatus {
    Ok,
    Truncated(String),
    Degraded(Vec<String>),
    Failed(String),
    TimedOut,
}

#[async_trait]
pub trait Collector: Send + Sync {
    fn id(&self) -> &'static str;

    fn filename(&self) -> &'static str;

    fn required(&self) -> bool {
        true
    }

    async fn probe(&self) -> ProbeResult;

    fn item_timeout(&self) -> Duration {
        Duration::from_secs(2)
    }

    async fn collect(
        &self,
        output: &OutputDir,
        probe: &ProbeResult,
    ) -> CollectResult;
}
