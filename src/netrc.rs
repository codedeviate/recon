//! `~/.netrc` parser and host lookup.
//!
//! Implements the traditional Unix netrc format curl uses: tokens are
//! whitespace-separated, case-insensitive keywords introduce a
//! `machine <host>` / `default` block, each block carries `login`,
//! `password`, `account`, `macdef` entries. We only care about
//! login + password.
//!
//! Matching: exact hostname wins; otherwise the first `default` block
//! (if any) is used. No wildcard support — matches curl's behaviour.

use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetrcEntry {
    pub login: Option<String>,
    pub password: Option<String>,
    pub account: Option<String>,
}

/// Parse a netrc file and return the entry matching `host`. Falls back
/// to the `default` block when no machine entry matches. Returns
/// `Ok(None)` when the file doesn't exist AND `optional` is true.
pub fn lookup(path: &Path, host: &str, optional: bool) -> Result<Option<NetrcEntry>> {
    if !path.exists() {
        if optional {
            return Ok(None);
        }
        anyhow::bail!(
            "netrc: {} does not exist (pass --netrc-optional to silence)",
            path.display()
        );
    }
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("netrc: read {}", path.display()))?;
    Ok(find_host(&contents, host))
}

fn find_host(input: &str, host: &str) -> Option<NetrcEntry> {
    let tokens = tokenize(input);
    let mut iter = tokens.iter().peekable();
    let mut current_host: Option<String> = None;
    let mut current_is_default = false;
    let mut current = NetrcEntry::default();
    let mut matched_host: Option<NetrcEntry> = None;
    let mut default_entry: Option<NetrcEntry> = None;

    while let Some(tok) = iter.next() {
        let k = tok.to_ascii_lowercase();
        match k.as_str() {
            "machine" => {
                // Finalise previous block.
                finalise(&current_host, current_is_default, &current, host, &mut matched_host, &mut default_entry);
                current_host = iter.next().cloned();
                current_is_default = false;
                current = NetrcEntry::default();
            }
            "default" => {
                finalise(&current_host, current_is_default, &current, host, &mut matched_host, &mut default_entry);
                current_host = None;
                current_is_default = true;
                current = NetrcEntry::default();
            }
            "login" => {
                if let Some(v) = iter.next() {
                    current.login = Some(v.clone());
                }
            }
            "password" | "passwd" => {
                if let Some(v) = iter.next() {
                    current.password = Some(v.clone());
                }
            }
            "account" => {
                if let Some(v) = iter.next() {
                    current.account = Some(v.clone());
                }
            }
            "macdef" => {
                // Consume macdef body until blank line. Tokenizer has
                // already flattened whitespace, so we simply skip the
                // name token and absorb tokens until the next keyword.
                iter.next();
                while let Some(n) = iter.peek() {
                    let nl = n.to_ascii_lowercase();
                    if matches!(nl.as_str(), "machine" | "default" | "login" | "password" | "passwd" | "account" | "macdef") {
                        break;
                    }
                    iter.next();
                }
            }
            _ => {
                // Unknown token — skip.
            }
        }
    }
    // Finalise the last block.
    finalise(&current_host, current_is_default, &current, host, &mut matched_host, &mut default_entry);

    matched_host.or(default_entry)
}

fn finalise(
    current_host: &Option<String>,
    current_is_default: bool,
    current: &NetrcEntry,
    target: &str,
    matched: &mut Option<NetrcEntry>,
    default_entry: &mut Option<NetrcEntry>,
) {
    if current.login.is_none() && current.password.is_none() && current.account.is_none() {
        return;
    }
    if let Some(host) = current_host {
        if host.eq_ignore_ascii_case(target) && matched.is_none() {
            *matched = Some(current.clone());
        }
    } else if current_is_default && default_entry.is_none() {
        *default_entry = Some(current.clone());
    }
}

/// Tokenize a netrc file — whitespace-separated, no quoting quirks.
fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_comment = false;
    for ch in input.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        match ch {
            '#' => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                in_comment = true;
            }
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Resolve the netrc file path from CLI args, respecting the
/// --netrc-file override and the NETRC environment variable. Returns
/// `None` when netrc support isn't requested at all.
pub fn resolve_netrc_path(args: &crate::cli::Args) -> Option<std::path::PathBuf> {
    if !args.netrc && args.netrc_file.is_none() && !args.netrc_optional {
        return None;
    }
    if let Some(p) = args.netrc_file.as_ref() {
        return Some(p.clone());
    }
    if let Ok(env) = std::env::var("NETRC") {
        if !env.is_empty() {
            return Some(std::path::PathBuf::from(env));
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Some(std::path::PathBuf::from(home).join(".netrc"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_matching_machine_block() {
        let input = r#"
machine api.example.com
login alice
password s3cr3t

machine other.example.com
login bob
password hunter2
"#;
        let got = find_host(input, "api.example.com").unwrap();
        assert_eq!(got.login.as_deref(), Some("alice"));
        assert_eq!(got.password.as_deref(), Some("s3cr3t"));
    }

    #[test]
    fn falls_back_to_default_block() {
        let input = r#"
machine api.example.com
login alice
password s3cr3t

default
login guest
password x
"#;
        let got = find_host(input, "somethingelse.com").unwrap();
        assert_eq!(got.login.as_deref(), Some("guest"));
    }

    #[test]
    fn returns_none_when_no_match_and_no_default() {
        let input = r#"
machine api.example.com
login alice
password s3cr3t
"#;
        assert!(find_host(input, "somethingelse.com").is_none());
    }

    #[test]
    fn tolerates_comments_and_blank_lines() {
        let input = r#"
# this is a comment
machine api.example.com  # inline comment
login alice
password s3cr3t
"#;
        let got = find_host(input, "api.example.com").unwrap();
        assert_eq!(got.login.as_deref(), Some("alice"));
    }

    #[test]
    fn ignores_macdef_body() {
        let input = r#"
machine api.example.com
login alice
password s3cr3t
macdef init
    cd /pub
    ls

machine other.example.com
login bob
password hunter2
"#;
        let got = find_host(input, "other.example.com").unwrap();
        assert_eq!(got.login.as_deref(), Some("bob"));
    }

    #[test]
    fn host_match_is_case_insensitive() {
        let input = r#"
machine API.Example.com
login alice
password s3cr3t
"#;
        let got = find_host(input, "api.example.com").unwrap();
        assert_eq!(got.login.as_deref(), Some("alice"));
    }
}
