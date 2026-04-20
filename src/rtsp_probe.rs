//! RTSP probe. Opens a TCP connection and sends `OPTIONS *` (RFC 2326).
//! Reports server banner, status line, and supported methods.
//!
//! URL grammar: `rtsp://host[:port][/path]`. Default port 554.
//! Exit 0 on any RTSP response; 7 refused; 28 timed out.

use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const DEFAULT_PORT: u16 = 554;

pub(crate) struct RtspUrl {
    pub host: String,
    pub port: u16,
    pub path: String,
}

pub(crate) fn parse_url(raw: &str) -> Result<RtspUrl> {
    let rest = raw
        .strip_prefix("rtsp://")
        .ok_or_else(|| anyhow!("rtsp: URL must start with rtsp://"))?;

    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(anyhow!("rtsp: URL missing host"));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|_| anyhow!("rtsp: invalid port '{p}'"))?,
        ),
        None => (authority.to_string(), DEFAULT_PORT),
    };

    Ok(RtspUrl {
        host,
        port,
        path: path.to_string(),
    })
}

pub fn run(url: &str, timeout_secs: u64) -> Result<()> {
    let parsed = parse_url(url)?;
    let addr = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()
        .with_context(|| format!("rtsp: could not resolve {}:{}", parsed.host, parsed.port))?
        .next()
        .ok_or_else(|| anyhow!("rtsp: no address for {}:{}", parsed.host, parsed.port))?;

    let timeout = Duration::from_secs(timeout_secs);
    let connect_start = Instant::now();
    let stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::TimedOut => {
            return Err(anyhow!("rtsp: connection to {} timed out", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::OperationTimedOut);
        }
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
            return Err(anyhow!("rtsp: connection refused by {}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
        Err(e) => {
            return Err(anyhow!("rtsp: connect to {} failed: {e}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
    };
    let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let target = format!("rtsp://{}:{}{}", parsed.host, parsed.port, parsed.path);
    let req = format!(
        "OPTIONS {target} RTSP/1.0\r\n\
         CSeq: 1\r\n\
         User-Agent: recon/{}\r\n\
         \r\n",
        env!("CARGO_PKG_VERSION")
    );

    let mut reader = BufReader::new(stream.try_clone().context("rtsp: clone stream")?);
    let mut writer = stream;

    println!("Connected to {}:{} in {connect_ms:.1}ms", parsed.host, parsed.port);

    writer
        .write_all(req.as_bytes())
        .context("rtsp: write OPTIONS")?;

    // Read status line + headers until blank line. Don't read beyond —
    // OPTIONS has no body for any server we care about.
    let mut status = String::new();
    let n = reader.read_line(&mut status).context("rtsp: read status")?;
    if n == 0 {
        return Err(anyhow!("rtsp: server closed without replying"));
    }
    print!("{status}");

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).context("rtsp: read header")?;
        if n == 0 {
            break;
        }
        print!("{line}");
        if line == "\r\n" || line == "\n" {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare() {
        let u = parse_url("rtsp://example.com").unwrap();
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 554);
        assert_eq!(u.path, "/");
    }

    #[test]
    fn parses_port_and_path() {
        let u = parse_url("rtsp://example.com:8554/stream1").unwrap();
        assert_eq!(u.port, 8554);
        assert_eq!(u.path, "/stream1");
    }

    #[test]
    fn rejects_missing_host() {
        assert!(parse_url("rtsp:///foo").is_err());
    }

    #[test]
    fn rejects_bad_port() {
        assert!(parse_url("rtsp://example.com:bad/").is_err());
    }
}
