use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use native_tls::TlsConnector;
use std::net::TcpStream;
use url::Url;
use x509_parser::prelude::*;

/// Resolve a user-provided target (bare host / host:port / https URL) into
/// `(host, port)`. Errors if the scheme is not https.
pub fn parse_target(url_str: &str) -> Result<(String, u16)> {
    let normalised = if url_str.contains("://") {
        url_str.to_string()
    } else {
        format!("https://{url_str}")
    };
    let parsed = Url::parse(&normalised).with_context(|| format!("Invalid URL: {url_str}"))?;
    if parsed.scheme() != "https" {
        return Err(anyhow!(
            "--cert only works with HTTPS URLs (got: {}://)",
            parsed.scheme()
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("Could not extract host from URL"))?
        .to_string();
    let port = parsed.port().unwrap_or(443);
    Ok((host, port))
}

/// Connect, perform a TLS handshake (hostname verification off, so that
/// self-signed / expired / mismatched certs can still be inspected), and
/// return the peer certificate's DER bytes.
pub fn fetch_der(host: &str, port: u16) -> Result<Vec<u8>> {
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .context("Failed to build TLS connector")?;

    let tcp = TcpStream::connect(format!("{host}:{port}"))
        .with_context(|| format!("Could not connect to {host}:{port}"))?;

    let tls = connector
        .connect(host, tcp)
        .with_context(|| format!("TLS handshake with {host} failed"))?;

    let native_cert = tls
        .peer_certificate()
        .context("Failed to retrieve server certificate")?
        .ok_or_else(|| anyhow!("Server did not send a certificate"))?;

    native_cert
        .to_der()
        .context("Failed to export certificate as DER")
}

pub fn fetch_and_print(url_str: &str) -> Result<()> {
    let (host, port) = parse_target(url_str)?;
    let der = fetch_der(&host, port)?;
    let (_, cert) = X509Certificate::from_der(&der)
        .map_err(|e| anyhow!("Failed to parse certificate: {e}"))?;
    print_cert(&cert, &host, port);
    Ok(())
}

fn print_cert(cert: &X509Certificate, host: &str, port: u16) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let not_before_ts = cert.validity().not_before.timestamp();
    let not_after_ts = cert.validity().not_after.timestamp();
    let days_remaining = (not_after_ts - now) / 86400;

    let is_expired = now > not_after_ts;
    let is_not_yet_valid = now < not_before_ts;
    let is_expiring_soon = !is_expired && days_remaining <= 30;

    println!("Certificate for {}:{}", host, port);
    println!("{}", "═".repeat(50));
    println!();

    println!("Subject:");
    print_name(cert.subject());
    println!();

    println!("Issuer:");
    print_name(cert.issuer());
    println!();

    println!("Validity:");
    println!("  Not Before:  {}", cert.validity().not_before);
    println!("  Not After:   {}", cert.validity().not_after);
    let status_str = if is_expired {
        format!("EXPIRED ({} days ago)", -days_remaining)
            .red()
            .bold()
            .to_string()
    } else if is_not_yet_valid {
        "Not yet valid".yellow().to_string()
    } else if is_expiring_soon {
        format!("Valid — expires in {} day(s)", days_remaining)
            .yellow()
            .to_string()
    } else {
        format!("Valid — expires in {} days", days_remaining)
            .green()
            .to_string()
    };
    println!("  Status:      {}", status_str);
    println!();

    // Subject Alternative Names
    let mut san_names: Vec<String> = Vec::new();
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for general_name in &san.general_names {
                match general_name {
                    GeneralName::DNSName(dns) => san_names.push(dns.to_string()),
                    GeneralName::IPAddress(ip) if ip.len() == 4 => {
                        san_names.push(format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]));
                    }
                    GeneralName::RFC822Name(email) => san_names.push(format!("email:{email}")),
                    _ => {}
                }
            }
        }
    }
    if !san_names.is_empty() {
        println!("Subject Alternative Names:");
        for name in &san_names {
            println!("  {name}");
        }
        println!();
    }

    // Serial number
    let serial_hex = cert
        .raw_serial()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":");
    println!("Serial Number:  {serial_hex}");

    // Signature algorithm
    let sig_oid = cert.signature_algorithm.algorithm.to_string();
    println!("Algorithm:      {}", sig_algo_name(&sig_oid));

    // Public key type and size
    let pk_oid = cert.public_key().algorithm.algorithm.to_string();
    let pk_info = if pk_oid == "1.2.840.113549.1.1.1" {
        match cert.public_key().parsed() {
            Ok(x509_parser::public_key::PublicKey::RSA(rsa)) => {
                // DER-encoded modulus may have a leading 0x00 padding byte for positive integers
                let byte_len = rsa.modulus.len().saturating_sub(if rsa.modulus.first() == Some(&0) { 1 } else { 0 });
                format!("RSA {}-bit", byte_len * 8)
            }
            _ => "RSA".to_string(),
        }
    } else {
        match pk_oid.as_str() {
            "1.2.840.10045.2.1" => "Elliptic Curve (EC)".to_string(),
            "1.3.101.112" => "Ed25519".to_string(),
            "1.3.101.113" => "Ed448".to_string(),
            other => other.to_string(),
        }
    };
    println!("Public Key:     {pk_info}");
}

fn print_name(name: &X509Name) {
    for cn in name.iter_common_name() {
        if let Ok(s) = cn.as_str() {
            println!("  Common Name:   {s}");
        }
    }
    for o in name.iter_organization() {
        if let Ok(s) = o.as_str() {
            println!("  Organization:  {s}");
        }
    }
    for ou in name.iter_organizational_unit() {
        if let Ok(s) = ou.as_str() {
            println!("  Org Unit:      {s}");
        }
    }
    for c in name.iter_country() {
        if let Ok(s) = c.as_str() {
            println!("  Country:       {s}");
        }
    }
    for st in name.iter_state_or_province() {
        if let Ok(s) = st.as_str() {
            println!("  State:         {s}");
        }
    }
    for l in name.iter_locality() {
        if let Ok(s) = l.as_str() {
            println!("  Locality:      {s}");
        }
    }
}

fn sig_algo_name(oid: &str) -> &str {
    match oid {
        "1.2.840.113549.1.1.5"  => "SHA1 with RSA",
        "1.2.840.113549.1.1.11" => "SHA256 with RSA",
        "1.2.840.113549.1.1.12" => "SHA384 with RSA",
        "1.2.840.113549.1.1.13" => "SHA512 with RSA",
        "1.2.840.10045.4.3.1"   => "SHA224 with ECDSA",
        "1.2.840.10045.4.3.2"   => "SHA256 with ECDSA",
        "1.2.840.10045.4.3.3"   => "SHA384 with ECDSA",
        "1.2.840.10045.4.3.4"   => "SHA512 with ECDSA",
        "1.3.101.112"           => "Ed25519",
        "1.3.101.113"           => "Ed448",
        other => other,
    }
}
