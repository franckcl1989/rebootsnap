use std::time::Duration;

pub mod block;
pub mod boot;
pub mod cache;
pub mod cpu;
pub mod device;
pub mod events;
pub mod fd;
pub mod ipc_ns_cg;
pub mod kernel;
pub mod memory;
pub mod mount;
pub mod netdev;
pub mod netfilter;
pub mod power;
pub mod process;
pub mod security;
pub mod session;
pub mod socket;
pub mod systemd;
pub mod time;
pub mod tmpfs;

use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Debug, Clone)]
pub struct ProbeOutcome {
    pub available: bool,
    pub degraded: Vec<String>,
    pub reason: Option<String>,
    pub roots: FsRoots,
}

pub fn probe_files(roots: &FsRoots, files: &[&str], all_missing_msg: &str) -> ProbeOutcome {
    let degraded: Vec<String> = files
        .iter()
        .filter(|f| !roots.exists(f))
        .map(|s| s.to_string())
        .collect();

    if degraded.len() == files.len() {
        ProbeOutcome {
            available: false,
            degraded: Vec::new(),
            reason: Some(all_missing_msg.into()),
            roots: roots.clone(),
        }
    } else if degraded.is_empty() {
        ProbeOutcome {
            available: true,
            degraded: Vec::new(),
            reason: None,
            roots: roots.clone(),
        }
    } else {
        ProbeOutcome {
            available: true,
            degraded,
            reason: None,
            roots: roots.clone(),
        }
    }
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
pub enum CollectionStatus {
    Ok,
    Truncated { reason: String },
    Degraded { missing: Vec<String> },
    Failed { reason: String },
    TimedOut,
}

#[derive(Clone)]
pub enum CollectionTask {
    Block(block::Block),
    Boot(boot::Boot),
    Cache(cache::Cache),
    Cpu(cpu::Cpu),
    Device(device::Device),
    Events(events::Events),
    Fd(fd::Fd),
    IpcNsCg(ipc_ns_cg::IpcNsCg),
    Kernel(kernel::Kernel),
    Memory(memory::Memory),
    Mount(mount::Mount),
    Netdev(netdev::Netdev),
    Netfilter(netfilter::Netfilter),
    Power(power::Power),
    Process(process::Process),
    Security(security::Security),
    Session(session::Session),
    Socket(socket::Socket),
    Systemd(systemd::Systemd),
    Time(time::Time),
    Tmpfs(tmpfs::Tmpfs),
}

impl CollectionTask {
    pub fn id(&self) -> &'static str {
        match self {
            CollectionTask::Block(_) => "RT-10",
            CollectionTask::Boot(_) => "RT-01",
            CollectionTask::Cache(_) => "RT-21",
            CollectionTask::Cpu(_) => "RT-05",
            CollectionTask::Device(_) => "RT-17",
            CollectionTask::Events(_) => "RT-20",
            CollectionTask::Fd(_) => "RT-07",
            CollectionTask::IpcNsCg(_) => "RT-14",
            CollectionTask::Kernel(_) => "RT-02",
            CollectionTask::Memory(_) => "RT-06",
            CollectionTask::Mount(_) => "RT-09",
            CollectionTask::Netdev(_) => "RT-11",
            CollectionTask::Netfilter(_) => "RT-13",
            CollectionTask::Power(_) => "RT-18",
            CollectionTask::Process(_) => "RT-04",
            CollectionTask::Security(_) => "RT-16",
            CollectionTask::Session(_) => "RT-15",
            CollectionTask::Socket(_) => "RT-12",
            CollectionTask::Systemd(_) => "RT-03",
            CollectionTask::Time(_) => "RT-19",
            CollectionTask::Tmpfs(_) => "RT-08",
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            CollectionTask::Block(_) => "block.json",
            CollectionTask::Boot(_) => "boot.json",
            CollectionTask::Cache(_) => "caches.json",
            CollectionTask::Cpu(_) => "cpu.json",
            CollectionTask::Device(_) => "devices.json",
            CollectionTask::Events(_) => "dmesg.txt",
            CollectionTask::Fd(_) => "fds.json",
            CollectionTask::IpcNsCg(_) => "ipc_ns_cg.json",
            CollectionTask::Kernel(_) => "kernel.json",
            CollectionTask::Memory(_) => "memory.json",
            CollectionTask::Mount(_) => "mounts.json",
            CollectionTask::Netdev(_) => "netdev.json",
            CollectionTask::Netfilter(_) => "netfilter.json",
            CollectionTask::Power(_) => "power.json",
            CollectionTask::Process(_) => "processes.jsonl",
            CollectionTask::Security(_) => "security.json",
            CollectionTask::Session(_) => "sessions.json",
            CollectionTask::Socket(_) => "sockets.json",
            CollectionTask::Systemd(_) => "systemd.json",
            CollectionTask::Time(_) => "time.json",
            CollectionTask::Tmpfs(_) => "tmpfs.json",
        }
    }

    pub fn item_timeout(&self) -> Duration {
        match self {
            CollectionTask::Process(_) => Duration::from_secs(10),
            CollectionTask::Memory(_) => Duration::from_secs(10),
            CollectionTask::Netdev(_) => Duration::from_secs(10),
            CollectionTask::Fd(_) => Duration::from_secs(10),
            CollectionTask::Mount(_) => Duration::from_secs(10),
            CollectionTask::Netfilter(_) => Duration::from_secs(5),
            CollectionTask::Socket(_) => Duration::from_secs(10),
            CollectionTask::Systemd(_) => Duration::from_secs(5),
            _ => Duration::from_secs(2),
        }
    }

    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        match self {
            CollectionTask::Block(c) => c.probe(roots).await,
            CollectionTask::Boot(c) => c.probe(roots).await,
            CollectionTask::Cache(c) => c.probe(roots).await,
            CollectionTask::Cpu(c) => c.probe(roots).await,
            CollectionTask::Device(c) => c.probe(roots).await,
            CollectionTask::Events(c) => c.probe(roots).await,
            CollectionTask::Fd(c) => c.probe(roots).await,
            CollectionTask::IpcNsCg(c) => c.probe(roots).await,
            CollectionTask::Kernel(c) => c.probe(roots).await,
            CollectionTask::Memory(c) => c.probe(roots).await,
            CollectionTask::Mount(c) => c.probe(roots).await,
            CollectionTask::Netdev(c) => c.probe(roots).await,
            CollectionTask::Netfilter(c) => c.probe(roots).await,
            CollectionTask::Power(c) => c.probe(roots).await,
            CollectionTask::Process(c) => c.probe(roots).await,
            CollectionTask::Security(c) => c.probe(roots).await,
            CollectionTask::Session(c) => c.probe(roots).await,
            CollectionTask::Socket(c) => c.probe(roots).await,
            CollectionTask::Systemd(c) => c.probe(roots).await,
            CollectionTask::Time(c) => c.probe(roots).await,
            CollectionTask::Tmpfs(c) => c.probe(roots).await,
        }
    }

    pub async fn collect(&self, output: &OutputDir, probe: &ProbeOutcome) -> CollectionOutcome {
        match self {
            CollectionTask::Block(c) => c.collect(output, probe).await,
            CollectionTask::Boot(c) => c.collect(output, probe).await,
            CollectionTask::Cache(c) => c.collect(output, probe).await,
            CollectionTask::Cpu(c) => c.collect(output, probe).await,
            CollectionTask::Device(c) => c.collect(output, probe).await,
            CollectionTask::Events(c) => c.collect(output, probe).await,
            CollectionTask::Fd(c) => c.collect(output, probe).await,
            CollectionTask::IpcNsCg(c) => c.collect(output, probe).await,
            CollectionTask::Kernel(c) => c.collect(output, probe).await,
            CollectionTask::Memory(c) => c.collect(output, probe).await,
            CollectionTask::Mount(c) => c.collect(output, probe).await,
            CollectionTask::Netdev(c) => c.collect(output, probe).await,
            CollectionTask::Netfilter(c) => c.collect(output, probe).await,
            CollectionTask::Power(c) => c.collect(output, probe).await,
            CollectionTask::Process(c) => c.collect(output, probe).await,
            CollectionTask::Security(c) => c.collect(output, probe).await,
            CollectionTask::Session(c) => c.collect(output, probe).await,
            CollectionTask::Socket(c) => c.collect(output, probe).await,
            CollectionTask::Systemd(c) => c.collect(output, probe).await,
            CollectionTask::Time(c) => c.collect(output, probe).await,
            CollectionTask::Tmpfs(c) => c.collect(output, probe).await,
        }
    }
}

pub fn all_tasks() -> [CollectionTask; 21] {
    [
        CollectionTask::Boot(self::boot::Boot),
        CollectionTask::Block(self::block::Block),
        CollectionTask::Cache(self::cache::Cache),
        CollectionTask::Cpu(self::cpu::Cpu),
        CollectionTask::Device(self::device::Device),
        CollectionTask::Events(self::events::Events),
        CollectionTask::Fd(self::fd::Fd),
        CollectionTask::IpcNsCg(self::ipc_ns_cg::IpcNsCg),
        CollectionTask::Kernel(self::kernel::Kernel),
        CollectionTask::Memory(self::memory::Memory),
        CollectionTask::Mount(self::mount::Mount),
        CollectionTask::Netdev(self::netdev::Netdev),
        CollectionTask::Netfilter(self::netfilter::Netfilter),
        CollectionTask::Power(self::power::Power),
        CollectionTask::Process(self::process::Process),
        CollectionTask::Security(self::security::Security),
        CollectionTask::Session(self::session::Session),
        CollectionTask::Socket(self::socket::Socket),
        CollectionTask::Systemd(self::systemd::Systemd),
        CollectionTask::Time(self::time::Time),
        CollectionTask::Tmpfs(self::tmpfs::Tmpfs),
    ]
}
