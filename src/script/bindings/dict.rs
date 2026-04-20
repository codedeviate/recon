//! `dict(url)` script binding (RFC 2229).
//!
//! Returns `#{ host, port, banner, responses: [#{ command, lines: [String],
//! final_status: i64 }] }`. Bare `dict://host/` runs the server-info
//! aggregate (SHOW SERVER + SHOW DATABASES + SHOW STRATEGIES).

use crate::dict_probe;
use crate::script::convert::{anyhow_to_rhai, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("dict", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_dict(url, d.connect_timeout)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "dict",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_dict(url, timeout)
            },
        );
    }
}

fn do_dict(url: &str, timeout_secs: u64) -> Result<Map, Box<EvalAltResult>> {
    let r = dict_probe::probe(url, timeout_secs).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("host".into(), r.host.into());
    m.insert("port".into(), (r.port as i64).into());
    m.insert("banner".into(), r.banner.into());

    let responses: Array = r
        .responses
        .into_iter()
        .map(|resp| {
            let mut rm = Map::new();
            rm.insert("command".into(), resp.command.into());
            let lines: Array = resp.lines.into_iter().map(Dynamic::from).collect();
            rm.insert("lines".into(), lines.into());
            if let Some(s) = resp.final_status {
                rm.insert("final_status".into(), (s as i64).into());
            }
            Dynamic::from(rm)
        })
        .collect();
    m.insert("responses".into(), responses.into());
    Ok(m)
}
