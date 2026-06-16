//! Custom `rustls::ClientConfig` for `--pinnedpubkey` and `--curves`.
//!
//! reqwest exposes no setter for public-key pinning or key-exchange-group
//! selection, so when either flag is set we build a `rustls::ClientConfig`
//! by hand and hand it to `reqwest::ClientBuilder::use_preconfigured_tls`.
//! That call *replaces* reqwest's entire TLS config, so this module must
//! also reproduce the other TLS-affecting flags (roots, version bounds,
//! `--insecure`, `--crlfile`). The common path (neither flag set) keeps
//! using reqwest's high-level setters in `client.rs` — see `needs_custom_tls`.
//!
//! Scope (v1): `--pinnedpubkey` accepts curl's `sha256//<base64>` hash form
//! only (not the file-path form); mTLS (`--cert`/`--key`) combined with
//! pinning/curves is rejected (the custom path doesn't build a client-auth
//! resolver yet). JA3/JA4 are unrelated (impersonate feature).

use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::{ring, CryptoProvider, SupportedKxGroup};
use rustls::pki_types::{CertificateDer, CertificateRevocationListDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use sha2::{Digest, Sha256};

use crate::cli::Args;

/// True when the custom rustls path is required (either flag present).
pub fn needs_custom_tls(args: &Args) -> bool {
    args.pinnedpubkey.is_some() || args.curves.is_some()
}

/// Build a `rustls::ClientConfig` reproducing recon's TLS-affecting flags
/// plus `--pinnedpubkey` / `--curves`. Only called when `needs_custom_tls`.
pub fn build_client_config(args: &Args) -> Result<ClientConfig> {
    // Crypto provider — ring default, with kx_groups overridden by --curves.
    let mut provider = ring::default_provider();
    if let Some(list) = &args.curves {
        provider.kx_groups = parse_curves(list)?;
    }
    let provider = Arc::new(provider);

    let versions = protocol_versions(args)?;
    let roots = Arc::new(build_root_store(args)?);
    let crls = load_crls(args)?;

    // Base verifier: accept-all under --insecure, else webpki chain
    // validation (with CRLs when supplied).
    let base: Arc<dyn ServerCertVerifier> = if args.insecure {
        Arc::new(InsecureVerifier::new(provider.clone()))
    } else {
        let mut builder = WebPkiServerVerifier::builder_with_provider(roots, provider.clone());
        if !crls.is_empty() {
            builder = builder.with_crls(crls);
        }
        builder.build().context("build webpki server verifier")?
    };

    // Wrap with public-key pinning when --pinnedpubkey is set. Pinning is
    // enforced independently of chain validation (so -k --pinnedpubkey
    // still pins, matching curl).
    let verifier: Arc<dyn ServerCertVerifier> = match &args.pinnedpubkey {
        Some(spec) => Arc::new(PinnedKeyVerifier {
            inner: base,
            pins: parse_pinned_pubkey(spec)?,
        }),
        None => base,
    };

    let cfg_builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&versions)
        .context("apply TLS protocol versions")?
        .dangerous()
        .with_custom_certificate_verifier(verifier);

    // mTLS: present a client certificate when --client-cert is set, reusing
    // the same validated PEM bundle as the default reqwest path.
    let mut config = match crate::client_cert::build_rustls_client_auth(args)? {
        Some((certs, key)) => cfg_builder
            .with_client_auth_cert(certs, key)
            .context("--cert/--key: configure client authentication")?,
        None => cfg_builder.with_no_client_auth(),
    };

    // Match reqwest's ALPN so HTTP/2 still negotiates on the custom path.
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

/// Resolve the enabled protocol-version slice from `--tlsv1.2/1.3` (min)
/// and `--tls-max` (max). Defaults to TLS 1.2 + 1.3.
fn protocol_versions(args: &Args) -> Result<Vec<&'static rustls::SupportedProtocolVersion>> {
    // min: 1.3 if --tlsv1.3, else 1.2 (default).
    let min_13 = args.tlsv13;
    // max: from --tls-max (default 1.3).
    let max_12_only = match args.tls_max.as_deref() {
        None | Some("1.3") => false,
        Some("1.2") => true,
        Some(other) => bail!("--tls-max: unknown version '{other}' (expected 1.2 or 1.3)"),
    };
    if min_13 && max_12_only {
        bail!("--tlsv1.3 conflicts with --tls-max 1.2 (empty TLS version range)");
    }
    let mut versions = Vec::new();
    if !min_13 {
        versions.push(&rustls::version::TLS12);
    }
    if !max_12_only {
        versions.push(&rustls::version::TLS13);
    }
    Ok(versions)
}

/// Build the trust-root store: default webpki roots unless `--ca-native`,
/// plus any `--cacert` / `--capath` PEMs.
fn build_root_store(args: &Args) -> Result<RootCertStore> {
    let mut store = RootCertStore::empty();
    if !args.ca_native {
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    if let Some(path) = &args.cacert {
        add_pem_roots(&mut store, path).with_context(|| format!("--cacert: {}", path.display()))?;
    }
    if let Some(dir) = &args.capath {
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("--capath: read dir {}", dir.display()))?;
        for entry in entries.flatten() {
            let p = entry.path();
            let ext_ok = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| {
                    let lo = s.to_ascii_lowercase();
                    lo == "pem" || lo == "crt" || lo == "cer"
                })
                .unwrap_or(false);
            if p.is_file() && ext_ok {
                add_pem_roots(&mut store, &p)
                    .with_context(|| format!("--capath: {}", p.display()))?;
            }
        }
    }
    Ok(store)
}

fn add_pem_roots(store: &mut RootCertStore, path: &std::path::Path) -> Result<()> {
    let pem = std::fs::read(path)?;
    let mut rd = std::io::BufReader::new(&pem[..]);
    for cert in rustls_pemfile::certs(&mut rd) {
        let cert = cert.context("parse PEM certificate")?;
        store.add(cert).context("add root certificate")?;
    }
    Ok(())
}

fn load_crls(args: &Args) -> Result<Vec<CertificateRevocationListDer<'static>>> {
    let Some(path) = &args.crlfile else {
        return Ok(Vec::new());
    };
    let pem = std::fs::read(path).with_context(|| format!("--crlfile: read {}", path.display()))?;
    let mut rd = std::io::BufReader::new(&pem[..]);
    let mut out = Vec::new();
    for crl in rustls_pemfile::crls(&mut rd) {
        out.push(crl.with_context(|| format!("--crlfile: parse {}", path.display()))?);
    }
    Ok(out)
}

// ---- --curves -------------------------------------------------------------

/// Parse a colon-separated curve list (curl/OpenSSL names) into ordered
/// rustls key-exchange groups.
pub fn parse_curves(list: &str) -> Result<Vec<&'static dyn SupportedKxGroup>> {
    let mut out: Vec<&'static dyn SupportedKxGroup> = Vec::new();
    for name in list.split(':') {
        let n = name.trim();
        if n.is_empty() {
            continue;
        }
        out.push(kx_group_for(n)?);
    }
    if out.is_empty() {
        bail!("--curves: no curves in '{list}'");
    }
    Ok(out)
}

fn kx_group_for(name: &str) -> Result<&'static dyn SupportedKxGroup> {
    Ok(match name.to_ascii_lowercase().as_str() {
        "x25519" => ring::kx_group::X25519,
        "p-256" | "prime256v1" | "secp256r1" => ring::kx_group::SECP256R1,
        "p-384" | "secp384r1" => ring::kx_group::SECP384R1,
        "p-521" | "secp521r1" => bail!(
            "--curves: secp521r1 (P-521) is unavailable under the ring crypto \
             backend recon builds against"
        ),
        other => bail!("--curves: unknown curve '{other}' (supported: X25519, P-256, P-384)"),
    })
}

// ---- --pinnedpubkey -------------------------------------------------------

/// Parse curl's `sha256//<base64>[;sha256//<base64>...]` pin spec into a
/// set of 32-byte SHA-256 digests. The file-path form is rejected.
pub fn parse_pinned_pubkey(spec: &str) -> Result<Vec<[u8; 32]>> {
    let mut out = Vec::new();
    for part in spec.split(';') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let b64 = p.strip_prefix("sha256//").ok_or_else(|| {
            anyhow!(
                "--pinnedpubkey: entry '{p}' must be 'sha256//<base64>' \
                 (public-key file paths are not supported in this release)"
            )
        })?;
        let bytes = STANDARD
            .decode(b64)
            .with_context(|| format!("--pinnedpubkey: invalid base64 in '{p}'"))?;
        if bytes.len() != 32 {
            bail!(
                "--pinnedpubkey: '{p}' decodes to {} bytes, not a 32-byte SHA-256",
                bytes.len()
            );
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        out.push(arr);
    }
    if out.is_empty() {
        bail!("--pinnedpubkey: no valid 'sha256//' pins in '{spec}'");
    }
    Ok(out)
}

/// SHA-256 of a DER certificate's `SubjectPublicKeyInfo` (curl's pin basis).
fn spki_sha256(cert_der: &[u8]) -> Result<[u8; 32]> {
    use x509_parser::prelude::FromDer;
    let (_, cert) = x509_parser::certificate::X509Certificate::from_der(cert_der)
        .map_err(|e| anyhow!("parse server certificate for pinning: {e}"))?;
    let spki_der = cert.public_key().raw; // DER of the SubjectPublicKeyInfo
    let digest = Sha256::digest(spki_der);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

/// Wraps an inner verifier and additionally requires the leaf cert's
/// SPKI-SHA256 to match one of the pinned hashes.
#[derive(Debug)]
struct PinnedKeyVerifier {
    inner: Arc<dyn ServerCertVerifier>,
    pins: Vec<[u8; 32]>,
}

impl ServerCertVerifier for PinnedKeyVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Chain validation first (or accept-all when inner is insecure).
        self.inner.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )?;
        // Then the pin: leaf SPKI SHA-256 must match at least one pin.
        let got = spki_sha256(end_entity.as_ref())
            .map_err(|e| rustls::Error::General(format!("pinnedpubkey: {e}")))?;
        if self.pins.iter().any(|p| *p == got) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "pinnedpubkey: server public-key hash does not match any --pinnedpubkey value"
                    .into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Accept-any verifier for `--insecure`. Pinning (if any) is layered on top
/// by `PinnedKeyVerifier`, so a bad cert chain is tolerated but the pin is
/// still enforced.
#[derive(Debug)]
struct InsecureVerifier {
    schemes: Vec<SignatureScheme>,
}

impl InsecureVerifier {
    fn new(provider: Arc<CryptoProvider>) -> Self {
        Self {
            schemes: provider
                .signature_verification_algorithms
                .supported_schemes(),
        }
    }
}

impl ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_parse_known_names_in_order() {
        let g = parse_curves("X25519:P-256:P-384").unwrap();
        assert_eq!(g.len(), 3);
        // order preserved: first is X25519
        assert_eq!(g[0].name(), ring::kx_group::X25519.name());
        assert_eq!(g[1].name(), ring::kx_group::SECP256R1.name());
    }

    #[test]
    fn curves_accept_openssl_aliases() {
        assert!(parse_curves("prime256v1:secp384r1").is_ok());
    }

    #[test]
    fn curves_p521_errors_under_ring() {
        let e = parse_curves("P-521").unwrap_err().to_string();
        assert!(e.contains("secp521r1") && e.contains("ring"), "got: {e}");
    }

    #[test]
    fn curves_unknown_errors() {
        assert!(parse_curves("bogus").is_err());
    }

    #[test]
    fn curves_empty_errors() {
        assert!(parse_curves(" : ").is_err());
    }

    #[test]
    fn pinned_parses_single_and_multiple() {
        let h = STANDARD.encode([0u8; 32]);
        let one = parse_pinned_pubkey(&format!("sha256//{h}")).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0], [0u8; 32]);
        let two = parse_pinned_pubkey(&format!("sha256//{h};sha256//{h}")).unwrap();
        assert_eq!(two.len(), 2);
    }

    #[test]
    fn pinned_rejects_missing_prefix() {
        let h = STANDARD.encode([0u8; 32]);
        assert!(parse_pinned_pubkey(&h).is_err());
    }

    #[test]
    fn pinned_rejects_file_path_form() {
        let e = parse_pinned_pubkey("/etc/keys/pub.der")
            .unwrap_err()
            .to_string();
        assert!(e.contains("sha256//"), "got: {e}");
    }

    #[test]
    fn pinned_rejects_bad_base64() {
        assert!(parse_pinned_pubkey("sha256//!!!notbase64!!!").is_err());
    }

    #[test]
    fn pinned_rejects_wrong_length() {
        let short = STANDARD.encode([0u8; 16]);
        let e = parse_pinned_pubkey(&format!("sha256//{short}"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("32-byte"), "got: {e}");
    }
}
