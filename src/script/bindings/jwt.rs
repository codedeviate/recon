//! `jwt::*` static module. Wraps `src/jwt.rs` primitive functions
//! (`parse_input`, `sign_claims`, `check_token`).

use crate::jwt::{self, CheckConfig, InputKind};
use crate::script::bindings::helpers::json_to_dynamic;
use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Module};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    // jwt::view(token_or_json) -> Map { header, payload }
    let _ = module.set_native_fn(
        "view",
        |input: &str| -> Result<Map, Box<EvalAltResult>> {
            let parsed = jwt::parse_input(input).map_err(|e| err(e.to_string()))?;
            let mut out = Map::new();
            match parsed {
                InputKind::FullToken { token } => {
                    let (header, payload) = decode_unverified(&token)?;
                    out.insert("header".into(), json_to_dynamic(header));
                    out.insert("payload".into(), json_to_dynamic(payload));
                }
                InputKind::PartialToken { header, payload } => {
                    out.insert(
                        "header".into(),
                        json_to_dynamic(serde_json::Value::Object(header)),
                    );
                    out.insert(
                        "payload".into(),
                        json_to_dynamic(serde_json::Value::Object(payload)),
                    );
                }
                InputKind::Json(map) | InputKind::PayloadOnly(map) => {
                    out.insert(
                        "payload".into(),
                        json_to_dynamic(serde_json::Value::Object(map)),
                    );
                }
            }
            Ok(out)
        },
    );

    // jwt::sign(claims_map, secret) -> String (HS256 default)
    let _ = module.set_native_fn(
        "sign",
        |claims: Map, secret: &str| -> Result<String, Box<EvalAltResult>> {
            let value = dynamic_map_to_json(claims)?;
            let obj = value.as_object().cloned().ok_or_else(|| {
                err("jwt::sign: claims must be a Rhai map (object)")
            })?;
            jwt::sign_claims(&obj, secret, "HS256").map_err(|e| err(e.to_string()))
        },
    );

    // jwt::sign(claims_map, secret, algorithm) -> String
    let _ = module.set_native_fn(
        "sign",
        |claims: Map, secret: &str, alg: &str| -> Result<String, Box<EvalAltResult>> {
            let value = dynamic_map_to_json(claims)?;
            let obj = value.as_object().cloned().ok_or_else(|| {
                err("jwt::sign: claims must be a Rhai map (object)")
            })?;
            jwt::sign_claims(&obj, secret, alg).map_err(|e| err(e.to_string()))
        },
    );

    // jwt::validate(token, secret) -> Map { valid, checks, header?, payload? }
    let _ = module.set_native_fn(
        "validate",
        |token: &str, secret: &str| -> Result<Map, Box<EvalAltResult>> {
            let config = CheckConfig::default();
            let results = jwt::check_token(token, secret, &config)
                .map_err(|e| err(e.to_string()))?;
            Ok(validate_result_map(&results, token))
        },
    );

    engine.register_static_module("jwt", module.into());
}

fn validate_result_map(results: &[jwt::CheckResult], token: &str) -> Map {
    let mut m = Map::new();
    let all_passed = results.iter().all(|r| r.passed);
    m.insert("valid".into(), all_passed.into());
    let checks: Array = results
        .iter()
        .map(|r| {
            let mut c = Map::new();
            c.insert("name".into(), r.name.to_string().into());
            c.insert("passed".into(), r.passed.into());
            c.insert(
                "detail".into(),
                match &r.detail {
                    Some(s) => Dynamic::from(s.clone()),
                    None => Dynamic::UNIT,
                },
            );
            Dynamic::from(c)
        })
        .collect();
    m.insert("checks".into(), checks.into());
    if let Ok((header, payload)) = decode_unverified(token) {
        m.insert("header".into(), json_to_dynamic(header));
        m.insert("payload".into(), json_to_dynamic(payload));
    }
    m
}

/// Decode header + payload from a JWT without signature verification.
/// Used by `view` and `validate` to surface the parsed components.
fn decode_unverified(
    token: &str,
) -> Result<(serde_json::Value, serde_json::Value), Box<EvalAltResult>> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return Err(err("jwt: token must have at least header.payload"));
    }
    Ok((b64url_json(parts[0])?, b64url_json(parts[1])?))
}

fn b64url_json(segment: &str) -> Result<serde_json::Value, Box<EvalAltResult>> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|e| err(format!("jwt: base64url decode: {e}")))?;
    serde_json::from_slice(&bytes).map_err(|e| err(format!("jwt: json decode: {e}")))
}

/// Convert a Rhai Map into `serde_json::Value::Object(...)`.
fn dynamic_map_to_json(m: Map) -> Result<serde_json::Value, Box<EvalAltResult>> {
    let mut obj = serde_json::Map::new();
    for (k, v) in m {
        obj.insert(k.to_string(), dynamic_to_json(v)?);
    }
    Ok(serde_json::Value::Object(obj))
}

fn dynamic_to_json(v: Dynamic) -> Result<serde_json::Value, Box<EvalAltResult>> {
    use serde_json::Value;
    if v.is_unit() {
        return Ok(Value::Null);
    }
    if let Ok(b) = v.as_bool() {
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = v.as_int() {
        return Ok(Value::Number(i.into()));
    }
    if let Ok(f) = v.as_float() {
        return serde_json::Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| err("jwt: non-finite float in claims"));
    }
    if v.is_string() {
        return Ok(Value::String(v.into_string().unwrap_or_default()));
    }
    if v.is_array() {
        let arr = v
            .into_array()
            .map_err(|_| err("jwt: array cast failed"))?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            out.push(dynamic_to_json(item)?);
        }
        return Ok(Value::Array(out));
    }
    if v.is_map() {
        let m = v
            .try_cast::<Map>()
            .ok_or_else(|| err("jwt: map cast failed"))?;
        return dynamic_map_to_json(m);
    }
    Err(err(format!(
        "jwt: unsupported claims value type {}",
        v.type_name()
    )))
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
    fn sign_and_validate_round_trip() {
        let e = engine();
        let script = r#"
let claims = #{ sub: "alice", iat: 1700000000 };
let token = jwt::sign(claims, "secret-key");
let result = jwt::validate(token, "secret-key");
result.valid
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(ok);
    }

    #[test]
    fn validate_rejects_wrong_secret() {
        let e = engine();
        let script = r#"
let token = jwt::sign(#{ sub: "x" }, "s1");
let result = jwt::validate(token, "s2");
result.valid
"#;
        let ok: bool = e.eval(script).expect("eval");
        assert!(!ok);
    }

    #[test]
    fn view_decodes_header_and_payload() {
        let e = engine();
        let script = r#"
let token = jwt::sign(#{ sub: "bob" }, "s");
let v = jwt::view(token);
v.payload.sub
"#;
        let sub: String = e.eval(script).expect("eval");
        assert_eq!(sub, "bob");
    }

    #[test]
    fn sign_custom_algorithm() {
        let e = engine();
        let token: String = e
            .eval(r#"jwt::sign(#{ sub: "x" }, "s", "HS512")"#)
            .expect("eval");
        assert!(token.starts_with("eyJ"));
    }

    #[test]
    fn sign_with_bad_algorithm_throws() {
        let e = engine();
        let res: Result<String, _> =
            e.eval(r#"jwt::sign(#{ sub: "x" }, "s", "MD5")"#);
        assert!(res.is_err());
    }
}
