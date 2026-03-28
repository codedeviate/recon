use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::error::ResolveErrorKind;
use hickory_resolver::proto::rr::{RData, RecordType};
use std::str::FromStr;

use crate::util::parse_target;

const DEFAULT_TYPES: &[&str] = &["A", "AAAA", "CNAME", "MX", "NS", "TXT", "SOA"];

pub fn run(input: &str, requested_types: &[String]) -> Result<()> {
    let (host, _) = parse_target(input);

    let types: Vec<String> = if requested_types.is_empty() {
        DEFAULT_TYPES.iter().map(|s| s.to_string()).collect()
    } else {
        requested_types.iter().map(|s| s.to_uppercase()).collect()
    };

    let explicit = !requested_types.is_empty();

    println!("DNS lookup for {}", host.bold());
    println!("{}", "═".repeat(50));
    println!();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create async runtime")?;

    let found_any = rt.block_on(async {
        let resolver = TokioAsyncResolver::tokio_from_system_conf()
            .unwrap_or_else(|_| {
                TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
            });

        let mut found_any = false;

        for type_str in &types {
            let record_type = RecordType::from_str(type_str)
                .map_err(|_| anyhow!("Unknown record type: {type_str}"))?;

            match resolver.lookup(host.as_str(), record_type).await {
                Ok(lookup) => {
                    let records: Vec<String> = lookup.iter().map(format_rdata).collect();
                    if !records.is_empty() {
                        println!("{}", type_str.green().bold());
                        for r in &records {
                            println!("  {r}");
                        }
                        println!();
                        found_any = true;
                    } else if explicit {
                        println!("{}", type_str.green().bold());
                        println!("  (no records)");
                        println!();
                    }
                }
                Err(e) => {
                    let is_no_records = matches!(
                        e.kind(),
                        ResolveErrorKind::NoRecordsFound { .. }
                    );
                    if explicit {
                        println!("{}", type_str.green().bold());
                        if is_no_records {
                            println!("  (no records)");
                        } else {
                            println!("  error: {e}");
                        }
                        println!();
                    }
                    // Silently skip for default types
                }
            }
        }

        Ok::<bool, anyhow::Error>(found_any)
    })?;

    if !found_any && !explicit {
        println!("No DNS records found for {host}");
    }

    Ok(())
}

fn format_rdata(rdata: &RData) -> String {
    match rdata {
        RData::A(a) => format!("{a}"),
        RData::AAAA(aaaa) => format!("{aaaa}"),
        RData::CNAME(c) => format!("{c}"),
        RData::NS(ns) => format!("{ns}"),
        RData::PTR(ptr) => format!("{ptr}"),
        RData::MX(mx) => format!("priority={} {}", mx.preference(), mx.exchange()),
        RData::TXT(txt) => txt
            .txt_data()
            .iter()
            .map(|chunk| String::from_utf8_lossy(chunk.as_ref()).into_owned())
            .collect::<Vec<_>>()
            .join(" "),
        RData::SOA(soa) => format!(
            "{} {} (serial:{} refresh:{} retry:{} expire:{} min:{})",
            soa.mname(),
            soa.rname(),
            soa.serial(),
            soa.refresh(),
            soa.retry(),
            soa.expire(),
            soa.minimum()
        ),
        RData::SRV(srv) => format!(
            "priority={} weight={} port={} target={}",
            srv.priority(),
            srv.weight(),
            srv.port(),
            srv.target()
        ),
        RData::CAA(caa) => format!(
            "flags={} tag={} value={:?}",
            caa.issuer_critical() as u8,
            caa.tag().as_str(),
            caa.value()
        ),
        other => format!("{other:?}"),
    }
}
