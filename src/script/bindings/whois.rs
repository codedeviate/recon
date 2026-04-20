//! `whois(host)` script binding. Auto-discovers the authoritative WHOIS
//! server via IANA then follows a registrar-WHOIS referral if present,
//! matching the CLI's --whois behaviour. Returns `#{ host, server, body }`.

use crate::script::convert::anyhow_to_rhai;
use crate::whois as core;
use rhai::{Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine) {
    engine.register_fn("whois", |host: &str| -> Result<Map, Box<EvalAltResult>> {
        let r = core::probe(host).map_err(anyhow_to_rhai)?;
        let mut m = Map::new();
        m.insert("host".into(), r.host.into());
        m.insert("server".into(), r.server.into());
        m.insert("body".into(), r.body.into());
        Ok(m)
    });
}
