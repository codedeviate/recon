use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, Error as TlsError,
    ProtocolVersion, SignatureScheme,
};
use x509_parser::prelude::*;

// ── Result type ───────────────────────────────────────────────────────────────

pub struct TlsProbeResult {
    pub version: &'static str,  // "TLSv1.3" / "TLSv1.2" / "TLS"
    pub cipher: String,         // "TLS13_CHACHA20_POLY1305_SHA256" etc.
    pub alpn: Option<String>,   // "h2" / "http/1.1"
    pub subject: String,
    pub issuer: String,
    pub not_after: String,
    pub days_remaining: i64,
    pub is_expired: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Completes a TLS handshake to `host:port` and returns session + certificate info.
/// Certificate verification is intentionally disabled so this works for dev / self-signed certs.
pub fn probe(host: &str, port: u16) -> Result<TlsProbeResult> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("Failed to configure TLS protocol versions")?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipVerification))
        .with_no_client_auth();

    // Advertise both h2 and http/1.1 so ALPN negotiation is visible
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let server_name = ServerName::try_from(host)
        .map_err(|_| anyhow!("Invalid TLS server name: {host}"))?
        .to_owned();

    let mut conn = ClientConnection::new(Arc::new(config), server_name)
        .context("Failed to create TLS client connection")?;

    let mut sock = TcpStream::connect(format!("{host}:{port}"))
        .with_context(|| format!("TCP connection to {host}:{port} failed"))?;
    sock.set_read_timeout(Some(Duration::from_secs(10)))?;

    // Drive the handshake to completion
    for _ in 0..100 {
        while conn.wants_write() {
            conn.write_tls(&mut sock).context("TLS write failed")?;
        }
        if !conn.is_handshaking() {
            break;
        }
        if conn.wants_read() {
            let n = conn.read_tls(&mut sock).context("TLS read failed")?;
            if n == 0 {
                break;
            }
            conn.process_new_packets()
                .map_err(|e| anyhow!("TLS handshake error: {e}"))?;
        }
    }

    let version = match conn.protocol_version() {
        Some(ProtocolVersion::TLSv1_3) => "TLSv1.3",
        Some(ProtocolVersion::TLSv1_2) => "TLSv1.2",
        Some(ProtocolVersion::TLSv1_1) => "TLSv1.1",
        Some(ProtocolVersion::TLSv1_0) => "TLSv1.0",
        _ => "TLS",
    };

    let cipher = conn
        .negotiated_cipher_suite()
        .map(|cs| format!("{:?}", cs.suite()))
        .unwrap_or_else(|| "(unknown)".to_string());

    let alpn = conn
        .alpn_protocol()
        .map(|p| String::from_utf8_lossy(p).into_owned());

    // Extract cert bytes while conn is still alive
    let cert_der: Option<Vec<u8>> = conn
        .peer_certificates()
        .and_then(|c| c.first())
        .map(|d| d.to_vec());

    let (subject, issuer, not_after, days_remaining, is_expired) = cert_der
        .as_deref()
        .and_then(parse_cert_summary)
        .unwrap_or_else(|| {
            (
                "(unknown)".to_string(),
                "(unknown)".to_string(),
                "(unknown)".to_string(),
                0,
                false,
            )
        });

    Ok(TlsProbeResult {
        version,
        cipher,
        alpn,
        subject,
        issuer,
        not_after,
        days_remaining,
        is_expired,
    })
}

// ── Certificate parsing ───────────────────────────────────────────────────────

fn parse_cert_summary(der: &[u8]) -> Option<(String, String, String, i64, bool)> {
    let (_, cert) = X509Certificate::from_der(der).ok()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    let not_after_ts = cert.validity().not_after.timestamp();
    let days_remaining = (not_after_ts - now) / 86400;
    let is_expired = now > not_after_ts;

    Some((
        x509_name_summary(cert.subject()),
        x509_name_summary(cert.issuer()),
        cert.validity().not_after.to_string(),
        days_remaining,
        is_expired,
    ))
}

/// Returns "CN=…; O=…; OU=…" preserving the order from the certificate.
fn x509_name_summary(name: &X509Name) -> String {
    let mut parts = Vec::new();
    for attr in name.iter_attributes() {
        let Ok(val) = attr.attr_value().as_str() else {
            continue;
        };
        let label = match attr.attr_type().to_id_string().as_str() {
            "2.5.4.3"  => "CN",
            "2.5.4.10" => "O",
            "2.5.4.11" => "OU",
            "2.5.4.6"  => "C",
            "2.5.4.8"  => "ST",
            "2.5.4.7"  => "L",
            _          => continue,
        };
        parts.push(format!("{label}={val}"));
    }
    if parts.is_empty() {
        format!("{name}")
    } else {
        parts.join("; ")
    }
}

// ── Skip-all certificate verifier (for informational pre-flight only) ─────────

#[derive(Debug)]
struct SkipVerification;

impl ServerCertVerifier for SkipVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
