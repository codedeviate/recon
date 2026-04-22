//! Standalone `--iconv FROM:TO` action. Reads from the positional URL
//! argument (treated as a file path when the iconv flag is set) or
//! stdin, transcodes, writes to `-o PATH` or stdout.
//!
//! Blank `FROM` means "auto-detect via BOM + chardetng". When unmappable
//! characters are encountered, `?` is substituted (iconv's `-c` mode).
//! Exit code 0 on success, 1 on unmappable substitution, 2 on parse /
//! lookup errors.

use crate::cli::Args;
use crate::text_encoding;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

/// CLI entry point invoked from main.rs when `args.iconv` is Some.
/// Returns a process exit code.
pub fn run_cli(args: &Args) -> i32 {
    let spec = args.iconv.as_deref().unwrap_or("");
    match run(spec, args) {
        Ok(had_unmappable) => {
            if had_unmappable {
                if !args.silent {
                    eprintln!("! iconv: some characters were unmappable and substituted with '?'");
                }
                1
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            2
        }
    }
}

/// Transcode according to `spec` (`FROM:TO` or `:TO`). Returns true when
/// substitution occurred.
pub fn run(spec: &str, args: &Args) -> Result<bool> {
    let (from_label, to_label) = parse_spec(spec)?;

    // Source bytes: positional arg treated as a file path when set,
    // else stdin.
    let input_path = positional_input_path(args);
    let input: Vec<u8> = match input_path {
        Some(path) => fs::read(path)
            .with_context(|| format!("failed to read input: {}", path.display()))?,
        None => {
            let mut buf = Vec::new();
            io::stdin()
                .read_to_end(&mut buf)
                .context("failed to read input from stdin")?;
            buf
        }
    };

    // Resolve source encoding.
    let from = if let Some(label) = from_label {
        text_encoding::resolve(label)?
    } else {
        let detected = text_encoding::detect(&input);
        if !args.silent {
            eprintln!("iconv: auto-detected source charset as {}", detected.charset);
        }
        text_encoding::resolve(detected.charset)?
    };

    let to = text_encoding::resolve(&to_label)?;

    let r = text_encoding::transcode(&input, from, to);

    // Sink: -o PATH when set, else stdout.
    if let Some(out_path) = &args.output {
        if args.create_dirs {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).ok();
            }
        }
        fs::write(out_path, &r.bytes)
            .with_context(|| format!("failed to write output: {}", out_path.display()))?;
        if !args.silent {
            eprintln!("Saved to {}", out_path.display());
        }
    } else {
        io::stdout()
            .write_all(&r.bytes)
            .context("failed to write to stdout")?;
    }

    Ok(r.had_unmappable)
}

fn parse_spec(spec: &str) -> Result<(Option<&str>, String)> {
    let (from, to) = spec
        .split_once(':')
        .ok_or_else(|| anyhow!("--iconv spec must be 'FROM:TO' or ':TO' (got '{spec}')"))?;
    let from = from.trim();
    let to = to.trim();
    if to.is_empty() {
        return Err(anyhow!("--iconv: TARGET encoding must be non-empty"));
    }
    Ok((
        if from.is_empty() { None } else { Some(from) },
        to.to_string(),
    ))
}

/// Positional URL arg is re-used as the input file path when `--iconv` is
/// set. Returns None when no positional arg was given (stdin mode).
fn positional_input_path(args: &Args) -> Option<&Path> {
    args.url.as_deref().map(Path::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_from_to() {
        let (from, to) = parse_spec("iso-8859-1:utf-8").unwrap();
        assert_eq!(from, Some("iso-8859-1"));
        assert_eq!(to, "utf-8");
    }

    #[test]
    fn parse_spec_blank_from() {
        let (from, to) = parse_spec(":utf-8").unwrap();
        assert_eq!(from, None);
        assert_eq!(to, "utf-8");
    }

    #[test]
    fn parse_spec_rejects_missing_colon() {
        assert!(parse_spec("utf-8").is_err());
    }

    #[test]
    fn parse_spec_rejects_empty_target() {
        assert!(parse_spec("utf-8:").is_err());
        assert!(parse_spec(":").is_err());
    }
}
