//! Core script helpers — `sleep_ms`, `env`, `env_all`, `load_dotenv`,
//! `now`, `now_ms`, `assert`, `json_parse`, `json_stringify`.
//!
//! `print` is provided by Rhai's default engine; we don't re-register it.

use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
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

    // env_all() — snapshot every process env var as a map.
    // Aliased as `envAll` for camelCase callers.
    engine.register_fn("env_all", env_all);
    engine.register_fn("envAll", env_all);

    // load_dotenv(PATH) — parse a .env file and set each KEY=VALUE in
    // the process environment. Default semantics: OVERRIDE existing
    // values, so `load_dotenv(".env"); load_dotenv(".env.script")`
    // layers correctly (later loads win). Returns the count of vars set.
    //
    // load_dotenv(PATH, OVERRIDE) — explicit control over override.
    // Pass `false` to leave pre-existing env vars alone (shell-env wins).
    //
    // NOTE: `std::env::set_var` is technically unsound under concurrent
    // reads on some platforms. Call `load_dotenv` at the top of a
    // script — before spawning threads (`spawn`, `parallel`, etc.) —
    // so the env mutation happens while the script is single-threaded.
    //
    // Aliased as `loadDotEnv` for camelCase callers.
    engine.register_fn("load_dotenv", |path: &str| -> Result<i64, Box<EvalAltResult>> {
        load_dotenv_impl(path, true)
    });
    engine.register_fn(
        "load_dotenv",
        |path: &str, override_existing: bool| -> Result<i64, Box<EvalAltResult>> {
            load_dotenv_impl(path, override_existing)
        },
    );
    engine.register_fn("loadDotEnv", |path: &str| -> Result<i64, Box<EvalAltResult>> {
        load_dotenv_impl(path, true)
    });
    engine.register_fn(
        "loadDotEnv",
        |path: &str, override_existing: bool| -> Result<i64, Box<EvalAltResult>> {
            load_dotenv_impl(path, override_existing)
        },
    );

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

    // json_stringify(value) — compact JSON.
    engine.register_fn("json_stringify", |v: Dynamic| -> Result<String, Box<EvalAltResult>> {
        let jv = dynamic_to_json(&v)?;
        serde_json::to_string(&jv)
            .map_err(|e| Box::<EvalAltResult>::from(format!("json_stringify: {e}")))
    });

    // json_stringify(value, true) — 2-space pretty.
    // json_stringify(value, false) — same as compact form.
    engine.register_fn(
        "json_stringify",
        |v: Dynamic, pretty: bool| -> Result<String, Box<EvalAltResult>> {
            let jv = dynamic_to_json(&v)?;
            let s = if pretty {
                serde_json::to_string_pretty(&jv)
            } else {
                serde_json::to_string(&jv)
            };
            s.map_err(|e| Box::<EvalAltResult>::from(format!("json_stringify: {e}")))
        },
    );

    // json_stringify(value, n) — N-space pretty (1..=8 clamped).
    // n <= 0 falls back to compact so callers can feature-flag via
    // `json_stringify(v, is_pretty ? 4 : 0)`.
    engine.register_fn(
        "json_stringify",
        |v: Dynamic, indent: i64| -> Result<String, Box<EvalAltResult>> {
            let jv = dynamic_to_json(&v)?;
            if indent <= 0 {
                return serde_json::to_string(&jv).map_err(|e| {
                    Box::<EvalAltResult>::from(format!("json_stringify: {e}"))
                });
            }
            let width = indent.clamp(1, 8) as usize;
            let indent_buf = " ".repeat(width);
            let formatter =
                serde_json::ser::PrettyFormatter::with_indent(indent_buf.as_bytes());
            let mut buf = Vec::new();
            let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
            serde::Serialize::serialize(&jv, &mut ser).map_err(|e| {
                Box::<EvalAltResult>::from(format!("json_stringify: {e}"))
            })?;
            String::from_utf8(buf)
                .map_err(|e| Box::<EvalAltResult>::from(format!("json_stringify: {e}")))
        },
    );
}

fn env_all() -> Map {
    let mut m = Map::new();
    for (k, v) in std::env::vars() {
        m.insert(k.into(), v.into());
    }
    m
}

fn load_dotenv_impl(path: &str, override_existing: bool) -> Result<i64, Box<EvalAltResult>> {
    let p = PathBuf::from(path);
    let iter = dotenvy::from_path_iter(&p)
        .map_err(|e| err(format!("load_dotenv: open '{}': {e}", p.display())))?;
    let mut count: i64 = 0;
    for item in iter {
        let (k, v) = item
            .map_err(|e| err(format!("load_dotenv: parse '{}': {e}", p.display())))?;
        if override_existing || std::env::var_os(&k).is_none() {
            std::env::set_var(&k, &v);
            count += 1;
        }
    }
    Ok(count)
}

pub(crate) fn json_to_dynamic(v: JsonValue) -> Dynamic {
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

pub(crate) fn dynamic_to_json(v: &Dynamic) -> Result<JsonValue, Box<EvalAltResult>> {
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
    fn env_all_includes_a_set_var() {
        std::env::set_var("RECON_TEST_ENV_ALL_KEY", "rainbow");
        let e = engine();
        let v: String = e
            .eval(r#"env_all()["RECON_TEST_ENV_ALL_KEY"]"#)
            .expect("eval");
        assert_eq!(v, "rainbow");
        std::env::remove_var("RECON_TEST_ENV_ALL_KEY");
    }

    #[test]
    fn env_all_camelcase_alias_works() {
        std::env::set_var("RECON_TEST_ENVALL_ALIAS", "yes");
        let e = engine();
        let v: String = e
            .eval(r#"envAll()["RECON_TEST_ENVALL_ALIAS"]"#)
            .expect("eval");
        assert_eq!(v, "yes");
        std::env::remove_var("RECON_TEST_ENVALL_ALIAS");
    }

    fn write_tempfile(contents: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().expect("tempfile");
        f.write_all(contents.as_bytes()).expect("write");
        f
    }

    #[test]
    fn load_dotenv_sets_env_vars() {
        std::env::remove_var("RECON_TEST_DOTENV_K1");
        let f = write_tempfile("RECON_TEST_DOTENV_K1=hello-from-file\n");
        let path = f.path().to_string_lossy().into_owned();
        let e = engine();
        let n: i64 = e
            .eval(&format!(r#"load_dotenv({path:?})"#))
            .expect("eval");
        assert_eq!(n, 1);
        assert_eq!(
            std::env::var("RECON_TEST_DOTENV_K1").ok().as_deref(),
            Some("hello-from-file")
        );
        std::env::remove_var("RECON_TEST_DOTENV_K1");
    }

    #[test]
    fn load_dotenv_default_overrides_existing() {
        std::env::set_var("RECON_TEST_DOTENV_OVERRIDE", "shell-value");
        let f = write_tempfile("RECON_TEST_DOTENV_OVERRIDE=file-value\n");
        let path = f.path().to_string_lossy().into_owned();
        let e = engine();
        let _: i64 = e
            .eval(&format!(r#"load_dotenv({path:?})"#))
            .expect("eval");
        assert_eq!(
            std::env::var("RECON_TEST_DOTENV_OVERRIDE").ok().as_deref(),
            Some("file-value")
        );
        std::env::remove_var("RECON_TEST_DOTENV_OVERRIDE");
    }

    #[test]
    fn load_dotenv_with_false_does_not_override() {
        std::env::set_var("RECON_TEST_DOTENV_NO_OVERRIDE", "shell-value");
        let f = write_tempfile("RECON_TEST_DOTENV_NO_OVERRIDE=file-value\n");
        let path = f.path().to_string_lossy().into_owned();
        let e = engine();
        let n: i64 = e
            .eval(&format!(r#"load_dotenv({path:?}, false)"#))
            .expect("eval");
        assert_eq!(n, 0);
        assert_eq!(
            std::env::var("RECON_TEST_DOTENV_NO_OVERRIDE").ok().as_deref(),
            Some("shell-value")
        );
        std::env::remove_var("RECON_TEST_DOTENV_NO_OVERRIDE");
    }

    #[test]
    fn load_dotenv_layered_common_then_specific() {
        std::env::remove_var("RECON_TEST_LAYERED_SHARED");
        std::env::remove_var("RECON_TEST_LAYERED_COMMON_ONLY");
        std::env::remove_var("RECON_TEST_LAYERED_SPECIFIC_ONLY");
        let common = write_tempfile(
            "RECON_TEST_LAYERED_SHARED=common\n\
             RECON_TEST_LAYERED_COMMON_ONLY=c\n",
        );
        let specific = write_tempfile(
            "RECON_TEST_LAYERED_SHARED=specific\n\
             RECON_TEST_LAYERED_SPECIFIC_ONLY=s\n",
        );
        let cp = common.path().to_string_lossy().into_owned();
        let sp = specific.path().to_string_lossy().into_owned();
        let e = engine();
        e.eval::<i64>(&format!(r#"load_dotenv({cp:?})"#)).expect("common");
        e.eval::<i64>(&format!(r#"load_dotenv({sp:?})"#)).expect("specific");
        assert_eq!(
            std::env::var("RECON_TEST_LAYERED_SHARED").ok().as_deref(),
            Some("specific")
        );
        assert_eq!(
            std::env::var("RECON_TEST_LAYERED_COMMON_ONLY").ok().as_deref(),
            Some("c")
        );
        assert_eq!(
            std::env::var("RECON_TEST_LAYERED_SPECIFIC_ONLY").ok().as_deref(),
            Some("s")
        );
        std::env::remove_var("RECON_TEST_LAYERED_SHARED");
        std::env::remove_var("RECON_TEST_LAYERED_COMMON_ONLY");
        std::env::remove_var("RECON_TEST_LAYERED_SPECIFIC_ONLY");
    }

    #[test]
    fn load_dotenv_missing_file_errors() {
        let e = engine();
        let res: Result<i64, _> = e.eval(r#"load_dotenv("/no/such/path/.env-xyz")"#);
        let err = res.expect_err("missing file must error");
        let msg = err.to_string();
        assert!(msg.contains("load_dotenv"), "err: {msg}");
        assert!(msg.contains("/no/such/path/.env-xyz"), "err: {msg}");
    }

    #[test]
    fn load_dotenv_camelcase_alias_works() {
        std::env::remove_var("RECON_TEST_DOTENV_CAMEL");
        let f = write_tempfile("RECON_TEST_DOTENV_CAMEL=ok\n");
        let path = f.path().to_string_lossy().into_owned();
        let e = engine();
        let _: i64 = e
            .eval(&format!(r#"loadDotEnv({path:?})"#))
            .expect("eval");
        assert_eq!(
            std::env::var("RECON_TEST_DOTENV_CAMEL").ok().as_deref(),
            Some("ok")
        );
        std::env::remove_var("RECON_TEST_DOTENV_CAMEL");
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

    #[test]
    fn json_stringify_pretty_bool() {
        let e = engine();
        let s: String = e
            .eval(r#"json_stringify(#{ a: 1, b: 2 }, true)"#)
            .expect("eval");
        // Pretty output has a newline between entries and 2-space indent.
        assert!(s.contains('\n'), "expected newlines, got: {s:?}");
        assert!(s.contains("  \"a\""), "expected 2-space indent, got: {s:?}");
    }

    #[test]
    fn json_stringify_pretty_false_equals_compact() {
        let e = engine();
        let pretty_false: String = e
            .eval(r#"json_stringify(#{ a: 1, b: 2 }, false)"#)
            .expect("eval false");
        let compact: String = e.eval(r#"json_stringify(#{ a: 1, b: 2 })"#).expect("eval compact");
        assert_eq!(pretty_false, compact);
    }

    #[test]
    fn json_stringify_pretty_integer_indent() {
        let e = engine();
        let s: String = e
            .eval(r#"json_stringify(#{ a: 1 }, 4)"#)
            .expect("eval");
        assert!(s.contains("    \"a\""), "expected 4-space indent, got: {s:?}");
    }

    #[test]
    fn json_stringify_zero_indent_is_compact() {
        let e = engine();
        let zero: String = e
            .eval(r#"json_stringify(#{ a: 1 }, 0)"#)
            .expect("eval 0");
        let compact: String = e.eval(r#"json_stringify(#{ a: 1 })"#).expect("eval compact");
        assert_eq!(zero, compact);
    }

    #[test]
    fn json_stringify_integer_indent_clamps() {
        let e = engine();
        // 100 clamps to 8.
        let s: String = e
            .eval(r#"json_stringify(#{ a: 1 }, 100)"#)
            .expect("eval");
        // 8-space indent expected.
        assert!(s.contains("        \"a\""), "expected 8-space indent, got: {s:?}");
    }
}
