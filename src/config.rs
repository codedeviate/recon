use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Default)]
pub struct ReconConfig {
    pub netstatus: Option<NetstatusConfig>,
    pub editor: Option<EditorConfig>,
    pub ai: Option<AiConfig>,
    #[serde(default)]
    pub sampledata: HashMap<String, SampleDataConfig>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct AiConfig {
    #[serde(default)]
    pub default_backend: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub backends: HashMap<String, AiBackendConfig>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct AiBackendConfig {
    /// argv for user-defined backends. Empty for built-in backends
    /// (`claude`, `codex`, `gemini`) where the entry only carries
    /// `model` and `system` overrides.
    #[serde(default)]
    pub cmd: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub model_flag: Option<String>,
    #[serde(default)]
    pub system_flag: Option<String>,
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

/// Loads the layered config (`/etc/recon/config.toml` + `~/.recon/config.toml`).
/// Both layers are optional; an empty config is the default. Returns an error
/// only when a file that exists fails to read or parse.
pub fn load() -> Result<ReconConfig> {
    let opts = crate::config_resolver::global();
    let value = crate::config_resolver::load_layered("config.toml", &opts)
        .map_err(|e| anyhow!("{e}"))?;
    let config: ReconConfig = value
        .try_into()
        .map_err(|e| anyhow!("Failed to parse merged config: {e}"))?;
    Ok(config)
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

    #[test]
    fn test_parse_ai_config_full() {
        let toml_str = r#"
[ai]
default_backend = "claude"
default_model = "sonnet"
timeout_secs = 90

[ai.backends.claude]
model = "claude-sonnet-4-5"

[ai.backends.my-llm]
cmd = ["my-llm-cli", "--print"]
model_flag = "--model"
system_flag = "--system"
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let ai = config.ai.expect("ai section should parse");
        assert_eq!(ai.default_backend.as_deref(), Some("claude"));
        assert_eq!(ai.default_model.as_deref(), Some("sonnet"));
        assert_eq!(ai.timeout_secs, Some(90));

        let claude = ai.backends.get("claude").expect("claude backend");
        assert_eq!(claude.model.as_deref(), Some("claude-sonnet-4-5"));
        assert!(claude.cmd.is_empty());

        let custom = ai.backends.get("my-llm").expect("my-llm backend");
        assert_eq!(custom.cmd, vec!["my-llm-cli", "--print"]);
        assert_eq!(custom.model_flag.as_deref(), Some("--model"));
        assert_eq!(custom.system_flag.as_deref(), Some("--system"));
    }

    #[test]
    fn test_parse_ai_config_all_optional() {
        let toml_str = r#"
[ai]
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        let ai = config.ai.expect("ai section");
        assert!(ai.default_backend.is_none());
        assert!(ai.default_model.is_none());
        assert!(ai.timeout_secs.is_none());
        assert!(ai.backends.is_empty());
    }

    #[test]
    fn test_ai_section_missing_is_none() {
        let toml_str = r#"
[netstatus]
ip_sources = []
dns_lookup_domains = []
probes = []
"#;
        let config: ReconConfig = toml::from_str(toml_str).unwrap();
        assert!(config.ai.is_none());
    }
}
