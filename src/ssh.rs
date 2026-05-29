use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use crossterm::event::Event;

use crate::termkey::key_event_to_bytes;
use ssh2::Session;

use crate::cli::Args;
use crate::ssh_auth;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn connect(raw_url: &str, args: &Args) -> Result<()> {
    let (user_from_url, host, port) = parse_ssh_url(raw_url)?;
    let (user, password) = ssh_auth::resolve_credentials(&user_from_url, args);

    eprintln!("Connecting to {}@{}:{} …", user, host, port);

    let tcp = TcpStream::connect(format!("{}:{}", host, port))
        .with_context(|| format!("Could not connect to {}:{}", host, port))?;

    let mut sess = Session::new().context("Failed to create SSH session")?;
    sess.set_tcp_stream(tcp);
    if args.compressed_ssh {
        sess.set_compress(true);
    }
    sess.handshake()
        .with_context(|| format!("SSH handshake failed with {}", host))?;
    sess.set_timeout(args.timeout.saturating_mul(1000).min(u64::from(u32::MAX)) as u32);

    ssh_auth::verify_host_key_with_pins(
        &sess,
        &host,
        port,
        args.insecure,
        args.hostpubsha256.as_deref(),
        args.hostpubmd5.as_deref(),
    )?;
    ssh_auth::authenticate(&sess, &user, args, password.as_deref())?;

    // Open a channel and request a PTY + shell
    let mut channel = sess.channel_session().context("Failed to open SSH channel")?;
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    channel
        .request_pty("xterm-256color", None, Some((cols as u32, rows as u32, 0, 0)))
        .context("Failed to request PTY")?;
    channel.shell().context("Failed to open shell")?;

    // Switch to non-blocking so we can interleave reads and writes in one thread
    sess.set_blocking(false);

    // Enable raw terminal mode — RAII guard restores it even on panic
    let _raw = RawModeGuard::enable()?;

    let mut stdout = io::stdout();

    loop {
        // ── Drain channel stdout (non-blocking) ───────────────────────────────
        let mut buf = [0u8; 4096];
        loop {
            match channel.read(&mut buf) {
                Ok(0) => break, // ssh2 returns Ok(0) for EOF in non-blocking mode (WouldBlock = no data)
                Ok(n) => {
                    stdout.write_all(&buf[..n])?;
                    stdout.flush()?;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }

        // Also drain stderr
        {
            let mut stderr_stream = channel.stderr();
            loop {
                match stderr_stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = io::stderr().write_all(&buf[..n]);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(_) => break, // stderr errors are non-fatal; best-effort only
                }
            }
        }

        // Remote shell exited
        if channel.eof() {
            break;
        }

        // ── Poll for terminal input / resize (10 ms timeout) ──────────────────
        if crossterm::event::poll(Duration::from_millis(10))? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    let bytes = key_event_to_bytes(&key);
                    if !bytes.is_empty() {
                        sess.set_blocking(true);
                        let write_result = channel.write_all(&bytes).and_then(|_| channel.flush());
                        sess.set_blocking(false);
                        write_result?;
                    }
                }
                Event::Resize(cols, rows) => {
                    sess.set_blocking(true);
                    let _ = channel.request_pty_size(cols as u32, rows as u32, None, None);
                    sess.set_blocking(false);
                }
                _ => {}
            }
        }
    }

    // Graceful close
    sess.set_blocking(true);
    let _ = channel.send_eof();
    let _ = channel.wait_eof();
    let _ = channel.close();
    let _ = channel.wait_close();

    Ok(())
}

// ── URL parsing ───────────────────────────────────────────────────────────────

fn parse_ssh_url(raw: &str) -> Result<(String, String, u16)> {
    let parsed = url::Url::parse(raw)
        .with_context(|| format!("Invalid SSH URL: {raw}"))?;
    let host = parsed
        .host_str()
        .filter(|h| !h.is_empty())
        .ok_or_else(|| anyhow!("SSH URL missing host: {raw}"))?
        .to_string();
    let port = parsed.port().unwrap_or(22);
    let user = parsed.username().to_string();
    Ok((user, host, port))
}

// ── Key event → bytes ─────────────────────────────────────────────────────────

// ── Raw mode RAII guard ───────────────────────────────────────────────────────

struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().context("Failed to enable raw terminal mode")?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssh_default_port() {
        let (user, host, port) = parse_ssh_url("ssh://server.local").unwrap();
        assert_eq!(host, "server.local");
        assert_eq!(port, 22);
        assert_eq!(user, "");
    }

    #[test]
    fn parse_ssh_with_user() {
        let (user, host, port) = parse_ssh_url("ssh://alice@server.local").unwrap();
        assert_eq!(user, "alice");
        assert_eq!(host, "server.local");
        assert_eq!(port, 22);
    }

    #[test]
    fn parse_ssh_custom_port() {
        let (user, host, port) = parse_ssh_url("ssh://alice@server.local:2222").unwrap();
        assert_eq!(user, "alice");
        assert_eq!(host, "server.local");
        assert_eq!(port, 2222);
    }

    #[test]
    fn parse_ssh_missing_host_errors() {
        assert!(parse_ssh_url("ssh://").is_err());
    }

}
