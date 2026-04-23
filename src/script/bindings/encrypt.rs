//! `encrypt::*` static module. Wraps age encryption.
//!
//! The `encrypt()` / `decrypt()` functions operate on in-memory blobs.
//! Recipients accept age1… literal strings OR paths to identity files
//! (same rules as `--recipient` on the CLI). `decrypt()` takes paths to
//! identity files — scripts that have a raw key string should write it
//! to a tempfile first.
//!
//! `keygen()` returns a fresh X25519 keypair as `#{ public, private }`.
//! Passphrase-based encrypt/decrypt is NOT exposed from scripts — the
//! CLI path prompts interactively and that's the wrong UX for scripts.
//! Script users needing passphrase-symmetric encryption can fall back
//! to running `recon --encrypt ...` via `agentBrowser::cmd` or similar.

use crate::encrypt;
use crate::script::convert::err;
use age::secrecy::ExposeSecret;
use rhai::{Array, Blob, Engine, EvalAltResult, Map, Module};
use std::path::PathBuf;

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    // encrypt::encrypt(plaintext_blob, recipients_array) -> Blob (binary)
    let _ = module.set_native_fn(
        "encrypt",
        |plaintext: Blob, recipients: Array| -> Result<Blob, Box<EvalAltResult>> {
            let recipients = array_to_strings(recipients)?;
            encrypt::encrypt_bytes_recipients(&plaintext, &recipients, false)
                .map_err(|e| err(e.to_string()))
        },
    );

    // encrypt::encrypt_armored(plaintext, recipients) -> Blob (ASCII armor)
    let _ = module.set_native_fn(
        "encrypt_armored",
        |plaintext: Blob, recipients: Array| -> Result<Blob, Box<EvalAltResult>> {
            let recipients = array_to_strings(recipients)?;
            encrypt::encrypt_bytes_recipients(&plaintext, &recipients, true)
                .map_err(|e| err(e.to_string()))
        },
    );

    // encrypt::decrypt(ciphertext_blob, identity_paths) -> Blob
    let _ = module.set_native_fn(
        "decrypt",
        |ciphertext: Blob, identities: Array| -> Result<Blob, Box<EvalAltResult>> {
            let paths: Vec<PathBuf> = array_to_strings(identities)?
                .into_iter()
                .map(PathBuf::from)
                .collect();
            encrypt::decrypt_bytes_identities(&ciphertext, &paths)
                .map_err(|e| err(e.to_string()))
        },
    );

    // encrypt::keygen() -> Map { public: "age1...", private: "AGE-SECRET-KEY-1..." }
    let _ = module.set_native_fn("keygen", || -> Result<Map, Box<EvalAltResult>> {
        let identity = age::x25519::Identity::generate();
        let public = identity.to_public().to_string();
        let private = identity.to_string().expose_secret().to_string();
        let mut m = Map::new();
        m.insert("public".into(), public.into());
        m.insert("private".into(), private.into());
        Ok(m)
    });

    // encrypt::rekey(ciphertext, old_identity_paths, new_recipients) -> Blob
    // Decrypts the input (age auto-detected) then re-encrypts to the new
    // recipient set. Binary-armored output; add `true` as a 4th arg for
    // ASCII armor. Works for age; PGP is not supported via script (use
    // encrypt::pgp_rekey via agentBrowser::cmd / shelling out).
    let _ = module.set_native_fn(
        "rekey",
        |ciphertext: Blob, old_identities: Array, new_recipients: Array| -> Result<Blob, Box<EvalAltResult>> {
            rekey_age(&ciphertext, old_identities, new_recipients, false)
        },
    );
    let _ = module.set_native_fn(
        "rekey",
        |ciphertext: Blob, old_identities: Array, new_recipients: Array, armor: bool| -> Result<Blob, Box<EvalAltResult>> {
            rekey_age(&ciphertext, old_identities, new_recipients, armor)
        },
    );

    // encrypt::pgp_encrypt(plaintext, recipients) -> Blob
    // Shells out to `gpg` (must be on PATH). Returns binary ciphertext;
    // use pgp_encrypt_armored for the ASCII form.
    let _ = module.set_native_fn(
        "pgp_encrypt",
        |plaintext: Blob, recipients: Array| -> Result<Blob, Box<EvalAltResult>> {
            let recipients = array_to_strings(recipients)?;
            encrypt::gpg_encrypt_bytes(&plaintext, &recipients, false)
                .map_err(|e| err(e.to_string()))
        },
    );
    let _ = module.set_native_fn(
        "pgp_encrypt_armored",
        |plaintext: Blob, recipients: Array| -> Result<Blob, Box<EvalAltResult>> {
            let recipients = array_to_strings(recipients)?;
            encrypt::gpg_encrypt_bytes(&plaintext, &recipients, true)
                .map_err(|e| err(e.to_string()))
        },
    );

    // encrypt::pgp_decrypt(ciphertext) -> Blob
    // Uses the user's gpg keyring. Passphrase-protected secret keys prompt
    // via gpg's usual pinentry unless the agent has unlocked them.
    let _ = module.set_native_fn(
        "pgp_decrypt",
        |ciphertext: Blob| -> Result<Blob, Box<EvalAltResult>> {
            encrypt::gpg_decrypt_bytes(&ciphertext, None)
                .map_err(|e| err(e.to_string()))
        },
    );

    // encrypt::detect_backend(recipient) -> "age" | "pgp"
    // Mirrors the CLI's auto-detection logic. Useful for scripts that
    // want to branch before calling encrypt::* vs encrypt::pgp_*.
    let _ = module.set_native_fn(
        "detect_backend",
        |recipient: &str| -> Result<String, Box<EvalAltResult>> {
            let t = recipient.trim();
            let backend =
                if t.starts_with("age1") || std::path::Path::new(t).exists() {
                    "age"
                } else {
                    "pgp"
                };
            Ok(backend.to_string())
        },
    );

    engine.register_static_module("encrypt", module.into());
}

fn rekey_age(
    ciphertext: &[u8],
    old_identities: Array,
    new_recipients: Array,
    armor: bool,
) -> Result<Blob, Box<EvalAltResult>> {
    let id_paths: Vec<PathBuf> = array_to_strings(old_identities)?
        .into_iter()
        .map(PathBuf::from)
        .collect();
    let recipients = array_to_strings(new_recipients)?;
    if recipients.is_empty() {
        return Err(err("encrypt::rekey: new_recipients must not be empty"));
    }
    let plaintext = encrypt::decrypt_bytes_age(ciphertext, &id_paths, None)
        .map_err(|e| err(format!("encrypt::rekey decrypt: {e}")))?;
    encrypt::encrypt_bytes_recipients(&plaintext, &recipients, armor)
        .map_err(|e| err(format!("encrypt::rekey encrypt: {e}")))
}

fn array_to_strings(arr: Array) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        if v.is_string() {
            out.push(v.into_string().unwrap_or_default());
        } else {
            return Err(err("encrypt: recipients / identities array must contain strings"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn keygen_returns_public_and_private() {
        let e = engine();
        let m: Map = e.eval("encrypt::keygen()").expect("eval");
        let pub_key = m.get("public").unwrap().clone().into_string().unwrap();
        let priv_key = m.get("private").unwrap().clone().into_string().unwrap();
        assert!(pub_key.starts_with("age1"));
        assert!(priv_key.starts_with("AGE-SECRET-KEY-"));
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        // Generate keypair; write identity to a tempfile so decrypt can
        // load it (the age API requires file-based identity loading for
        // X25519).
        let e = engine();
        let keys: Map = e.eval("encrypt::keygen()").unwrap();
        let pub_key = keys.get("public").unwrap().clone().into_string().unwrap();
        let priv_key = keys.get("private").unwrap().clone().into_string().unwrap();

        let id_file = NamedTempFile::new().unwrap();
        std::fs::write(id_file.path(), format!("{priv_key}\n")).unwrap();

        let script = format!(
            r#"
let plain = "the quick brown fox".to_blob();
let cipher = encrypt::encrypt(plain, ["{pub_key}"]);
assert(cipher.len() > 0, "ciphertext empty");
let back = encrypt::decrypt(cipher, ["{path}"]);
back == "the quick brown fox".to_blob()
"#,
            pub_key = pub_key,
            path = id_file.path().display()
        );
        let ok: bool = e.eval(&script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn empty_recipients_throws() {
        let e = engine();
        let res: Result<Blob, _> =
            e.eval(r#"encrypt::encrypt("x".to_blob(), [])"#);
        assert!(res.is_err());
    }

    #[test]
    fn rekey_age_round_trip() {
        let e = engine();
        // Generate two independent keypairs.
        let k1: Map = e.eval("encrypt::keygen()").unwrap();
        let k2: Map = e.eval("encrypt::keygen()").unwrap();
        let pub1 = k1.get("public").unwrap().clone().into_string().unwrap();
        let priv1 = k1.get("private").unwrap().clone().into_string().unwrap();
        let pub2 = k2.get("public").unwrap().clone().into_string().unwrap();
        let priv2 = k2.get("private").unwrap().clone().into_string().unwrap();

        let id1 = NamedTempFile::new().unwrap();
        std::fs::write(id1.path(), format!("{priv1}\n")).unwrap();
        let id2 = NamedTempFile::new().unwrap();
        std::fs::write(id2.path(), format!("{priv2}\n")).unwrap();

        let script = format!(
            r#"
let plain = "rotate this".to_blob();
let c1 = encrypt::encrypt(plain, ["{pub1}"]);
let c2 = encrypt::rekey(c1, ["{id1}"], ["{pub2}"]);
let back = encrypt::decrypt(c2, ["{id2}"]);
back == "rotate this".to_blob()
"#,
            pub1 = pub1,
            pub2 = pub2,
            id1 = id1.path().display(),
            id2 = id2.path().display(),
        );
        let ok: bool = e.eval(&script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn detect_backend_classifies() {
        let e = engine();
        let s1: String = e.eval(r#"encrypt::detect_backend("age1abcdef")"#).unwrap();
        assert_eq!(s1, "age");
        let s2: String = e.eval(r#"encrypt::detect_backend("0xDEADBEEF")"#).unwrap();
        assert_eq!(s2, "pgp");
        let s3: String = e.eval(r#"encrypt::detect_backend("alice@example.com")"#).unwrap();
        assert_eq!(s3, "pgp");
    }
}
