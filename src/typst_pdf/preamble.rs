use anyhow::{bail, Result};

/// Map a --page-size value to the inner args of typst `set page(...)`.
/// Named papers map to typst paper ids; `WxH` (with units) -> width/height.
#[allow(dead_code)]
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

#[allow(dead_code)]
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
}
