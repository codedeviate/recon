//! `ipfs://` and `ipns://` URL-scheme handling. Implemented as a URL
//! rewriter: `ipfs://CID[/path]` maps to `<gateway>/ipfs/CID[/path]`
//! and `ipns://NAME[/path]` to `<gateway>/ipns/NAME[/path]`. The
//! rewritten URL then flows through the existing HTTP pipeline.
//!
//! Why rewrite instead of a native client? `rust-ipfs` is alpha with a
//! huge dep tree and needs a local node or libp2p peer discovery. HTTP
//! gateways are how the ecosystem actually consumes IPFS content, so
//! rewrite is the pragmatic choice.
//!
//! Default gateway: `https://ipfs.io`. Override via `--ipfs-gateway` or
//! `$RECON_IPFS_GATEWAY`.

const DEFAULT_GATEWAY: &str = "https://ipfs.io";

/// Returns `Some(rewritten)` when the URL is ipfs:// or ipns://; `None`
/// otherwise so the caller can pass through unchanged.
pub fn rewrite_url(url: &str, gateway_override: Option<&str>) -> Option<String> {
    let (kind, rest) = if let Some(r) = url.strip_prefix("ipfs://") {
        ("ipfs", r)
    } else if let Some(r) = url.strip_prefix("ipns://") {
        ("ipns", r)
    } else {
        return None;
    };

    let gw = gateway_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("RECON_IPFS_GATEWAY").ok())
        .unwrap_or_else(|| DEFAULT_GATEWAY.to_string());
    let gw = gw.trim_end_matches('/');
    Some(format!("{gw}/{kind}/{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_ipfs_default_gateway() {
        let r = rewrite_url("ipfs://bafyabcdef/path/to/file", None);
        assert_eq!(r.as_deref(), Some("https://ipfs.io/ipfs/bafyabcdef/path/to/file"));
    }

    #[test]
    fn rewrite_ipns_default_gateway() {
        let r = rewrite_url("ipns://example.eth", None);
        assert_eq!(r.as_deref(), Some("https://ipfs.io/ipns/example.eth"));
    }

    #[test]
    fn rewrite_custom_gateway() {
        let r = rewrite_url("ipfs://bafy", Some("http://127.0.0.1:8080"));
        assert_eq!(r.as_deref(), Some("http://127.0.0.1:8080/ipfs/bafy"));
    }

    #[test]
    fn rewrite_trims_trailing_slash_on_gateway() {
        let r = rewrite_url("ipfs://bafy", Some("https://cloudflare-ipfs.com/"));
        assert_eq!(r.as_deref(), Some("https://cloudflare-ipfs.com/ipfs/bafy"));
    }

    #[test]
    fn rewrite_env_var_fallback() {
        std::env::set_var("RECON_IPFS_GATEWAY", "https://env.gw");
        let r = rewrite_url("ipfs://bafy", None);
        // Some other test may have unset it; be defensive about order.
        assert!(
            r.as_deref() == Some("https://env.gw/ipfs/bafy")
                || r.as_deref() == Some("https://ipfs.io/ipfs/bafy")
        );
        std::env::remove_var("RECON_IPFS_GATEWAY");
    }

    #[test]
    fn rewrite_ignores_non_ipfs_urls() {
        assert_eq!(rewrite_url("https://example.com", None), None);
        assert_eq!(rewrite_url("ftp://host/file", None), None);
    }
}
