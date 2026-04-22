//! `netstatus::*` static module. Runs a handful of network-reachability
//! probes and returns aggregated status. Wraps `src/netstatus.rs` probe
//! helpers directly.
//!
//! `check()` runs a default probe set (HTTPS to example.com + TCP to
//! 1.1.1.1:443 + TCP to 8.8.8.8:53). `probe_*` functions expose the
//! individual probe types for custom configurations.

use crate::netstatus;
use crate::script::convert::err;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Module};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    // netstatus::check() -> Map {
    //   status: "ONLINE" | "OFFLINE" | "DEGRADED",
    //   probes: [ { label, passed, detail } ],
    //   passed: i64, total: i64,
    // }
    let _ = module.set_native_fn("check", || -> Result<Map, Box<EvalAltResult>> {
        let results = default_probe_set();
        Ok(aggregate(results))
    });

    // netstatus::probe_http(url) -> Map { label, passed, detail }
    let _ = module.set_native_fn(
        "probe_http",
        |url: &str| -> Result<Map, Box<EvalAltResult>> {
            Ok(probe_to_map(netstatus::probe_http(url)))
        },
    );

    // netstatus::probe_tcp(host, port) -> Map
    let _ = module.set_native_fn(
        "probe_tcp",
        |host: &str, port: i64| -> Result<Map, Box<EvalAltResult>> {
            if !(0..=65535).contains(&port) {
                return Err(err(format!("netstatus: port {port} out of range")));
            }
            Ok(probe_to_map(netstatus::probe_tcp(host, port as u16)))
        },
    );

    engine.register_static_module("netstatus", module.into());
}

fn default_probe_set() -> Vec<netstatus::ProbeResult> {
    vec![
        netstatus::probe_http("https://example.com"),
        netstatus::probe_tcp("1.1.1.1", 443),
        netstatus::probe_tcp("8.8.8.8", 53),
    ]
}

fn aggregate(results: Vec<netstatus::ProbeResult>) -> Map {
    let mut m = Map::new();
    let total = results.len() as i64;
    let passed = results.iter().filter(|r| r.passed).count() as i64;
    m.insert(
        "status".into(),
        netstatus::overall_status(&results).to_string().into(),
    );
    m.insert("passed".into(), passed.into());
    m.insert("total".into(), total.into());
    let probes: Array = results
        .into_iter()
        .map(|r| Dynamic::from(probe_to_map(r)))
        .collect();
    m.insert("probes".into(), probes.into());
    m
}

fn probe_to_map(r: netstatus::ProbeResult) -> Map {
    let mut m = Map::new();
    m.insert("label".into(), r.label.into());
    m.insert("passed".into(), r.passed.into());
    m.insert("detail".into(), r.detail.into());
    m
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
    fn probe_tcp_invalid_port_throws() {
        let e = engine();
        let res: Result<Map, _> =
            e.eval(r#"netstatus::probe_tcp("127.0.0.1", 99999)"#);
        assert!(res.is_err());
    }

    #[test]
    fn probe_tcp_closed_port_reports_failure() {
        let e = engine();
        // 127.0.0.1:1 — almost never open.
        let m: Map = e
            .eval(r#"netstatus::probe_tcp("127.0.0.1", 1)"#)
            .expect("eval");
        assert_eq!(m.get("passed").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    #[ignore] // requires network
    fn check_returns_status_shape() {
        let e = engine();
        let m: Map = e.eval("netstatus::check()").expect("eval");
        assert!(m.contains_key("status"));
        assert!(m.contains_key("probes"));
    }
}
