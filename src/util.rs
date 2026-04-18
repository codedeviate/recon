/// Extracts (host, port) from any of:
///   "example.com", "example.com:8080", "https://example.com", "https://example.com:8080"
pub fn parse_target(input: &str) -> (String, Option<u16>) {
    // Strip protocol
    let s = if let Some(pos) = input.find("://") {
        &input[pos + 3..]
    } else {
        input
    };
    // Strip path, query, fragment
    let s = s
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(s);
    // IPv6 [::1]:port
    if s.starts_with('[') {
        if let Some(end) = s.find(']') {
            let host = s[1..end].to_string();
            let port = s[end + 1..]
                .strip_prefix(':')
                .and_then(|p| p.parse().ok());
            return (host, port);
        }
    }
    // Hostname or IPv4 with optional port
    if let Some(pos) = s.rfind(':') {
        if let Ok(port) = s[pos + 1..].parse::<u16>() {
            return (s[..pos].to_string(), Some(port));
        }
    }
    (s.to_string(), None)
}

/// Derive a local filename from a URL's final path segment.
/// Strips query and fragment, percent-decodes the segment. Errors when the
/// URL has no filename component (empty path / trailing slash), or when the
/// decoded name would escape the current directory (contains `/`, `\`,
/// or equals `.` / `..`).
pub fn filename_from_url(url: &str) -> Result<String, String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL '{url}': {e}"))?;
    let path = parsed.path();

    // The segment AFTER the last '/' in the path. Any query and fragment
    // live on parsed.query()/parsed.fragment(), not on path().
    let raw = match path.rsplit_once('/') {
        Some((_, last)) => last,
        None => path,
    };
    if raw.is_empty() {
        return Err(format!("--remote-name: URL '{url}' has no filename component"));
    }

    let decoded = percent_decode(raw);

    if decoded.is_empty()
        || decoded == "."
        || decoded == ".."
        || decoded.contains('/')
        || decoded.contains('\\')
    {
        return Err(format!(
            "--remote-name: derived filename '{decoded}' is not a safe local path"
        ));
    }

    Ok(decoded)
}

/// Minimal percent-decoder for URL path segments. Keeps the ASCII subset
/// recon URLs actually carry; invalid UTF-8 sequences fall back to a lossy
/// conversion.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_nibble(bytes[i + 1]);
            let lo = hex_nibble(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|e| {
        String::from_utf8_lossy(e.as_bytes()).into_owned()
    })
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_happy_path() {
        assert_eq!(
            filename_from_url("https://example.com/files/report.pdf").unwrap(),
            "report.pdf"
        );
    }

    #[test]
    fn filename_strips_query() {
        assert_eq!(
            filename_from_url("https://example.com/files/report.pdf?v=2").unwrap(),
            "report.pdf"
        );
    }

    #[test]
    fn filename_strips_fragment() {
        assert_eq!(
            filename_from_url("https://example.com/files/report.pdf#toc").unwrap(),
            "report.pdf"
        );
    }

    #[test]
    fn filename_percent_decodes() {
        assert_eq!(
            filename_from_url("https://example.com/files/my%20file.txt").unwrap(),
            "my file.txt"
        );
    }

    #[test]
    fn filename_trailing_slash_errors() {
        let err = filename_from_url("https://example.com/files/").unwrap_err();
        assert!(err.contains("no filename"));
    }

    #[test]
    fn filename_empty_path_errors() {
        let err = filename_from_url("https://example.com").unwrap_err();
        assert!(err.contains("no filename"));
    }

    #[test]
    fn filename_dotdot_errors() {
        // The url crate normalises both "/.." and "/%2e%2e" to "/", so the
        // last path segment becomes empty and we get "no filename" rather than
        // "not a safe local path". Either error correctly rejects "..".
        let err = filename_from_url("https://example.com/%2e%2e").unwrap_err();
        assert!(err.contains("not a safe local path") || err.contains("no filename"));
    }

    #[test]
    fn filename_embedded_slash_errors() {
        // %2f decodes to '/'.
        let err = filename_from_url("https://example.com/a%2fb").unwrap_err();
        assert!(err.contains("not a safe local path"));
    }
}
