use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::{Reader, Writer};

pub enum Format {
    Json,
    Xml,
    Html,
    Yaml,
    Csv,
    Tsv,
    Unknown,
}

/// Detect format from the Content-Type header value, falling back to body sniffing.
pub fn detect(content_type: &str, body: &str) -> Format {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    match mime.as_str() {
        "application/json" | "text/json" | "application/ld+json" => Format::Json,
        "application/xml" | "text/xml" | "application/rss+xml" | "application/atom+xml" => {
            Format::Xml
        }
        "text/html" | "application/xhtml+xml" => Format::Html,
        "application/yaml" | "text/yaml" | "application/x-yaml" | "text/x-yaml" => Format::Yaml,
        "text/csv" => Format::Csv,
        "text/tab-separated-values" | "text/tsv" => Format::Tsv,
        _ => sniff(body),
    }
}

fn sniff(body: &str) -> Format {
    let t = body.trim_start();
    if t.starts_with('{') || t.starts_with('[') {
        Format::Json
    } else if t.starts_with("<?xml") {
        Format::Xml
    } else if t.to_ascii_lowercase().contains("<!doctype html")
        || t.to_ascii_lowercase().starts_with("<html")
    {
        Format::Html
    } else {
        Format::Unknown
    }
}

pub fn run(body: &str, format: Format) -> Result<String> {
    match format {
        Format::Json => prettify_json(body),
        Format::Xml => prettify_xml(body),
        Format::Html => Ok(prettify_html(body)),
        Format::Yaml => prettify_yaml(body),
        Format::Csv => Ok(prettify_delimited(body, ',')),
        Format::Tsv => Ok(prettify_delimited(body, '\t')),
        Format::Unknown => Ok(body.to_string()),
    }
}

// ── JSON ─────────────────────────────────────────────────────────────────────

fn prettify_json(body: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(body).context("Failed to parse JSON")?;
    serde_json::to_string_pretty(&value).context("Failed to serialize JSON")
}

// ── XML ──────────────────────────────────────────────────────────────────────

fn prettify_xml(body: &str) -> Result<String> {
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(event) => writer.write_event(event).context("XML write error")?,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "XML parse error at position {}: {}",
                    reader.error_position(),
                    e
                ))
            }
        }
    }

    String::from_utf8(writer.into_inner()).context("XML output is not valid UTF-8")
}

// ── HTML ─────────────────────────────────────────────────────────────────────

fn prettify_html(body: &str) -> String {
    // Elements that are never followed by a closing tag
    const VOID: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ];
    // Raw-text elements whose content should not be re-indented
    const RAW: &[&str] = &["script", "style"];

    let mut out = String::new();
    let mut depth: usize = 0;
    let mut i = 0;
    let b = body.as_bytes();

    while i < b.len() {
        if b[i] != b'<' {
            // Collect text until the next tag
            let start = i;
            while i < b.len() && b[i] != b'<' {
                i += 1;
            }
            let text = std::str::from_utf8(&b[start..i])
                .unwrap_or("")
                .trim();
            if !text.is_empty() {
                push_indented(&mut out, depth, text);
            }
            continue;
        }

        // Collect the full tag (respecting quoted attribute values)
        let tag_start = i;
        i += 1;
        let mut in_str = false;
        let mut str_char = b'"';
        while i < b.len() {
            if !in_str && (b[i] == b'"' || b[i] == b'\'') {
                in_str = true;
                str_char = b[i];
            } else if in_str && b[i] == str_char {
                in_str = false;
            }
            if !in_str && b[i] == b'>' {
                i += 1;
                break;
            }
            i += 1;
        }

        let tag = match std::str::from_utf8(&b[tag_start..i]) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if tag.starts_with("</") {
            // Closing tag — dedent before printing
            depth = depth.saturating_sub(1);
            push_indented(&mut out, depth, tag);
        } else if tag.ends_with("/>")
            || tag.starts_with("<!--")
            || tag.starts_with("<?")
            || tag.starts_with("<!")
        {
            // Self-closing, comment, or declaration — no indent change
            push_indented(&mut out, depth, tag);
        } else {
            // Opening tag
            let name = tag[1..]
                .split(|c: char| !c.is_alphanumeric() && c != '-')
                .next()
                .unwrap_or("")
                .to_lowercase();

            push_indented(&mut out, depth, tag);

            if RAW.contains(&name.as_str()) {
                // Collect raw content verbatim until the matching closing tag
                let close = format!("</{name}");
                let raw_start = i;
                while i < b.len() {
                    if b[i..].starts_with(close.as_bytes()) {
                        break;
                    }
                    i += 1;
                }
                let raw = std::str::from_utf8(&b[raw_start..i]).unwrap_or("").trim();
                if !raw.is_empty() {
                    push_indented(&mut out, depth + 1, raw);
                }
            } else if !VOID.contains(&name.as_str()) {
                depth += 1;
            }
        }
    }

    out
}

fn push_indented(out: &mut String, depth: usize, s: &str) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(s);
    out.push('\n');
}

// ── YAML ─────────────────────────────────────────────────────────────────────

fn prettify_yaml(body: &str) -> Result<String> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(body).context("Failed to parse YAML")?;
    let out = serde_yaml::to_string(&value).context("Failed to serialize YAML")?;
    // serde_yaml prepends "---\n"; strip it so output stays clean
    Ok(out.strip_prefix("---\n").unwrap_or(&out).to_string())
}

// ── CSV / TSV ─────────────────────────────────────────────────────────────────

fn prettify_delimited(body: &str, delimiter: char) -> String {
    let rows: Vec<Vec<String>> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| split_delimited(line, delimiter))
        .collect();

    if rows.is_empty() {
        return body.to_string();
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; col_count];
    for row in &rows {
        for (j, cell) in row.iter().enumerate() {
            widths[j] = widths[j].max(cell.len());
        }
    }

    let divider = |fill: char| -> String {
        let parts: Vec<String> = widths.iter().map(|&w| fill.to_string().repeat(w + 2)).collect();
        format!("+{}+", parts.join("+"))
    };

    let row_divider = divider('-');
    let header_divider = divider('=');
    let mut out = String::new();

    for (i, row) in rows.iter().enumerate() {
        out.push_str(&row_divider);
        out.push('\n');

        let cells: String = (0..col_count)
            .map(|j| {
                let cell = row.get(j).map(String::as_str).unwrap_or("");
                format!(" {:width$} ", cell, width = widths[j])
            })
            .collect::<Vec<_>>()
            .join("|");
        out.push('|');
        out.push_str(&cells);
        out.push_str("|\n");

        // Heavier separator after the header row
        if i == 0 {
            out.push_str(&header_divider);
            out.push('\n');
        }
    }

    out.push_str(&row_divider);
    out.push('\n');
    out
}

fn split_delimited(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in line.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            d if d == delimiter && !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    fields.push(current.trim().to_string());
    fields
}
