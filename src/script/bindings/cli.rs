//! `args` and `flags` constants exposed to Rhai scripts.
//!
//! Both are pushed into the script's Scope via `Scope::push_constant`
//! (read-only from inside the script). Scripts can parameterise themselves
//! by reading `args[1..]` and branch on CLI state via `flags.insecure`,
//! `flags.headers`, etc.

use crate::cli::Args;
use rhai::{Array, Dynamic, Map};

/// Build the `args` array: `[script_name, ...script_args]`. Uses the
/// `--script` value as given (pre-resolution) so `args[0]` matches what
/// the user typed — "health" rather than "/Users/x/.recon/script/health.rhai".
pub fn build_args_array(args: &Args) -> Array {
    let mut out = Array::with_capacity(1 + args.script_args.len());
    let script_name = args
        .script
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    out.push(script_name.into());
    for a in &args.script_args {
        out.push(a.clone().into());
    }
    out
}

/// Build the `flags` map — a snapshot of CLI flags that affect script
/// behaviour. Missing optional values are `()` (Rhai unit) rather than
/// absent keys, so scripts can `if flags.user_agent != () {}` without
/// `contains_key` guards.
pub fn build_flags_map(args: &Args) -> Map {
    let mut m = Map::new();

    // Always-present fields.
    let headers: Array = args
        .header
        .iter()
        .map(|h| Dynamic::from(h.clone()))
        .collect();
    m.insert("headers".into(), headers.into());
    m.insert("insecure".into(), args.insecure.into());
    m.insert("connect_timeout".into(), (args.timeout as i64).into());
    m.insert("follow_redirects".into(), args.follow_redirects.into());
    m.insert("max_redirs".into(), (args.max_redirs as i64).into());
    m.insert("verbose".into(), (args.verbose as i64).into());
    m.insert("wait_time".into(), args.wait_time.into());
    m.insert("ping_count".into(), (args.ping_count as i64).into());
    m.insert("max_hops".into(), (args.max_hops as i64).into());

    // Optional scalars: () when unset.
    m.insert("max_time".into(), opt_f64(args.max_time));
    m.insert("user_agent".into(), opt_string(args.user_agent.as_deref()));
    m.insert("referer".into(), opt_string(args.referer.as_deref()));
    m.insert("user".into(), opt_string(args.user.as_deref()));
    m.insert("method".into(), opt_string(args.method.as_deref()));
    m.insert("data".into(), opt_string(args.data.as_deref()));
    m.insert(
        "output".into(),
        opt_string(
            args.output
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .as_deref(),
        ),
    );
    m.insert("tlsv12".into(), args.tlsv12.into());
    m.insert("tlsv13".into(), args.tlsv13.into());
    m.insert(
        "cacert".into(),
        opt_string(
            args.cacert
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .as_deref(),
        ),
    );
    m.insert("interface".into(), opt_string(args.interface.as_deref()));

    m
}

/// Same shape as `build_flags_map`, but built from `ScriptDefaults`
/// instead of `Args`. Used by the REPL's `:set` to rebuild the `flags`
/// scope binding after a mutation. Keys and value types must match
/// `build_flags_map` exactly so user Rhai code sees consistent shape
/// regardless of how the map was built.
pub fn build_flags_from_defaults(d: &crate::script::defaults::ScriptDefaults) -> Map {
    let mut m = Map::new();

    let headers: Array = d.headers.iter().map(|h| Dynamic::from(h.clone())).collect();
    m.insert("headers".into(), headers.into());
    m.insert("insecure".into(), d.insecure.into());
    m.insert("connect_timeout".into(), (d.connect_timeout as i64).into());
    m.insert("follow_redirects".into(), d.follow_redirects.into());
    m.insert("max_redirs".into(), (d.max_redirs as i64).into());
    m.insert("verbose".into(), (d.verbose as i64).into());
    m.insert("wait_time".into(), d.wait_time.into());
    m.insert("ping_count".into(), (d.ping_count as i64).into());
    m.insert("max_hops".into(), (d.max_hops as i64).into());

    m.insert("max_time".into(), opt_f64(d.max_time));
    m.insert("user_agent".into(), opt_string(d.user_agent.as_deref()));
    m.insert("referer".into(), opt_string(d.referer.as_deref()));
    m.insert("user".into(), opt_string(d.user.as_deref()));
    m.insert("method".into(), opt_string(d.method.as_deref()));
    m.insert("data".into(), opt_string(d.data.as_deref()));
    m.insert(
        "output".into(),
        opt_string(
            d.output
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .as_deref(),
        ),
    );
    m.insert("tlsv12".into(), d.tlsv12.into());
    m.insert("tlsv13".into(), d.tlsv13.into());
    m.insert(
        "cacert".into(),
        opt_string(
            d.cacert
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .as_deref(),
        ),
    );
    m.insert("interface".into(), opt_string(d.interface.as_deref()));

    m
}

fn opt_string(v: Option<&str>) -> Dynamic {
    match v {
        Some(s) => s.to_string().into(),
        None => Dynamic::UNIT,
    }
}

fn opt_f64(v: Option<f64>) -> Dynamic {
    match v {
        Some(f) => f.into(),
        None => Dynamic::UNIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> Args {
        Args::parse_with_script_split(argv.iter().map(|s| s.to_string())).unwrap()
    }

    #[test]
    fn args_array_starts_with_script_name() {
        let a = parse(&["recon", "--script", "health", "foo", "bar"]);
        let arr = build_args_array(&a);
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].clone().into_string().unwrap(), "health");
        assert_eq!(arr[1].clone().into_string().unwrap(), "foo");
        assert_eq!(arr[2].clone().into_string().unwrap(), "bar");
    }

    #[test]
    fn args_array_preserves_script_name_as_given() {
        // When the user types "health", args[0] is "health" even though
        // the runtime may later resolve it to ~/.recon/script/health.rhai.
        let a = parse(&["recon", "--script", "health"]);
        let arr = build_args_array(&a);
        assert_eq!(arr[0].clone().into_string().unwrap(), "health");
    }

    #[test]
    fn args_array_allows_hyphen_values() {
        let a = parse(&["recon", "--script", "foo.rhai", "-v", "--bar"]);
        let arr = build_args_array(&a);
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[1].clone().into_string().unwrap(), "-v");
        assert_eq!(arr[2].clone().into_string().unwrap(), "--bar");
    }

    #[test]
    fn flags_map_has_expected_defaults() {
        let a = parse(&["recon", "--script", "x"]);
        let m = build_flags_map(&a);
        assert_eq!(m.get("insecure").unwrap().as_bool().unwrap(), false);
        assert_eq!(m.get("connect_timeout").unwrap().as_int().unwrap(), 30);
        assert_eq!(m.get("follow_redirects").unwrap().as_bool().unwrap(), false);
        let headers = m
            .get("headers")
            .and_then(|v| v.clone().try_cast::<Array>())
            .unwrap();
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn flags_map_captures_headers_and_insecure() {
        let a = parse(&[
            "recon",
            "-H",
            "X-Foo: bar",
            "-H",
            "X-Baz: qux",
            "-k",
            "--script",
            "x",
        ]);
        let m = build_flags_map(&a);
        let headers = m
            .get("headers")
            .and_then(|v| v.clone().try_cast::<Array>())
            .unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(
            headers[0].clone().into_string().unwrap(),
            "X-Foo: bar"
        );
        assert_eq!(m.get("insecure").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn flags_map_optional_none_becomes_unit() {
        let a = parse(&["recon", "--script", "x"]);
        let m = build_flags_map(&a);
        assert!(m.get("user_agent").unwrap().is_unit());
        assert!(m.get("referer").unwrap().is_unit());
        assert!(m.get("max_time").unwrap().is_unit());
        assert!(m.get("data").unwrap().is_unit());
        assert!(m.get("output").unwrap().is_unit());
    }

    #[test]
    fn flags_map_optional_some_becomes_string() {
        let a = parse(&[
            "recon",
            "-A",
            "my-bot/1.0",
            "--max-time",
            "5.5",
            "--script",
            "x",
        ]);
        let m = build_flags_map(&a);
        assert_eq!(
            m.get("user_agent").unwrap().clone().into_string().unwrap(),
            "my-bot/1.0"
        );
        assert!((m.get("max_time").unwrap().as_float().unwrap() - 5.5).abs() < 1e-9);
    }
}
