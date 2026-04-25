//! wget-style filename suffix filters for `--accept` / `--reject`.
//!
//! Both flags take a comma-separated list of suffixes (with or without a
//! leading dot, case-insensitive). A URL is kept when:
//!   - if `--accept` is set: any suffix matches the URL's final path
//!     segment, AND
//!   - if `--reject` is set: no suffix matches.
//!
//! URLs whose final path segment is empty (path ending in `/`) fail any
//! `--accept` check (matches wget) and pass any `--reject` check.

/// Parse a comma-separated suffix list into a normalised vector. Each
/// suffix is trimmed, lowercased, and prefixed with `.` if missing so
/// `jpg`, `.jpg`, `JPG` all collapse to `.jpg`. Empty entries dropped.
pub fn parse_suffix_list(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .map(|s| if s.starts_with('.') { s } else { format!(".{s}") })
        .collect()
}

/// Return the lowercased final path segment of a URL string, or empty
/// if the URL has no segment (root path or trailing slash).
fn final_segment(url: &str) -> String {
    let no_frag = url.split('#').next().unwrap_or("");
    let no_query = no_frag.split('?').next().unwrap_or("");
    let after_scheme = match no_query.find("://") {
        Some(i) => &no_query[i + 3..],
        None => no_query,
    };
    let path = match after_scheme.find('/') {
        Some(i) => &after_scheme[i + 1..],
        None => {
            // No `/`: schemed URL like `https://e.com` has empty path,
            // schemeless input like `file.zip` is a bare filename.
            if no_query.contains("://") {
                ""
            } else {
                after_scheme
            }
        }
    };
    let last = path.rsplit('/').next().unwrap_or("");
    last.to_ascii_lowercase()
}

/// Decide whether to keep a URL given optional accept/reject suffix
/// lists. Empty segments fail `accept` and pass `reject` (wget parity).
pub fn should_keep(url: &str, accept: Option<&str>, reject: Option<&str>) -> bool {
    let seg = final_segment(url);
    if let Some(spec) = accept {
        let list = parse_suffix_list(spec);
        if !list.is_empty() && !list.iter().any(|s| seg.ends_with(s.as_str())) {
            return false;
        }
    }
    if let Some(spec) = reject {
        let list = parse_suffix_list(spec);
        if list.iter().any(|s| seg.ends_with(s.as_str())) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_normalises_dots_and_case() {
        let v = parse_suffix_list("jpg, .PNG ,gif");
        assert_eq!(v, vec![".jpg", ".png", ".gif"]);
    }

    #[test]
    fn parse_drops_empty_entries() {
        let v = parse_suffix_list(", ,jpg,,");
        assert_eq!(v, vec![".jpg"]);
    }

    #[test]
    fn final_segment_extracts_last_path_part() {
        assert_eq!(final_segment("https://e.com/a/b/c.JPG"), "c.jpg");
        assert_eq!(final_segment("https://e.com/a/b/c.jpg?x=1"), "c.jpg");
        assert_eq!(final_segment("https://e.com/a/b/c.jpg#frag"), "c.jpg");
        assert_eq!(final_segment("https://e.com/"), "");
        assert_eq!(final_segment("https://e.com"), "");
        assert_eq!(final_segment("file.zip"), "file.zip");
    }

    #[test]
    fn accept_keeps_matching() {
        assert!(should_keep("https://e.com/x.jpg", Some("jpg,png"), None));
        assert!(should_keep("https://e.com/x.PNG", Some("jpg,png"), None));
        assert!(!should_keep("https://e.com/x.gif", Some("jpg,png"), None));
    }

    #[test]
    fn reject_drops_matching() {
        assert!(!should_keep("https://e.com/x.bak", None, Some("bak,tmp")));
        assert!(should_keep("https://e.com/x.jpg", None, Some("bak,tmp")));
    }

    #[test]
    fn accept_and_reject_combine() {
        // Pass accept (.jpg) AND not in reject (.thumb.jpg-like)
        assert!(should_keep(
            "https://e.com/photo.jpg",
            Some("jpg,png"),
            Some("thumb"),
        ));
        // Fails reject
        assert!(!should_keep(
            "https://e.com/photo-thumb",
            Some("jpg,png,thumb"),
            Some("thumb"),
        ));
    }

    #[test]
    fn empty_segment_fails_accept_passes_reject() {
        assert!(!should_keep("https://e.com/", Some("jpg"), None));
        assert!(should_keep("https://e.com/", None, Some("bak")));
    }

    #[test]
    fn no_filters_keeps_everything() {
        assert!(should_keep("https://e.com/anything", None, None));
    }
}
