//! Browser TLS+H2 fingerprint impersonation via `wreq` (BoringSSL).
//!
//! Activated when any of `--impersonate`, `--ja3`, `--ja4`, or
//! `--http2-fingerprint` is set. Owns its own `wreq::Client` driven by a
//! tokio runtime parallel to `client::execute`; supports a deliberate subset
//! of recon's HTTP feature surface (see HISTORY.md and OUT-OF-SCOPE.md for
//! the v1 incompat list).

#![cfg(feature = "impersonate")]

pub mod h2_fingerprint;

use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Response as ReqwestResponse;
use wreq::{Http2Builder, Priority, StreamDependency, StreamId};

use crate::cli::Args;
use crate::metrics::RequestMetrics;
use h2_fingerprint::H2Fingerprint;

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
    // impersonation", "profile", "wreq" instead.
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
/// wreq_util's Emulation enum. Normalizes hyphens to underscores so users
/// can type either form on the CLI.
fn parse_emulation(name: &str) -> Result<wreq_util::Emulation> {
    // wreq_util's serde format is `chrome_131`, `safari_17.5`, etc.
    // Normalize hyphens to underscores so users can type either form.
    let normalized = name.trim().replace('-', "_").to_ascii_lowercase();
    let value = serde_json::Value::String(normalized.clone());
    serde_json::from_value::<wreq_util::Emulation>(value).map_err(|e| {
        anyhow!(
            "unknown impersonate profile '{name}' (normalized to '{normalized}'). \
             Valid examples: chrome_131, firefox_128, safari_17.5, edge_131, \
             chrome_android_131, safari_ios_17.4.1, okhttp_5. \
             See `recon --help impersonate` for the full list. ({e})"
        )
    })
}

/// Convert an wreq::Response (async client) into a reqwest::blocking::Response
/// so the rest of recon's output pipeline (reqwest-typed) consumes it unchanged.
/// Buffers the body in memory; acceptable for the v1 captcha-testing use case.
async fn convert_response(resp: wreq::Response) -> Result<ReqwestResponse> {
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

    // JA3 / JA4 raw overrides stay deferred: JA3 strings don't capture
    // sigalgs or extension order, and JA4's cipher/extension components are
    // non-invertible SHA-256 truncations — both reconstruct a partial,
    // lossy TLS fingerprint. The HTTP/2 layer, by contrast, is fully
    // introspectable, so --http2-fingerprint IS implemented below.
    // Rationale in OUT-OF-SCOPE.md under "Raw fingerprint overrides".
    // NOTE: keep the words "TLS" and "certificate" out of this string —
    // main.rs::friendly_message rewrites any error containing them to a
    // generic "TLS/certificate error" placeholder, which would hide this
    // message from the user (see validate_combination above).
    if args.ja3.is_some() || args.ja4.is_some() {
        return Err(anyhow!(
            "--ja3 / --ja4 are not implemented (lossy / non-invertible browser \
             fingerprints — see OUT-OF-SCOPE.md). Use --impersonate <profile> \
             for a named browser fingerprint, or --http2-fingerprint for a raw \
             Akamai HTTP/2 fingerprint."
        ));
    }

    // Parse the H2 fingerprint up front so malformed input fails before we
    // open a connection.
    let fingerprint = match args.http2_fingerprint.as_deref() {
        Some(s) => Some(h2_fingerprint::parse(s)?),
        None => None,
    };

    // A profile is optional when a raw H2 fingerprint is supplied:
    // fingerprint-only means default wreq TLS plus the custom H2 layer.
    let emulation = match args.impersonate.as_deref() {
        Some(name) => Some(parse_emulation(name)?),
        None => None,
    };
    if emulation.is_none() && fingerprint.is_none() {
        return Err(anyhow!(
            "impersonate::execute called without --impersonate or \
             --http2-fingerprint (this is a bug)"
        ));
    }

    let mut metrics = RequestMetrics::default();
    metrics.request_start = Some(Instant::now());

    // wreq is async-only; spin up a current-thread tokio runtime and
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
        let mut builder = wreq::Client::builder()
            .cert_verification(!insecure)
            .connect_timeout(std::time::Duration::from_secs(timeout_secs));
        // Profile (if any) sets TLS + a baseline H2 config first...
        if let Some(emulation) = emulation {
            builder = builder.emulation(emulation);
        }
        // ...then the raw H2 fingerprint overrides the H2 layer per-field.
        // Calling .http2() after .emulation() wins because both mutate the
        // same underlying HTTP/2 builder.
        if let Some(fp) = fingerprint {
            builder = builder.http2(move |mut h2| apply_fingerprint(&mut h2, &fp));
        }
        let client = builder
            .build()
            .map_err(|e| anyhow!("wreq client build: {e}"))?;

        let method = wreq::Method::from_bytes(method_str.as_bytes())
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

/// Apply a parsed Akamai H2 fingerprint to wreq's `Http2Builder`. Mirrors
/// wreq's internal `apply_http2_config` so a profile's baseline H2 settings
/// are overridden per-field by the raw fingerprint.
fn apply_fingerprint(h2: &mut Http2Builder<'_>, fp: &H2Fingerprint) {
    for &(id, val) in &fp.settings {
        match id {
            1 => { h2.header_table_size(val); }
            2 => { h2.enable_push(val != 0); }
            3 => { h2.max_concurrent_streams(val); }
            4 => { h2.initial_stream_window_size(val); }
            5 => { h2.max_frame_size(val); }
            6 => { h2.max_header_list_size(val); }
            8 => { h2.unknown_setting8(val != 0); }
            9 => { h2.unknown_setting9(val != 0); }
            _ => {} // parse() already rejected unknown ids
        }
    }
    h2.settings_order(Some(fp.settings_order()));
    h2.headers_pseudo_order(Some(fp.pseudo_order));
    if let Some(wu) = fp.window_update {
        h2.initial_connection_window_size(wu);
    }
    if !fp.priorities.is_empty() {
        let priorities: Vec<Priority> = fp
            .priorities
            .iter()
            .map(|p| {
                Priority::new(
                    StreamId::from(p.stream_id),
                    StreamDependency::new(StreamId::from(p.depends_on), p.weight, p.exclusive),
                )
            })
            .collect();
        h2.priority(Some(std::borrow::Cow::Owned(priorities)));
    }
}
