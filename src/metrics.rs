use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Timings captured during the connection phase (DNS, TCP, TLS).
/// Populated by `InstrumentedConnector` when the custom connector is wired in;
/// left at defaults otherwise, in which case the four phase variables in
/// `-w` render as `0.000000`.
#[derive(Clone, Debug, Default)]
pub struct PhaseTiming {
    pub dns_duration: Option<Duration>,
    pub tcp_duration: Option<Duration>,
    pub tls_duration: Option<Duration>,
    pub remote_ip: Option<SocketAddr>,
    pub local_ip: Option<SocketAddr>,
}

/// Metrics captured during a request. Populated incrementally by the client
/// and consumed by the `-w` / `--write-out` renderer.
#[derive(Clone, Debug, Default)]
pub struct RequestMetrics {
    /// When the client started the request.
    pub request_start: Option<Instant>,
    /// When the first response byte was received.
    pub first_response_byte: Option<Instant>,
    /// When the full response was consumed.
    pub response_end: Option<Instant>,
    /// Cumulative time spent in redirect hops (excluding the final hop).
    /// Zero when no redirects occurred.
    pub redirect_duration: Duration,
    /// Number of redirect hops followed.
    pub num_redirects: u32,
    /// Bytes written in the request body.
    pub size_upload: u64,
    /// Bytes read from the response body (post-decompression).
    pub size_download: u64,
    /// Total bytes in all response headers.
    pub size_header: u64,
    /// Response header count.
    pub num_headers: u32,
    /// Final URL after redirects (if any).
    pub url_effective: Option<String>,
    /// Next URL if 3xx and redirects were not followed.
    pub redirect_url: Option<String>,
    /// Shared handle populated by `InstrumentedConnector`.
    pub phase: Arc<Mutex<PhaseTiming>>,
}

impl RequestMetrics {
    /// Wall-clock total duration (start → end).
    pub fn time_total(&self) -> Duration {
        match (self.request_start, self.response_end) {
            (Some(s), Some(e)) => e.saturating_duration_since(s),
            _ => Duration::ZERO,
        }
    }

    /// Time to first response byte (start → first byte).
    pub fn time_starttransfer(&self) -> Duration {
        match (self.request_start, self.first_response_byte) {
            (Some(s), Some(fb)) => fb.saturating_duration_since(s),
            _ => Duration::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_zero() {
        let m = RequestMetrics::default();
        assert_eq!(m.time_total(), Duration::ZERO);
        assert_eq!(m.time_starttransfer(), Duration::ZERO);
        assert_eq!(m.num_redirects, 0);
        assert_eq!(m.size_download, 0);
        assert!(m.url_effective.is_none());
    }

    #[test]
    fn time_total_computes_from_start_and_end() {
        let mut m = RequestMetrics::default();
        let t0 = Instant::now();
        m.request_start = Some(t0);
        m.response_end = Some(t0 + Duration::from_millis(250));
        assert_eq!(m.time_total(), Duration::from_millis(250));
    }

    #[test]
    fn time_total_zero_without_end() {
        let mut m = RequestMetrics::default();
        m.request_start = Some(Instant::now());
        assert_eq!(m.time_total(), Duration::ZERO);
    }

    #[test]
    fn time_starttransfer_computes_from_start_and_first_byte() {
        let mut m = RequestMetrics::default();
        let t0 = Instant::now();
        m.request_start = Some(t0);
        m.first_response_byte = Some(t0 + Duration::from_millis(100));
        assert_eq!(m.time_starttransfer(), Duration::from_millis(100));
    }
}
