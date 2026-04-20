//! TCP connect probe: `tcp://host:port/`
//!
//! Opens a TCP connection, reports success + elapsed time + resolved IP,
//! closes cleanly. Exit 0 on connect, 7 on refuse/unreachable, 28 on
//! timeout. Reuses recon's `ProtocolExitCode` tag for error classification.

use anyhow::{anyhow, Context, Result};
use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub fn run(url: &str, timeout_secs: u64) -> Result<()> {
    let (host, port) = parse_url(url)?;

    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .with_context(|| format!("tcp: could not resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("tcp: no addresses resolved for {host}:{port}"))?;

    let start = Instant::now();
    let result = TcpStream::connect_timeout(&addr, Duration::from_secs(timeout_secs));
    let elapsed = start.elapsed();

    match result {
        Ok(stream) => {
            let local = stream
                .local_addr()
                .map(|a| a.to_string())
                .unwrap_or_else(|_| "?".to_string());
            println!("* Connected to {host}:{port} ({})", fmt_elapsed(elapsed));
            println!("* Resolved address: {}", addr.ip());
            println!("* Local address: {local}");
            // Stream drops here → TCP FIN sent cleanly.
            Ok(())
        }
        Err(e) if e.kind() == ErrorKind::TimedOut => Err(anyhow!(
            "tcp: connection to {host}:{port} timed out after {}s",
            timeout_secs
        )
        .context(crate::mqtt::ProtocolExitCode::OperationTimedOut)),
        Err(e)
            if matches!(
                e.kind(),
                ErrorKind::ConnectionRefused
                    | ErrorKind::ConnectionReset
                    | ErrorKind::HostUnreachable
                    | ErrorKind::NetworkUnreachable
                    | ErrorKind::AddrNotAvailable
                    | ErrorKind::NotFound
            ) =>
        {
            Err(anyhow!("tcp: could not connect to {host}:{port}: {e}")
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect))
        }
        Err(e) => Err(anyhow!("tcp: {e}")),
    }
}

fn parse_url(url: &str) -> Result<(String, u16)> {
    let parsed = url::Url::parse(url)
        .with_context(|| format!("malformed tcp URL: {url}"))?;
    if parsed.scheme() != "tcp" {
        anyhow::bail!("tcp_probe::run called with non-tcp scheme");
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("tcp URL missing host: {url}"))?
        .to_string();
    let port = parsed
        .port()
        .ok_or_else(|| anyhow!("tcp URL missing port (tcp://host:port/)"))?;
    Ok((host, port))
}

fn fmt_elapsed(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1.0 {
        format!("{:.3}ms", ms)
    } else if ms < 100.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{:.0}ms", ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tcp_url() {
        let (h, p) = parse_url("tcp://example.com:8080/").unwrap();
        assert_eq!(h, "example.com");
        assert_eq!(p, 8080);
    }

    #[test]
    fn parses_tcp_url_without_trailing_slash() {
        let (h, p) = parse_url("tcp://example.com:22").unwrap();
        assert_eq!(h, "example.com");
        assert_eq!(p, 22);
    }

    #[test]
    fn rejects_missing_port() {
        let err = parse_url("tcp://example.com/").unwrap_err();
        assert!(err.to_string().contains("missing port"));
    }

    #[test]
    fn rejects_non_tcp_scheme() {
        assert!(parse_url("http://example.com:80/").is_err());
    }

    #[test]
    fn fmt_elapsed_ranges() {
        assert_eq!(fmt_elapsed(Duration::from_micros(500)), "0.500ms");
        assert_eq!(fmt_elapsed(Duration::from_millis(45)), "45.0ms");
        assert_eq!(fmt_elapsed(Duration::from_millis(750)), "750ms");
    }
}
