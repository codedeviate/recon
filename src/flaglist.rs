//! `recon --flags` — curl-style alphabetical flag listing.
//!
//! Walks the clap `Command` tree, extracts every argument's short +
//! long + value-name + short description, and renders them sorted by
//! long name. One line per flag, curl-compatible layout:
//!
//!   (short or 4-space pad) (, if short) --long-name VALUE  description
//!
//! The short description is truncated to a single line so the listing
//! scans quickly. For the full multi-line description use
//! `recon --help <topic>`.

use clap::CommandFactory;

use crate::cli::Args;

/// Left-hand gutter width: `-E, ` is 4 chars; padding when no short.
const SHORT_GUTTER: usize = 4;

/// Total width of the label side (short + long + value). Pad here so
/// every description starts at the same column.
const LABEL_WIDTH: usize = 40;

/// Cap the description at this many characters. Keeps every line
/// under ~95 columns when combined with the label. curl does the
/// same — descriptions in `curl --help all` are almost always ≤ 50
/// characters.
const DESCRIPTION_MAX: usize = 52;

/// Print the alphabetical flag listing to stdout. Returns `true`
/// (the caller exits after printing).
pub fn print_flags_listing() {
    let cmd = Args::command();
    let mut entries: Vec<FlagEntry> = cmd
        .get_arguments()
        .filter_map(FlagEntry::from_arg)
        .collect();
    entries.sort_by_key(|a| a.long.to_ascii_lowercase());

    for entry in &entries {
        println!("{}", entry.render());
    }
}

struct FlagEntry {
    short: Option<char>,
    long: String,
    value: Option<String>,
    description: String,
}

impl FlagEntry {
    fn from_arg(arg: &clap::Arg) -> Option<Self> {
        let long = arg.get_long()?.to_string();
        // Hide the "display-only" help/version stubs — clap handles
        // them before we'd ever get here. Users learn about them from
        // the footer anyway.
        if matches!(long.as_str(), "help") {
            return None;
        }
        let short = arg.get_short();
        // Pick the first value name when present (`--cert <PATH>`) —
        // but only for args that actually take a value. Boolean
        // flags (SetTrue / SetFalse) shouldn't render `<FOO>`.
        let takes_value = !matches!(
            arg.get_action(),
            clap::ArgAction::SetTrue | clap::ArgAction::SetFalse | clap::ArgAction::Count,
        );
        let value = if takes_value {
            arg.get_value_names()
                .and_then(|names| names.first().map(|n| n.to_string()))
        } else {
            None
        };
        // Prefer the short help if clap has one; fall back to the
        // long description's first line.
        let description = arg
            .get_help()
            .map(|s| s.to_string())
            .or_else(|| arg.get_long_help().map(|s| s.to_string()))
            .unwrap_or_default();
        let description = first_line(&description);
        Some(FlagEntry {
            short,
            long,
            value,
            description,
        })
    }

    fn render(&self) -> String {
        // Short side: "-X, " when present, else 4-space pad.
        let short_prefix = match self.short {
            Some(c) => format!("-{c}, "),
            None => " ".repeat(SHORT_GUTTER),
        };

        // Label: --long <VALUE>
        let mut label = format!("{}--{}", short_prefix, self.long);
        if let Some(v) = self.value.as_deref() {
            label.push_str(&format!(" <{v}>"));
        }

        // Pad label to LABEL_WIDTH so descriptions line up. If the
        // label is already longer (long flag + value name), fall back
        // to a two-space gap before the description.
        if label.len() < LABEL_WIDTH {
            format!(
                "{:<width$}{}",
                label,
                self.description,
                width = LABEL_WIDTH
            )
        } else {
            format!("{}  {}", label, self.description)
        }
    }
}

/// First non-empty line of a clap help string, truncated to a short
/// curl-style summary.
///
/// Priority:
/// 1. Use the first sentence (split on `. `) when it fits within
///    `DESCRIPTION_MAX`.
/// 2. Otherwise truncate to `DESCRIPTION_MAX` at the last word
///    boundary and append an ellipsis.
/// 3. Strip any trailing period so the column reads cleanly.
fn first_line(s: &str) -> String {
    let raw = s
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();

    // First sentence — same heuristic as curl.
    let first_sentence = match raw.split_once(". ") {
        Some((s, _)) if !s.is_empty() => s,
        _ => raw,
    };

    let candidate = first_sentence.trim_end_matches('.').trim();
    if candidate.len() <= DESCRIPTION_MAX {
        return candidate.to_string();
    }

    // Word-boundary truncation.
    let mut out = String::with_capacity(DESCRIPTION_MAX + 1);
    for word in candidate.split_whitespace() {
        if out.len() + 1 + word.len() > DESCRIPTION_MAX - 1 {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_sorted_alphabetically() {
        let cmd = Args::command();
        let mut entries: Vec<_> = cmd
            .get_arguments()
            .filter_map(FlagEntry::from_arg)
            .collect();
        entries.sort_by_key(|a| a.long.to_ascii_lowercase());
        assert!(!entries.is_empty(), "no flags extracted");
        for pair in entries.windows(2) {
            assert!(
                pair[0].long.to_ascii_lowercase() <= pair[1].long.to_ascii_lowercase(),
                "unsorted: {} vs {}",
                pair[0].long,
                pair[1].long,
            );
        }
    }

    #[test]
    fn help_stub_is_hidden() {
        let cmd = Args::command();
        let entries: Vec<_> = cmd
            .get_arguments()
            .filter_map(FlagEntry::from_arg)
            .collect();
        assert!(
            entries.iter().all(|e| e.long != "help"),
            "--help should be excluded",
        );
    }

    #[test]
    fn render_with_short_prefix() {
        let e = FlagEntry {
            short: Some('H'),
            long: "header".into(),
            value: Some("NAME: VALUE".into()),
            description: "Add a request header".into(),
        };
        let s = e.render();
        assert!(s.starts_with("-H, --header <NAME: VALUE>"), "got: {s}");
    }

    #[test]
    fn render_without_short_pads_gutter() {
        let e = FlagEntry {
            short: None,
            long: "cacert".into(),
            value: Some("PATH".into()),
            description: "Trust this CA".into(),
        };
        let s = e.render();
        assert!(s.starts_with("    --cacert <PATH>"), "got: {s}");
    }

    #[test]
    fn render_boolean_has_no_value_suffix() {
        let e = FlagEntry {
            short: Some('v'),
            long: "verbose".into(),
            value: None,
            description: "Increase verbosity".into(),
        };
        let s = e.render();
        assert!(s.starts_with("-v, --verbose "), "got: {s}");
        assert!(!s.contains('<'), "got: {s}");
    }

    #[test]
    fn first_line_strips_continuations() {
        assert_eq!(first_line("First line\nSecond line"), "First line");
        assert_eq!(first_line("  \n  Actual  \n  "), "Actual");
        assert_eq!(first_line(""), "");
    }

    #[test]
    fn first_line_takes_first_sentence() {
        assert_eq!(
            first_line("Fetch a URL. Also supports redirects and retries. More detail."),
            "Fetch a URL",
        );
    }

    #[test]
    fn first_line_truncates_overlong_single_sentence() {
        let long = "Probe the remote host with a multi-stage DNS + TCP + TLS handshake verification that needs wrapping";
        let out = first_line(long);
        assert!(
            out.ends_with('…'),
            "expected ellipsis on truncation, got: {out}",
        );
        // The ellipsis is multi-byte UTF-8 (3 bytes, 1 display column);
        // measure display width via `chars().count()`.
        let display_len = out.chars().count();
        assert!(
            display_len <= DESCRIPTION_MAX,
            "display length {display_len} > max {DESCRIPTION_MAX}, got: {out}",
        );
    }

    #[test]
    fn first_line_strips_trailing_period() {
        assert_eq!(first_line("Short description."), "Short description");
    }
}
