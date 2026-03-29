use anyhow::{bail, Context, Result};
use colored::Colorize;
use rustls::crypto::ring::sign::any_supported_type;
use rustls::server::ResolvesServerCert;
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A single SNI entry mapping a hostname to its cert and key files.
pub struct SniEntry {
    pub hostname: String,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

/// Auto-detect the format and parse one `--serve-sni` value into entries.
///
/// Formats:
///   - Inline:    `hostname:cert.pem:key.pem`
///   - Directory: scan for `<host>-cert.pem` / `<host>-key.pem` pairs
///   - Config:    file with lines `hostname cert.pem key.pem` (# comments, blank lines skipped)
pub fn parse_sni_mapping(value: &str) -> Result<Vec<SniEntry>> {
    // Inline format: contains `:` and is not a plain filesystem path
    if value.contains(':') {
        let parts: Vec<&str> = value.splitn(3, ':').collect();
        if parts.len() != 3 {
            bail!(
                "Invalid inline SNI mapping: expected host:cert:key, got {value:?}"
            );
        }
        let hostname = parts[0].to_lowercase();
        let cert_path = PathBuf::from(parts[1]);
        let key_path = PathBuf::from(parts[2]);
        if !cert_path.exists() {
            bail!("SNI cert not found for {hostname}: {}", cert_path.display());
        }
        if !key_path.exists() {
            bail!("SNI key not found for {hostname}: {}", key_path.display());
        }
        return Ok(vec![SniEntry {
            hostname,
            cert_path,
            key_path,
        }]);
    }

    let path = Path::new(value);

    // Directory mode
    if path.is_dir() {
        return parse_directory(path);
    }

    // Config file mode
    if path.is_file() {
        return parse_config_file(path);
    }

    bail!(
        "SNI mapping {value:?} is not a valid inline mapping (host:cert:key), \
         directory, or config file"
    );
}

/// Scan a directory for `<host>-cert.pem` / `<host>-key.pem` pairs.
fn parse_directory(dir: &Path) -> Result<Vec<SniEntry>> {
    let mut certs: HashMap<String, PathBuf> = HashMap::new();
    let mut keys: HashMap<String, PathBuf> = HashMap::new();

    for entry in fs::read_dir(dir)
        .with_context(|| format!("Cannot read SNI directory: {}", dir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(host) = name.strip_suffix("-cert.pem") {
            certs.insert(host.to_lowercase(), entry.path());
        } else if let Some(host) = name.strip_suffix("-key.pem") {
            keys.insert(host.to_lowercase(), entry.path());
        }
    }

    // Warn about orphans
    for host in certs.keys() {
        if !keys.contains_key(host) {
            eprintln!(
                "{} SNI directory: found {host}-cert.pem but no {host}-key.pem — skipping",
                "warning:".yellow().bold()
            );
        }
    }
    for host in keys.keys() {
        if !certs.contains_key(host) {
            eprintln!(
                "{} SNI directory: found {host}-key.pem but no {host}-cert.pem — skipping",
                "warning:".yellow().bold()
            );
        }
    }

    let mut entries = Vec::new();
    for (host, cert_path) in &certs {
        if let Some(key_path) = keys.get(host) {
            entries.push(SniEntry {
                hostname: host.clone(),
                cert_path: cert_path.clone(),
                key_path: key_path.clone(),
            });
        }
    }

    if entries.is_empty() {
        bail!(
            "No valid SNI cert/key pairs found in directory: {}\n\
             Expected files named <hostname>-cert.pem and <hostname>-key.pem",
            dir.display()
        );
    }

    Ok(entries)
}

/// Parse a config file with lines: `hostname cert.pem key.pem`.
fn parse_config_file(path: &Path) -> Result<Vec<SniEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Cannot read SNI config file: {}", path.display()))?;

    let config_dir = path.parent().unwrap_or(Path::new("."));
    let mut entries = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            bail!(
                "{}:{}: expected `hostname cert.pem key.pem`, got: {line}",
                path.display(),
                line_no + 1,
            );
        }

        let hostname = parts[0].to_lowercase();
        let cert_path = config_dir.join(parts[1]);
        let key_path = config_dir.join(parts[2]);

        if !cert_path.exists() {
            bail!(
                "{}:{}: cert not found: {}",
                path.display(),
                line_no + 1,
                cert_path.display()
            );
        }
        if !key_path.exists() {
            bail!(
                "{}:{}: key not found: {}",
                path.display(),
                line_no + 1,
                key_path.display()
            );
        }

        entries.push(SniEntry {
            hostname,
            cert_path,
            key_path,
        });
    }

    if entries.is_empty() {
        bail!(
            "SNI config file contains no entries: {}",
            path.display()
        );
    }

    Ok(entries)
}

/// Load a cert + key from disk into a `CertifiedKey`.
fn load_certified_key(cert_path: &Path, key_path: &Path) -> Result<CertifiedKey> {
    let cert_bytes = fs::read(cert_path)
        .with_context(|| format!("Failed to read SNI cert: {}", cert_path.display()))?;
    let key_bytes = fs::read(key_path)
        .with_context(|| format!("Failed to read SNI key: {}", key_path.display()))?;

    let certs: Vec<rustls::pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(cert_bytes.as_slice()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("Failed to parse certs from {}", cert_path.display()))?;

    let key = rustls_pemfile::private_key(&mut BufReader::new(key_bytes.as_slice()))
        .with_context(|| format!("Failed to parse key from {}", key_path.display()))?
        .with_context(|| format!("No private key found in {}", key_path.display()))?;

    let signing_key = any_supported_type(&key)
        .context("Unsupported private key type for SNI entry")?;

    Ok(CertifiedKey::new(certs, signing_key))
}

/// Custom SNI resolver that maps hostnames to certificates with an optional default.
struct SniResolver {
    map: HashMap<String, Arc<CertifiedKey>>,
    default: Option<Arc<CertifiedKey>>,
}

impl std::fmt::Debug for SniResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SniResolver")
            .field("hostnames", &self.map.keys().collect::<Vec<_>>())
            .field("has_default", &self.default.is_some())
            .finish()
    }
}

impl ResolvesServerCert for SniResolver {
    fn resolve(
        &self,
        client_hello: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<CertifiedKey>> {
        if let Some(server_name) = client_hello.server_name() {
            let name = server_name.to_lowercase();
            if let Some(ck) = self.map.get(&name) {
                return Some(Arc::clone(ck));
            }
        }
        self.default.as_ref().map(Arc::clone)
    }
}

/// Build an SNI resolver from parsed entries and an optional default cert.
///
/// The default cert is used as a fallback when no SNI hostname matches (or no
/// SNI extension is present).  Pass `None` to reject unmatched connections.
pub fn build_sni_resolver(
    entries: &[SniEntry],
    default_cert: Option<(&Path, &Path)>,
) -> Result<Arc<dyn ResolvesServerCert>> {
    let mut map = HashMap::new();

    for entry in entries {
        let ck = load_certified_key(&entry.cert_path, &entry.key_path)
            .with_context(|| format!("Failed to load cert for SNI host {}", entry.hostname))?;
        map.insert(entry.hostname.clone(), Arc::new(ck));
    }

    let default = match default_cert {
        Some((cert, key)) => Some(Arc::new(
            load_certified_key(cert, key).context("Failed to load default TLS cert for SNI fallback")?,
        )),
        None => None,
    };

    Ok(Arc::new(SniResolver { map, default }))
}
