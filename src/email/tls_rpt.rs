use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::{CheckResult, Detail, Verdict, lookup_txt};

pub async fn check(resolver: &TokioAsyncResolver, host: &str) -> Result<CheckResult> {
    let query = format!("_smtp._tls.{}", host);
    let txt_records = lookup_txt(resolver, &query).await?;

    // Filter to TLS-RPT records
    let tls_rpt_records: Vec<&String> = txt_records
        .iter()
        .filter(|r| r.starts_with("v=TLSRPTv1"))
        .collect();

    // ── No record ────────────────────────────────────────────────────────────
    if tls_rpt_records.is_empty() {
        return Ok(CheckResult {
            name: "TLS-RPT".to_string(),
            verdict: Verdict::Fail,
            summary: format!("No TLS-RPT record found at {}", query),
            details: vec![
                Detail::with_verdict(
                    Verdict::Fail,
                    format!("DNS TXT lookup: {} — no v=TLSRPTv1 record", query),
                ),
            ],
        });
    }

    // ── Multiple records ─────────────────────────────────────────────────────
    if tls_rpt_records.len() > 1 {
        return Ok(CheckResult {
            name: "TLS-RPT".to_string(),
            verdict: Verdict::Fail,
            summary: format!("Multiple TLS-RPT records found at {} (ambiguous)", query),
            details: vec![
                Detail::with_verdict(
                    Verdict::Fail,
                    format!(
                        "Found {} v=TLSRPTv1 records — RFC 8460 requires exactly one",
                        tls_rpt_records.len()
                    ),
                ),
            ],
        });
    }

    let record = tls_rpt_records[0].clone();
    let mut details: Vec<Detail> = Vec::new();
    let mut overall = Verdict::Pass;

    details.push(Detail::new(format!("Record: {}", record)));

    // ── Tag parsing ──────────────────────────────────────────────────────────
    let mut tags: Vec<(String, String)> = Vec::new();
    for part in record.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(eq) = part.find('=') {
            let tag = part[..eq].trim().to_string();
            let val = part[eq + 1..].trim().to_string();
            tags.push((tag, val));
        }
    }

    // v=TLSRPTv1 must be first tag
    match tags.first() {
        Some((t, v)) if t == "v" && v == "TLSRPTv1" => {}
        _ => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "v=TLSRPTv1 must be the first tag".to_string(),
            ));
        }
    }

    // Helper: find tag value
    let get = |name: &str| -> Option<String> {
        tags.iter()
            .find(|(t, _)| t == name)
            .map(|(_, v)| v.clone())
    };

    // ── rua= (required) ──────────────────────────────────────────────────────
    match get("rua") {
        None => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "rua= tag is required but missing".to_string(),
            ));
        }
        Some(rua) if rua.trim().is_empty() => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "rua= tag is present but empty".to_string(),
            ));
        }
        Some(rua) => {
            details.push(Detail::new(format!("rua={}", rua)));
            for uri in rua.split(',') {
                let uri = uri.trim();
                if uri.starts_with("mailto:") {
                    let addr = &uri["mailto:".len()..];
                    if addr.contains('@') {
                        details.push(Detail::with_verdict(
                            Verdict::Pass,
                            format!("rua: {} (mailto)", uri),
                        ));
                    } else {
                        overall = overall.merge(Verdict::Fail);
                        details.push(Detail::with_verdict(
                            Verdict::Fail,
                            format!("rua mailto address missing '@': {}", uri),
                        ));
                    }
                } else if uri.starts_with("https:") {
                    details.push(Detail::with_verdict(
                        Verdict::Pass,
                        format!("rua: {} (https)", uri),
                    ));
                } else if uri.starts_with("http:") {
                    overall = overall.merge(Verdict::Warn);
                    details.push(Detail::with_verdict(
                        Verdict::Warn,
                        format!("rua: {} — should be https", uri),
                    ));
                } else {
                    overall = overall.merge(Verdict::Warn);
                    details.push(Detail::with_verdict(
                        Verdict::Warn,
                        format!("rua: {} — unrecognised URI scheme", uri),
                    ));
                }
            }
        }
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    let summary = match overall {
        Verdict::Pass => "TLS-RPT record found and valid".to_string(),
        Verdict::Warn => "TLS-RPT record found with warnings".to_string(),
        Verdict::Fail => "TLS-RPT record found but invalid".to_string(),
    };

    Ok(CheckResult {
        name: "TLS-RPT".to_string(),
        verdict: overall,
        summary,
        details,
    })
}
