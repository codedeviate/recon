//! Interface-name to IP-address resolution for `--interface`.
//!
//! Extends `--interface` past the original IP-literal-only form. The
//! user can now pass `eth0` / `en0` / `lo0` etc.; this module uses
//! `libc::getifaddrs` (Unix) to walk the system's interface list and
//! pick the first non-loopback address matching the requested family.
//!
//! Windows falls through to an error — `GetAdapterAddresses` is a
//! bigger undertaking and nobody's asked yet.

use anyhow::{anyhow, Result};
use std::net::IpAddr;

/// Resolve the `--interface` argument to an `IpAddr`.
///
/// First tries parsing as an IP literal (old behaviour). If that fails,
/// falls back to an interface-name lookup via `getifaddrs`. On
/// Windows or when the interface doesn't exist, returns an
/// actionable error.
pub fn resolve_interface(spec: &str) -> Result<IpAddr> {
    if let Ok(ip) = spec.parse::<IpAddr>() {
        return Ok(ip);
    }

    #[cfg(unix)]
    {
        resolve_by_name(spec)
    }
    #[cfg(not(unix))]
    {
        Err(anyhow!(
            "--interface: '{spec}' is not an IP literal and interface-name \
             resolution is Unix-only (Linux/macOS). Pass the IP address directly."
        ))
    }
}

#[cfg(unix)]
fn resolve_by_name(name: &str) -> Result<IpAddr> {
    use std::ffi::CStr;
    use std::mem::MaybeUninit;

    // SAFETY: getifaddrs / freeifaddrs are the canonical way to walk
    // the system's interface list on Linux/macOS/BSD. We allocate
    // nothing ourselves — the libc-owned list is freed at scope end.
    unsafe {
        let mut head: MaybeUninit<*mut libc::ifaddrs> = MaybeUninit::uninit();
        if libc::getifaddrs(head.as_mut_ptr()) != 0 {
            return Err(anyhow!(
                "--interface: getifaddrs() failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let head = head.assume_init();
        if head.is_null() {
            return Err(anyhow!(
                "--interface: getifaddrs() returned an empty list"
            ));
        }

        let mut first_match: Option<IpAddr> = None;
        let mut seen_interface = false;
        let mut cur = head;
        while !cur.is_null() {
            let entry = &*cur;
            if !entry.ifa_name.is_null() {
                let this_name = CStr::from_ptr(entry.ifa_name).to_string_lossy();
                if this_name == name {
                    seen_interface = true;
                    if let Some(ip) = sockaddr_to_ip(entry.ifa_addr) {
                        if !ip.is_loopback() || first_match.is_none() {
                            first_match = Some(ip);
                            if !ip.is_loopback() {
                                // Prefer non-loopback; stop on the first.
                                break;
                            }
                        }
                    }
                }
            }
            cur = entry.ifa_next;
        }

        libc::freeifaddrs(head);

        match first_match {
            Some(ip) => Ok(ip),
            None if seen_interface => Err(anyhow!(
                "--interface: found '{name}' but it has no assigned IPv4/IPv6 address"
            )),
            None => Err(anyhow!(
                "--interface: no interface named '{name}' (use `ifconfig` / `ip addr` to list)"
            )),
        }
    }
}

#[cfg(unix)]
unsafe fn sockaddr_to_ip(addr: *const libc::sockaddr) -> Option<IpAddr> {
    if addr.is_null() {
        return None;
    }
    // SAFETY: we dereference exactly one byte to check the family.
    match unsafe { (*addr).sa_family as i32 } {
        libc::AF_INET => {
            let p = addr as *const libc::sockaddr_in;
            let octets = unsafe { (*p).sin_addr.s_addr }.to_ne_bytes();
            Some(IpAddr::V4(std::net::Ipv4Addr::from(octets)))
        }
        libc::AF_INET6 => {
            let p = addr as *const libc::sockaddr_in6;
            let octets = unsafe { (*p).sin6_addr.s6_addr };
            Some(IpAddr::V6(std::net::Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_literal_passes_through() {
        assert_eq!(
            resolve_interface("127.0.0.1").unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap(),
        );
        assert_eq!(
            resolve_interface("::1").unwrap(),
            "::1".parse::<IpAddr>().unwrap(),
        );
    }

    #[cfg(unix)]
    #[test]
    fn loopback_interface_resolves_on_unix() {
        // Every Unix has `lo` or `lo0`. At least one should work.
        let lo0 = resolve_interface("lo0");
        let lo = resolve_interface("lo");
        assert!(
            lo0.is_ok() || lo.is_ok(),
            "expected lo or lo0 to resolve; lo0: {lo0:?}, lo: {lo:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unknown_interface_gives_actionable_error() {
        let err = resolve_interface("nonexistent-iface-xyz-123").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no interface named") || msg.contains("has no assigned"),
            "unexpected error message: {msg}"
        );
    }
}
