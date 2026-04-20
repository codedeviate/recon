//! `ping(host)` / `ping(host, count)` script binding.
//!
//! Returns `#{ protocol, host, resolved_ip, port, sent, received, loss_pct,
//! replies: [#{seq, ms}], min_ms, avg_ms, max_ms }`. Host may be `host` for
//! ICMP (requires privileges on Linux; works unprivileged on macOS) or
//! `host:port` for TCP ping. Default count inherits `--ping-count` from the
//! CLI (default 4).

use crate::ping as core;
use crate::script::convert::anyhow_to_rhai;
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("ping", move |host: &str| -> Result<Map, Box<EvalAltResult>> {
            do_ping(host, d.ping_count)
        });
    }
    engine.register_fn(
        "ping",
        move |host: &str, count: i64| -> Result<Map, Box<EvalAltResult>> {
            if count <= 0 {
                return Err("ping: count must be positive".into());
            }
            do_ping(host, count as u32)
        },
    );
}

fn do_ping(host: &str, count: u32) -> Result<Map, Box<EvalAltResult>> {
    let result = core::probe(host, count).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("protocol".into(), result.protocol.to_string().into());
    m.insert("host".into(), result.host.clone().into());
    if let Some(ip) = result.resolved_ip {
        m.insert("resolved_ip".into(), ip.to_string().into());
    }
    if let Some(p) = result.port {
        m.insert("port".into(), (p as i64).into());
    }
    m.insert("sent".into(), (result.sent as i64).into());
    m.insert("received".into(), (result.received as i64).into());
    m.insert("loss_pct".into(), (result.loss_pct as i64).into());

    let replies: Array = result
        .replies
        .iter()
        .map(|r| {
            let mut reply = Map::new();
            reply.insert("seq".into(), (r.seq as i64).into());
            reply.insert("ms".into(), r.ms.into());
            Dynamic::from(reply)
        })
        .collect();
    m.insert("replies".into(), replies.into());

    if let Some(v) = result.min_ms() {
        m.insert("min_ms".into(), v.into());
    }
    if let Some(v) = result.avg_ms() {
        m.insert("avg_ms".into(), v.into());
    }
    if let Some(v) = result.max_ms() {
        m.insert("max_ms".into(), v.into());
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use std::net::TcpListener;
    use std::thread;

    fn engine() -> Engine {
        let args = Args::try_parse_from(["recon", "--script", "/dev/null"]).unwrap();
        let defaults = ScriptDefaults::from_args(&args);
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e, defaults);
        e
    }

    #[test]
    fn tcp_ping_to_listener_reports_received() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // Accept once — tcp_ping uses a fresh connection per seq; for count=1 this suffices.
        let accept_thread = thread::spawn(move || {
            let _ = listener.accept();
        });
        let e = engine();
        let script = format!(r#"ping("127.0.0.1:{port}", 1)"#);
        let m: Map = e.eval(&script).expect("eval");
        assert_eq!(m.get("sent").unwrap().as_int().unwrap(), 1);
        assert_eq!(m.get("received").unwrap().as_int().unwrap(), 1);
        assert_eq!(
            m.get("protocol").unwrap().clone().into_string().unwrap(),
            "tcp"
        );
        accept_thread.join().unwrap();
    }

    #[test]
    fn tcp_ping_to_closed_port_reports_loss() {
        let e = engine();
        // 127.0.0.1:1 should be closed.
        let m: Map = e.eval(r#"ping("127.0.0.1:1", 1)"#).expect("eval");
        assert_eq!(m.get("sent").unwrap().as_int().unwrap(), 1);
        assert_eq!(m.get("received").unwrap().as_int().unwrap(), 0);
        assert_eq!(m.get("loss_pct").unwrap().as_int().unwrap(), 100);
    }

    #[test]
    fn ping_count_zero_errors() {
        let e = engine();
        let res = e.eval::<Map>(r#"ping("127.0.0.1", 0)"#);
        assert!(res.is_err());
    }
}
