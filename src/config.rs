use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct ReconConfig {
    pub netstatus: Option<NetstatusConfig>,
}

#[derive(Deserialize, Default)]
pub struct NetstatusConfig {
    #[serde(default)]
    pub ip_sources: Vec<String>,
    #[serde(default)]
    pub dns_lookup_domains: Vec<String>,
    #[serde(default)]
    pub probes: Vec<String>,
    #[serde(default)]
    pub dns_hijack_checks: Vec<DnsHijackCheck>,
}

#[derive(Deserialize, Clone)]
pub struct DnsHijackCheck {
    pub server: String,
    pub domain: String,
    pub expected: String,
}

impl NetstatusConfig {
    /// Returns an error if the config is internally inconsistent.
    pub fn validate(&self) -> Result<()> {
        let has_dns_probe = self.probes.iter().any(|p| p.starts_with("dns://"));
        if has_dns_probe && self.dns_lookup_domains.is_empty() {
            return Err(anyhow!(
                "dns:// probes require at least one entry in dns_lookup_domains"
            ));
        }
        Ok(())
    }
}

/// Loads ~/.recon/config.toml. Returns an error if the file is missing or invalid.
pub fn load() -> Result<ReconConfig> {
    let path = config_path();
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "Cannot read config file: {}\n\
             Create it with a [netstatus] section — see: recon --help netstatus",
            path.display()
        )
    })?;
    let config: ReconConfig =
        toml::from_str(&text).map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
    Ok(config)
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".recon").join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let toml_str = r#"
[netstatus]
ip_sources = ["https://api.ipify.org", "https://ifconfig.me/ip"]
dns_lookup_domains = ["example.com"]
probes = ["https://www.google.com", "ping://8.8.8.8"]
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let ns = config.netstatus.unwrap();
        assert_eq!(ns.ip_sources.len(), 2);
        assert_eq!(ns.dns_lookup_domains, vec!["example.com"]);
        assert_eq!(ns.probes.len(), 2);
        assert!(ns.dns_hijack_checks.is_empty());
    }

    #[test]
    fn test_parse_dns_hijack_checks() {
        let toml_str = r#"
[netstatus]
ip_sources = []
dns_lookup_domains = ["example.com"]
probes = []

[[netstatus.dns_hijack_checks]]
server = "8.8.8.8"
domain = "example.com"
expected = "93.184.216.34"

[[netstatus.dns_hijack_checks]]
server = "1.1.1.1"
domain = "example.com"
expected = "93.184.216.34"
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let ns = config.netstatus.unwrap();
        assert_eq!(ns.dns_hijack_checks.len(), 2);
        assert_eq!(ns.dns_hijack_checks[0].server, "8.8.8.8");
        assert_eq!(ns.dns_hijack_checks[1].server, "1.1.1.1");
    }

    #[test]
    fn test_validate_dns_probe_requires_lookup_domains() {
        let config = NetstatusConfig {
            dns_lookup_domains: vec![],
            probes: vec!["dns://8.8.8.8".to_string()],
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_passes_when_no_dns_probes() {
        let config = NetstatusConfig {
            dns_lookup_domains: vec![],
            probes: vec!["https://www.google.com".to_string()],
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }
}
