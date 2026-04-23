//! `imap://` and `imaps://` probe + FETCH via the `imap` crate.
//!
//! Path grammar (curl-compatible):
//!   imap://host/                       -> probe (CAPABILITY + LIST)
//!   imap://host/INBOX                  -> EXAMINE INBOX + report EXISTS/RECENT
//!   imap://host/INBOX;UID=N            -> FETCH UID N message body
//!
//! TLS: `imaps://` is implicit TLS (port 993); `imap://` is plaintext
//! (port 143). STARTTLS negotiation is handled transparently by the
//! crate when the server advertises it.

use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};

const DEFAULT_IMAP_PORT: u16 = 143;
const DEFAULT_IMAPS_PORT: u16 = 993;

pub struct ImapProbeOk {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub capabilities: Vec<String>,
    pub mailbox: Option<String>,
    pub exists: Option<u32>,
    pub recent: Option<u32>,
    pub mailboxes: Option<Vec<String>>,
    /// UID of the fetched message when in fetch mode; `None` otherwise.
    /// Consumers that surface the fetch target (scripts) read this.
    #[allow(dead_code)]
    pub uid: Option<u32>,
    pub body: Option<Vec<u8>>,
}

pub struct ImapArgs<'a> {
    pub user: Option<&'a str>,
    pub pass: Option<&'a str>,
    pub insecure: bool,
    pub peek: bool,
}

pub fn probe(url: &str, iargs: &ImapArgs<'_>) -> Result<ImapProbeOk> {
    let parsed = parse_url(url)?;
    let tls = parsed.tls;
    let port = parsed.port.unwrap_or(if tls {
        DEFAULT_IMAPS_PORT
    } else {
        DEFAULT_IMAP_PORT
    });
    let user = parsed.user.as_deref().or(iargs.user);
    let pass = parsed.pass.as_deref().or(iargs.pass);

    let mode = if tls {
        imap::ConnectionMode::Tls
    } else {
        imap::ConnectionMode::StartTls
    };
    let _ = iargs.insecure; // TLS-verify override not wired through imap 3.0 alpha
    let builder = imap::ClientBuilder::new(parsed.host.clone(), port).mode(mode);

    let mut client = builder.connect().map_err(|e| {
        anyhow!("imap: connect {}:{}: {e}", parsed.host, port)
            .context(ProtocolExitCode::CouldntConnect)
    })?;

    let caps = client
        .capabilities()
        .context("imap: CAPABILITY")?;
    let capabilities: Vec<String> = caps.iter().map(|c| format!("{c:?}")).collect();

    // No auth supplied: probe-only, report capabilities and quit.
    let Some(user) = user else {
        // Client::logout exists only on the authenticated Session; drop
        // the Client to close the connection.
        drop(client);
        return Ok(ImapProbeOk {
            host: parsed.host,
            port,
            tls,
            capabilities,
            mailbox: None,
            exists: None,
            recent: None,
            mailboxes: None,
            uid: None,
            body: None,
        });
    };
    let pass = pass.unwrap_or("");
    let mut session = client.login(user, pass).map_err(|(e, _)| {
        anyhow!("imap: LOGIN: {e}").context(ProtocolExitCode::LoginDenied)
    })?;

    // Path drives the action.
    match parsed.action {
        ImapAction::Probe => {
            let boxes = session
                .list(Some(""), Some("*"))
                .context("imap: LIST")?;
            let mailboxes: Vec<String> = boxes.iter().map(|m| m.name().to_string()).collect();
            let _ = session.logout();
            Ok(ImapProbeOk {
                host: parsed.host,
                port,
                tls,
                capabilities,
                mailbox: None,
                exists: None,
                recent: None,
                mailboxes: Some(mailboxes),
                uid: None,
                body: None,
            })
        }
        ImapAction::Examine(mbox) => {
            let m = session.examine(&mbox).context("imap: EXAMINE")?;
            let _ = session.logout();
            Ok(ImapProbeOk {
                host: parsed.host,
                port,
                tls,
                capabilities,
                mailbox: Some(mbox),
                exists: Some(m.exists),
                recent: Some(m.recent),
                mailboxes: None,
                uid: None,
                body: None,
            })
        }
        ImapAction::Fetch { mailbox, uid } => {
            session.examine(&mailbox).context("imap: EXAMINE")?;
            let fetch_spec = if iargs.peek { "BODY.PEEK[]" } else { "BODY[]" };
            let fetches = session
                .uid_fetch(uid.to_string(), fetch_spec)
                .context("imap: UID FETCH")?;
            let body = fetches
                .iter()
                .next()
                .and_then(|f| f.body().map(|b| b.to_vec()));
            let _ = session.logout();
            Ok(ImapProbeOk {
                host: parsed.host,
                port,
                tls,
                capabilities,
                mailbox: Some(mailbox),
                exists: None,
                recent: None,
                mailboxes: None,
                uid: Some(uid),
                body,
            })
        }
    }
}

pub fn run(url: &str, iargs: &ImapArgs<'_>) -> Result<()> {
    use std::io::Write;
    let r = probe(url, iargs)?;
    let label = if r.tls { " (TLS)" } else { "" };
    eprintln!("Connected to {}:{}{}", r.host, r.port, label);
    eprintln!("Capabilities: {}", r.capabilities.join(", "));
    if let Some(boxes) = &r.mailboxes {
        eprintln!("Mailboxes ({}):", boxes.len());
        for m in boxes {
            println!("{m}");
        }
    }
    if let Some(mb) = &r.mailbox {
        if let (Some(e), Some(rc)) = (r.exists, r.recent) {
            eprintln!("Mailbox: {mb}  exists={e}  recent={rc}");
        }
    }
    if let Some(body) = r.body {
        std::io::stdout().write_all(&body)?;
    }
    Ok(())
}

enum ImapAction {
    Probe,
    Examine(String),
    Fetch { mailbox: String, uid: u32 },
}

struct ParsedImap {
    host: String,
    port: Option<u16>,
    tls: bool,
    user: Option<String>,
    pass: Option<String>,
    action: ImapAction,
}

fn parse_url(url: &str) -> Result<ParsedImap> {
    let parsed = url::Url::parse(url).with_context(|| format!("imap: bad URL '{url}'"))?;
    let scheme = parsed.scheme();
    let tls = match scheme {
        "imap" => false,
        "imaps" => true,
        other => bail!("imap: unknown scheme '{other}:' (expected imap / imaps)"),
    };
    let host = parsed.host_str().ok_or_else(|| anyhow!("imap: host missing"))?.to_string();
    let port = parsed.port();
    let user = if parsed.username().is_empty() { None } else { Some(parsed.username().to_string()) };
    let pass = parsed.password().map(|s| s.to_string());

    let raw_path = parsed.path().trim_start_matches('/');
    let action = if raw_path.is_empty() {
        ImapAction::Probe
    } else if let Some((mbox, params)) = raw_path.split_once(';') {
        let uid = params
            .split(';')
            .find_map(|p| p.strip_prefix("UID="))
            .ok_or_else(|| anyhow!("imap: URL params must include UID=N"))?;
        let uid: u32 = uid
            .parse()
            .map_err(|e| anyhow!("imap: invalid UID '{uid}': {e}"))?;
        ImapAction::Fetch { mailbox: urlencoding_decode(mbox), uid }
    } else {
        ImapAction::Examine(urlencoding_decode(raw_path))
    };

    Ok(ParsedImap { host, port, tls, user, pass, action })
}

fn urlencoding_decode(s: &str) -> String {
    // Handles %20, %2F, etc.
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_root_probes() {
        let p = parse_url("imap://host/").unwrap();
        assert_eq!(p.host, "host");
        assert!(matches!(p.action, ImapAction::Probe));
    }

    #[test]
    fn parse_mailbox_examines() {
        let p = parse_url("imap://host/INBOX").unwrap();
        match p.action {
            ImapAction::Examine(m) => assert_eq!(m, "INBOX"),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_uid_fetches() {
        let p = parse_url("imap://host/INBOX;UID=42").unwrap();
        match p.action {
            ImapAction::Fetch { mailbox, uid } => {
                assert_eq!(mailbox, "INBOX");
                assert_eq!(uid, 42);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_imaps_is_tls() {
        let p = parse_url("imaps://user:pass@host/").unwrap();
        assert!(p.tls);
        assert_eq!(p.user.as_deref(), Some("user"));
        assert_eq!(p.pass.as_deref(), Some("pass"));
    }

    #[test]
    fn parse_rejects_non_imap() {
        assert!(parse_url("ftp://host/").is_err());
    }

    #[test]
    fn parse_uid_rejects_non_numeric() {
        assert!(parse_url("imap://host/INBOX;UID=abc").is_err());
    }
}
