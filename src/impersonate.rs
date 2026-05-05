//! Browser TLS+H2 fingerprint impersonation via `rquest` (BoringSSL).
//!
//! Activated when any of `--impersonate`, `--ja3`, `--ja4`, or
//! `--http2-fingerprint` is set. Owns its own `rquest::Client` driven by a
//! tokio runtime parallel to `client::execute`; supports a deliberate subset
//! of recon's HTTP feature surface (see HISTORY.md and OUT-OF-SCOPE.md for
//! the v1 incompat list).

#![cfg(feature = "impersonate")]

use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Response as ReqwestResponse;

use crate::cli::Args;
use crate::metrics::RequestMetrics;

/// True if any flag in this module's surface is set on `args`.
pub fn is_active(args: &Args) -> bool {
    args.impersonate.is_some()
        || args.ja3.is_some()
        || args.ja4.is_some()
        || args.http2_fingerprint.is_some()
}

/// Validate that none of the v1-incompatible flags are combined with
/// any impersonation flag. Errors out with a clear message naming the
/// offending flag pair.
pub fn validate_combination(args: &Args) -> Result<()> {
    // Note: avoid the words "TLS" and "certificate" in these error strings
    // because main.rs::friendly_message rewrites any error containing them
    // to the generic "TLS/certificate error" placeholder, which would hide
    // the actually-helpful message from the user. Use "browser fingerprint
    // impersonation", "profile", "rquest" instead.
    if args.ciphers.is_some() || args.tls13_ciphers.is_some() {
        return Err(anyhow!(
            "--ciphers / --tls13-ciphers cannot be combined with browser \
             fingerprint impersonation: the profile owns the cipher list."
        ));
    }
    if args.tlsv12 || args.tlsv13 {
        return Err(anyhow!(
            "--tlsv1.2 / --tlsv1.3 cannot be combined with browser fingerprint \
             impersonation: the profile owns the protocol version."
        ));
    }
    if args.client_cert.is_some() || args.client_key.is_some() {
        return Err(anyhow!(
            "--client-cert / --client-key (mutual auth) is not supported with \
             browser fingerprint impersonation in v1 (deferred — see OUT-OF-SCOPE.md)."
        ));
    }
    if args.cacert.is_some() || args.capath.is_some() {
        return Err(anyhow!(
            "--cacert / --capath is not supported with browser fingerprint \
             impersonation in v1 (system roots only — see OUT-OF-SCOPE.md)."
        ));
    }
    if args.ja3.is_some() && args.ja4.is_some() {
        eprintln!(
            "warning: both --ja3 and --ja4 set; JA4 will take precedence \
             (they describe overlapping but different views of the ClientHello)."
        );
    }
    Ok(())
}

/// Parse a user-supplied profile string ("chrome_131", "chrome-131") into
/// rquest_util's Emulation enum. Normalizes hyphens to underscores so users
/// can type either form on the CLI.
fn parse_emulation(name: &str) -> Result<rquest_util::Emulation> {
    // rquest_util's serde format is `chrome_131`, `safari_17.5`, etc.
    // Normalize hyphens to underscores so users can type either form.
    let normalized = name.trim().replace('-', "_").to_ascii_lowercase();
    let value = serde_json::Value::String(normalized.clone());
    serde_json::from_value::<rquest_util::Emulation>(value).map_err(|e| {
        anyhow!(
            "unknown impersonate profile '{name}' (normalized to '{normalized}'). \
             Valid examples: chrome_131, firefox_128, safari_17.5, edge_131, \
             chrome_android_131, safari_ios_17.4.1, okhttp_5. \
             See `recon --help impersonate` for the full list. ({e})"
        )
    })
}

/// Convert an rquest::Response (async client) into a reqwest::blocking::Response
/// so the rest of recon's output pipeline (reqwest-typed) consumes it unchanged.
/// Buffers the body in memory; acceptable for the v1 captcha-testing use case.
async fn convert_response(resp: rquest::Response) -> Result<ReqwestResponse> {
    let status = resp.status();
    let headers = resp.headers().clone();
    let version = resp.version();
    let bytes = resp.bytes().await.map_err(|e| anyhow!("read body: {e}"))?;

    let mut builder = http::Response::builder()
        .status(status)
        .version(version);
    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }
    let http_resp = builder
        .body(bytes.to_vec())
        .map_err(|e| anyhow!("build http::Response: {e}"))?;
    Ok(ReqwestResponse::from(http_resp))
}

/// Public entry — mirrors `client::execute` for the impersonation path.
pub fn execute(args: &Args) -> Result<(ReqwestResponse, RequestMetrics)> {
    validate_combination(args)?;

    // Defer raw-fingerprint flags to v0.78 (stubbed in Task 5). Fail fast
    // if any of them is set so the user sees a clear "not yet implemented"
    // rather than a silent no-op.
    if args.ja3.is_some() || args.ja4.is_some() || args.http2_fingerprint.is_some() {
        return Err(anyhow!(
            "--ja3 / --ja4 / --http2-fingerprint are not implemented in v1 \
             (tracked for v0.78). Use --impersonate <profile> for a named-profile \
             fingerprint instead."
        ));
    }

    let profile_name = args.impersonate.as_deref().ok_or_else(|| {
        anyhow!("impersonate::execute called without --impersonate (this is a bug)")
    })?;
    let emulation = parse_emulation(profile_name)?;

    let mut metrics = RequestMetrics::default();
    metrics.request_start = Some(Instant::now());

    // rquest 5.1.0 is async-only; spin up a current-thread tokio runtime and
    // block on the request. recon's pipeline above us is blocking, so this
    // is the standard sync-bridges-async pattern.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime for impersonate path")?;

    let url = args.target_url().to_string();
    let method_str = args.effective_method();
    let timeout_secs = args.timeout;
    let insecure = args.insecure;

    let converted = runtime.block_on(async move {
        let client = rquest::Client::builder()
            .emulation(emulation)
            .cert_verification(!insecure)
            .connect_timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| anyhow!("rquest client build: {e}"))?;

        let method = rquest::Method::from_bytes(method_str.as_bytes())
            .map_err(|e| anyhow!("invalid HTTP method '{method_str}': {e}"))?;

        let resp = client
            .request(method, url)
            .send()
            .await
            .map_err(|e| anyhow!("impersonate request: {e}"))?;

        convert_response(resp).await
    })?;

    // Populate response-side metrics in the same shape client.rs uses.
    crate::client::snapshot_response_for_impersonate(&mut metrics, args, &converted);

    // The reqwest::blocking::Response synthesised from http::Response in
    // convert_response carries reqwest's placeholder URL, so the
    // url_effective set by snapshot_response is wrong on this path. Override
    // with the actual target URL. (Redirects aren't supported on the
    // impersonate path in v1 — the URL we sent IS the URL the response came
    // from.)
    metrics.url_effective = Some(args.target_url().to_string());

    Ok((converted, metrics))
}
