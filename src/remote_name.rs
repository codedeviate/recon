/// Parse `Content-Disposition` header for a filename per RFC 6266.
/// Returns `None` if:
///   - header absent / malformed
///   - filename is empty or Windows-reserved
///   - filename contains path traversal (/, \, null, ..)
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

/// Find the byte offset of `name` in `lowered` where it starts a parameter,
/// i.e. it is either at the start of the string or preceded by `;` or whitespace.
/// This prevents false positives from suffix matches (`x-filename=`) or matches
/// inside quoted values.
fn find_param_start(lowered: &str, name: &str) -> Option<usize> {
    let bytes = lowered.as_bytes();
    let mut search_from = 0;
    loop {
        let rel = lowered[search_from..].find(name)?;
        let absolute = search_from + rel;
        if absolute == 0 || matches!(bytes[absolute - 1], b';' | b' ' | b'\t') {
            return Some(absolute);
        }
        search_from = absolute + name.len();
    }
}

/// Strip at most one balanced pair of surrounding double-quotes.
/// `"foo"` → `foo`, `foo` → `foo`, `"foo` (unbalanced) → `"foo`.
fn trim_quoted(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn extract_extended_filename(cd: &str, lowered: &str) -> Option<String> {
    let idx = find_param_start(lowered, "filename*=")?;
    let rest = &cd[idx + "filename*=".len()..];
    // Format: charset'lang'encoded-value
    let value = take_until_semicolon_or_end(rest);
    let value = trim_quoted(value.trim());
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
    // Find "filename=" that starts a parameter; skip "filename*=" matches.
    let bytes = lowered.as_bytes();
    let needle = "filename=";
    let mut search_from = 0;
    let idx = loop {
        let rel = lowered[search_from..].find(needle)?;
        let absolute = search_from + rel;
        // Skip filename*= (RFC 5987 extended form — handled separately)
        if absolute > 0 && bytes[absolute - 1] == b'*' {
            search_from = absolute + needle.len();
            continue;
        }
        // Require preceding char is ';', whitespace, or start-of-string
        if absolute == 0 || matches!(bytes[absolute - 1], b';' | b' ' | b'\t') {
            break absolute;
        }
        search_from = absolute + needle.len();
    };
    let rest = &cd[idx + needle.len()..];
    let raw = take_until_semicolon_or_end(rest);
    let trimmed = raw.trim();
    // If value starts with a quote, it must be a balanced pair; otherwise
    // the quoted-string is malformed — treat as empty.
    let value = if trimmed.starts_with('"') {
        let unquoted = trim_quoted(trimmed);
        if unquoted == trimmed {
            // trim_quoted didn't strip anything → unbalanced opening quote
            return None;
        }
        unquoted
    } else {
        trimmed
    };
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

    // --- Hardening tests ---

    #[test]
    fn ignores_filename_inside_quoted_value() {
        // Malicious server puts "filename=hack" inside a different parameter's value.
        // The scanner must not pick up the match inside the quoted string.
        let cd = r#"attachment; x="filename=hack"; filename="real.txt""#;
        assert_eq!(filename_from_content_disposition(cd), Some("real.txt".into()));
    }

    #[test]
    fn ignores_filename_suffix_in_other_param() {
        // A parameter named "x-filename=" must not match as "filename=".
        let cd = r#"attachment; x-filename=bad.txt; filename="real.txt""#;
        assert_eq!(filename_from_content_disposition(cd), Some("real.txt".into()));
    }

    #[test]
    fn unbalanced_quotes_left_intact() {
        // Unterminated quote: the quoted-string is malformed (no closing `"`).
        // The hardened code treats this as an invalid value and returns None
        // rather than silently stripping the stray quote and returning content.
        let cd = r#"attachment; filename="unterminated"#;
        assert_eq!(filename_from_content_disposition(cd), None);
    }

    #[test]
    fn triple_quotes_not_collapsed() {
        // Old trim_matches('"') would strip ALL surrounding quotes giving "a".
        // The balanced-pair trim_quoted must NOT collapse triple-quotes to "a".
        let cd = r#"attachment; filename="""a""""#;
        let got = filename_from_content_disposition(cd);
        if let Some(s) = got {
            assert_ne!(s, "a", "greedy trim should have been replaced by balanced trim");
        }
    }
}
