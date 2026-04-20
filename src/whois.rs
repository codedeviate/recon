use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::util::parse_target;

pub struct WhoisProbeOk {
    pub host: String,
    pub server: String,
    pub body: String,
}

pub fn probe(input: &str) -> Result<WhoisProbeOk> {
    let (host, _) = parse_target(input);

    let iana = query("whois.iana.org", &host).context("Failed to reach whois.iana.org")?;

    let (server, body) = match parse_refer(&iana) {
        None => ("whois.iana.org".to_string(), iana),
        Some(server1) => {
            let resp1 = query(&server1, &host)
                .with_context(|| format!("Failed to query {server1}"))?;
            let registrar = parse_registrar_whois(&resp1).filter(|s| s != &server1);
            match registrar {
                Some(reg) => match query(&reg, &host) {
                    Ok(resp2) => (reg, resp2),
                    Err(_) => (server1, resp1),
                },
                None => (server1, resp1),
            }
        }
    };

    Ok(WhoisProbeOk { host, server, body })
}

pub fn run(input: &str) -> Result<()> {
    let r = probe(input)?;
    println!("WHOIS for {}", r.host.bold());
    println!("{}", "═".repeat(50));
    println!();
    println!("{}", r.body);
    Ok(())
}

fn query(server: &str, domain: &str) -> Result<String> {
    let addr = format!("{server}:43")
        .to_socket_addrs()
        .with_context(|| format!("Could not resolve WHOIS server: {server}"))?
        .next()
        .ok_or_else(|| anyhow!("No address found for {server}"))?;

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(10))
        .with_context(|| format!("Could not connect to {server}:43"))?;

    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    stream
        .write_all(format!("{domain}\r\n").as_bytes())
        .context("Failed to send WHOIS query")?;

    // Read until EOF or timeout, collecting partial results
    let mut response = String::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock => {
                break
            }
            Err(e) => return Err(anyhow!("Read error from {server}: {e}")),
        }
    }

    Ok(response)
}

fn parse_refer(response: &str) -> Option<String> {
    for line in response.lines() {
        let line = line.trim();
        let lower = line.to_lowercase();
        // "refer: server" or "whois: server"
        for prefix in &["refer:", "whois:"] {
            if let Some(val) = lower.strip_prefix(prefix) {
                let server = val.trim().to_string();
                if !server.is_empty() {
                    return Some(server);
                }
            }
        }
    }
    None
}

fn parse_registrar_whois(response: &str) -> Option<String> {
    for line in response.lines() {
        let lower = line.trim().to_lowercase();
        if let Some(val) = lower.strip_prefix("registrar whois server:") {
            let server = val
                .trim()
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/')
                .to_string();
            if !server.is_empty() {
                return Some(server);
            }
        }
    }
    None
}
