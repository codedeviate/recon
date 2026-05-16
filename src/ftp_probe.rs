//! `ftp://` and `ftps://` probe + retrieve via `suppaftp`.
//!
//! Path semantics match curl:
//!   ftp://host/           -> list root directory
//!   ftp://host/dir/       -> list that directory
//!   ftp://host/dir/file   -> retrieve that file
//!
//! Auth: URL userinfo -> `-u user:pass` -> anonymous (`anonymous` /
//! `anonymous@recon`).
//!
//! `ftps://` uses AUTH TLS (explicit FTPS). Implicit-TLS FTPS is
//! currently not supported; `--ftps-implicit` is accepted but warned
//! about. Revisit if someone asks for a server that requires it.

use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use suppaftp::{FtpStream, Mode, RustlsConnector, RustlsFtpStream, Status};

const DEFAULT_PORT: u16 = 21;

/// All 2xx FTP response codes accepted as success by `-Q / --quote`.
/// suppaftp's `custom_command` requires an explicit list of expected codes;
/// we accept every standard 2xx code so ad-hoc commands (PWD, FEAT, NOOP …)
/// don't need the caller to predict the exact reply code.
const QUOTE_ACCEPT: &[Status] = &[
    Status::CommandOk,           // 200
    Status::CommandNotImplemented, // 202 (NOOP on some servers)
    Status::System,              // 211
    Status::Directory,           // 212
    Status::File,                // 213
    Status::Help,                // 214
    Status::Name,                // 215
    Status::Ready,               // 220
    Status::Closing,             // 221
    Status::DataConnectionOpen,  // 225
    Status::ClosingDataConnection, // 226
    Status::PassiveMode,         // 227
    Status::LongPassiveMode,     // 228
    Status::ExtendedPassiveMode, // 229
    Status::LoggedIn,            // 230
    Status::LoggedOut,           // 231
    Status::LogoutAck,           // 232
    Status::AuthOk,              // 234
    Status::RequestedFileActionOk, // 250
    Status::PathCreated,         // 257
];

pub enum FtpMode {
    List(Vec<String>),
    Retrieve(Vec<u8>),
}

pub struct FtpProbeOk {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub user: String,
    pub connect_ms: f64,
    pub welcome: Option<String>,
    pub pwd: Option<String>,
    pub mode: FtpMode,
}

pub struct FtpArgs<'a> {
    pub user: Option<&'a str>,
    pub pass: Option<&'a str>,
    pub passive: bool,
    pub implicit_tls: bool,
    pub insecure: bool,
    pub timeout_secs: u64,
    pub list_only: bool,
    pub quote: Vec<String>,
    pub ftp_skip_pasv_ip: bool,
    pub disable_epsv: bool,
    pub disable_eprt: bool,
    pub ftp_pasv: bool,
    pub verbose: u8,
}

pub fn probe(url: &str, fargs: &FtpArgs<'_>) -> Result<FtpProbeOk> {
    let (scheme, host, port, url_user, url_pass, path) = parse_url(url)?;
    let use_tls = matches!(scheme.as_str(), "ftps");
    let port = port.unwrap_or(DEFAULT_PORT);
    if use_tls && fargs.implicit_tls {
        eprintln!("! ftp: --ftps-implicit not yet implemented; falling back to explicit AUTH TLS");
    }

    let (user, pass) = resolve_creds(&url_user, &url_pass, fargs.user, fargs.pass);

    let t0 = Instant::now();
    let timeout = Duration::from_secs(fargs.timeout_secs.max(1));
    let addr = std::net::ToSocketAddrs::to_socket_addrs(&format!("{host}:{port}"))
        .with_context(|| format!("ftp: resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("ftp: no address for {host}:{port}"))?;
    let tcp = std::net::TcpStream::connect_timeout(&addr, timeout).map_err(|e| {
        anyhow!("ftp: connect {host}:{port}: {e}").context(ProtocolExitCode::CouldntConnect)
    })?;
    tcp.set_read_timeout(Some(timeout))?;
    tcp.set_write_timeout(Some(timeout))?;
    let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;

    if use_tls {
        let mut plain = RustlsFtpStream::connect_with_stream(tcp)
            .map_err(|e| anyhow!("ftp: init: {e}"))?;
        plain.set_mode(if fargs.passive { Mode::Passive } else { Mode::Active });
        let welcome = plain.get_welcome_msg().map(|s| s.to_string());
        let connector = build_rustls_connector(fargs.insecure)?;
        let mut stream = plain
            .into_secure(connector, &host)
            .map_err(|e| anyhow!("ftps: AUTH TLS upgrade: {e}"))?;
        stream.login(&user, &pass).map_err(map_ftp_err)?;
        // --ftp-skip-pasv-ip
        if fargs.ftp_skip_pasv_ip {
            stream.set_passive_nat_workaround(true);
        }
        // --disable-epsv / --disable-eprt / --ftp-pasv (passive-mode confirmation)
        if (fargs.disable_epsv || fargs.disable_eprt || fargs.ftp_pasv) && fargs.verbose >= 1 {
            eprintln!("* FTP: passive mode (suppaftp 6 default; --ftp-pasv / --disable-eprt confirmed)");
        }
        // -Q / --quote
        for cmd in &fargs.quote {
            stream.custom_command(cmd, QUOTE_ACCEPT).with_context(|| format!("FTP --quote: {cmd} failed"))?;
        }
        let pwd = stream.pwd().ok();
        let mode = do_path_op_tls(&mut stream, &path, fargs.list_only)?;
        let _ = stream.quit();
        Ok(FtpProbeOk {
            host, port, tls: true, user, connect_ms, welcome, pwd, mode,
        })
    } else {
        let mut plain = FtpStream::connect_with_stream(tcp)
            .map_err(|e| anyhow!("ftp: init: {e}"))?;
        plain.set_mode(if fargs.passive { Mode::Passive } else { Mode::Active });
        let welcome = plain.get_welcome_msg().map(|s| s.to_string());
        plain.login(&user, &pass).map_err(map_ftp_err)?;
        // --ftp-skip-pasv-ip
        if fargs.ftp_skip_pasv_ip {
            plain.set_passive_nat_workaround(true);
        }
        // --disable-epsv / --disable-eprt / --ftp-pasv (passive-mode confirmation)
        if (fargs.disable_epsv || fargs.disable_eprt || fargs.ftp_pasv) && fargs.verbose >= 1 {
            eprintln!("* FTP: passive mode (suppaftp 6 default; --ftp-pasv / --disable-eprt confirmed)");
        }
        // -Q / --quote
        for cmd in &fargs.quote {
            plain.custom_command(cmd, QUOTE_ACCEPT).with_context(|| format!("FTP --quote: {cmd} failed"))?;
        }
        let pwd = plain.pwd().ok();
        let mode = do_path_op_plain(&mut plain, &path, fargs.list_only)?;
        let _ = plain.quit();
        Ok(FtpProbeOk {
            host, port, tls: false, user, connect_ms, welcome, pwd, mode,
        })
    }
}

fn do_path_op_plain(stream: &mut FtpStream, path: &str, list_only: bool) -> Result<FtpMode> {
    if path.is_empty() || path.ends_with('/') {
        let dir = path.trim_end_matches('/');
        if !dir.is_empty() {
            stream.cwd(dir).map_err(map_ftp_err)?;
        }
        let entries = if list_only {
            stream.nlst(None).map_err(map_ftp_err)?
        } else {
            stream.list(None).map_err(map_ftp_err)?
        };
        Ok(FtpMode::List(entries))
    } else {
        let buf = stream.retr_as_buffer(path).map_err(map_ftp_err)?;
        Ok(FtpMode::Retrieve(buf.into_inner()))
    }
}

fn do_path_op_tls(stream: &mut RustlsFtpStream, path: &str, list_only: bool) -> Result<FtpMode> {
    if path.is_empty() || path.ends_with('/') {
        let dir = path.trim_end_matches('/');
        if !dir.is_empty() {
            stream.cwd(dir).map_err(map_ftp_err)?;
        }
        let entries = if list_only {
            stream.nlst(None).map_err(map_ftp_err)?
        } else {
            stream.list(None).map_err(map_ftp_err)?
        };
        Ok(FtpMode::List(entries))
    } else {
        let buf = stream.retr_as_buffer(path).map_err(map_ftp_err)?;
        Ok(FtpMode::Retrieve(buf.into_inner()))
    }
}

fn map_ftp_err(e: suppaftp::FtpError) -> anyhow::Error {
    let msg = e.to_string();
    let tag = if msg.contains("530")
        || msg.to_ascii_lowercase().contains("login")
        || msg.to_ascii_lowercase().contains("auth")
    {
        ProtocolExitCode::LoginDenied
    } else {
        ProtocolExitCode::CouldntConnect
    };
    anyhow!("ftp: {msg}").context(tag)
}

fn build_rustls_connector(insecure: bool) -> Result<RustlsConnector> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let cfg = if insecure {
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("ftps TLS: protocol versions")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(danger::NoopVerifier))
            .with_no_client_auth()
    } else {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("ftps TLS: protocol versions")?
            .with_root_certificates(roots)
            .with_no_client_auth()
    };
    Ok(RustlsConnector::from(Arc::new(cfg)))
}

pub fn run(url: &str, fargs: &FtpArgs<'_>, output: Option<&std::path::Path>) -> Result<()> {
    let r = probe(url, fargs)?;
    let label = if r.tls { " (TLS)" } else { "" };
    eprintln!(
        "Connected to {}:{}{} as {} in {:.1}ms",
        r.host, r.port, label, r.user, r.connect_ms
    );
    if let Some(w) = &r.welcome {
        eprintln!("Welcome: {}", w.lines().next().unwrap_or(""));
    }
    if let Some(pwd) = &r.pwd {
        eprintln!("PWD: {pwd}");
    }
    match r.mode {
        FtpMode::List(entries) => {
            for e in entries {
                println!("{e}");
            }
        }
        FtpMode::Retrieve(bytes) => {
            if let Some(path) = output {
                std::fs::write(path, &bytes)
                    .with_context(|| format!("ftp: write {}", path.display()))?;
                eprintln!("Saved to {}", path.display());
            } else {
                std::io::stdout().write_all(&bytes)?;
            }
        }
    }
    Ok(())
}

fn parse_url(url: &str) -> Result<(String, String, Option<u16>, String, String, String)> {
    let parsed = url::Url::parse(url).with_context(|| format!("ftp: bad URL '{url}'"))?;
    let scheme = parsed.scheme().to_string();
    if scheme != "ftp" && scheme != "ftps" {
        bail!("ftp: unknown scheme '{scheme}:' (expected ftp or ftps)");
    }
    let host = parsed.host_str().ok_or_else(|| anyhow!("ftp: host missing"))?.to_string();
    let port = parsed.port();
    let user = percent_decode(parsed.username());
    let pass = percent_decode(parsed.password().unwrap_or(""));
    let raw_path = parsed.path();
    let path = if raw_path == "/" {
        String::new()
    } else {
        raw_path.trim_start_matches('/').to_string()
    };
    Ok((scheme, host, port, user, pass, path))
}

fn percent_decode(s: &str) -> String {
    // url::Url decodes host/path but not userinfo; do a simple decode.
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(n) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                out.push(n as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn resolve_creds(
    url_user: &str,
    url_pass: &str,
    flag_user: Option<&str>,
    flag_pass: Option<&str>,
) -> (String, String) {
    if !url_user.is_empty() {
        return (url_user.to_string(), url_pass.to_string());
    }
    if let Some(u) = flag_user {
        return (u.to_string(), flag_pass.unwrap_or("").to_string());
    }
    ("anonymous".to_string(), "anonymous@recon".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_ftp() {
        let (s, h, p, u, pw, path) = parse_url("ftp://host/dir/file").unwrap();
        assert_eq!(s, "ftp");
        assert_eq!(h, "host");
        assert_eq!(p, None);
        assert_eq!(u, "");
        assert_eq!(pw, "");
        assert_eq!(path, "dir/file");
    }

    #[test]
    fn parse_ftps_with_auth() {
        let (s, h, _, u, pw, _) = parse_url("ftps://alice:secret@host:990/").unwrap();
        assert_eq!(s, "ftps");
        assert_eq!(h, "host");
        assert_eq!(u, "alice");
        assert_eq!(pw, "secret");
    }

    #[test]
    fn parse_dir_keeps_trailing_slash() {
        let (_, _, _, _, _, path) = parse_url("ftp://host/dir/").unwrap();
        assert_eq!(path, "dir/");
    }

    #[test]
    fn parse_root_is_empty() {
        let (_, _, _, _, _, path) = parse_url("ftp://host/").unwrap();
        assert_eq!(path, "");
    }

    #[test]
    fn resolve_creds_defaults_to_anonymous() {
        let (u, p) = resolve_creds("", "", None, None);
        assert_eq!(u, "anonymous");
        assert_eq!(p, "anonymous@recon");
    }

    #[test]
    fn resolve_creds_url_wins() {
        let (u, _) = resolve_creds("alice", "secret", Some("bob"), Some("x"));
        assert_eq!(u, "alice");
    }

    #[test]
    fn percent_decode_ampersand() {
        assert_eq!(percent_decode("a%26b"), "a&b");
        assert_eq!(percent_decode("plain"), "plain");
    }
}
