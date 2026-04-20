//! NTP (SNTPv4) probe: `ntp://host[:port]/`
//!
//! Sends a single SNTPv4 request (RFC 4330) to the server and reports
//! stratum, reference identifier, offset from local clock, round-trip
//! delay, precision, poll interval, and the server's reference time.
//! Exit 0 on response, 28 on timeout, 7 on unreachable.
//!
//! Hand-rolled because SNTP is a single 48-byte request and a tiny
//! response parse — keeps the dep graph small.

use anyhow::{anyhow, Context, Result};
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Seconds between the NTP epoch (1900-01-01) and the Unix epoch (1970-01-01).
const NTP_UNIX_DELTA: u64 = 2_208_988_800;

/// Structured SNTPv4 probe result.
pub struct NtpProbeOk {
    pub host: String,
    pub port: u16,
    pub stratum: u8,
    pub poll_interval: i8,
    pub precision: i8,
    pub ref_id_formatted: String,
    pub reference_ts: f64,
    pub offset_secs: f64,
    pub delay_secs: f64,
}

pub fn probe(url: &str, timeout_secs: u64) -> Result<NtpProbeOk> {
    let (host, port) = parse_url(url)?;

    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .with_context(|| format!("ntp: could not resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("ntp: no addresses resolved for {host}:{port}"))?;

    let bind_addr = if addr.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
    let socket = UdpSocket::bind(bind_addr)
        .with_context(|| format!("ntp: could not bind local socket ({bind_addr})"))?;
    socket
        .set_read_timeout(Some(Duration::from_secs(timeout_secs)))
        .context("ntp: set_read_timeout failed")?;
    socket
        .set_write_timeout(Some(Duration::from_secs(timeout_secs)))
        .context("ntp: set_write_timeout failed")?;
    socket
        .connect(addr)
        .with_context(|| format!("ntp: connect failed to {host}:{port}"))?;

    let mut request = [0u8; 48];
    request[0] = 0x23;

    let local_send_sys = SystemTime::now();
    socket.send(&request).context("ntp: send failed")?;

    let mut response = [0u8; 48];
    let nbytes = match socket.recv(&mut response) {
        Ok(n) => n,
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            return Err(anyhow!(
                "ntp: {host}:{port} did not respond within {}s",
                timeout_secs
            )
            .context(crate::mqtt::ProtocolExitCode::OperationTimedOut));
        }
        Err(e) => {
            return Err(anyhow!("ntp: recv failed from {host}:{port}: {e}")
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect));
        }
    };
    let local_recv_sys = SystemTime::now();

    if nbytes < 48 {
        return Err(anyhow!(
            "ntp: short response from {host}:{port} ({nbytes} bytes, expected 48)"
        ));
    }

    let stratum = response[1];
    let poll_interval = response[2] as i8;
    let precision = response[3] as i8;
    let ref_id_formatted = format_ref_id(stratum, &response[12..16]);

    let reference_ts = ntp_to_unix_secs(&response[16..24]);
    let receive_ts = ntp_to_unix_secs(&response[32..40]);
    let transmit_ts = ntp_to_unix_secs(&response[40..48]);

    let local_send = local_send_sys
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let local_recv = local_recv_sys
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let offset = ((receive_ts - local_send) + (transmit_ts - local_recv)) / 2.0;
    let delay = (local_recv - local_send) - (transmit_ts - receive_ts);

    Ok(NtpProbeOk {
        host,
        port,
        stratum,
        poll_interval,
        precision,
        ref_id_formatted,
        reference_ts,
        offset_secs: offset,
        delay_secs: delay,
    })
}

pub fn run(url: &str, timeout_secs: u64) -> Result<()> {
    let p = probe(url, timeout_secs)?;
    println!("* Connected to {}:{} (NTP)", p.host, p.port);
    println!("< Stratum: {} ({})", p.stratum, stratum_name(p.stratum));
    println!("< Reference ID: {}", p.ref_id_formatted);
    println!(
        "< Precision: 2^{} s = {:.6}s",
        p.precision,
        2f64.powi(p.precision as i32)
    );
    println!(
        "< Poll Interval: 2^{} s = {}s",
        p.poll_interval,
        1i64.checked_shl(p.poll_interval.max(0) as u32).unwrap_or(i64::MAX)
    );
    println!("< Reference Time: {}", format_ts(p.reference_ts));
    println!(
        "< Offset: {:+.6}s (local clock is {})",
        p.offset_secs,
        if p.offset_secs > 0.0 {
            "behind server"
        } else {
            "ahead of server"
        }
    );
    println!("< Round-trip Delay: {:.6}s", p.delay_secs.abs());
    Ok(())
}

fn parse_url(url: &str) -> Result<(String, u16)> {
    let parsed = url::Url::parse(url)
        .with_context(|| format!("malformed ntp URL: {url}"))?;
    if parsed.scheme() != "ntp" {
        anyhow::bail!("ntp_probe::run called with non-ntp scheme");
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("ntp URL missing host: {url}"))?
        .to_string();
    let port = parsed.port().unwrap_or(123);
    Ok((host, port))
}

fn ntp_to_unix_secs(bytes: &[u8]) -> f64 {
    let secs = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64;
    let frac = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let frac_secs = frac as f64 / 4_294_967_296.0;
    (secs.saturating_sub(NTP_UNIX_DELTA)) as f64 + frac_secs
}

fn format_ts(unix_secs: f64) -> String {
    if unix_secs <= 0.0 {
        return "(unset)".to_string();
    }
    // Use SystemTime's Debug impl for a readable absolute time; include
    // the raw unix-epoch seconds for grep-ability.
    let whole_secs = unix_secs as u64;
    let sys = UNIX_EPOCH.checked_add(Duration::from_secs(whole_secs));
    match sys {
        Some(t) => format!("{:?} ({:.6} epoch seconds)", t, unix_secs),
        None => format!("{unix_secs:.6} epoch seconds"),
    }
}

fn stratum_name(s: u8) -> &'static str {
    match s {
        0 => "unspecified / kiss-of-death",
        1 => "primary (reference clock)",
        2..=15 => "secondary (NTP network)",
        16 => "unsynchronized",
        _ => "reserved",
    }
}

fn format_ref_id(stratum: u8, ref_id: &[u8]) -> String {
    // Stratum 1: ASCII reference identifier (e.g. "GPS ", "PPS ", "DCF ").
    if stratum == 1 && ref_id.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
        return format!(
            "{:?}",
            std::str::from_utf8(ref_id).unwrap_or("?").trim()
        );
    }
    // Stratum 2+: IPv4 address of the server this one syncs with.
    if stratum >= 2 {
        return format!("{}.{}.{}.{}", ref_id[0], ref_id[1], ref_id[2], ref_id[3]);
    }
    // Stratum 0: kiss-of-death code (ASCII, e.g. "RATE", "DENY").
    let code = std::str::from_utf8(ref_id).unwrap_or("");
    format!("KoD:{code:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ntp_url_default_port() {
        let (h, p) = parse_url("ntp://pool.ntp.org/").unwrap();
        assert_eq!(h, "pool.ntp.org");
        assert_eq!(p, 123);
    }

    #[test]
    fn parses_ntp_url_explicit_port() {
        let (_h, p) = parse_url("ntp://pool.ntp.org:12345/").unwrap();
        assert_eq!(p, 12345);
    }

    #[test]
    fn rejects_non_ntp_scheme() {
        assert!(parse_url("http://pool.ntp.org/").is_err());
    }

    #[test]
    fn ntp_epoch_delta_sanity() {
        // 3_914_000_000 NTP secs ≈ 2023-12-10 Unix.
        let mut b = [0u8; 8];
        b[..4].copy_from_slice(&3_914_000_000u32.to_be_bytes());
        let unix = ntp_to_unix_secs(&b);
        assert!(
            unix > 1_700_000_000.0 && unix < 1_800_000_000.0,
            "unix conversion out of expected range: {unix}"
        );
    }

    #[test]
    fn stratum_names_cover_rfc_bands() {
        assert_eq!(stratum_name(0), "unspecified / kiss-of-death");
        assert_eq!(stratum_name(1), "primary (reference clock)");
        assert_eq!(stratum_name(5), "secondary (NTP network)");
        assert_eq!(stratum_name(15), "secondary (NTP network)");
        assert_eq!(stratum_name(16), "unsynchronized");
    }

    #[test]
    fn ref_id_stratum_1_ascii() {
        let b = [b'G', b'P', b'S', b' '];
        assert_eq!(format_ref_id(1, &b), "\"GPS\"");
    }

    #[test]
    fn ref_id_stratum_2_as_ipv4() {
        let b = [198u8, 51, 100, 42];
        assert_eq!(format_ref_id(2, &b), "198.51.100.42");
    }

    #[test]
    fn ntp_frac_parses() {
        // secs=0, frac=u32::MAX/2 = 0x80000000 → 0.5s
        let mut b = [0u8; 8];
        b[4..].copy_from_slice(&0x8000_0000u32.to_be_bytes());
        let unix = ntp_to_unix_secs(&b);
        // secs=0 before NTP_UNIX_DELTA saturates, result is the fractional only.
        assert!((unix - 0.5).abs() < 1e-9, "expected 0.5, got {unix}");
    }
}
