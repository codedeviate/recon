//! Memcached probe. Connects over TCP, issues `version\r\n`, reports
//! server version + roundtrip. Optionally issues `stats\r\n` when the
//! URL path is `/stats`.
//!
//! URL grammar: `memcached://host[:port][/stats]`. Default port 11211.
//! Exit 0 on VERSION reply, 7 refused, 28 timed out.

use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const DEFAULT_PORT: u16 = 11211;

pub(crate) struct MemcachedUrl {
    pub host: String,
    pub port: u16,
    pub want_stats: bool,
}

pub(crate) fn parse_url(raw: &str) -> Result<MemcachedUrl> {
    let rest = raw
        .strip_prefix("memcached://")
        .ok_or_else(|| anyhow!("memcached: URL must start with memcached://"))?;

    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(anyhow!("memcached: URL missing host"));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|_| anyhow!("memcached: invalid port '{p}'"))?,
        ),
        None => (authority.to_string(), DEFAULT_PORT),
    };

    let want_stats = matches!(path.trim_end_matches('/'), "stats");

    Ok(MemcachedUrl { host, port, want_stats })
}

pub struct MemcachedProbeOk {
    pub host: String,
    pub port: u16,
    pub connect_ms: f64,
    pub version_line: String,
    pub version_ms: f64,
    pub stats: std::collections::BTreeMap<String, String>,
}

pub fn probe(url: &str, timeout_secs: u64) -> Result<MemcachedProbeOk> {
    let parsed = parse_url(url)?;
    let addr = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()
        .with_context(|| format!("memcached: could not resolve {}:{}", parsed.host, parsed.port))?
        .next()
        .ok_or_else(|| anyhow!("memcached: no address for {}:{}", parsed.host, parsed.port))?;

    let timeout = Duration::from_secs(timeout_secs);
    let connect_start = Instant::now();
    let stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::TimedOut => {
            return Err(anyhow!("memcached: connection to {} timed out", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::OperationTimedOut);
        }
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
            return Err(anyhow!("memcached: connection refused by {}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
        Err(e) => {
            return Err(anyhow!("memcached: connect to {} failed: {e}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
    };
    let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let mut reader = BufReader::new(stream.try_clone().context("memcached: clone stream")?);
    let mut writer = stream;

    let cmd_start = Instant::now();
    writer
        .write_all(b"version\r\n")
        .context("memcached: write version")?;
    let mut version_line = String::new();
    reader
        .read_line(&mut version_line)
        .context("memcached: read version reply")?;
    let version_ms = cmd_start.elapsed().as_secs_f64() * 1000.0;

    let mut stats: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    if parsed.want_stats {
        writer
            .write_all(b"stats\r\n")
            .context("memcached: write stats")?;
        loop {
            let mut l = String::new();
            let n = reader
                .read_line(&mut l)
                .context("memcached: read stats line")?;
            if n == 0 {
                break;
            }
            if l.starts_with("END") || l.starts_with("ERROR") {
                break;
            }
            // "STAT key value\r\n"
            if let Some(rest) = l.trim_end_matches(['\r', '\n']).strip_prefix("STAT ") {
                if let Some((k, v)) = rest.split_once(' ') {
                    stats.insert(k.to_string(), v.to_string());
                }
            }
        }
    }

    let _ = writer.write_all(b"quit\r\n");

    Ok(MemcachedProbeOk {
        host: parsed.host,
        port: parsed.port,
        connect_ms,
        version_line: version_line.trim_end_matches(['\r', '\n']).to_string(),
        version_ms,
        stats,
    })
}

pub fn run(url: &str, timeout_secs: u64) -> Result<()> {
    let r = probe(url, timeout_secs)?;
    println!(
        "Connected to {}:{} in {:.1}ms",
        r.host, r.port, r.connect_ms
    );
    println!("{}", r.version_line);
    println!("  roundtrip: {:.1}ms", r.version_ms);
    if !r.stats.is_empty() {
        for (k, v) in &r.stats {
            println!("STAT {k} {v}");
        }
        println!("END");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_url() {
        let u = parse_url("memcached://localhost").unwrap();
        assert_eq!(u.host, "localhost");
        assert_eq!(u.port, 11211);
        assert!(!u.want_stats);
    }

    #[test]
    fn parses_host_port() {
        let u = parse_url("memcached://localhost:12345").unwrap();
        assert_eq!(u.port, 12345);
    }

    #[test]
    fn parses_stats_path() {
        let u = parse_url("memcached://localhost/stats").unwrap();
        assert!(u.want_stats);
    }

    #[test]
    fn rejects_missing_host() {
        assert!(parse_url("memcached:///").is_err());
    }

    #[test]
    fn rejects_bad_port() {
        assert!(parse_url("memcached://host:abc").is_err());
    }
}
