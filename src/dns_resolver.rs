//! Custom DNS resolver wiring for `--dns-servers`, `--dns-ipv4-addr`,
//! `--dns-ipv6-addr`. Builds a hickory `TokioAsyncResolver` from the CLI
//! flags and wraps it in a type implementing reqwest's `Resolve` trait.
//!
//! `--dns-interface` is accepted on the CLI for completeness but not yet
//! plumbed (hickory's public `NameServerConfig` doesn't expose a socket-
//! binding hook). Users see a clear error on attempt; documented in
//! OUT-OF-SCOPE.md.

use anyhow::{anyhow, Context, Result};
use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

/// Returns Some(Arc<impl Resolve>) when any of the DNS override flags
/// are set; returns Ok(None) otherwise (the caller falls back to
/// reqwest's default getaddrinfo resolver). Errors on malformed flag
/// values.
pub fn build_from_args(args: &crate::cli::Args) -> Result<Option<Arc<CustomResolver>>> {
    if args.dns_interface.is_some() {
        return Err(anyhow!(
            "--dns-interface: binding DNS queries to a named interface is \
             not yet supported. Use --dns-ipv4-addr / --dns-ipv6-addr with \
             the interface's literal address instead."
        ));
    }

    let have_override = args.dns_servers.is_some()
        || args.dns_ipv4_addr.is_some()
        || args.dns_ipv6_addr.is_some();
    if !have_override {
        return Ok(None);
    }

    let servers = match &args.dns_servers {
        Some(s) => parse_servers(s)?,
        None => {
            // If the user set --dns-*-addr without --dns-servers, fall back
            // to Cloudflare as a sensible default instead of inheriting
            // system resolvers (which would ignore the bind addrs).
            vec!["1.1.1.1:53".parse::<SocketAddr>().unwrap()]
        }
    };
    let v4_bind = args
        .dns_ipv4_addr
        .as_deref()
        .map(parse_ipv4)
        .transpose()?
        .map(|ip| SocketAddr::new(ip.into(), 0));
    let v6_bind = args
        .dns_ipv6_addr
        .as_deref()
        .map(parse_ipv6)
        .transpose()?
        .map(|ip| SocketAddr::new(ip.into(), 0));

    let mut config = ResolverConfig::new();
    for addr in servers {
        let bind = match addr.ip() {
            IpAddr::V4(_) => v4_bind,
            IpAddr::V6(_) => v6_bind,
        };
        config.add_name_server(NameServerConfig {
            socket_addr: addr,
            protocol: Protocol::Udp,
            tls_dns_name: None,
            trust_negative_responses: true,
            bind_addr: bind,
        });
        // Match default behaviour: UDP + TCP fallback for truncated responses.
        config.add_name_server(NameServerConfig {
            socket_addr: addr,
            protocol: Protocol::Tcp,
            tls_dns_name: None,
            trust_negative_responses: true,
            bind_addr: bind,
        });
    }

    let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());
    Ok(Some(Arc::new(CustomResolver {
        inner: Arc::new(resolver),
    })))
}

pub struct CustomResolver {
    inner: Arc<TokioAsyncResolver>,
}

impl Resolve for CustomResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let inner = self.inner.clone();
        Box::pin(async move {
            let lookup = inner.lookup_ip(host.as_str()).await?;
            let addrs: Vec<SocketAddr> = lookup
                .into_iter()
                .map(|ip| SocketAddr::new(ip, 0))
                .collect();
            let iter: Addrs = Box::new(addrs.into_iter());
            Ok(iter)
        })
    }
}

fn parse_servers(spec: &str) -> Result<Vec<SocketAddr>> {
    let mut out = Vec::new();
    for raw in spec.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        // Accept `IP` (port 53 assumed) or `IP:PORT`.
        let addr = if let Ok(sa) = raw.parse::<SocketAddr>() {
            sa
        } else {
            let ip: IpAddr = raw
                .parse()
                .with_context(|| format!("--dns-servers: not an IP or IP:port ({raw})"))?;
            SocketAddr::new(ip, 53)
        };
        out.push(addr);
    }
    if out.is_empty() {
        return Err(anyhow!("--dns-servers: no nameservers parsed from '{spec}'"));
    }
    Ok(out)
}

fn parse_ipv4(s: &str) -> Result<std::net::Ipv4Addr> {
    s.parse()
        .with_context(|| format!("--dns-ipv4-addr: not an IPv4 literal ({s})"))
}

fn parse_ipv6(s: &str) -> Result<std::net::Ipv6Addr> {
    s.parse()
        .with_context(|| format!("--dns-ipv6-addr: not an IPv6 literal ({s})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_servers_with_default_port() {
        let v = parse_servers("1.1.1.1,8.8.8.8").unwrap();
        assert_eq!(v[0], "1.1.1.1:53".parse::<SocketAddr>().unwrap());
        assert_eq!(v[1], "8.8.8.8:53".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn parse_servers_with_explicit_port() {
        let v = parse_servers("1.1.1.1:5353,9.9.9.9:5354").unwrap();
        assert_eq!(v[0].port(), 5353);
        assert_eq!(v[1].port(), 5354);
    }

    #[test]
    fn parse_servers_skips_empty() {
        let v = parse_servers("1.1.1.1, ,8.8.8.8").unwrap();
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn parse_servers_empty_errors() {
        assert!(parse_servers("").is_err());
        assert!(parse_servers(" , ").is_err());
    }

    #[test]
    fn parse_servers_bad_ip_errors() {
        assert!(parse_servers("not.an.ip").is_err());
    }

    #[test]
    fn parse_v4_and_v6() {
        assert_eq!(
            parse_ipv4("10.0.0.1").unwrap(),
            "10.0.0.1".parse::<std::net::Ipv4Addr>().unwrap()
        );
        assert_eq!(
            parse_ipv6("::1").unwrap(),
            "::1".parse::<std::net::Ipv6Addr>().unwrap()
        );
        assert!(parse_ipv4("::1").is_err());
        assert!(parse_ipv6("10.0.0.1").is_err());
    }
}
