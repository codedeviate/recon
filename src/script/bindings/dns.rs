//! `dns(host)` / `dns(host, types)` script binding.
//!
//! Returns `#{ host, records: #{ "A": [...], "AAAA": [...], ... }, errors:
//! #{ "TYPE": "message" }, duration_ms }`. Types default to the standard
//! set (A, AAAA, CNAME, MX, NS, TXT, SOA); pass an array like `["A"]` to
//! query a specific subset. `records[type]` is always present for
//! requested types and is an empty array when no records were returned.

use crate::dns as core;
use crate::script::convert::anyhow_to_rhai;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use std::time::Instant;

pub fn register(engine: &mut Engine) {
    engine.register_fn("dns", |host: &str| -> Result<Map, Box<EvalAltResult>> {
        do_dns(host, &[])
    });
    engine.register_fn(
        "dns",
        |host: &str, types: Array| -> Result<Map, Box<EvalAltResult>> {
            let types: Vec<String> = types
                .iter()
                .filter_map(|d| {
                    if d.is_string() {
                        Some(d.clone().into_string().unwrap_or_default())
                    } else {
                        None
                    }
                })
                .collect();
            do_dns(host, &types)
        },
    );
}

fn do_dns(host: &str, types: &[String]) -> Result<Map, Box<EvalAltResult>> {
    let t0 = Instant::now();
    let result = core::probe(host, types).map_err(anyhow_to_rhai)?;
    let duration_ms = t0.elapsed().as_millis() as i64;

    let mut records_map = Map::new();
    for (k, v) in &result.records {
        let arr: Array = v.iter().map(|s| Dynamic::from(s.clone())).collect();
        records_map.insert(k.as_str().into(), arr.into());
    }
    let mut errors_map = Map::new();
    for (k, v) in &result.errors {
        errors_map.insert(k.as_str().into(), v.clone().into());
    }

    let mut m = Map::new();
    m.insert("host".into(), result.host.into());
    m.insert("records".into(), records_map.into());
    m.insert("errors".into(), errors_map.into());
    m.insert("duration_ms".into(), duration_ms.into());
    Ok(m)
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
    fn dns_default_resolves_localhost() {
        let e = engine();
        let m: Map = e.eval(r#"dns("localhost")"#).expect("eval");
        let records = m
            .get("records")
            .and_then(|v| v.clone().try_cast::<Map>())
            .expect("records is a map");
        // localhost almost always has an A record
        let a = records
            .get("A")
            .and_then(|v| v.clone().try_cast::<Array>())
            .expect("A records is an array");
        assert!(
            !a.is_empty(),
            "expected localhost A records, got empty: {records:?}"
        );
    }

    #[test]
    fn dns_explicit_types_array() {
        let e = engine();
        let m: Map = e.eval(r#"dns("localhost", ["A"])"#).expect("eval");
        let records = m
            .get("records")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap();
        assert!(records.contains_key("A"));
    }
}
