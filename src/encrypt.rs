//! `--encrypt` / `--decrypt`: age-format encryption over any source,
//! with passphrase or X25519 recipient-based modes. Includes
//! `--encrypt-keygen` for generating a fresh X25519 key pair.

use anyhow::{anyhow, Context, Result};
use secrecy::{ExposeSecret, Secret};
use std::path::Path;

// ---- Passphrase resolution --------------------------------------------

/// Test seam: when set via `set_prompt_override`, `prompt_passphrase`
/// returns this value instead of calling rpassword. Allows tests to
/// exercise the prompt branch without a real TTY.
#[cfg(test)]
thread_local! {
    static PROMPT_OVERRIDE: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn set_prompt_override(v: Option<&str>) {
    PROMPT_OVERRIDE.with(|slot| {
        *slot.borrow_mut() = v.map(|s| s.to_string());
    });
}

/// Prompt for a passphrase using the OS's TTY. In tests, returns the
/// value set by `set_prompt_override`, or errors if not set.
fn prompt_passphrase(confirm: bool) -> Result<String> {
    #[cfg(test)]
    {
        let maybe = PROMPT_OVERRIDE.with(|slot| slot.borrow().clone());
        if let Some(v) = maybe {
            return Ok(v);
        }
        return Err(anyhow!("prompt override not set in test"));
    }
    #[cfg(not(test))]
    {
        let first = rpassword::prompt_password("Passphrase: ")
            .map_err(|e| anyhow!(
                "no passphrase source; set --passphrase-file <PATH>, $RECON_PASSPHRASE, or run with a terminal ({e})"
            ))?;
        if confirm {
            let second = rpassword::prompt_password("Confirm passphrase: ")
                .map_err(|e| anyhow!("passphrase confirm prompt failed: {e}"))?;
            if first != second {
                return Err(anyhow!("passphrases do not match"));
            }
        }
        Ok(first)
    }
}

/// Resolve the passphrase using the priority from the spec:
///   1. --passphrase-file <PATH>
///   2. $RECON_PASSPHRASE env var
///   3. interactive hidden prompt (with optional confirm for encrypt)
pub fn resolve_passphrase(
    passphrase_file: Option<&Path>,
    confirm: bool,
) -> Result<Secret<String>> {
    if let Some(path) = passphrase_file {
        let bytes = std::fs::read(path)
            .with_context(|| format!("failed to read passphrase file '{}'", path.display()))?;
        let s = String::from_utf8(bytes)
            .map_err(|_| anyhow!("passphrase file '{}' is not valid UTF-8", path.display()))?;
        let trimmed = if s.ends_with('\n') { &s[..s.len() - 1] } else { &s[..] };
        if trimmed.is_empty() {
            return Err(anyhow!("passphrase file '{}' is empty", path.display()));
        }
        return Ok(Secret::new(trimmed.to_string()));
    }
    if let Ok(v) = std::env::var("RECON_PASSPHRASE") {
        if !v.is_empty() {
            return Ok(Secret::new(v));
        }
    }
    let prompted = prompt_passphrase(confirm)?;
    if prompted.is_empty() {
        return Err(anyhow!("passphrase cannot be empty"));
    }
    Ok(Secret::new(prompted))
}

// ---- Recipient / identity resolution — stubs filled in Task 3 --------

pub fn resolve_recipients(_values: &[String]) -> Result<Vec<Box<dyn age::Recipient + Send>>> {
    Err(anyhow!("resolve_recipients not yet implemented"))
}

pub fn resolve_identities(_paths: &[std::path::PathBuf]) -> Result<Vec<Box<dyn age::Identity>>> {
    Err(anyhow!("resolve_identities not yet implemented"))
}

// ---- Execution stubs — filled in Tasks 4 and 5 -----------------------

pub fn run_encrypt(_args: &crate::cli::Args) -> Result<()> {
    Err(anyhow!("run_encrypt not yet implemented"))
}

pub fn run_decrypt(_args: &crate::cli::Args) -> Result<()> {
    Err(anyhow!("run_decrypt not yet implemented"))
}

pub fn run_keygen(_args: &crate::cli::Args) -> Result<()> {
    Err(anyhow!("run_keygen not yet implemented"))
}

pub fn run(_args: &crate::cli::Args) -> Result<()> {
    Err(anyhow!("encrypt::run not yet implemented"))
}

// Suppress unused-import warnings for the yet-to-wire pieces.
#[allow(dead_code)]
fn _ensure_exposesecret_used(s: &Secret<String>) -> &str {
    s.expose_secret()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    /// Helper: write `content` to a tmp file and return its path.
    fn write_tmp(name: &str, content: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "recon-encrypt-test-{}-{}.bin",
            std::process::id(),
            name,
        ));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    #[test]
    fn passphrase_from_file() {
        let path = write_tmp("pp1", b"hunter2\n");
        let sec = resolve_passphrase(Some(&path), false).unwrap();
        assert_eq!(sec.expose_secret(), "hunter2");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn passphrase_file_no_trailing_newline() {
        let path = write_tmp("pp2", b"hunter2");
        let sec = resolve_passphrase(Some(&path), false).unwrap();
        assert_eq!(sec.expose_secret(), "hunter2");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn passphrase_file_empty_errors() {
        let path = write_tmp("pp3", b"\n");
        let err = resolve_passphrase(Some(&path), false).unwrap_err().to_string();
        assert!(err.contains("empty"), "got: {err}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn passphrase_file_missing_errors() {
        let path = PathBuf::from("/tmp/recon-encrypt-does-not-exist");
        let err = resolve_passphrase(Some(&path), false).unwrap_err().to_string();
        assert!(err.contains("failed to read"), "got: {err}");
    }

    #[test]
    fn passphrase_from_env_when_file_absent() {
        let _guard = env_mutex().lock().unwrap();
        std::env::set_var("RECON_PASSPHRASE", "envpass");
        let sec = resolve_passphrase(None, false).unwrap();
        assert_eq!(sec.expose_secret(), "envpass");
        std::env::remove_var("RECON_PASSPHRASE");
    }

    #[test]
    fn passphrase_file_beats_env() {
        let _guard = env_mutex().lock().unwrap();
        let path = write_tmp("pp4", b"filepass");
        std::env::set_var("RECON_PASSPHRASE", "envpass");
        let sec = resolve_passphrase(Some(&path), false).unwrap();
        assert_eq!(sec.expose_secret(), "filepass");
        std::env::remove_var("RECON_PASSPHRASE");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn passphrase_from_prompt_when_neither_set() {
        let _guard = env_mutex().lock().unwrap();
        std::env::remove_var("RECON_PASSPHRASE");
        set_prompt_override(Some("promptpass"));
        let sec = resolve_passphrase(None, false).unwrap();
        assert_eq!(sec.expose_secret(), "promptpass");
        set_prompt_override(None);
    }

    #[test]
    fn passphrase_empty_prompt_errors() {
        let _guard = env_mutex().lock().unwrap();
        std::env::remove_var("RECON_PASSPHRASE");
        set_prompt_override(Some(""));
        let err = resolve_passphrase(None, false).unwrap_err().to_string();
        assert!(err.contains("empty"), "got: {err}");
        set_prompt_override(None);
    }

    fn env_mutex() -> &'static std::sync::Mutex<()> {
        use std::sync::OnceLock;
        static M: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        M.get_or_init(|| std::sync::Mutex::new(()))
    }
}
