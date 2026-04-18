//! Unified input source layer. Resolves recon's positional argument (or its
//! absence) into a `SourceKind`, then opens a streaming reader. HTTP sources
//! flow through the existing `client::execute` pipeline so all HTTP flags
//! (`-H`, `-u`, `-L`, `-k`, `-A`, cookies, `-e/--referer`, …) remain honored.
//! File, stdin, and `file://` sources bypass HTTP entirely.
//!
//! Detection rules are intentionally strict: a bare word like `example.com`
//! is treated as a file path, not as an auto-HTTPS URL. Users who want HTTPS
//! must write the scheme explicitly. This differs from recon's normal
//! positional handling but prevents surprising network side-effects in
//! source-layer contexts (hash, compress, encrypt, qr, barcode).

use anyhow::{anyhow, bail, Context, Result};
use std::io::Read;
use std::path::PathBuf;

use crate::cli::Args;

/// Where the bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    /// Process stdin (pipe or explicit `-`).
    Stdin,
    /// Local file at the given path.
    File(PathBuf),
    /// HTTP(S) URL; the full URL string is preserved.
    Http(String),
}

/// Inspect `args` (positional URL + stdin state) and decide which source
/// to use. Never performs I/O.
pub fn resolve(args: &Args) -> Result<SourceKind> {
    let positional = args.target_url();

    if positional.is_empty() {
        if stdin_is_pipe() {
            return Ok(SourceKind::Stdin);
        }
        bail!("no input source; pass a path, URL, or pipe to stdin");
    }

    if positional == "-" {
        return Ok(SourceKind::Stdin);
    }

    let lower = positional.to_ascii_lowercase();

    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(SourceKind::Http(positional.to_string()));
    }

    if lower.starts_with("file://") {
        return resolve_file_url(positional);
    }

    for scheme in ["ssh://", "scp://", "telnet://"] {
        if lower.starts_with(scheme) {
            let name = scheme.trim_end_matches("://");
            bail!("source-layer features don't support {name}:// URLs");
        }
    }

    // Default: treat as a local path. No auto-HTTPS promotion in
    // source-layer contexts.
    Ok(SourceKind::File(PathBuf::from(positional)))
}

fn resolve_file_url(raw: &str) -> Result<SourceKind> {
    // Strip the scheme manually rather than using the `url` crate —
    // url::Url normalises path components (e.g. `..`) in ways we don't
    // want here, and we only need straightforward string surgery.
    let rest = &raw[7..]; // drop "file://"
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host_ok = host.is_empty() || host.eq_ignore_ascii_case("localhost");
    if !host_ok {
        bail!(
            "file:// sources must use an empty host or 'localhost'; got '{host}'"
        );
    }
    if path.is_empty() {
        bail!("file:// URL '{raw}' has no path component");
    }
    Ok(SourceKind::File(PathBuf::from(path)))
}

#[cfg(test)]
thread_local! {
    static STDIN_IS_PIPE_OVERRIDE: std::cell::Cell<Option<bool>> =
        const { std::cell::Cell::new(None) };
}

/// Returns true when stdin appears to be a pipe (not a terminal). The
/// `#[cfg(test)]` override lets tests control the answer deterministically.
fn stdin_is_pipe() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = STDIN_IS_PIPE_OVERRIDE.with(|c| c.get()) {
            return v;
        }
    }
    use std::io::IsTerminal;
    !std::io::stdin().is_terminal()
}

/// Open a streaming reader for the given source.
/// Filled out in Task 2 of the source-layer plan.
pub fn open(source: SourceKind, args: &Args) -> Result<Box<dyn Read>> {
    let _ = (source, args);
    Err(anyhow!("source::open not yet implemented"))
}

/// Convenience: `resolve` + `open` + `read_to_end` into a `Vec<u8>`.
/// Filled out in Task 2 of the source-layer plan.
pub fn read_all(args: &Args) -> Result<Vec<u8>> {
    let _ = args;
    Err(anyhow!("source::read_all not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args_with_url(url: &str) -> Args {
        Args::try_parse_from(["recon", url]).unwrap()
    }

    fn args_without_url() -> Args {
        // --netstatus is in required_unless_present_any, so clap accepts it
        // without a positional URL and target_url() returns "".
        Args::try_parse_from(["recon", "--netstatus"]).unwrap()
    }

    fn with_stdin_pipe<T>(is_pipe: bool, f: impl FnOnce() -> T) -> T {
        STDIN_IS_PIPE_OVERRIDE.with(|c| c.set(Some(is_pipe)));
        let out = f();
        STDIN_IS_PIPE_OVERRIDE.with(|c| c.set(None));
        out
    }

    #[test]
    fn resolve_stdin_when_dash() {
        let args = args_with_url("-");
        assert_eq!(resolve(&args).unwrap(), SourceKind::Stdin);
    }

    #[test]
    fn resolve_stdin_when_empty_and_pipe() {
        let args = args_without_url();
        with_stdin_pipe(true, || {
            assert_eq!(resolve(&args).unwrap(), SourceKind::Stdin);
        });
    }

    #[test]
    fn resolve_errors_when_empty_and_tty() {
        let args = args_without_url();
        with_stdin_pipe(false, || {
            let err = resolve(&args).unwrap_err().to_string();
            assert!(err.contains("no input source"), "got: {err}");
        });
    }

    #[test]
    fn resolve_http_scheme_https() {
        let args = args_with_url("https://example.com/x");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::Http("https://example.com/x".into()),
        );
    }

    #[test]
    fn resolve_http_scheme_http() {
        let args = args_with_url("http://example.com/x");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::Http("http://example.com/x".into()),
        );
    }

    #[test]
    fn resolve_http_scheme_case_insensitive() {
        let args = args_with_url("HTTPS://example.com/x");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::Http("HTTPS://example.com/x".into()),
        );
    }

    #[test]
    fn resolve_file_scheme_empty_host() {
        let args = args_with_url("file:///tmp/foo.bin");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::File(PathBuf::from("/tmp/foo.bin")),
        );
    }

    #[test]
    fn resolve_file_scheme_localhost_host() {
        let args = args_with_url("file://localhost/tmp/foo.bin");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::File(PathBuf::from("/tmp/foo.bin")),
        );
    }

    #[test]
    fn resolve_file_scheme_localhost_case_insensitive() {
        let args = args_with_url("file://LocalHost/tmp/foo.bin");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::File(PathBuf::from("/tmp/foo.bin")),
        );
    }

    #[test]
    fn resolve_file_scheme_rejects_other_host() {
        let args = args_with_url("file://other.example/tmp/foo");
        let err = resolve(&args).unwrap_err().to_string();
        assert!(err.contains("empty host or 'localhost'"), "got: {err}");
        assert!(err.contains("other.example"), "got: {err}");
    }

    #[test]
    fn resolve_file_scheme_errors_on_missing_path() {
        let args = args_with_url("file://");
        let err = resolve(&args).unwrap_err().to_string();
        assert!(err.contains("no path component"), "got: {err}");
    }

    #[test]
    fn resolve_rejects_ssh_scheme() {
        let args = args_with_url("ssh://server/file");
        let err = resolve(&args).unwrap_err().to_string();
        assert!(err.contains("ssh://"), "got: {err}");
    }

    #[test]
    fn resolve_rejects_scp_scheme() {
        let args = args_with_url("scp://server/file");
        let err = resolve(&args).unwrap_err().to_string();
        assert!(err.contains("scp://"), "got: {err}");
    }

    #[test]
    fn resolve_rejects_telnet_scheme() {
        let args = args_with_url("telnet://server");
        let err = resolve(&args).unwrap_err().to_string();
        assert!(err.contains("telnet://"), "got: {err}");
    }

    #[test]
    fn resolve_treats_bare_word_as_file() {
        let args = args_with_url("example.com");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::File(PathBuf::from("example.com")),
        );
    }

    #[test]
    fn resolve_treats_absolute_path_as_file() {
        let args = args_with_url("/var/log/messages");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::File(PathBuf::from("/var/log/messages")),
        );
    }

    #[test]
    fn resolve_treats_relative_path_as_file() {
        let args = args_with_url("./relative/path.bin");
        assert_eq!(
            resolve(&args).unwrap(),
            SourceKind::File(PathBuf::from("./relative/path.bin")),
        );
    }
}
