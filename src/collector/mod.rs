use std::time::Duration;

pub mod boot;
pub mod cpu;
pub mod memory;
pub mod process;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ProbeOutcome {
    pub available: bool,
    pub degraded: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub struct CollectionOutcome {
    pub status: CollectionStatus,
    pub duration: Duration,
    pub file_size: u64,
    pub items_total: Option<u64>,
    pub items_collected: Option<u64>,

    pub mem_total_kb: Option<i64>,
    pub mem_available_kb: Option<i64>,
    pub hostname: Option<String>,
    pub kernel_version: Option<String>,
    pub boot_id: Option<String>,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum CollectionStatus {
    Ok,
    Truncated { reason: String },
    Degraded { missing: Vec<String> },
    Failed { reason: String },
    TimedOut,
}

pub enum CollectionTask {
    Boot(boot::Boot),
    Cpu(cpu::Cpu),
    Memory(memory::Memory),
    Process(process::Process),
}

impl CollectionTask {
    pub fn id(&self) -> &'static str {
        match self {
            CollectionTask::Boot(_) => "RT-01",
            CollectionTask::Cpu(_) => "RT-05",
            CollectionTask::Memory(_) => "RT-06",
            CollectionTask::Process(_) => "RT-04",
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            CollectionTask::Boot(_) => "boot.json",
            CollectionTask::Cpu(_) => "cpu.json",
            CollectionTask::Memory(_) => "memory.json",
            CollectionTask::Process(_) => "processes.jsonl",
        }
    }

    pub fn item_timeout(&self) -> Duration {
        match self {
            CollectionTask::Process(_) => Duration::from_secs(10),
            _ => Duration::from_secs(2),
        }
    }

    pub async fn probe(&self) -> ProbeOutcome {
        match self {
            CollectionTask::Boot(c) => c.probe().await,
            CollectionTask::Cpu(c) => c.probe().await,
            CollectionTask::Memory(c) => c.probe().await,
            CollectionTask::Process(c) => c.probe().await,
        }
    }
}
