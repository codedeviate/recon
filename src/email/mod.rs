pub mod bimi;
pub mod dkim;
pub mod dmarc;
pub mod mta_sts;
pub mod spf;
pub mod tls_rpt;

use anyhow::Result;
use colored::Colorize;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::proto::rr::RecordType;

// ── Verdict ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

impl Verdict {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Verdict::Pass => "✓ PASS".green().bold(),
            Verdict::Warn => "⚠ WARN".yellow().bold(),
            Verdict::Fail => "✗ FAIL".red().bold(),
        }
    }

    pub fn is_worse_than(&self, other: &Verdict) -> bool {
        matches!(
            (self, other),
            (Verdict::Fail, Verdict::Warn)
                | (Verdict::Fail, Verdict::Pass)
                | (Verdict::Warn, Verdict::Pass)
        )
    }

    pub fn merge(self, other: Verdict) -> Verdict {
        if self.is_worse_than(&other) { self } else { other }
    }
}

// ── Detail ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Detail {
    pub verdict: Option<Verdict>,
    pub text: String,
}

impl Detail {
    pub fn new(text: impl Into<String>) -> Self {
        Detail { verdict: None, text: text.into() }
    }

    pub fn with_verdict(verdict: Verdict, text: impl Into<String>) -> Self {
        Detail { verdict: Some(verdict), text: text.into() }
    }
}

// ── CheckResult ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub verdict: Verdict,
    pub summary: String,
    pub details: Vec<Detail>,
}

// ── EmailChecks ───────────────────────────────────────────────────────────────

pub struct EmailChecks {
    pub spf: bool,
    pub dmarc: bool,
    pub dkim_selectors: Vec<String>,
    pub mta_sts: bool,
    pub bimi: Option<String>,
    pub tls_rpt: bool,
    pub insecure: bool,
}

// ── Orchestrator ─────────────────────────────────────────────────────────────

pub fn run(host: &str, checks: EmailChecks) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

        let mut results: Vec<CheckResult> = Vec::new();

        if checks.spf {
            match spf::check(&resolver, host).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(CheckResult {
                    name: "SPF".to_string(),
                    verdict: Verdict::Fail,
                    summary: format!("check error: {e}"),
                    details: vec![],
                }),
            }
        }

        if checks.dmarc {
            match dmarc::check(&resolver, host).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(CheckResult {
                    name: "DMARC".to_string(),
                    verdict: Verdict::Fail,
                    summary: format!("check error: {e}"),
                    details: vec![],
                }),
            }
        }

        for selector in &checks.dkim_selectors {
            match dkim::check(&resolver, host, selector).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(CheckResult {
                    name: format!("DKIM({selector})"),
                    verdict: Verdict::Fail,
                    summary: format!("check error: {e}"),
                    details: vec![],
                }),
            }
        }

        if checks.mta_sts {
            match mta_sts::check(&resolver, host, checks.insecure).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(CheckResult {
                    name: "MTA-STS".to_string(),
                    verdict: Verdict::Fail,
                    summary: format!("check error: {e}"),
                    details: vec![],
                }),
            }
        }

        if let Some(ref selector) = checks.bimi {
            match bimi::check(&resolver, host, selector, checks.insecure).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(CheckResult {
                    name: "BIMI".to_string(),
                    verdict: Verdict::Fail,
                    summary: format!("check error: {e}"),
                    details: vec![],
                }),
            }
        }

        if checks.tls_rpt {
            match tls_rpt::check(&resolver, host).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(CheckResult {
                    name: "TLS-RPT".to_string(),
                    verdict: Verdict::Fail,
                    summary: format!("check error: {e}"),
                    details: vec![],
                }),
            }
        }

        let cross = cross_validate(&results);
        for r in &results {
            print_result(r);
        }
        for r in &cross {
            print_result(r);
        }

        Ok(())
    })
}

// ── Cross-validation ──────────────────────────────────────────────────────────

fn cross_validate(results: &[CheckResult]) -> Vec<CheckResult> {
    let mut notes: Vec<CheckResult> = Vec::new();

    let dmarc = results.iter().find(|r| r.name == "DMARC");
    let spf = results.iter().find(|r| r.name == "SPF");
    let bimi = results.iter().find(|r| r.name == "BIMI");
    let mta_sts = results.iter().find(|r| r.name == "MTA-STS");
    let tls_rpt = results.iter().find(|r| r.name == "TLS-RPT");
    let has_dkim = results.iter().any(|r| r.name.starts_with("DKIM("));

    // BIMI requires DMARC with enforcement policy (p=quarantine or p=reject)
    if let (Some(bimi_r), Some(dmarc_r)) = (bimi, dmarc) {
        if bimi_r.verdict != Verdict::Fail {
            // Extract the DMARC policy value from its detail lines (e.g. "p=none — …")
            let dmarc_policy = dmarc_r.details.iter().find_map(|d| {
                let text = &d.text;
                if let Some(rest) = text.strip_prefix("p=") {
                    // Extract the policy token: "p=none — …" → "none"
                    let end = rest.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(rest.len());
                    Some(rest[..end].to_string())
                } else {
                    None
                }
            });

            match dmarc_policy.as_deref() {
                Some("none") => {
                    notes.push(CheckResult {
                        name: "Cross: BIMI+DMARC".to_string(),
                        verdict: Verdict::Warn,
                        summary: "BIMI is present but DMARC policy is p=none — BIMI requires p=quarantine or p=reject".to_string(),
                        details: vec![],
                    });
                }
                None if dmarc_r.verdict == Verdict::Fail => {
                    notes.push(CheckResult {
                        name: "Cross: BIMI+DMARC".to_string(),
                        verdict: Verdict::Warn,
                        summary: "BIMI is present but DMARC failed — BIMI requires a valid DMARC record with p=quarantine or p=reject".to_string(),
                        details: vec![],
                    });
                }
                _ => {} // p=quarantine or p=reject — good
            }
        }
    }

    // MTA-STS and TLS-RPT are most useful together
    if let (Some(_), None) = (mta_sts, tls_rpt) {
        notes.push(CheckResult {
            name: "Cross: MTA-STS+TLS-RPT".to_string(),
            verdict: Verdict::Warn,
            summary: "MTA-STS is present but TLS-RPT was not checked — consider adding --tls-rpt to enable reporting".to_string(),
            details: vec![],
        });
    }
    if let (None, Some(_)) = (mta_sts, tls_rpt) {
        notes.push(CheckResult {
            name: "Cross: TLS-RPT+MTA-STS".to_string(),
            verdict: Verdict::Warn,
            summary: "TLS-RPT is present but MTA-STS was not checked — consider adding --mta-sts".to_string(),
            details: vec![],
        });
    }

    // BIMI without DMARC checked
    if bimi.is_some() && dmarc.is_none() {
        notes.push(CheckResult {
            name: "Cross: BIMI suggestion".to_string(),
            verdict: Verdict::Warn,
            summary: "BIMI checked but DMARC was not — add --dmarc to verify the enforcement policy required by BIMI".to_string(),
            details: vec![],
        });
    }

    // DMARC without SPF
    if dmarc.is_some() && spf.is_none() {
        notes.push(CheckResult {
            name: "Cross: DMARC+SPF alignment".to_string(),
            verdict: Verdict::Warn,
            summary: "DMARC checked but SPF was not — add --spf to verify SPF alignment".to_string(),
            details: vec![],
        });
    }

    // DMARC without DKIM
    if dmarc.is_some() && !has_dkim {
        notes.push(CheckResult {
            name: "Cross: DMARC+DKIM alignment".to_string(),
            verdict: Verdict::Warn,
            summary: "DMARC checked but no DKIM selectors were checked — add --dkim <selector> to verify DKIM alignment".to_string(),
            details: vec![],
        });
    }

    notes
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn print_result(r: &CheckResult) {
    println!("[{}] {}: {}", r.verdict.badge(), r.name.bold(), r.summary);
    for detail in &r.details {
        match &detail.verdict {
            Some(v) => println!("       [{}] {}", v.badge(), detail.text),
            None => println!("       {}", detail.text),
        }
    }
}

// ── DNS helpers ───────────────────────────────────────────────────────────────

pub async fn lookup_txt(resolver: &TokioAsyncResolver, name: &str) -> Result<Vec<String>> {
    use hickory_resolver::error::ResolveErrorKind;

    match resolver.lookup(name, RecordType::TXT).await {
        Ok(lookup) => {
            let records: Vec<String> = lookup
                .iter()
                .filter_map(|r| r.as_txt())
                .map(|txt| {
                    txt.iter()
                        .map(|b| String::from_utf8_lossy(b).into_owned())
                        .collect::<String>()
                })
                .collect();
            Ok(records)
        }
        Err(e) => {
            if matches!(e.kind(), ResolveErrorKind::NoRecordsFound { .. }) {
                Ok(vec![])
            } else {
                Err(anyhow::anyhow!("TXT lookup failed for {}: {}", name, e))
            }
        }
    }
}

pub async fn lookup_mx(resolver: &TokioAsyncResolver, domain: &str) -> Result<Vec<(u16, String)>> {
    use hickory_resolver::error::ResolveErrorKind;

    match resolver.mx_lookup(domain).await {
        Ok(lookup) => {
            let mut records: Vec<(u16, String)> = lookup
                .iter()
                .map(|mx| {
                    let pref = mx.preference();
                    let exchange = mx.exchange().to_string();
                    (pref, exchange)
                })
                .collect();
            records.sort_by_key(|(pref, _)| *pref);
            Ok(records)
        }
        Err(e) => {
            if matches!(e.kind(), ResolveErrorKind::NoRecordsFound { .. }) {
                Ok(vec![])
            } else {
                Err(anyhow::anyhow!("MX lookup failed for {}: {}", domain, e))
            }
        }
    }
}
