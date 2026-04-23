//! Client-certificate loader — turns the `--cert / --key / --cert-type /
//! --key-type / --pass` flag cluster into a `reqwest::Identity`.
//!
//! Scope for 0.54.0: PEM-only, both combined (cert + key in one file)
//! and split (`--cert` + `--key`). DER format is accepted at parse
//! time but produces a clear error pointing users at PEM; `--key-type
//! ENG` is rejected because rustls has no crypto-engine concept.
//!
//! The combined path handles the common case: a single PEM with both
//! a `-----BEGIN CERTIFICATE-----` chain AND a `-----BEGIN PRIVATE
//! KEY-----` block. The split path concatenates the two files in
//! memory before handing to `reqwest::Identity::from_pem`, which
//! accepts the same combined form.

use anyhow::{bail, Context, Result};

use crate::cli::Args;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CertFormat {
    Pem,
    Der,
}

impl CertFormat {
    fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "PEM" => Ok(CertFormat::Pem),
            "DER" => Ok(CertFormat::Der),
            other => bail!("--cert-type / --key-type: unknown value '{other}' (want PEM or DER)"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyFormat {
    Pem,
    Der,
    Eng,
}

impl KeyFormat {
    fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "PEM" => Ok(KeyFormat::Pem),
            "DER" => Ok(KeyFormat::Der),
            "ENG" | "ENGINE" => Ok(KeyFormat::Eng),
            other => bail!("--key-type: unknown value '{other}' (want PEM, DER, or ENG)"),
        }
    }
}

/// Build a reqwest `Identity` from the flag cluster. Returns `None`
/// when `--cert` isn't set. Validates formats even when no identity is
/// produced so users see format errors early.
pub fn build_identity(args: &Args) -> Result<Option<reqwest::Identity>> {
    let cert_path = match args.client_cert.as_ref() {
        Some(p) => p,
        None => {
            // Still validate the format flags so a misspelled --key-type
            // surfaces even when the user forgot --cert entirely.
            CertFormat::parse(&args.cert_type)?;
            KeyFormat::parse(&args.key_type)?;
            return Ok(None);
        }
    };

    let cert_format = CertFormat::parse(&args.cert_type)?;
    let key_format = KeyFormat::parse(&args.key_type)?;

    match key_format {
        KeyFormat::Eng => bail!(
            "--key-type ENG: rustls has no crypto-engine concept — \
             client-cert loading uses file-backed PEM keys only. \
             Export the key to a PEM file and retry."
        ),
        KeyFormat::Der => bail!(
            "--key-type DER: not yet wired into the rustls-backed \
             client-cert path. Convert the key to PEM \
             (`openssl pkcs8 -topk8 -nocrypt -in key.der -inform DER \
             -out key.pem`) and retry."
        ),
        KeyFormat::Pem => {}
    }

    if cert_format == CertFormat::Der {
        bail!(
            "--cert-type DER: not yet wired into the rustls-backed \
             client-cert path. Convert the cert to PEM \
             (`openssl x509 -in cert.der -inform DER -out cert.pem`) \
             and retry."
        );
    }

    let cert_bytes = std::fs::read(cert_path)
        .with_context(|| format!("--cert: read {}", cert_path.display()))?;

    let combined = match args.client_key.as_ref() {
        Some(key_path) => {
            let key_bytes = std::fs::read(key_path)
                .with_context(|| format!("--key: read {}", key_path.display()))?;
            if has_encrypted_key(&key_bytes) {
                bail!(
                    "--key: encrypted PKCS#8 keys are not yet decrypted by \
                     recon. Decrypt externally first: \
                     `openssl pkcs8 -in {0} -out {0}.plain` then pass \
                     --key {0}.plain.",
                    key_path.display()
                );
            }
            if args.cert_pass.is_some() && !has_encrypted_key(&key_bytes) {
                eprintln!(
                    "warning: --pass ignored — key file contains no \
                     ENCRYPTED PRIVATE KEY block"
                );
            }
            let mut out = cert_bytes.clone();
            if !out.ends_with(b"\n") {
                out.push(b'\n');
            }
            out.extend_from_slice(&key_bytes);
            out
        }
        None => {
            if has_encrypted_key(&cert_bytes) {
                bail!(
                    "--cert: combined PEM contains an encrypted key block. \
                     Split the cert and key, decrypt the key with \
                     `openssl pkcs8`, then re-feed via --cert + --key."
                );
            }
            cert_bytes
        }
    };

    let identity = reqwest::Identity::from_pem(&combined)
        .context("--cert/--key: failed to build TLS identity from PEM bundle")?;
    Ok(Some(identity))
}

fn has_encrypted_key(pem: &[u8]) -> bool {
    std::str::from_utf8(pem)
        .map(|s| s.contains("ENCRYPTED PRIVATE KEY"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn args_with(extra: &[&str]) -> Args {
        let mut argv = vec!["recon", "https://example.com/"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv).expect("parse")
    }

    #[test]
    fn no_cert_returns_none() {
        let a = args_with(&[]);
        assert!(build_identity(&a).unwrap().is_none());
    }

    #[test]
    fn eng_key_type_errors() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"dummy").unwrap();
        let a = args_with(&["--client-cert", f.path().to_str().unwrap(), "--key-type", "ENG"]);
        let err = build_identity(&a).unwrap_err().to_string();
        assert!(err.contains("ENG"), "got: {err}");
    }

    #[test]
    fn der_cert_errors_cleanly() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"dummy").unwrap();
        let a = args_with(&["--client-cert", f.path().to_str().unwrap(), "--cert-type", "DER"]);
        let err = build_identity(&a).unwrap_err().to_string();
        assert!(err.contains("DER"), "got: {err}");
    }

    #[test]
    fn format_validated_even_without_cert() {
        let a = args_with(&["--cert-type", "bogus"]);
        assert!(build_identity(&a).is_err());
    }

    #[test]
    fn bogus_pem_errors_on_load() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this is not a PEM").unwrap();
        let a = args_with(&["--client-cert", f.path().to_str().unwrap()]);
        let err = build_identity(&a).unwrap_err().to_string();
        assert!(err.contains("PEM") || err.contains("identity"), "got: {err}");
    }

    #[test]
    fn encrypted_key_is_refused_with_clear_message() {
        let mut cf = NamedTempFile::new().unwrap();
        cf.write_all(b"-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n")
            .unwrap();
        let mut kf = NamedTempFile::new().unwrap();
        kf.write_all(
            b"-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA\n-----END ENCRYPTED PRIVATE KEY-----\n",
        )
        .unwrap();
        let a = args_with(&[
            "--client-cert",
            cf.path().to_str().unwrap(),
            "--key",
            kf.path().to_str().unwrap(),
        ]);
        let err = build_identity(&a).unwrap_err().to_string();
        assert!(err.to_lowercase().contains("encrypted"), "got: {err}");
    }

    #[test]
    fn detects_encrypted_in_combined_pem() {
        let mut cf = NamedTempFile::new().unwrap();
        cf.write_all(
            b"-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n\
              -----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA\n-----END ENCRYPTED PRIVATE KEY-----\n",
        )
        .unwrap();
        let a = args_with(&["--client-cert", cf.path().to_str().unwrap()]);
        let err = build_identity(&a).unwrap_err().to_string();
        assert!(err.to_lowercase().contains("encrypted"), "got: {err}");
    }
}
