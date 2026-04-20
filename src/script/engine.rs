//! Engine setup and script execution.

use crate::cli::Args;
use std::path::Path;

/// Execute a script file. Returns the process exit code.
pub fn run_file(path: &Path, _args: &Args) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read script '{}': {e}", path.display());
            return 1;
        }
    };

    let engine = build_engine();
    match engine.eval::<rhai::Dynamic>(&source) {
        Ok(val) => {
            if let Ok(n) = val.as_int() {
                // Rhai i64 — clamp to reasonable exit code range
                (n & 0xff) as i32
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

/// Build a fresh Rhai engine with no recon-specific bindings yet.
/// Task 2 will register helpers; subsequent tasks register protocol bindings.
pub fn build_engine() -> rhai::Engine {
    rhai::Engine::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_script(body: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(body.as_bytes()).expect("write");
        f
    }

    fn dummy_args() -> Args {
        use clap::Parser;
        Args::try_parse_from(["recon", "--script", "/dev/null"]).expect("parse")
    }

    #[test]
    fn returns_zero_for_empty_script() {
        let f = write_script("");
        let code = run_file(f.path(), &dummy_args());
        assert_eq!(code, 0);
    }

    #[test]
    fn return_integer_maps_to_exit_code() {
        let f = write_script("return 3;");
        let code = run_file(f.path(), &dummy_args());
        assert_eq!(code, 3);
    }

    #[test]
    fn missing_file_reports_error() {
        let code = run_file(Path::new("/nonexistent/path/xyz.rhai"), &dummy_args());
        assert_eq!(code, 1);
    }

    #[test]
    fn syntax_error_returns_one() {
        let f = write_script("this is not valid rhai @#$");
        let code = run_file(f.path(), &dummy_args());
        assert_eq!(code, 1);
    }
}
