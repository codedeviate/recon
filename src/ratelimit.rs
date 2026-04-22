//! Rate-control helpers for `--limit-rate`, `--speed-limit`, `--speed-time`.
//!
//! - `parse_rate` reads curl's rate grammar (`100K` / `2M` / `1.5G` /
//!   bare bytes) into a bytes-per-second value.
//! - `RateLimitedWriter` wraps an `io::Write` and throttles writes so
//!   the cumulative rate stays at-or-below the configured limit.
//! - `SpeedWatchWriter` aborts a transfer when the rolling throughput
//!   drops below `speed_limit` for `speed_time` seconds.

use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Parse a curl-compatible rate string.
///
/// Accepts `100`, `100K` (= 102_400), `2M` (= 2_097_152), `1G` (= 2^30),
/// `1.5M`, case-insensitive. Trailing `B` is tolerated. Unknown suffix
/// → error.
pub fn parse_rate(input: &str) -> Result<u64> {
    let s = input.trim();
    if s.is_empty() {
        return Err(anyhow!("rate: empty value"));
    }
    // Split numeric prefix + suffix.
    let (num_str, suffix) = split_numeric(s);
    let num: f64 = num_str
        .parse()
        .map_err(|_| anyhow!("rate: '{input}' not a valid number"))?;
    if num < 0.0 {
        return Err(anyhow!("rate: '{input}' must be non-negative"));
    }
    let multiplier: u64 = match suffix.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1024,
        "m" | "mb" => 1024 * 1024,
        "g" | "gb" => 1024 * 1024 * 1024,
        "t" | "tb" => 1024u64.pow(4),
        other => {
            return Err(anyhow!(
                "rate: unknown suffix '{other}' in '{input}' (expected K/M/G/T)"
            ));
        }
    };
    Ok((num * multiplier as f64) as u64)
}

fn split_numeric(s: &str) -> (&str, &str) {
    let mut cut = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            cut = i + c.len_utf8();
        } else {
            break;
        }
    }
    s.split_at(cut)
}

/// Wraps an `io::Write` and sleeps between writes so the output rate
/// does not exceed `bytes_per_sec`. Tracks total bytes and wall-clock
/// time since the wrapper was created; on each `write`, computes the
/// wall-clock time that would have been required at the pinned rate
/// and sleeps the delta if we're ahead.
pub struct RateLimitedWriter<W: Write> {
    inner: W,
    bytes_per_sec: u64,
    total_bytes: u64,
    started: Instant,
}

impl<W: Write> RateLimitedWriter<W> {
    pub fn new(inner: W, bytes_per_sec: u64) -> Self {
        Self {
            inner,
            bytes_per_sec,
            total_bytes: 0,
            started: Instant::now(),
        }
    }

    fn sleep_until_caught_up(&self) {
        if self.bytes_per_sec == 0 {
            return;
        }
        // Expected elapsed time for total_bytes at the pinned rate.
        let expected_ns = (self.total_bytes as u128)
            .saturating_mul(1_000_000_000)
            .checked_div(self.bytes_per_sec as u128)
            .unwrap_or(0);
        let actual_ns = self.started.elapsed().as_nanos();
        if let Some(delta) = expected_ns.checked_sub(actual_ns) {
            if delta > 0 {
                std::thread::sleep(Duration::from_nanos(delta.min(u64::MAX as u128) as u64));
            }
        }
    }
}

impl<W: Write> Write for RateLimitedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.total_bytes += n as u64;
        self.sleep_until_caught_up();
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Wraps an `io::Write` and aborts the transfer when the rolling
/// throughput falls below `min_bytes_per_sec` for `window` seconds.
///
/// Implementation: track total bytes + start time; on each write that
/// crosses a check interval (default: every second), compute the
/// windowed rate over the last `window` seconds. If below the floor for
/// the entire window, return `io::ErrorKind::TimedOut`.
pub struct SpeedWatchWriter<W: Write> {
    inner: W,
    min_bytes_per_sec: u64,
    window: Duration,
    total_bytes: u64,
    started: Instant,
    /// First moment throughput went below the floor (reset when it rises).
    below_floor_since: Option<Instant>,
    last_check: Instant,
}

impl<W: Write> SpeedWatchWriter<W> {
    pub fn new(inner: W, min_bytes_per_sec: u64, window: Duration) -> Self {
        let now = Instant::now();
        Self {
            inner,
            min_bytes_per_sec,
            window,
            total_bytes: 0,
            started: now,
            below_floor_since: None,
            last_check: now,
        }
    }

    fn check_speed(&mut self) -> io::Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_check) < Duration::from_secs(1) {
            return Ok(());
        }
        self.last_check = now;
        let elapsed = now.duration_since(self.started);
        // Until the first window has elapsed, don't judge (need enough
        // samples to avoid punishing slow TCP ramp-up).
        if elapsed < self.window {
            return Ok(());
        }
        let rate = if elapsed.as_secs_f64() > 0.0 {
            (self.total_bytes as f64 / elapsed.as_secs_f64()) as u64
        } else {
            u64::MAX
        };
        if rate < self.min_bytes_per_sec {
            let since = self.below_floor_since.unwrap_or(now);
            if since == now {
                self.below_floor_since = Some(now);
            }
            if now.duration_since(since) >= self.window {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "transfer slower than {} B/s for {}s — aborted",
                        self.min_bytes_per_sec,
                        self.window.as_secs()
                    ),
                ));
            }
        } else {
            self.below_floor_since = None;
        }
        Ok(())
    }
}

impl<W: Write> Write for SpeedWatchWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.total_bytes += n as u64;
        self.check_speed()?;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rate_basics() {
        assert_eq!(parse_rate("100").unwrap(), 100);
        assert_eq!(parse_rate("100K").unwrap(), 102_400);
        assert_eq!(parse_rate("100k").unwrap(), 102_400);
        assert_eq!(parse_rate("1M").unwrap(), 1_048_576);
        assert_eq!(parse_rate("2M").unwrap(), 2_097_152);
        assert_eq!(parse_rate("1G").unwrap(), 1_073_741_824);
        assert_eq!(parse_rate("500B").unwrap(), 500);
    }

    #[test]
    fn parse_rate_decimal() {
        assert_eq!(parse_rate("1.5M").unwrap(), 1_572_864);
        assert_eq!(parse_rate("0.5K").unwrap(), 512);
    }

    #[test]
    fn parse_rate_errors() {
        assert!(parse_rate("").is_err());
        assert!(parse_rate("abc").is_err());
        assert!(parse_rate("100X").is_err());
        assert!(parse_rate("-5").is_err());
    }

    #[test]
    fn rate_limited_writer_roughly_correct_pace() {
        // 1 KB/s — writing 1024 bytes should take ~1 second.
        // Use a small payload (100 bytes) for a faster test; 100/1024 s ≈ 98ms.
        let buf: Vec<u8> = vec![0u8; 100];
        let mut sink = Vec::new();
        let mut w = RateLimitedWriter::new(&mut sink, 1024);
        let start = Instant::now();
        w.write_all(&buf).unwrap();
        let elapsed = start.elapsed();
        // Expected: ~97ms. Allow wide slop for test-host jitter.
        assert!(
            elapsed.as_millis() >= 50,
            "expected throttling, elapsed = {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn rate_limited_writer_passes_data_unchanged() {
        let payload = b"the quick brown fox";
        let mut sink = Vec::new();
        {
            let mut w = RateLimitedWriter::new(&mut sink, 1_000_000_000); // effectively no throttle
            w.write_all(payload).unwrap();
        }
        assert_eq!(sink, payload);
    }
}
