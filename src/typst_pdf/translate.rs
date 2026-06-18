//! Minimal comrak AST → typst markup translation.
//!
//! Stage 1 scope: headings, paragraphs, text, soft/line breaks. Full GFM
//! (emphasis, lists, tables, code, links, images, …) arrives in later tasks;
//! unknown nodes currently descend and emit their child text.

use comrak::nodes::{AstNode, NodeValue};

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
    let mut out = String::new();
    for child in root.children() {
        walk(child, &mut out)?;
    }
    Ok(out.trim().to_string())
}

fn walk<'a>(node: &'a AstNode<'a>, out: &mut String) -> anyhow::Result<()> {
    match &node.data.borrow().value {
        NodeValue::Heading(h) => {
            out.push_str(&"=".repeat(h.level as usize));
            out.push(' ');
            for c in node.children() {
                walk(c, out)?;
            }
            out.push_str("\n\n");
        }
        NodeValue::Paragraph => {
            for c in node.children() {
                walk(c, out)?;
            }
            out.push_str("\n\n");
        }
        NodeValue::Text(t) => out.push_str(&escape_typst(t)),
        NodeValue::SoftBreak => out.push(' '),
        NodeValue::LineBreak => out.push_str(" \\\n"),
        _ => {
            // Stage 1: descend into anything else, emitting child text.
            for c in node.children() {
                walk(c, out)?;
            }
        }
    }
    Ok(())
}
