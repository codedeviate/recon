//! `memcached(url)` / `memcached(url, opts)` script binding.
//!
//! Returns `#{ host, port, connect_ms, version, version_ms, stats: #{...} }`.
//! When URL path is `/stats`, the full parsed stats map is included.

use crate::memcached_probe;
use crate::script::convert::{anyhow_to_rhai, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn(
            "memcached",
            move |url: &str| -> Result<Map, Box<EvalAltResult>> {
                do_mc(url, d.connect_timeout)
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "memcached",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_mc(url, timeout)
            },
        );
    }
}

fn do_mc(url: &str, timeout_secs: u64) -> Result<Map, Box<EvalAltResult>> {
    let full = if url.contains("://") {
        url.to_string()
    } else {
        format!("memcached://{url}")
    };
    let r = memcached_probe::probe(&full, timeout_secs).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("host".into(), r.host.into());
    m.insert("port".into(), (r.port as i64).into());
    m.insert("connect_ms".into(), r.connect_ms.into());
    m.insert("version".into(), r.version_line.into());
    m.insert("version_ms".into(), r.version_ms.into());
    let mut stats_map = Map::new();
    for (k, v) in &r.stats {
        stats_map.insert(k.as_str().into(), v.clone().into());
    }
    m.insert("stats".into(), stats_map.into());
    Ok(m)
}
