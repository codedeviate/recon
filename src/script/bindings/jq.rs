//! `jq(value, filter)` and `jq_all(value, filter)` — apply a jq-style
//! filter to any Rhai Map / Array.
//!
//! `jq` returns the *first* result (or `()` if the filter yields none);
//! `jq_all` returns *all* results as an Array. The split avoids the
//! shape ambiguity of a single auto-shaping method.
//!
//! Backed by the `jaq` 3.x crate family. Dynamic ↔ serde_json
//! conversion reuses the helper that already backs `json_parse` and
//! `json_stringify` in `bindings/helpers.rs`.

use crate::script::bindings::helpers;
use crate::script::convert::err;
use jaq_core::{
    data,
    load::{Arena, File, Loader},
    unwrap_valr, Compiler, Ctx, Vars,
};
use jaq_json::Val;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

// Convenience alias for the compiled filter type we use throughout.
type JaqFilter = jaq_core::compile::Filter<jaq_core::Native<data::JustLut<Val>>>;

pub fn register(engine: &mut Engine) {
    // jq(Array, &str) — first result, or () if filter yields nothing.
    engine.register_fn(
        "jq",
        |a: Array, filter: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_filter(Dynamic::from(a), filter, FirstOnly::Yes)
        },
    );
    // jq(Map, &str) — same.
    engine.register_fn(
        "jq",
        |m: Map, filter: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_filter(Dynamic::from(m), filter, FirstOnly::Yes)
        },
    );

    // jq_all forms land in Task 5.
}

#[allow(dead_code)] // No variant used by jq_all, which lands in Task 5
#[derive(Copy, Clone)]
enum FirstOnly {
    Yes,
    No,
}

fn run_filter(
    input: Dynamic,
    filter_src: &str,
    mode: FirstOnly,
) -> Result<Dynamic, Box<EvalAltResult>> {
    // 1. Compile the filter.
    let filter = compile_filter(filter_src)?;

    // 2. Convert the Rhai input to a serde_json::Value, then produce compact
    //    JSON text and parse it into jaq's Val via hifijson.
    //
    // TODO: enable `jaq-json/serde` feature to use `serde_json::from_value::<Val>`
    // here instead of stringify-then-parse. Output path is unavoidable
    // (Val implements Deserialize but not Serialize in jaq-json 2.x).
    let json_val = helpers::dynamic_to_json(&input)
        .map_err(|e| err(format!("jq: input not JSON-compatible: {e}")))?;
    let json_bytes = json_val.to_string();
    let val_in: Val = jaq_json::read::parse_single(json_bytes.as_bytes())
        .map_err(|e| err(format!("jq: input serialise error: {e}")))?;

    // 3. Build execution context. For `JustLut<Val>`, `Data<'a>` is
    //    `&'a Lut<Native<JustLut<Val>>>`, i.e. a reference to the filter's
    //    own lookup table.
    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
    // `unwrap_valr` converts Exn → Error<Val>; Error<Val> implements Display.
    let mut results = filter.id.run((ctx, val_in)).map(unwrap_valr);

    match mode {
        FirstOnly::Yes => match results.next() {
            None => Ok(Dynamic::UNIT),
            Some(Ok(v)) => val_to_dynamic(v),
            Some(Err(e)) => Err(err(format!("jq: filter error: {e}"))),
        },
        FirstOnly::No => {
            // (not yet registered; Task 5)
            let mut out = Array::new();
            for r in results {
                let v = r.map_err(|e| err(format!("jq: filter error: {e}")))?;
                out.push(val_to_dynamic(v)?);
            }
            Ok(Dynamic::from(out))
        }
    }
}

fn compile_filter(src: &str) -> Result<JaqFilter, Box<EvalAltResult>> {
    // jaq 3.x compile pipeline: Loader → File → Compiler.
    // Pulls in core + std + json defs/funs so `select`, `map`, etc. are
    // available. The DataT is `JustLut<Val>` — the standard "static value
    // type, no global inputs" data kind.
    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let funs = jaq_core::funs::<data::JustLut<Val>>()
        .chain(jaq_std::funs::<data::JustLut<Val>>())
        .chain(jaq_json::funs::<data::JustLut<Val>>());

    let arena = Arena::default();
    let loader = Loader::new(defs);
    let file = File { code: src, path: () };

    let modules = loader.load(&arena, file).map_err(|errs| {
        let msg = errs
            .into_iter()
            .map(|(_, e)| format!("{e:?}"))
            .collect::<Vec<_>>()
            .join("; ");
        err(format!("jq: filter parse error: {msg}"))
    })?;

    let filter = Compiler::default()
        .with_funs(funs)
        .compile(modules)
        .map_err(|errs| {
            let msg = errs
                .into_iter()
                .map(|(_, e)| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("; ");
            err(format!("jq: filter compile error: {msg}"))
        })?;

    Ok(filter)
}

fn val_to_dynamic(v: Val) -> Result<Dynamic, Box<EvalAltResult>> {
    // Val::BStr (byte string) renders as b"..." via Display, which is not
    // valid JSON. The `tobytes` builtin and byte-manipulation filters can
    // produce BStr at runtime. Guard here so the caller gets a clear message
    // rather than a confusing serde parse error downstream.
    if matches!(&v, Val::BStr(_)) {
        return Err(err(
            "jq: output is a byte string (BStr); use a text-producing filter",
        ));
    }

    // Val doesn't implement serde::Serialize; use Display (compact JSON text)
    // then re-parse with serde_json into Dynamic. This round-trip is
    // unavoidable: Val implements Deserialize but not Serialize in jaq-json 2.x,
    // so there is no cheaper conversion for the output path.
    let json_str = v.to_string();
    let json_val: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| err(format!("jq: output deserialise error: {e}")))?;
    Ok(helpers::json_to_dynamic(json_val))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Dynamic, Engine};

    fn engine() -> Engine {
        let mut e = Engine::new();
        register(&mut e);
        e
    }

    #[test]
    fn jq_returns_first_result_on_array() {
        let e = engine();
        let s: String = e
            .eval(r#"["a", "b", "c"].jq(".[]")"#)
            .unwrap();
        assert_eq!(s, "a");
    }

    #[test]
    fn jq_returns_unit_when_filter_yields_nothing() {
        let e = engine();
        let r: Dynamic = e
            .eval(r#"[1, 2, 3].jq(".[] | select(. > 10)")"#)
            .unwrap();
        assert!(r.is::<()>());
    }

    #[test]
    fn jq_path_into_map() {
        let e = engine();
        let n: i64 = e
            .eval(r#"#{ a: #{ b: 42 } }.jq(".a.b")"#)
            .unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn jq_filter_parse_error_throws() {
        let e = engine();
        let err = e
            .eval::<Dynamic>(r#"[1, 2].jq("invalid syntax (")"#)
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("filter"));
    }

    #[test]
    fn jq_filter_runtime_error_message_is_human_readable() {
        // `true | length` is a valid filter (parses OK) but fails at runtime
        // because jaq rejects `length` on booleans. The error message must be
        // human-readable, not a Rust debug dump containing "Exn(".
        let e = engine();
        let result = e
            .eval::<Dynamic>(r#"[true].jq(".[0] | length")"#)
            .unwrap_err();
        let msg = result.to_string();
        assert!(
            !msg.contains("Exn("),
            "error message must not be a Rust debug blob: {msg}"
        );
        // jaq-json renders this as "true has no length"
        assert!(
            msg.contains("true") || msg.contains("length") || msg.contains("no"),
            "expected a jq-style error message, got: {msg}"
        );
    }

    #[test]
    fn jq_bstr_output_gives_clear_error() {
        // `tobytes` converts an array of byte values into a Val::BStr.
        // That cannot be rendered as JSON, so val_to_dynamic must return
        // a clear "byte string" error rather than a confusing serde parse fail.
        let e = engine();
        let result = e
            .eval::<Dynamic>(r#"[72, 101, 108, 108, 111].jq("tobytes")"#)
            .unwrap_err();
        let msg = result.to_string();
        assert!(
            msg.contains("byte string"),
            "expected 'byte string' in error, got: {msg}"
        );
    }
}
