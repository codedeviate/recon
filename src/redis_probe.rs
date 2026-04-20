//! Redis probe. Connects via RESP2 and sends PING; optionally AUTH if
//! a password is present in the URL. Reports server hello (PONG or +OK
//! echo), connect latency, and PING round-trip.
//!
//! URL grammar: `redis://[[:PASSWORD]@]host[:port][/DB]`. Default port 6379.
//! Exit 0 on PONG, 7 on connect refused, 28 on timeout, 67 on AUTH failure.

use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const DEFAULT_PORT: u16 = 6379;

pub(crate) struct RedisUrl {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
}

pub(crate) fn parse_url(raw: &str) -> Result<RedisUrl> {
    let rest = raw
        .strip_prefix("redis://")
        .ok_or_else(|| anyhow!("redis: URL must start with redis://"))?;

    // userinfo@authority[/path]
    let (userinfo, after) = match rest.rfind('@') {
        Some(i) => (Some(&rest[..i]), &rest[i + 1..]),
        None => (None, rest),
    };

    let password = userinfo.and_then(|u| {
        // Either ":pass" (curl style) or "user:pass" — we use just the pass
        u.split_once(':').map(|(_, p)| p.to_string())
            .or_else(|| if u.is_empty() { None } else { Some(u.to_string()) })
    });

    let authority = match after.find('/') {
        Some(i) => &after[..i],
        None => after,
    };
    if authority.is_empty() {
        return Err(anyhow!("redis: URL missing host"));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|_| anyhow!("redis: invalid port '{p}'"))?,
        ),
        None => (authority.to_string(), DEFAULT_PORT),
    };

    Ok(RedisUrl { host, port, password })
}

pub fn run(url: &str, args: &crate::cli::Args) -> Result<()> {
    let timeout_secs = args.timeout;
    let command_args: Option<Vec<String>> = match &args.data {
        Some(d) => {
            let bytes = crate::client::load_body_from_string(d)?;
            let text = String::from_utf8(bytes)
                .map_err(|_| anyhow!("redis: -d payload must be valid UTF-8"))?;
            let toks = shell_split(&text)
                .ok_or_else(|| anyhow!("redis: unbalanced quotes in -d command"))?;
            if toks.is_empty() {
                return Err(anyhow!("redis: -d was empty"));
            }
            Some(toks)
        }
        None => None,
    };

    let parsed = parse_url(url)?;
    let addr = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()
        .with_context(|| format!("redis: could not resolve {}:{}", parsed.host, parsed.port))?
        .next()
        .ok_or_else(|| anyhow!("redis: no address for {}:{}", parsed.host, parsed.port))?;

    let timeout = Duration::from_secs(timeout_secs);
    let connect_start = Instant::now();
    let stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::TimedOut => {
            return Err(anyhow!("redis: connection to {} timed out", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::OperationTimedOut);
        }
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
            return Err(anyhow!("redis: connection refused by {}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
        Err(e) => {
            return Err(anyhow!("redis: connect to {} failed: {e}", parsed.host))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect);
        }
    };
    let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let peer = stream.peer_addr().ok();
    let mut reader = BufReader::new(stream.try_clone().context("redis: clone stream")?);
    let mut writer = stream;

    println!("Connected to {}:{} in {:.1}ms", parsed.host, parsed.port, connect_ms);
    if let Some(p) = peer {
        println!("  peer: {p}");
    }

    if let Some(pw) = &parsed.password {
        let cmd = resp_array(&["AUTH", pw]);
        writer.write_all(&cmd).context("redis: write AUTH")?;
        let reply = read_reply(&mut reader)?;
        if !reply.starts_with("+OK") {
            return Err(anyhow!("redis: AUTH rejected: {}", reply.trim_end()))
                .context(crate::mqtt::ProtocolExitCode::LoginDenied);
        }
        println!("AUTH: {}", reply.trim_end());
    }

    let (label, wire) = match &command_args {
        Some(toks) => {
            let refs: Vec<&str> = toks.iter().map(String::as_str).collect();
            (toks.join(" "), resp_array(&refs))
        }
        None => ("PING".to_string(), resp_array(&["PING"])),
    };

    let cmd_start = Instant::now();
    writer
        .write_all(&wire)
        .with_context(|| format!("redis: write {label}"))?;
    let reply = read_reply(&mut reader)?;
    let cmd_ms = cmd_start.elapsed().as_secs_f64() * 1000.0;
    println!("{label}: {} ({:.1}ms)", reply.trim_end(), cmd_ms);

    let _ = writer.write_all(&resp_array(&["QUIT"]));
    Ok(())
}

/// Simple shell-style splitter: splits on whitespace, supports
/// "double quoted" and 'single quoted' tokens, and backslash escapes
/// (`\"`, `\ `, `\\`). Returns None on unbalanced quotes.
fn shell_split(input: &str) -> Option<Vec<String>> {
    #[derive(PartialEq)]
    enum State {
        Normal,
        InDouble,
        InSingle,
    }
    let mut state = State::Normal;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match (&state, c) {
            (State::Normal, ch) if ch.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            (State::Normal, '"') => state = State::InDouble,
            (State::Normal, '\'') => state = State::InSingle,
            (State::Normal, '\\') => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            (State::InDouble, '"') => state = State::Normal,
            (State::InDouble, '\\') => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            (State::InSingle, '\'') => state = State::Normal,
            (_, ch) => cur.push(ch),
        }
    }
    if state != State::Normal {
        return None;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Some(out)
}

/// RESP2 array encoding: *N\r\n$len\r\nARG\r\n…
fn resp_array(args: &[&str]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", args.len()).into_bytes();
    for a in args {
        out.extend_from_slice(format!("${}\r\n", a.len()).as_bytes());
        out.extend_from_slice(a.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// Read one RESP reply. Handles +simple, -error, :integer, $bulk, *array
/// shallowly — enough for PING/AUTH responses.
fn read_reply<R: BufRead>(r: &mut R) -> Result<String> {
    let mut first = String::new();
    let n = r.read_line(&mut first).context("redis: read reply")?;
    if n == 0 {
        return Err(anyhow!("redis: server closed connection"));
    }
    match first.as_bytes().first() {
        Some(b'+') | Some(b'-') | Some(b':') => Ok(first),
        Some(b'$') => {
            // Bulk string: read length, then that many bytes + CRLF
            let len: i64 = first[1..]
                .trim_end_matches(['\r', '\n'])
                .parse()
                .map_err(|_| anyhow!("redis: bad bulk length"))?;
            if len < 0 {
                return Ok("$-1 (nil)\r\n".into());
            }
            let mut buf = vec![0u8; len as usize + 2];
            r.read_exact(&mut buf).context("redis: read bulk")?;
            let body = String::from_utf8_lossy(&buf[..len as usize]).into_owned();
            Ok(format!("${len} {body}\r\n"))
        }
        _ => Ok(first),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_url() {
        let u = parse_url("redis://localhost").unwrap();
        assert_eq!(u.host, "localhost");
        assert_eq!(u.port, 6379);
        assert_eq!(u.password, None);
    }

    #[test]
    fn parses_host_port() {
        let u = parse_url("redis://localhost:9999").unwrap();
        assert_eq!(u.port, 9999);
    }

    #[test]
    fn parses_password() {
        let u = parse_url("redis://:secret@localhost").unwrap();
        assert_eq!(u.password.as_deref(), Some("secret"));
    }

    #[test]
    fn parses_user_and_password() {
        let u = parse_url("redis://default:secret@localhost").unwrap();
        assert_eq!(u.password.as_deref(), Some("secret"));
    }

    #[test]
    fn parses_db_path_ignored() {
        let u = parse_url("redis://localhost/0").unwrap();
        assert_eq!(u.host, "localhost");
    }

    #[test]
    fn rejects_missing_host() {
        assert!(parse_url("redis:///").is_err());
    }

    #[test]
    fn resp_array_format() {
        assert_eq!(
            resp_array(&["PING"]),
            b"*1\r\n$4\r\nPING\r\n".to_vec()
        );
        assert_eq!(
            resp_array(&["AUTH", "x"]),
            b"*2\r\n$4\r\nAUTH\r\n$1\r\nx\r\n".to_vec()
        );
    }

    #[test]
    fn shell_split_simple() {
        assert_eq!(
            shell_split("SET key value"),
            Some(vec!["SET".into(), "key".into(), "value".into()])
        );
    }

    #[test]
    fn shell_split_double_quoted() {
        assert_eq!(
            shell_split("SET key \"hello world\""),
            Some(vec!["SET".into(), "key".into(), "hello world".into()])
        );
    }

    #[test]
    fn shell_split_single_quoted() {
        assert_eq!(
            shell_split("SET key 'a b c'"),
            Some(vec!["SET".into(), "key".into(), "a b c".into()])
        );
    }

    #[test]
    fn shell_split_backslash_escape() {
        assert_eq!(
            shell_split(r#"SET key value\ with\ spaces"#),
            Some(vec![
                "SET".into(),
                "key".into(),
                "value with spaces".into()
            ])
        );
    }

    #[test]
    fn shell_split_unbalanced() {
        assert_eq!(shell_split("SET key \"unterminated"), None);
    }

    #[test]
    fn shell_split_empty() {
        assert_eq!(shell_split(""), Some(vec![]));
        assert_eq!(shell_split("   "), Some(vec![]));
    }
}
