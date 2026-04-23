//! `ipfs(url [, opts])` — convenience wrapper that rewrites
//! `ipfs://` / `ipns://` to a gateway HTTP URL and dispatches through
//! the existing `http()` binding. `opts.gateway` overrides the default.
//!
//! Without this wrapper, scripts can still fetch IPFS content by
//! constructing the gateway URL manually:
//!   http(`https://ipfs.io/ipfs/${cid}`)
//! The binding just saves that ceremony.

use crate::ipfs;
use crate::script::convert::{err, opts_get_str};
use crate::script::defaults::ScriptDefaults;
use rhai::{Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, _defaults: ScriptDefaults) {
    engine.register_fn("ipfs_url", |url: &str| -> Result<String, Box<EvalAltResult>> {
        ipfs::rewrite_url(url, None)
            .ok_or_else(|| err(format!("ipfs_url: not an ipfs:// or ipns:// URL: '{url}'")))
    });
    engine.register_fn(
        "ipfs_url",
        |url: &str, opts: Map| -> Result<String, Box<EvalAltResult>> {
            let gateway = opts_get_str(&opts, "gateway");
            ipfs::rewrite_url(url, gateway.as_deref())
                .ok_or_else(|| err(format!("ipfs_url: not an ipfs:// or ipns:// URL: '{url}'")))
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        use crate::script::defaults::ScriptDefaults;
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e, dummy_defaults());
        e
    }

    fn dummy_defaults() -> ScriptDefaults {
        use crate::cli::Args;
        use clap::Parser;
        let args = Args::try_parse_from(["recon", "--script", "/dev/null"]).unwrap();
        ScriptDefaults::from_args(&args)
    }

    #[test]
    fn ipfs_url_default_gateway() {
        let e = engine();
        let s: String = e.eval(r#"ipfs_url("ipfs://bafy/path")"#).unwrap();
        assert_eq!(s, "https://ipfs.io/ipfs/bafy/path");
    }

    #[test]
    fn ipfs_url_custom_gateway() {
        let e = engine();
        let s: String = e
            .eval(r#"ipfs_url("ipfs://bafy", #{ gateway: "http://127.0.0.1:8080" })"#)
            .unwrap();
        assert_eq!(s, "http://127.0.0.1:8080/ipfs/bafy");
    }

    #[test]
    fn ipfs_url_rejects_non_ipfs() {
        let e = engine();
        let res: Result<String, _> = e.eval(r#"ipfs_url("https://example.com")"#);
        assert!(res.is_err());
    }
}
