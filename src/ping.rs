use anyhow::{anyhow, Result};
use colored::Colorize;
use socket2::{Domain, Protocol, Socket, Type};
use std::mem::MaybeUninit;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::util::parse_target;

/// Structured per-reply record.
#[derive(Clone, Copy, Debug)]
pub struct PingReply {
    pub seq: u32,
    pub ms: f64,
}

/// Aggregated probe result consumed by the CLI's printed stats and by the
/// script binding's return map.
#[derive(Debug)]
pub struct PingResult {
    pub protocol: &'static str, // "tcp" | "icmp"
    pub host: String,
    pub resolved_ip: Option<IpAddr>,
    pub port: Option<u16>,
    pub sent: u32,
    pub received: u32,
    pub loss_pct: u32,
    pub replies: Vec<PingReply>,
}

impl PingResult {
    pub fn min_ms(&self) -> Option<f64> {
        self.replies
            .iter()
            .map(|r| r.ms)
            .fold(None, |acc: Option<f64>, x| {
                Some(acc.map_or(x, |a| a.min(x)))
            })
    }
    pub fn max_ms(&self) -> Option<f64> {
        self.replies
            .iter()
            .map(|r| r.ms)
            .fold(None, |acc: Option<f64>, x| {
                Some(acc.map_or(x, |a| a.max(x)))
            })
    }
    pub fn avg_ms(&self) -> Option<f64> {
        if self.replies.is_empty() {
            None
        } else {
            Some(self.replies.iter().map(|r| r.ms).sum::<f64>() / self.replies.len() as f64)
        }
    }
}

pub fn run(input: &str, count: u32) -> Result<()> {
    let (host, port) = parse_target(input);
    if let Some(port) = port {
        let result = tcp_probe(&host, port, count, true)?;
        print_tcp_stats(&result);
        Ok(())
    } else {
        let result = icmp_probe(&host, count, true)?;
        print_icmp_stats(&result);
        Ok(())
    }
}

/// Run a ping and return structured results without printing. Used by the
/// `ping()` script binding. For TCP (host:port) or ICMP (bare host).
pub fn probe(input: &str, count: u32) -> Result<PingResult> {
    let (host, port) = parse_target(input);
    if let Some(port) = port {
        tcp_probe(&host, port, count, false)
    } else {
        icmp_probe(&host, count, false)
    }
}

// ── TCP ping ─────────────────────────────────────────────────────────────────

fn tcp_probe(host: &str, port: u16, count: u32, emit_per_reply: bool) -> Result<PingResult> {
    let addr: SocketAddr = format!("{host}:{port}")
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("Could not resolve {host}"))?;

    if emit_per_reply {
        println!("TCP ping to {}:{}", host.bold(), port);
        println!("{}", "═".repeat(50));
    }

    let mut replies: Vec<PingReply> = Vec::new();
    let mut failures = 0u32;

    for seq in 0..count {
        let start = Instant::now();
        match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
            Ok(_) => {
                let rtt = start.elapsed().as_secs_f64() * 1000.0;
                if emit_per_reply {
                    println!(
                        "Connected to {}:{}: seq={} time={:.3}ms",
                        host, port, seq, rtt
                    );
                }
                replies.push(PingReply { seq, ms: rtt });
            }
            Err(e) => {
                if emit_per_reply {
                    println!("seq={}: {}", seq, e.to_string().red());
                }
                failures += 1;
            }
        }
        if seq + 1 < count {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    let received = replies.len() as u32;
    let loss_pct = if count > 0 { 100 * failures / count } else { 0 };
    Ok(PingResult {
        protocol: "tcp",
        host: host.to_string(),
        resolved_ip: Some(addr.ip()),
        port: Some(port),
        sent: count,
        received,
        loss_pct,
        replies,
    })
}

fn print_tcp_stats(r: &PingResult) {
    println!();
    println!(
        "--- {}:{} TCP ping statistics ---",
        r.host,
        r.port.unwrap_or(0)
    );
    println!(
        "{} attempts, {} connected, {}% failure rate",
        r.sent, r.received, r.loss_pct
    );
    if let (Some(min), Some(avg), Some(max)) = (r.min_ms(), r.avg_ms(), r.max_ms()) {
        println!("round-trip min/avg/max = {min:.3}/{avg:.3}/{max:.3} ms");
    }
}

// ── ICMP ping ─────────────────────────────────────────────────────────────────

fn icmp_probe(host: &str, count: u32, emit_per_reply: bool) -> Result<PingResult> {
    let addr: SocketAddr = format!("{host}:0")
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("Could not resolve {host}"))?;

    let ip = addr.ip();
    let target = socket2::SockAddr::from(SocketAddr::new(ip, 0));

    // SOCK_DGRAM + ICMP works without root on macOS (since 10.14).
    // On Linux it requires net.ipv4.ping_group_range or root.
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            anyhow!(
                "ICMP ping requires elevated privileges on this system.\n\
                 Tip: use --ping {host}:<port> to do a TCP ping instead."
            )
        } else {
            anyhow!("Failed to create ICMP socket: {e}")
        }
    })?;

    socket.set_read_timeout(Some(Duration::from_secs(2)))?;

    let pid = (std::process::id() & 0xffff) as u16;

    if emit_per_reply {
        println!("PING {} ({}): 16 data bytes", host.bold(), ip);
        println!("{}", "═".repeat(50));
    }

    let mut replies: Vec<PingReply> = Vec::new();

    for seq in 0..count {
        let packet = build_icmp_echo(pid, seq as u16);
        let start = Instant::now();

        if let Err(e) = socket.send_to(&packet, &target) {
            if emit_per_reply {
                println!("seq={seq}: send error: {e}");
            }
            continue;
        }

        let mut buf = [MaybeUninit::uninit(); 512];
        match socket.recv_from(&mut buf) {
            Ok((n, _from)) => {
                let rtt = start.elapsed().as_secs_f64() * 1000.0;
                let data: &[u8] =
                    unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };

                let offset = ip_header_len(data);
                let icmp = &data[offset..];

                if icmp.len() >= 8 && icmp[0] == 0 {
                    let reply_id = u16::from_be_bytes([icmp[4], icmp[5]]);
                    let reply_seq = u16::from_be_bytes([icmp[6], icmp[7]]);
                    if reply_id == pid {
                        if emit_per_reply {
                            println!(
                                "16 bytes from {ip}: icmp_seq={reply_seq} time={:.3}ms",
                                rtt
                            );
                        }
                        replies.push(PingReply {
                            seq: reply_seq as u32,
                            ms: rtt,
                        });
                    }
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                if emit_per_reply {
                    println!("Request timeout for icmp_seq={seq}");
                }
            }
            Err(e) => return Err(anyhow!("Receive error: {e}")),
        }

        if seq + 1 < count {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    let received = replies.len() as u32;
    let loss_pct = if count > 0 {
        100 * (count - received) / count
    } else {
        0
    };
    Ok(PingResult {
        protocol: "icmp",
        host: host.to_string(),
        resolved_ip: Some(ip),
        port: None,
        sent: count,
        received,
        loss_pct,
        replies,
    })
}

fn print_icmp_stats(r: &PingResult) {
    println!();
    println!("--- {} ping statistics ---", r.host);
    println!(
        "{} packets transmitted, {} received, {}% packet loss",
        r.sent, r.received, r.loss_pct
    );
    if let (Some(min), Some(avg), Some(max)) = (r.min_ms(), r.avg_ms(), r.max_ms()) {
        println!("round-trip min/avg/max = {min:.3}/{avg:.3}/{max:.3} ms");
    }
}

fn build_icmp_echo(id: u16, seq: u16) -> Vec<u8> {
    let mut pkt = vec![
        8u8, 0, 0, 0, // type=8 (echo request), code=0, checksum placeholder
        (id >> 8) as u8,
        id as u8,
        (seq >> 8) as u8,
        seq as u8,
        // 16 bytes of payload
        b'c', b'u', b'r', b'l', b'c', b'l', b'o', b'n',
        b'e', b'_', b'p', b'i', b'n', b'g', b'_', b'_',
    ];
    let cs = icmp_checksum(&pkt);
    pkt[2] = (cs >> 8) as u8;
    pkt[3] = cs as u8;
    pkt
}

fn icmp_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    for chunk in data.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], 0])
        };
        sum += word as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// Returns the IP header length in bytes if an IPv4 header is present, else 0.
fn ip_header_len(data: &[u8]) -> usize {
    if !data.is_empty() && (data[0] >> 4) == 4 {
        (data[0] & 0x0f) as usize * 4
    } else {
        0
    }
}
