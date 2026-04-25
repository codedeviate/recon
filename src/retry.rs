//! Retry layer wrapping `client::execute` for the `--retry*` flags.
//!
//! Policy:
//! - `retry = 0` → call `execute` once. No wrapping.
//! - `retry > 0` → retry on transient errors (timeouts, connection
//!   resets, DNS failures, 5xx responses) up to `retry` times.
//! - `--retry-all-errors` → treat every error as retryable.
//! - `--retry-connrefused` → add ECONNREFUSED to the retry set.
//! - `--retry-delay <SECS>` → fixed delay; otherwise exponential
//!   backoff (1s, 2s, 4s, 8s, 16s, 32s cap).
//! - `--retry-max-time <SECS>` → total budget across all retries.

use anyhow::Result;
use std::time::{Duration, Instant};

use crate::cli::Args;
use crate::metrics::RequestMetrics;

/// Execute the HTTP request with retry wrapping. Delegates to
/// `client::execute` on the first try; re-invokes it per policy on
/// retryable failures.
pub fn execute_with_retry(args: &Args) -> Result<(reqwest::blocking::Response, RequestMetrics)> {
    // wget-style --tries N means N total attempts (so retries = N-1).
    // When set, it overrides --retry. --tries 0 is rejected at parse
    // time; the saturating_sub here is defensive.
    let effective_retries = args
        .tries
        .map(|t| t.saturating_sub(1))
        .unwrap_or(args.retry);

    if effective_retries == 0 {
        return crate::client::execute(args);
    }

    let start = Instant::now();
    let budget = args.retry_max_time.map(Duration::from_secs);
    let mut attempt: u32 = 0;
    let max_attempts = effective_retries.saturating_add(1);

    loop {
        attempt += 1;
        let result = crate::client::execute(args);

        match result {
            Ok((response, metrics)) => {
                if should_retry_status(&response, args) && attempt < max_attempts {
                    let code = response.status().as_u16();
                    let delay = compute_delay(attempt, args);
                    if let Some(b) = budget {
                        if start.elapsed() + delay > b {
                            // Out of budget — surface whatever we got.
                            return Ok((response, metrics));
                        }
                    }
                    if args.verbose >= 1 {
                        eprintln!(
                            "* retry {attempt}/{max_attempts}: HTTP {code} — sleeping {}ms",
                            delay.as_millis()
                        );
                    }
                    std::thread::sleep(delay);
                    continue;
                }
                return Ok((response, metrics));
            }
            Err(e) => {
                if !is_retryable_error(&e, args) || attempt >= max_attempts {
                    return Err(e);
                }
                let delay = compute_delay(attempt, args);
                if let Some(b) = budget {
                    if start.elapsed() + delay > b {
                        return Err(e);
                    }
                }
                if args.verbose >= 1 {
                    eprintln!(
                        "* retry {attempt}/{max_attempts}: {} — sleeping {}ms",
                        e,
                        delay.as_millis()
                    );
                }
                std::thread::sleep(delay);
            }
        }
    }
}

fn should_retry_status(response: &reqwest::blocking::Response, args: &Args) -> bool {
    let status = response.status();
    if status.is_server_error() {
        return true;
    }
    if args.retry_all_errors && status.is_client_error() {
        return true;
    }
    false
}

fn is_retryable_error(err: &anyhow::Error, args: &Args) -> bool {
    if args.retry_all_errors {
        return true;
    }
    let s = format!("{err:#}").to_lowercase();
    // Transient: timeout, connection reset, DNS failure, TLS handshake
    // blips, generic I/O errors.
    for marker in [
        "timed out",
        "timeout",
        "deadline has elapsed",
        "connection reset",
        "reset by peer",
        "broken pipe",
        "dns error",
        "failed to lookup",
        "os error 35", // macOS EAGAIN
        "temporary failure",
    ] {
        if s.contains(marker) {
            return true;
        }
    }
    if args.retry_connrefused && s.contains("connection refused") {
        return true;
    }
    false
}

fn compute_delay(attempt: u32, args: &Args) -> Duration {
    if let Some(fixed) = args.retry_delay {
        return Duration::from_secs(fixed);
    }
    // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s cap.
    let secs = 1u64 << (attempt.saturating_sub(1).min(5));
    Duration::from_secs(secs.min(32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args_with(extra: &[&str]) -> Args {
        let mut argv = vec!["recon", "https://example.com/"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv).unwrap()
    }

    #[test]
    fn delay_is_fixed_when_explicit() {
        let a = args_with(&["--retry", "3", "--retry-delay", "5"]);
        assert_eq!(compute_delay(1, &a), Duration::from_secs(5));
        assert_eq!(compute_delay(3, &a), Duration::from_secs(5));
    }

    #[test]
    fn delay_is_exponential_when_default() {
        let a = args_with(&["--retry", "5"]);
        assert_eq!(compute_delay(1, &a), Duration::from_secs(1));
        assert_eq!(compute_delay(2, &a), Duration::from_secs(2));
        assert_eq!(compute_delay(3, &a), Duration::from_secs(4));
        assert_eq!(compute_delay(4, &a), Duration::from_secs(8));
        assert_eq!(compute_delay(6, &a), Duration::from_secs(32));
        // Cap at 32s.
        assert_eq!(compute_delay(10, &a), Duration::from_secs(32));
    }

    #[test]
    fn transient_error_triggers_retry_without_all_errors() {
        let a = args_with(&["--retry", "3"]);
        let err = anyhow::anyhow!("request timed out");
        assert!(is_retryable_error(&err, &a));
    }

    #[test]
    fn non_transient_error_skipped_by_default() {
        let a = args_with(&["--retry", "3"]);
        let err = anyhow::anyhow!("Invalid header format");
        assert!(!is_retryable_error(&err, &a));
    }

    #[test]
    fn all_errors_flag_retries_anything() {
        let a = args_with(&["--retry", "3", "--retry-all-errors"]);
        let err = anyhow::anyhow!("anything at all");
        assert!(is_retryable_error(&err, &a));
    }

    #[test]
    fn connrefused_gated_by_flag() {
        let a = args_with(&["--retry", "3"]);
        let err = anyhow::anyhow!("Connection refused");
        assert!(!is_retryable_error(&err, &a));
        let a = args_with(&["--retry", "3", "--retry-connrefused"]);
        assert!(is_retryable_error(&err, &a));
    }

    fn effective(args: &Args) -> u32 {
        args.tries
            .map(|t| t.saturating_sub(1))
            .unwrap_or(args.retry)
    }

    #[test]
    fn tries_overrides_retry_when_set() {
        let a = args_with(&["--retry", "1", "--tries", "5"]);
        assert_eq!(effective(&a), 4);
    }

    #[test]
    fn tries_alone_sets_retries() {
        let a = args_with(&["--tries", "3"]);
        assert_eq!(effective(&a), 2);
    }

    #[test]
    fn retry_used_when_tries_unset() {
        let a = args_with(&["--retry", "7"]);
        assert_eq!(effective(&a), 7);
    }

    #[test]
    fn tries_one_disables_retries() {
        let a = args_with(&["--tries", "1"]);
        assert_eq!(effective(&a), 0);
    }
}
