//! `gopher://` and `gophers://` probes. RFC 1436 is tiny: open TCP,
//! send the selector followed by CRLF, read until close. Everything
//! the server returns is the response.
//!
//! Path grammar: `gopher://host[:port]/[TYPE]/selector`
//! - `TYPE` is a single character (RFC 1436 item type — 0 text, 1 dir,
//!   7 search, etc.). Informational only; the protocol sends the
//!   selector verbatim.
//! - Selector is the remainder after the type char (or the whole path
//!   when no type is present).

use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_PORT: u16 = 70;

pub struct GopherProbeOk {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub selector: String,
    pub item_type: Option<char>,
    pub connect_ms: f64,
    pub content: Vec<u8>,
}

pub fn probe(url: &str, timeout_secs: u64, insecure: bool) -> Result<GopherProbeOk> {
    let (host, port, tls, item_type, selector) = parse_url(url)?;
    let timeout = Duration::from_secs(timeout_secs.max(1));

    let t0 = Instant::now();
    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .with_context(|| format!("gopher: could not resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("gopher: no address for {host}:{port}"))?;
    let tcp = TcpStream::connect_timeout(&addr, timeout).map_err(|e| {
        anyhow!("gopher: connect to {host}:{port}: {e}").context(ProtocolExitCode::CouldntConnect)
    })?;
    tcp.set_read_timeout(Some(timeout))?;
    tcp.set_write_timeout(Some(timeout))?;
    let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let request = format!("{selector}\r\n");
    let content = if tls {
        read_over_tls(tcp, &host, insecure, request.as_bytes())?
    } else {
        read_plain(tcp, request.as_bytes())?
    };

    Ok(GopherProbeOk {
        host,
        port,
        tls,
        selector,
        item_type,
        connect_ms,
        content,
    })
}

pub fn run(url: &str, timeout_secs: u64, insecure: bool) -> Result<()> {
    let r = probe(url, timeout_secs, insecure)?;
    let label = if r.tls { " (TLS)" } else { "" };
    eprintln!(
        "Connected to {}:{}{} in {:.1}ms",
        r.host, r.port, label, r.connect_ms
    );
    if let Some(t) = r.item_type {
        eprintln!("Item type: {t}");
    }
    eprintln!("Selector: {:?}", r.selector);
    eprintln!("Content: {} bytes", r.content.len());
    std::io::stdout().write_all(&r.content)?;
    Ok(())
}

fn read_plain(mut tcp: TcpStream, request: &[u8]) -> Result<Vec<u8>> {
    tcp.write_all(request).context("gopher: write selector")?;
    let mut buf = Vec::new();
    tcp.read_to_end(&mut buf).context("gopher: read response")?;
    Ok(buf)
}

fn read_over_tls(
    tcp: TcpStream,
    host: &str,
    insecure: bool,
    request: &[u8],
) -> Result<Vec<u8>> {
    use rustls::pki_types::ServerName;
    use rustls::{ClientConnection, Stream};

    let config = build_tls_config(insecure)?;
    let server_name: ServerName<'static> = ServerName::try_from(host.to_string())
        .map_err(|e| anyhow!("gopher: invalid hostname '{host}': {e}"))?;
    let mut conn = ClientConnection::new(Arc::new(config), server_name)
        .map_err(|e| anyhow!("gopher: TLS init: {e}"))?;
    let mut tcp = tcp;
    let mut stream = Stream::new(&mut conn, &mut tcp);
    stream.write_all(request).context("gopher (TLS): write")?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).context("gopher (TLS): read")?;
    Ok(buf)
}

fn build_tls_config(insecure: bool) -> Result<rustls::ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    if insecure {
        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("gopher TLS: protocol versions")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(danger::NoopVerifier))
            .with_no_client_auth();
        return Ok(config);
    }
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("gopher TLS: protocol versions")?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(config)
}

mod danger {
    use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use rustls::{DigitallySignedStruct, Error, SignatureScheme};

    #[derive(Debug)]
    pub struct NoopVerifier;
    impl ServerCertVerifier for NoopVerifier {
        fn verify_server_cert(
            &self,
            _: &CertificateDer<'_>,
            _: &[CertificateDer<'_>],
            _: &ServerName<'_>,
            _: &[u8],
            _: UnixTime,
        ) -> Result<ServerCertVerified, Error> {
            Ok(ServerCertVerified::assertion())
        }
        fn verify_tls12_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            Ok(HandshakeSignatureValid::assertion())
        }
        fn verify_tls13_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            Ok(HandshakeSignatureValid::assertion())
        }
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ED25519,
            ]
        }
    }
}

fn parse_url(url: &str) -> Result<(String, u16, bool, Option<char>, String)> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| anyhow!("gopher: URL must be gopher://host[:port]/[TYPE]/selector"))?;
    let tls = match scheme {
        "gopher" => false,
        "gophers" => true,
        other => bail!("gopher: unknown scheme '{other}:' (expected gopher / gophers)"),
    };

    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, p),
        None => (rest, ""),
    };
    let (host, port) = if let Some((h, p)) = authority.rsplit_once(':') {
        let h = h.trim_start_matches('[').trim_end_matches(']');
        (h.to_string(), p.parse::<u16>().map_err(|e| anyhow!("gopher: bad port '{p}': {e}"))?)
    } else {
        (authority.to_string(), DEFAULT_PORT)
    };
    if host.is_empty() {
        bail!("gopher: host missing");
    }

    // Gopher path: first char is the item type (single char), rest is
    // the selector. Empty path = root selector (item type '1').
    let (item_type, selector) = if path.is_empty() {
        (None, String::new())
    } else {
        let mut chars = path.chars();
        let t = chars.next();
        let rest: String = chars.collect();
        let selector = rest.strip_prefix('/').unwrap_or(&rest).to_string();
        (t, selector)
    };

    Ok((host, port, tls, item_type, selector))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_root_selector() {
        let (h, p, tls, t, sel) = parse_url("gopher://gopher.floodgap.com/").unwrap();
        assert_eq!(h, "gopher.floodgap.com");
        assert_eq!(p, 70);
        assert!(!tls);
        assert_eq!(t, None);
        assert_eq!(sel, "");
    }

    #[test]
    fn parse_item_and_selector() {
        let (h, p, tls, t, sel) = parse_url("gopher://host:7070/1/about").unwrap();
        assert_eq!(h, "host");
        assert_eq!(p, 7070);
        assert!(!tls);
        assert_eq!(t, Some('1'));
        assert_eq!(sel, "about");
    }

    #[test]
    fn parse_gophers_tls() {
        let (_, _, tls, _, _) = parse_url("gophers://host/").unwrap();
        assert!(tls);
    }

    #[test]
    fn parse_rejects_bad_scheme() {
        assert!(parse_url("ftp://host/").is_err());
    }
}
