use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The result of handling a file-serve request.
pub struct ServeResponse {
    pub response: Response<Full<Bytes>>,
    pub bytes: u64,
}

/// Main entry point: serve a file or directory listing.
pub fn handle_request(req: &Request<Incoming>, root: &Path) -> ServeResponse {
    let uri_path = percent_decode(req.uri().path());
    let rel = uri_path.trim_start_matches('/');

    // Build the requested filesystem path
    let requested = root.join(rel);

    // Canonicalize (resolves symlinks and ..)
    let canonical = match requested.canonicalize() {
        Ok(p) => p,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "404 Not Found"),
    };

    // Path traversal check: canonical must be inside root
    let canon_root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "500 Internal Server Error"),
    };
    if !canonical.starts_with(&canon_root) {
        return error_response(StatusCode::FORBIDDEN, "403 Forbidden");
    }

    if canonical.is_dir() {
        // Check for index.html
        let index = canonical.join("index.html");
        if index.is_file() {
            return serve_file(&index);
        }
        // Directory listing
        let accept = req
            .headers()
            .get("accept")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let wants_html = accept.contains("text/html");
        serve_directory(&canonical, &uri_path, wants_html)
    } else if canonical.is_file() {
        serve_file(&canonical)
    } else {
        error_response(StatusCode::NOT_FOUND, "404 Not Found")
    }
}

fn serve_file(path: &Path) -> ServeResponse {
    let body = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "500 Read Error"),
    };

    let mime = mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream");

    let len = body.len() as u64;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", mime)
        .header("content-length", len)
        .body(Full::new(Bytes::from(body)))
        .unwrap();

    ServeResponse {
        response,
        bytes: len,
    }
}

fn serve_directory(dir: &Path, uri_path: &str, html: bool) -> ServeResponse {
    let mut entries = match collect_entries(dir) {
        Ok(e) => e,
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "500 Read Error"),
    };

    // Sort: directories first, then alphabetical
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name))
    });

    let trailing = if uri_path.ends_with('/') { "" } else { "/" };
    let base = format!("{}{}", uri_path, trailing);

    let (body, content_type) = if html {
        (render_html(&entries, &base, uri_path), "text/html; charset=utf-8")
    } else {
        (render_text(&entries, &base), "text/plain; charset=utf-8")
    };

    let len = body.len() as u64;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type)
        .header("content-length", len)
        .body(Full::new(Bytes::from(body)))
        .unwrap();

    ServeResponse {
        response,
        bytes: len,
    }
}

struct DirEntry {
    name: String,
    is_dir: bool,
    size: u64,
    modified: Option<SystemTime>,
}

fn collect_entries(dir: &Path) -> Result<Vec<DirEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(DirEntry {
            name,
            is_dir: meta.is_dir(),
            size: meta.len(),
            modified: meta.modified().ok(),
        });
    }
    Ok(entries)
}

fn render_html(entries: &[DirEntry], base: &str, uri_path: &str) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str(&format!(
        "<title>Index of {}</title>\n",
        html_escape(uri_path)
    ));
    html.push_str("<style>\n");
    html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace; margin: 2em; color: #333; }\n");
    html.push_str("h1 { font-size: 1.3em; }\n");
    html.push_str("table { border-collapse: collapse; width: 100%; max-width: 800px; }\n");
    html.push_str("th, td { text-align: left; padding: 4px 12px; }\n");
    html.push_str("th { border-bottom: 2px solid #ccc; }\n");
    html.push_str("td { border-bottom: 1px solid #eee; }\n");
    html.push_str("a { text-decoration: none; color: #0366d6; }\n");
    html.push_str("a:hover { text-decoration: underline; }\n");
    html.push_str(".size, .modified { color: #666; }\n");
    html.push_str("</style>\n</head>\n<body>\n");
    html.push_str(&format!("<h1>Index of {}</h1>\n", html_escape(uri_path)));
    html.push_str("<table>\n<tr><th>Name</th><th>Size</th><th>Modified</th></tr>\n");

    // Parent directory link
    if base != "/" {
        let parent = parent_path(base);
        html.push_str(&format!(
            "<tr><td><a href=\"{}\">../</a></td><td class=\"size\">-</td><td class=\"modified\">-</td></tr>\n",
            parent
        ));
    }

    for e in entries {
        let display = if e.is_dir {
            format!("{}/", e.name)
        } else {
            e.name.clone()
        };
        let href = format!("{}{}", base, percent_encode(&e.name));
        let size = if e.is_dir {
            "-".to_string()
        } else {
            humanize_size(e.size)
        };
        let modified = e
            .modified
            .map(|t| format_time(t))
            .unwrap_or_else(|| "-".to_string());
        html.push_str(&format!(
            "<tr><td><a href=\"{}\">{}</a></td><td class=\"size\">{}</td><td class=\"modified\">{}</td></tr>\n",
            html_escape(&href),
            html_escape(&display),
            size,
            modified
        ));
    }

    html.push_str("</table>\n</body>\n</html>\n");
    html
}

fn render_text(entries: &[DirEntry], base: &str) -> String {
    let mut lines = Vec::new();

    // Header
    lines.push(format!("{:<40} {:>10}  {}", "Name", "Size", "Modified"));
    lines.push(format!("{}", "-".repeat(70)));

    // Parent
    if base != "/" {
        lines.push(format!("{:<40} {:>10}  {}", "../", "-", "-"));
    }

    for e in entries {
        let display = if e.is_dir {
            format!("{}/", e.name)
        } else {
            e.name.clone()
        };
        let size = if e.is_dir {
            "-".to_string()
        } else {
            humanize_size(e.size)
        };
        let modified = e
            .modified
            .map(|t| format_time(t))
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!("{:<40} {:>10}  {}", display, size, modified));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn error_response(status: StatusCode, message: &str) -> ServeResponse {
    let body = format!(
        "<!DOCTYPE html><html><body><h1>{}</h1></body></html>",
        message
    );
    let len = body.len() as u64;
    let response = Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .header("content-length", len)
        .body(Full::new(Bytes::from(body)))
        .unwrap();
    ServeResponse {
        response,
        bytes: len,
    }
}

/// Human-readable file size.
pub fn humanize_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a SystemTime as "YYYY-MM-DD HH:MM" without chrono.
fn format_time(time: SystemTime) -> String {
    let dur = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() as i64;

    // Simple UTC breakdown
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    // Date from days since epoch (simplified Gregorian)
    let (year, month, day) = days_to_date(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, hours, minutes)
}

fn days_to_date(days: i64) -> (i64, i64, i64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Percent-decode a URL path segment.
fn percent_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// Percent-encode a filename for use in a URL.
fn percent_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn parent_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(pos) => format!("{}/", &trimmed[..pos]),
        None => "/".to_string(),
    }
}
