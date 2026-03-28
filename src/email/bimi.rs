use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::{CheckResult, Detail, Verdict, lookup_txt};

pub async fn check(resolver: &TokioAsyncResolver, host: &str, selector: &str, insecure: bool) -> Result<CheckResult> {
    let name = format!("BIMI ({})", selector);
    let query = format!("{}._bimi.{}", selector, host);

    // ── DNS lookup ────────────────────────────────────────────────────────────

    let txt_records = lookup_txt(resolver, &query).await?;

    let bimi_records: Vec<&String> = txt_records
        .iter()
        .filter(|r| r.starts_with("v=BIMI1"))
        .collect();

    if bimi_records.is_empty() {
        return Ok(CheckResult {
            name,
            verdict: Verdict::Fail,
            summary: format!("No BIMI record found at {}", query),
            details: vec![
                Detail::with_verdict(Verdict::Fail, format!("DNS TXT lookup: {} — no v=BIMI1 record", query)),
            ],
        });
    }

    let record = bimi_records[0].clone();
    let mut details: Vec<Detail> = Vec::new();
    let mut overall = Verdict::Pass;

    details.push(Detail::new(format!("Record: {}", record)));
    details.push(Detail::new(format!("DNS query: {}", query)));

    // ── Tag parsing ───────────────────────────────────────────────────────────

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

    let get = |name: &str| -> Option<String> {
        tags.iter()
            .find(|(t, _)| t == name)
            .map(|(_, v)| v.clone())
    };

    // v=BIMI1 — must be first tag
    match tags.first() {
        Some((t, v)) if t == "v" && v == "BIMI1" => {
            details.push(Detail::with_verdict(Verdict::Pass, "v=BIMI1 — valid version tag".to_string()));
        }
        _ => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(Verdict::Fail, "v=BIMI1 must be the first tag".to_string()));
        }
    }

    // ── l= (logo URL) ─────────────────────────────────────────────────────────

    let logo_url = get("l");
    match &logo_url {
        None => {
            overall = overall.merge(Verdict::Fail);
            details.push(Detail::with_verdict(Verdict::Fail, "l= (logo URL) tag is required but missing".to_string()));
        }
        Some(l) if l.is_empty() => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(Verdict::Warn, "l= explicitly disabled (empty) — logo will not display".to_string()));
        }
        Some(l) => {
            details.push(Detail::new(format!("l={}", l)));

            if l.starts_with("http://") {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(Verdict::Fail, "l= must use HTTPS, not HTTP".to_string()));
            } else if !l.starts_with("https://") {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(Verdict::Fail, format!("l= URL is not HTTPS: {}", l)));
            } else {
                // HEAD request to check Content-Type
                match build_client(insecure) {
                    Err(e) => {
                        overall = overall.merge(Verdict::Warn);
                        details.push(Detail::with_verdict(Verdict::Warn, format!("Could not build HTTP client: {}", e)));
                    }
                    Ok(client) => {
                        match client.head(l.as_str()).send().await {
                            Err(e) => {
                                overall = overall.merge(Verdict::Warn);
                                details.push(Detail::with_verdict(Verdict::Warn, format!("Logo URL unreachable: {}", e)));
                            }
                            Ok(resp) => {
                                let ct = resp
                                    .headers()
                                    .get("content-type")
                                    .and_then(|v| v.to_str().ok())
                                    .unwrap_or("")
                                    .to_string();

                                if ct.starts_with("image/svg+xml") {
                                    details.push(Detail::with_verdict(Verdict::Pass, format!("Logo Content-Type: {} — OK", ct)));
                                } else {
                                    overall = overall.merge(Verdict::Warn);
                                    details.push(Detail::with_verdict(
                                        Verdict::Warn,
                                        format!("Logo Content-Type: {} — expected image/svg+xml", ct),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── a= (VMC authority) ────────────────────────────────────────────────────

    let vmc_url = get("a");
    match &vmc_url {
        None => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                "a= not set — no VMC; logo may not display in all clients".to_string(),
            ));
        }
        Some(a) if a.is_empty() => {
            overall = overall.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                "a= is empty — no VMC; logo may not display in all clients".to_string(),
            ));
        }
        Some(a) => {
            details.push(Detail::new(format!("a={}", a)));

            if a.starts_with("http://") {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(Verdict::Fail, "a= VMC URL must use HTTPS, not HTTP".to_string()));
            } else if !a.starts_with("https://") {
                overall = overall.merge(Verdict::Fail);
                details.push(Detail::with_verdict(Verdict::Fail, format!("a= VMC URL is not HTTPS: {}", a)));
            } else {
                // Fetch PEM and parse certificate
                match fetch_vmc(a, insecure).await {
                    Err(e) => {
                        overall = overall.merge(Verdict::Warn);
                        details.push(Detail::with_verdict(Verdict::Warn, format!("VMC fetch/parse failed: {}", e)));
                    }
                    Ok(vmc_info) => {
                        // Display certificate info
                        details.push(Detail::new(format!("VMC issuer: {}", vmc_info.issuer)));
                        details.push(Detail::new(format!("VMC expiry: {} ({} days remaining)", vmc_info.expiry_str, vmc_info.days_remaining)));

                        // Check expiry
                        if vmc_info.days_remaining < 0 {
                            overall = overall.merge(Verdict::Fail);
                            details.push(Detail::with_verdict(
                                Verdict::Fail,
                                format!("VMC certificate expired {} days ago", -vmc_info.days_remaining),
                            ));
                        } else if vmc_info.days_remaining < 30 {
                            overall = overall.merge(Verdict::Warn);
                            details.push(Detail::with_verdict(
                                Verdict::Warn,
                                format!("VMC certificate expires in {} days — renew soon", vmc_info.days_remaining),
                            ));
                        } else {
                            details.push(Detail::with_verdict(
                                Verdict::Pass,
                                format!("VMC certificate valid ({} days remaining)", vmc_info.days_remaining),
                            ));
                        }

                        // Check BIMI EKU OID
                        if vmc_info.has_bimi_eku {
                            details.push(Detail::with_verdict(
                                Verdict::Pass,
                                "VMC has BIMI EKU OID 1.3.6.1.5.5.7.3.31".to_string(),
                            ));
                        } else {
                            overall = overall.merge(Verdict::Warn);
                            details.push(Detail::with_verdict(
                                Verdict::Warn,
                                "VMC missing BIMI EKU OID 1.3.6.1.5.5.7.3.31 — certificate may not be a valid VMC".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    // ── Summary ───────────────────────────────────────────────────────────────

    let summary = match overall {
        Verdict::Pass => format!("BIMI record found and valid at {}", query),
        Verdict::Warn => format!("BIMI record found with warnings at {}", query),
        Verdict::Fail => format!("BIMI record found but has failures at {}", query),
    };

    Ok(CheckResult {
        name,
        verdict: overall,
        summary,
        details,
    })
}

// ── HTTP client ───────────────────────────────────────────────────────────────

fn build_client(insecure: bool) -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(insecure)
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    Ok(client)
}

// ── VMC info ──────────────────────────────────────────────────────────────────

struct VmcInfo {
    issuer: String,
    expiry_str: String,
    days_remaining: i64,
    has_bimi_eku: bool,
}

async fn fetch_vmc(url: &str, insecure: bool) -> Result<VmcInfo> {
    let client = build_client(insecure)?;
    let resp = client.get(url).send().await?;
    let pem_text = resp.text().await?;

    // Parse PEM
    let pem_data = ::pem::parse(&pem_text).map_err(|e| anyhow::anyhow!("PEM parse error: {e}"))?;
    let der_bytes = pem_data.contents();

    // Parse X.509
    use x509_parser::prelude::*;
    let (_, cert) = X509Certificate::from_der(der_bytes)?;

    // Issuer CN
    let issuer = cert
        .issuer()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("(unknown)")
        .to_string();

    // Validity
    let expiry_ts = cert.validity().not_after.timestamp();
    let expiry_str = cert.validity().not_after.to_string();
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days_remaining = (expiry_ts - now_ts) / 86400;

    // BIMI EKU OID
    let has_bimi_eku = cert
        .extended_key_usage()
        .ok()
        .flatten()
        .map(|eku| {
            eku.value.other.iter().any(|oid| oid.to_id_string() == "1.3.6.1.5.5.7.3.31")
        })
        .unwrap_or(false);

    Ok(VmcInfo {
        issuer,
        expiry_str,
        days_remaining,
        has_bimi_eku,
    })
}
