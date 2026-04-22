//! Frozen snapshot of the CLI flags that scripts inherit as defaults.
//!
//! Script bindings (e.g. `http(url)`) reference these values when the caller
//! doesn't override them via a per-call opts map. Cloned once at engine build
//! time so each binding closure owns its own `ScriptDefaults` — keeps the
//! engine `!Sync` footprint small and avoids threading `&Args` through every
//! binding.

use crate::cli::Args;
use std::path::PathBuf;

#[derive(Clone, Debug)]
#[allow(dead_code)] // fields consumed by probe bindings landed in later tasks
pub struct ScriptDefaults {
    pub headers: Vec<String>,
    pub insecure: bool,
    pub connect_timeout: u64,
    pub max_time: Option<f64>,
    pub follow_redirects: bool,
    pub max_redirs: usize,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub user: Option<String>,
    pub method: Option<String>,
    pub wait_time: f64,
    pub verbose: u8,
    pub ping_count: u32,
    pub max_hops: u8,
    pub tlsv12: bool,
    pub tlsv13: bool,
    pub cacert: Option<PathBuf>,
    pub interface: Option<String>,
    pub limit_rate: Option<String>,
    pub speed_limit: Option<u64>,
    pub speed_time: u64,
    pub dns_servers: Option<String>,
    pub dns_ipv4_addr: Option<String>,
    pub dns_ipv6_addr: Option<String>,
    pub dns_interface: Option<String>,
}

impl ScriptDefaults {
    pub fn from_args(args: &Args) -> Self {
        Self {
            headers: args.header.clone(),
            insecure: args.insecure,
            connect_timeout: args.timeout,
            max_time: args.max_time,
            follow_redirects: args.follow_redirects,
            max_redirs: args.max_redirs,
            user_agent: args.user_agent.clone(),
            referer: args.referer.clone(),
            user: args.user.clone(),
            method: args.method.clone(),
            wait_time: args.wait_time,
            verbose: args.verbose,
            ping_count: args.ping_count,
            max_hops: args.max_hops,
            tlsv12: args.tlsv12,
            tlsv13: args.tlsv13,
            cacert: args.cacert.clone(),
            interface: args.interface.clone(),
            limit_rate: args.limit_rate.clone(),
            speed_limit: args.speed_limit,
            speed_time: args.speed_time,
            dns_servers: args.dns_servers.clone(),
            dns_ipv4_addr: args.dns_ipv4_addr.clone(),
            dns_ipv6_addr: args.dns_ipv6_addr.clone(),
            dns_interface: args.dns_interface.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn from_args_copies_relevant_fields() {
        let args = Args::try_parse_from([
            "recon",
            "--script",
            "/tmp/x.rhai",
            "-H",
            "X-Foo: bar",
            "-k",
            "--connect-timeout",
            "7",
        ])
        .unwrap();
        let d = ScriptDefaults::from_args(&args);
        assert_eq!(d.headers, vec!["X-Foo: bar".to_string()]);
        assert!(d.insecure);
        assert_eq!(d.connect_timeout, 7);
    }
}
