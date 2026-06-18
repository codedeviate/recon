use anyhow::{bail, Result};

use crate::docs::DocOptions;

/// Quote a Rust string as a typst string literal.
pub fn typ_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Build an automatic centered title page from document metadata.
///
/// Emits a vertically-centered block: large title, smaller subtitle, a
/// horizontal divider, then version / date / author lines. Content is
/// escaped for typst markup context. Ends with its own `#pagebreak()`.
pub fn auto_cover(
    title: &str,
    subtitle: Option<&str>,
    author: Option<&str>,
    version: Option<&str>,
    date: Option<&str>,
) -> String {
    use super::translate::escape_typst;

    let mut c = String::new();
    c.push_str("#align(center + horizon)[\n");
    c.push_str(&format!("  #text(32pt, weight: \"bold\")[{}]\n", escape_typst(title)));
    if let Some(sub) = subtitle {
        c.push_str("  #v(0.6em)\n");
        c.push_str(&format!("  #text(18pt, fill: rgb(\"#555555\"))[{}]\n", escape_typst(sub)));
    }
    c.push_str("  #v(1.2em)\n");
    c.push_str("  #line(length: 40%)\n");
    c.push_str("  #v(1.2em)\n");
    for (label, val) in [("", version), ("", date), ("", author)] {
        if let Some(v) = val {
            let _ = label;
            c.push_str(&format!("  #text(12pt)[{}]\n", escape_typst(v)));
            c.push_str("  #linebreak()\n");
        }
    }
    c.push_str("]\n");
    c.push_str("#pagebreak()\n");
    c
}

/// Build a cover page from a user-supplied typst template body.
///
/// Prepends `#let` bindings for `title`, `subtitle`, `author`, `version`,
/// and `date` (absent values become the empty string `""` so the template
/// can always reference them), then the template body, then a
/// `#pagebreak()`.
pub fn cover_from_template(
    template_body: &str,
    title: &str,
    subtitle: Option<&str>,
    author: Option<&str>,
    version: Option<&str>,
    date: Option<&str>,
) -> String {
    let mut c = String::new();
    c.push_str(&format!("#let title = {}\n", typ_str(title)));
    c.push_str(&format!("#let subtitle = {}\n", typ_str(subtitle.unwrap_or(""))));
    c.push_str(&format!("#let author = {}\n", typ_str(author.unwrap_or(""))));
    c.push_str(&format!("#let version = {}\n", typ_str(version.unwrap_or(""))));
    c.push_str(&format!("#let date = {}\n", typ_str(date.unwrap_or(""))));
    c.push_str(template_body);
    if !c.ends_with('\n') {
        c.push('\n');
    }
    c.push_str("#pagebreak()\n");
    c
}

/// Build the typst preamble (`#set page`, `#set document`) for a document.
pub fn build_preamble(opts: &DocOptions) -> Result<String> {
    let mut p = String::new();
    p.push_str(&format!("#set page({})\n", typst_page_arg(&opts.page_size)?));
    let mut doc_args = Vec::new();
    if let Some(t) = &opts.title {
        doc_args.push(format!("title: {}", typ_str(t)));
    }
    if let Some(a) = &opts.author {
        doc_args.push(format!("author: {}", typ_str(a)));
    }
    if let Some(k) = &opts.keywords {
        let kws: Vec<String> = k.split(',').map(|s| typ_str(s.trim())).collect();
        doc_args.push(format!("keywords: ({})", kws.join(", ")));
    }
    if !doc_args.is_empty() {
        p.push_str(&format!("#set document({})\n", doc_args.join(", ")));
    }
    // Body text font (`--font`). Unset keeps typst's default serif
    // (Libertinus Serif). When set, the body switches to the named font and
    // code is explicitly pinned to the bundled monospace so prose-font
    // changes never bleed into code blocks / inline code.
    if let Some(font) = &opts.font {
        p.push_str(&format!("#set text(font: {})\n", typ_str(font)));
        p.push_str("#show raw: set text(font: \"DejaVu Sans Mono\")\n");
    }
    // Default code styling: a light-grey shaded background for block and
    // inline code, matching the chrome/CSS path so code-heavy documents
    // read clearly. Not configurable (a broader body-theme hook is the
    // place for that — see OUT-OF-SCOPE).
    p.push_str(
        "#show raw.where(block: true): it => block(\
         fill: luma(245), inset: 8pt, radius: 3pt, width: 100%)[#it]\n",
    );
    p.push_str(
        "#show raw.where(block: false): it => box(\
         fill: luma(245), inset: (x: 3pt), outset: (y: 2pt), radius: 2pt)[#it]\n",
    );
    Ok(p)
}

/// Map a --page-size value to the inner args of typst `set page(...)`.
/// Named papers map to typst paper ids; `WxH` (with units) -> width/height.
pub fn typst_page_arg(size: &str) -> Result<String> {
    let s = size.trim().to_ascii_lowercase();
    let named = match s.as_str() {
        "a3" => Some("a3"),
        "a4" => Some("a4"),
        "a5" => Some("a5"),
        "letter" => Some("us-letter"),
        "legal" => Some("us-legal"),
        _ => None,
    };
    if let Some(p) = named {
        return Ok(format!("paper: \"{p}\""));
    }
    if let Some((w, h)) = s.split_once('x') {
        if is_typst_len(w) && is_typst_len(h) {
            return Ok(format!("width: {w}, height: {h}"));
        }
    }
    bail!(
        "--page-size: unknown size '{size}' (expected a4, a3, a5, letter, \
           legal, or a custom WxH like 210mmx297mm)"
    )
}

fn is_typst_len(s: &str) -> bool {
    let units = ["mm", "cm", "in", "pt"];
    units.iter().any(|u| {
        s.strip_suffix(u)
            .map(|n| !n.is_empty() && n.parse::<f64>().is_ok())
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_named() {
        assert_eq!(typst_page_arg("a4").unwrap(), "paper: \"a4\"");
        assert_eq!(typst_page_arg("letter").unwrap(), "paper: \"us-letter\"");
        assert_eq!(typst_page_arg("legal").unwrap(), "paper: \"us-legal\"");
        assert_eq!(typst_page_arg("A3").unwrap(), "paper: \"a3\"");
    }

    #[test]
    fn page_size_custom_wxh() {
        assert_eq!(
            typst_page_arg("210mmx297mm").unwrap(),
            "width: 210mm, height: 297mm"
        );
    }

    #[test]
    fn page_size_unknown_errors() {
        assert!(typst_page_arg("banana").is_err());
        assert!(typst_page_arg("210x").is_err());
    }

    #[test]
    fn auto_cover_from_metadata() {
        let c = auto_cover("how to git", Some("subtitle"), Some("Thomas"), Some("2026.1"), Some("2026"));
        assert!(c.contains("how to git") && c.contains("#pagebreak()"));
    }

    #[test]
    fn cover_template_injects_lets() {
        let c = cover_from_template("#title", "how to git", None, Some("T"), None, None);
        assert!(c.contains("#let title = \"how to git\""));
        assert!(c.contains("#let author = \"T\""));
        assert!(c.contains("#pagebreak()"));
    }

    #[test]
    fn preamble_sets_body_font_and_pins_code_to_mono() {
        let mut o = DocOptions::default();
        o.page_size = "a4".into();
        o.font = Some("IBM Plex Sans".into());
        let p = build_preamble(&o).unwrap();
        assert!(p.contains("#set text(font: \"IBM Plex Sans\")"), "body font not set: {p}");
        assert!(
            p.contains("#show raw: set text(font: \"DejaVu Sans Mono\")"),
            "code not pinned to mono: {p}"
        );
    }

    #[test]
    fn preamble_omits_body_font_when_unset() {
        let mut o = DocOptions::default();
        o.page_size = "a4".into();
        let p = build_preamble(&o).unwrap();
        assert!(!p.contains("#set text(font:"), "should not set a body font: {p}");
    }

    #[test]
    fn preamble_shades_block_and_inline_code() {
        let mut o = DocOptions::default();
        o.page_size = "a4".into();
        let p = build_preamble(&o).unwrap();
        assert!(p.contains("#show raw.where(block: true):"), "missing block-code rule: {p}");
        assert!(p.contains("#show raw.where(block: false):"), "missing inline-code rule: {p}");
        assert!(p.contains("luma(245)"), "missing shaded fill: {p}");
    }
}
