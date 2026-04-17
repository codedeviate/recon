//! Fetch canned ecommerce sample data from known APIs, plus a local
//! lorem ipsum generator. Config-overridable.

/// Unit suffix on `--sample-count`, only meaningful for the local `lorem`
/// sample. Non-lorem samples reject a non-`None` unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountUnit {
    /// Paragraphs.
    P,
    /// Words.
    W,
    /// Characters.
    C,
}

/// Parsed `--sample-count` value: a non-negative integer plus an optional
/// single-letter unit suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountSpec {
    pub n: u32,
    pub unit: Option<CountUnit>,
}

/// Parse a `--sample-count` string into a `CountSpec`. Accepts `\d+` or
/// `\d+[pwc]`. Rejects everything else with an error that lists the grammar.
pub fn parse_count(input: &str) -> Result<CountSpec, String> {
    if input.is_empty() {
        return Err(format!(
            "invalid --sample-count '{input}': expected N or N{{p|w|c}}"
        ));
    }

    let bytes = input.as_bytes();
    let last = *bytes.last().unwrap();
    let (digits, unit) = if last.is_ascii_digit() {
        (input, None)
    } else {
        let unit = match last {
            b'p' => CountUnit::P,
            b'w' => CountUnit::W,
            b'c' => CountUnit::C,
            _ => {
                return Err(format!(
                    "invalid --sample-count '{input}': expected N or N{{p|w|c}}"
                ));
            }
        };
        (&input[..input.len() - 1], Some(unit))
    };

    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "invalid --sample-count '{input}': expected N or N{{p|w|c}}"
        ));
    }

    let n: u32 = digits
        .parse()
        .map_err(|_| format!("invalid --sample-count '{input}': number out of range"))?;

    Ok(CountSpec { n, unit })
}

use std::collections::HashMap;

/// Substitute `{{key}}` placeholders in `template` using `vars`. Any
/// placeholder whose key is not in `vars` → error that names the placeholder.
/// Single-brace text (`{` or `}` not part of a `{{…}}` pair) is preserved
/// literally.
pub fn expand_template(
    template: &str,
    vars: &HashMap<&str, String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let close = after.find("}}").ok_or_else(|| {
            format!("unterminated placeholder in template near '{}'", &rest[open..])
        })?;
        let key = &after[..close];
        match vars.get(key) {
            Some(v) => out.push_str(v),
            None => return Err(format!("unknown placeholder {{{{{key}}}}} in template")),
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Expand `${VAR}` references against the process environment. `$$` is an
/// escape for a literal `$`. A bare `$` not followed by `{...}` or another
/// `$` is preserved literally.
pub fn expand_env(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('$') => {
                chars.next();
                out.push('$');
            }
            Some('{') => {
                chars.next(); // consume '{'
                let mut name = String::new();
                let mut closed = false;
                for nc in chars.by_ref() {
                    if nc == '}' {
                        closed = true;
                        break;
                    }
                    name.push(nc);
                }
                if !closed {
                    return Err(format!("unterminated ${{…}} reference in '{input}'"));
                }
                let value = std::env::var(&name).map_err(|_| {
                    format!("config references ${{{name}}} which is not set in the environment")
                })?;
                out.push_str(&value);
            }
            _ => {
                // Bare '$' not followed by '{' or '$' — keep literal.
                out.push('$');
            }
        }
    }
    Ok(out)
}

/// How a sample is fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleMode {
    /// One HTTP call returns many records.
    Bulk,
    /// One HTTP call per record; iterate `count` times.
    PerItem,
    /// No HTTP call; content is generated in-process. Reserved for built-ins.
    Local,
}

/// A sample definition. Built-ins and config-loaded entries share this shape.
#[derive(Debug, Clone)]
pub struct SampleSpec {
    pub mode: SampleMode,
    pub default_format: String,
    pub count: u32,
    pub description: String,
    pub urls: HashMap<String, String>,
    pub headers: Vec<String>,
    pub basic_auth: Option<String>,
    /// Built-in flag (not settable from config): this sample's upstream
    /// endpoint ignores count. Passing `--sample-count` warns instead of
    /// passing through.
    pub count_ignored: bool,
}

impl SampleSpec {
    pub fn supported_formats(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.urls.keys().map(String::as_str).collect();
        v.sort();
        v
    }
}

/// Return the seven built-in samples, keyed by name. This is called on
/// every invocation of `--sample` / `--sample-list`; it's cheap (all
/// data is static) and simpler than a `lazy_static`.
pub fn builtin_samples() -> HashMap<String, SampleSpec> {
    fn entry(
        mode: SampleMode,
        default_format: &str,
        count: u32,
        description: &str,
        urls: &[(&str, &str)],
        count_ignored: bool,
    ) -> SampleSpec {
        SampleSpec {
            mode,
            default_format: default_format.to_string(),
            count,
            description: description.to_string(),
            urls: urls
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            headers: Vec::new(),
            basic_auth: None,
            count_ignored,
        }
    }

    let mut m = HashMap::new();

    m.insert(
        "customer".into(),
        entry(
            SampleMode::Bulk,
            "json",
            10,
            "Customer profiles (users)",
            &[("json", "https://dummyjson.com/users?limit={{count}}")],
            false,
        ),
    );
    m.insert(
        "product".into(),
        entry(
            SampleMode::Bulk,
            "json",
            10,
            "Products with price, category, and images",
            &[("json", "https://dummyjson.com/products?limit={{count}}")],
            false,
        ),
    );
    m.insert(
        "order".into(),
        entry(
            SampleMode::Bulk,
            "json",
            10,
            "Orders / carts with line items",
            &[("json", "https://dummyjson.com/carts?limit={{count}}")],
            false,
        ),
    );
    m.insert(
        "category".into(),
        entry(
            SampleMode::Bulk,
            "json",
            10,
            "Product category list",
            &[("json", "https://dummyjson.com/products/categories")],
            true, // count_ignored: endpoint returns the full list
        ),
    );
    m.insert(
        "address".into(),
        entry(
            SampleMode::Bulk,
            "json",
            10,
            "Postal addresses",
            &[("json", "https://fakerapi.it/api/v2/addresses?_quantity={{count}}")],
            false,
        ),
    );
    m.insert(
        "image".into(),
        entry(
            SampleMode::PerItem,
            "jpg",
            1,
            "Random placeholder image (JPEG)",
            &[("jpg", "https://picsum.photos/400/300")],
            false,
        ),
    );
    m.insert(
        "lorem".into(),
        SampleSpec {
            mode: SampleMode::Local,
            default_format: "txt".into(),
            count: 1,
            description: "Local lorem ipsum text (supports p/w/c units)".into(),
            urls: HashMap::new(),
            headers: Vec::new(),
            basic_auth: None,
            count_ignored: false,
        },
    );

    m
}

/// Where a sample ultimately came from (built-in, user config, or user
/// config overriding a built-in of the same name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleSource {
    BuiltIn,
    Config,
    Overridden,
}

/// A fully-resolved sample ready to execute.
#[derive(Debug, Clone)]
pub struct ResolvedSample {
    pub name: String,
    pub spec: SampleSpec,
    pub format: String,
    pub count: CountSpec,
    pub source_tag: SampleSource,
}

/// Resolve a `--sample <name>` invocation to a ready-to-execute
/// `ResolvedSample`, merging CLI overrides with config and built-ins.
///
/// Resolution precedence: config entry fully replaces built-in of same name.
/// CLI overrides replace the resolved spec's defaults for format and count.
/// Unknown sample / format / unit errors are returned as `Err(String)`.
pub fn resolve(
    name: &str,
    format_override: Option<&str>,
    count_override: Option<CountSpec>,
    config: &HashMap<String, crate::config::SampleDataConfig>,
) -> Result<ResolvedSample, String> {
    let builtins = builtin_samples();
    let in_config = config.contains_key(name);
    let in_builtin = builtins.contains_key(name);

    let (spec, source_tag) = match (in_config, in_builtin) {
        (true, true) => (spec_from_config(name, &config[name])?, SampleSource::Overridden),
        (true, false) => (spec_from_config(name, &config[name])?, SampleSource::Config),
        (false, true) => (builtins[name].clone(), SampleSource::BuiltIn),
        (false, false) => {
            return Err(format!(
                "unknown sample '{name}'; try --sample-list"
            ));
        }
    };

    let format = format_override.map(str::to_string).unwrap_or_else(|| spec.default_format.clone());

    // Validate format against the spec's URL map — unless mode is Local
    // (where urls is always empty and format is a cosmetic extension only).
    if spec.mode != SampleMode::Local && !spec.urls.contains_key(&format) {
        let mut supported = spec.supported_formats();
        supported.sort();
        return Err(format!(
            "sample '{name}' does not support format '{format}'; supported: {}",
            supported.join(", ")
        ));
    }

    let count = match count_override {
        Some(c) => c,
        None => CountSpec { n: spec.count, unit: None },
    };

    // Unit suffix is only valid for the local lorem built-in (SampleMode::Local).
    if count.unit.is_some() && spec.mode != SampleMode::Local {
        return Err(format!(
            "sample '{name}' does not accept count units"
        ));
    }

    Ok(ResolvedSample {
        name: name.to_string(),
        spec,
        format,
        count,
        source_tag,
    })
}

/// One entry for `--sample-list` output. Separate from `SampleSpec` because
/// we want to show merged built-in + config state at a glance without re-running
/// the full `resolve` validation.
#[derive(Debug, Clone)]
pub struct SampleListEntry {
    pub name: String,
    pub description: String,
    pub mode: SampleMode,
    pub default_format: String,
    pub formats: Vec<String>,
    pub count: u32,
    pub source_tag: SampleSource,
}

/// Produce a sorted list of all samples (built-in and config-defined),
/// without making any HTTP requests. Config entries override built-ins
/// of the same name.
pub fn list_samples(
    config: &HashMap<String, crate::config::SampleDataConfig>,
) -> Vec<SampleListEntry> {
    let builtins = builtin_samples();
    let mut names: Vec<String> = builtins.keys().cloned().collect();
    for k in config.keys() {
        if !names.iter().any(|n| n == k) {
            names.push(k.clone());
        }
    }
    names.sort();

    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let (spec, tag) = match (config.contains_key(&name), builtins.contains_key(&name)) {
            (true, true) => match spec_from_config(&name, &config[&name]) {
                Ok(s) => (s, SampleSource::Overridden),
                Err(_) => (builtins[&name].clone(), SampleSource::BuiltIn),
            },
            (true, false) => match spec_from_config(&name, &config[&name]) {
                Ok(s) => (s, SampleSource::Config),
                // Config-only entry that won't load: skip in the listing.
                Err(_) => continue,
            },
            (false, true) => (builtins[&name].clone(), SampleSource::BuiltIn),
            (false, false) => continue,
        };
        let mut formats: Vec<String> = spec.urls.keys().cloned().collect();
        formats.sort();
        if formats.is_empty() {
            formats.push(spec.default_format.clone()); // local mode
        }
        out.push(SampleListEntry {
            name,
            description: spec.description.clone(),
            mode: spec.mode,
            default_format: spec.default_format.clone(),
            formats,
            count: spec.count,
            source_tag: tag,
        });
    }
    out
}

/// Parsed form of the raw `--sample` CLI value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleArg {
    pub name: String,
    pub format: Option<String>,
    pub count: Option<CountSpec>,
}

/// Parse `NAME[:FORMAT[:COUNT]]`. Empty format/count slots produce `None`.
/// Errors on empty name or more than three colon-separated parts.
pub fn parse_sample_arg(raw: &str) -> Result<SampleArg, String> {
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() > 3 {
        return Err(format!(
            "invalid --sample value '{raw}': expected NAME[:FORMAT[:COUNT]]"
        ));
    }
    let name = parts[0].trim();
    if name.is_empty() {
        return Err(format!(
            "invalid --sample value '{raw}': name is empty"
        ));
    }
    let format = parts.get(1).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let count = match parts.get(2).map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(c) => Some(parse_count(c)?),
        None => None,
    };
    Ok(SampleArg {
        name: name.to_string(),
        format,
        count,
    })
}

/// Build a `SampleSpec` from a user config entry. Reports errors for
/// missing-required-field combinations and for the reserved `mode = "local"`.
fn spec_from_config(
    name: &str,
    cfg: &crate::config::SampleDataConfig,
) -> Result<SampleSpec, String> {
    let mode = match cfg.mode.as_deref().unwrap_or("bulk") {
        "bulk" => SampleMode::Bulk,
        "per_item" => SampleMode::PerItem,
        "local" => {
            return Err(format!(
                "sample '{name}': mode = \"local\" is reserved for built-in samples"
            ));
        }
        other => {
            return Err(format!(
                "sample '{name}': unknown mode '{other}'; expected 'bulk' or 'per_item'"
            ));
        }
    };

    let default_format = cfg
        .default_format
        .clone()
        .ok_or_else(|| format!("sample '{name}': 'default_format' is required"))?;

    if cfg.urls.is_empty() {
        return Err(format!(
            "sample '{name}': at least one URL is required under [sampledata.{name}.urls]"
        ));
    }
    if !cfg.urls.contains_key(&default_format) {
        return Err(format!(
            "sample '{name}': default_format '{default_format}' has no matching urls.{default_format}"
        ));
    }

    // Resolve env vars in URLs, headers, basic_auth at load time.
    let mut urls = HashMap::new();
    for (k, v) in &cfg.urls {
        urls.insert(k.clone(), expand_env(v)?);
    }
    let mut headers = Vec::with_capacity(cfg.headers.len());
    for h in &cfg.headers {
        headers.push(expand_env(h)?);
    }
    let basic_auth = match &cfg.basic_auth {
        Some(s) => Some(expand_env(s)?),
        None => None,
    };

    Ok(SampleSpec {
        mode,
        default_format,
        count: cfg.count.unwrap_or(10),
        description: cfg
            .description
            .clone()
            .unwrap_or_else(|| format!("User-defined sample '{name}'")),
        urls,
        headers,
        basic_auth,
        count_ignored: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_count_plain_number() {
        assert_eq!(parse_count("10"), Ok(CountSpec { n: 10, unit: None }));
        assert_eq!(parse_count("0"), Ok(CountSpec { n: 0, unit: None }));
        assert_eq!(parse_count("1000000"), Ok(CountSpec { n: 1_000_000, unit: None }));
    }

    #[test]
    fn parse_count_with_unit_suffix() {
        assert_eq!(parse_count("2p"), Ok(CountSpec { n: 2, unit: Some(CountUnit::P) }));
        assert_eq!(parse_count("50w"), Ok(CountSpec { n: 50, unit: Some(CountUnit::W) }));
        assert_eq!(parse_count("1000c"), Ok(CountSpec { n: 1000, unit: Some(CountUnit::C) }));
    }

    #[test]
    fn parse_count_rejects_invalid() {
        assert!(parse_count("").is_err());
        assert!(parse_count("abc").is_err());
        assert!(parse_count("10x").is_err());
        assert!(parse_count("p10").is_err());
        assert!(parse_count("50ww").is_err());
        assert!(parse_count("5.0").is_err());
        assert!(parse_count("-3").is_err());
        assert!(parse_count("p").is_err());
    }

    #[test]
    fn parse_count_error_message() {
        let err = parse_count("10x").unwrap_err();
        assert!(err.contains("10x"), "error should echo input, got: {err}");
        assert!(err.contains("N{p|w|c}") || err.contains("p|w|c"),
            "error should describe grammar, got: {err}");
    }

    use std::collections::HashMap;

    #[test]
    fn expand_template_substitutes_known_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("count", "10".to_string());
        vars.insert("format", "json".to_string());
        let out = expand_template(
            "https://api/x?limit={{count}}&fmt={{format}}",
            &vars,
        ).unwrap();
        assert_eq!(out, "https://api/x?limit=10&fmt=json");
    }

    #[test]
    fn expand_template_errors_on_unknown_placeholder() {
        let vars = HashMap::new();
        let err = expand_template("hello {{name}}", &vars).unwrap_err();
        assert!(err.contains("{{name}}"), "error should name the placeholder: {err}");
    }

    #[test]
    fn expand_template_preserves_literal_braces_when_not_placeholder() {
        let vars = HashMap::new();
        // Single braces should pass through untouched.
        let out = expand_template("plain {x} text", &vars).unwrap();
        assert_eq!(out, "plain {x} text");
    }

    #[test]
    fn expand_env_substitutes() {
        std::env::set_var("RECON_SAMPLE_TEST_A", "hello");
        let out = expand_env("prefix-${RECON_SAMPLE_TEST_A}-suffix").unwrap();
        assert_eq!(out, "prefix-hello-suffix");
        std::env::remove_var("RECON_SAMPLE_TEST_A");
    }

    #[test]
    fn expand_env_errors_on_unset_var() {
        std::env::remove_var("RECON_SAMPLE_DEFINITELY_UNSET");
        let err = expand_env("${RECON_SAMPLE_DEFINITELY_UNSET}").unwrap_err();
        assert!(err.contains("RECON_SAMPLE_DEFINITELY_UNSET"),
            "error should name the variable: {err}");
    }

    #[test]
    fn expand_env_escapes_double_dollar() {
        let out = expand_env("cost is $$5").unwrap();
        assert_eq!(out, "cost is $5");
    }

    #[test]
    fn expand_env_leaves_standalone_dollar_alone() {
        let out = expand_env("no vars here $").unwrap();
        assert_eq!(out, "no vars here $");
    }

    #[test]
    fn builtin_samples_contains_expected_names() {
        let all = builtin_samples();
        for name in ["customer", "product", "order", "category", "address", "image", "lorem"] {
            assert!(all.contains_key(name), "missing built-in: {name}");
        }
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn builtin_lorem_is_local_mode() {
        let all = builtin_samples();
        let lorem = all.get("lorem").unwrap();
        assert_eq!(lorem.mode, SampleMode::Local);
        assert!(lorem.urls.is_empty(), "local mode has no URLs");
        assert_eq!(lorem.default_format, "txt");
    }

    #[test]
    fn builtin_image_is_per_item_mode() {
        let all = builtin_samples();
        let image = all.get("image").unwrap();
        assert_eq!(image.mode, SampleMode::PerItem);
        assert!(image.urls.contains_key("jpg"));
    }

    #[test]
    fn builtin_customer_is_bulk_mode() {
        let all = builtin_samples();
        let c = all.get("customer").unwrap();
        assert_eq!(c.mode, SampleMode::Bulk);
        assert!(c.urls.get("json").unwrap().contains("{{count}}"));
    }

    #[test]
    fn builtin_category_has_count_ignored_flag() {
        let all = builtin_samples();
        let cat = all.get("category").unwrap();
        assert!(cat.count_ignored, "category should have count_ignored = true");
    }

    use crate::config::SampleDataConfig;

    fn empty_cfg() -> HashMap<String, SampleDataConfig> {
        HashMap::new()
    }

    #[test]
    fn resolve_builtin_customer_defaults() {
        let r = resolve("customer", None, None, &empty_cfg()).unwrap();
        assert_eq!(r.name, "customer");
        assert_eq!(r.format, "json");
        assert_eq!(r.count.n, 10);
        assert_eq!(r.spec.mode, SampleMode::Bulk);
    }

    #[test]
    fn resolve_cli_overrides() {
        let r = resolve("customer", Some("json"), Some(CountSpec { n: 25, unit: None }), &empty_cfg()).unwrap();
        assert_eq!(r.count.n, 25);
    }

    #[test]
    fn resolve_unknown_sample() {
        let err = resolve("doesnotexist", None, None, &empty_cfg()).unwrap_err();
        assert!(err.contains("doesnotexist"));
        assert!(err.contains("--sample-list"));
    }

    #[test]
    fn resolve_unknown_format_for_sample() {
        let err = resolve("customer", Some("xml"), None, &empty_cfg()).unwrap_err();
        assert!(err.contains("customer"));
        assert!(err.contains("xml"));
        assert!(err.contains("json"), "error should list supported formats");
    }

    #[test]
    fn resolve_unit_rejected_for_non_lorem() {
        let err = resolve(
            "customer",
            None,
            Some(CountSpec { n: 5, unit: Some(CountUnit::P) }),
            &empty_cfg(),
        ).unwrap_err();
        assert!(err.contains("customer"));
        assert!(err.contains("unit"));
    }

    #[test]
    fn resolve_unit_accepted_for_lorem() {
        let r = resolve(
            "lorem",
            None,
            Some(CountSpec { n: 3, unit: Some(CountUnit::P) }),
            &empty_cfg(),
        ).unwrap();
        assert_eq!(r.count.n, 3);
        assert_eq!(r.count.unit, Some(CountUnit::P));
    }

    #[test]
    fn resolve_config_overrides_builtin() {
        let mut cfg = HashMap::new();
        cfg.insert(
            "customer".into(),
            SampleDataConfig {
                mode: Some("bulk".into()),
                default_format: Some("xml".into()),
                count: Some(50),
                description: Some("Custom customer".into()),
                urls: {
                    let mut u = HashMap::new();
                    u.insert("xml".into(), "https://internal/x?n={{count}}".into());
                    u
                },
                headers: vec![],
                basic_auth: None,
            },
        );
        let r = resolve("customer", None, None, &cfg).unwrap();
        assert_eq!(r.format, "xml");
        assert_eq!(r.count.n, 50);
        assert_eq!(r.source_tag, SampleSource::Overridden);
    }

    #[test]
    fn resolve_config_new_sample() {
        let mut cfg = HashMap::new();
        cfg.insert(
            "myapi".into(),
            SampleDataConfig {
                mode: Some("bulk".into()),
                default_format: Some("json".into()),
                count: Some(7),
                description: None,
                urls: {
                    let mut u = HashMap::new();
                    u.insert("json".into(), "https://my.internal/x".into());
                    u
                },
                headers: vec![],
                basic_auth: None,
            },
        );
        let r = resolve("myapi", None, None, &cfg).unwrap();
        assert_eq!(r.source_tag, SampleSource::Config);
        assert_eq!(r.count.n, 7);
    }

    #[test]
    fn resolve_rejects_config_mode_local() {
        let mut cfg = HashMap::new();
        cfg.insert(
            "bad".into(),
            SampleDataConfig {
                mode: Some("local".into()),
                default_format: Some("txt".into()),
                count: None,
                description: None,
                urls: HashMap::new(),
                headers: vec![],
                basic_auth: None,
            },
        );
        let err = resolve("bad", None, None, &cfg).unwrap_err();
        assert!(err.contains("local"));
    }

    #[test]
    fn list_samples_merges_builtins_and_config() {
        let mut cfg = HashMap::new();
        cfg.insert(
            "customer".into(), // overrides built-in
            SampleDataConfig {
                mode: Some("bulk".into()),
                default_format: Some("json".into()),
                count: Some(50),
                description: Some("Override".into()),
                urls: {
                    let mut u = HashMap::new();
                    u.insert("json".into(), "https://x/users".into());
                    u
                },
                headers: vec![],
                basic_auth: None,
            },
        );
        cfg.insert(
            "myapi".into(), // new name
            SampleDataConfig {
                mode: Some("bulk".into()),
                default_format: Some("json".into()),
                count: Some(5),
                description: None,
                urls: {
                    let mut u = HashMap::new();
                    u.insert("json".into(), "https://my/api".into());
                    u
                },
                headers: vec![],
                basic_auth: None,
            },
        );

        let list = list_samples(&cfg);
        assert!(list.iter().any(|e| e.name == "customer" && e.source_tag == SampleSource::Overridden));
        assert!(list.iter().any(|e| e.name == "myapi" && e.source_tag == SampleSource::Config));
        assert!(list.iter().any(|e| e.name == "product" && e.source_tag == SampleSource::BuiltIn));
    }

    #[test]
    fn parse_sample_arg_plain_name() {
        let p = parse_sample_arg("customer").unwrap();
        assert_eq!(p.name, "customer");
        assert_eq!(p.format, None);
        assert_eq!(p.count, None);
    }

    #[test]
    fn parse_sample_arg_with_format() {
        let p = parse_sample_arg("customer:csv").unwrap();
        assert_eq!(p.name, "customer");
        assert_eq!(p.format.as_deref(), Some("csv"));
        assert_eq!(p.count, None);
    }

    #[test]
    fn parse_sample_arg_with_all_three() {
        let p = parse_sample_arg("customer:csv:25").unwrap();
        assert_eq!(p.name, "customer");
        assert_eq!(p.format.as_deref(), Some("csv"));
        assert_eq!(p.count, Some(CountSpec { n: 25, unit: None }));
    }

    #[test]
    fn parse_sample_arg_empty_slots_are_none() {
        let p = parse_sample_arg("customer::5").unwrap();
        assert_eq!(p.name, "customer");
        assert_eq!(p.format, None);
        assert_eq!(p.count, Some(CountSpec { n: 5, unit: None }));

        let p = parse_sample_arg("customer:csv:").unwrap();
        assert_eq!(p.format.as_deref(), Some("csv"));
        assert_eq!(p.count, None);
    }

    #[test]
    fn parse_sample_arg_empty_name_errors() {
        assert!(parse_sample_arg("").is_err());
        assert!(parse_sample_arg(":csv").is_err());
    }

    #[test]
    fn parse_sample_arg_too_many_parts_errors() {
        assert!(parse_sample_arg("customer:csv:5:extra").is_err());
    }

    #[test]
    fn parse_sample_arg_lorem_with_unit() {
        let p = parse_sample_arg("lorem::3p").unwrap();
        assert_eq!(p.count, Some(CountSpec { n: 3, unit: Some(CountUnit::P) }));
    }
}
