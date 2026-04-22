//! `email::*` static module — DNS-based email-security checks.
//!
//! Each function spins a throwaway current-thread tokio runtime, builds
//! a system-default hickory resolver inside it, and awaits the existing
//! async check. Simple and robust; the per-call runtime overhead is
//! tiny relative to the DNS roundtrips themselves.

use crate::email::{bimi, dkim, dmarc, mta_sts, spf, tls_rpt, CheckResult, Verdict};
use crate::script::convert::err;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Module};

pub fn register(engine: &mut Engine) {
    let mut module = Module::new();

    let _ = module.set_native_fn(
        "spf",
        |host: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            run_one(move |r| {
                let host = host.clone();
                Box::pin(async move { spf::check(&r, &host).await })
            })
        },
    );

    let _ = module.set_native_fn(
        "dmarc",
        |host: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            run_one(move |r| {
                let host = host.clone();
                Box::pin(async move { dmarc::check(&r, &host).await })
            })
        },
    );

    let _ = module.set_native_fn(
        "dkim",
        |host: &str, selector: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            let selector = selector.to_string();
            run_one(move |r| {
                let host = host.clone();
                let selector = selector.clone();
                Box::pin(async move { dkim::check(&r, &host, &selector).await })
            })
        },
    );

    let _ = module.set_native_fn(
        "mta_sts",
        |host: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            run_one(move |r| {
                let host = host.clone();
                Box::pin(async move { mta_sts::check(&r, &host, false).await })
            })
        },
    );

    let _ = module.set_native_fn(
        "bimi",
        |host: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            run_one(move |r| {
                let host = host.clone();
                Box::pin(async move { bimi::check(&r, &host, "default", false).await })
            })
        },
    );

    let _ = module.set_native_fn(
        "bimi",
        |host: &str, selector: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            let selector = selector.to_string();
            run_one(move |r| {
                let host = host.clone();
                let selector = selector.clone();
                Box::pin(async move { bimi::check(&r, &host, &selector, false).await })
            })
        },
    );

    let _ = module.set_native_fn(
        "tls_rpt",
        |host: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            run_one(move |r| {
                let host = host.clone();
                Box::pin(async move { tls_rpt::check(&r, &host).await })
            })
        },
    );

    // email::all(host) -> Map { spf, dmarc, mta_sts, tls_rpt, bimi } each a result map
    let _ = module.set_native_fn(
        "all",
        |host: &str| -> Result<Map, Box<EvalAltResult>> {
            let host = host.to_string();
            run_all(&host)
        },
    );

    engine.register_static_module("email", module.into());
}

type CheckFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<CheckResult>>>>;

fn run_one<F>(make_fut: F) -> Result<Map, Box<EvalAltResult>>
where
    F: FnOnce(TokioAsyncResolver) -> CheckFuture + 'static,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| err(format!("email: build runtime: {e}")))?;
    let r = rt.block_on(async move {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
        make_fut(resolver).await
    });
    match r {
        Ok(res) => Ok(result_to_map(res)),
        Err(e) => Err(err(format!("email: {e}"))),
    }
}

fn run_all(host: &str) -> Result<Map, Box<EvalAltResult>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| err(format!("email: build runtime: {e}")))?;
    let host_owned = host.to_string();
    let results = rt.block_on(async move {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
        let spf_r = spf::check(&resolver, &host_owned).await;
        let dmarc_r = dmarc::check(&resolver, &host_owned).await;
        let mta_r = mta_sts::check(&resolver, &host_owned, false).await;
        let tls_r = tls_rpt::check(&resolver, &host_owned).await;
        let bimi_r = bimi::check(&resolver, &host_owned, "default", false).await;
        [
            ("spf", spf_r),
            ("dmarc", dmarc_r),
            ("mta_sts", mta_r),
            ("tls_rpt", tls_r),
            ("bimi", bimi_r),
        ]
    });
    let mut m = Map::new();
    for (key, res) in results {
        let dyn_map = match res {
            Ok(r) => Dynamic::from(result_to_map(r)),
            Err(e) => {
                let mut em = Map::new();
                em.insert("error".into(), e.to_string().into());
                Dynamic::from(em)
            }
        };
        m.insert(key.into(), dyn_map);
    }
    Ok(m)
}

fn result_to_map(r: CheckResult) -> Map {
    let mut m = Map::new();
    m.insert("name".into(), r.name.into());
    m.insert(
        "verdict".into(),
        match r.verdict {
            Verdict::Pass => "pass",
            Verdict::Warn => "warn",
            Verdict::Fail => "fail",
        }
        .to_string()
        .into(),
    );
    m.insert("summary".into(), r.summary.into());
    let details: Array = r
        .details
        .into_iter()
        .map(|d| {
            let mut dm = Map::new();
            dm.insert("text".into(), d.text.into());
            if let Some(v) = d.verdict {
                dm.insert(
                    "verdict".into(),
                    match v {
                        Verdict::Pass => "pass",
                        Verdict::Warn => "warn",
                        Verdict::Fail => "fail",
                    }
                    .to_string()
                    .into(),
                );
            }
            Dynamic::from(dm)
        })
        .collect();
    m.insert("details".into(), details.into());
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_to_map_shape() {
        let r = CheckResult {
            name: "SPF".into(),
            verdict: Verdict::Pass,
            summary: "ok".into(),
            details: vec![crate::email::Detail::new("detail line")],
        };
        let m = result_to_map(r);
        assert_eq!(m.get("name").unwrap().clone().into_string().unwrap(), "SPF");
        assert_eq!(
            m.get("verdict").unwrap().clone().into_string().unwrap(),
            "pass"
        );
    }
}
