use anyhow::{anyhow, Result};
use colored::Colorize;
use socket2::{Domain, Protocol, Socket, Type};
use std::mem::MaybeUninit;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::util::parse_target;

pub fn run(input: &str, count: u32) -> Result<()> {
    let (host, port) = parse_target(input);
    if let Some(port) = port {
        tcp_ping(&host, port, count)
    } else {
        icmp_ping(&host, count)
    }
}

// ── TCP ping ─────────────────────────────────────────────────────────────────

fn tcp_ping(host: &str, port: u16, count: u32) -> Result<()> {
    let addr: SocketAddr = format!("{host}:{port}")
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("Could not resolve {host}"))?;

    println!("TCP ping to {}:{}", host.bold(), port);
    println!("{}", "═".repeat(50));

    let mut rtts: Vec<f64> = Vec::new();
    let mut failures = 0u32;

    for seq in 0..count {
        let start = Instant::now();
        match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
            Ok(_) => {
                let rtt = start.elapsed().as_secs_f64() * 1000.0;
                println!(
                    "Connected to {}:{}: seq={} time={:.3}ms",
                    host, port, seq, rtt
                );
                rtts.push(rtt);
            }
            Err(e) => {
                println!("seq={}: {}", seq, e.to_string().red());
                failures += 1;
            }
        }
        if seq + 1 < count {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    println!();
    println!("--- {}:{} TCP ping statistics ---", host, port);
    let received = rtts.len() as u32;
    let loss_pct = if count > 0 { 100 * failures / count } else { 0 };
    println!("{count} attempts, {received} connected, {loss_pct}% failure rate");
    if !rtts.is_empty() {
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;
        println!("round-trip min/avg/max = {min:.3}/{avg:.3}/{max:.3} ms");
    }
    Ok(())
}

// ── ICMP ping ─────────────────────────────────────────────────────────────────

fn icmp_ping(host: &str, count: u32) -> Result<()> {
    let addr: SocketAddr = format!("{host}:0")
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("Could not resolve {host}"))?;

    let ip = addr.ip();
    let target = socket2::SockAddr::from(SocketAddr::new(ip, 0));

    // SOCK_DGRAM + ICMP works without root on macOS (since 10.14).
    // On Linux it requires net.ipv4.ping_group_range or root.
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))
        .map_err(|e| {
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

    println!("PING {} ({}): 16 data bytes", host.bold(), ip);
    println!("{}", "═".repeat(50));

    let mut rtts: Vec<f64> = Vec::new();

    for seq in 0..count {
        let packet = build_icmp_echo(pid, seq as u16);
        let start = Instant::now();

        if let Err(e) = socket.send_to(&packet, &target) {
            println!("seq={seq}: send error: {e}");
            continue;
        }

        let mut buf = [MaybeUninit::uninit(); 512];
        match socket.recv_from(&mut buf) {
            Ok((n, _from)) => {
                let rtt = start.elapsed().as_secs_f64() * 1000.0;
                let data: &[u8] =
                    unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };

                // Skip IP header if present (DGRAM on macOS includes it)
                let offset = ip_header_len(data);
                let icmp = &data[offset..];

                if icmp.len() >= 8 && icmp[0] == 0 {
                    // type 0 = echo reply
                    let reply_id = u16::from_be_bytes([icmp[4], icmp[5]]);
                    let reply_seq = u16::from_be_bytes([icmp[6], icmp[7]]);
                    if reply_id == pid {
                        println!(
                            "16 bytes from {ip}: icmp_seq={reply_seq} time={:.3}ms",
                            rtt
                        );
                        rtts.push(rtt);
                    }
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                println!("Request timeout for icmp_seq={seq}");
            }
            Err(e) => return Err(anyhow!("Receive error: {e}")),
        }

        if seq + 1 < count {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    println!();
    println!("--- {host} ping statistics ---");
    let received = rtts.len() as u32;
    let loss_pct = if count > 0 {
        100 * (count - received) / count
    } else {
        0
    };
    println!("{count} packets transmitted, {received} received, {loss_pct}% packet loss");
    if !rtts.is_empty() {
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;
        println!("round-trip min/avg/max = {min:.3}/{avg:.3}/{max:.3} ms");
    }
    Ok(())
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
