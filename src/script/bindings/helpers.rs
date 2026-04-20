//! Core script helpers — `sleep_ms`, `env`, `now`, `now_ms`, `assert`.
//!
//! `print` is provided by Rhai's default engine; we don't re-register it.

use rhai::{Engine, EvalAltResult};
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
}
