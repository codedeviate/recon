//! `ldap(url)` / `ldaps(url)` script binding. Anonymous bind + RootDSE
//! query. Returns `#{ url, connect_ms, attrs: #{ "namingContexts": [...],
//! "supportedLDAPVersion": [...], ... } }`. Invalid-credentials failures
//! exit 67; connect refused = 7; timeout = 28.

use crate::ldap_probe;
use crate::script::convert::{anyhow_to_rhai, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    for scheme in ["ldap", "ldaps"] {
        let prefix = format!("{scheme}://");
        let d = defaults.clone();
        let prefix_clone = prefix.clone();
        engine.register_fn(
            scheme,
            move |url: &str| -> Result<Map, Box<EvalAltResult>> {
                do_ldap(url, &prefix_clone, d.connect_timeout)
            },
        );
        let d = defaults.clone();
        let prefix_clone = prefix.clone();
        engine.register_fn(
            scheme,
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_ldap(url, &prefix_clone, timeout)
            },
        );
    }
}

fn do_ldap(url: &str, prefix: &str, timeout_secs: u64) -> Result<Map, Box<EvalAltResult>> {
    let full = if url.contains("://") {
        url.to_string()
    } else {
        format!("{prefix}{url}")
    };
    let r = ldap_probe::probe(&full, timeout_secs).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("url".into(), r.display_url.into());
    m.insert("connect_ms".into(), r.connect_ms.into());

    let mut attrs_map = Map::new();
    for (k, values) in &r.attrs {
        let arr: Array = values.iter().map(|s| Dynamic::from(s.clone())).collect();
        attrs_map.insert(k.as_str().into(), arr.into());
    }
    m.insert("attrs".into(), attrs_map.into());
    Ok(m)
}
