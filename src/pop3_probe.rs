//! `pop3://` and `pop3s://` probe + RETR. Hand-rolled over TCP
//! (+ optional TLS). Text protocol mirroring SMTP's probe shape.
//!
//! Path grammar (curl-compatible):
//!   pop3://host/              -> probe (CAPA + STAT)
//!   pop3://user:pass@host/N   -> RETR message N (requires auth)
//!
//! `pop3s://` is implicit TLS (port 995). `pop3://` is plaintext
//! (default port 110); `--stls` upgrades via the STLS command after
//! CAPA (mirrors SMTP's STARTTLS).

use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_POP3_PORT: u16 = 110;
const DEFAULT_POP3S_PORT: u16 = 995;

/// (message_count, total_bytes, message_body) returned by auth+retrieve helpers.
type MailboxStats = (Option<u32>, Option<u64>, Option<Vec<u8>>);

/// Parsed components of a pop3[s]:// URL.
type ParsedUrl = (String, String, Option<u16>, Option<String>, Option<String>, String);

pub struct Pop3ProbeOk {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub banner: String,
    pub capabilities: Vec<String>,
    pub message_count: Option<u32>,
    pub total_bytes: Option<u64>,
    pub message: Option<Vec<u8>>,
    pub connect_ms: f64,
}

pub struct Pop3Args<'a> {
    pub user: Option<&'a str>,
    pub pass: Option<&'a str>,
    pub stls: bool,
    pub insecure: bool,
    pub timeout_secs: u64,
}

pub fn probe(url: &str, pargs: &Pop3Args<'_>) -> Result<Pop3ProbeOk> {
    let (scheme, host, port_opt, url_user, url_pass, path) = parse_url(url)?;
    let implicit_tls = scheme == "pop3s";
    let port = port_opt.unwrap_or(if implicit_tls {
        DEFAULT_POP3S_PORT
    } else {
        DEFAULT_POP3_PORT
    });
    let timeout = Duration::from_secs(pargs.timeout_secs.max(1));

    let t0 = Instant::now();
    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .with_context(|| format!("pop3: resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("pop3: no address for {host}:{port}"))?;
    let tcp = TcpStream::connect_timeout(&addr, timeout).map_err(|e| {
        anyhow!("pop3: connect {host}:{port}: {e}").context(ProtocolExitCode::CouldntConnect)
    })?;
    tcp.set_read_timeout(Some(timeout))?;
    tcp.set_write_timeout(Some(timeout))?;
    let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let user = url_user.as_deref().or(pargs.user);
    let pass = url_pass.as_deref().or(pargs.pass);

    let message_number: Option<u32> = if path.is_empty() {
        None
    } else {
        Some(
            path.parse()
                .map_err(|e| anyhow!("pop3: path must be a message number (got '{path}'): {e}"))?,
        )
    };

    if implicit_tls {
        let mut conn = tls_connect(tcp, &host, pargs.insecure)?;
        run_session_tls(&mut conn, host, port, true, user, pass, message_number, connect_ms, false)
    } else if pargs.stls {
        run_session_starttls(tcp, &host, port, user, pass, message_number, pargs.insecure, connect_ms)
    } else {
        run_session_plain(tcp, host, port, user, pass, message_number, connect_ms)
    }
}

fn run_session_plain(
    tcp: TcpStream,
    host: String,
    port: u16,
    user: Option<&str>,
    pass: Option<&str>,
    msg_n: Option<u32>,
    connect_ms: f64,
) -> Result<Pop3ProbeOk> {
    let mut reader = BufReader::new(tcp.try_clone()?);
    let mut writer = tcp;
    let banner = read_line_ok(&mut reader)?;
    let caps = fetch_capa(&mut writer, &mut reader)?;
    let (count, bytes, body) = maybe_auth_and_retrieve(&mut writer, &mut reader, user, pass, msg_n)?;
    let _ = writer.write_all(b"QUIT\r\n");
    let _ = read_line(&mut reader);
    Ok(Pop3ProbeOk {
        host,
        port,
        tls: false,
        banner,
        capabilities: caps,
        message_count: count,
        total_bytes: bytes,
        message: body,
        connect_ms,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_session_starttls(
    tcp: TcpStream,
    host: &str,
    port: u16,
    user: Option<&str>,
    pass: Option<&str>,
    msg_n: Option<u32>,
    insecure: bool,
    connect_ms: f64,
) -> Result<Pop3ProbeOk> {
    let mut reader = BufReader::new(tcp.try_clone()?);
    let mut writer = tcp.try_clone()?;
    let banner = read_line_ok(&mut reader)?;
    let caps = fetch_capa(&mut writer, &mut reader)?;
    writer.write_all(b"STLS\r\n").context("pop3: STLS")?;
    read_line_ok(&mut reader).context("pop3: STLS response")?;
    drop(reader);
    drop(writer);
    let mut conn = tls_connect(tcp, host, insecure)?;
    run_session_tls(
        &mut conn,
        host.to_string(),
        port,
        true,
        user,
        pass,
        msg_n,
        connect_ms,
        /* banner already read */ true,
    )
    .map(|mut r| {
        r.banner = banner.clone();
        if r.capabilities.is_empty() {
            r.capabilities = caps.clone();
        }
        r
    })
}

#[allow(clippy::too_many_arguments)]
fn run_session_tls<S: Read + Write>(
    conn: &mut S,
    host: String,
    port: u16,
    tls: bool,
    user: Option<&str>,
    pass: Option<&str>,
    msg_n: Option<u32>,
    connect_ms: f64,
    banner_already_read: bool,
) -> Result<Pop3ProbeOk> {
    // BufReader over `&mut S` — read_line-friendly. We build it fresh each
    // call; the borrow ends when we drop it.
    let mut buf = Vec::<u8>::with_capacity(1024);
    let banner = if banner_already_read {
        String::new()
    } else {
        read_line_from(conn, &mut buf)?
    };
    let caps = fetch_capa_rw(conn, &mut buf)?;
    let (count, bytes, body) =
        maybe_auth_and_retrieve_rw(conn, &mut buf, user, pass, msg_n)?;
    let _ = conn.write_all(b"QUIT\r\n");
    let _ = read_line_from(conn, &mut buf);
    Ok(Pop3ProbeOk {
        host,
        port,
        tls,
        banner,
        capabilities: caps,
        message_count: count,
        total_bytes: bytes,
        message: body,
        connect_ms,
    })
}

fn tls_connect(
    tcp: TcpStream,
    host: &str,
    insecure: bool,
) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>> {
    use rustls::pki_types::ServerName;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let cfg = if insecure {
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("pop3s TLS: protocol versions")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(danger::NoopVerifier))
            .with_no_client_auth()
    } else {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("pop3s TLS: protocol versions")?
            .with_root_certificates(roots)
            .with_no_client_auth()
    };
    let sn: ServerName<'static> = ServerName::try_from(host.to_string())
        .map_err(|e| anyhow!("pop3: invalid hostname '{host}': {e}"))?;
    let conn = rustls::ClientConnection::new(Arc::new(cfg), sn)
        .map_err(|e| anyhow!("pop3s: TLS init: {e}"))?;
    Ok(rustls::StreamOwned::new(conn, tcp))
}

pub fn run(url: &str, pargs: &Pop3Args<'_>) -> Result<()> {
    let r = probe(url, pargs)?;
    let label = if r.tls { " (TLS)" } else { "" };
    eprintln!(
        "Connected to {}:{}{} in {:.1}ms",
        r.host, r.port, label, r.connect_ms
    );
    if !r.banner.is_empty() {
        eprintln!("{}", r.banner.trim());
    }
    eprintln!("Capabilities:");
    for c in &r.capabilities {
        eprintln!("  {c}");
    }
    if let (Some(c), Some(b)) = (r.message_count, r.total_bytes) {
        eprintln!("Mailbox: {c} messages, {b} bytes");
    }
    if let Some(msg) = r.message {
        std::io::stdout().write_all(&msg)?;
    }
    Ok(())
}

// ── line-level I/O helpers ──────────────────────────────────────────────────

fn read_line<R: BufRead>(r: &mut R) -> Result<String> {
    let mut s = String::new();
    let n = r.read_line(&mut s).context("pop3: read line")?;
    if n == 0 {
        bail!("pop3: server closed connection");
    }
    Ok(s)
}

fn read_line_ok<R: BufRead>(r: &mut R) -> Result<String> {
    let s = read_line(r)?;
    if !s.starts_with("+OK") {
        bail!("pop3: expected +OK, got {:?}", s.trim());
    }
    Ok(s)
}

fn read_line_from<S: Read>(s: &mut S, scratch: &mut Vec<u8>) -> Result<String> {
    scratch.clear();
    let mut byte = [0u8; 1];
    loop {
        let n = s.read(&mut byte).context("pop3: read")?;
        if n == 0 {
            if scratch.is_empty() {
                bail!("pop3: server closed connection");
            }
            break;
        }
        scratch.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    Ok(String::from_utf8_lossy(scratch).into_owned())
}

fn fetch_capa<W: Write, R: BufRead>(w: &mut W, r: &mut R) -> Result<Vec<String>> {
    w.write_all(b"CAPA\r\n").context("pop3: CAPA write")?;
    let first = read_line(r)?;
    if !first.starts_with("+OK") {
        // Server may not support CAPA — still OK.
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    loop {
        let line = read_line(r)?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line == "." {
            break;
        }
        out.push(line.to_string());
    }
    Ok(out)
}

fn fetch_capa_rw<S: Read + Write>(s: &mut S, scratch: &mut Vec<u8>) -> Result<Vec<String>> {
    s.write_all(b"CAPA\r\n").context("pop3: CAPA write")?;
    let first = read_line_from(s, scratch)?;
    if !first.starts_with("+OK") {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    loop {
        let line = read_line_from(s, scratch)?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line == "." {
            break;
        }
        out.push(line.to_string());
    }
    Ok(out)
}

fn maybe_auth_and_retrieve<W: Write, R: BufRead>(
    w: &mut W,
    r: &mut R,
    user: Option<&str>,
    pass: Option<&str>,
    msg_n: Option<u32>,
) -> Result<MailboxStats> {
    let Some(u) = user else {
        return Ok((None, None, None));
    };
    let p = pass.unwrap_or("");
    writeln!(w, "USER {u}\r")?;
    read_line_ok(r).context("pop3: USER response")?;
    writeln!(w, "PASS {p}\r")?;
    read_line_ok(r).map_err(|e| {
        anyhow!("pop3: AUTH failed: {e}").context(ProtocolExitCode::LoginDenied)
    })?;

    w.write_all(b"STAT\r\n")?;
    let stat = read_line_ok(r)?;
    let (count, bytes) = parse_stat(&stat);

    let body = if let Some(n) = msg_n {
        writeln!(w, "RETR {n}\r")?;
        Some(read_dot_terminated(r)?)
    } else {
        None
    };
    Ok((count, bytes, body))
}

fn maybe_auth_and_retrieve_rw<S: Read + Write>(
    s: &mut S,
    scratch: &mut Vec<u8>,
    user: Option<&str>,
    pass: Option<&str>,
    msg_n: Option<u32>,
) -> Result<MailboxStats> {
    let Some(u) = user else {
        return Ok((None, None, None));
    };
    let p = pass.unwrap_or("");
    s.write_all(format!("USER {u}\r\n").as_bytes())?;
    let line = read_line_from(s, scratch)?;
    if !line.starts_with("+OK") {
        bail!("pop3: USER rejected: {}", line.trim());
    }
    s.write_all(format!("PASS {p}\r\n").as_bytes())?;
    let line = read_line_from(s, scratch)?;
    if !line.starts_with("+OK") {
        bail!(anyhow!("pop3: AUTH failed: {}", line.trim())
            .context(ProtocolExitCode::LoginDenied)
            .to_string());
    }
    s.write_all(b"STAT\r\n")?;
    let stat = read_line_from(s, scratch)?;
    let (count, bytes) = parse_stat(&stat);

    let body = if let Some(n) = msg_n {
        s.write_all(format!("RETR {n}\r\n").as_bytes())?;
        let mut buf = Vec::<u8>::new();
        // Read dot-terminated response as bytes.
        loop {
            let line = read_line_from(s, scratch)?;
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if buf.is_empty() {
                if !trimmed.starts_with("+OK") {
                    bail!("pop3: RETR rejected: {}", trimmed);
                }
                continue;
            }
            // This shouldn't happen — we reset buf at the start — but keep
            // for clarity: read until dot.
            if trimmed == "." {
                break;
            }
            let mut stripped = trimmed;
            if stripped.starts_with("..") {
                stripped = &stripped[1..];
            }
            buf.extend_from_slice(stripped.as_bytes());
            buf.push(b'\n');
        }
        Some(buf)
    } else {
        None
    };
    Ok((count, bytes, body))
}

fn parse_stat(s: &str) -> (Option<u32>, Option<u64>) {
    let parts: Vec<&str> = s.split_whitespace().collect();
    // "+OK N BYTES"
    let count = parts.get(1).and_then(|s| s.parse().ok());
    let bytes = parts.get(2).and_then(|s| s.parse().ok());
    (count, bytes)
}

fn read_dot_terminated<R: BufRead>(r: &mut R) -> Result<Vec<u8>> {
    let first = read_line(r)?;
    if !first.starts_with("+OK") {
        bail!("pop3: RETR rejected: {}", first.trim());
    }
    let mut out = Vec::new();
    loop {
        let line = read_line(r)?;
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "." {
            break;
        }
        let stripped = if trimmed.starts_with("..") {
            &trimmed[1..]
        } else {
            trimmed
        };
        out.extend_from_slice(stripped.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

fn parse_url(url: &str) -> Result<ParsedUrl> {
    let parsed = url::Url::parse(url).with_context(|| format!("pop3: bad URL '{url}'"))?;
    let scheme = parsed.scheme().to_string();
    if scheme != "pop3" && scheme != "pop3s" {
        bail!("pop3: unknown scheme '{scheme}:' (expected pop3 / pop3s)");
    }
    let host = parsed.host_str().ok_or_else(|| anyhow!("pop3: host missing"))?.to_string();
    let user = (!parsed.username().is_empty()).then(|| parsed.username().to_string());
    let pass = parsed.password().map(|s| s.to_string());
    let path = parsed.path().trim_start_matches('/').to_string();
    Ok((scheme, host, parsed.port(), user, pass, path))
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
    fn parse_basic_pop3() {
        let (s, h, p, u, pw, path) = parse_url("pop3://host/").unwrap();
        assert_eq!(s, "pop3");
        assert_eq!(h, "host");
        assert_eq!(p, None);
        assert_eq!(u, None);
        assert_eq!(pw, None);
        assert_eq!(path, "");
    }

    #[test]
    fn parse_pop3s_with_auth() {
        let (s, _, p, u, pw, path) = parse_url("pop3s://alice:secret@mail.example.com/3").unwrap();
        assert_eq!(s, "pop3s");
        assert_eq!(p, None);
        assert_eq!(u.as_deref(), Some("alice"));
        assert_eq!(pw.as_deref(), Some("secret"));
        assert_eq!(path, "3");
    }

    #[test]
    fn parse_rejects_non_pop3() {
        assert!(parse_url("ftp://host/").is_err());
    }

    #[test]
    fn parse_stat_extracts_count_bytes() {
        let (c, b) = parse_stat("+OK 5 2048\r\n");
        assert_eq!(c, Some(5));
        assert_eq!(b, Some(2048));
    }
}
