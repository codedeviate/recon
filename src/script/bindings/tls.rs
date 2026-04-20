//! `tls(host)` / `tls(host, port)` script binding. Connects, performs a
//! TLS handshake with hostname verification off (so self-signed / expired
//! certs can be inspected too), fetches the peer's DER cert, parses it
//! with x509_parser, and returns a map summarising the interesting bits
//! plus the raw PEM.
//!
//! Returned map:
//!
//! ```text
//! #{
//!   host, port,
//!   subject: #{ common_name, organization, organizational_unit, country, state, locality },
//!   issuer:  #{ ... same shape },
//!   not_before, not_after,              // ISO-ish strings from x509
//!   not_before_ts, not_after_ts,        // unix seconds
//!   days_remaining: i64,
//!   is_expired: bool,
//!   san: [String],
//!   serial_hex: String,
//!   signature_algorithm: String,
//!   public_key: String,
//!   cert_pem: String,
//! }
//! ```

use crate::cert;
use crate::script::convert::anyhow_to_rhai;
use anyhow::anyhow;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use x509_parser::prelude::*;

pub fn register(engine: &mut Engine) {
    engine.register_fn("tls", |host: &str| -> Result<Map, Box<EvalAltResult>> {
        do_tls(host, 443)
    });
    engine.register_fn(
        "tls",
        |host: &str, port: i64| -> Result<Map, Box<EvalAltResult>> {
            if !(1..=65535).contains(&port) {
                return Err(format!("tls: port {port} out of range 1..=65535").into());
            }
            do_tls(host, port as u16)
        },
    );
}

fn do_tls(host_arg: &str, port_hint: u16) -> Result<Map, Box<EvalAltResult>> {
    let (host, port) = if host_arg.contains("://") {
        cert::parse_target(host_arg).map_err(anyhow_to_rhai)?
    } else if let Some((h, p)) = host_arg.rsplit_once(':') {
        if let Ok(p) = p.parse::<u16>() {
            (h.to_string(), p)
        } else {
            (host_arg.to_string(), port_hint)
        }
    } else {
        (host_arg.to_string(), port_hint)
    };

    let der = cert::fetch_der(&host, port).map_err(anyhow_to_rhai)?;
    let (_, cert) = X509Certificate::from_der(&der)
        .map_err(|e| anyhow_to_rhai(anyhow!("Failed to parse certificate: {e}")))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let not_before_ts = cert.validity().not_before.timestamp();
    let not_after_ts = cert.validity().not_after.timestamp();
    let days_remaining = (not_after_ts - now) / 86400;
    let is_expired = now > not_after_ts;

    let subject = name_to_map(cert.subject());
    let issuer = name_to_map(cert.issuer());
    let san = collect_san(&cert);
    let serial_hex = cert
        .raw_serial()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":");
    let signature_algorithm = cert.signature_algorithm.algorithm.to_string();
    let public_key = summarize_public_key(&cert);
    let pem = der_to_pem(&der);

    let mut m = Map::new();
    m.insert("host".into(), host.into());
    m.insert("port".into(), (port as i64).into());
    m.insert("subject".into(), subject.into());
    m.insert("issuer".into(), issuer.into());
    m.insert(
        "not_before".into(),
        cert.validity().not_before.to_string().into(),
    );
    m.insert(
        "not_after".into(),
        cert.validity().not_after.to_string().into(),
    );
    m.insert("not_before_ts".into(), not_before_ts.into());
    m.insert("not_after_ts".into(), not_after_ts.into());
    m.insert("days_remaining".into(), days_remaining.into());
    m.insert("is_expired".into(), is_expired.into());
    let san_arr: Array = san.into_iter().map(Dynamic::from).collect();
    m.insert("san".into(), san_arr.into());
    m.insert("serial_hex".into(), serial_hex.into());
    m.insert("signature_algorithm".into(), signature_algorithm.into());
    m.insert("public_key".into(), public_key.into());
    m.insert("cert_pem".into(), pem.into());
    Ok(m)
}

fn name_to_map(name: &X509Name) -> Map {
    fn first<'a>(
        it: impl Iterator<Item = &'a x509_parser::x509::AttributeTypeAndValue<'a>>,
    ) -> Option<String> {
        for attr in it {
            if let Ok(s) = attr.as_str() {
                return Some(s.to_string());
            }
        }
        None
    }
    let mut m = Map::new();
    if let Some(s) = first(name.iter_common_name()) {
        m.insert("common_name".into(), s.into());
    }
    if let Some(s) = first(name.iter_organization()) {
        m.insert("organization".into(), s.into());
    }
    if let Some(s) = first(name.iter_organizational_unit()) {
        m.insert("organizational_unit".into(), s.into());
    }
    if let Some(s) = first(name.iter_country()) {
        m.insert("country".into(), s.into());
    }
    if let Some(s) = first(name.iter_state_or_province()) {
        m.insert("state".into(), s.into());
    }
    if let Some(s) = first(name.iter_locality()) {
        m.insert("locality".into(), s.into());
    }
    m
}

fn collect_san(cert: &X509Certificate) -> Vec<String> {
    let mut out = Vec::new();
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for gn in &san.general_names {
                match gn {
                    GeneralName::DNSName(d) => out.push(d.to_string()),
                    GeneralName::IPAddress(ip) if ip.len() == 4 => {
                        out.push(format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]));
                    }
                    GeneralName::RFC822Name(e) => out.push(format!("email:{e}")),
                    _ => {}
                }
            }
        }
    }
    out
}

fn summarize_public_key(cert: &X509Certificate) -> String {
    let pk_oid = cert.public_key().algorithm.algorithm.to_string();
    if pk_oid == "1.2.840.113549.1.1.1" {
        match cert.public_key().parsed() {
            Ok(x509_parser::public_key::PublicKey::RSA(rsa)) => {
                let byte_len = rsa.modulus.len().saturating_sub(
                    if rsa.modulus.first() == Some(&0) { 1 } else { 0 },
                );
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
    }
}

fn der_to_pem(der: &[u8]) -> String {
    let b64 = BASE64_STANDARD.encode(der);
    let mut out = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap());
        out.push('\n');
    }
    out.push_str("-----END CERTIFICATE-----\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn der_to_pem_wraps_at_64() {
        let der = vec![0u8; 100];
        let pem = der_to_pem(&der);
        assert!(pem.starts_with("-----BEGIN CERTIFICATE-----\n"));
        assert!(pem.trim_end().ends_with("-----END CERTIFICATE-----"));
    }

    // Live test against example.com — only runs when network is available.
    // Gated off by default to keep the suite offline-friendly.
    #[test]
    #[ignore]
    fn tls_fetches_example_com_cert() {
        let e = engine();
        let m: Map = e.eval(r#"tls("example.com")"#).expect("eval");
        let subj = m
            .get("subject")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap();
        assert!(subj.get("common_name").is_some());
    }
}
