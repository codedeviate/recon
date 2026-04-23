//! HTTP/1.1 over Unix-domain sockets. Hand-rolled because reqwest's
//! blocking client has no UDS support, and pulling in a full
//! async-inside-sync stack (hyper + tokio + custom connector) just for
//! one-shot diagnostic requests to Docker / systemd / kubelet is
//! overkill.
//!
//! Scope: GET / POST / PUT / DELETE / HEAD / OPTIONS / PATCH, with
//! user headers + body. No HTTP/2 (UDS peers don't need it), no
//! streaming upload, no redirects (UDS endpoints don't redirect), no
//! TLS (doesn't make sense over a local socket). The request goes over
//! the socket, the response comes back as `#{url, status, headers,
//! body, body_bytes, http_version, duration_ms}` — same shape as
//! `http()` so callers can't tell the difference.

use crate::cli::Args;
use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

pub struct UdsResponse {
    /// Preserved so downstream renderers (prettify, write-out, editor)
    /// can echo the request target back. The CLI path doesn't use this
    /// field directly — the original URL is already in args.
    #[allow(dead_code)]
    pub url: String,
    pub final_url: String,
    pub status: u16,
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
    pub http_version: String,
    pub duration_ms: u64,
}

pub fn execute(args: &Args) -> Result<UdsResponse> {
    let socket_path = args
        .unix_socket
        .as_ref()
        .ok_or_else(|| anyhow!("unix-socket: --unix-socket path is required"))?;
    if !socket_path.exists() {
        bail!(
            "unix-socket: '{}' does not exist",
            socket_path.display()
        );
    }

    let url = args.target_url();
    let (host, path) = parse_url(url)?;

    let method = resolve_method(args);
    let body = resolve_body(args)?;
    let timeout = Duration::from_secs(args.timeout.max(1));

    let t0 = Instant::now();
    let mut stream = UnixStream::connect(socket_path).map_err(|e| {
        anyhow!(
            "unix-socket: connect to {}: {e}",
            socket_path.display()
        )
        .context(ProtocolExitCode::CouldntConnect)
    })?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    // Build the request line + headers.
    let mut header_lines: Vec<(String, String)> = vec![
        ("Host".to_string(), host.clone()),
        (
            "User-Agent".to_string(),
            args.user_agent
                .clone()
                .unwrap_or_else(|| concat!("recon/", env!("CARGO_PKG_VERSION")).to_string()),
        ),
        ("Accept".to_string(), "*/*".to_string()),
        ("Connection".to_string(), "close".to_string()),
    ];
    for h in &args.header {
        if let Some((k, v)) = h.split_once(':') {
            // Replace defaults when the user explicitly set the header.
            let name = k.trim().to_string();
            let val = v.trim().to_string();
            header_lines.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
            header_lines.push((name, val));
        }
    }
    if !body.is_empty() {
        let has_cl = header_lines
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("Content-Length"));
        if !has_cl {
            header_lines.push(("Content-Length".to_string(), body.len().to_string()));
        }
    }

    let mut request = String::new();
    request.push_str(&format!("{method} {path} HTTP/1.1\r\n"));
    for (k, v) in &header_lines {
        request.push_str(&format!("{k}: {v}\r\n"));
    }
    request.push_str("\r\n");

    stream.write_all(request.as_bytes()).context("unix-socket: write request")?;
    if !body.is_empty() {
        stream.write_all(&body).context("unix-socket: write body")?;
    }
    stream.flush().ok();

    let mut reader = BufReader::new(stream);

    // Status line.
    let mut status_line = String::new();
    let n = reader
        .read_line(&mut status_line)
        .context("unix-socket: read status")?;
    if n == 0 {
        bail!("unix-socket: server closed connection without reply");
    }
    let (http_version, status) = parse_status_line(&status_line)?;

    // Headers until blank line.
    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).context("unix-socket: read header")?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }

    // Body: respect Content-Length when present; otherwise read to EOF.
    let content_length: Option<usize> = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Content-Length"))
        .and_then(|(_, v)| v.parse().ok());

    let body = if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).context("unix-socket: read body")?;
        buf
    } else {
        // Simplification: no chunked decoding. Read to EOF (works when
        // Connection: close is honored).
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).context("unix-socket: read body to EOF")?;
        buf
    };

    Ok(UdsResponse {
        url: url.to_string(),
        final_url: url.to_string(),
        status,
        body,
        headers,
        http_version,
        duration_ms: t0.elapsed().as_millis() as u64,
    })
}

pub fn run(args: &Args) -> Result<()> {
    let r = execute(args)?;
    // Write the body to stdout or -o, using the same rendering logic as
    // the HTTP path via a tiny inline writer. Headers to stderr at -v.
    if args.verbose >= 1 || args.include_headers || args.head_only || args.full {
        let put_to_stdout = args.include_headers || args.head_only || args.full;
        let out: &mut dyn Write = if put_to_stdout {
            &mut std::io::stdout().lock()
        } else {
            &mut std::io::stderr().lock()
        };
        writeln!(out, "< HTTP/{} {}", r.http_version, r.status)?;
        for (k, v) in &r.headers {
            writeln!(out, "< {k}: {v}")?;
        }
        writeln!(out, "<")?;
    }
    if args.head_only && !args.full {
        return Ok(());
    }
    if let Some(path) = &args.output {
        std::fs::write(path, &r.body)
            .with_context(|| format!("unix-socket: write {}", path.display()))?;
        if !args.silent {
            eprintln!("Saved to {}", path.display());
        }
    } else {
        std::io::stdout().write_all(&r.body)?;
    }
    Ok(())
}

fn resolve_method(args: &Args) -> String {
    if let Some(m) = &args.method {
        return m.to_uppercase();
    }
    if args.head_only {
        return "HEAD".to_string();
    }
    if args.data.is_some()
        || args.data_raw.is_some()
        || args.data_binary.is_some()
        || !args.data_urlencode.is_empty()
        || args.json.is_some()
    {
        return "POST".to_string();
    }
    if args.upload_file.is_some() {
        return "PUT".to_string();
    }
    "GET".to_string()
}

fn resolve_body(args: &Args) -> Result<Vec<u8>> {
    if let Some(path) = &args.upload_file {
        return std::fs::read(path).with_context(|| format!("upload file '{}'", path.display()));
    }
    if let Some(json) = &args.json {
        return Ok(load_body_string(json)?);
    }
    if let Some(raw) = &args.data_raw {
        return Ok(raw.as_bytes().to_vec());
    }
    if let Some(bin) = &args.data_binary {
        if let Some(p) = bin.strip_prefix('@') {
            return std::fs::read(p).with_context(|| format!("data-binary file '{p}'"));
        }
        return Ok(bin.as_bytes().to_vec());
    }
    if !args.data_urlencode.is_empty() {
        // urlencode is joined the same way client.rs does it; but for UDS
        // diagnostic use, the bare -d case is enough. Not implementing the
        // full --data-urlencode grammar here.
        bail!("unix-socket: --data-urlencode not supported over UDS; use -d / --json");
    }
    if let Some(data) = &args.data {
        return Ok(load_body_string(data)?);
    }
    Ok(Vec::new())
}

fn load_body_string(s: &str) -> Result<Vec<u8>> {
    if s == "@-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).context("body from stdin")?;
        return Ok(buf);
    }
    if let Some(p) = s.strip_prefix('@') {
        let raw = std::fs::read(p).with_context(|| format!("body file '{p}'"))?;
        return Ok(raw.into_iter().filter(|&b| b != b'\r' && b != b'\n').collect());
    }
    Ok(s.as_bytes().to_vec())
}

fn parse_url(url: &str) -> Result<(String, String)> {
    // UDS URLs: host is advisory (used as the Host: header), path goes
    // on the wire. Accept http://host/path and plain /path.
    let (host, path) = if let Some(rest) = url.strip_prefix("http://") {
        let (h, p) = rest.split_once('/').unwrap_or((rest, ""));
        (h.to_string(), format!("/{p}"))
    } else if let Some(rest) = url.strip_prefix("https://") {
        let (h, p) = rest.split_once('/').unwrap_or((rest, ""));
        (h.to_string(), format!("/{p}"))
    } else if url.starts_with('/') {
        ("localhost".to_string(), url.to_string())
    } else {
        bail!("unix-socket: URL must start with http://, https://, or /");
    };
    Ok((host, path))
}

fn parse_status_line(line: &str) -> Result<(String, u16)> {
    // "HTTP/1.1 200 OK\r\n"
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
    if parts.len() < 2 {
        bail!("unix-socket: malformed status line: {line:?}");
    }
    let version = parts[0].trim_start_matches("HTTP/").to_string();
    let status: u16 = parts[1]
        .parse()
        .map_err(|e| anyhow!("unix-socket: bad status code '{}': {e}", parts[1]))?;
    Ok((version, status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_http_scheme() {
        let (h, p) = parse_url("http://localhost/v1.40/version").unwrap();
        assert_eq!(h, "localhost");
        assert_eq!(p, "/v1.40/version");
    }

    #[test]
    fn parse_url_path_only() {
        let (h, p) = parse_url("/_ping").unwrap();
        assert_eq!(h, "localhost");
        assert_eq!(p, "/_ping");
    }

    #[test]
    fn parse_url_https_scheme() {
        let (h, p) = parse_url("https://api/v1/info").unwrap();
        assert_eq!(h, "api");
        assert_eq!(p, "/v1/info");
    }

    #[test]
    fn parse_url_rejects_unknown_scheme() {
        assert!(parse_url("ftp://host/").is_err());
    }

    #[test]
    fn parse_status_ok() {
        let (v, s) = parse_status_line("HTTP/1.1 200 OK\r\n").unwrap();
        assert_eq!(v, "1.1");
        assert_eq!(s, 200);
    }

    #[test]
    fn parse_status_no_reason() {
        let (v, s) = parse_status_line("HTTP/1.0 404\r\n").unwrap();
        assert_eq!(v, "1.0");
        assert_eq!(s, 404);
    }

    #[test]
    fn parse_status_rejects_malformed() {
        assert!(parse_status_line("garbage").is_err());
    }
}
