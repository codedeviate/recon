use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use reqwest::Method;
use std::fs;
use std::io;
use std::net::ToSocketAddrs;
use std::str::FromStr;
use std::time::Duration;

use crate::cli::Args;
use crate::cookiejar::CookieJar;

pub fn execute(args: &Args) -> Result<Response> {
    let mut builder = Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(args.insecure)
        .timeout(Duration::from_secs(args.timeout));

    // --LHEAD follows redirects manually; otherwise use reqwest's built-in policy
    builder = builder.redirect(if args.follow_redirects && !args.lhead {
        Policy::limited(args.max_redirs)
    } else {
        Policy::none()
    });

    if let Some(ua) = &args.user_agent {
        builder = builder.user_agent(ua.as_str());
    } else {
        builder = builder.user_agent(concat!("recon/", env!("CARGO_PKG_VERSION")));
    }

    let client = builder.build().context("Failed to build HTTP client")?;
    let method = resolve_method(args)?;
    let start_url = effective_url(args);

    let jar = args
        .cookiejar
        .as_deref()
        .map(CookieJar::open)
        .transpose()?;

    if args.lhead {
        execute_lhead(args, &client, method, jar.as_ref(), &start_url)
    } else {
        let cookie = cookie_header(jar.as_ref(), &start_url)?;
        let response = send_request(args, &client, method, &start_url, cookie.as_deref())?;
        if let Some(j) = &jar {
            save_cookies(&response, j, &start_url)?;
        }
        Ok(response)
    }
}

/// Returns the effective request URL. When -G / --get is active, appends -d data as a query string.
fn effective_url(args: &Args) -> String {
    if args.get_data {
        if let Some(data) = &args.data {
            let base = args.target_url();
            return if base.contains('?') {
                format!("{base}&{data}")
            } else {
                format!("{base}?{data}")
            };
        }
    }
    args.target_url().to_string()
}

fn execute_lhead(args: &Args, client: &Client, method: Method, jar: Option<&CookieJar>, start_url: &str) -> Result<Response> {
    let mut current_url = start_url.to_string();
    let mut redirects = 0;

    loop {
        let cookie = cookie_header(jar, &current_url)?;
        let response = send_request(args, client, method.clone(), &current_url, cookie.as_deref())?;

        if let Some(j) = jar {
            save_cookies(&response, j, &current_url)?;
        }

        let status = response.status();
        let next_url = if status.is_redirection() && redirects < args.max_redirs {
            response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .map(|loc| resolve_redirect(&current_url, loc))
                .transpose()?
        } else {
            None
        };

        if let Some(next) = next_url {
            print_hop_headers(&response, &current_url, &next);
            current_url = next;
            redirects += 1;
        } else {
            return Ok(response);
        }
    }
}

fn send_request(
    args: &Args,
    client: &Client,
    method: Method,
    url: &str,
    cookie: Option<&str>,
) -> Result<Response> {
    let mut request = client.request(method, url);

    // Check if user explicitly provided a Referer header
    let user_provided_referer = args.header.iter().any(|h| {
        h.split_once(':')
            .map(|(name, _)| name.trim().eq_ignore_ascii_case("Referer"))
            .unwrap_or(false)
    });

    if let Some(ref_url) = &args.referer {
        if !user_provided_referer {
            request = request.header("Referer", ref_url.as_str());
        }
    }

    for header_str in &args.header {
        let (name, value) = parse_header(header_str)?;
        request = request.header(name, value);
    }

    if let Some(c) = cookie {
        request = request.header(reqwest::header::COOKIE, c);
    }

    // --json: auto-add Content-Type and Accept unless user-overridden via -H
    if args.json.is_some() {
        let user_has_content_type = args.header.iter().any(|h| {
            h.split_once(':')
                .map(|(n, _)| n.trim().eq_ignore_ascii_case("Content-Type"))
                .unwrap_or(false)
        });
        let user_has_accept = args.header.iter().any(|h| {
            h.split_once(':')
                .map(|(n, _)| n.trim().eq_ignore_ascii_case("Accept"))
                .unwrap_or(false)
        });
        if !user_has_content_type {
            request = request.header("Content-Type", "application/json");
        }
        if !user_has_accept {
            request = request.header("Accept", "application/json");
        }
    }

    // Body source priority: -T (upload-file) > --json > -d (data, unless -G).
    if let Some(path) = &args.upload_file {
        let body = fs::read(path)
            .with_context(|| format!("Failed to read upload file: {}", path.display()))?;
        request = request.body(body);
    } else if let Some(json_data) = &args.json {
        request = request.body(load_body_from_string(json_data)?);
    } else if !args.get_data {
        if let Some(data) = &args.data {
            request = request.body(load_body_from_string(data)?);
        }
    }

    if let Some(user_pass) = &args.user {
        let (user, pass) = user_pass
            .split_once(':')
            .map(|(u, p)| (u, Some(p)))
            .unwrap_or((user_pass.as_str(), None));
        request = request.basic_auth(user, pass);
    }

    if args.verbose >= 1 {
        let (host, port) = if let Ok(parsed) = url::Url::parse(url) {
            let h = parsed.host_str().unwrap_or("?").to_string();
            let p = parsed
                .port()
                .unwrap_or(if url.starts_with("https://") { 443 } else { 80 });
            (h, p)
        } else {
            ("?".to_string(), 0u16)
        };
        let is_https = url.starts_with("https://");

        // DNS resolution
        let resolved: Vec<std::net::SocketAddr> = format!("{host}:{port}")
            .to_socket_addrs()
            .map(|it| it.collect())
            .unwrap_or_default();

        let ipv4s: Vec<String> = resolved
            .iter()
            .filter(|a| a.is_ipv4())
            .map(|a| a.ip().to_string())
            .collect();
        let ipv6s: Vec<String> = resolved
            .iter()
            .filter(|a| a.is_ipv6())
            .map(|a| a.ip().to_string())
            .collect();

        if !resolved.is_empty() {
            eprintln!("* Host {}:{} was resolved.", host, port);
            eprintln!("* IPv6: {}", if ipv6s.is_empty() { "(none)".to_string() } else { ipv6s.join(", ") });
            eprintln!("* IPv4: {}", if ipv4s.is_empty() { "(none)".to_string() } else { ipv4s.join(", ") });
        }

        // Show the IP we'll try first
        if let Some(addr) = resolved.first() {
            eprintln!("* Trying {}:{}...", addr.ip(), port);
            eprintln!("* Connected to {} ({}) port {}", host, addr.ip(), port);
        }

        if is_https {
            eprintln!("* ALPN: recon offers h2,http/1.1");

            if args.verbose >= 2 {
                // Pre-flight rustls handshake: TLS version, cipher, ALPN, cert
                match crate::tls_probe::probe(&host, port) {
                    Ok(tls) => {
                        eprintln!(
                            "* SSL connection using {} / {}",
                            tls.version, tls.cipher
                        );
                        if let Some(ref alpn) = tls.alpn {
                            eprintln!("* ALPN: server accepted {alpn}");
                        }
                        eprintln!("* Server certificate:");
                        eprintln!("*  subject: {}", tls.subject);
                        eprintln!("*  issuer: {}", tls.issuer);
                        eprintln!("*  start date: (see --cert for full details)");
                        if tls.is_expired {
                            eprintln!("*  expire date: {} (EXPIRED)", tls.not_after);
                        } else {
                            eprintln!(
                                "*  expire date: {} ({} days remaining)",
                                tls.not_after, tls.days_remaining
                            );
                        }
                    }
                    Err(e) => eprintln!("* TLS probe unavailable: {e}"),
                }
                if let Some(user_pass) = &args.user {
                    let username = user_pass.split(':').next().unwrap_or(user_pass);
                    eprintln!("* Using Basic authentication for user '{username}'");
                }
            } else {
                eprintln!("* SSL/TLS connection to {host}");
            }
        }

        eprintln!(">");
        eprintln!("> {} {}", args.effective_method(), url);
        for h in &args.header {
            eprintln!("> {h}");
        }
        if let Some(c) = cookie {
            eprintln!("> Cookie: {c}");
        }
        eprintln!(">");
    }

    request
        .send()
        .with_context(|| format!("Request to {url} failed"))
}

// ── Cookie helpers ────────────────────────────────────────────────────────────

/// Builds a `Cookie: name=val; …` header value for the given URL, or returns None.
fn cookie_header(jar: Option<&CookieJar>, url: &str) -> Result<Option<String>> {
    let Some(jar) = jar else { return Ok(None) };
    let (domain, path) = url_domain_path(url);
    let is_https = url.starts_with("https://");
    let cookies = jar.cookies_for(&domain, &path, is_https)?;
    if cookies.is_empty() {
        Ok(None)
    } else {
        let s = cookies
            .iter()
            .map(|(n, v)| format!("{n}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        Ok(Some(s))
    }
}

/// Collects `Set-Cookie` headers from the response and persists them in the jar.
fn save_cookies(response: &Response, jar: &CookieJar, url: &str) -> Result<()> {
    let (domain, path) = url_domain_path(url);
    let set_cookies: Vec<String> = response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(String::from))
        .collect();
    for sc in set_cookies {
        jar.process_set_cookie(&sc, &domain, &path)?;
    }
    Ok(())
}

fn url_domain_path(url: &str) -> (String, String) {
    if let Ok(parsed) = url::Url::parse(url) {
        let domain = parsed.host_str().unwrap_or("").to_lowercase();
        let path = parsed.path().to_string();
        let path = if path.is_empty() { "/".to_string() } else { path };
        (domain, path)
    } else {
        (url.to_string(), "/".to_string())
    }
}

// ── Output helpers ────────────────────────────────────────────────────────────

fn print_hop_headers(response: &Response, from_url: &str, to_url: &str) {
    let status = response.status();

    let status_str = format!(
        "HTTP/{} {} {}",
        match response.version() {
            reqwest::Version::HTTP_10 => "1.0",
            reqwest::Version::HTTP_11 => "1.1",
            reqwest::Version::HTTP_2 => "2",
            reqwest::Version::HTTP_3 => "3",
            _ => "?",
        },
        status.as_u16(),
        status.canonical_reason().unwrap_or("")
    );

    writeln_stdout(&format!("* {from_url}"));
    writeln_stdout(&format!("< {}", status_str.yellow()));
    for (name, value) in response.headers() {
        writeln_stdout(&format!("< {}: {}", name, value.to_str().unwrap_or("?")));
    }
    writeln_stdout("<");
    writeln_stdout(&format!("* Redirecting to {to_url}"));
    writeln_stdout("");
}

fn writeln_stdout(line: &str) {
    use io::Write;
    let _ = writeln!(io::stdout(), "{line}");
}

// ── URL / method helpers ──────────────────────────────────────────────────────

fn resolve_redirect(base: &str, location: &str) -> Result<String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        Ok(location.to_string())
    } else {
        let base_url = url::Url::parse(base)
            .with_context(|| format!("Invalid base URL: {base}"))?;
        Ok(base_url
            .join(location)
            .with_context(|| format!("Invalid redirect location: {location}"))?
            .to_string())
    }
}

fn resolve_method(args: &Args) -> Result<Method> {
    let method_str = args.effective_method();
    Method::from_str(method_str.as_str())
        .map_err(|_| anyhow!("Invalid HTTP method: {}", method_str))
}

fn parse_header(header: &str) -> Result<(String, String)> {
    let pos = header
        .find(':')
        .ok_or_else(|| anyhow!("Invalid header format (expected 'Name: Value'): {header}"))?;
    let name = header[..pos].trim().to_string();
    let value = header[pos + 1..].trim().to_string();
    Ok((name, value))
}

/// Shared body loader for -d and --json: `@file` reads file, otherwise literal bytes.
fn load_body_from_string(s: &str) -> Result<Vec<u8>> {
    if let Some(path) = s.strip_prefix('@') {
        fs::read(path).with_context(|| format!("Failed to read file: {path}"))
    } else {
        Ok(s.as_bytes().to_vec())
    }
}
