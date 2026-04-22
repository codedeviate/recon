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

    engine.register_static_module("encrypt", module.into());
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
}
