//! Core script helpers — `sleep_ms`, `env`, `now`, `now_ms`, `assert`,
//! `json_parse`, `json_stringify`.
//!
//! `print` is provided by Rhai's default engine; we don't re-register it.

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use serde_json::Value as JsonValue;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn register(engine: &mut Engine) {
    // sleep_ms(n) — block the current thread.
    engine.register_fn("sleep_ms", |ms: i64| {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms as u64));
        }
    });

    // env(NAME) — returns empty string if unset.
    engine.register_fn("env", |name: &str| -> String {
        std::env::var(name).unwrap_or_default()
    });

    // env(NAME, DEFAULT) — returns the fallback if unset.
    engine.register_fn("env", |name: &str, default: &str| -> String {
        std::env::var(name).unwrap_or_else(|_| default.to_string())
    });

    // now() — unix seconds.
    engine.register_fn("now", || -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    });

    // now_ms() — unix milliseconds.
    engine.register_fn("now_ms", || -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    });

    // assert(cond, msg) — throws a Rhai error when cond is false.
    engine.register_fn(
        "assert",
        |cond: bool, msg: &str| -> Result<(), Box<EvalAltResult>> {
            if cond {
                Ok(())
            } else {
                Err(format!("assertion failed: {msg}").into())
            }
        },
    );

    // json_parse(s) — parse a JSON string into a Rhai Dynamic.
    engine.register_fn("json_parse", |s: &str| -> Result<Dynamic, Box<EvalAltResult>> {
        let v: JsonValue = serde_json::from_str(s)
            .map_err(|e| Box::<EvalAltResult>::from(format!("json_parse: {e}")))?;
        Ok(json_to_dynamic(v))
    });

    // json_stringify(value) — serialise a Rhai Dynamic to JSON text.
    engine.register_fn("json_stringify", |v: Dynamic| -> Result<String, Box<EvalAltResult>> {
        let jv = dynamic_to_json(&v)?;
        serde_json::to_string(&jv)
            .map_err(|e| Box::<EvalAltResult>::from(format!("json_stringify: {e}")))
    });
}

fn json_to_dynamic(v: JsonValue) -> Dynamic {
    match v {
        JsonValue::Null => Dynamic::UNIT,
        JsonValue::Bool(b) => b.into(),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into()
            } else if let Some(f) = n.as_f64() {
                f.into()
            } else {
                Dynamic::UNIT
            }
        }
        JsonValue::String(s) => s.into(),
        JsonValue::Array(arr) => {
            let a: Array = arr.into_iter().map(json_to_dynamic).collect();
            a.into()
        }
        JsonValue::Object(obj) => {
            let mut m = Map::new();
            for (k, v) in obj {
                m.insert(k.into(), json_to_dynamic(v));
            }
            m.into()
        }
    }
}

fn dynamic_to_json(v: &Dynamic) -> Result<JsonValue, Box<EvalAltResult>> {
    if v.is_unit() {
        return Ok(JsonValue::Null);
    }
    if let Ok(b) = v.as_bool() {
        return Ok(JsonValue::Bool(b));
    }
    if let Ok(i) = v.as_int() {
        return Ok(JsonValue::Number(i.into()));
    }
    if let Ok(f) = v.as_float() {
        return serde_json::Number::from_f64(f)
            .map(JsonValue::Number)
            .ok_or_else(|| Box::<EvalAltResult>::from("json_stringify: non-finite float"));
    }
    if v.is_string() {
        return Ok(JsonValue::String(
            v.clone().into_string().unwrap_or_default(),
        ));
    }
    if v.is_array() {
        let arr = v
            .clone()
            .into_array()
            .map_err(|_| Box::<EvalAltResult>::from("json_stringify: array cast failed"))?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            out.push(dynamic_to_json(&item)?);
        }
        return Ok(JsonValue::Array(out));
    }
    if v.is_map() {
        let m = v
            .clone()
            .try_cast::<Map>()
            .ok_or_else(|| Box::<EvalAltResult>::from("json_stringify: map cast failed"))?;
        let mut obj = serde_json::Map::new();
        for (k, val) in m {
            obj.insert(k.to_string(), dynamic_to_json(&val)?);
        }
        return Ok(JsonValue::Object(obj));
    }
    Err(format!("json_stringify: unsupported type {}", v.type_name()).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        register(&mut e);
        e
    }

    #[test]
    fn sleep_ms_does_not_error() {
        let e = engine();
        e.eval::<()>("sleep_ms(5);").expect("sleep ok");
    }

    #[test]
    fn env_with_default_returns_default() {
        let e = engine();
        let v: String = e
            .eval(r#"env("RECON_TEST_UNDEFINED_XYZ_000", "fallback")"#)
            .expect("eval");
        assert_eq!(v, "fallback");
    }

    #[test]
    fn env_without_default_returns_empty() {
        let e = engine();
        let v: String = e
            .eval(r#"env("RECON_TEST_UNDEFINED_XYZ_000")"#)
            .expect("eval");
        assert_eq!(v, "");
    }

    #[test]
    fn env_reads_process_env() {
        // Safe: PATH is virtually always set on Unix.
        std::env::set_var("RECON_TEST_ENV_VAR_SET_BY_TEST", "hello");
        let e = engine();
        let v: String = e
            .eval(r#"env("RECON_TEST_ENV_VAR_SET_BY_TEST")"#)
            .expect("eval");
        assert_eq!(v, "hello");
        std::env::remove_var("RECON_TEST_ENV_VAR_SET_BY_TEST");
    }

    #[test]
    fn now_is_recent() {
        let e = engine();
        let t: i64 = e.eval("now()").expect("eval");
        let expected = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!((t - expected).abs() < 5, "now() = {t}, expected ~{expected}");
    }

    #[test]
    fn now_ms_is_larger_than_now() {
        let e = engine();
        let t: i64 = e.eval("now()").expect("eval");
        let tm: i64 = e.eval("now_ms()").expect("eval");
        assert!(tm > t, "now_ms ({tm}) should be > now ({t})");
    }

    #[test]
    fn assert_true_is_noop() {
        let e = engine();
        e.eval::<()>(r#"assert(true, "should not fire")"#)
            .expect("assert(true) ok");
    }

    #[test]
    fn assert_false_throws() {
        let e = engine();
        let res = e.eval::<()>(r#"assert(false, "boom")"#);
        let err = res.expect_err("assert(false) must throw");
        assert!(err.to_string().contains("boom"), "err: {err}");
    }

    #[test]
    fn json_parse_object() {
        let e = engine();
        let script = r#"
let o = json_parse(`{"a": 1, "b": "two", "c": [1, 2, 3], "d": null}`);
assert(o.a == 1, "a");
assert(o.b == "two", "b");
assert(o.c.len() == 3, "c");
o
"#;
        let m: rhai::Map = e.eval(script).expect("eval");
        assert_eq!(m.get("a").unwrap().as_int().unwrap(), 1);
    }

    #[test]
    fn json_stringify_round_trip() {
        let e = engine();
        let script = r#"
let o = #{ name: "recon", n: 42, ok: true, tags: ["net", "cli"] };
let s = json_stringify(o);
let back = json_parse(s);
back
"#;
        let m: rhai::Map = e.eval(script).expect("eval");
        assert_eq!(
            m.get("name").unwrap().clone().into_string().unwrap(),
            "recon"
        );
        assert_eq!(m.get("n").unwrap().as_int().unwrap(), 42);
        assert_eq!(m.get("ok").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn json_parse_malformed_throws() {
        let e = engine();
        let res: Result<Dynamic, _> = e.eval(r#"json_parse("{ not valid")"#);
        assert!(res.is_err());
    }
}
