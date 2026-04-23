//! `tftp://host[:port]/filename` — RFC 1350 TFTP read (RRQ).
//!
//! Hand-rolled over UDP, mirroring `src/ntp_probe.rs`'s byte-level
//! approach. Upload (WRQ) not implemented; download-only for
//! diagnostic use. Block size negotiation (RFC 2348) optional via
//! `--tftp-blksize`.
//!
//! Protocol in brief:
//!   Client -> Server (port 69): RRQ packet [opcode=1, filename, 0, "octet", 0]
//!   Server -> Client (new port): DATA packet [opcode=3, block#, data...]
//!   Client -> Server: ACK [opcode=4, block#]
//!   ...until a DATA packet arrives that's < (blksize+4) bytes long.

use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

const DEFAULT_PORT: u16 = 69;
const DEFAULT_BLKSIZE: usize = 512;
const OP_RRQ: u16 = 1;
const OP_DATA: u16 = 3;
const OP_ACK: u16 = 4;
const OP_ERROR: u16 = 5;
const OP_OACK: u16 = 6;

pub struct TftpProbeOk {
    pub host: String,
    pub port: u16,
    pub filename: String,
    pub blksize: usize,
    pub bytes: Vec<u8>,
    pub connect_ms: f64,
}

pub fn probe(url: &str, timeout_secs: u64, blksize: Option<usize>) -> Result<TftpProbeOk> {
    let (host, port, filename) = parse_url(url)?;
    if filename.is_empty() {
        bail!("tftp: URL must include a filename (e.g. tftp://host/file.bin)");
    }
    let timeout = Duration::from_secs(timeout_secs.max(1));

    let t0 = Instant::now();
    let server_addr: SocketAddr = format!("{host}:{port}")
        .to_socket_addrs()
        .with_context(|| format!("tftp: resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("tftp: no address for {host}:{port}"))?;

    let bind = if server_addr.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
    let sock = UdpSocket::bind(bind).with_context(|| format!("tftp: bind {bind}"))?;
    sock.set_read_timeout(Some(timeout))?;
    sock.set_write_timeout(Some(timeout))?;

    // Build RRQ with optional blksize extension.
    let mut rrq = Vec::with_capacity(64);
    rrq.extend_from_slice(&OP_RRQ.to_be_bytes());
    rrq.extend_from_slice(filename.as_bytes());
    rrq.push(0);
    rrq.extend_from_slice(b"octet");
    rrq.push(0);
    if let Some(bs) = blksize {
        rrq.extend_from_slice(b"blksize");
        rrq.push(0);
        rrq.extend_from_slice(bs.to_string().as_bytes());
        rrq.push(0);
    }
    sock.send_to(&rrq, server_addr).context("tftp: send RRQ")?;
    let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // Server replies from a new ephemeral port. Capture it + negotiated blksize.
    let mut buf = vec![0u8; 65535];
    let mut negotiated_blksize = DEFAULT_BLKSIZE;
    let mut bytes: Vec<u8> = Vec::new();
    let mut next_block: u16 = 1;

    // First packet: DATA (no extension) or OACK (if we asked for blksize).
    let (n, from) = sock.recv_from(&mut buf).map_err(|e| {
        anyhow!("tftp: recv first packet: {e}").context(ProtocolExitCode::OperationTimedOut)
    })?;
    let peer = from;
    let opcode = u16::from_be_bytes([buf[0], buf[1]]);
    match opcode {
        OP_OACK => {
            // Parse for blksize; then ACK block 0 to start the transfer.
            for (k, v) in parse_oack_options(&buf[2..n]) {
                if k.eq_ignore_ascii_case("blksize") {
                    if let Ok(bs) = v.parse::<usize>() {
                        negotiated_blksize = bs;
                    }
                }
            }
            let mut ack = Vec::with_capacity(4);
            ack.extend_from_slice(&OP_ACK.to_be_bytes());
            ack.extend_from_slice(&0u16.to_be_bytes());
            sock.send_to(&ack, peer).context("tftp: send ACK(0)")?;
            // Next loop iteration will grab the first DATA packet.
        }
        OP_DATA => {
            let block = u16::from_be_bytes([buf[2], buf[3]]);
            if block != next_block {
                bail!("tftp: expected DATA block {next_block}, got {block}");
            }
            bytes.extend_from_slice(&buf[4..n]);
            let mut ack = Vec::with_capacity(4);
            ack.extend_from_slice(&OP_ACK.to_be_bytes());
            ack.extend_from_slice(&block.to_be_bytes());
            sock.send_to(&ack, peer).context("tftp: send ACK")?;
            if n < 4 + DEFAULT_BLKSIZE {
                // Short DATA means EOF (blksize hasn't been negotiated yet).
                return Ok(TftpProbeOk {
                    host,
                    port,
                    filename,
                    blksize: negotiated_blksize,
                    bytes,
                    connect_ms,
                });
            }
            next_block = next_block.wrapping_add(1);
        }
        OP_ERROR => {
            let code = u16::from_be_bytes([buf[2], buf[3]]);
            let msg = std::str::from_utf8(&buf[4..n.saturating_sub(1)]).unwrap_or("");
            bail!("tftp: server ERROR code {code}: {msg}");
        }
        other => bail!("tftp: unexpected opcode {other} in first reply"),
    }

    // Main receive loop.
    loop {
        let (n, _) = sock.recv_from(&mut buf).map_err(|e| {
            anyhow!("tftp: recv: {e}").context(ProtocolExitCode::OperationTimedOut)
        })?;
        let opcode = u16::from_be_bytes([buf[0], buf[1]]);
        match opcode {
            OP_DATA => {
                let block = u16::from_be_bytes([buf[2], buf[3]]);
                if block != next_block {
                    // Duplicate; just re-ACK and continue.
                    let mut ack = Vec::with_capacity(4);
                    ack.extend_from_slice(&OP_ACK.to_be_bytes());
                    ack.extend_from_slice(&block.to_be_bytes());
                    sock.send_to(&ack, peer)?;
                    continue;
                }
                let payload = &buf[4..n];
                bytes.extend_from_slice(payload);
                let mut ack = Vec::with_capacity(4);
                ack.extend_from_slice(&OP_ACK.to_be_bytes());
                ack.extend_from_slice(&block.to_be_bytes());
                sock.send_to(&ack, peer)?;
                next_block = next_block.wrapping_add(1);
                if payload.len() < negotiated_blksize {
                    break;
                }
            }
            OP_ERROR => {
                let code = u16::from_be_bytes([buf[2], buf[3]]);
                let msg = std::str::from_utf8(&buf[4..n.saturating_sub(1)]).unwrap_or("");
                bail!("tftp: server ERROR code {code}: {msg}");
            }
            other => bail!("tftp: unexpected opcode {other} mid-transfer"),
        }
    }

    Ok(TftpProbeOk {
        host,
        port,
        filename,
        blksize: negotiated_blksize,
        bytes,
        connect_ms,
    })
}

pub fn run(url: &str, timeout_secs: u64, blksize: Option<usize>) -> Result<()> {
    use std::io::Write;
    let r = probe(url, timeout_secs, blksize)?;
    eprintln!(
        "Fetched {} from {}:{} ({} bytes, blksize={}, rtt≈{:.1}ms)",
        r.filename,
        r.host,
        r.port,
        r.bytes.len(),
        r.blksize,
        r.connect_ms
    );
    std::io::stdout().write_all(&r.bytes)?;
    Ok(())
}

fn parse_url(url: &str) -> Result<(String, u16, String)> {
    let rest = url
        .strip_prefix("tftp://")
        .ok_or_else(|| anyhow!("tftp: URL must start with tftp://"))?;
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, p),
        None => (rest, ""),
    };
    let (host, port) = if let Some((h, p)) = authority.rsplit_once(':') {
        let h = h.trim_start_matches('[').trim_end_matches(']');
        (h.to_string(), p.parse::<u16>().map_err(|e| anyhow!("tftp: bad port '{p}': {e}"))?)
    } else {
        (authority.to_string(), DEFAULT_PORT)
    };
    if host.is_empty() {
        bail!("tftp: host missing in URL");
    }
    Ok((host, port, path.to_string()))
}

fn parse_oack_options(bytes: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut parts = bytes.split(|&b| b == 0).filter(|s| !s.is_empty());
    while let (Some(k), Some(v)) = (parts.next(), parts.next()) {
        let k = String::from_utf8_lossy(k).into_owned();
        let v = String::from_utf8_lossy(v).into_owned();
        out.push((k, v));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tftp_url() {
        let (h, p, f) = parse_url("tftp://host/file.bin").unwrap();
        assert_eq!(h, "host");
        assert_eq!(p, 69);
        assert_eq!(f, "file.bin");
    }

    #[test]
    fn parse_tftp_custom_port() {
        let (h, p, f) = parse_url("tftp://host:6969/path/to/f").unwrap();
        assert_eq!(h, "host");
        assert_eq!(p, 6969);
        assert_eq!(f, "path/to/f");
    }

    #[test]
    fn parse_rejects_non_tftp() {
        assert!(parse_url("ftp://host/f").is_err());
    }

    #[test]
    fn parse_oack_extracts_blksize() {
        let bytes = b"blksize\x001024\0tsize\x00100\0";
        let opts = parse_oack_options(bytes);
        assert_eq!(opts, vec![
            ("blksize".into(), "1024".into()),
            ("tsize".into(), "100".into()),
        ]);
    }
}
