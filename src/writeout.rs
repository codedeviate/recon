use anyhow::{Context, Result};
use std::io::Read;

/// A single token in a parsed format string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Literal(String),
    Variable(String),       // %{name}
    Header(String),         // %{header{name}}
    Json,                   // %{json}
    StderrSwitch,           // %{stderr}
    StdoutSwitch,           // %{stdout}
}

/// Load a format string argument: plain string, `@<file>`, or `@-` (stdin).
pub fn load_format(arg: &str) -> Result<String> {
    if arg == "@-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).context("reading stdin for -w")?;
        return Ok(s);
    }
    if let Some(path) = arg.strip_prefix('@') {
        return std::fs::read_to_string(path)
            .with_context(|| format!("reading format file: {path}"));
    }
    Ok(arg.to_string())
}

/// Parse a format string into tokens.
/// Unknown %{var} tokens are preserved as `Token::Variable(name)` so the
/// renderer (Task 12) can decide whether to emit literally or error.
pub fn parse(format: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut lit = String::new();
    let mut iter = format.char_indices().peekable();

    while let Some((i, c)) = iter.next() {
        // Escape sequences: \n \t \r \\, unknown preserved as "\X"
        if c == '\\' {
            if let Some(&(_, next)) = iter.peek() {
                match next {
                    'n' => { lit.push('\n'); iter.next(); continue; }
                    't' => { lit.push('\t'); iter.next(); continue; }
                    'r' => { lit.push('\r'); iter.next(); continue; }
                    '\\' => { lit.push('\\'); iter.next(); continue; }
                    other => {
                        lit.push('\\');
                        lit.push(other);
                        iter.next();
                        continue;
                    }
                }
            }
        }

        // %% → literal %
        if c == '%' {
            if let Some(&(_, '%')) = iter.peek() {
                lit.push('%');
                iter.next();
                continue;
            }
            // %{...}
            if let Some(&(_, '{')) = iter.peek() {
                // Byte-based matching-brace search starting at `i` in the full string
                if let Some(end_rel) = find_matching_brace(&format.as_bytes()[i..]) {
                    // end_rel is index of matching '}' relative to i; body is format[i+2..i+end_rel]
                    let inner = &format[i + 2..i + end_rel];
                    if !lit.is_empty() {
                        tokens.push(Token::Literal(std::mem::take(&mut lit)));
                    }
                    tokens.push(classify_token(inner));
                    // Advance iter past the closing brace
                    // We need to skip chars until we reach byte index i + end_rel + 1
                    let target = i + end_rel + 1;
                    while let Some(&(j, _)) = iter.peek() {
                        if j >= target { break; }
                        iter.next();
                    }
                    continue;
                }
                // Unmatched %{ — fall through and treat '%' as literal
            }
        }

        lit.push(c);
    }

    if !lit.is_empty() {
        tokens.push(Token::Literal(lit));
    }
    tokens
}

/// Given bytes starting with `%{`, return the index of the matching `}`
/// relative to the start. Handles nested braces for `%{header{name}}`.
fn find_matching_brace(s: &[u8]) -> Option<usize> {
    debug_assert_eq!(&s[..2], b"%{");
    let mut depth = 1;
    let mut i = 2;
    while i < s.len() {
        match s[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn classify_token(inner: &str) -> Token {
    if inner == "json" { return Token::Json; }
    if inner == "stderr" { return Token::StderrSwitch; }
    if inner == "stdout" { return Token::StdoutSwitch; }
    if let Some(rest) = inner.strip_prefix("header{") {
        if let Some(name) = rest.strip_suffix('}') {
            return Token::Header(name.to_string());
        }
    }
    Token::Variable(inner.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_literal() {
        assert_eq!(parse("hello"), vec![Token::Literal("hello".into())]);
    }

    #[test]
    fn escapes() {
        assert_eq!(
            parse(r"line1\nline2\t\\end"),
            vec![Token::Literal("line1\nline2\t\\end".into())]
        );
    }

    #[test]
    fn double_percent() {
        assert_eq!(parse("50%%"), vec![Token::Literal("50%".into())]);
    }

    #[test]
    fn simple_variable() {
        assert_eq!(parse("%{http_code}"), vec![Token::Variable("http_code".into())]);
    }

    #[test]
    fn variable_embedded_in_literal() {
        assert_eq!(
            parse("status=%{http_code}\n"),
            vec![
                Token::Literal("status=".into()),
                Token::Variable("http_code".into()),
                Token::Literal("\n".into()),
            ]
        );
    }

    #[test]
    fn header_extraction() {
        assert_eq!(
            parse("%{header{Content-Type}}"),
            vec![Token::Header("Content-Type".into())]
        );
    }

    #[test]
    fn json_token() {
        assert_eq!(parse("%{json}"), vec![Token::Json]);
    }

    #[test]
    fn stream_switches() {
        assert_eq!(
            parse("out%{stderr}err%{stdout}back"),
            vec![
                Token::Literal("out".into()),
                Token::StderrSwitch,
                Token::Literal("err".into()),
                Token::StdoutSwitch,
                Token::Literal("back".into()),
            ]
        );
    }

    #[test]
    fn unknown_variable_preserved() {
        assert_eq!(
            parse("%{unknown_xyz}"),
            vec![Token::Variable("unknown_xyz".into())]
        );
    }

    #[test]
    fn unterminated_brace_treated_as_literal() {
        // %{no closing brace → fall through: '%' and '{' emit as literal chars.
        assert_eq!(parse("%{open"), vec![Token::Literal("%{open".into())]);
    }

    #[test]
    fn preserves_utf8_in_literal() {
        // Regression: byte-based parser would corrupt this to garbage
        assert_eq!(
            parse("é=%{http_code}"),
            vec![
                Token::Literal("é=".into()),
                Token::Variable("http_code".into()),
            ]
        );
    }

    #[test]
    fn load_plain_string() {
        assert_eq!(load_format("hello").unwrap(), "hello");
    }

    #[test]
    fn load_at_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "from file").unwrap();
        let arg = format!("@{}", tmp.path().display());
        assert_eq!(load_format(&arg).unwrap(), "from file");
    }
}

use crate::metrics::RequestMetrics;
use reqwest::header::HeaderMap;
use serde_json::{json, Map, Value};
use std::io::Write;

/// Render destination for `-w` output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Render parsed tokens using captured metrics. Handles `%{stderr}` /
/// `%{stdout}` switches inline.
pub fn render(tokens: &[Token], metrics: &RequestMetrics) -> anyhow::Result<()> {
    let empty_hdrs = HeaderMap::new();
    let headers = metrics.headers.as_ref().unwrap_or(&empty_hdrs);
    let vars = build_variables(metrics);
    let mut current = Stream::Stdout;

    for tok in tokens {
        let out = match tok {
            Token::Literal(s) => s.clone(),
            Token::Variable(name) => vars
                .get(name.as_str())
                .cloned()
                .unwrap_or_else(|| format!("%{{{}}}", name)), // curl: preserve unknown
            Token::Header(name) => header_value(headers, name),
            Token::Json => render_json(&vars),
            Token::StderrSwitch => {
                current = Stream::Stderr;
                continue;
            }
            Token::StdoutSwitch => {
                current = Stream::Stdout;
                continue;
            }
        };
        write_to(current, &out)?;
    }
    Ok(())
}

fn write_to(stream: Stream, s: &str) -> anyhow::Result<()> {
    match stream {
        Stream::Stdout => write!(std::io::stdout(), "{}", s)?,
        Stream::Stderr => write!(std::io::stderr(), "{}", s)?,
    }
    Ok(())
}

fn header_value(headers: &HeaderMap, name: &str) -> String {
    for (k, v) in headers {
        if k.as_str().eq_ignore_ascii_case(name) {
            return v.to_str().unwrap_or("").to_string();
        }
    }
    String::new()
}

/// Build the variable → string map from RequestMetrics.
fn build_variables(m: &RequestMetrics) -> std::collections::HashMap<String, String> {
    let mut v = std::collections::HashMap::new();

    let status = m.status.unwrap_or(0).to_string();
    v.insert("http_code".into(), status.clone());
    v.insert("response_code".into(), status);

    v.insert(
        "http_version".into(),
        m.http_version.clone().unwrap_or_default(),
    );

    let url_effective = m.url_effective.clone().unwrap_or_default();
    v.insert("url_effective".into(), url_effective.clone());
    v.insert("url".into(), url_effective.clone());

    let scheme = reqwest::Url::parse(&url_effective)
        .ok()
        .map(|u| u.scheme().to_string())
        .unwrap_or_default();
    v.insert("scheme".into(), scheme);

    let ct = m
        .headers
        .as_ref()
        .and_then(|h| h.get(reqwest::header::CONTENT_TYPE))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    v.insert("content_type".into(), ct);

    v.insert("size_download".into(), m.size_download.to_string());
    v.insert("size_upload".into(), m.size_upload.to_string());
    v.insert("size_header".into(), m.size_header.to_string());

    let total_secs = m.time_total().as_secs_f64();
    let speed = if total_secs > 0.0 {
        (m.size_download as f64 / total_secs) as u64
    } else {
        0
    };
    v.insert("speed_download".into(), speed.to_string());

    v.insert("num_redirects".into(), m.num_redirects.to_string());
    v.insert("num_headers".into(), m.num_headers.to_string());
    v.insert(
        "redirect_url".into(),
        m.redirect_url.clone().unwrap_or_default(),
    );

    let phase = m.phase.lock().unwrap();
    let remote = phase.remote_ip.map(|a| a.ip().to_string()).unwrap_or_default();
    let local = phase.local_ip.map(|a| a.ip().to_string()).unwrap_or_default();
    v.insert("remote_ip".into(), remote);
    v.insert("local_ip".into(), local);

    let ns = phase.dns_duration.map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let tc = phase.tcp_duration.map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let tl = phase.tls_duration.map(|d| d.as_secs_f64()).unwrap_or(0.0);
    drop(phase);

    v.insert("time_namelookup".into(), fmt_time(ns));
    v.insert("time_connect".into(), fmt_time(ns + tc));
    v.insert("time_appconnect".into(), fmt_time(ns + tc + tl));
    v.insert("time_pretransfer".into(), fmt_time(ns + tc + tl));
    v.insert(
        "time_starttransfer".into(),
        fmt_time(m.time_starttransfer().as_secs_f64()),
    );
    v.insert(
        "time_redirect".into(),
        fmt_time(m.redirect_duration.as_secs_f64()),
    );
    v.insert("time_total".into(), fmt_time(m.time_total().as_secs_f64()));

    v
}

fn fmt_time(secs: f64) -> String {
    format!("{:.6}", secs)
}

/// Render %{json}: all variables as a JSON object with stable alphabetical keys.
/// Numeric variables are typed as numbers; strings as strings.
fn render_json(vars: &std::collections::HashMap<String, String>) -> String {
    const NUMERIC: &[&str] = &[
        "http_code",
        "response_code",
        "size_download",
        "size_upload",
        "size_header",
        "speed_download",
        "num_redirects",
        "num_headers",
        "time_namelookup",
        "time_connect",
        "time_appconnect",
        "time_pretransfer",
        "time_starttransfer",
        "time_redirect",
        "time_total",
    ];

    let mut keys: Vec<&String> = vars.keys().collect();
    keys.sort();

    let mut map = Map::new();
    for k in keys {
        let value = &vars[k];
        if NUMERIC.contains(&k.as_str()) {
            let n: Value = value
                .parse::<f64>()
                .map(|f| json!(f))
                .unwrap_or_else(|_| json!(value));
            map.insert(k.clone(), n);
        } else {
            map.insert(k.clone(), json!(value));
        }
    }
    Value::Object(map).to_string()
}

#[cfg(test)]
mod render_tests {
    use super::*;

    #[test]
    fn fmt_time_six_decimals() {
        assert_eq!(fmt_time(0.0), "0.000000");
        assert_eq!(fmt_time(0.123), "0.123000");
        assert_eq!(fmt_time(1.234567), "1.234567");
    }
}
