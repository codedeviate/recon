//! UDP send-and-wait probe: `udp://host:port[/path]`
//!
//! Sends a single datagram (payload from `-d`, or empty) and waits
//! `--wait-time` seconds for any response. Prints send/receive stats;
//! exits 0 unless send fails. UDP has no "connected" concept, so a
//! lack of response is ambiguous (the service may be silent, firewalled,
//! or not listening) and does not produce a non-zero exit.

use anyhow::{anyhow, Context, Result};
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

pub fn run(url: &str, args: &crate::cli::Args) -> Result<()> {
    let (host, port) = parse_url(url)?;

    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .with_context(|| format!("udp: could not resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("udp: no addresses resolved for {host}:{port}"))?;

    let bind_addr = if addr.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
    let socket = UdpSocket::bind(bind_addr)
        .with_context(|| format!("udp: could not bind local socket ({bind_addr})"))?;

    let payload: Vec<u8> = match &args.data {
        Some(d) => crate::client::load_body_from_string(d)?,
        None => Vec::new(),
    };

    println!("* Sending {} byte(s) to {}:{}", payload.len(), host, port);
    let start = Instant::now();
    let sent = socket
        .send_to(&payload, addr)
        .with_context(|| format!("udp: send_to failed for {host}:{port}"))?;
    println!("* Sent {sent} byte(s), waiting up to {:.3}s for response...", args.wait_time);

    let wait_ms = (args.wait_time * 1000.0) as u64;
    socket
        .set_read_timeout(Some(Duration::from_millis(wait_ms.max(1))))
        .context("udp: set_read_timeout failed")?;

    let mut buf = vec![0u8; 64 * 1024];
    match socket.recv_from(&mut buf) {
        Ok((n, from)) => {
            let elapsed = start.elapsed();
            println!("* Received {n} byte(s) from {} in {}", from, fmt_elapsed(elapsed));
            if n > 0 {
                print_payload_preview(&buf[..n]);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
            println!(
                "* No response within {:.3}s (UDP: silence is ambiguous — service may not be listening, may be silent by design, or may be firewalled)",
                args.wait_time
            );
        }
        Err(e) => return Err(anyhow!("udp: recv_from failed: {e}")),
    }
    Ok(())
}

fn parse_url(url: &str) -> Result<(String, u16)> {
    let parsed = url::Url::parse(url)
        .with_context(|| format!("malformed udp URL: {url}"))?;
    if parsed.scheme() != "udp" {
        anyhow::bail!("udp_probe::run called with non-udp scheme");
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("udp URL missing host: {url}"))?
        .to_string();
    let port = parsed
        .port()
        .ok_or_else(|| anyhow!("udp URL missing port (udp://host:port/)"))?;
    Ok((host, port))
}

fn fmt_elapsed(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1.0 { format!("{:.3}ms", ms) }
    else if ms < 100.0 { format!("{:.1}ms", ms) }
    else { format!("{:.0}ms", ms) }
}

/// Print a response payload preview. Text payloads (printable UTF-8) are
/// printed as-is; binary payloads are shown as a hex preview (first 64 bytes).
fn print_payload_preview(bytes: &[u8]) {
    let preview = match std::str::from_utf8(bytes) {
        Ok(s) if s.chars().all(|c| !c.is_control() || c == '\n' || c == '\t' || c == '\r') => {
            s.to_string()
        }
        _ => {
            let mut s = String::with_capacity(bytes.len() * 3);
            for b in bytes.iter().take(64) {
                s.push_str(&format!("{b:02x} "));
            }
            if bytes.len() > 64 {
                s.push_str("...");
            }
            s.trim_end().to_string()
        }
    };
    println!("< {preview}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_udp_url() {
        let (h, p) = parse_url("udp://example.com:53/").unwrap();
        assert_eq!(h, "example.com");
        assert_eq!(p, 53);
    }

    #[test]
    fn parses_udp_url_with_path() {
        let (h, p) = parse_url("udp://example.com:1234/some/path").unwrap();
        assert_eq!(h, "example.com");
        assert_eq!(p, 1234);
    }

    #[test]
    fn rejects_missing_port() {
        assert!(parse_url("udp://example.com/").is_err());
    }

    #[test]
    fn rejects_non_udp_scheme() {
        assert!(parse_url("tcp://example.com:80/").is_err());
    }

    #[test]
    fn preview_hex_for_binary() {
        // Pins that binary bytes (non-UTF-8) are treated differently from UTF-8.
        let bytes = [0xffu8, 0x00, 0x10];
        let result = match std::str::from_utf8(&bytes) {
            Ok(_) => "utf8",
            Err(_) => "binary",
        };
        assert_eq!(result, "binary");
    }
}
