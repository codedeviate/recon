use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Default)]
pub struct ReconConfig {
    pub netstatus: Option<NetstatusConfig>,
    pub editor: Option<EditorConfig>,
    #[serde(default)]
    pub sampledata: HashMap<String, SampleDataConfig>,
}

#[derive(Deserialize, Default, Debug)]
pub struct EditorConfig {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct SampleDataConfig {
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub default_format: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub urls: HashMap<String, String>,
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub basic_auth: Option<String>,
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

    #[test]
    fn test_parse_editor_config() {
        let toml_str = r#"
[editor]
default = "zed"

[editor.aliases]
mycode = "code --new-window"
altzed = "zed --dev"
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let editor = config.editor.expect("editor section should parse");
        assert_eq!(editor.default.as_deref(), Some("zed"));
        assert_eq!(
            editor.aliases.get("mycode").map(String::as_str),
            Some("code --new-window"),
        );
        assert_eq!(
            editor.aliases.get("altzed").map(String::as_str),
            Some("zed --dev"),
        );
    }

    #[test]
    fn test_editor_config_all_optional() {
        let toml_str = r#"
[editor]
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let editor = config.editor.expect("editor section should parse");
        assert!(editor.default.is_none());
        assert!(editor.aliases.is_empty());
    }

    #[test]
    fn test_editor_section_missing_is_none() {
        let toml_str = r#"
[netstatus]
ip_sources = []
dns_lookup_domains = []
probes = []
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        assert!(config.editor.is_none());
    }

    #[test]
    fn test_parse_sampledata_full_entry() {
        let toml_str = r#"
[sampledata.customer]
mode = "bulk"
default_format = "json"
count = 25
description = "Customer profiles"
urls.json = "https://api.example.com/users?limit={{count}}"
urls.csv  = "https://api.example.com/users.csv?n={{count}}"
headers = ["Authorization: Bearer xxx", "X-Tenant: acme"]
basic_auth = "alice:secret"
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let s = config.sampledata.get("customer").expect("present");
        assert_eq!(s.mode.as_deref(), Some("bulk"));
        assert_eq!(s.default_format.as_deref(), Some("json"));
        assert_eq!(s.count, Some(25));
        assert_eq!(s.description.as_deref(), Some("Customer profiles"));
        assert_eq!(s.urls.len(), 2);
        assert_eq!(
            s.urls.get("json").map(String::as_str),
            Some("https://api.example.com/users?limit={{count}}"),
        );
        assert_eq!(s.headers.len(), 2);
        assert_eq!(s.basic_auth.as_deref(), Some("alice:secret"));
    }

    #[test]
    fn test_parse_sampledata_minimal_entry() {
        let toml_str = r#"
[sampledata.foo]
default_format = "json"
urls.json = "https://example.com/foo"
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let s = config.sampledata.get("foo").expect("present");
        assert!(s.mode.is_none());
        assert_eq!(s.default_format.as_deref(), Some("json"));
        assert!(s.count.is_none());
        assert!(s.headers.is_empty());
        assert!(s.basic_auth.is_none());
    }

    #[test]
    fn test_sampledata_missing_is_empty_map() {
        let toml_str = r#"
[netstatus]
ip_sources = []
dns_lookup_domains = []
probes = []
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        assert!(config.sampledata.is_empty());
    }
}
