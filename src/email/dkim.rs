use anyhow::Result;
use base64::Engine;
use hickory_resolver::TokioAsyncResolver;
use super::{CheckResult, Detail, Verdict, lookup_txt};

// ── Tag parsing ───────────────────────────────────────────────────────────────

struct DkimTags {
    v: Option<String>,
    h: Option<String>,
    k: Option<String>,
    n: Option<String>,
    p: Option<String>,
    s: Option<String>,
    t: Option<String>,
}

fn parse_tags(record: &str) -> DkimTags {
    let mut tags = DkimTags {
        v: None,
        h: None,
        k: None,
        n: None,
        p: None,
        s: None,
        t: None,
    };

    for part in record.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((name, value)) = part.split_once('=') {
            let name = name.trim();
            let value = value.trim().to_string();
            match name {
                "v" => tags.v = Some(value),
                "h" => tags.h = Some(value),
                "k" => tags.k = Some(value),
                "n" => tags.n = Some(value),
                "p" => tags.p = Some(value),
                "s" => tags.s = Some(value),
                "t" => tags.t = Some(value),
                _ => {} // unknown tags are allowed
            }
        }
    }

    tags
}

/// Check whether the first tag in the semicolon-separated record is `v=DKIM1`.
fn v_is_first(record: &str) -> bool {
    let first = record.split(';').next().unwrap_or("").trim();
    // Accept "v=DKIM1" with optional surrounding whitespace
    first.split_once('=').map(|(k, v)| k.trim() == "v" && v.trim() == "DKIM1").unwrap_or(false)
}

// ── RSA key size extraction ───────────────────────────────────────────────────

/// Parse a DER-encoded length field. Returns (length, bytes_consumed).
fn der_read_length(data: &[u8]) -> Option<(usize, usize)> {
    let first = *data.first()?;
    if first < 0x80 {
        Some((first as usize, 1))
    } else {
        let num_bytes = (first & 0x7f) as usize;
        if num_bytes == 0 || num_bytes > 4 || data.len() < 1 + num_bytes {
            return None;
        }
        let mut len: usize = 0;
        for &b in &data[1..1 + num_bytes] {
            len = (len << 8) | (b as usize);
        }
        Some((len, 1 + num_bytes))
    }
}

/// Read a DER TLV element. Returns (tag, value_slice, total_bytes_consumed).
fn der_read_tlv(data: &[u8]) -> Option<(u8, &[u8], usize)> {
    if data.is_empty() {
        return None;
    }
    let tag = data[0];
    let (len, len_bytes) = der_read_length(&data[1..])?;
    let start = 1 + len_bytes;
    let end = start + len;
    if data.len() < end {
        return None;
    }
    Some((tag, &data[start..end], end))
}

/// Extract RSA modulus bit length from a base64-encoded SPKI DER blob.
fn rsa_key_size_bits(b64: &str) -> Option<usize> {
    // Strip all whitespace from the base64 value
    let clean: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let der = base64::engine::general_purpose::STANDARD.decode(&clean).ok()?;

    // Outer SEQUENCE (SubjectPublicKeyInfo)
    let (outer_tag, outer_val, _) = der_read_tlv(&der)?;
    if outer_tag != 0x30 {
        return None;
    }

    // First child: algorithm SEQUENCE — skip it entirely
    let (algo_tag, _, _) = der_read_tlv(outer_val)?;
    if algo_tag != 0x30 {
        return None;
    }
    let algo_total = {
        let (_, _, n) = der_read_tlv(outer_val)?;
        n
    };
    let after_algo = &outer_val[algo_total..];

    // BIT STRING
    let (bs_tag, bs_val, _) = der_read_tlv(after_algo)?;
    if bs_tag != 0x03 {
        return None;
    }
    // First byte of BIT STRING is the unused-bits count
    if bs_val.is_empty() {
        return None;
    }
    let inner_der = &bs_val[1..]; // skip unused-bits byte

    // Inner SEQUENCE (RSAPublicKey)
    let (inner_tag, inner_val, _) = der_read_tlv(inner_der)?;
    if inner_tag != 0x30 {
        return None;
    }

    // First INTEGER = modulus
    let (int_tag, int_val, _) = der_read_tlv(inner_val)?;
    if int_tag != 0x02 {
        return None;
    }

    // Strip leading zero (sign padding)
    let modulus = if int_val.first() == Some(&0x00) {
        &int_val[1..]
    } else {
        int_val
    };

    Some(modulus.len() * 8)
}

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn check(resolver: &TokioAsyncResolver, host: &str, selector: &str) -> Result<CheckResult> {
    let name = format!("DKIM ({})", selector);
    let lookup_name = format!("{}._domainkey.{}", selector, host);

    let records = lookup_txt(resolver, &lookup_name).await?;

    if records.is_empty() {
        return Ok(CheckResult {
            name,
            verdict: Verdict::Fail,
            summary: format!("No DKIM record found at {}", lookup_name),
            details: vec![],
        });
    }

    // Use the first record (already concatenated by lookup_txt)
    let record = &records[0];

    let mut details: Vec<Detail> = Vec::new();
    details.push(Detail::new(format!("Record: {}", record)));

    let tags = parse_tags(record);

    // Overall verdict accumulator
    let mut verdict = Verdict::Pass;

    // ── v= (version) ─────────────────────────────────────────────────────────
    if let Some(ref v) = tags.v {
        if v != "DKIM1" {
            verdict = verdict.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                format!("v={}: must be DKIM1", v),
            ));
        } else if !v_is_first(record) {
            verdict = verdict.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "v=DKIM1 is present but not the first tag (required by RFC 6376)".to_string(),
            ));
        } else {
            details.push(Detail::with_verdict(Verdict::Pass, "v=DKIM1".to_string()));
        }
    }

    // ── p= (public key) ───────────────────────────────────────────────────────
    let p_value = match &tags.p {
        None => {
            verdict = verdict.merge(Verdict::Fail);
            details.push(Detail::with_verdict(
                Verdict::Fail,
                "p= (public key) is missing — required tag".to_string(),
            ));
            None
        }
        Some(p) if p.is_empty() => {
            verdict = verdict.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                "p= is empty — key has been revoked".to_string(),
            ));
            None
        }
        Some(p) => Some(p.clone()),
    };

    // ── k= (key type) ─────────────────────────────────────────────────────────
    let key_type = tags.k.as_deref().unwrap_or("rsa");
    match key_type {
        "rsa" => {
            if let Some(ref p) = p_value {
                match rsa_key_size_bits(p) {
                    None => {
                        verdict = verdict.merge(Verdict::Warn);
                        details.push(Detail::with_verdict(
                            Verdict::Warn,
                            "k=rsa: could not parse public key to determine key size".to_string(),
                        ));
                    }
                    Some(bits) => {
                        if bits < 1024 {
                            verdict = verdict.merge(Verdict::Warn);
                            details.push(Detail::with_verdict(
                                Verdict::Warn,
                                format!("k=rsa: {}-bit key — weak key, upgrade immediately", bits),
                            ));
                        } else if bits == 1024 {
                            verdict = verdict.merge(Verdict::Warn);
                            details.push(Detail::with_verdict(
                                Verdict::Warn,
                                format!("k=rsa: {}-bit key — minimum, consider upgrading to 2048", bits),
                            ));
                        } else {
                            details.push(Detail::with_verdict(
                                Verdict::Pass,
                                format!("k=rsa: {}-bit key", bits),
                            ));
                        }
                    }
                }
            } else {
                details.push(Detail::new("k=rsa (default)"));
            }
        }
        "ed25519" => {
            details.push(Detail::with_verdict(Verdict::Pass, "k=ed25519".to_string()));
        }
        other => {
            verdict = verdict.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                format!("k={}: unknown key type", other),
            ));
        }
    }

    // ── h= (hash algorithms) ──────────────────────────────────────────────────
    let hash_desc = match &tags.h {
        None => "all (default)".to_string(),
        Some(h) => {
            let algos: Vec<&str> = h.split(':').map(str::trim).collect();
            let sha1_only = algos.len() == 1 && algos[0].eq_ignore_ascii_case("sha1");
            if sha1_only {
                verdict = verdict.merge(Verdict::Warn);
                details.push(Detail::with_verdict(
                    Verdict::Warn,
                    "h=sha1: only sha1 accepted — sha256 is recommended".to_string(),
                ));
            }
            h.clone()
        }
    };
    if tags.h.is_none() {
        details.push(Detail::new(format!("h={}", hash_desc)));
    }

    // ── s= (service type) ─────────────────────────────────────────────────────
    let service = tags.s.as_deref().unwrap_or("*");
    match service {
        "*" | "email" => {
            details.push(Detail::new(format!("s={}", service)));
        }
        other => {
            verdict = verdict.merge(Verdict::Warn);
            details.push(Detail::with_verdict(
                Verdict::Warn,
                format!("s={}: unexpected service type (expected * or email)", other),
            ));
        }
    }

    // ── t= (flags) ────────────────────────────────────────────────────────────
    if let Some(ref t) = tags.t {
        for flag in t.split(':').map(str::trim) {
            match flag {
                "y" => {
                    verdict = verdict.merge(Verdict::Warn);
                    details.push(Detail::with_verdict(
                        Verdict::Warn,
                        "t=y: testing mode — DKIM failures should not affect delivery".to_string(),
                    ));
                }
                "s" => {
                    details.push(Detail::new(
                        "t=s: strict alignment — i= must match d= exactly".to_string(),
                    ));
                }
                other => {
                    details.push(Detail::new(format!("t={}: unknown flag", other)));
                }
            }
        }
    }

    // ── n= (notes) ────────────────────────────────────────────────────────────
    if let Some(ref n) = tags.n {
        details.push(Detail::new(format!("n={}", n)));
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    let summary = match verdict {
        Verdict::Pass => format!("Valid DKIM record for selector '{}'", selector),
        Verdict::Warn => format!("DKIM record for selector '{}' has warnings", selector),
        Verdict::Fail => format!("DKIM record for selector '{}' failed validation", selector),
    };

    Ok(CheckResult {
        name,
        verdict,
        summary,
        details,
    })
}
