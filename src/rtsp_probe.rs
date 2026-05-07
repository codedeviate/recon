//! RTSP probe. Opens a TCP (or TLS for rtsps://) connection and sends
//! `OPTIONS *` (RFC 2326). Reports server banner, status line, and
//! supported methods.
//!
//! URL grammar: `rtsp://host[:port][/path]` (default 554) or
//! `rtsps://host[:port][/path]` (default 322, RTSP over TLS per IANA).
//! Exit 0 on any RTSP response; 7 refused; 28 timed out.

use anyhow::{anyhow, Context, Result};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_PORT_PLAIN: u16 = 554;
const DEFAULT_PORT_TLS: u16 = 322;

pub(crate) struct RtspUrl {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub tls: bool,
    /// Raw `user[:pass]` captured from the URL, before percent-decoding.
    /// RTSP auth (Basic/Digest) is not yet implemented; this is recorded
    /// so we can wire it up later, and so it stops leaking into the host
    /// component (which previously broke DNS resolution for any URL
    /// shaped like `rtsp://demo:demo@host:port/...`).
    #[allow(dead_code)]
    pub userinfo: Option<String>,
}

pub(crate) fn parse_url(raw: &str) -> Result<RtspUrl> {
    let (tls, rest) = if let Some(r) = raw.strip_prefix("rtsps://") {
        (true, r)
    } else if let Some(r) = raw.strip_prefix("rtsp://") {
        (false, r)
    } else {
        return Err(anyhow!("rtsp: URL must start with rtsp:// or rtsps://"));
    };

    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(anyhow!("rtsp: URL missing host"));
    }

    // Strip optional `user[:pass]@` prefix. Use rsplit_once so a `@`
    // inside the password (rare but legal pre-encoding) doesn't confuse
    // the split — only the LAST `@` separates userinfo from host.
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((u, hp)) => (Some(u.to_string()), hp),
        None => (None, authority),
    };
    if hostport.is_empty() {
        return Err(anyhow!("rtsp: URL missing host"));
    }

    let default_port = if tls { DEFAULT_PORT_TLS } else { DEFAULT_PORT_PLAIN };

    // IPv6 literal: `[::1]` or `[::1]:554`. Brackets fence off the
    // colons in the address from the host:port separator. Only the
    // bytes after `]` may carry an explicit port.
    let (host, port) = if let Some(after_open) = hostport.strip_prefix('[') {
        let close = after_open
            .find(']')
            .ok_or_else(|| anyhow!("rtsp: unterminated IPv6 literal in '{hostport}'"))?;
        let host = &after_open[..close];
        let tail = &after_open[close + 1..];
        let port = if tail.is_empty() {
            default_port
        } else if let Some(p) = tail.strip_prefix(':') {
            p.parse::<u16>()
                .map_err(|_| anyhow!("rtsp: invalid port '{p}'"))?
        } else {
            return Err(anyhow!("rtsp: unexpected text after IPv6 host: '{tail}'"));
        };
        (host.to_string(), port)
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) => (
                h.to_string(),
                p.parse::<u16>()
                    .map_err(|_| anyhow!("rtsp: invalid port '{p}'"))?,
            ),
            None => (hostport.to_string(), default_port),
        }
    };

    Ok(RtspUrl {
        host,
        port,
        path: path.to_string(),
        tls,
        userinfo,
    })
}

pub struct RtspProbeOk {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub connect_ms: f64,
    pub status_line: String,
    pub headers: Vec<(String, String)>,
}

pub fn probe(url: &str, insecure: bool, timeout_secs: u64) -> Result<RtspProbeOk> {
    let parsed = parse_url(url)?;
    let addr = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()
        .with_context(|| format!("rtsp: could not resolve {}:{}", parsed.host, parsed.port))?
        .next()
        .ok_or_else(|| anyhow!("rtsp: no address for {}:{}", parsed.host, parsed.port))?;

    let timeout = Duration::from_secs(timeout_secs);
    let connect_start = Instant::now();
    let tcp = match TcpStream::connect_timeout(&addr, timeout) {
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

    tcp.set_read_timeout(Some(timeout)).ok();
    tcp.set_write_timeout(Some(timeout)).ok();

    let scheme = if parsed.tls { "rtsps" } else { "rtsp" };
    let target = format!("{scheme}://{}:{}{}", parsed.host, parsed.port, parsed.path);
    let req = format!(
        "OPTIONS {target} RTSP/1.0\r\n\
         CSeq: 1\r\n\
         User-Agent: recon/{}\r\n\
         \r\n",
        env!("CARGO_PKG_VERSION")
    );

    let (status_line, headers) = if parsed.tls {
        let config = build_rustls_config(insecure)?;
        let server_name = ServerName::try_from(parsed.host.clone())
            .map_err(|_| anyhow!("rtsp: invalid TLS server name '{}'", parsed.host))?;
        let mut conn = ClientConnection::new(Arc::new(config), server_name)
            .context("rtsp: create TLS client connection")?;
        let mut tcp_for_handshake = tcp;
        conn.complete_io(&mut tcp_for_handshake)
            .with_context(|| format!("rtsp: TLS handshake with {} failed", parsed.host))?;
        let mut stream = StreamOwned::new(conn, tcp_for_handshake);
        stream
            .write_all(req.as_bytes())
            .context("rtsp: write OPTIONS over TLS")?;
        read_response(&mut stream)?
    } else {
        let mut reader = BufReader::new(tcp.try_clone().context("rtsp: clone stream")?);
        let mut writer = tcp;
        writer
            .write_all(req.as_bytes())
            .context("rtsp: write OPTIONS")?;
        read_response(&mut reader)?
    };

    Ok(RtspProbeOk {
        host: parsed.host,
        port: parsed.port,
        tls: parsed.tls,
        connect_ms,
        status_line,
        headers,
    })
}

pub fn run(url: &str, insecure: bool, timeout_secs: u64) -> Result<()> {
    let r = probe(url, insecure, timeout_secs)?;
    println!("Connected to {}:{} in {:.1}ms", r.host, r.port, r.connect_ms);
    print!("{}", r.status_line);
    for (k, v) in &r.headers {
        print!("{k}: {v}\r\n");
    }
    print!("\r\n");
    Ok(())
}

fn read_response<R: Read>(r: &mut R) -> Result<(String, Vec<(String, String)>)> {
    let mut reader = std::io::BufReader::new(r);
    let mut status = String::new();
    let n = reader.read_line(&mut status).context("rtsp: read status")?;
    if n == 0 {
        return Err(anyhow!("rtsp: server closed without replying"));
    }

    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).context("rtsp: read header")?;
        if n == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((k, v)) = line.trim_end_matches(['\r', '\n']).split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    Ok((status, headers))
}

fn build_rustls_config(insecure: bool) -> Result<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());

    if insecure {
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("rtsp TLS: failed to configure protocol versions")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
            .with_no_client_auth();
        Ok(config)
    } else {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("rtsp TLS: failed to configure protocol versions")?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(config)
    }
}

/// Accepts every server certificate — used only under -k / --insecure.
#[derive(Debug)]
struct NoCertificateVerification;

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _: &[u8],
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
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
        assert!(!u.tls);
    }

    #[test]
    fn parses_rtsps() {
        let u = parse_url("rtsps://example.com").unwrap();
        assert_eq!(u.port, 322);
        assert!(u.tls);
    }

    #[test]
    fn parses_rtsps_custom_port() {
        let u = parse_url("rtsps://example.com:443/").unwrap();
        assert_eq!(u.port, 443);
        assert!(u.tls);
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

    #[test]
    fn strips_userinfo_user_pass() {
        let u = parse_url("rtsp://demo:demo@host:5541/onvif-media/media.amp").unwrap();
        assert_eq!(u.host, "host");
        assert_eq!(u.port, 5541);
        assert_eq!(u.userinfo.as_deref(), Some("demo:demo"));
        assert_eq!(u.path, "/onvif-media/media.amp");
    }

    #[test]
    fn strips_userinfo_user_only() {
        let u = parse_url("rtsp://alice@example.com/stream").unwrap();
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 554);
        assert_eq!(u.userinfo.as_deref(), Some("alice"));
    }

    #[test]
    fn ipv6_default_port() {
        let u = parse_url("rtsp://[::1]/stream").unwrap();
        assert_eq!(u.host, "::1");
        assert_eq!(u.port, 554);
        assert_eq!(u.path, "/stream");
    }

    #[test]
    fn ipv6_explicit_port() {
        let u = parse_url("rtsp://[fe80::1]:8554/cam").unwrap();
        assert_eq!(u.host, "fe80::1");
        assert_eq!(u.port, 8554);
    }

    #[test]
    fn ipv6_with_userinfo() {
        let u = parse_url("rtsps://demo:pw@[2001:db8::1]:443/").unwrap();
        assert_eq!(u.host, "2001:db8::1");
        assert_eq!(u.port, 443);
        assert!(u.tls);
        assert_eq!(u.userinfo.as_deref(), Some("demo:pw"));
    }
}
