//! Top-level `file://` URL dispatch. Reads the referenced local file and
//! writes its bytes to stdout, or to `-o <path>` when supplied. Makes
//! `recon file:///tmp/x` behave like `cat /tmp/x`, mirroring curl.

use anyhow::{Context, Result};
use std::io::Write;

use crate::cli::Args;
use crate::source::{resolve_file_url, SourceKind};

pub fn run(url: &str, args: &Args) -> Result<()> {
    let path = match resolve_file_url(url)? {
        SourceKind::File(p) => p,
        _ => unreachable!("resolve_file_url only returns SourceKind::File"),
    };

    let bytes = std::fs::read(&path)
        .with_context(|| format!("file: could not read '{}'", path.display()))?;

    if let Some(out) = &args.output {
        std::fs::write(out, &bytes)
            .with_context(|| format!("file: could not write '{}'", out.display()))?;
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(&bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args(argv: &[&str]) -> Args {
        Args::try_parse_from(argv).unwrap()
    }

    #[test]
    fn reads_file_to_output_path() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.txt");
        let dst = dir.path().join("out.txt");
        std::fs::write(&src, b"hello world").unwrap();

        let url = format!("file://{}", src.display());
        let dst_s = dst.to_string_lossy().to_string();
        let a = args(&["recon", &url, "-o", &dst_s]);

        run(&url, &a).unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello world");
    }

    #[test]
    fn rejects_non_local_host() {
        let a = args(&["recon", "file://other.example/tmp/foo"]);
        let err = run("file://other.example/tmp/foo", &a).unwrap_err();
        assert!(err.to_string().contains("empty host or 'localhost'"));
    }

    #[test]
    fn accepts_localhost_host() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.txt");
        let dst = dir.path().join("out.txt");
        std::fs::write(&src, b"x").unwrap();

        let url = format!("file://localhost{}", src.display());
        let dst_s = dst.to_string_lossy().to_string();
        let a = args(&["recon", &url, "-o", &dst_s]);

        run(&url, &a).unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), b"x");
    }

    #[test]
    fn missing_file_is_error() {
        let a = args(&["recon", "file:///nonexistent/path/xyz"]);
        let err = run("file:///nonexistent/path/xyz", &a).unwrap_err();
        assert!(err.to_string().starts_with("file: could not read"));
    }
}
