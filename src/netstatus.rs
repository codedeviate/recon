use anyhow::{anyhow, Result};

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

// ── Placeholder run() — will be fleshed out in Task 10 ───────────────────────

pub fn run(_config: &crate::config::NetstatusConfig, _silent: bool) -> anyhow::Result<()> {
    todo!("implemented in Task 10")
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
