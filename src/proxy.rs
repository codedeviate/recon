//! Proxy plumbing: `--proxy`, `--proxy-user`, `--noproxy`,
//! `--proxy-insecure`, `--proxy-cacert`. Routes HTTP(S) requests
//! through HTTP / HTTPS / SOCKS5 proxies via reqwest's `Proxy` API.
//!
//! Environment-variable precedence (matches curl):
//!   - `https://` target → `$HTTPS_PROXY` (or `$https_proxy`)
//!   - `http://` target  → `$HTTP_PROXY`  (or `$http_proxy`)
//!   - either            → `$ALL_PROXY`   (or `$all_proxy`)
//!
//! `$NO_PROXY` (or `$no_proxy`) provides a bypass list; `--noproxy`
//! overrides it.
//!
//! Scheme routing on the proxy URL:
//!   http://proxy:8080      → HTTP proxy (plaintext CONNECT)
//!   https://proxy:8443     → HTTPS-to-proxy (TLS before CONNECT)
//!   socks5://proxy:1080    → SOCKS5 with remote DNS
//!   socks5h://proxy:1080   → SOCKS5 forcing hostname (client-side DNS)

use crate::cli::Args;
use anyhow::{Context, Result};
use reqwest::Proxy;

/// Resolve proxy configuration from CLI flags + env vars. Returns the
/// `Proxy` to install on the `ClientBuilder`, or `None` to skip.
pub fn build_proxy_from_args(args: &Args) -> Result<Option<Proxy>> {
    let target_scheme = detect_target_scheme(args);
    let (url, source) = resolve_proxy_url(args, target_scheme);
    let Some(url) = url else {
        return Ok(None);
    };

    // Parse the URL to let us inject auth + classify scheme.
    let parsed = url::Url::parse(&url)
        .with_context(|| format!("proxy: malformed URL '{url}' (from {source})"))?;
    let (user, pass) = resolve_proxy_credentials(args, &parsed);

    // reqwest's Proxy::all covers every target scheme. We could use
    // Proxy::http / https for finer control but `all` matches curl's
    // default routing semantics.
    let mut proxy = Proxy::all(&url)
        .with_context(|| format!("proxy: reqwest rejected '{url}'"))?;

    if let Some(u) = user {
        proxy = proxy.basic_auth(&u, pass.as_deref().unwrap_or(""));
    }

    // Bypass list.
    let bypass = resolve_noproxy(args);
    if !bypass.is_empty() {
        let matcher = bypass.clone();
        proxy = proxy.custom_http_auth(reqwest::header::HeaderValue::from_static(""))
            .no_proxy(reqwest::NoProxy::from_string(&matcher));
    }

    Ok(Some(proxy))
}

fn detect_target_scheme(args: &Args) -> &'static str {
    let url = args.target_url();
    if url.starts_with("https://") {
        "https"
    } else {
        "http"
    }
}

fn resolve_proxy_url(args: &Args, target_scheme: &str) -> (Option<String>, &'static str) {
    if let Some(explicit) = args.proxy.as_deref() {
        return (Some(explicit.to_string()), "--proxy");
    }
    let candidate = match target_scheme {
        "https" => env_first(&["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]),
        _ => env_first(&["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"]),
    };
    match candidate {
        Some(v) => (Some(v), "env"),
        None => (None, "none"),
    }
}

fn env_first(names: &[&str]) -> Option<String> {
    for n in names {
        if let Ok(v) = std::env::var(n) {
            if !v.trim().is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn resolve_proxy_credentials(args: &Args, parsed: &url::Url) -> (Option<String>, Option<String>) {
    // --proxy-user takes priority over URL userinfo.
    if let Some(cred) = args.proxy_user.as_deref() {
        let (u, p) = match cred.split_once(':') {
            Some((u, p)) => (u.to_string(), Some(p.to_string())),
            None => (cred.to_string(), None),
        };
        return (Some(u), p);
    }
    let user = parsed.username();
    if !user.is_empty() {
        return (
            Some(user.to_string()),
            parsed.password().map(|s| s.to_string()),
        );
    }
    (None, None)
}

fn resolve_noproxy(args: &Args) -> String {
    if let Some(v) = args.noproxy.as_deref() {
        return v.to_string();
    }
    env_first(&["NO_PROXY", "no_proxy"]).unwrap_or_default()
}

/// Apply TLS overrides relevant to the proxy (`--proxy-insecure`,
/// `--proxy-cacert`). reqwest 0.12 doesn't expose per-proxy TLS
/// configuration, so these layer into the global ClientBuilder TLS
/// config. Documented in the --help proxy topic.
pub fn apply_proxy_tls(
    mut builder: reqwest::blocking::ClientBuilder,
    args: &Args,
) -> Result<reqwest::blocking::ClientBuilder> {
    if args.proxy_pass.is_some() {
        eprintln!(
            "warning: --proxy-pass: passphrase support for proxy mTLS is not yet \
             exposed by reqwest 0.12. The flag is accepted but has no effect. \
             See OUT-OF-SCOPE.md (Deferred)."
        );
    }
    if args.proxy_insecure {
        builder = builder.danger_accept_invalid_certs(true);
    }
    if let Some(path) = &args.proxy_cacert {
        let pem = std::fs::read(path)
            .with_context(|| format!("--proxy-cacert: read {}", path.display()))?;
        let cert = reqwest::Certificate::from_pem(&pem)
            .with_context(|| format!("--proxy-cacert: parse {}", path.display()))?;
        builder = builder.add_root_certificate(cert);
    }

    // --proxy-capath: trust every *.pem / *.crt / *.cer in the directory.
    // reqwest 0.12 has no per-proxy TLS roots; this adds to the global chain,
    // same as --capath — the "proxy" prefix is for curl-parity convention.
    if let Some(dir) = &args.proxy_capath {
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("--proxy-capath: read dir {}", dir.display()))?;
        let mut count = 0usize;
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext_ok = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| {
                    let lo = s.to_ascii_lowercase();
                    lo == "pem" || lo == "crt" || lo == "cer"
                })
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
            let pem = std::fs::read(&p)
                .with_context(|| format!("--proxy-capath: read {}", p.display()))?;
            let cert = reqwest::Certificate::from_pem(&pem)
                .with_context(|| format!("--proxy-capath: parse PEM from {}", p.display()))?;
            builder = builder.add_root_certificate(cert);
            count += 1;
        }
        if args.verbose >= 1 {
            eprintln!(
                "* proxy TLS: loaded {} cert(s) from --proxy-capath {}",
                count,
                dir.display()
            );
        }
    }

    // --proxy-ca-native: disable built-in webpki roots, leaving only OS native
    // roots. reqwest 0.12 applies this globally (server + proxy). Mirrors
    // --ca-native; both flags flip the same switch — separate for curl-parity.
    if args.proxy_ca_native {
        builder = builder.tls_built_in_root_certs(false);
        if args.verbose >= 1 {
            eprintln!(
                "* proxy TLS: --proxy-ca-native (webpki roots disabled, native roots only)"
            );
        }
    }

    Ok(builder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::sync::Mutex;

    // Serialise env-var mutations across parallel test threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn args_with_proxy(proxy: Option<&str>, url: &str) -> Args {
        let mut v = vec!["recon", url];
        if let Some(p) = proxy {
            v.insert(1, "--proxy");
            v.insert(2, p);
        }
        Args::try_parse_from(v).unwrap()
    }

    fn clear_proxy_env() {
        for n in [
            "HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy",
            "ALL_PROXY", "all_proxy", "NO_PROXY", "no_proxy",
        ] {
            std::env::remove_var(n);
        }
    }

    #[test]
    fn explicit_flag_wins_over_env() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        clear_proxy_env();
        std::env::set_var("HTTPS_PROXY", "http://env.example:8080");
        let args = args_with_proxy(Some("http://flag.example:3128"), "https://example.com/");
        let (url, source) = resolve_proxy_url(&args, "https");
        assert_eq!(url.as_deref(), Some("http://flag.example:3128"));
        assert_eq!(source, "--proxy");
        clear_proxy_env();
    }

    #[test]
    fn https_target_picks_https_proxy_env() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        clear_proxy_env();
        std::env::set_var("HTTPS_PROXY", "http://env.example:8080");
        let args = args_with_proxy(None, "https://example.com/");
        let (url, _) = resolve_proxy_url(&args, "https");
        assert_eq!(url.as_deref(), Some("http://env.example:8080"));
        clear_proxy_env();
    }

    #[test]
    fn env_var_empty_is_treated_as_none() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        clear_proxy_env();
        std::env::set_var("HTTP_PROXY", "   ");
        let args = args_with_proxy(None, "http://example.com/");
        let (url, _) = resolve_proxy_url(&args, "http");
        assert_eq!(url, None);
        clear_proxy_env();
    }

    #[test]
    fn credentials_from_flag_beat_url_userinfo() {
        let args = args_with_proxy(Some("http://alice:secret@p:3128"), "http://example.com/");
        let mut args = args;
        args.proxy_user = Some("bob:other".into());
        let parsed = url::Url::parse(args.proxy.as_deref().unwrap()).unwrap();
        let (u, p) = resolve_proxy_credentials(&args, &parsed);
        assert_eq!(u.as_deref(), Some("bob"));
        assert_eq!(p.as_deref(), Some("other"));
    }

    #[test]
    fn credentials_fall_back_to_url_userinfo() {
        let args = args_with_proxy(Some("http://alice:secret@p:3128"), "http://example.com/");
        let parsed = url::Url::parse(args.proxy.as_deref().unwrap()).unwrap();
        let (u, p) = resolve_proxy_credentials(&args, &parsed);
        assert_eq!(u.as_deref(), Some("alice"));
        assert_eq!(p.as_deref(), Some("secret"));
    }

    #[test]
    fn noproxy_flag_wins_over_env() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        clear_proxy_env();
        std::env::set_var("NO_PROXY", "env.example");
        let mut args = args_with_proxy(None, "http://example.com/");
        args.noproxy = Some("flag.example,other.example".into());
        assert_eq!(resolve_noproxy(&args), "flag.example,other.example");
        clear_proxy_env();
    }
}
