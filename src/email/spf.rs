use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use std::cell::Cell;
use super::{CheckResult, Detail, Verdict, lookup_txt};

// ── Counters shared across recursive evaluation ─────────────────────────────

struct SpfContext {
    dns_lookups: Cell<u32>,
    void_lookups: Cell<u32>,
}

impl SpfContext {
    fn new() -> Self {
        Self {
            dns_lookups: Cell::new(0),
            void_lookups: Cell::new(0),
        }
    }

    fn increment_dns(&self) -> Result<(), &'static str> {
        let n = self.dns_lookups.get() + 1;
        self.dns_lookups.set(n);
        if n > 10 {
            Err("DNS lookup limit exceeded (>10)")
        } else {
            Ok(())
        }
    }

    fn increment_void(&self) -> Result<(), &'static str> {
        let n = self.void_lookups.get() + 1;
        self.void_lookups.set(n);
        if n > 2 {
            Err("Void lookup limit exceeded (>2)")
        } else {
            Ok(())
        }
    }
}

// ── Qualifier ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Qualifier {
    Pass,     // +
    Fail,     // -
    SoftFail, // ~
    Neutral,  // ?
}

impl Qualifier {
    fn symbol(&self) -> &'static str {
        match self {
            Qualifier::Pass => "+",
            Qualifier::Fail => "-",
            Qualifier::SoftFail => "~",
            Qualifier::Neutral => "?",
        }
    }
}

// ── SPF term ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum SpfTerm {
    All(Qualifier),
    Include(Qualifier, String),
    A(Qualifier, Option<String>),
    Mx(Qualifier, Option<String>),
    Ptr(Qualifier, Option<String>),
    Ip4(Qualifier, String),
    Ip6(Qualifier, String),
    Exists(Qualifier, String),
    Redirect(String),
    Exp(String),
}

fn parse_qualifier(s: &str) -> (Qualifier, &str) {
    match s.as_bytes().first() {
        Some(b'+') => (Qualifier::Pass, &s[1..]),
        Some(b'-') => (Qualifier::Fail, &s[1..]),
        Some(b'~') => (Qualifier::SoftFail, &s[1..]),
        Some(b'?') => (Qualifier::Neutral, &s[1..]),
        _ => (Qualifier::Pass, s),
    }
}

fn parse_term(raw: &str) -> Result<SpfTerm, String> {
    // Modifiers
    if let Some(val) = raw.strip_prefix("redirect=") {
        return Ok(SpfTerm::Redirect(val.to_string()));
    }
    if let Some(val) = raw.strip_prefix("exp=") {
        return Ok(SpfTerm::Exp(val.to_string()));
    }

    let (qual, body) = parse_qualifier(raw);

    // Split mechanism:value or mechanism/cidr
    let lower = body.to_ascii_lowercase();
    if lower == "all" {
        return Ok(SpfTerm::All(qual));
    }
    if lower.starts_with("include:") {
        let domain = &body[8..];
        if domain.is_empty() {
            return Err("include: missing domain".into());
        }
        return Ok(SpfTerm::Include(qual, domain.to_string()));
    }
    if lower.starts_with("a:") || lower.starts_with("a/") || lower == "a" {
        let arg = if lower == "a" { None } else { Some(body[2..].to_string()) };
        return Ok(SpfTerm::A(qual, arg));
    }
    if lower.starts_with("mx:") || lower.starts_with("mx/") || lower == "mx" {
        let arg = if lower == "mx" { None } else { Some(body[if lower.starts_with("mx:") { 3 } else { 2 }..].to_string()) };
        return Ok(SpfTerm::Mx(qual, arg));
    }
    if lower.starts_with("ptr:") || lower == "ptr" {
        let arg = if lower == "ptr" { None } else { Some(body[4..].to_string()) };
        return Ok(SpfTerm::Ptr(qual, arg));
    }
    if lower.starts_with("ip4:") {
        return Ok(SpfTerm::Ip4(qual, body[4..].to_string()));
    }
    if lower.starts_with("ip6:") {
        return Ok(SpfTerm::Ip6(qual, body[4..].to_string()));
    }
    if lower.starts_with("exists:") {
        let domain = &body[7..];
        return Ok(SpfTerm::Exists(qual, domain.to_string()));
    }

    Err(format!("Unknown mechanism: {}", raw))
}

fn parse_spf_record(record: &str) -> Result<Vec<SpfTerm>, String> {
    // Strip "v=spf1" prefix
    let body = if record.len() == 6 {
        "" // just "v=spf1" with nothing after
    } else {
        &record[6..] // includes leading space
    };

    let mut terms = Vec::new();
    for token in body.split_whitespace() {
        terms.push(parse_term(token)?);
    }
    Ok(terms)
}

// ── Recursive evaluation ────────────────────────────────────────────────────

struct TreeLine {
    indent: usize,
    text: String,
    verdict: Option<Verdict>,
}

fn evaluate_spf<'a>(
    resolver: &'a TokioAsyncResolver,
    host: &'a str,
    ctx: &'a SpfContext,
    depth: usize,
    tree: &'a mut Vec<TreeLine>,
    warnings: &'a mut Vec<(Verdict, String)>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + 'a>> {
    Box::pin(async move {
    if depth > 10 {
        return Err("Recursion depth exceeded (>10)".into());
    }

    let prefix = "  ".repeat(depth);

    // DNS lookup for SPF record
    let txt_records = lookup_txt(resolver, host)
        .await
        .map_err(|e| format!("DNS error for {}: {}", host, e))?;

    if txt_records.is_empty() {
        ctx.increment_void().map_err(|e| e.to_string())?;
    }

    // Filter to SPF records
    let spf_records: Vec<&String> = txt_records
        .iter()
        .filter(|r| {
            r == &"v=spf1" || r.starts_with("v=spf1 ")
        })
        .collect();

    if spf_records.is_empty() {
        tree.push(TreeLine {
            indent: depth,
            text: format!("{} — no SPF record found", host),
            verdict: Some(Verdict::Fail),
        });
        return Ok(None);
    }

    if spf_records.len() > 1 {
        tree.push(TreeLine {
            indent: depth,
            text: format!("{} — multiple SPF records (PermError)", host),
            verdict: Some(Verdict::Fail),
        });
        return Err(format!("Multiple SPF records for {} (PermError per RFC 7208)", host));
    }

    let record = spf_records[0].clone();
    tree.push(TreeLine {
        indent: depth,
        text: format!("{} → {}", host, record),
        verdict: Some(Verdict::Pass),
    });

    let terms = parse_spf_record(&record)?;

    let mut has_all = false;
    let mut has_redirect = false;
    let mut default_mechanism = None;

    for term in &terms {
        match term {
            SpfTerm::All(qual) => {
                has_all = true;
                let label = format!("{}all", qual.symbol());
                default_mechanism = Some(label.clone());
                match qual {
                    Qualifier::Pass => {
                        warnings.push((Verdict::Warn, format!("{}+all — Dangerously permissive — allows any sender", prefix)));
                    }
                    Qualifier::SoftFail => {
                        warnings.push((Verdict::Pass, format!("{}~all — soft fail (common, recommended)", prefix)));
                    }
                    Qualifier::Fail => {
                        warnings.push((Verdict::Pass, format!("{}-all — hard fail (strict)", prefix)));
                    }
                    Qualifier::Neutral => {
                        warnings.push((Verdict::Warn, format!("{}?all — neutral (no opinion on unauthorized senders)", prefix)));
                    }
                }
            }
            SpfTerm::Include(_, domain) => {
                if ctx.increment_dns().is_err() {
                    tree.push(TreeLine {
                        indent: depth + 1,
                        text: format!("include:{} — DNS lookup limit exceeded (>10)", domain),
                        verdict: Some(Verdict::Fail),
                    });
                    return Err("DNS lookup limit exceeded (>10)".into());
                }
                // Recurse
                let _ = evaluate_spf(resolver, domain, ctx, depth + 1, tree, warnings).await;
            }
            SpfTerm::A(_, _) => {
                if ctx.increment_dns().is_err() {
                    return Err("DNS lookup limit exceeded (>10)".into());
                }
                tree.push(TreeLine {
                    indent: depth + 1,
                    text: "a (checks A/AAAA of domain)".to_string(),
                    verdict: None,
                });
            }
            SpfTerm::Mx(_, _) => {
                if ctx.increment_dns().is_err() {
                    return Err("DNS lookup limit exceeded (>10)".into());
                }
                tree.push(TreeLine {
                    indent: depth + 1,
                    text: "mx (checks MX hosts)".to_string(),
                    verdict: None,
                });
            }
            SpfTerm::Ptr(_, _) => {
                if ctx.increment_dns().is_err() {
                    return Err("DNS lookup limit exceeded (>10)".into());
                }
                warnings.push((Verdict::Warn, format!("{}ptr — RFC 7208 recommends against using ptr", prefix)));
                tree.push(TreeLine {
                    indent: depth + 1,
                    text: "ptr (deprecated per RFC 7208)".to_string(),
                    verdict: Some(Verdict::Warn),
                });
            }
            SpfTerm::Ip4(_, cidr) => {
                tree.push(TreeLine {
                    indent: depth + 1,
                    text: format!("ip4:{}", cidr),
                    verdict: None,
                });
            }
            SpfTerm::Ip6(_, cidr) => {
                tree.push(TreeLine {
                    indent: depth + 1,
                    text: format!("ip6:{}", cidr),
                    verdict: None,
                });
            }
            SpfTerm::Exists(_, domain) => {
                if ctx.increment_dns().is_err() {
                    return Err("DNS lookup limit exceeded (>10)".into());
                }
                tree.push(TreeLine {
                    indent: depth + 1,
                    text: format!("exists:{}", domain),
                    verdict: None,
                });
            }
            SpfTerm::Redirect(domain) => {
                has_redirect = true;
                default_mechanism = Some(format!("redirect={}", domain));
                if ctx.increment_dns().is_err() {
                    return Err("DNS lookup limit exceeded (>10)".into());
                }
                tree.push(TreeLine {
                    indent: depth + 1,
                    text: format!("redirect={}", domain),
                    verdict: None,
                });
                let _ = evaluate_spf(resolver, domain, ctx, depth + 1, tree, warnings).await;
            }
            SpfTerm::Exp(_) => {
                // exp= is informational, no DNS cost for SPF evaluation counting
            }
        }
    }

    if !has_all && !has_redirect && depth == 0 {
        warnings.push((Verdict::Warn, "No default result; implicit ?all".to_string()));
    }

    Ok(default_mechanism)
    }) // Box::pin
}

// ── Public entry point ──────────────────────────────────────────────────────

pub async fn check(resolver: &TokioAsyncResolver, host: &str) -> Result<CheckResult> {
    let ctx = SpfContext::new();
    let mut tree: Vec<TreeLine> = Vec::new();
    let mut warnings: Vec<(Verdict, String)> = Vec::new();

    let result = evaluate_spf(resolver, host, &ctx, 0, &mut tree, &mut warnings).await;

    let dns_count = ctx.dns_lookups.get();
    let void_count = ctx.void_lookups.get();

    let mut details: Vec<Detail> = Vec::new();

    // Lookup counters
    let lookup_verdict = if dns_count > 10 {
        Verdict::Fail
    } else if dns_count >= 8 {
        Verdict::Warn
    } else {
        Verdict::Pass
    };
    details.push(Detail::with_verdict(
        lookup_verdict.clone(),
        format!("DNS lookups: {}/10", dns_count),
    ));
    if dns_count >= 8 && dns_count <= 10 {
        details.push(Detail::with_verdict(
            Verdict::Warn,
            format!("Approaching DNS lookup limit ({}/10)", dns_count),
        ));
    }

    let void_verdict = if void_count > 2 {
        Verdict::Fail
    } else if void_count >= 2 {
        Verdict::Warn
    } else {
        Verdict::Pass
    };
    details.push(Detail::with_verdict(
        void_verdict.clone(),
        format!("Void lookups: {}/2", void_count),
    ));

    // Tree output
    details.push(Detail::new(""));
    details.push(Detail::new("SPF tree:"));
    for line in &tree {
        let indent = "  ".repeat(line.indent);
        match &line.verdict {
            Some(v) => details.push(Detail::with_verdict(v.clone(), format!("{}{}", indent, line.text))),
            None => details.push(Detail::new(format!("  {}{}", indent, line.text))),
        }
    }

    // Warnings
    if !warnings.is_empty() {
        details.push(Detail::new(""));
        for (v, text) in &warnings {
            details.push(Detail::with_verdict(v.clone(), text.clone()));
        }
    }

    // Overall verdict
    let (verdict, summary) = match &result {
        Err(e) => (Verdict::Fail, format!("PermError: {}", e)),
        Ok(default_mech) => {
            let mut v = lookup_verdict.merge(void_verdict);
            // Merge warning verdicts
            for (wv, _) in &warnings {
                v = v.merge(wv.clone());
            }
            let desc = default_mech
                .as_deref()
                .unwrap_or("(none)");
            (v, format!("Valid SPF record — default: {}", desc))
        }
    };

    Ok(CheckResult {
        name: "SPF".to_string(),
        verdict,
        summary,
        details,
    })
}
