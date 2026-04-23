//! `--compare <A> <B>` — diff two sources.
//!
//! Each source is resolved through `crate::source` so URLs, files, and
//! stdin all flow through the same pipeline. HTTP(S) sources honor all
//! existing request flags (headers, auth, redirects, TLS). Binary
//! content is detected and reported as a byte-count delta rather than
//! attempting a line diff.

use anyhow::{bail, Context, Result};
use similar::{ChangeTag, TextDiff};

use crate::cli::Args;

/// Exit codes mirror the GNU `diff` / `cmp` convention:
/// - `0` — sources are byte-identical
/// - `1` — sources differ
/// - `2+` — load error (caller handles via `Result`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareVerdict {
    Identical,
    Differ,
}

impl CompareVerdict {
    pub fn exit_code(self) -> i32 {
        match self {
            CompareVerdict::Identical => 0,
            CompareVerdict::Differ => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareFormat {
    Unified,
    Summary,
    Sxs,
}

impl CompareFormat {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "unified" | "u" => Ok(CompareFormat::Unified),
            "summary" | "s" => Ok(CompareFormat::Summary),
            "sxs" | "side-by-side" => Ok(CompareFormat::Sxs),
            other => bail!("unknown --compare-format '{other}' (want unified|summary|sxs)"),
        }
    }
}

/// Public entry point from `main.rs`. Returns the verdict on success.
pub fn run(args: &Args) -> Result<CompareVerdict> {
    let pair = args
        .compare
        .as_ref()
        .context("--compare requires exactly two source arguments")?;
    if pair.len() != 2 {
        bail!("--compare requires exactly two source arguments");
    }
    let format = CompareFormat::parse(&args.compare_format)?;

    let a = load_source(args, &pair[0])
        .with_context(|| format!("failed to load --compare source A ({})", pair[0]))?;
    let b = load_source(args, &pair[1])
        .with_context(|| format!("failed to load --compare source B ({})", pair[1]))?;

    let verdict = render(&a, &b, &pair[0], &pair[1], format, args.compare_context);
    Ok(verdict)
}

fn load_source(args: &Args, spec: &str) -> Result<Vec<u8>> {
    let mut args = args.clone();
    args.compare = None;
    args.url = Some(spec.to_string());
    args.url_flag = None;
    crate::source::read_all(&args)
}

/// Heuristic binary detection: NUL in the first 8 KiB of either source.
pub(crate) fn is_binary(data: &[u8]) -> bool {
    let window = data.len().min(8192);
    data[..window].contains(&0)
}

fn render(
    a: &[u8],
    b: &[u8],
    label_a: &str,
    label_b: &str,
    format: CompareFormat,
    context: usize,
) -> CompareVerdict {
    if a == b {
        if matches!(format, CompareFormat::Summary) {
            println!("identical");
        }
        return CompareVerdict::Identical;
    }

    let binary = is_binary(a) || is_binary(b);

    match format {
        CompareFormat::Summary => {
            if binary {
                println!("differ (binary: {} vs {} bytes)", a.len(), b.len());
            } else {
                let (added, removed) = line_delta(a, b);
                println!("differ ({added} added, {removed} removed)");
            }
        }
        CompareFormat::Unified if binary => {
            println!(
                "Binary files {label_a} and {label_b} differ ({} vs {} bytes)",
                a.len(),
                b.len(),
            );
        }
        CompareFormat::Unified => {
            let ta = String::from_utf8_lossy(a);
            let tb = String::from_utf8_lossy(b);
            let diff = TextDiff::from_lines(ta.as_ref(), tb.as_ref());
            print!(
                "{}",
                diff.unified_diff()
                    .context_radius(context)
                    .header(label_a, label_b)
            );
        }
        CompareFormat::Sxs if binary => {
            println!(
                "Binary files {label_a} and {label_b} differ ({} vs {} bytes)",
                a.len(),
                b.len(),
            );
        }
        CompareFormat::Sxs => {
            let ta = String::from_utf8_lossy(a);
            let tb = String::from_utf8_lossy(b);
            let diff = TextDiff::from_lines(ta.as_ref(), tb.as_ref());
            print_sxs(&diff);
        }
    }
    CompareVerdict::Differ
}

fn line_delta(a: &[u8], b: &[u8]) -> (usize, usize) {
    let ta = String::from_utf8_lossy(a);
    let tb = String::from_utf8_lossy(b);
    let diff = TextDiff::from_lines(ta.as_ref(), tb.as_ref());
    let mut added = 0usize;
    let mut removed = 0usize;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => added += 1,
            ChangeTag::Delete => removed += 1,
            ChangeTag::Equal => {}
        }
    }
    (added, removed)
}

fn print_sxs<'a>(diff: &TextDiff<'a, 'a, 'a, str>) {
    let width = terminal_half_width();
    for change in diff.iter_all_changes() {
        let text = change.value().trim_end_matches('\n');
        let (left, right, marker) = match change.tag() {
            ChangeTag::Equal => (text, text, ' '),
            ChangeTag::Delete => (text, "", '<'),
            ChangeTag::Insert => ("", text, '>'),
        };
        println!(
            "{:<w$} {} {:<w$}",
            truncate(left, width),
            marker,
            truncate(right, width),
            w = width,
        );
    }
}

fn terminal_half_width() -> usize {
    let total = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(120);
    ((total.saturating_sub(3)) / 2).max(20)
}

fn truncate(s: &str, w: usize) -> String {
    let mut buf = String::with_capacity(w);
    for (i, c) in s.chars().enumerate() {
        if i >= w {
            break;
        }
        buf.push(c);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_byte_slices_return_identical() {
        let v = render(b"hello\n", b"hello\n", "a", "b", CompareFormat::Summary, 3);
        assert_eq!(v, CompareVerdict::Identical);
    }

    #[test]
    fn differing_slices_return_differ() {
        let v = render(b"hello\n", b"world\n", "a", "b", CompareFormat::Summary, 3);
        assert_eq!(v, CompareVerdict::Differ);
    }

    #[test]
    fn nul_byte_triggers_binary_detection() {
        assert!(is_binary(b"pre\0post"));
        assert!(!is_binary(b"plain text"));
    }

    #[test]
    fn parse_unified_accepts_aliases() {
        assert!(matches!(
            CompareFormat::parse("unified").unwrap(),
            CompareFormat::Unified
        ));
        assert!(matches!(
            CompareFormat::parse("u").unwrap(),
            CompareFormat::Unified
        ));
        assert!(matches!(
            CompareFormat::parse("SUMMARY").unwrap(),
            CompareFormat::Summary
        ));
        assert!(matches!(
            CompareFormat::parse("sxs").unwrap(),
            CompareFormat::Sxs
        ));
        assert!(CompareFormat::parse("bogus").is_err());
    }

    #[test]
    fn exit_codes_match_diff_convention() {
        assert_eq!(CompareVerdict::Identical.exit_code(), 0);
        assert_eq!(CompareVerdict::Differ.exit_code(), 1);
    }

    #[test]
    fn line_delta_counts_changes() {
        let (a, r) = line_delta(b"one\ntwo\n", b"one\ntwo\nthree\n");
        assert_eq!((a, r), (1, 0));
        let (a, r) = line_delta(b"one\ntwo\n", b"one\n");
        assert_eq!((a, r), (0, 1));
    }
}
