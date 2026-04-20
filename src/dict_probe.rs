//! DICT (RFC 2229) client supporting curl's URL grammar.
//!
//! Supported URL forms:
//! - `dict://host[:port]/d:word[:db[:strat]]` — DEFINE
//! - `dict://host[:port]/m:word[:db[:strat]]` — MATCH
//! - `dict://host[:port]/show:server|databases|strategies|info:db`
//!
//! Default port 2628. Request command → read multi-line text response
//! until status 250 "ok" or a 5xx error. Connect errors map to exit 7,
//! timeouts to 28.

use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const DEFAULT_PORT: u16 = 2628;

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Define { word: String, db: String },
    Match { word: String, db: String, strat: String },
    ShowServer,
    ShowDatabases,
    ShowStrategies,
    ShowInfo { db: String },
}

pub(crate) struct DictUrl {
    pub host: String,
    pub port: u16,
    pub command: Command,
}

pub(crate) fn parse_url(raw: &str) -> Result<DictUrl> {
    let rest = raw
        .strip_prefix("dict://")
        .ok_or_else(|| anyhow!("dict: URL must start with dict://"))?;

    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(anyhow!("dict: URL missing host"));
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|_| anyhow!("dict: invalid port '{p}'"))?,
        ),
        None => (authority.to_string(), DEFAULT_PORT),
    };

    if path.is_empty() {
        return Err(anyhow!(
            "dict: URL needs a command path: /d:WORD, /m:WORD, or /show:…"
        ));
    }

    let command = parse_command(path)?;
    Ok(DictUrl { host, port, command })
}

fn parse_command(path: &str) -> Result<Command> {
    let parts: Vec<&str> = path.split(':').collect();
    match parts.as_slice() {
        ["d", word] => Ok(Command::Define {
            word: pct_decode(word),
            db: "*".into(),
        }),
        ["d", word, db] => Ok(Command::Define {
            word: pct_decode(word),
            db: pct_decode(db),
        }),
        ["d", word, db, _strat] => Ok(Command::Define {
            word: pct_decode(word),
            db: pct_decode(db),
        }),
        ["m", word] => Ok(Command::Match {
            word: pct_decode(word),
            db: "*".into(),
            strat: ".".into(),
        }),
        ["m", word, db] => Ok(Command::Match {
            word: pct_decode(word),
            db: pct_decode(db),
            strat: ".".into(),
        }),
        ["m", word, db, strat] => Ok(Command::Match {
            word: pct_decode(word),
            db: pct_decode(db),
            strat: pct_decode(strat),
        }),
        ["show", "server"] => Ok(Command::ShowServer),
        ["show", "databases"] | ["show", "db"] => Ok(Command::ShowDatabases),
        ["show", "strategies"] | ["show", "strat"] => Ok(Command::ShowStrategies),
        ["show", "info", db] => Ok(Command::ShowInfo { db: pct_decode(db) }),
        _ => Err(anyhow!(
            "dict: unrecognized command path '{path}' (expected /d:WORD, /m:WORD, or /show:…)"
        )),
    }
}

fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn wire_command(c: &Command) -> String {
    match c {
        Command::Define { word, db } => format!("DEFINE {db} \"{word}\"\r\n"),
        Command::Match { word, db, strat } => {
            format!("MATCH {db} {strat} \"{word}\"\r\n")
        }
        Command::ShowServer => "SHOW SERVER\r\n".into(),
        Command::ShowDatabases => "SHOW DB\r\n".into(),
        Command::ShowStrategies => "SHOW STRAT\r\n".into(),
        Command::ShowInfo { db } => format!("SHOW INFO {db}\r\n"),
    }
}

pub fn run(url: &str, timeout_secs: u64) -> Result<()> {
    let parsed = parse_url(url)?;
    let addr = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()
        .with_context(|| format!("dict: could not resolve {}:{}", parsed.host, parsed.port))?
        .next()
        .ok_or_else(|| anyhow!("dict: no address for {}:{}", parsed.host, parsed.port))?;

    let timeout = Duration::from_secs(timeout_secs);
    let stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::TimedOut => {
            return Err(anyhow!("dict: connection to {} timed out", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::OperationTimedOut);
        }
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
            return Err(anyhow!("dict: connection refused by {}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
        Err(e) => {
            return Err(anyhow!("dict: connect to {} failed: {e}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
    };

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let mut reader = BufReader::new(stream.try_clone().context("dict: clone stream")?);
    let mut writer = stream;

    // Server banner (220 <text>)
    let banner = read_status_line(&mut reader)?;
    println!("{banner}");

    // Send CLIENT announcement (RFC 2229 recommended)
    writer
        .write_all(b"CLIENT recon\r\n")
        .context("dict: write CLIENT")?;
    let _ = read_status_line(&mut reader)?; // 250 ok

    // Send real command
    let cmd_line = wire_command(&parsed.command);
    writer
        .write_all(cmd_line.as_bytes())
        .context("dict: write command")?;

    // Read response until 250 terminator or 5xx
    loop {
        let line = read_line(&mut reader)?;
        if line.is_empty() {
            return Err(anyhow!("dict: server closed connection mid-response"));
        }
        print!("{line}");
        let code = status_code(&line);
        match code {
            Some(c) if c == 250 => break,
            Some(c) if (500..600).contains(&c) => break,
            _ => continue,
        }
    }

    // Polite QUIT
    let _ = writer.write_all(b"QUIT\r\n");
    Ok(())
}

fn read_status_line<R: BufRead>(r: &mut R) -> Result<String> {
    let line = read_line(r)?;
    if line.is_empty() {
        return Err(anyhow!("dict: server closed connection"));
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

fn read_line<R: BufRead>(r: &mut R) -> Result<String> {
    let mut s = String::new();
    let n = r.read_line(&mut s).context("dict: read")?;
    if n == 0 {
        return Ok(String::new());
    }
    Ok(s)
}

fn status_code(line: &str) -> Option<u16> {
    let code = line.get(0..3)?;
    if code.chars().all(|c| c.is_ascii_digit())
        && matches!(line.as_bytes().get(3), Some(b' ') | Some(b'-') | None | Some(b'\r'))
    {
        code.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_define_default() {
        let u = parse_url("dict://dict.org/d:recon").unwrap();
        assert_eq!(u.host, "dict.org");
        assert_eq!(u.port, 2628);
        assert_eq!(
            u.command,
            Command::Define {
                word: "recon".into(),
                db: "*".into()
            }
        );
    }

    #[test]
    fn parses_define_with_db() {
        let u = parse_url("dict://dict.org/d:recon:wn").unwrap();
        assert_eq!(
            u.command,
            Command::Define {
                word: "recon".into(),
                db: "wn".into()
            }
        );
    }

    #[test]
    fn parses_match() {
        let u = parse_url("dict://dict.org/m:recon").unwrap();
        assert_eq!(
            u.command,
            Command::Match {
                word: "recon".into(),
                db: "*".into(),
                strat: ".".into()
            }
        );
    }

    #[test]
    fn parses_show_variants() {
        assert_eq!(
            parse_url("dict://h/show:server").unwrap().command,
            Command::ShowServer
        );
        assert_eq!(
            parse_url("dict://h/show:databases").unwrap().command,
            Command::ShowDatabases
        );
        assert_eq!(
            parse_url("dict://h/show:strategies").unwrap().command,
            Command::ShowStrategies
        );
        assert_eq!(
            parse_url("dict://h/show:info:wn").unwrap().command,
            Command::ShowInfo { db: "wn".into() }
        );
    }

    #[test]
    fn custom_port() {
        let u = parse_url("dict://dict.org:9999/d:word").unwrap();
        assert_eq!(u.port, 9999);
    }

    #[test]
    fn percent_decodes_word() {
        let u = parse_url("dict://h/d:hello%20world").unwrap();
        assert_eq!(
            u.command,
            Command::Define {
                word: "hello world".into(),
                db: "*".into()
            }
        );
    }

    #[test]
    fn rejects_missing_path() {
        assert!(parse_url("dict://dict.org").is_err());
        assert!(parse_url("dict://dict.org/").is_err());
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse_url("dict://h/foo:bar").is_err());
    }

    #[test]
    fn wire_define_quoted() {
        let c = Command::Define {
            word: "test".into(),
            db: "wn".into(),
        };
        assert_eq!(wire_command(&c), "DEFINE wn \"test\"\r\n");
    }

    #[test]
    fn status_code_parses_space_or_dash() {
        assert_eq!(status_code("220 hello"), Some(220));
        assert_eq!(status_code("151-definition"), Some(151));
        assert_eq!(status_code("250"), Some(250));
        assert_eq!(status_code("abc hi"), None);
    }
}
