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
use crate::metrics::RequestMetrics;

/// Populate the response-snapshot fields of `metrics` from a `Response`.
/// Called after the final response has been received (post-redirects, if any).
fn snapshot_response(metrics: &mut RequestMetrics, args: &Args, response: &Response) {
    metrics.first_response_byte = Some(std::time::Instant::now());
    metrics.url_effective = Some(response.url().to_string());
    metrics.status = Some(response.status().as_u16());
    metrics.http_version = Some(match response.version() {
        reqwest::Version::HTTP_10 => "1.0".to_string(),
        reqwest::Version::HTTP_11 => "1.1".to_string(),
        reqwest::Version::HTTP_2 => "2".to_string(),
        reqwest::Version::HTTP_3 => "3".to_string(),
        _ => "?".to_string(),
    });
    metrics.headers = Some(response.headers().clone());
    metrics.num_headers = response.headers().len() as u32;
    let hdr_bytes: u64 = response
        .headers()
        .iter()
        .map(|(k, v)| k.as_str().len() as u64 + v.as_bytes().len() as u64 + 4) // "K: V\r\n"
        .sum();
    metrics.size_header = hdr_bytes;
    // Not following redirects + 3xx → capture redirect_url
    if !args.follow_redirects && response.status().is_redirection() {
        if let Some(loc) = response.headers().get(reqwest::header::LOCATION) {
            metrics.redirect_url = loc.to_str().ok().map(String::from);
        }
    }
}

pub fn execute(args: &Args) -> Result<(Response, RequestMetrics)> {
    // --request-target: reqwest's blocking client doesn't expose the
    // request-target directly. Accepted at parse time but errors here
    // until we bypass reqwest for a direct hyper path.
    if args.request_target.is_some() {
        anyhow::bail!(
            "--request-target: not yet supported (reqwest 0.12 has no hook for \
             the request-line target; would require direct hyper). Use --url \
             with the desired path for now."
        );
    }

    // --disallow-username-in-url: security hardening. Reject URLs
    // that carry userinfo in command-line args.
    if args.disallow_username_in_url {
        let url_str = args.target_url();
        if let Ok(url) = reqwest::Url::parse(url_str) {
            if !url.username().is_empty() || url.password().is_some() {
                anyhow::bail!(
                    "--disallow-username-in-url: URL contains a user/pass component — \
                     pass credentials via `-u user:pass` instead"
                );
            }
        }
    }

    let mut builder = Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(args.insecure)
        .connect_timeout(Duration::from_secs(args.timeout));

    // --tlsv1.2 / --tlsv1.3: pin a minimum TLS version. If both are set,
    // tlsv1.3 wins (higher minimum).
    if args.tlsv13 {
        builder = builder.min_tls_version(reqwest::tls::Version::TLS_1_3);
    } else if args.tlsv12 {
        builder = builder.min_tls_version(reqwest::tls::Version::TLS_1_2);
    }

    // --cacert: trust an additional PEM root.
    if let Some(path) = &args.cacert {
        let pem = std::fs::read(path)
            .with_context(|| format!("--cacert: read {}", path.display()))?;
        let cert = reqwest::Certificate::from_pem(&pem)
            .with_context(|| format!("--cacert: parse PEM from {}", path.display()))?;
        builder = builder.add_root_certificate(cert);
    }

    // --capath: trust every *.pem / *.crt in the directory.
    if let Some(dir) = &args.capath {
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("--capath: read dir {}", dir.display()))?;
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext_ok = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| {
                    let lo = s.to_ascii_lowercase();
                    lo == "pem" || lo == "crt" || lo == "cer"
                })
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
            let pem = std::fs::read(&p)
                .with_context(|| format!("--capath: read {}", p.display()))?;
            let cert = reqwest::Certificate::from_pem(&pem)
                .with_context(|| format!("--capath: parse PEM from {}", p.display()))?;
            builder = builder.add_root_certificate(cert);
        }
    }

    // --ca-native: switch to OS-native trust roots.
    if args.ca_native {
        builder = builder.tls_built_in_root_certs(false);
        // reqwest's "use_native_tls_roots" pairs with rustls-native-certs
        // under feature "rustls-tls-native-roots"; here we're using the
        // plain rustls feature, so we fall through to zero roots unless
        // the user also supplied --cacert / --capath. That's the honest
        // behaviour — document it and log a warning.
        if args.cacert.is_none() && args.capath.is_none() && !args.insecure {
            eprintln!(
                "warning: --ca-native on a rustls build without \
                 rustls-tls-native-roots does nothing useful unless paired \
                 with --cacert or --capath; request will likely fail cert \
                 verification"
            );
        }
    }

    // --tls-max: upper bound on negotiated TLS version.
    if let Some(raw) = &args.tls_max {
        let v = match raw.as_str() {
            "1.2" => reqwest::tls::Version::TLS_1_2,
            "1.3" => reqwest::tls::Version::TLS_1_3,
            other => anyhow::bail!("--tls-max: unknown version '{other}' (expected 1.2 or 1.3)"),
        };
        builder = builder.max_tls_version(v);
    }

    // --http1.1 / --http2-prior-knowledge (HTTP version pinning).
    // --http2 alone is default reqwest behaviour for https://; accept
    // but don't toggle anything.
    if args.http11 {
        builder = builder.http1_only();
    } else if args.http2_prior_knowledge {
        builder = builder.http2_prior_knowledge();
    }

    // --tcp-nodelay, --no-keepalive, --keepalive-time.
    if args.tcp_nodelay {
        builder = builder.tcp_nodelay(true);
    }
    if args.no_keepalive {
        builder = builder.tcp_keepalive(None);
    } else if let Some(secs) = args.keepalive_time {
        builder = builder.tcp_keepalive(Duration::from_secs(secs));
    }

    // --connect-to: HOST1:PORT1:HOST2:PORT2 overrides.
    for spec in &args.connect_to {
        let parts: Vec<&str> = spec.splitn(4, ':').collect();
        if parts.len() != 4 {
            anyhow::bail!(
                "--connect-to: expected HOST1:PORT1:HOST2:PORT2, got '{spec}'"
            );
        }
        let port1: u16 = parts[1]
            .parse()
            .with_context(|| format!("--connect-to: port1 parse '{}'", parts[1]))?;
        let resolve_addr = format!("{}:{}", parts[2], parts[3])
            .to_socket_addrs()
            .with_context(|| format!("--connect-to: resolve '{}:{}'", parts[2], parts[3]))?
            .next()
            .ok_or_else(|| anyhow::anyhow!("--connect-to: no address for '{}:{}'", parts[2], parts[3]))?;
        builder = builder.resolve(parts[0], std::net::SocketAddr::new(resolve_addr.ip(), port1));
    }

    // --interface: bind to a specific local IP. Accepts IP literals OR
    // interface names (eth0, en0, lo0 on Linux/macOS via getifaddrs).
    // Windows falls back to "literal IP only" with a clear error.
    if let Some(iface) = &args.interface {
        let ip = crate::iface::resolve_interface(iface)?;
        builder = builder.local_address(ip);
    }

    // --dns-servers / --dns-ipv4-addr / --dns-ipv6-addr: install a
    // custom hickory-backed resolver. When none of these flags are set,
    // reqwest uses its default getaddrinfo path.
    if let Some(resolver) = crate::dns_resolver::build_from_args(args)? {
        builder = builder.dns_resolver(resolver);
    }

    // --max-time: total operation timeout. Accepts fractional seconds.
    if let Some(max) = args.max_time {
        builder = builder.timeout(Duration::from_millis((max * 1000.0) as u64));
    }

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

    if !args.compressed {
        builder = builder
            .no_gzip()
            .no_deflate()
            .no_brotli()
            .no_zstd();
    }

    // Proxy routing (--proxy + env vars).
    if let Some(proxy) = crate::proxy::build_proxy_from_args(args)? {
        builder = builder.proxy(proxy);
    }
    builder = crate::proxy::apply_proxy_tls(builder, args)?;

    // Client certificate (--cert + --key, mTLS).
    if let Some(identity) = crate::client_cert::build_identity(args)? {
        builder = builder.identity(identity);
    }

    let client = builder.build().context("Failed to build HTTP client")?;
    let method = resolve_method(args)?;
    let start_url = effective_url(args);

    let jar = args
        .cookiejar
        .as_deref()
        .map(CookieJar::open)
        .transpose()?;

    let mut metrics = RequestMetrics {
        request_start: Some(std::time::Instant::now()),
        ..RequestMetrics::default()
    };

    if args.lhead {
        execute_lhead(args, &client, method, jar.as_ref(), &start_url, &mut metrics)
            .map(|r| (r, metrics))
    } else {
        let cookie = cookie_header(jar.as_ref(), &start_url)?;
        let response = send_request(args, &client, method, &start_url, cookie.as_deref())?;
        if let Some(j) = &jar {
            save_cookies(&response, j, &start_url)?;
        }
        update_hsts_from_response(args, &response);
        snapshot_response(&mut metrics, args, &response);
        Ok((response, metrics))
    }
}

/// Consume the response's Strict-Transport-Security header (if any) and
/// update the HSTS store when `--hsts <path>` is set. Best-effort — a
/// load or save failure logs a warning but doesn't fail the request.
fn update_hsts_from_response(args: &Args, response: &reqwest::blocking::Response) {
    let Some(hsts_path) = &args.hsts else { return };
    let Some(sts_value) = response
        .headers()
        .get(reqwest::header::STRICT_TRANSPORT_SECURITY)
        .and_then(|v| v.to_str().ok())
    else {
        return;
    };
    // Only https:// responses carry authoritative HSTS directives.
    if response.url().scheme() != "https" {
        return;
    }
    let Some(host) = response.url().host_str() else { return };

    let mut store = match crate::hsts::HstsStore::load(hsts_path) {
        Ok(s) => s,
        Err(e) => {
            if !args.silent {
                eprintln!("warning: HSTS load: {e}");
            }
            return;
        }
    };
    if store.update_from_sts_header(host, sts_value) {
        if let Err(e) = store.save(hsts_path) {
            if !args.silent {
                eprintln!("warning: HSTS save: {e}");
            }
        }
    }
}

/// Returns the effective request URL. When -G / --get is active, appends -d data as a query string.
/// Resolve `--time-cond` / `--timestamping` into (header_name, value).
/// Returns `Ok(None)` when no conditional is requested. Header name
/// is `If-Modified-Since` by default; prefix the `-z` value with `-`
/// to switch to `If-Unmodified-Since`.
fn resolve_time_cond(args: &Args) -> Result<Option<(&'static str, String)>> {
    let raw = if let Some(s) = args.time_cond.as_deref() {
        s.to_string()
    } else if args.timestamping {
        // --timestamping uses the -o target's mtime.
        let target = args.output.as_ref().ok_or_else(|| {
            anyhow::anyhow!("--timestamping requires -o <PATH> to know which file's mtime to use")
        })?;
        if !target.exists() {
            // No local file → don't send a conditional; the server will
            // return the full body. Same as curl's behaviour.
            return Ok(None);
        }
        return mtime_to_header("If-Modified-Since", target).map(Some);
    } else {
        return Ok(None);
    };
    let (invert, rest) = if let Some(r) = raw.strip_prefix('-') {
        (true, r.to_string())
    } else {
        (false, raw)
    };
    let header_name: &'static str = if invert {
        "If-Unmodified-Since"
    } else {
        "If-Modified-Since"
    };
    // Try path first; fall back to date-parsing.
    let as_path = std::path::Path::new(&rest);
    if as_path.exists() {
        mtime_to_header(header_name, as_path).map(Some)
    } else {
        // Assume RFC 2616 / RFC 2822 date string. Re-format via httpdate.
        let ts = httpdate::parse_http_date(&rest)
            .or_else(|_| {
                httpdate::parse_http_date(&rest.replace(' ', ", "))
            })
            .map_err(|_| {
                anyhow::anyhow!(
                    "--time-cond: '{rest}' is neither a valid HTTP-date nor a readable file path"
                )
            })?;
        Ok(Some((header_name, httpdate::fmt_http_date(ts))))
    }
}

fn mtime_to_header(name: &'static str, path: &std::path::Path) -> Result<(&'static str, String)> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("read mtime of {}", path.display()))?;
    let mtime = meta
        .modified()
        .with_context(|| format!("mtime of {} not available", path.display()))?;
    Ok((name, httpdate::fmt_http_date(mtime)))
}

fn effective_url(args: &Args) -> String {
    let mut base = args.target_url().to_string();

    // --url-query: append URL-encoded data to the query string.
    // Same sub-form grammar as --data-urlencode.
    for raw in &args.url_query {
        if let Ok(encoded) = encode_url_query_part(raw) {
            base = join_query(&base, &encoded);
        }
    }

    if args.get_data {
        if let Some(data) = &args.data {
            base = join_query(&base, data);
        }
    }
    base
}

fn join_query(base: &str, encoded: &str) -> String {
    if base.contains('?') {
        format!("{base}&{encoded}")
    } else {
        format!("{base}?{encoded}")
    }
}

/// Encode a --url-query / --data-urlencode sub-form into a query
/// fragment. Sub-forms:
/// - `content`        →  URL-encode the whole value
/// - `=content`       →  same (leading `=` stripped)
/// - `name=content`   →  name (literal) + "=" + URL-encoded content
/// - `@file`          →  read file, URL-encode the contents
/// - `name@file`      →  name (literal) + "=" + URL-encoded file contents
fn encode_url_query_part(raw: &str) -> Result<String> {
    use reqwest::Url;
    let percent = |s: &str| -> String {
        // reqwest's internal form-encoder is reqwest::Url-agnostic, so
        // drive percent-encoding via urlencoding-compatible path.
        Url::parse_with_params("http://x/", &[("k", s)])
            .ok()
            .and_then(|u| u.query().map(|q| q.trim_start_matches("k=").to_string()))
            .unwrap_or_default()
    };
    if let Some(path) = raw.strip_prefix('@') {
        let bytes = std::fs::read_to_string(path)
            .with_context(|| format!("--url-query @{path}: read"))?;
        return Ok(percent(bytes.trim_end_matches('\n')));
    }
    if let Some((name, rest)) = raw.split_once('=') {
        if name.is_empty() {
            return Ok(percent(rest));
        }
        return Ok(format!("{name}={}", percent(rest)));
    }
    if let Some((name, path)) = raw.split_once('@') {
        let bytes = std::fs::read_to_string(path)
            .with_context(|| format!("--url-query {name}@{path}: read"))?;
        return Ok(format!("{name}={}", percent(bytes.trim_end_matches('\n'))));
    }
    Ok(percent(raw))
}

fn execute_lhead(
    args: &Args,
    client: &Client,
    method: Method,
    jar: Option<&CookieJar>,
    start_url: &str,
    metrics: &mut RequestMetrics,
) -> Result<Response> {
    let mut current_url = start_url.to_string();
    let mut redirects: u32 = 0;

    loop {
        let cookie = cookie_header(jar, &current_url)?;
        let response = send_request(args, client, method.clone(), &current_url, cookie.as_deref())?;

        if let Some(j) = jar {
            save_cookies(&response, j, &current_url)?;
        }

        let status = response.status();
        let next_url = if status.is_redirection() && (redirects as usize) < args.max_redirs {
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
            metrics.num_redirects = redirects;
            snapshot_response(metrics, args, &response);
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
        if !user_has_header(&args.header, "Content-Type") {
            request = request.header("Content-Type", "application/json");
        }
        if !user_has_header(&args.header, "Accept") {
            request = request.header("Accept", "application/json");
        }
    }

    // --compressed: advertise supported encodings unless user set their own
    if args.compressed && !user_has_header(&args.header, "Accept-Encoding") {
        request = request.header("Accept-Encoding", "gzip, deflate, br, zstd");
    }

    // -r, --range: set Range header unless user already did.
    if let Some(range) = &args.range {
        if !user_has_header(&args.header, "Range") {
            request = request.header("Range", format!("bytes={range}"));
        }
    }

    // -z, --time-cond / --timestamping: If-Modified-Since or
    // If-Unmodified-Since from date string or local file mtime.
    if let Some(resolved) = resolve_time_cond(args)? {
        let (name, value) = resolved;
        if !user_has_header(&args.header, name) {
            request = request.header(name, value);
        }
    }

    // --oauth2-bearer: Authorization header sugar.
    if let Some(token) = &args.oauth2_bearer {
        if !user_has_header(&args.header, "Authorization") {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
    }

    // --etag-compare: If-None-Match from an ETag file.
    if let Some(path) = &args.etag_compare {
        let etag = std::fs::read_to_string(path)
            .with_context(|| format!("--etag-compare: read {}", path.display()))?;
        let trimmed = etag.trim();
        if !trimmed.is_empty() && !user_has_header(&args.header, "If-None-Match") {
            request = request.header("If-None-Match", trimmed);
        }
    }

    // Body source priority: -F (multipart) wins over anything else.
    // Then: -T > --json > --data-raw > --data-binary > --data-urlencode > -d (unless -G).
    if !args.form.is_empty() || !args.form_string.is_empty() {
        request = apply_multipart_form(request, args)?;
    } else if let Some(path) = &args.upload_file {
        // curl-compatible: -T - reads from stdin.
        let body = if path.as_os_str() == "-" {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)
                .context("Failed to read upload body from stdin")?;
            buf
        } else {
            fs::read(path)
                .with_context(|| format!("Failed to read upload file: {}", path.display()))?
        };
        // --crlf: convert bare LFs to CRLFs before sending.
        let body = if args.crlf {
            crlf_convert(&body)
        } else {
            body
        };
        request = apply_request_body(request, body, args)?;
    } else if let Some(json_data) = &args.json {
        request = apply_request_body(request, load_body_from_string(json_data)?, args)?;
    } else if let Some(raw) = &args.data_raw {
        request = apply_request_body(request, raw.as_bytes().to_vec(), args)?;
    } else if let Some(bin) = &args.data_binary {
        let body = if let Some(path) = bin.strip_prefix('@') {
            fs::read(path).with_context(|| format!("Failed to read file: {path}"))?
        } else {
            bin.as_bytes().to_vec()
        };
        request = apply_request_body(request, body, args)?;
    } else if !args.data_urlencode.is_empty() {
        let joined = args.data_urlencode
            .iter()
            .map(|s| urlencode_form(s))
            .collect::<Result<Vec<_>>>()?
            .join("&");
        if !user_has_header(&args.header, "Content-Type") {
            request = request.header("Content-Type", "application/x-www-form-urlencoded");
        }
        request = apply_request_body(request, joined.into_bytes(), args)?;
    } else if !args.get_data {
        if let Some(data) = &args.data {
            // @- reads from stdin (no CRLF stripping — curl doesn't strip stdin).
            // @file reads from a file (CRLF stripped, matching curl -d @file).
            // Anything else is used as literal bytes.
            let body = if data == "@-" {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)
                    .context("Failed to read body from stdin")?;
                buf
            } else if let Some(path) = data.strip_prefix('@') {
                let raw = fs::read(path).with_context(|| format!("Failed to read file: {path}"))?;
                raw.into_iter().filter(|&b| b != b'\r' && b != b'\n').collect()
            } else {
                data.as_bytes().to_vec()
            };
            request = apply_request_body(request, body, args)?;
        }
    }

    if let Some(user_pass) = &args.user {
        let (user, pass) = user_pass
            .split_once(':')
            .map(|(u, p)| (u, Some(p)))
            .unwrap_or((user_pass.as_str(), None));
        request = request.basic_auth(user, pass);
    } else if let Some(path) = crate::netrc::resolve_netrc_path(args) {
        // --netrc / --netrc-file / --netrc-optional: inject Basic auth
        // from ~/.netrc (or override file) when -u isn't set.
        let url = reqwest::Url::parse(url).ok();
        if let Some(host) = url.as_ref().and_then(|u| u.host_str()) {
            match crate::netrc::lookup(&path, host, args.netrc_optional) {
                Ok(Some(entry)) => {
                    if let Some(login) = entry.login {
                        request = request.basic_auth(login, entry.password);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    if !args.netrc_optional {
                        return Err(e);
                    }
                }
            }
        }
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

/// Build and attach a `multipart::Form` from --form + --form-string args.
///
/// Grammar (curl-compatible):
/// - `name=value`             — literal value
/// - `name=@file`             — file contents; MIME inferred from extension
/// - `name=@file;type=MIME`   — explicit MIME type
/// - `name=@file;filename=X`  — override the reported filename
/// - `name=<file`             — file contents, filename NOT attached
/// - `name=<-`                — content from stdin
fn apply_multipart_form(
    mut request: reqwest::blocking::RequestBuilder,
    args: &Args,
) -> Result<reqwest::blocking::RequestBuilder> {
    use reqwest::blocking::multipart::{Form, Part};

    let mut form = Form::new();

    // --form-string: always literal (no @ / < interpretation).
    for spec in &args.form_string {
        let (name, value) = spec
            .split_once('=')
            .ok_or_else(|| anyhow!("--form-string: expected NAME=VALUE, got '{spec}'"))?;
        let escaped_name = maybe_escape(name, args.form_escape);
        form = form.part(escaped_name.clone(), Part::text(value.to_string()));
    }

    // --form: full curl grammar.
    for spec in &args.form {
        let (name, value) = spec
            .split_once('=')
            .ok_or_else(|| anyhow!("--form: expected NAME=VALUE, got '{spec}'"))?;
        let escaped_name = maybe_escape(name, args.form_escape);
        let part = build_form_part(value, args.form_escape)
            .with_context(|| format!("--form '{spec}'"))?;
        form = form.part(escaped_name, part);
    }

    request = request.multipart(form);
    Ok(request)
}

fn build_form_part(value: &str, escape: bool) -> Result<reqwest::blocking::multipart::Part> {
    use reqwest::blocking::multipart::Part;

    // `<-`: content from stdin, no filename.
    if value == "<-" {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)
            .context("--form: read stdin")?;
        return Ok(Part::bytes(buf));
    }

    // `<file`: content from file, no filename attached.
    if let Some(path) = value.strip_prefix('<') {
        let bytes =
            fs::read(path).with_context(|| format!("--form: read {path}"))?;
        return Ok(Part::bytes(bytes));
    }

    // `@file[;type=MIME][;filename=NAME]`: content from file, attach filename.
    if let Some(rest) = value.strip_prefix('@') {
        // Split off optional ;type= / ;filename= modifiers.
        let mut path = rest.to_string();
        let mut mime: Option<String> = None;
        let mut filename_override: Option<String> = None;

        while let Some(pos) = path.rfind(';') {
            let (head, tail) = path.split_at(pos);
            let tail = &tail[1..]; // drop `;`
            let (k, v) = tail
                .split_once('=')
                .ok_or_else(|| anyhow!("--form modifier expects key=value, got '{tail}'"))?;
            match k.trim().to_ascii_lowercase().as_str() {
                "type" => mime = Some(v.trim().to_string()),
                "filename" => filename_override = Some(v.trim().to_string()),
                other => return Err(anyhow!("--form: unknown modifier '{other}'")),
            }
            path = head.to_string();
        }

        let bytes =
            fs::read(&path).with_context(|| format!("--form: read {path}"))?;
        let default_name = std::path::Path::new(&path)
            .file_name()
            .map(|o| o.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let filename = filename_override.unwrap_or(default_name);
        let filename = maybe_escape(&filename, escape);

        let mut part = Part::bytes(bytes).file_name(filename);
        if let Some(m) = mime {
            part = part
                .mime_str(&m)
                .with_context(|| format!("--form: invalid MIME '{m}'"))?;
        }
        return Ok(part);
    }

    // Literal value.
    Ok(Part::text(value.to_string()))
}

/// LF → CRLF conversion for `--crlf`. Leaves existing CRLFs alone.
fn crlf_convert(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + input.len() / 32);
    let mut prev = 0u8;
    for &b in input {
        if b == b'\n' && prev != b'\r' {
            out.push(b'\r');
        }
        out.push(b);
        prev = b;
    }
    out
}

fn maybe_escape(s: &str, on: bool) -> String {
    if !on {
        return s.to_string();
    }
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn resolve_method(args: &Args) -> Result<Method> {
    // --spider overrides the method; always HEAD regardless of -X / -d.
    if args.spider {
        return Ok(Method::HEAD);
    }
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

/// Shared body loader for -d and --json.
/// - `@-` reads from stdin
/// - `@file` reads from file
/// - anything else is literal bytes
pub(crate) fn load_body_from_string(s: &str) -> Result<Vec<u8>> {
    if s == "@-" {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)
            .context("Failed to read body from stdin")?;
        return Ok(buf);
    }
    if let Some(path) = s.strip_prefix('@') {
        return fs::read(path).with_context(|| format!("Failed to read file: {path}"));
    }
    Ok(s.as_bytes().to_vec())
}

fn user_has_header(headers: &[String], name: &str) -> bool {
    headers.iter().any(|h| {
        h.split_once(':')
            .map(|(n, _)| n.trim().eq_ignore_ascii_case(name))
            .unwrap_or(false)
    })
}

/// Set the request body, transcoding from UTF-8 to the target charset
/// when one is in scope. Priority (first match wins):
///   1. `--request-charset-passthrough` → skip (return body verbatim).
///   2. `--request-charset NAME` → use NAME.
///   3. `charset=X` on an explicit `Content-Type` header → use X.
///   4. Otherwise → return body verbatim.
fn apply_request_body(
    request: reqwest::blocking::RequestBuilder,
    body: Vec<u8>,
    args: &Args,
) -> Result<reqwest::blocking::RequestBuilder> {
    use encoding_rs::UTF_8;

    if args.request_charset_passthrough {
        return Ok(request.body(body));
    }

    let target_label: Option<String> = if let Some(label) = &args.request_charset {
        Some(label.clone())
    } else {
        args.header.iter().find_map(|h| {
            let (name, value) = h.split_once(':')?;
            if !name.trim().eq_ignore_ascii_case("content-type") {
                return None;
            }
            crate::text_encoding::parse_content_type_charset(value.trim())
        })
    };

    let Some(label) = target_label else {
        return Ok(request.body(body));
    };

    let target = crate::text_encoding::resolve(&label)
        .with_context(|| format!("request-charset: unknown charset '{label}'"))?;

    if target == UTF_8 {
        return Ok(request.body(body));
    }

    // Input is UTF-8 (from shell / file). Decode to str then re-encode.
    let text = String::from_utf8_lossy(&body);
    let r = crate::text_encoding::encode_from_str(&text, target);
    if r.had_unmappable && !args.silent {
        eprintln!(
            "! request body: one or more characters not representable in {} — substituted with '?'",
            target.name()
        );
    }
    Ok(request.body(r.bytes))
}

/// Implements curl's --data-urlencode sub-forms.
/// Returns the URL-encoded key=value (or raw encoded value) fragment, ready to
/// be joined with `&`.
fn urlencode_form(s: &str) -> Result<String> {
    if let Some(at_idx) = s.find('@') {
        let (prefix, at_and_rest) = s.split_at(at_idx);
        let path = &at_and_rest[1..];
        if !prefix.is_empty() && !prefix.contains('=') {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read file: {path}"))?;
            return Ok(format!("{}={}", prefix, percent_encode(&content)));
        }
        if prefix.is_empty() {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read file: {path}"))?;
            return Ok(percent_encode(&content));
        }
        // prefix contains '=' → fall through to name=content handling
    }
    if let Some((name, content)) = s.split_once('=') {
        if !name.is_empty() {
            return Ok(format!("{}={}", name, percent_encode(content)));
        }
        return Ok(percent_encode(content));
    }
    Ok(percent_encode(s))
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod urlencode_tests {
    use super::*;

    #[test]
    fn encode_plain_content() {
        assert_eq!(urlencode_form("hello world").unwrap(), "hello%20world");
    }

    #[test]
    fn encode_equals_prefix_keeps_eq() {
        assert_eq!(urlencode_form("=hello world").unwrap(), "hello%20world");
    }

    #[test]
    fn encode_name_equals_content() {
        assert_eq!(urlencode_form("name=hello world").unwrap(), "name=hello%20world");
    }

    #[test]
    fn encode_at_file_reads_literal() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "a=b&c").unwrap();
        let form = format!("@{}", tmp.path().display());
        assert_eq!(urlencode_form(&form).unwrap(), "a%3Db%26c");
    }

    #[test]
    fn encode_name_at_file_keeps_name() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "x y").unwrap();
        let form = format!("key@{}", tmp.path().display());
        assert_eq!(urlencode_form(&form).unwrap(), "key=x%20y");
    }
}

#[cfg(test)]
mod load_body_from_string_tests {
    use super::*;

    #[test]
    fn load_body_from_string_literal() {
        let body = load_body_from_string("hello").unwrap();
        assert_eq!(body, b"hello");
    }

    #[test]
    fn load_body_from_string_at_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"binary\x00data").unwrap();
        let arg = format!("@{}", tmp.path().display());
        let body = load_body_from_string(&arg).unwrap();
        assert_eq!(body, b"binary\x00data");
    }
}
