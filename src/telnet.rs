use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::cli::Args;
use crate::ssh::key_event_to_bytes;

// Telnet protocol constants (RFC 854)
const IAC: u8 = 0xFF;
const WILL: u8 = 0xFB;
const WONT: u8 = 0xFC;
const DO: u8 = 0xFD;
const DONT: u8 = 0xFE;
const SB: u8 = 0xFA;
const SE: u8 = 0xF0;
const OPT_ECHO: u8 = 1;
const OPT_SGA: u8 = 3;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn connect(raw_url: &str, args: &Args) -> Result<()> {
    let (host, port) = parse_telnet_url(raw_url)?;

    eprintln!("Connecting to {}:{} …", host, port);

    let mut read_stream = TcpStream::connect(format!("{}:{}", host, port))
        .with_context(|| format!("Could not connect to {}:{}", host, port))?;
    read_stream.set_nonblocking(true)?;
    let write_stream = read_stream.try_clone().context("Failed to clone TcpStream")?;
    let mut write_stream = io::BufWriter::new(write_stream);

    // Enable raw terminal mode — RAII guard restores it even on panic
    let _raw = RawModeGuard::enable()?;

    let mut stdout = io::stdout();
    let mut read_buf = [0u8; 4096];

    loop {
        // ── Drain incoming server data (non-blocking) ─────────────────────────
        let mut reply_buf = Vec::new();
        let mut display_buf = Vec::new();

        loop {
            match read_stream.read(&mut read_buf) {
                Ok(0) => return Ok(()), // server closed connection
                Ok(n) => {
                    process_bytes(&read_buf[..n], &mut display_buf, &mut reply_buf);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }

        if !display_buf.is_empty() {
            stdout.write_all(&display_buf)?;
            stdout.flush()?;
        }
        if !reply_buf.is_empty() {
            write_stream.write_all(&reply_buf)?;
            write_stream.flush()?;
        }

        // ── Poll for keyboard input (10 ms timeout) ───────────────────────────
        if crossterm::event::poll(Duration::from_millis(10))? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    // Ctrl+D closes the connection
                    if key.code == KeyCode::Char('d')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        return Ok(());
                    }
                    let bytes = key_event_to_bytes(&key);
                    let escaped = escape_iac(&bytes);
                    write_stream.write_all(&escaped)?;
                    write_stream.flush()?;
                }
                _ => {}
            }
        }
    }
}

// ── URL parsing ───────────────────────────────────────────────────────────────

fn parse_telnet_url(raw: &str) -> Result<(String, u16)> {
    let parsed = url::Url::parse(raw)
        .with_context(|| format!("Invalid Telnet URL: {raw}"))?;
    let host = parsed
        .host_str()
        .filter(|h| !h.is_empty())
        .ok_or_else(|| anyhow!("Telnet URL missing host: {raw}"))?
        .to_string();
    let port = parsed.port().unwrap_or(23);
    Ok((host, port))
}

// ── IAC processing ────────────────────────────────────────────────────────────

/// Process a chunk of incoming Telnet bytes.
/// - `out`: bytes to display (non-IAC data)
/// - `replies`: IAC responses to send back to the server
pub(crate) fn process_bytes(input: &[u8], out: &mut Vec<u8>, replies: &mut Vec<u8>) {
    #[derive(PartialEq)]
    enum State {
        Normal,
        Iac,
        IacVerb(u8),
        Sb,
        SbIac,
    }

    let mut state = State::Normal;

    for &byte in input {
        match state {
            State::Normal => {
                if byte == IAC {
                    state = State::Iac;
                } else {
                    out.push(byte);
                }
            }
            State::Iac => match byte {
                IAC => {
                    out.push(0xFF); // IAC IAC = literal 0xFF
                    state = State::Normal;
                }
                WILL | WONT | DO | DONT => {
                    state = State::IacVerb(byte);
                }
                SB => {
                    state = State::Sb;
                }
                _ => {
                    state = State::Normal; // Other IAC commands (NOP, GA, etc.) — ignore
                }
            },
            State::IacVerb(verb) => {
                match (verb, byte) {
                    (WILL, OPT_ECHO) | (WILL, OPT_SGA) => {
                        replies.extend_from_slice(&[IAC, DO, byte]);
                    }
                    (WILL, opt) => {
                        replies.extend_from_slice(&[IAC, DONT, opt]);
                    }
                    (DO, opt) => {
                        replies.extend_from_slice(&[IAC, WONT, opt]);
                    }
                    _ => {} // WONT/DONT from server — no reply needed
                }
                state = State::Normal;
            }
            State::Sb => {
                if byte == IAC {
                    state = State::SbIac;
                }
                // else: consume subnegotiation data
            }
            State::SbIac => {
                if byte == SE {
                    state = State::Normal;
                } else {
                    state = State::Sb;
                }
            }
        }
    }
}

/// Escape any 0xFF bytes in `data` as IAC IAC (two 0xFF bytes) per RFC 854.
pub(crate) fn escape_iac(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for &b in data {
        if b == IAC {
            out.push(IAC);
        }
        out.push(b);
    }
    out
}

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
    fn parse_telnet_default_port() {
        let (host, port) = parse_telnet_url("telnet://bbs.example.com").unwrap();
        assert_eq!(host, "bbs.example.com");
        assert_eq!(port, 23);
    }

    #[test]
    fn parse_telnet_custom_port() {
        let (host, port) = parse_telnet_url("telnet://host:2323").unwrap();
        assert_eq!(host, "host");
        assert_eq!(port, 2323);
    }

    #[test]
    fn parse_telnet_missing_host_errors() {
        assert!(parse_telnet_url("telnet://").is_err());
    }

    #[test]
    fn iac_strips_will_unknown_and_replies_dont() {
        // Server: IAC WILL 5 → we send IAC DONT 5, display nothing
        let input = vec![0xFF, 0xFB, 5u8, b'h', b'i'];
        let mut out = Vec::new();
        let mut replies = Vec::new();
        process_bytes(&input, &mut out, &mut replies);
        assert_eq!(out, b"hi");
        assert_eq!(replies, vec![0xFF, 0xFE, 5u8]);
    }

    #[test]
    fn iac_accepts_will_echo() {
        let input = vec![0xFF, 0xFB, 1u8];
        let mut out = Vec::new();
        let mut replies = Vec::new();
        process_bytes(&input, &mut out, &mut replies);
        assert_eq!(out, b"");
        assert_eq!(replies, vec![0xFF, 0xFD, 1u8]);
    }

    #[test]
    fn iac_accepts_will_sga() {
        let input = vec![0xFF, 0xFB, 3u8];
        let mut out = Vec::new();
        let mut replies = Vec::new();
        process_bytes(&input, &mut out, &mut replies);
        assert_eq!(replies, vec![0xFF, 0xFD, 3u8]);
    }

    #[test]
    fn iac_rejects_do_with_wont() {
        let input = vec![0xFF, 0xFD, 31u8];
        let mut out = Vec::new();
        let mut replies = Vec::new();
        process_bytes(&input, &mut out, &mut replies);
        assert_eq!(replies, vec![0xFF, 0xFC, 31u8]);
    }

    #[test]
    fn iac_discards_subnegotiation() {
        let input = vec![0xFF, 0xFA, 1u8, 0u8, 0xFF, 0xF0, b'o', b'k'];
        let mut out = Vec::new();
        let mut replies = Vec::new();
        process_bytes(&input, &mut out, &mut replies);
        assert_eq!(out, b"ok");
        assert!(replies.is_empty());
    }

    #[test]
    fn iac_iac_outputs_single_0xff() {
        let input = vec![0xFF, 0xFF, b'x'];
        let mut out = Vec::new();
        let mut replies = Vec::new();
        process_bytes(&input, &mut out, &mut replies);
        assert_eq!(out, vec![0xFF, b'x']);
    }

    #[test]
    fn escape_iac_in_input() {
        assert_eq!(escape_iac(&[b'a', 0xFF, b'b']), vec![b'a', 0xFF, 0xFF, b'b']);
    }

    #[test]
    fn escape_iac_no_special() {
        assert_eq!(escape_iac(&[b'h', b'i']), vec![b'h', b'i']);
    }
}
