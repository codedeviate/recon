/// Parse `Content-Disposition` header for a filename per RFC 6266.
/// Returns `None` if:
///   - header absent / malformed
///   - filename is empty
///   - filename contains path traversal (/, \, null, ..)
///   - filename is a Windows-reserved device name
pub fn filename_from_content_disposition(cd: &str) -> Option<String> {
    let lowered = cd.to_lowercase();

    // Prefer filename*= (RFC 5987 extended form, UTF-8 aware)
    if let Some(name) = extract_extended_filename(cd, &lowered) {
        return sanitize_filename(&name);
    }

    // Fall back to plain filename=
    if let Some(name) = extract_plain_filename(cd, &lowered) {
        return sanitize_filename(&name);
    }

    None
}

fn extract_extended_filename(cd: &str, lowered: &str) -> Option<String> {
    let idx = lowered.find("filename*=")?;
    let rest = &cd[idx + "filename*=".len()..];
    // Format: charset'lang'encoded-value
    let value = take_until_semicolon_or_end(rest);
    let value = value.trim().trim_matches('"');
    let mut parts = value.splitn(3, '\'');
    let charset = parts.next()?.to_lowercase();
    let _lang = parts.next()?;
    let encoded = parts.next()?;
    if charset != "utf-8" && charset != "us-ascii" {
        return None;
    }
    Some(percent_decode_utf8(encoded))
}

fn extract_plain_filename(cd: &str, lowered: &str) -> Option<String> {
    let idx = lowered.find("filename=")?;
    // Ensure we didn't match "filename*=" — skip if preceding char is '*'
    if idx > 0 && &cd[idx - 1..idx] == "*" {
        return None;
    }
    let rest = &cd[idx + "filename=".len()..];
    let value = take_until_semicolon_or_end(rest);
    let value = value.trim().trim_matches('"');
    Some(value.to_string())
}

fn take_until_semicolon_or_end(s: &str) -> &str {
    let mut in_quotes = false;
    let mut end = s.len();
    for (i, c) in s.char_indices() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == ';' && !in_quotes {
            end = i;
            break;
        }
    }
    &s[..end]
}

/// Percent-decode `s` into a byte buffer, then interpret as UTF-8.
/// This matters because RFC 5987 filename* values encode UTF-8 bytes as
/// `%XX` pairs — e.g. `r%C3%A9sum%C3%A9` is the byte sequence for "résumé".
/// Pushing bytes as `char` values individually would Latin-1-encode them,
/// producing garbage. Collect bytes first, decode as UTF-8 at the end.
fn percent_decode_utf8(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h1), Some(h2)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((h1 * 16 + h2) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Reject filenames with path traversal, nulls, or Windows-reserved names.
fn sanitize_filename(name: &str) -> Option<String> {
    if name.is_empty() {
        return None;
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') || name.contains("..") {
        return None;
    }
    let lower = name.to_lowercase();
    for reserved in [
        "con", "prn", "aux", "nul",
        "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
        "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ] {
        if lower == reserved || lower.starts_with(&format!("{}.", reserved)) {
            return None;
        }
    }
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ascii_filename() {
        let cd = r#"attachment; filename="report.pdf""#;
        assert_eq!(filename_from_content_disposition(cd), Some("report.pdf".into()));
    }

    #[test]
    fn extended_utf8_filename() {
        let cd = "attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf";
        assert_eq!(filename_from_content_disposition(cd), Some("résumé.pdf".into()));
    }

    #[test]
    fn extended_preferred_over_plain() {
        let cd = r#"attachment; filename="fallback.txt"; filename*=UTF-8''real.txt"#;
        assert_eq!(filename_from_content_disposition(cd), Some("real.txt".into()));
    }

    #[test]
    fn rejects_path_traversal() {
        let cd = r#"attachment; filename="../../etc/passwd""#;
        assert_eq!(filename_from_content_disposition(cd), None);
    }

    #[test]
    fn rejects_slash() {
        let cd = r#"attachment; filename="sub/file.txt""#;
        assert_eq!(filename_from_content_disposition(cd), None);
    }

    #[test]
    fn rejects_empty() {
        let cd = r#"attachment; filename=""#;
        assert_eq!(filename_from_content_disposition(cd), None);
    }

    #[test]
    fn rejects_reserved_windows_names() {
        let cd = r#"attachment; filename="CON""#;
        assert_eq!(filename_from_content_disposition(cd), None);
        let cd = r#"attachment; filename="nul.txt""#;
        assert_eq!(filename_from_content_disposition(cd), None);
    }

    #[test]
    fn no_filename_returns_none() {
        assert_eq!(filename_from_content_disposition("inline"), None);
    }
}
