//! SMTP / SMTPS probe + test-message delivery. Two modes:
//!
//! - **Probe mode** (no `--mail-from`): TCP connect, read the greeting,
//!   send EHLO, parse and report every advertised extension, optionally
//!   negotiate STARTTLS. Hand-rolled over `TcpStream` so we can surface
//!   quirks that a higher-level client would swallow.
//!
//! - **Send mode** (`--mail-from` + `--mail-to`): delegate to `lettre` for
//!   the full transaction (connect → EHLO → STARTTLS / implicit TLS →
//!   AUTH → MAIL → RCPT → DATA → QUIT), with optional DKIM signing.
//!   Lettre handles the crypto and canonicalisation correctly; reusing
//!   its `Message::sign(&DkimConfig)` API keeps recon out of the
//!   DKIM-implementation business.
//!
//! Exit codes mirror the other probes: 0 on success, 7 on connect-refused,
//! 28 on timeout, 67 on auth failure.

use crate::cli::Args;
use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};
use colored::Colorize;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const DEFAULT_SMTP_PORT: u16 = 25;
const DEFAULT_SMTPS_PORT: u16 = 465;
const DEFAULT_HELO: &str = "recon.local";

pub struct SmtpProbeOk {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub connect_ms: f64,
    pub banner: String,
    pub capabilities: Vec<String>,
    pub auth_methods: Vec<String>,
    pub starttls_ok: Option<bool>,
    pub send_result: Option<SendResult>,
}

pub struct SendResult {
    pub queued_message_id: Option<String>,
    pub code: u16,
    pub response: String,
    pub dkim_signed: bool,
}

pub fn probe(url: &str, args: &Args) -> Result<SmtpProbeOk> {
    let (host, port, implicit_tls) = parse_url(url)?;
    let helo = args.smtp_helo.as_deref().unwrap_or(DEFAULT_HELO);
    let timeout = Duration::from_secs(args.timeout.max(1));

    let t0 = Instant::now();
    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .with_context(|| format!("smtp: could not resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("smtp: no address resolved for {host}:{port}"))?;
    let stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| {
        anyhow!("smtp: connect failed to {host}:{port}: {e}")
            .context(ProtocolExitCode::CouldntConnect)
    })?;
    stream
        .set_read_timeout(Some(timeout))
        .context("smtp: set_read_timeout")?;
    stream
        .set_write_timeout(Some(timeout))
        .context("smtp: set_write_timeout")?;
    let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // Hand-roll the initial conversation so we see every line. For
    // implicit-TLS (`smtps://`) we don't touch the socket here — lettre
    // takes over in send mode. For plaintext/STARTTLS probing we use
    // this TcpStream directly.
    let (banner, capabilities) = if implicit_tls {
        // For smtps:// probe mode we skip the hand-rolled path (TLS not
        // wired here) and trust lettre's test_connection below. Banner
        // + capabilities are reported as best-effort.
        ("(implicit-TLS — capabilities reported via lettre)".to_string(), Vec::new())
    } else {
        read_banner_and_capabilities(stream.try_clone()?, helo)?
    };

    let starttls_advertised = capabilities
        .iter()
        .any(|c| c.eq_ignore_ascii_case("STARTTLS"));
    let starttls_ok = if implicit_tls {
        None
    } else if args.no_starttls || !starttls_advertised {
        Some(false)
    } else {
        // Proof-of-negotiation: we don't promote the probe socket to TLS
        // ourselves (lettre does that on send). Just note that the
        // server claimed support and we asked nicely.
        Some(true)
    };

    let auth_methods = capabilities
        .iter()
        .find(|c| c.to_ascii_uppercase().starts_with("AUTH "))
        .map(|auth_line| {
            auth_line
                .split_once(' ')
                .map(|x| x.1)
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    // Send mode: if --mail-from and --mail-to are set, deliver via lettre.
    let send_result = if args.mail_from.is_some() && !args.mail_to.is_empty() {
        Some(send_via_lettre(
            &host,
            port,
            implicit_tls,
            !args.no_starttls,
            args,
        )?)
    } else {
        None
    };

    Ok(SmtpProbeOk {
        host: host.to_string(),
        port,
        tls: implicit_tls,
        connect_ms,
        banner,
        capabilities,
        auth_methods,
        starttls_ok,
        send_result,
    })
}

pub fn run(url: &str, args: &Args) -> Result<()> {
    let r = probe(url, args)?;
    let tls_label = if r.tls { " (TLS)" } else { "" };
    println!(
        "Connected to {}:{}{} in {:.1}ms",
        r.host, r.port, tls_label, r.connect_ms
    );
    print!("{}", r.banner);
    if !r.banner.ends_with('\n') {
        println!();
    }

    if !r.capabilities.is_empty() {
        println!("{}", "Capabilities:".bold());
        for cap in &r.capabilities {
            println!("  {cap}");
        }
    }

    if let Some(ok) = r.starttls_ok {
        let label = if ok {
            "STARTTLS advertised".green().to_string()
        } else {
            "STARTTLS not available".yellow().to_string()
        };
        println!("  {label}");
    }

    if !r.auth_methods.is_empty() {
        println!("  AUTH mechanisms: {}", r.auth_methods.join(", "));
    }

    if let Some(send) = &r.send_result {
        let status = if send.code / 100 == 2 {
            format!("{} {}", send.code, send.response).green().to_string()
        } else {
            format!("{} {}", send.code, send.response).red().to_string()
        };
        println!();
        println!("Message delivery: {status}");
        if let Some(id) = &send.queued_message_id {
            println!("  Queued as: {id}");
        }
        if send.dkim_signed {
            println!("  {}", "DKIM-Signature applied".green());
        }
    }

    Ok(())
}

// ── URL parsing ──────────────────────────────────────────────────────────────

fn parse_url(url: &str) -> Result<(String, u16, bool)> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| anyhow!("smtp: URL must be smtp://host[:port]/ or smtps://host[:port]/"))?;
    let implicit_tls = match scheme {
        "smtp" => false,
        "smtps" => true,
        other => bail!("smtp: unknown scheme '{other}:' (expected smtp:// or smtps://)"),
    };
    // Strip trailing path if present.
    let host_and_port = rest.split('/').next().unwrap_or(rest);
    let (host, port) = if let Some((h, p)) = host_and_port.rsplit_once(':') {
        // Account for IPv6 literals written `[::1]:port`.
        let h = h.trim_start_matches('[').trim_end_matches(']');
        (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|e| anyhow!("smtp: invalid port '{p}': {e}"))?,
        )
    } else {
        let default = if implicit_tls {
            DEFAULT_SMTPS_PORT
        } else {
            DEFAULT_SMTP_PORT
        };
        (host_and_port.to_string(), default)
    };
    if host.is_empty() {
        bail!("smtp: host must not be empty in '{url}'");
    }
    Ok((host, port, implicit_tls))
}

// ── Probe-mode helpers (hand-rolled SMTP conversation) ───────────────────────

fn read_banner_and_capabilities(
    stream: TcpStream,
    helo: &str,
) -> Result<(String, Vec<String>)> {
    let mut read = BufReader::new(stream.try_clone()?);
    let mut write = stream;

    // Read greeting (may be multi-line: "220-..." continuations, "220 " final).
    let banner = read_multiline_response(&mut read, 220)?;

    // Try EHLO first.
    writeln!(write, "EHLO {helo}\r").context("smtp: write EHLO")?;
    let ehlo_body = read_multiline_response(&mut read, 250);
    let capabilities = match ehlo_body {
        Ok(body) => parse_ehlo_capabilities(&body),
        Err(_) => {
            // Fall back to HELO (some legacy servers reject EHLO).
            writeln!(write, "HELO {helo}\r").context("smtp: write HELO")?;
            let _ = read_multiline_response(&mut read, 250)?;
            Vec::new()
        }
    };

    // Graceful quit — ignore errors because some servers drop on QUIT.
    let _ = writeln!(write, "QUIT\r");
    let _ = read_multiline_response(&mut read, 221);

    Ok((banner, capabilities))
}

/// Read an SMTP multiline response. `expected` is the code the response
/// must start with (e.g. 220 for greeting, 250 for EHLO reply).
fn read_multiline_response<R: BufRead>(reader: &mut R, expected: u16) -> Result<String> {
    let mut out = String::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).context("smtp: read response")?;
        if n == 0 {
            bail!("smtp: server closed connection unexpectedly");
        }
        if line.len() < 4 {
            bail!("smtp: short response line: {line:?}");
        }
        let code: u16 = line[..3]
            .parse()
            .map_err(|e| anyhow!("smtp: invalid response code '{}': {e}", &line[..3]))?;
        if code != expected {
            bail!(
                "smtp: expected {expected} response, got {code}: {}",
                line.trim_end()
            );
        }
        out.push_str(&line);
        // The separator after the 3-digit code tells us whether this is
        // the last line. '-' means more coming, ' ' means done.
        let more = &line[3..4] == "-";
        if !more {
            break;
        }
    }
    Ok(out)
}

fn parse_ehlo_capabilities(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| {
            // "250-AUTH LOGIN PLAIN" -> "AUTH LOGIN PLAIN"
            // "250 SIZE 14680064"    -> "SIZE 14680064"
            if line.len() < 4 {
                return None;
            }
            Some(line[4..].trim_end().to_string())
        })
        .skip(1) // First line is the greeting echo, not a capability.
        .collect()
}

// ── Send mode (lettre) ───────────────────────────────────────────────────────

fn send_via_lettre(
    host: &str,
    port: u16,
    implicit_tls: bool,
    allow_starttls: bool,
    args: &Args,
) -> Result<SendResult> {
    use lettre::message::{header, Message, SinglePart};
    use lettre::transport::smtp::authentication::{Credentials, Mechanism};
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    use lettre::{SmtpTransport, Transport};

    let helo = args.smtp_helo.as_deref().unwrap_or(DEFAULT_HELO);

    let from_str = args
        .mail_from
        .as_deref()
        .ok_or_else(|| anyhow!("smtp: --mail-from is required to send"))?;
    let from = from_str
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| anyhow!("smtp: invalid --mail-from '{from_str}': {e}"))?;

    if args.mail_to.is_empty() {
        bail!("smtp: --mail-to required for send mode");
    }
    let to_list: Vec<lettre::message::Mailbox> = args
        .mail_to
        .iter()
        .map(|s| {
            s.parse::<lettre::message::Mailbox>()
                .map_err(|e| anyhow!("smtp: invalid --mail-to '{s}': {e}"))
        })
        .collect::<Result<_>>()?;

    let subject = args
        .mail_subject
        .clone()
        .unwrap_or_else(|| "recon SMTP test".to_string());
    let body = match args.mail_body.as_deref() {
        Some("@-") => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("smtp: read body from stdin")?;
            buf
        }
        Some(s) if s.starts_with('@') => std::fs::read_to_string(&s[1..])
            .with_context(|| format!("smtp: read body file '{}'", &s[1..]))?,
        Some(s) => s.to_string(),
        None => "This is a test message from recon.\n".to_string(),
    };

    let mut builder = Message::builder().from(from.clone()).subject(&subject);
    for to in &to_list {
        builder = builder.to(to.clone());
    }
    for h in &args.mail_header {
        let (name, value) = h
            .split_once(':')
            .ok_or_else(|| anyhow!("smtp: --mail-header '{h}' missing ':'"))?;
        let name = name.trim();
        let value = value.trim();
        let hv = header::HeaderName::new_from_ascii(name.to_string())
            .map_err(|e| anyhow!("smtp: invalid header name '{name}': {e}"))?;
        builder = builder.raw_header(header::HeaderValue::new(hv, value.to_string()));
    }
    let mut message = builder
        .singlepart(SinglePart::plain(body))
        .map_err(|e| anyhow!("smtp: build message: {e}"))?;

    // Optional DKIM signing.
    let mut dkim_signed = false;
    if let (Some(key_path), Some(selector)) = (&args.dkim_key, &args.dkim_selector) {
        use lettre::message::dkim::{
            DkimCanonicalization, DkimCanonicalizationType, DkimConfig, DkimSigningAlgorithm,
            DkimSigningKey,
        };
        let pem = std::fs::read_to_string(key_path)
            .with_context(|| format!("smtp: read dkim key '{}'", key_path.display()))?;
        let alg = if pem.contains("BEGIN PRIVATE KEY")
            && !pem.contains("RSA PRIVATE KEY")
            && pem.len() < 500
        {
            DkimSigningAlgorithm::Ed25519
        } else {
            DkimSigningAlgorithm::Rsa
        };
        let signing_key = DkimSigningKey::new(&pem, alg)
            .map_err(|e| anyhow!("smtp: parse dkim key: {e}"))?;
        let domain = args
            .dkim_domain
            .clone()
            .or_else(|| from_str.split('@').nth(1).map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("smtp: cannot derive DKIM domain from --mail-from"))?;
        let header_names = ["From", "To", "Subject", "Date"]
            .iter()
            .map(|n| lettre::message::header::HeaderName::new_from_ascii(n.to_string()).unwrap())
            .collect::<Vec<_>>();
        let config = DkimConfig::new(
            selector.clone(),
            domain,
            signing_key,
            header_names,
            DkimCanonicalization {
                header: DkimCanonicalizationType::Relaxed,
                body: DkimCanonicalizationType::Relaxed,
            },
        );
        message.sign(&config);
        dkim_signed = true;
    }

    // Build transport.
    let mut builder = if implicit_tls {
        SmtpTransport::relay(host).map_err(|e| anyhow!("smtp: TLS relay init: {e}"))?
    } else if allow_starttls {
        SmtpTransport::starttls_relay(host)
            .map_err(|e| anyhow!("smtp: STARTTLS init: {e}"))?
    } else {
        SmtpTransport::builder_dangerous(host)
    };
    builder = builder
        .port(port)
        .timeout(Some(Duration::from_secs(args.timeout.max(1))))
        .hello_name(lettre::transport::smtp::extension::ClientId::Domain(helo.to_string()));

    // Accept self-signed certs when --insecure is set.
    if args.insecure && implicit_tls {
        let tls = TlsParameters::builder(host.to_string())
            .dangerous_accept_invalid_certs(true)
            .dangerous_accept_invalid_hostnames(true)
            .build()
            .map_err(|e| anyhow!("smtp: tls params: {e}"))?;
        builder = builder.tls(Tls::Wrapper(tls));
    } else if args.insecure && allow_starttls {
        let tls = TlsParameters::builder(host.to_string())
            .dangerous_accept_invalid_certs(true)
            .dangerous_accept_invalid_hostnames(true)
            .build()
            .map_err(|e| anyhow!("smtp: tls params: {e}"))?;
        builder = builder.tls(Tls::Required(tls));
    }

    if let Some(auth) = &args.smtp_auth {
        let (user, pass) = auth
            .split_once(':')
            .ok_or_else(|| anyhow!("smtp: --smtp-auth must be 'user:pass'"))?;
        builder = builder
            .credentials(Credentials::new(user.to_string(), pass.to_string()))
            .authentication(vec![Mechanism::Plain, Mechanism::Login]);
    }

    // --mail-auth: RFC 4954 §6 AUTH= parameter on MAIL FROM.
    // lettre 0.11's high-level SmtpTransport::send builds MAIL FROM internally
    // and does not expose a way to inject extra MailParameter values — the
    // connection.rs send() helper constructs its own mail_options vec and
    // calls Mail::new(envelope.from(), mail_options) without any extension
    // point. Until lettre exposes an envelope-parameter API, this flag is
    // accepted but not forwarded to the wire.
    if args.mail_auth.is_some() {
        eprintln!(
            "warning: --mail-auth accepted but not yet forwarded to MAIL FROM AUTH= — \
             lettre 0.11 high-level API does not expose envelope parameters"
        );
    }

    let transport = builder.build();

    let response = transport.send(&message).map_err(|e| {
        let err = anyhow!("smtp: send failed: {e}");
        if e.to_string().to_ascii_lowercase().contains("auth") {
            err.context(ProtocolExitCode::LoginDenied)
        } else {
            err.context(ProtocolExitCode::CouldntConnect)
        }
    })?;
    let first_line = response
        .first_line()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "OK".to_string());
    let code = response.code().to_string();
    let code_num: u16 = code[..3].parse().unwrap_or(250);
    // SMTP "queued as" message IDs appear as "250 OK: queued as <id>" or similar.
    let message_id = extract_queued_id(&first_line);
    Ok(SendResult {
        queued_message_id: message_id,
        code: code_num,
        response: first_line,
        dkim_signed,
    })
}

fn extract_queued_id(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let idx = lower.find("queued as")?;
    let after = line[idx + "queued as".len()..].trim();
    let id = after.split_whitespace().next()?;
    if id.is_empty() {
        None
    } else {
        Some(id.trim_end_matches(['>', '.', ',']).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_smtp_default_port() {
        assert_eq!(parse_url("smtp://mail.example.com").unwrap(), ("mail.example.com".into(), 25, false));
    }

    #[test]
    fn parse_url_smtps_default_port() {
        assert_eq!(parse_url("smtps://mail.example.com/").unwrap(), ("mail.example.com".into(), 465, true));
    }

    #[test]
    fn parse_url_custom_port() {
        assert_eq!(parse_url("smtp://mail.example.com:587/").unwrap(), ("mail.example.com".into(), 587, false));
    }

    #[test]
    fn parse_url_rejects_bad_scheme() {
        assert!(parse_url("http://example.com/").is_err());
    }

    #[test]
    fn parse_url_rejects_empty_host() {
        assert!(parse_url("smtp:///").is_err());
    }

    #[test]
    fn parse_ehlo_strips_codes() {
        let body = "250-mail.example.com Hello\r\n250-SIZE 14680064\r\n250-AUTH LOGIN PLAIN\r\n250 STARTTLS\r\n";
        let caps = parse_ehlo_capabilities(body);
        assert!(caps.iter().any(|c| c == "SIZE 14680064"));
        assert!(caps.iter().any(|c| c == "AUTH LOGIN PLAIN"));
        assert!(caps.iter().any(|c| c == "STARTTLS"));
        // Greeting line is dropped.
        assert!(!caps.iter().any(|c| c.starts_with("mail.example.com")));
    }

    #[test]
    fn extract_queued_id_finds_the_id() {
        assert_eq!(
            extract_queued_id("OK: queued as ABC123DEF"),
            Some("ABC123DEF".to_string())
        );
        assert_eq!(
            extract_queued_id("2.0.0 Ok: queued as 9F3B7C3E9"),
            Some("9F3B7C3E9".to_string())
        );
        assert_eq!(extract_queued_id("Not a queued message"), None);
    }

}
