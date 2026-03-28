use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::{CheckResult, Detail, Verdict, lookup_txt};

// ── Policy strength ──────────────────────────────────────────────────────────

fn policy_strength(p: &str) -> u8 {
    match p {
        "none"       => 0,
        "quarantine" => 1,
        "reject"     => 2,
        _            => 0,
    }
}

// ── RUA URI parsing ──────────────────────────────────────────────────────────

/// Extract the domain from a mailto: address, e.g. "mailto:dmarc@example.com" → "example.com"
fn mailto_domain(uri: &str) -> Option<&str> {
    let addr = uri.strip_prefix("mailto:")?;
    let at = addr.find('@')?;
    Some(&addr[at + 1..])
}

// ── Public entry point ───────────────────────────────────────────────────────

pub async fn check(resolver: &TokioAsyncResolver, host: &str) -> Result<CheckResult> {
    let query = format!("_dmarc.{}", host);
    let txt_records = lookup_txt(resolver, &query).await?;

    // Filter to DMARC records
    let dmarc_records: Vec<&String> = txt_records
        .iter()
        .filter(|r| r.starts_with("v=DMARC1"))
        .collect();

    // ── No record ────────────────────────────────────────────────────────────
    if dmarc_records.is_empty() {
        return Ok(CheckResult {
            name: "DMARC".to_string(),
            verdict: Verdict::Fail,
            summary: format!("No DMARC record found at {}", query),
            details: vec![
                Detail::with_verdict(Verdict::Fail, format!("DNS TXT lookup: {} — no v=DMARC1 record", query)),
            ],
        });
    }

    // ── Multiple records → PermError ─────────────────────────────────────────
    if dmarc_records.len() > 1 {
        return Ok(CheckResult {
            name: "DMARC".to_string(),
            verdict: Verdict::Fail,
            summary: format!("Multiple DMARC records found at {} (PermError)", query),
            details: vec![
                Detail::with_verdict(
                    Verdict::Fail,
                    format!("Found {} v=DMARC1 records — RFC 7489 requires exactly one", dmarc_records.len()),
                ),
            ],
        });
    }

    let record = dmarc_records[0].clone();
    let mut details: Vec<Detail> = Vec::new();
    let mut overall = Verdict::Pass;

    details.push(Detail::new(format!("Record: {}", record)));

    // ── Tag parsing ──────────────────────────────────────────────────────────

    let known_tags = &[
        "v", "p", "sp", "rua", "ruf", "adkim", "aspf", "pct", "rf", "ri", "fo",
    ];

    let mut tags: Vec<(String, String)> = Vec::new();
    for part in record.split(';') {
        let part = part.trim();
        if part.is_empty() { continue; }
        if let Some(eq) = part.find('=') {
            let tag = part[..eq].trim().to_string();
            let val = part[eq + 1..].trim().to_string();
            tags.push((tag, val));
        }
    }

    // v=DMARC1 must be first tag
    match tags.first() {
        Some((t, v)) if t == "v" && v == "DMARC1" => {}
        _ => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "v=DMARC1 must be the first tag".to_string(),
            ));
        }
    }

    // ── Warn on unknown tags ─────────────────────────────────────────────────
    for (tag, _) in &tags {
        if !known_tags.contains(&tag.as_str()) {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                format!("Unknown tag: {}", tag),
            ));
        }
    }

    // Helper: find tag value
    let get = |name: &str| -> Option<String> {
        tags.iter()
            .find(|(t, _)| t == name)
            .map(|(_, v)| v.clone())
    };

    // ── p= (required) ────────────────────────────────────────────────────────
    let policy = match get("p") {
        None => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "p= (policy) tag is required but missing".to_string(),
            ));
            None
        }
        Some(p) => match p.as_str() {
            "none" => {
                overall = overall.merge(Verdict::Warn);
                details.push(Detail::with_verdict(
                    Verdict::Warn,
                    "p=none — monitoring mode only; no enforcement action taken".to_string(),
                ));
                Some(p)
            }
            "quarantine" => {
                details.push(Detail::with_verdict(
                    Verdict::Pass,
                    "p=quarantine — failing mail is quarantined (e.g. sent to spam)".to_string(),
                ));
                Some(p)
            }
            "reject" => {
                details.push(Detail::with_verdict(
                    Verdict::Pass,
                    "p=reject — failing mail is rejected outright".to_string(),
                ));
                Some(p)
            }
            other => {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(
                    Verdict::Fail,
                    format!("p={} — invalid policy value", other),
                ));
                None
            }
        },
    };

    // ── sp= (subdomain policy) ───────────────────────────────────────────────
    match get("sp") {
        None => {
            let inherited = policy.as_deref().unwrap_or("none");
            details.push(Detail::new(format!(
                "sp= not set — subdomains inherit p={}", inherited
            )));
        }
        Some(sp) => {
            let sp_str = sp.as_str();
            let valid_sp = matches!(sp_str, "none" | "quarantine" | "reject");
            if !valid_sp {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(
                    Verdict::Fail,
                    format!("sp={} — invalid subdomain policy value", sp),
                ));
            } else {
                // Warn if sp is weaker than p
                let p_strength = policy.as_deref().map(policy_strength).unwrap_or(0);
                let sp_strength = policy_strength(sp_str);
                if sp_strength < p_strength {
                    overall = overall.merge(Verdict::Warn);
                    details.push(Detail::with_verdict(
                        Verdict::Warn,
                        format!(
                            "sp={} is weaker than p={} — subdomains have less protection",
                            sp,
                            policy.as_deref().unwrap_or("none")
                        ),
                    ));
                } else {
                    details.push(Detail::with_verdict(
                        Verdict::Pass,
                        format!("sp={}", sp),
                    ));
                }
            }
        }
    }

    // ── adkim= alignment ─────────────────────────────────────────────────────
    {
        let adkim = get("adkim").unwrap_or_else(|| "r".to_string());
        let (verdict, note) = match adkim.as_str() {
            "r" => (Verdict::Pass, "adkim=r (relaxed, default) — DKIM alignment"),
            "s" => (Verdict::Pass, "adkim=s (strict) — DKIM alignment"),
            _ => {
                overall = overall.merge(Verdict::Warn);
                (Verdict::Warn, "adkim has unrecognised value")
            }
        };
        let text = if get("adkim").is_none() {
            format!("{} [default]", note)
        } else {
            note.to_string()
        };
        details.push(Detail::with_verdict(verdict, text));
    }

    // ── aspf= alignment ──────────────────────────────────────────────────────
    {
        let aspf = get("aspf").unwrap_or_else(|| "r".to_string());
        let (verdict, note) = match aspf.as_str() {
            "r" => (Verdict::Pass, "aspf=r (relaxed, default) — SPF alignment"),
            "s" => (Verdict::Pass, "aspf=s (strict) — SPF alignment"),
            _ => {
                overall = overall.merge(Verdict::Warn);
                (Verdict::Warn, "aspf has unrecognised value")
            }
        };
        let text = if get("aspf").is_none() {
            format!("{} [default]", note)
        } else {
            note.to_string()
        };
        details.push(Detail::with_verdict(verdict, text));
    }

    // ── pct= ─────────────────────────────────────────────────────────────────
    {
        let pct_raw = get("pct").unwrap_or_else(|| "100".to_string());
        let is_default = get("pct").is_none();
        match pct_raw.parse::<u32>() {
            Err(_) => {
                overall = overall.merge(Verdict::Warn);
                details.push(Detail::with_verdict(
                    Verdict::Warn,
                    format!("pct={} — not a valid integer (0-100)", pct_raw),
                ));
            }
            Ok(pct) if pct > 100 => {
                overall = overall.merge(Verdict::Warn);
                details.push(Detail::with_verdict(
                    Verdict::Warn,
                    format!("pct={} — out of range (must be 0-100)", pct),
                ));
            }
            Ok(pct) if pct < 100 => {
                overall = overall.merge(Verdict::Warn);
                details.push(Detail::with_verdict(
                    Verdict::Warn,
                    format!(
                        "pct={} — policy applied to only {}% of messages{}",
                        pct,
                        pct,
                        if is_default { " [default]" } else { "" }
                    ),
                ));
            }
            Ok(pct) => {
                details.push(Detail::with_verdict(
                    Verdict::Pass,
                    format!(
                        "pct={} — policy applies to 100% of messages{}",
                        pct,
                        if is_default { " [default]" } else { "" }
                    ),
                ));
            }
        }
    }

    // ── rua= (aggregate reporting) ───────────────────────────────────────────
    match get("rua") {
        None => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                "rua= not set — no aggregate report destination configured".to_string(),
            ));
        }
        Some(rua) => {
            details.push(Detail::new(format!("rua={}", rua)));
            for uri in rua.split(',') {
                let uri = uri.trim();
                if !uri.starts_with("mailto:") {
                    overall = overall.merge(Verdict::Warn);
                    details.push(Detail::with_verdict(
                        Verdict::Warn,
                        format!("rua URI not a mailto: — {}", uri),
                    ));
                    continue;
                }
                let addr = &uri["mailto:".len()..];
                if !addr.contains('@') {
                    overall = overall.merge(Verdict::Fail);
                    details.push(Detail::with_verdict(
                        Verdict::Fail,
                        format!("rua mailto address missing '@': {}", uri),
                    ));
                    continue;
                }
                details.push(Detail::with_verdict(
                    Verdict::Pass,
                    format!("rua: {}", uri),
                ));

                // External authorization check
                if let Some(reporting_domain) = mailto_domain(uri) {
                    // Compare apex domains (simple: trim trailing dot, compare)
                    let policy_domain = host.trim_end_matches('.');
                    let rep_domain = reporting_domain.trim_end_matches('.');

                    if !rep_domain.eq_ignore_ascii_case(policy_domain) {
                        let auth_query = format!(
                            "{}._report._dmarc.{}",
                            policy_domain, rep_domain
                        );
                        match lookup_txt(resolver, &auth_query).await {
                            Err(e) => {
                                overall = overall.merge(Verdict::Warn);
                                details.push(Detail::with_verdict(
                                    Verdict::Warn,
                                    format!(
                                        "External rua domain {} — auth check failed: {}",
                                        rep_domain, e
                                    ),
                                ));
                            }
                            Ok(auth_records) => {
                                let authorized = auth_records
                                    .iter()
                                    .any(|r| r.starts_with("v=DMARC1"));
                                if authorized {
                                    details.push(Detail::with_verdict(
                                        Verdict::Pass,
                                        format!(
                                            "External rua domain {} — authorized via {}",
                                            rep_domain, auth_query
                                        ),
                                    ));
                                } else {
                                    overall = overall.merge(Verdict::Warn);
                                    details.push(Detail::with_verdict(
                                        Verdict::Warn,
                                        format!(
                                            "External rua domain {} — no authorization record at {}",
                                            rep_domain, auth_query
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── ruf= (forensic reporting) ─────────────────────────────────────────────
    match get("ruf") {
        None => {
            details.push(Detail::new("ruf= not set — no forensic report destination"));
        }
        Some(ruf) => {
            details.push(Detail::with_verdict(
                Verdict::Pass,
                format!("ruf={} — forensic reports configured", ruf),
            ));
        }
    }

    // ── Summary ───────────────────────────────────────────────────────────────

    let summary = match policy.as_deref() {
        Some(p) => format!(
            "DMARC record found — p={} ({})",
            p,
            match p {
                "none"       => "monitoring only",
                "quarantine" => "quarantine enforcement",
                "reject"     => "reject enforcement",
                _            => "unknown",
            }
        ),
        None => "DMARC record found but p= tag is missing or invalid".to_string(),
    };

    Ok(CheckResult {
        name: "DMARC".to_string(),
        verdict: overall,
        summary,
        details,
    })
}
