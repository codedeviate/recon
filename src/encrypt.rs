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

pub fn resolve_recipients(values: &[String]) -> Result<Vec<Box<dyn age::Recipient + Send>>> {
    let mut out: Vec<Box<dyn age::Recipient + Send>> = Vec::new();
    for v in values {
        if let Some(rec) = parse_recipient_literal(v)? {
            out.push(Box::new(rec));
            continue;
        }
        // Otherwise, treat the value as a file path.
        let path = Path::new(v);
        if !path.exists() {
            return Err(anyhow!(
                "invalid recipient '{v}': not an age1... public key or a readable file"
            ));
        }
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read recipient file '{v}'"))?;
        let mut found = 0usize;
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let rec = parse_recipient_literal(trimmed)?
                .ok_or_else(|| anyhow!(
                    "invalid recipient key in '{v}': '{trimmed}'"
                ))?;
            out.push(Box::new(rec));
            found += 1;
        }
        if found == 0 {
            return Err(anyhow!(
                "recipient file '{v}' contains no age1... keys"
            ));
        }
    }
    Ok(out)
}

/// If `s` looks like an age1... literal public key, parse it; otherwise Ok(None).
fn parse_recipient_literal(s: &str) -> Result<Option<age::x25519::Recipient>> {
    if !s.starts_with("age1") {
        return Ok(None);
    }
    let rec: age::x25519::Recipient = s.parse()
        .map_err(|e| anyhow!("invalid recipient '{s}': {e}"))?;
    Ok(Some(rec))
}

pub fn resolve_identities(
    paths: &[std::path::PathBuf],
) -> Result<Vec<Box<dyn age::Identity>>> {
    let mut out: Vec<Box<dyn age::Identity>> = Vec::new();
    for path in paths {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read identity file '{}'", path.display()))?;
        for (i, line) in contents.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let id: age::x25519::Identity = trimmed.parse().map_err(|e| {
                anyhow!(
                    "invalid identity in '{}' at line {}: {e}",
                    path.display(),
                    i + 1,
                )
            })?;
            out.push(Box::new(id));
        }
    }
    Ok(out)
}

// ---- Execution stubs — filled in Tasks 4 and 5 -----------------------

/// Parameters for [`encrypt_streaming`] and [`decrypt_streaming`], assembled
/// from CLI args once the corresponding fields are added to `cli::Args` (Task 5).
pub struct EncryptParams<'a> {
    pub recipients: &'a [String],
    pub passphrase_file: Option<&'a std::path::Path>,
    pub armor: bool,
    pub verbose: u8,
    pub output: Option<&'a std::path::Path>,
}

pub struct DecryptParams<'a> {
    pub passphrase_file: Option<&'a std::path::Path>,
    pub identity_paths: &'a [std::path::PathBuf],
    pub verbose: u8,
    pub output: Option<&'a std::path::Path>,
}

fn source_label(kind: &crate::source::SourceKind) -> String {
    match kind {
        crate::source::SourceKind::Stdin => "stdin".to_string(),
        crate::source::SourceKind::File(p) => p.display().to_string(),
        crate::source::SourceKind::Http(u) => u.clone(),
    }
}

fn open_output_path(path: Option<&std::path::Path>) -> Result<Box<dyn std::io::Write>> {
    match path {
        Some(p) => Ok(Box::new(std::fs::File::create(p)?)),
        None => Ok(Box::new(std::io::stdout().lock())),
    }
}

/// Core encrypt logic: reads from `reader`, encrypts with age, writes to
/// `output` (stdout if `None`). Called by `run_encrypt` once cli fields exist.
pub fn encrypt_streaming(
    mut reader: impl std::io::Read,
    source_kind: &crate::source::SourceKind,
    params: &EncryptParams<'_>,
) -> Result<()> {
    use std::io::Write;

    // Decide: recipient mode or passphrase mode. If both are supplied,
    // recipient mode wins (v1 simplification).
    let encryptor = if !params.recipients.is_empty() {
        let recipients = resolve_recipients(params.recipients)?;
        age::Encryptor::with_recipients(
            recipients.iter().map(|b| b.as_ref() as &dyn age::Recipient),
        )
        .map_err(|e| anyhow!("encrypt: {e}"))?
    } else {
        let passphrase = resolve_passphrase(params.passphrase_file, true)?;
        age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(
            passphrase.expose_secret().to_string(),
        ))
    };

    if params.verbose >= 1 {
        let label = source_label(source_kind);
        eprintln!(
            "* encrypt: age {} ({})",
            if params.armor { "armored" } else { "binary" },
            label
        );
    }

    let mut out = open_output_path(params.output)?;

    if params.armor {
        let armored = age::armor::ArmoredWriter::wrap_output(
            &mut out,
            age::armor::Format::AsciiArmor,
        )
        .map_err(|e| anyhow!("armor: {e}"))?;
        let mut writer = encryptor
            .wrap_output(armored)
            .map_err(|e| anyhow!("encrypt: {e}"))?;
        std::io::copy(&mut reader, &mut writer)?;
        let armored = writer.finish().map_err(|e| anyhow!("encrypt finish: {e}"))?;
        armored.finish().map_err(|e| anyhow!("armor finish: {e}"))?;
    } else {
        let mut writer = encryptor
            .wrap_output(&mut out)
            .map_err(|e| anyhow!("encrypt: {e}"))?;
        std::io::copy(&mut reader, &mut writer)?;
        writer.finish().map_err(|e| anyhow!("encrypt finish: {e}"))?;
    }
    Ok(())
}

/// Core decrypt logic: reads ciphertext from `buf`, decrypts, writes plaintext.
/// Called by `run_decrypt` once cli fields exist.
pub fn decrypt_streaming(
    buf: &[u8],
    source_kind: &crate::source::SourceKind,
    params: &DecryptParams<'_>,
) -> Result<()> {
    use std::io::Write;

    // ArmoredReader transparently handles both armored and binary age files.
    let armored = age::armor::ArmoredReader::new(std::io::Cursor::new(buf));
    let decryptor = age::Decryptor::new_buffered(armored)
        .map_err(|e| anyhow!("decrypt header: {e}"))?;

    let mut plaintext_reader: Box<dyn std::io::Read> = if decryptor.is_scrypt() {
        // Passphrase-based decryption.
        let passphrase = resolve_passphrase(params.passphrase_file, false)?;
        let pp = age::secrecy::SecretString::from(passphrase.expose_secret().to_string());
        let identity = age::scrypt::Identity::new(pp);
        let r = decryptor
            .decrypt(std::iter::once(&identity as &dyn age::Identity))
            .map_err(|e| anyhow!("decryption failed: {e}"))?;
        Box::new(r)
    } else {
        // Recipient (X25519 / SSH / plugin) decryption.
        if params.identity_paths.is_empty() {
            return Err(anyhow!(
                "no matching identity for this payload; supply --identity or a passphrase"
            ));
        }
        let identities = resolve_identities(params.identity_paths)?;
        let id_refs: Vec<&dyn age::Identity> =
            identities.iter().map(|b| b.as_ref()).collect();
        let r = decryptor
            .decrypt(id_refs.into_iter())
            .map_err(|e| anyhow!("decryption failed: {e}"))?;
        Box::new(r)
    };

    if params.verbose >= 1 {
        let label = source_label(source_kind);
        eprintln!("* decrypt: age from {label}");
    }

    let mut out = open_output_path(params.output)?;
    std::io::copy(&mut plaintext_reader, &mut out)?;
    Ok(())
}

pub fn run_encrypt(args: &crate::cli::Args) -> Result<()> {
    let _ = args;
    Err(anyhow!("run_encrypt: pending CLI wiring (Task 5)"))
}

pub fn run_decrypt(args: &crate::cli::Args) -> Result<()> {
    let _ = args;
    Err(anyhow!("run_decrypt: pending CLI wiring (Task 5)"))
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

    fn write_text_tmp(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "recon-encrypt-text-{}-{}.txt",
            std::process::id(),
            name,
        ));
        std::fs::write(&path, content).unwrap();
        path
    }

    fn make_keypair() -> (age::x25519::Identity, String) {
        let id = age::x25519::Identity::generate();
        let pub_key = id.to_public().to_string();
        (id, pub_key)
    }

    #[test]
    fn recipients_literal_age1_key() {
        let (_, pub_key) = make_keypair();
        let recs = resolve_recipients(&[pub_key])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn recipients_from_file() {
        let (_, pub_key) = make_keypair();
        let path = write_text_tmp("recips1", &format!("# comment\n{pub_key}\n"));
        let recs = resolve_recipients(&[path.to_str().unwrap().to_string()])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(recs.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn recipients_empty_file_errors() {
        let path = write_text_tmp("recips2", "# only comments\n\n#\n");
        let err = resolve_recipients(&[path.to_str().unwrap().to_string()])
            .err()
            .expect("expected an error")
            .to_string();
        assert!(err.contains("no age1") || err.contains("no age"), "got: {err}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn recipients_missing_path_errors() {
        let err = resolve_recipients(&["/tmp/definitely-not-here.rec".to_string()])
            .err()
            .expect("expected an error")
            .to_string();
        assert!(err.contains("invalid recipient"), "got: {err}");
    }

    #[test]
    fn recipients_malformed_literal_errors() {
        let err = resolve_recipients(&["age1notvalid".to_string()])
            .err()
            .expect("expected an error")
            .to_string();
        assert!(err.contains("invalid recipient"), "got: {err}");
    }

    #[test]
    fn identities_from_file() {
        // Use a known well-formed age secret key so we don't need to call
        // expose_secret() on Identity::to_string()'s SecretString return value
        // (which comes from age_core::secrecy, a different secrecy version than
        // the one this crate depends on directly).
        const TEST_SK: &str =
            "AGE-SECRET-KEY-1GQ9778VQXMMJVE8SK7J6VT8UJ4HDQAJUVSFCWCM02D8GEWQ72PVQ2Y5J33";
        let path = write_text_tmp("id1", &format!("# my key\n{TEST_SK}\n"));
        let ids = resolve_identities(&[path.clone()])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(ids.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn identities_malformed_line_reports_line_number() {
        let path = write_text_tmp("id2", "# header\nNOT-AN-AGE-KEY\n");
        let err = resolve_identities(&[path.clone()])
            .err()
            .expect("expected an error")
            .to_string();
        assert!(err.contains("line 2"), "got: {err}");
        let _ = std::fs::remove_file(&path);
    }

    fn round_trip_passphrase(plaintext: &[u8], armor: bool) -> Vec<u8> {
        let pp = age::secrecy::SecretString::from("hunter2".to_string());
        let encryptor = age::Encryptor::with_user_passphrase(pp);

        let mut ciphertext: Vec<u8> = Vec::new();
        if armor {
            let armored = age::armor::ArmoredWriter::wrap_output(
                &mut ciphertext,
                age::armor::Format::AsciiArmor,
            )
            .unwrap();
            let mut writer = encryptor.wrap_output(armored).unwrap();
            writer.write_all(plaintext).unwrap();
            let armored = writer.finish().unwrap();
            armored.finish().unwrap();
        } else {
            let mut writer = encryptor.wrap_output(&mut ciphertext).unwrap();
            writer.write_all(plaintext).unwrap();
            writer.finish().unwrap();
        }

        // Decrypt using scrypt::Identity as the identity for passphrase mode.
        let armored =
            age::armor::ArmoredReader::new(std::io::Cursor::new(&ciphertext[..]));
        let decryptor = age::Decryptor::new_buffered(armored).unwrap();
        let pp2 = age::secrecy::SecretString::from("hunter2".to_string());
        let identity = age::scrypt::Identity::new(pp2);
        let mut reader = decryptor
            .decrypt(std::iter::once(&identity as &dyn age::Identity))
            .unwrap();
        let mut decrypted = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut decrypted).unwrap();
        decrypted
    }

    #[test]
    fn round_trip_passphrase_binary() {
        let pt = b"hello encryption";
        let got = round_trip_passphrase(pt, false);
        assert_eq!(got, pt);
    }

    #[test]
    fn round_trip_passphrase_armored() {
        let pt = b"hello encryption";
        let got = round_trip_passphrase(pt, true);
        assert_eq!(got, pt);
    }

    #[test]
    fn round_trip_x25519() {
        let id = age::x25519::Identity::generate();
        let pub_str = id.to_public().to_string();

        let recipient: age::x25519::Recipient = pub_str.parse().unwrap();
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
                .unwrap();
        let plaintext = b"x25519 payload";

        let mut ciphertext: Vec<u8> = Vec::new();
        let mut writer = encryptor.wrap_output(&mut ciphertext).unwrap();
        writer.write_all(plaintext).unwrap();
        writer.finish().unwrap();

        let armored =
            age::armor::ArmoredReader::new(std::io::Cursor::new(&ciphertext[..]));
        let decryptor = age::Decryptor::new_buffered(armored).unwrap();
        let ids: Vec<&dyn age::Identity> = vec![&id];
        let mut reader = decryptor.decrypt(ids.into_iter()).unwrap();
        let mut decrypted = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut decrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_passphrase_errors() {
        let pp = age::secrecy::SecretString::from("hunter2".to_string());
        let encryptor = age::Encryptor::with_user_passphrase(pp);
        let mut ciphertext: Vec<u8> = Vec::new();
        let mut writer = encryptor.wrap_output(&mut ciphertext).unwrap();
        writer.write_all(b"plaintext").unwrap();
        writer.finish().unwrap();

        let armored =
            age::armor::ArmoredReader::new(std::io::Cursor::new(&ciphertext[..]));
        let decryptor = age::Decryptor::new_buffered(armored).unwrap();
        let wrong = age::secrecy::SecretString::from("wrong".to_string());
        let identity = age::scrypt::Identity::new(wrong);
        let err = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity));
        assert!(err.is_err(), "expected decryption failure with wrong passphrase");
    }

    fn env_mutex() -> &'static std::sync::Mutex<()> {
        use std::sync::OnceLock;
        static M: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        M.get_or_init(|| std::sync::Mutex::new(()))
    }
}
