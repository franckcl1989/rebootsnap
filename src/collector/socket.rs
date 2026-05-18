use serde::Serialize;
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::Instant;

use crate::collector::util::{read_trimmed, SCHEMA_VERSION};
use crate::collector::{probe_files, CollectionOutcome, CollectionStatus, ProbeOutcome};
use crate::fs::FsRoots;
use crate::output::OutputDir;

#[derive(Clone)]
pub struct Socket;

#[derive(Serialize)]
struct ListeningEntry {
    local_addr: String,
    local_port: u16,
    proto: String,
    inode: u32,
}

#[derive(Serialize)]
struct SocketStateCounts {
    established: u32,
    close_wait: u32,
    time_wait: u32,
    listen: u32,
    syn_sent: u32,
    other: u32,
}

#[derive(Serialize)]
struct SocketRecord {
    schema_version: &'static str,
    collection: &'static str,
    tcp: Option<String>,
    tcp6: Option<String>,
    udp: Option<String>,
    udp6: Option<String>,
    unix: Option<String>,
    raw: Option<String>,
    raw6: Option<String>,
    snmp: Option<String>,
    snmp6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tcp_connections: Option<Vec<TcpConnEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tcp6_connections: Option<Vec<TcpConnEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    udp_connections: Option<Vec<UdpConnEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    udp6_connections: Option<Vec<UdpConnEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    listening_sockets: Option<Vec<ListeningEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    socket_state_counts: Option<SocketStateCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    listening_ports: Option<Vec<u16>>,
}

#[derive(Serialize, Clone)]
struct TcpConnEntry {
    local_addr: String,
    local_port: u16,
    remote_addr: String,
    remote_port: u16,
    state_hex: String,
    uid: u32,
    inode: u32,
}

#[derive(Serialize, Clone)]
struct UdpConnEntry {
    local_addr: String,
    local_port: u16,
    remote_addr: String,
    remote_port: u16,
    uid: u32,
    inode: u32,
}

const FILES: &[&str] = &[
    "/proc/net/tcp",
    "/proc/net/tcp6",
    "/proc/net/udp",
    "/proc/net/udp6",
    "/proc/net/unix",
    "/proc/net/raw",
    "/proc/net/raw6",
    "/proc/net/snmp",
    "/proc/net/snmp6",
];

const UNSUPPORTED_IF_MISSING: &[&str] = &[];

fn hex_to_ipv4(hex: &str) -> String {
    if hex.len() < 8 {
        return hex.to_string();
    }
    let Ok(addr) = u32::from_str_radix(hex, 16) else {
        return hex.to_string();
    };
    let ip = Ipv4Addr::from(addr.to_be());
    ip.to_string()
}

fn parse_tcp_line(line: &str) -> Option<TcpConnEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }
    let local_parts: Vec<&str> = parts[1].split(':').collect();
    let remote_parts: Vec<&str> = parts[2].split(':').collect();
    if local_parts.len() != 2 || remote_parts.len() != 2 {
        return None;
    }
    let local_port = u16::from_str_radix(local_parts[1], 16).ok()?;
    let remote_port = u16::from_str_radix(remote_parts[1], 16).ok()?;
    let uid = parts[7].parse().ok()?;
    let inode = parts[9].parse().ok()?;
    Some(TcpConnEntry {
        local_addr: hex_to_ipv4(local_parts[0]),
        local_port,
        remote_addr: hex_to_ipv4(remote_parts[0]),
        remote_port,
        state_hex: parts[3].to_string(),
        uid,
        inode,
    })
}

fn parse_udp_line(line: &str) -> Option<UdpConnEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }
    let local_parts: Vec<&str> = parts[1].split(':').collect();
    let remote_parts: Vec<&str> = parts[2].split(':').collect();
    if local_parts.len() != 2 || remote_parts.len() != 2 {
        return None;
    }
    let local_port = u16::from_str_radix(local_parts[1], 16).ok()?;
    let remote_port = u16::from_str_radix(remote_parts[1], 16).ok()?;
    let uid = parts[7].parse().ok()?;
    let inode = parts[9].parse().ok()?;
    Some(UdpConnEntry {
        local_addr: hex_to_ipv4(local_parts[0]),
        local_port,
        remote_addr: hex_to_ipv4(remote_parts[0]),
        remote_port,
        uid,
        inode,
    })
}

fn parse_connections(content: &Option<String>, parse_fn: fn(&str) -> Option<TcpConnEntry>) -> Option<Vec<TcpConnEntry>> {
    let lines = content.as_ref()?;
    let mut conns = Vec::new();
    for line in lines.lines().skip(1) {
        if let Some(conn) = parse_fn(line) && conn.inode != 0 {
            conns.push(conn);
        }
        if conns.len() >= 200 {
            break;
        }
    }
    if conns.is_empty() { None } else { Some(conns) }
}

fn parse_udp_connections(content: &Option<String>) -> Option<Vec<UdpConnEntry>> {
    let lines = content.as_ref()?;
    let mut conns = Vec::new();
    for line in lines.lines().skip(1) {
        if let Some(conn) = parse_udp_line(line) && conn.inode != 0 {
            conns.push(conn);
        }
        if conns.len() >= 200 {
            break;
        }
    }
    if conns.is_empty() { None } else { Some(conns) }
}

fn collect_listening_and_counts(
    tcp: &[TcpConnEntry],
    tcp6: &[TcpConnEntry],
) -> (Vec<ListeningEntry>, SocketStateCounts, Vec<u16>) {
    let mut listening = Vec::new();
    let mut counts = SocketStateCounts {
        established: 0,
        close_wait: 0,
        time_wait: 0,
        listen: 0,
        syn_sent: 0,
        other: 0,
    };
    let mut ports: BTreeMap<u16, bool> = BTreeMap::new();

    let mut process = |conn: &TcpConnEntry, proto: &str| {
        match conn.state_hex.as_str() {
            "01" => counts.established += 1,
            "08" => counts.close_wait += 1,
            "06" => counts.time_wait += 1,
            "0A" => {
                counts.listen += 1;
                if ports.len() < 500 {
                    ports.insert(conn.local_port, true);
                }
                listening.push(ListeningEntry {
                    local_addr: conn.local_addr.clone(),
                    local_port: conn.local_port,
                    proto: proto.to_string(),
                    inode: conn.inode,
                });
            }
            "02" => counts.syn_sent += 1,
            _ => counts.other += 1,
        }
    };

    for conn in tcp { process(conn, "tcp"); }
    for conn in tcp6 { process(conn, "tcp6"); }

    let listening_ports: Vec<u16> = ports.into_keys().collect();
    (listening, counts, listening_ports)
}

impl Socket {
    pub async fn probe(&self, roots: &FsRoots) -> ProbeOutcome {
        probe_files(roots, FILES, "all net socket files missing")
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

        let tcp_raw = read_trimmed(&probe.roots, FILES[0]);
        let tcp6_raw = read_trimmed(&probe.roots, FILES[1]);
        let udp_raw = read_trimmed(&probe.roots, FILES[2]);
        let udp6_raw = read_trimmed(&probe.roots, FILES[3]);

        let tcp_connections = parse_connections(&tcp_raw, parse_tcp_line);
        let tcp6_connections = parse_connections(&tcp6_raw, parse_tcp_line);

        let (listening_sockets, socket_state_counts, listening_ports) = collect_listening_and_counts(
            &tcp_connections.clone().unwrap_or_default(),
            &tcp6_connections.clone().unwrap_or_default(),
        );

        let record = SocketRecord {
            schema_version: SCHEMA_VERSION,
            collection: "RT-12",
            tcp: tcp_raw.clone(),
            tcp6: tcp6_raw.clone(),
            udp: udp_raw.clone(),
            udp6: udp6_raw.clone(),
            unix: read_trimmed(&probe.roots, FILES[4]),
            raw: read_trimmed(&probe.roots, FILES[5]),
            raw6: read_trimmed(&probe.roots, FILES[6]),
            snmp: read_trimmed(&probe.roots, FILES[7]),
            snmp6: read_trimmed(&probe.roots, FILES[8]),
            tcp_connections,
            tcp6_connections,
            udp_connections: parse_udp_connections(&udp_raw),
            udp6_connections: parse_udp_connections(&udp6_raw),
            listening_sockets: if listening_sockets.is_empty() { None } else { Some(listening_sockets) },
            socket_state_counts: Some(socket_state_counts),
            listening_ports: if listening_ports.is_empty() { None } else { Some(listening_ports) },
        };

        let writer = match output.json_writer("sockets.json") {
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
                    mem_total_kb: None,
                    mem_available_kb: None,
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
                    mem_total_kb: None,
                    mem_available_kb: None,
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
                CollectionStatus::Partial {
                    degrading: degraded,
                }
            } else if !unsupported.is_empty() {
                CollectionStatus::Unsupported { reason: unsupported.join(", ") }
            } else {
                CollectionStatus::Ok
            },
            duration: start.elapsed(),
            file_size: size,
            items_total: None,
            items_collected: None,
            mem_total_kb: None,
            mem_available_kb: None,
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
        let probe = Socket.probe(&ctx.roots).await;
        if !probe.available {
            eprintln!("Socket probe unavailable in mock - skipping");
            return;
        }
        let outcome = Socket.collect(&ctx.output, &probe).await;
        if let crate::collector::CollectionStatus::Failed { reason } = &outcome.status {
            panic!("collect failed: {reason}");
        }
        assert!(outcome.file_size > 0, "no output produced");
    }
}
