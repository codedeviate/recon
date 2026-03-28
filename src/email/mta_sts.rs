use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::{CheckResult, Detail, Verdict, lookup_txt, lookup_mx};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn humanize_seconds(secs: u64) -> String {
    if secs >= 86400 {
        let days = secs / 86400;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    } else if secs >= 3600 {
        let hours = secs / 3600;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else {
        let minutes = secs / 60;
        format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" })
    }
}

/// Returns true if the MTA-STS MX pattern matches the given hostname.
/// Wildcard `*.example.com` matches `mail.example.com` (one label only).
fn mx_pattern_matches(pattern: &str, hostname: &str) -> bool {
    let pattern = pattern.trim_end_matches('.');
    let hostname = hostname.trim_end_matches('.');

    if let Some(suffix) = pattern.strip_prefix("*.") {
        // Wildcard: matches exactly one label before the suffix
        if let Some(rest) = hostname.strip_suffix(suffix) {
            let label = rest.trim_end_matches('.');
            // Must be a single label (no dots)
            return !label.is_empty() && !label.contains('.');
        }
        false
    } else {
        pattern.eq_ignore_ascii_case(hostname)
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn check(resolver: &TokioAsyncResolver, host: &str, insecure: bool) -> Result<CheckResult> {
    let mut details: Vec<Detail> = Vec::new();
    let mut overall = Verdict::Pass;

    // ── Part A: DNS record ────────────────────────────────────────────────────

    let dns_name = format!("_mta-sts.{}", host);
    let txt_records = lookup_txt(resolver, &dns_name).await?;

    let sts_records: Vec<&String> = txt_records
        .iter()
        .filter(|r| r.starts_with("v=STSv1") || r.contains("v=STSv1"))
        .filter(|r| {
            // More precise: must have v=STSv1 as a tag
            r.split(';').any(|part| part.trim() == "v=STSv1")
        })
        .collect();

    if sts_records.is_empty() {
        return Ok(CheckResult {
            name: "MTA-STS".to_string(),
            verdict: Verdict::Fail,
            summary: format!("No MTA-STS DNS record found at {}", dns_name),
            details: vec![
                Detail::with_verdict(
                    Verdict::Fail,
                    format!("DNS TXT lookup: {} — no v=STSv1 record", dns_name),
                ),
            ],
        });
    }

    let dns_record = sts_records[0].clone();
    details.push(Detail::new(format!("DNS record: {}", dns_record)));

    // Parse DNS record tags (space-separated tag=value)
    let mut dns_tags: Vec<(String, String)> = Vec::new();
    for part in dns_record.split(';') {
        let part = part.trim();
        if part.is_empty() { continue; }
        if let Some(eq) = part.find('=') {
            let tag = part[..eq].trim().to_string();
            let val = part[eq + 1..].trim().to_string();
            dns_tags.push((tag, val));
        }
    }

    let get_dns = |name: &str| -> Option<String> {
        dns_tags.iter().find(|(t, _)| t == name).map(|(_, v)| v.clone())
    };

    // Validate v=STSv1
    match dns_tags.first() {
        Some((t, v)) if t == "v" && v == "STSv1" => {
            details.push(Detail::with_verdict(Verdict::Pass, "v=STSv1 — valid version tag"));
        }
        _ => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "v=STSv1 must be the first tag in the DNS record",
            ));
        }
    }

    // Validate id= present
    match get_dns("id") {
        None => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "id= tag is required but missing from DNS record",
            ));
        }
        Some(id) => {
            details.push(Detail::with_verdict(
                Verdict::Pass,
                format!("id={} — policy identifier", id),
            ));
        }
    }

    // ── Part B: HTTPS policy fetch ────────────────────────────────────────────

    let policy_url = format!("https://mta-sts.{}/.well-known/mta-sts.txt", host);
    details.push(Detail::new(format!("Fetching policy: {}", policy_url)));

    let policy_body = {
        let url = policy_url.clone();
        let result = tokio::task::spawn_blocking(move || {
            reqwest::blocking::Client::builder()
                .danger_accept_invalid_certs(insecure)
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .and_then(|client| client.get(&url).send())
                .and_then(|resp| resp.text())
        })
        .await?;

        match result {
            Ok(body) => body,
            Err(e) => {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(
                    Verdict::Fail,
                    format!("Failed to fetch policy file: {}", e),
                ));
                return Ok(CheckResult {
                    name: "MTA-STS".to_string(),
                    verdict: overall,
                    summary: format!("MTA-STS DNS record found but policy fetch failed: {}", e),
                    details,
                });
            }
        }
    };

    // ── Parse policy file ─────────────────────────────────────────────────────

    let mut policy_version: Option<String> = None;
    let mut policy_mode: Option<String> = None;
    let mut policy_max_age: Option<u64> = None;
    let mut policy_mx: Vec<String> = Vec::new();

    for line in policy_body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim();
            let val = line[colon + 1..].trim();
            match key {
                "version" => policy_version = Some(val.to_string()),
                "mode"    => policy_mode = Some(val.to_string()),
                "max_age" => {
                    match val.parse::<u64>() {
                        Ok(n) => policy_max_age = Some(n),
                        Err(_) => {
                            overall = overall.merge(Verdict::Fail);
                            details.push(Detail::with_verdict(
                                Verdict::Fail,
                                format!("max_age value is not a valid integer: {}", val),
                            ));
                        }
                    }
                }
                "mx" => policy_mx.push(val.to_string()),
                _ => {} // ignore unknown keys
            }
        }
    }

    // Validate version: STSv1
    match policy_version.as_deref() {
        Some("STSv1") => {
            details.push(Detail::with_verdict(Verdict::Pass, "version: STSv1"));
        }
        Some(v) => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                format!("version: {} — expected STSv1", v),
            ));
        }
        None => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "Policy missing required field: version",
            ));
        }
    }

    // Validate mode
    let mode_verdict = match policy_mode.as_deref() {
        None => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "Policy missing required field: mode",
            ));
            None
        }
        Some("enforce") => {
            details.push(Detail::with_verdict(
                Verdict::Pass,
                "mode: enforce — TLS is required; non-compliant mail is rejected",
            ));
            Some("enforce")
        }
        Some("testing") => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                "mode: testing — policy is in testing mode; failures are reported but not enforced",
            ));
            Some("testing")
        }
        Some("none") => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                "mode: none — policy is disabled; no enforcement",
            ));
            Some("none")
        }
        Some(m) => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                format!("mode: {} — unrecognised mode value", m),
            ));
            None
        }
    };

    // Validate max_age
    match policy_max_age {
        None => {
            if policy_body.lines().any(|l| l.trim_start().starts_with("max_age")) {
                // already handled parse error above
            } else {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(
                    Verdict::Fail,
                    "Policy missing required field: max_age",
                ));
            }
        }
        Some(0) => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "max_age: 0 — must be greater than 0",
            ));
        }
        Some(secs) => {
            let human = humanize_seconds(secs);
            if secs < 86400 {
                overall = overall.merge(Verdict::Warn);
                details.push(Detail::with_verdict(
                    Verdict::Warn,
                    format!("max_age: {} ({}) — too short; recommended minimum is 86400 (1 day)", secs, human),
                ));
            } else if secs > 31_557_600 {
                overall = overall.merge(Verdict::Warn);
                details.push(Detail::with_verdict(
                    Verdict::Warn,
                    format!("max_age: {} ({}) — over 1 year; very long cache lifetime", secs, human),
                ));
            } else {
                details.push(Detail::with_verdict(
                    Verdict::Pass,
                    format!("max_age: {} ({})", secs, human),
                ));
            }
        }
    }

    // Validate mx patterns
    if policy_mx.is_empty() {
        overall = overall.merge(Verdict::Fail);
        details.push(Detail::with_verdict(
            Verdict::Fail,
            "Policy must contain at least one mx: entry",
        ));
    } else {
        for pattern in &policy_mx {
            details.push(Detail::new(format!("mx: {}", pattern)));
        }
    }

    // ── MX cross-check ────────────────────────────────────────────────────────

    match lookup_mx(resolver, host).await {
        Err(e) => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                format!("MX lookup failed — cannot cross-check against policy: {}", e),
            ));
        }
        Ok(mx_records) if mx_records.is_empty() => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                "No MX records found — cannot cross-check against policy",
            ));
        }
        Ok(mx_records) => {
            details.push(Detail::new(format!(
                "MX records: {}",
                mx_records
                    .iter()
                    .map(|(pref, exch)| format!("{} ({})", exch, pref))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));

            for (_, mx_host) in &mx_records {
                let mx_clean = mx_host.trim_end_matches('.');
                let matched = policy_mx.iter().any(|pat| mx_pattern_matches(pat, mx_clean));
                if matched {
                    details.push(Detail::with_verdict(
                        Verdict::Pass,
                        format!("MX {} — matched by policy", mx_clean),
                    ));
                } else {
                    overall = overall.merge(Verdict::Warn);
                    details.push(Detail::with_verdict(
                        Verdict::Warn,
                        format!("MX {} — not matched by any policy mx: pattern", mx_clean),
                    ));
                }
            }
        }
    }

    // ── Summary ───────────────────────────────────────────────────────────────

    let summary = match mode_verdict {
        Some(mode) => format!(
            "MTA-STS policy found — mode: {}, {} MX pattern{}",
            mode,
            policy_mx.len(),
            if policy_mx.len() == 1 { "" } else { "s" }
        ),
        None => "MTA-STS policy found but mode is missing or invalid".to_string(),
    };

    Ok(CheckResult {
        name: "MTA-STS".to_string(),
        verdict: overall,
        summary,
        details,
    })
}
