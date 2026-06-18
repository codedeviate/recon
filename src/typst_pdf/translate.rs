//! comrak AST → typst markup translation.
//!
//! Covers GFM: headings, paragraphs, emphasis, inline/block code, links,
//! strikethrough, block quotes, thematic breaks, ordered/unordered/nested/task
//! lists, tables, and footnotes. Raw HTML is rejected (use `--pdf-engine
//! chrome`), except two recognised comment directives (`<!-- toc -->`,
//! `<!-- page-break -->`).

use std::collections::HashMap;

use comrak::nodes::{AstNode, ListType, NodeValue};

/// Sentinel emitted for the `<!-- toc -->` directive; consumed downstream.
const TOC_SENTINEL: &str = "%RECON_TOC%";

/// Escape text for typst markup context. (Minimal now; expanded later.)
pub fn escape_typst(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '#' | '$' | '*' | '_' | '`' | '<' | '@' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Translate the document body (all top-level children of `root`) to typst.
pub fn body<'a>(root: &'a AstNode<'a>, _opts: &crate::docs::DocOptions) -> anyhow::Result<String> {
    // Pre-pass: collect footnote definitions (name → translated body) so
    // references can be inlined as `#footnote[...]`. The definition blocks
    // themselves are not emitted.
    let mut footnotes: HashMap<String, String> = HashMap::new();
    collect_footnotes(root, &mut footnotes)?;

    let mut out = String::new();
    for child in root.children() {
        walk(child, &mut out, 0, &footnotes)?;
    }
    Ok(out.trim().to_string())
}

/// Recursively collect footnote definitions into `map` (name → translated body).
fn collect_footnotes<'a>(
    node: &'a AstNode<'a>,
    map: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    for child in node.children() {
        if let NodeValue::FootnoteDefinition(def) = &child.data.borrow().value {
            // Definitions may themselves reference footnotes; pass the
            // accumulated map (empty entries fall back gracefully).
            let snapshot = map.clone();
            let translated = inline_children(child, &snapshot)?;
            map.insert(def.name.clone(), translated);
        }
        collect_footnotes(child, map)?;
    }
    Ok(())
}

/// Render a node's children, then trim, producing inline-style content
/// (strips the trailing `\n\n` a `Paragraph` would otherwise append).
fn inline_children<'a>(
    node: &'a AstNode<'a>,
    footnotes: &HashMap<String, String>,
) -> anyhow::Result<String> {
    let mut buf = String::new();
    for c in node.children() {
        walk(c, &mut buf, 0, footnotes)?;
    }
    Ok(buf.trim().to_string())
}

fn walk<'a>(
    node: &'a AstNode<'a>,
    out: &mut String,
    depth: usize,
    footnotes: &HashMap<String, String>,
) -> anyhow::Result<()> {
    match &node.data.borrow().value {
        NodeValue::Document => {
            for c in node.children() {
                walk(c, out, depth, footnotes)?;
            }
        }
        NodeValue::Heading(h) => {
            out.push_str(&"=".repeat(h.level as usize));
            out.push(' ');
            for c in node.children() {
                walk(c, out, depth, footnotes)?;
            }
            out.push_str("\n\n");
        }
        NodeValue::Paragraph => {
            for c in node.children() {
                walk(c, out, depth, footnotes)?;
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
                walk(c, out, depth, footnotes)?;
            }
            out.push('*');
        }
        NodeValue::Emph => {
            out.push('_');
            for c in node.children() {
                walk(c, out, depth, footnotes)?;
            }
            out.push('_');
        }
        NodeValue::Strikethrough => {
            out.push_str("#strike[");
            for c in node.children() {
                walk(c, out, depth, footnotes)?;
            }
            out.push(']');
        }
        NodeValue::Code(code) => {
            // Inline raw — content is verbatim, never escaped.
            out.push('`');
            out.push_str(&code.literal);
            out.push('`');
        }
        NodeValue::Link(link) => {
            out.push_str("#link(\"");
            out.push_str(&link.url);
            out.push_str("\")[");
            for c in node.children() {
                walk(c, out, depth, footnotes)?;
            }
            out.push(']');
        }
        NodeValue::FootnoteReference(fref) => {
            let def = footnotes.get(&fref.name).cloned().unwrap_or_default();
            out.push_str("#footnote[");
            out.push_str(&def);
            out.push(']');
        }

        // ---- Blocks ----
        NodeValue::BlockQuote => {
            let inner = inline_children(node, footnotes)?;
            out.push_str("#quote(block: true)[");
            out.push_str(&inner);
            out.push_str("]\n\n");
        }
        NodeValue::ThematicBreak => {
            out.push_str("#line(length: 100%)\n\n");
        }
        NodeValue::CodeBlock(cb) => {
            // Block raw — verbatim content with optional language token.
            let lang = cb.info.split_whitespace().next().unwrap_or("");
            out.push_str("```");
            out.push_str(lang);
            out.push('\n');
            out.push_str(cb.literal.trim_end_matches('\n'));
            out.push_str("\n```\n\n");
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
                emit_item(item, out, depth, footnotes)?;
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            out.push('\n');
        }

        // ---- Tables ----
        NodeValue::Table(_) => {
            emit_table(node, out, footnotes)?;
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
                walk(c, out, depth, footnotes)?;
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
) -> anyhow::Result<()> {
    for child in item.children() {
        match &child.data.borrow().value {
            NodeValue::Paragraph => {
                for c in child.children() {
                    walk(c, out, depth, footnotes)?;
                }
            }
            NodeValue::List(_) => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                walk(child, out, depth + 1, footnotes)?;
            }
            _ => {
                walk(child, out, depth, footnotes)?;
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
                cells.push(inline_children(cell, footnotes)?);
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
        body(root, &doc_opts).map(|s| s.trim().to_string())
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
        assert_eq!(t("`git log <branch>`"), "`git log <branch>`");
    }

    #[test]
    fn strikethrough() {
        assert_eq!(t("~~x~~"), "#strike[x]");
    }

    #[test]
    fn link() {
        assert_eq!(t("[a](http://x)"), "#link(\"http://x\")[a]");
    }

    #[test]
    fn bare_text_escaped() {
        assert_eq!(t("a #b *c*"), "a \\#b _c_");
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
        assert_eq!(t("```sh\ngit log <b>\n```"), "```sh\ngit log <b>\n```");
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

    // ---- Footnotes ----
    #[test]
    fn footnote() {
        assert_eq!(t("x[^1]\n\n[^1]: note"), "x#footnote[note]");
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
