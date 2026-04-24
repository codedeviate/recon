//! `-K / --config <FILE>` loader.
//!
//! Reads a curl-style config file and expands its contents into
//! argv. Called before clap parses, so the expanded flags slot into
//! the command line as if the user had typed them.
//!
//! Format:
//! - One option per line: `--flag value` or `flag = value`.
//! - `#` or `;` starts a comment to end-of-line.
//! - Blank lines ignored.
//! - Values with spaces must be quoted (`"..."` or `'...'`).
//! - `@another-file` on its own line includes another config.
//!
//! Safety: include cycles are caught via a depth limit (8 levels).

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const INCLUDE_LIMIT: usize = 8;

/// Expand `--config` into argv tokens. Returns the list of tokens
/// in file order, NOT prefixed with the program name.
pub fn load(path: &Path) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    load_recursive(path, &mut seen, 0)
}

fn load_recursive(path: &Path, seen: &mut HashSet<PathBuf>, depth: usize) -> Result<Vec<String>> {
    if depth > INCLUDE_LIMIT {
        anyhow::bail!("--config: include depth exceeded {INCLUDE_LIMIT} levels");
    }
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(canon.clone()) {
        anyhow::bail!("--config: include cycle detected at {}", canon.display());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("--config: read {}", path.display()))?;
    let mut out = Vec::new();
    for (lineno, raw_line) in text.lines().enumerate() {
        let stripped = strip_comments(raw_line);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(inc) = line.strip_prefix('@') {
            let inc_path = resolve_relative(path, inc.trim());
            out.extend(load_recursive(&inc_path, seen, depth + 1)?);
            continue;
        }
        let tokens = tokenize_line(line)
            .with_context(|| format!("--config: {} line {}: tokenize", path.display(), lineno + 1))?;
        // Accept both `--flag value` and `flag = value` forms.
        for t in tokens {
            out.push(t);
        }
    }
    Ok(out)
}

fn strip_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_single = false;
    let mut in_double = false;
    for c in line.chars() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' | ';' if !in_single && !in_double => break,
            _ => {}
        }
        out.push(c);
    }
    out
}

fn tokenize_line(line: &str) -> Result<Vec<String>> {
    // Phase 1: split on whitespace, honoring quotes.
    let mut raw = Vec::new();
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if in_double && i + 1 < chars.len() => {
                cur.push(chars[i + 1]);
                i += 2;
                continue;
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !cur.is_empty() {
                    raw.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
        i += 1;
    }
    if in_single || in_double {
        anyhow::bail!("unterminated quoted string");
    }
    if !cur.is_empty() {
        raw.push(cur);
    }

    // Phase 2: normalise. The first token is the key. If it contains
    // a `=`, split at the first `=` into key + value. Prefix bare
    // keys (not starting with `-`) with `--`.
    let mut tokens = Vec::with_capacity(raw.len() + 1);
    let mut iter = raw.into_iter();
    if let Some(first) = iter.next() {
        let (key, rest) = match first.split_once('=') {
            Some((k, v)) => (k.to_string(), Some(v.to_string())),
            None => (first, None),
        };
        // Accept `key = value` (space-separated) — next token is then
        // "=" or "=value" or the value itself.
        let prefixed_key = if key.starts_with('-') {
            key
        } else {
            format!("--{key}")
        };
        tokens.push(prefixed_key);
        if let Some(v) = rest {
            if !v.is_empty() {
                tokens.push(v);
            }
        }
    }
    // Remaining tokens are values / continuations. If the NEXT token
    // is exactly "=" or starts with "=", strip it as the separator.
    for t in iter {
        if let Some(v) = t.strip_prefix('=') {
            if !v.is_empty() {
                tokens.push(v.to_string());
            }
            continue;
        }
        if t == "=" {
            continue;
        }
        tokens.push(t);
    }
    Ok(tokens)
}

fn resolve_relative(base: &Path, target: &str) -> PathBuf {
    let p = Path::new(target);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    base.parent().unwrap_or(Path::new(".")).join(target)
}

/// Pre-parse argv: if `-K <file>` or `--config <file>` appears,
/// expand it in place. Called BEFORE clap parses. The file's
/// tokens are inserted right after the original `-K FILE` pair.
pub fn expand_config_in_argv(argv: Vec<String>) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(argv.len());
    let mut iter = argv.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "-K" || arg == "--config" {
            match iter.next() {
                Some(path) => {
                    let tokens = load(Path::new(&path))
                        .with_context(|| format!("--config {path}"))?;
                    // Don't push the original -K/--config pair —
                    // they've been consumed.
                    out.extend(tokens);
                }
                None => anyhow::bail!("--config / -K needs a file path"),
            }
            continue;
        }
        // Also handle -K=FILE / --config=FILE form.
        if let Some(rest) = arg.strip_prefix("--config=") {
            out.extend(load(Path::new(rest))?);
            continue;
        }
        out.push(arg);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn basic_config_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "# comment").unwrap();
        writeln!(tmp, "--insecure").unwrap();
        writeln!(tmp, "--user alice:secret").unwrap();
        writeln!(tmp, "url = https://example.com/").unwrap();
        let tokens = load(tmp.path()).unwrap();
        assert!(tokens.contains(&"--insecure".into()));
        assert!(tokens.contains(&"--user".into()));
        assert!(tokens.contains(&"alice:secret".into()));
        assert!(tokens.contains(&"--url".into()));
        assert!(tokens.contains(&"https://example.com/".into()));
    }

    #[test]
    fn inline_comments_and_quoted_values() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "--user-agent \"recon CI\"  # trailing comment").unwrap();
        writeln!(tmp, "--header 'X-Trace: yes # not-a-comment'").unwrap();
        let tokens = load(tmp.path()).unwrap();
        assert!(tokens.iter().any(|t| t == "recon CI"));
        assert!(tokens.iter().any(|t| t.contains("# not-a-comment")));
    }

    #[test]
    fn include_another_file() {
        let mut base = tempfile::NamedTempFile::new().unwrap();
        let mut inc = tempfile::NamedTempFile::new().unwrap();
        writeln!(inc, "--verbose").unwrap();
        writeln!(base, "@{}", inc.path().display()).unwrap();
        writeln!(base, "--insecure").unwrap();
        let tokens = load(base.path()).unwrap();
        assert!(tokens.contains(&"--verbose".into()));
        assert!(tokens.contains(&"--insecure".into()));
    }

    #[test]
    fn include_cycle_is_caught() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::write(&a, format!("@{}\n", b.display())).unwrap();
        std::fs::write(&b, format!("@{}\n", a.display())).unwrap();
        let err = load(&a).unwrap_err();
        assert!(err.to_string().contains("cycle"), "{err}");
    }

    #[test]
    fn argv_expansion_preserves_other_args() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "--verbose").unwrap();
        let argv = vec![
            "recon".into(),
            "--config".into(),
            tmp.path().to_string_lossy().into_owned(),
            "https://example.com/".into(),
        ];
        let out = expand_config_in_argv(argv).unwrap();
        assert_eq!(out[0], "recon");
        assert!(out.contains(&"--verbose".into()));
        assert!(out.contains(&"https://example.com/".into()));
        assert!(!out.contains(&"--config".into()));
    }
}
