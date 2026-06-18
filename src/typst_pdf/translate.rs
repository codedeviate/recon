//! comrak AST → typst markup translation.
//!
//! Covers GFM: headings, paragraphs, emphasis, inline/block code, links,
//! strikethrough, block quotes, thematic breaks, ordered/unordered/nested/task
//! lists, tables, and footnotes. Raw HTML is rejected (use `--pdf-engine
//! chrome`), except two recognised comment directives (`<!-- toc -->`,
//! `<!-- page-break -->`).

use std::collections::HashMap;
use std::path::Path;

use comrak::nodes::{AstNode, ListType, NodeValue};

use super::images;
use super::preamble::typ_str;

/// Sentinel emitted for the `<!-- toc -->` directive; consumed downstream.
const TOC_SENTINEL: &str = "%RECON_TOC%";

/// Shared, read-only context the walker needs for resolving image sources.
pub(crate) struct ImgCtx<'c> {
    /// Base directory for resolving relative local image paths.
    pub base_dir: &'c Path,
    /// HTTP client for fetching remote images.
    pub http: &'c reqwest::blocking::Client,
}

/// Escape text for typst markup context. (Minimal now; expanded later.)
pub fn escape_typst(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '#' | '$' | '*' | '_' | '`' | '<' | '@' | '\\' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Translate the document body (all top-level children of `root`) to typst.
///
/// `base_dir` resolves relative local image paths and `http` fetches remote
/// images; both are carried through every recursion via [`ImgCtx`].
pub fn body<'a>(
    root: &'a AstNode<'a>,
    _opts: &crate::docs::DocOptions,
    base_dir: &Path,
    http: &reqwest::blocking::Client,
) -> anyhow::Result<String> {
    let ctx = ImgCtx { base_dir, http };

    // Pre-pass: collect footnote definitions (name → translated body) so
    // references can be inlined as `#footnote[...]`. The definition blocks
    // themselves are not emitted.
    let mut footnotes: HashMap<String, String> = HashMap::new();
    collect_footnotes(root, &mut footnotes, &ctx)?;

    let mut out = String::new();
    for child in root.children() {
        walk(child, &mut out, 0, &footnotes, &ctx)?;
    }
    Ok(out.trim().to_string())
}

/// Recursively collect footnote definitions into `map` (name → translated body).
fn collect_footnotes<'a>(
    node: &'a AstNode<'a>,
    map: &mut HashMap<String, String>,
    ctx: &ImgCtx<'_>,
) -> anyhow::Result<()> {
    for child in node.children() {
        if let NodeValue::FootnoteDefinition(def) = &child.data.borrow().value {
            // Definitions may themselves reference footnotes; pass the
            // accumulated map (empty entries fall back gracefully).
            let snapshot = map.clone();
            let translated = inline_children(child, &snapshot, ctx)?;
            map.insert(def.name.clone(), translated);
        }
        collect_footnotes(child, map, ctx)?;
    }
    Ok(())
}

/// Render a node's children, then trim, producing inline-style content
/// (strips the trailing `\n\n` a `Paragraph` would otherwise append).
fn inline_children<'a>(
    node: &'a AstNode<'a>,
    footnotes: &HashMap<String, String>,
    ctx: &ImgCtx<'_>,
) -> anyhow::Result<String> {
    let mut buf = String::new();
    for c in node.children() {
        walk(c, &mut buf, 0, footnotes, ctx)?;
    }
    Ok(buf.trim().to_string())
}

fn walk<'a>(
    node: &'a AstNode<'a>,
    out: &mut String,
    depth: usize,
    footnotes: &HashMap<String, String>,
    ctx: &ImgCtx<'_>,
) -> anyhow::Result<()> {
    match &node.data.borrow().value {
        NodeValue::Document => {
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
        }
        NodeValue::Heading(h) => {
            out.push_str(&"=".repeat(h.level as usize));
            out.push(' ');
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
            out.push_str("\n\n");
        }
        NodeValue::Paragraph => {
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
            out.push_str("\n\n");
        }
        NodeValue::Text(t) => out.push_str(&escape_typst(t)),
        NodeValue::SoftBreak => out.push(' '),
        NodeValue::LineBreak => out.push_str(" \\\n"),

        // ---- Inline ----
        NodeValue::Strong => {
            out.push('*');
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
            out.push('*');
        }
        NodeValue::Emph => {
            out.push('_');
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
            out.push('_');
        }
        NodeValue::Strikethrough => {
            out.push_str("#strike[");
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
            out.push(']');
        }
        NodeValue::Code(code) => {
            // Inline raw via #raw(...) — verbatim content carried as a typst
            // string literal, so backticks/quotes/backslashes can't break out.
            out.push_str("#raw(");
            out.push_str(&typ_str(&code.literal));
            out.push(')');
        }
        NodeValue::Link(link) => {
            // Escape the URL via typ_str so a `"` or trailing `\` can't break
            // the string literal. (typ_str returns the value WITH quotes.)
            out.push_str("#link(");
            out.push_str(&typ_str(&link.url));
            out.push_str(")[");
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
            out.push(']');
        }
        NodeValue::Image(link) => {
            emit_image(&link.url, node, out, footnotes, ctx)?;
        }
        NodeValue::FootnoteReference(fref) => {
            let def = footnotes.get(&fref.name).cloned().unwrap_or_default();
            out.push_str("#footnote[");
            out.push_str(&def);
            out.push(']');
        }

        // ---- Blocks ----
        NodeValue::BlockQuote => {
            let inner = inline_children(node, footnotes, ctx)?;
            out.push_str("#quote(block: true)[");
            out.push_str(&inner);
            out.push_str("]\n\n");
        }
        NodeValue::ThematicBreak => {
            out.push_str("#line(length: 100%)\n\n");
        }
        NodeValue::CodeBlock(cb) => {
            // Block raw via #raw(block: true, ...) — verbatim content carried
            // as a typst string literal, so a literal fence line can't break
            // out and the language token is always safe.
            let lang = cb.info.split_whitespace().next().unwrap_or("");
            let literal = cb.literal.trim_end_matches('\n');
            out.push_str("#raw(block: true, ");
            if !lang.is_empty() {
                out.push_str("lang: ");
                out.push_str(&typ_str(lang));
                out.push_str(", ");
            }
            out.push_str(&typ_str(literal));
            out.push_str(")\n\n");
        }

        // ---- Lists ----
        NodeValue::List(list) => {
            let marker = match list.list_type {
                ListType::Bullet => '-',
                ListType::Ordered => '+',
            };
            for item in node.children() {
                let task_box = if let NodeValue::TaskItem(state) = &item.data.borrow().value {
                    Some(if state.is_some() {
                        "#box[☒] "
                    } else {
                        "#box[☐] "
                    })
                } else {
                    None
                };

                // Indent: 2 spaces per nesting level.
                out.push_str(&"  ".repeat(depth));
                out.push(marker);
                out.push(' ');
                if let Some(b) = task_box {
                    out.push_str(b);
                }

                // Item children: a tight item is a Paragraph wrapping inlines,
                // possibly followed by a nested List. Emit the paragraph
                // inline, recurse into nested lists at depth+1.
                emit_item(item, out, depth, footnotes, ctx)?;
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            out.push('\n');
        }

        // ---- Tables ----
        NodeValue::Table(_) => {
            emit_table(node, out, footnotes, ctx)?;
            out.push_str("\n\n");
        }

        // ---- Raw HTML + directives ----
        NodeValue::HtmlBlock(b) => {
            emit_html(&b.literal, out)?;
        }
        NodeValue::HtmlInline(s) => {
            emit_html(s, out)?;
        }

        // Footnote definitions are collected by the pre-pass and emitted
        // inline at their reference; the definition block itself produces
        // no output.
        NodeValue::FootnoteDefinition(_) => {}

        _ => {
            // Descend into anything not explicitly handled.
            for c in node.children() {
                walk(c, out, depth, footnotes, ctx)?;
            }
        }
    }
    Ok(())
}

/// Emit a list item's content: its paragraph(s) inline, plus nested lists.
fn emit_item<'a>(
    item: &'a AstNode<'a>,
    out: &mut String,
    depth: usize,
    footnotes: &HashMap<String, String>,
    ctx: &ImgCtx<'_>,
) -> anyhow::Result<()> {
    for child in item.children() {
        match &child.data.borrow().value {
            NodeValue::Paragraph => {
                for c in child.children() {
                    walk(c, out, depth, footnotes, ctx)?;
                }
            }
            NodeValue::List(_) => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                walk(child, out, depth + 1, footnotes, ctx)?;
            }
            _ => {
                walk(child, out, depth, footnotes, ctx)?;
            }
        }
    }
    Ok(())
}

/// Emit a GFM table as `#table(columns: N, [cell], ...)`.
fn emit_table<'a>(
    node: &'a AstNode<'a>,
    out: &mut String,
    footnotes: &HashMap<String, String>,
    ctx: &ImgCtx<'_>,
) -> anyhow::Result<()> {
    // Column count: cells in the first row.
    let mut cols = 0;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in node.children() {
        if !matches!(row.data.borrow().value, NodeValue::TableRow(_)) {
            continue;
        }
        let mut cells = Vec::new();
        for cell in row.children() {
            if matches!(cell.data.borrow().value, NodeValue::TableCell) {
                cells.push(inline_children(cell, footnotes, ctx)?);
            }
        }
        if cols == 0 {
            cols = cells.len();
        }
        rows.push(cells);
    }

    out.push_str(&format!("#table(columns: {},\n", cols));
    for row in &rows {
        out.push_str("  ");
        let rendered: Vec<String> = row.iter().map(|c| format!("[{}]", c)).collect();
        out.push_str(&rendered.join(", "));
        out.push_str(",\n");
    }
    out.push(')');
    Ok(())
}

/// Emit a markdown image: resolve the source to bytes and embed them inline as
/// `#image(bytes((...)))`. The detached main source cannot resolve filesystem
/// paths, so inlining the raw bytes is the only embedding mechanism that
/// renders. On any resolve failure (missing file, network error, non-200) the
/// image degrades to its escaped alt text plus a stderr warning — a broken
/// image must never abort the whole PDF.
fn emit_image<'a>(
    url: &str,
    node: &'a AstNode<'a>,
    out: &mut String,
    footnotes: &HashMap<String, String>,
    ctx: &ImgCtx<'_>,
) -> anyhow::Result<()> {
    // Alt text is the image node's child text.
    let alt = inline_children(node, footnotes, ctx)?;
    match images::resolve(url, ctx.base_dir, ctx.http) {
        Ok((bytes, hint)) => {
            out.push_str("#image(bytes((");
            for (i, b) in bytes.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(itoa_u8(*b));
            }
            // A trailing comma keeps a single-element array a valid typst array.
            out.push_str("),)");
            if let Some(fmt) = hint {
                out.push_str(", format: \"");
                out.push_str(&fmt);
                out.push('"');
            }
            out.push(')');
        }
        Err(e) => {
            eprintln!("recon: warning: image '{url}' unavailable: {e}");
            out.push_str(&escape_typst(&alt));
        }
    }
    Ok(())
}

/// Format a `u8` as a decimal string slice without allocating per byte.
fn itoa_u8(b: u8) -> &'static str {
    // Precomputed "0".."255" table — avoids a String allocation per byte for
    // images that can run to tens of thousands of bytes.
    const TABLE: [&str; 256] = build_u8_table();
    TABLE[b as usize]
}

/// Build the 0..=255 decimal-string lookup table at compile time.
const fn build_u8_table() -> [&'static str; 256] {
    // `concat!`/format aren't const, so the table is written out explicitly via
    // a macro that stringifies each literal.
    macro_rules! s {
        ($($n:literal),* $(,)?) => { [ $( stringify!($n) ),* ] };
    }
    s!(
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70,
        71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93,
        94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
        113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130,
        131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
        149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166,
        167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184,
        185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202,
        203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220,
        221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238,
        239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255
    )
}

/// Handle raw HTML: reject real markup, honour the two known comment
/// directives.
fn emit_html(literal: &str, out: &mut String) -> anyhow::Result<()> {
    let trimmed = literal.trim();
    match trimmed {
        "<!-- toc -->" => {
            out.push_str(TOC_SENTINEL);
            out.push_str("\n\n");
            Ok(())
        }
        "<!-- page-break -->" => {
            out.push_str("#pagebreak()\n\n");
            Ok(())
        }
        _ => Err(anyhow::anyhow!(
            "raw HTML is not supported by the typst engine; use --pdf-engine chrome"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{parse_document, Arena, ExtensionOptions, Options};

    fn opts<'a>() -> Options<'a> {
        let mut ext = ExtensionOptions::default();
        ext.strikethrough = true;
        ext.tagfilter = true;
        ext.table = true;
        ext.autolink = true;
        ext.tasklist = true;
        ext.footnotes = true;
        Options {
            extension: ext,
            parse: Default::default(),
            render: Default::default(),
        }
    }

    fn t_res(md: &str) -> anyhow::Result<String> {
        let arena = Arena::new();
        let copts = opts();
        let root = parse_document(&arena, md, &copts);
        let doc_opts = crate::docs::DocOptions::default();
        let http = reqwest::blocking::Client::new();
        let base = std::path::Path::new(".");
        body(root, &doc_opts, base, &http).map(|s| s.trim().to_string())
    }

    fn t(md: &str) -> String {
        t_res(md).unwrap()
    }

    // ---- Inline ----
    #[test]
    fn strong_and_emph() {
        assert_eq!(t("**bold** and *it*"), "*bold* and _it_");
    }

    #[test]
    fn inline_code_verbatim() {
        // Inline code now uses #raw(...) so verbatim content (incl. `<branch>`,
        // quotes, backslashes) survives without breaking out of typst source.
        assert_eq!(t("`git log <branch>`"), "#raw(\"git log <branch>\")");
    }

    #[test]
    fn inline_code_with_backtick_preserved() {
        // A backtick inside inline code cannot be wrapped with single
        // backticks. The #raw(...) form keeps the literal verbatim.
        let out = t("``a`b``");
        assert_eq!(out, "#raw(\"a`b\")");
        // No bare single-backtick wrapping that would break the source.
        assert!(!out.starts_with('`'));
    }

    #[test]
    fn strikethrough() {
        assert_eq!(t("~~x~~"), "#strike[x]");
    }

    #[test]
    fn strikethrough_bracket_escaped() {
        // A `]` inside #strike[..] must be escaped or it closes the block.
        let out = t("~~a]b~~");
        assert_eq!(out, "#strike[a\\]b]");
        assert!(out.contains("\\]"));
    }

    #[test]
    fn link() {
        assert_eq!(t("[a](http://x)"), "#link(\"http://x\")[a]");
    }

    #[test]
    fn link_url_quote_escaped() {
        // A `"` inside the URL must be escaped via typ_str or it closes the
        // string literal early.
        let out = t("[x](http://e/\"q)");
        assert!(out.contains("\\\""), "url quote not escaped: {out}");
        // The escaped quote must appear inside the #link("...") string, before
        // the closing `)[`.
        assert!(out.starts_with("#link(\"http://e/\\\"q\")["), "got: {out}");
    }

    #[test]
    fn bare_text_escaped() {
        assert_eq!(t("a #b *c*"), "a \\#b _c_");
    }

    #[test]
    fn bare_bracket_escaped() {
        // A lone `]` in a top-level paragraph fails to compile unless escaped.
        let out = t("a ] b");
        assert!(out.contains("\\]"), "bracket not escaped: {out}");
        assert_eq!(out, "a \\] b");
    }

    #[test]
    fn bracket_open_escaped() {
        assert_eq!(t("a [ b"), "a \\[ b");
    }

    // ---- Blocks ----
    #[test]
    fn heading() {
        assert_eq!(t("## Two"), "== Two");
    }

    #[test]
    fn block_quote() {
        assert_eq!(t("> q"), "#quote(block: true)[q]");
    }

    #[test]
    fn thematic_break() {
        assert_eq!(t("---"), "#line(length: 100%)");
    }

    #[test]
    fn code_block() {
        // Block code now uses #raw(block: true, lang: ..., ...) so a literal
        // containing a fence cannot break out and the language token is safe.
        assert_eq!(
            t("```sh\ngit log <b>\n```"),
            "#raw(block: true, lang: \"sh\", \"git log <b>\")"
        );
    }

    #[test]
    fn code_block_no_lang() {
        // No info string → omit lang:.
        assert_eq!(
            t("```\nplain\n```"),
            "#raw(block: true, \"plain\")"
        );
    }

    #[test]
    fn code_block_with_fence_literal() {
        // A tilde-fenced block whose literal contains a ``` line must not break
        // out of the source — #raw() carries it verbatim.
        let md = "~~~\nbefore\n```\nafter\n~~~";
        let out = t(md);
        // typ_str escapes only \ and ", so real newlines stay literal inside
        // the typst string and the verbatim fence survives intact.
        assert_eq!(out, "#raw(block: true, \"before\n```\nafter\")");
        assert!(out.contains("```"));
    }

    // ---- Lists ----
    #[test]
    fn unordered_list() {
        assert_eq!(t("- a\n- b"), "- a\n- b");
    }

    #[test]
    fn ordered_list() {
        assert_eq!(t("1. a\n2. b"), "+ a\n+ b");
    }

    #[test]
    fn nested_list() {
        assert_eq!(t("- a\n  - b"), "- a\n  - b");
    }

    #[test]
    fn task_list() {
        assert_eq!(
            t("- [ ] todo\n- [x] done"),
            "- #box[☐] todo\n- #box[☒] done"
        );
    }

    // ---- Tables ----
    #[test]
    fn table() {
        assert_eq!(
            t("| A | B |\n|---|---|\n| 1 | 2 |"),
            "#table(columns: 2,\n  [A], [B],\n  [1], [2],\n)"
        );
    }

    #[test]
    fn table_cell_bracket_escaped() {
        // A `]` inside a table cell `[..]` closes the cell early unless escaped.
        let out = t("| A | B |\n|---|---|\n| x]y | 2 |");
        assert!(out.contains("[x\\]y]"), "table cell bracket not escaped: {out}");
    }

    // ---- Footnotes ----
    #[test]
    fn footnote() {
        assert_eq!(t("x[^1]\n\n[^1]: note"), "x#footnote[note]");
    }

    #[test]
    fn footnote_bracket_escaped() {
        // A `]` inside #footnote[..] must be escaped.
        let out = t("x[^1]\n\n[^1]: a]b");
        assert!(out.contains("#footnote[a\\]b]"), "footnote bracket not escaped: {out}");
    }

    #[test]
    fn quote_bracket_escaped() {
        // A `]` inside #quote(block: true)[..] must be escaped.
        let out = t("> a]b");
        assert!(out.contains("\\]"), "quote bracket not escaped: {out}");
        assert_eq!(out, "#quote(block: true)[a\\]b]");
    }

    // ---- Raw HTML + directives ----
    #[test]
    fn html_block_errors() {
        assert!(t_res("<div>x</div>").is_err());
    }

    #[test]
    fn html_inline_errors() {
        assert!(t_res("a <span>b</span>").is_err());
    }

    #[test]
    fn toc_directive_ok() {
        assert!(t_res("<!-- toc -->\n\n# H").is_ok());
    }

    #[test]
    fn page_break_directive() {
        assert!(t("a\n\n<!-- page-break -->\n\nb").contains("#pagebreak()"));
    }

    #[test]
    fn unknown_comment_errors() {
        assert!(t_res("<!-- random -->").is_err());
    }
}
