use anyhow::{anyhow, Result};
use colored::Colorize;
use socket2::{Domain, Protocol, Socket, Type};
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum Probe {
    Http(String),
    Ping { host: String, port: Option<u16> },
    Dns { server: String, domain: String },
    Tcp { host: String, port: u16 },
    Tls { host: String, port: u16 },
    Ntp(String),
}

pub struct ProbeResult {
    pub label: String,
    pub passed: bool,
    pub detail: String,
}

// ── Probe parsing ─────────────────────────────────────────────────────────────

/// Parses a probe URL string into a typed `Probe`.
/// `dns_lookup_domains` must be non-empty if the scheme is `dns://`.
pub fn parse_probe(s: &str, dns_lookup_domains: &[String]) -> Result<Probe> {
    if s.starts_with("http://") || s.starts_with("https://") {
        return Ok(Probe::Http(s.to_string()));
    }

    let (scheme, rest) = s
        .split_once("://")
        .ok_or_else(|| anyhow!("Invalid probe URL (missing scheme): {s}"))?;

    match scheme {
        "ping" => {
            let (host, port) = split_host_port(rest);
            Ok(Probe::Ping { host, port })
        }
        "dns" => {
            let domain = dns_lookup_domains
                .first()
                .ok_or_else(|| anyhow!("dns:// probe requires dns_lookup_domains in config"))?
                .clone();
            Ok(Probe::Dns {
                server: rest.to_string(),
                domain,
            })
        }
        "tcp" => {
            let (host, port) = split_host_port(rest);
            let port = port.ok_or_else(|| anyhow!("tcp:// probe requires a port: {s}"))?;
            Ok(Probe::Tcp { host, port })
        }
        "tls" => {
            let (host, port) = split_host_port(rest);
            let port = port.ok_or_else(|| anyhow!("tls:// probe requires a port: {s}"))?;
            Ok(Probe::Tls { host, port })
        }
        "ntp" => Ok(Probe::Ntp(rest.to_string())),
        other => Err(anyhow!("Unknown probe scheme: {other}://")),
    }
}

fn split_host_port(s: &str) -> (String, Option<u16>) {
    // IPv6 bracket notation: [::1]:53
    if s.starts_with('[') {
        if let Some(end) = s.find(']') {
            let host = s[1..end].to_string();
            let port = s[end + 1..]
                .strip_prefix(':')
                .and_then(|p| p.parse().ok());
            return (host, port);
        }
    }
    // Plain host or host:port
    if let Some(pos) = s.rfind(':') {
        if let Ok(port) = s[pos + 1..].parse::<u16>() {
            return (s[..pos].to_string(), Some(port));
        }
    }
    (s.to_string(), None)
}

// ── Status aggregation ────────────────────────────────────────────────────────

pub fn overall_status(results: &[ProbeResult]) -> &'static str {
    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    if passed == total { "ONLINE" }
    else if passed == 0 { "OFFLINE" }
    else { "DEGRADED" }
}

// ── Probe runners ─────────────────────────────────────────────────────────────

fn probe_http(url: &str) -> ProbeResult {
    let label = url.to_string();
    let result = (|| -> anyhow::Result<String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .danger_accept_invalid_certs(true)
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()?;
        let start = Instant::now();
        let resp = client.head(url).send()?;
        let elapsed = start.elapsed().as_millis();
        Ok(format!("{} ({}ms)", resp.status(), elapsed))
    })();
    match result {
        Ok(detail) => ProbeResult { label, passed: true, detail },
        Err(e) => ProbeResult { label, passed: false, detail: e.to_string() },
    }
}

fn probe_tcp(host: &str, port: u16) -> ProbeResult {
    let label = format!("tcp://{}:{}", host, port);
    let result = (|| -> anyhow::Result<String> {
        let start = Instant::now();
        let addr = format!("{}:{}", host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("Could not resolve {}", host))?;
        std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        Ok(format!("connected ({}ms)", start.elapsed().as_millis()))
    })();
    match result {
        Ok(detail) => ProbeResult { label, passed: true, detail },
        Err(e) => ProbeResult { label, passed: false, detail: e.to_string() },
    }
}

fn probe_ping(host: &str, port: Option<u16>) -> ProbeResult {
    if let Some(port) = port {
        probe_ping_tcp(host, port)
    } else {
        probe_ping_icmp(host)
    }
}

fn probe_ping_tcp(host: &str, port: u16) -> ProbeResult {
    let label = format!("ping://{}:{}", host, port);
    let result = (|| -> anyhow::Result<String> {
        let start = Instant::now();
        let addr = format!("{}:{}", host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("Could not resolve {}", host))?;
        std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        Ok(format!("rtt={}ms", start.elapsed().as_millis()))
    })();
    match result {
        Ok(detail) => ProbeResult { label, passed: true, detail },
        Err(e) => ProbeResult { label, passed: false, detail: e.to_string() },
    }
}

fn probe_ping_icmp(host: &str) -> ProbeResult {
    let label = format!("ping://{}", host);
    let result = (|| -> anyhow::Result<String> {
        let addr: SocketAddr = format!("{}:0", host)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("Could not resolve {}", host))?;
        let ip = addr.ip();
        if ip.is_ipv6() {
            anyhow::bail!("ICMP ping does not support IPv6; use ping://{}:<port> for TCP ping", host);
        }
        let target = socket2::SockAddr::from(SocketAddr::new(ip, 0));

        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    anyhow!("ICMP requires elevated privileges; use ping://{}:<port> instead", host)
                } else {
                    anyhow!("Failed to create ICMP socket: {e}")
                }
            })?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;

        let pid = (std::process::id() & 0xffff) as u16;
        let packet = build_icmp_echo(pid, 0);
        let start = Instant::now();
        socket.send_to(&packet, &target)?;

        let mut buf = [MaybeUninit::uninit(); 512];
        let (n, _) = socket.recv_from(&mut buf)
            .map_err(|e| anyhow!("No reply: {e}"))?;
        // SAFETY: MaybeUninit<u8> has the same layout as u8, and recv_from
        // confirmed that exactly `n` bytes were written into `buf`.
        let data: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };
        let offset = if !data.is_empty() && (data[0] >> 4) == 4 { (data[0] & 0x0f) as usize * 4 } else { 0 };
        let icmp = &data[offset..];
        if icmp.len() < 8 || icmp[0] != 0 {
            anyhow::bail!("Unexpected ICMP response");
        }
        Ok(format!("rtt={}ms", start.elapsed().as_millis()))
    })();
    match result {
        Ok(detail) => ProbeResult { label, passed: true, detail },
        Err(e) => ProbeResult { label, passed: false, detail: e.to_string() },
    }
}

fn build_icmp_echo(id: u16, seq: u16) -> Vec<u8> {
    let mut pkt = vec![
        8u8, 0, 0, 0,
        (id >> 8) as u8, id as u8,
        (seq >> 8) as u8, seq as u8,
        b'r', b'e', b'c', b'o', b'n', b'_', b'n', b's',
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
    while sum >> 16 != 0 { sum = (sum & 0xffff) + (sum >> 16); }
    !(sum as u16)
}

fn probe_dns(server: &str, domain: &str) -> ProbeResult {
    let label = format!("dns://{}", server);
    let server = server.to_string();
    let domain = domain.to_string();
    let result = (|| -> anyhow::Result<String> {
        let server_ip: std::net::IpAddr = server
            .parse()
            .map_err(|_| anyhow!("Invalid DNS server IP: {}", server))?;
        // A new current-thread runtime is safe here because probe_dns* is always
        // called from spawn_blocking, which runs on a dedicated blocking-thread
        // pool — not on a tokio executor thread. There is no outer async context
        // on this thread, so block_on cannot deadlock.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
            use hickory_resolver::TokioAsyncResolver;
            let group = NameServerConfigGroup::from_ips_clear(&[server_ip], 53, true);
            let config = ResolverConfig::from_parts(None, vec![], group);
            let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());
            resolver
                .lookup_ip(domain.as_str())
                .await
                .map_err(|e| anyhow!("DNS lookup failed: {e}"))?;
            Ok::<String, anyhow::Error>(format!("resolved {}", domain))
        })
    })();
    match result {
        Ok(detail) => ProbeResult { label, passed: true, detail },
        Err(e) => ProbeResult { label, passed: false, detail: e.to_string() },
    }
}

fn probe_dns_hijack(check: &crate::config::DnsHijackCheck) -> ProbeResult {
    let label = format!("{} → {}", check.server, check.domain);
    let server = check.server.clone();
    let domain = check.domain.clone();
    let expected = check.expected.clone();
    let result = (|| -> anyhow::Result<String> {
        let server_ip: std::net::IpAddr = server
            .parse()
            .map_err(|_| anyhow!("Invalid DNS server IP: {}", server))?;
        // A new current-thread runtime is safe here because probe_dns* is always
        // called from spawn_blocking, which runs on a dedicated blocking-thread
        // pool — not on a tokio executor thread. There is no outer async context
        // on this thread, so block_on cannot deadlock.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
            use hickory_resolver::TokioAsyncResolver;
            let group = NameServerConfigGroup::from_ips_clear(&[server_ip], 53, true);
            let config = ResolverConfig::from_parts(None, vec![], group);
            let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());
            let lookup = resolver
                .lookup_ip(domain.as_str())
                .await
                .map_err(|e| anyhow!("DNS lookup failed: {e}"))?;
            let ips: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
            if ips.iter().any(|ip| ip == &expected) {
                Ok::<String, anyhow::Error>(format!("{} (match)", expected))
            } else {
                Err(anyhow!("got {}, expected {}", ips.join(", "), expected))
            }
        })
    })();
    match result {
        Ok(detail) => ProbeResult { label, passed: true, detail },
        Err(e) => ProbeResult { label, passed: false, detail: e.to_string() },
    }
}

fn probe_tls(host: &str, port: u16) -> ProbeResult {
    let label = format!("tls://{}:{}", host, port);
    match crate::tls_probe::probe(host, port) {
        Ok(r) if r.is_expired => ProbeResult {
            label,
            passed: false,
            detail: format!("certificate expired ({})", r.not_after),
        },
        Ok(r) => ProbeResult {
            label,
            passed: true,
            detail: format!("{}, cert valid ({} days)", r.version, r.days_remaining),
        },
        Err(e) => ProbeResult {
            label,
            passed: false,
            detail: e.to_string(),
        },
    }
}

fn probe_ntp(host: &str) -> ProbeResult {
    let label = format!("ntp://{}", host);
    let result = (|| -> anyhow::Result<String> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        socket.connect(format!("{}:123", host))?;

        // NTP v3 client request: LI=0, VN=3, Mode=3
        let mut packet = [0u8; 48];
        packet[0] = 0x1B;
        socket.send(&packet)?;

        let mut buf = [0u8; 48];
        let n = socket.recv(&mut buf)?;
        if n < 48 || (buf[0] & 0x07) != 4 {
            anyhow::bail!("invalid NTP response (mode={})", buf[0] & 0x07);
        }

        // Transmit timestamp is at bytes 40–43 (seconds since 1900-01-01)
        let ntp_secs = u32::from_be_bytes([buf[40], buf[41], buf[42], buf[43]]) as i64;
        let unix_secs = ntp_secs - 2_208_988_800; // 70 years offset
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let offset = unix_secs - now;
        Ok(format!("offset={:+}s", offset))
    })();
    match result {
        Ok(detail) => ProbeResult { label, passed: true, detail },
        Err(e) => ProbeResult { label, passed: false, detail: e.to_string() },
    }
}

// ── Public IP check ───────────────────────────────────────────────────────────

struct IpCheckResult {
    ips: Vec<(String, String)>, // (source_url, returned_ip)
    agreed: bool,
    agreed_ip: Option<String>,
}

async fn fetch_ip_from(source: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let text = client.get(source).send().await?.text().await?;
    Ok(text.trim().to_string())
}

async fn check_public_ip(sources: &[String]) -> IpCheckResult {
    let handles: Vec<_> = sources
        .iter()
        .map(|src| {
            let src = src.clone();
            tokio::spawn(async move {
                let ip = fetch_ip_from(&src).await.unwrap_or_else(|e| format!("error: {e}"));
                (src, ip)
            })
        })
        .collect();

    let mut ips = Vec::new();
    for h in handles {
        if let Ok(pair) = h.await {
            ips.push(pair);
        }
    }

    if ips.is_empty() {
        return IpCheckResult { ips, agreed: false, agreed_ip: None };
    }

    let first_ip = &ips[0].1;
    let agreed = ips.iter().all(|(_, ip)| ip == first_ip)
        && !first_ip.starts_with("error:");
    let agreed_ip = if agreed { Some(first_ip.clone()) } else { None };

    IpCheckResult { ips, agreed, agreed_ip }
}

// ── Parallel execution and output ────────────────────────────────────────────

pub fn run(config: &crate::config::NetstatusConfig, silent: bool) -> anyhow::Result<()> {
    // Parse all probe strings into typed Probes
    let probes: Vec<Probe> = config
        .probes
        .iter()
        .map(|s| parse_probe(s, &config.dns_lookup_domains))
        .collect::<Result<Vec<_>>>()?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow!("Failed to build async runtime: {e}"))?;

    let (ip_result, probe_results, hijack_results) = rt.block_on(async {
        // ── Public IP check (async) ───────────────────────────────────────
        let ip_fut = check_public_ip(&config.ip_sources);

        // ── Probe tasks (blocking, in thread pool) ────────────────────────
        let probe_handles: Vec<_> = probes
            .into_iter()
            .map(|probe| {
                tokio::task::spawn_blocking(move || match &probe {
                    Probe::Http(url) => probe_http(url),
                    Probe::Ping { host, port } => probe_ping(host, *port),
                    Probe::Dns { server, domain } => probe_dns(server, domain),
                    Probe::Tcp { host, port } => probe_tcp(host, *port),
                    Probe::Tls { host, port } => probe_tls(host, *port),
                    Probe::Ntp(host) => probe_ntp(host),
                })
            })
            .collect();

        // ── DNS hijack tasks (blocking, in thread pool) ───────────────────
        let hijack_handles: Vec<_> = config
            .dns_hijack_checks
            .iter()
            .map(|check| {
                let check = check.clone();
                tokio::task::spawn_blocking(move || probe_dns_hijack(&check))
            })
            .collect();

        // ── Collect results (maintain order) ─────────────────────────────
        let ip_result = ip_fut.await;

        let mut probe_results = Vec::new();
        for h in probe_handles {
            probe_results.push(h.await.unwrap_or_else(|_| ProbeResult {
                label: "(probe panicked)".into(),
                passed: false,
                detail: "internal error".into(),
            }));
        }

        let mut hijack_results = Vec::new();
        for h in hijack_handles {
            hijack_results.push(h.await.unwrap_or_else(|_| ProbeResult {
                label: "(hijack check panicked)".into(),
                passed: false,
                detail: "internal error".into(),
            }));
        }

        (ip_result, probe_results, hijack_results)
    });

    // ── Convert IP result to ProbeResult so overall_status() covers it ──
    let any_ip_check = !config.ip_sources.is_empty();
    let ip_probe = if any_ip_check {
        let (passed, detail) = if ip_result.ips.is_empty() {
            (false, "all sources failed".to_string())
        } else if ip_result.agreed {
            (true, format!(
                "{} ({}/{} sources agree)",
                ip_result.agreed_ip.as_deref().unwrap_or("?"),
                ip_result.ips.len(),
                config.ip_sources.len()
            ))
        } else {
            (false, "IP mismatch across sources:".to_string())
        };
        Some(ProbeResult { label: "Public IP".to_string(), passed, detail })
    } else {
        None
    };

    // Combine all results and compute status
    let all_owned: Vec<ProbeResult> = ip_probe.iter()
        .map(|r| ProbeResult { label: r.label.clone(), passed: r.passed, detail: r.detail.clone() })
        .chain(probe_results.iter().map(|r| ProbeResult { label: r.label.clone(), passed: r.passed, detail: r.detail.clone() }))
        .chain(hijack_results.iter().map(|r| ProbeResult { label: r.label.clone(), passed: r.passed, detail: r.detail.clone() }))
        .collect();
    let status = overall_status(&all_owned);

    if silent {
        if status != "ONLINE" {
            return Err(anyhow!("network check failed"));
        }
        return Ok(());
    }

    // ── Print output ──────────────────────────────────────────────────────
    println!("Network Status");
    println!("{}", "═".repeat(50));

    // Public IP section
    if let Some(ref r) = ip_probe {
        println!();
        println!("Public IP");
        let mark = if r.passed { "✓".green() } else { "✗".red() };
        println!("  {} {}", mark, r.detail);
        // On mismatch, also print per-source breakdown
        if !r.passed && !ip_result.ips.is_empty() {
            for (src, ip) in &ip_result.ips {
                println!("    {}: {}", src, ip);
            }
        }
    }

    // Probes section
    if !probe_results.is_empty() {
        println!();
        println!("Probes");
        for r in &probe_results {
            let mark = if r.passed { "✓".green() } else { "✗".red() };
            println!("  {} {:<40} {}", mark, r.label, r.detail);
        }
    }

    // DNS Hijack Checks section
    if !hijack_results.is_empty() {
        println!();
        println!("DNS Hijack Checks");
        for r in &hijack_results {
            let mark = if r.passed { "✓".green() } else { "✗".red() };
            println!("  {} {:<35} {}", mark, r.label, r.detail);
        }
    }

    // Overall
    println!();
    let status_colored = match status {
        "ONLINE" => "ONLINE".green().bold(),
        "OFFLINE" => "OFFLINE".red().bold(),
        _ => "DEGRADED".yellow().bold(),
    };
    println!("Overall: {}", status_colored);

    if status != "ONLINE" {
        return Err(anyhow!("network check failed"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_probe() {
        let p = parse_probe("https://www.google.com", &[]).unwrap();
        assert!(matches!(p, Probe::Http(_)));
        if let Probe::Http(url) = p { assert_eq!(url, "https://www.google.com"); }
    }

    #[test]
    fn test_parse_http_probe_plain() {
        let p = parse_probe("http://example.com", &[]).unwrap();
        assert!(matches!(p, Probe::Http(_)));
    }

    #[test]
    fn test_parse_ping_no_port() {
        let p = parse_probe("ping://8.8.8.8", &[]).unwrap();
        assert!(matches!(p, Probe::Ping { port: None, .. }));
        if let Probe::Ping { host, port } = p { assert_eq!(host, "8.8.8.8"); assert!(port.is_none()); }
    }

    #[test]
    fn test_parse_ping_with_port() {
        let p = parse_probe("ping://example.com:443", &[]).unwrap();
        if let Probe::Ping { host, port } = p {
            assert_eq!(host, "example.com");
            assert_eq!(port, Some(443));
        } else { panic!("wrong variant"); }
    }

    #[test]
    fn test_parse_dns_probe() {
        let domains = vec!["example.com".to_string()];
        let p = parse_probe("dns://8.8.8.8", &domains).unwrap();
        if let Probe::Dns { server, domain } = p {
            assert_eq!(server, "8.8.8.8");
            assert_eq!(domain, "example.com");
        } else { panic!("wrong variant"); }
    }

    #[test]
    fn test_parse_tcp_probe() {
        let p = parse_probe("tcp://8.8.8.8:53", &[]).unwrap();
        if let Probe::Tcp { host, port } = p {
            assert_eq!(host, "8.8.8.8");
            assert_eq!(port, 53);
        } else { panic!("wrong variant"); }
    }

    #[test]
    fn test_parse_tls_probe() {
        let p = parse_probe("tls://www.google.com:443", &[]).unwrap();
        if let Probe::Tls { host, port } = p {
            assert_eq!(host, "www.google.com");
            assert_eq!(port, 443);
        } else { panic!("wrong variant"); }
    }

    #[test]
    fn test_parse_ntp_probe() {
        let p = parse_probe("ntp://pool.ntp.org", &[]).unwrap();
        if let Probe::Ntp(host) = p { assert_eq!(host, "pool.ntp.org"); }
        else { panic!("wrong variant"); }
    }

    #[test]
    fn test_parse_unknown_scheme_errors() {
        assert!(parse_probe("ftp://example.com", &[]).is_err());
    }

    #[test]
    fn test_parse_tcp_missing_port_errors() {
        assert!(parse_probe("tcp://8.8.8.8", &[]).is_err());
    }

    #[test]
    fn test_parse_tls_missing_port_errors() {
        assert!(parse_probe("tls://example.com", &[]).is_err());
    }

    #[test]
    fn test_overall_status_all_pass() {
        let results = vec![
            ProbeResult { label: "a".into(), passed: true, detail: "ok".into() },
            ProbeResult { label: "b".into(), passed: true, detail: "ok".into() },
        ];
        assert_eq!(overall_status(&results), "ONLINE");
    }

    #[test]
    fn test_overall_status_some_fail() {
        let results = vec![
            ProbeResult { label: "a".into(), passed: true, detail: "ok".into() },
            ProbeResult { label: "b".into(), passed: false, detail: "fail".into() },
        ];
        assert_eq!(overall_status(&results), "DEGRADED");
    }

    #[test]
    fn test_overall_status_all_fail() {
        let results = vec![
            ProbeResult { label: "a".into(), passed: false, detail: "fail".into() },
            ProbeResult { label: "b".into(), passed: false, detail: "fail".into() },
        ];
        assert_eq!(overall_status(&results), "OFFLINE");
    }

    #[test]
    fn test_overall_status_empty_is_online() {
        assert_eq!(overall_status(&[]), "ONLINE");
    }
}
