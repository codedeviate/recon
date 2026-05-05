//! Browser TLS+H2 fingerprint impersonation via `rquest` (BoringSSL).
//!
//! Activated when any of `--impersonate`, `--ja3`, `--ja4`, or
//! `--http2-fingerprint` is set. Owns its own `rquest::Client` driven by a
//! tokio runtime parallel to `client::execute`; supports a deliberate subset
//! of recon's HTTP feature surface (see HISTORY.md and OUT-OF-SCOPE.md for
//! the v1 incompat list).

#![cfg(feature = "impersonate")]

use anyhow::{anyhow, Result};

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
    if args.ciphers.is_some() || args.tls13_ciphers.is_some() {
        return Err(anyhow!(
            "--ciphers / --tls13-ciphers cannot be combined with TLS \
             impersonation: the profile owns the cipher list."
        ));
    }
    if args.tlsv12 || args.tlsv13 {
        return Err(anyhow!(
            "--tlsv1.2 / --tlsv1.3 cannot be combined with TLS \
             impersonation: the profile owns the TLS version."
        ));
    }
    if args.client_cert.is_some() || args.client_key.is_some() {
        return Err(anyhow!(
            "--client-cert / --client-key (client cert auth) is not supported with \
             TLS impersonation in v1 (deferred — see OUT-OF-SCOPE.md)."
        ));
    }
    if args.cacert.is_some() || args.capath.is_some() {
        return Err(anyhow!(
            "--cacert / --capath is not supported with TLS impersonation \
             in v1 (system roots only — see OUT-OF-SCOPE.md)."
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

/// Public entry — mirrors `client::execute` for the impersonation path.
pub fn execute(args: &Args) -> Result<(reqwest::blocking::Response, RequestMetrics)> {
    validate_combination(args)?;
    Err(anyhow!(
        "--impersonate: not yet implemented (Tasks 4–7 wire the rquest client). \
         This stub proves dispatch works."
    ))
}
