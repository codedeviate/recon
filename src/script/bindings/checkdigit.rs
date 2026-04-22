//! `checkdigit::*` static module. Verifies + creates check digits for
//! the 80+ algorithms recon already ships (VAT per-country, ISBN,
//! EAN-13, Luhn, VIN, etc.).

use crate::checkdigit::{registry, Verdict};
use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Module};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    // checkdigit::verify(algo, input) -> bool
    let _ = module.set_native_fn(
        "verify",
        |algo: &str, input: &str| -> Result<bool, Box<EvalAltResult>> {
            let spec = registry::resolve(algo)
                .ok_or_else(|| err(format!("checkdigit: unknown algorithm '{algo}'")))?;
            let verdict = (spec.verify_fn)(input);
            Ok(matches!(verdict, Verdict::Valid { .. }))
        },
    );

    // checkdigit::inspect(algo, input) -> Map { valid, formatted?, detected?, reason? }
    let _ = module.set_native_fn(
        "inspect",
        |algo: &str, input: &str| -> Result<Map, Box<EvalAltResult>> {
            let spec = registry::resolve(algo)
                .ok_or_else(|| err(format!("checkdigit: unknown algorithm '{algo}'")))?;
            let verdict = (spec.verify_fn)(input);
            let mut m = Map::new();
            match verdict {
                Verdict::Valid {
                    formatted,
                    detected,
                    comment,
                } => {
                    m.insert("valid".into(), true.into());
                    m.insert("formatted".into(), formatted.into());
                    if !detected.is_empty() {
                        m.insert("detected".into(), detected.into());
                    }
                    if !comment.is_empty() {
                        m.insert("comment".into(), comment.into());
                    }
                }
                Verdict::Invalid { reason } => {
                    m.insert("valid".into(), false.into());
                    m.insert("reason".into(), reason.into());
                }
            }
            Ok(m)
        },
    );

    // checkdigit::create(algo, body) -> String
    let _ = module.set_native_fn(
        "create",
        |algo: &str, body: &str| -> Result<String, Box<EvalAltResult>> {
            let spec = registry::resolve(algo)
                .ok_or_else(|| err(format!("checkdigit: unknown algorithm '{algo}'")))?;
            (spec.create_fn)(body, false).map_err(|e| err(e.to_string()))
        },
    );

    // checkdigit::create_raw(algo, body) -> String  (digit only, no formatting)
    let _ = module.set_native_fn(
        "create_raw",
        |algo: &str, body: &str| -> Result<String, Box<EvalAltResult>> {
            let spec = registry::resolve(algo)
                .ok_or_else(|| err(format!("checkdigit: unknown algorithm '{algo}'")))?;
            (spec.create_fn)(body, true).map_err(|e| err(e.to_string()))
        },
    );

    // checkdigit::list() -> Array of Maps {canonical, aliases, description}
    let _ = module.set_native_fn("list", || -> Result<Array, Box<EvalAltResult>> {
        let mut out = Array::new();
        for spec in registry::SPECS {
            let mut m = Map::new();
            m.insert("canonical".into(), spec.canonical.to_string().into());
            let aliases: Array = spec
                .aliases
                .iter()
                .map(|a| Dynamic::from(a.to_string()))
                .collect();
            m.insert("aliases".into(), aliases.into());
            m.insert("description".into(), spec.description.to_string().into());
            out.push(Dynamic::from(m));
        }
        Ok(out)
    });

    engine.register_static_module("checkdigit", module.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn luhn_verify_valid_and_invalid() {
        let e = engine();
        let ok: bool = e
            .eval(r#"checkdigit::verify("luhn", "4111111111111111")"#)
            .expect("eval");
        assert!(ok);
        let bad: bool = e
            .eval(r#"checkdigit::verify("luhn", "4111111111111112")"#)
            .expect("eval");
        assert!(!bad);
    }

    #[test]
    fn unknown_algo_throws() {
        let e = engine();
        let res: Result<bool, _> =
            e.eval(r#"checkdigit::verify("does-not-exist-xyz", "1234")"#);
        assert!(res.is_err());
    }

    #[test]
    fn inspect_returns_map() {
        let e = engine();
        let m: Map = e
            .eval(r#"checkdigit::inspect("luhn", "4111111111111111")"#)
            .expect("eval");
        assert_eq!(m.get("valid").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn list_includes_luhn_and_isbn() {
        let e = engine();
        let arr: Array = e.eval("checkdigit::list()").expect("eval");
        let names: Vec<String> = arr
            .iter()
            .filter_map(|m| {
                m.clone()
                    .try_cast::<Map>()
                    .and_then(|m| m.get("canonical").cloned())
                    .and_then(|d| d.into_string().ok())
            })
            .collect();
        assert!(names.iter().any(|n| n == "luhn"));
        assert!(names.iter().any(|n| n.starts_with("isbn")));
    }
}
