//! Open response output in an editor of the user's choice.

/// Returns a filename extension (without the leading dot) for a response
/// `Content-Type` header value. Matches are case-insensitive and apply to the
/// type/subtype portion before any `;` parameters. Unknown types fall back to
/// `"txt"`.
pub fn extension_for_content_type(content_type: &str) -> &'static str {
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    match ct.as_str() {
        "application/json" => "json",
        "text/html" => "html",
        "application/xml" | "text/xml" => "xml",
        "text/yaml" | "application/yaml" | "application/x-yaml" => "yaml",
        "text/csv" => "csv",
        "text/tab-separated-values" => "tsv",
        "text/markdown" => "md",
        "application/javascript" | "text/javascript" => "js",
        "text/css" => "css",
        _ => {
            // Handle structured-syntax suffixes like application/ld+json.
            if ct.ends_with("+json") {
                "json"
            } else if ct.ends_with("+xml") {
                "xml"
            } else if ct.ends_with("+yaml") {
                "yaml"
            } else {
                "txt"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_common_types() {
        assert_eq!(extension_for_content_type("application/json"), "json");
        assert_eq!(extension_for_content_type("text/html"), "html");
        assert_eq!(extension_for_content_type("application/xml"), "xml");
        assert_eq!(extension_for_content_type("text/xml"), "xml");
        assert_eq!(extension_for_content_type("text/yaml"), "yaml");
        assert_eq!(extension_for_content_type("application/yaml"), "yaml");
        assert_eq!(extension_for_content_type("text/csv"), "csv");
        assert_eq!(extension_for_content_type("text/tab-separated-values"), "tsv");
        assert_eq!(extension_for_content_type("text/markdown"), "md");
        assert_eq!(extension_for_content_type("application/javascript"), "js");
        assert_eq!(extension_for_content_type("text/javascript"), "js");
        assert_eq!(extension_for_content_type("text/css"), "css");
    }

    #[test]
    fn extension_strips_parameters() {
        assert_eq!(
            extension_for_content_type("application/json; charset=utf-8"),
            "json",
        );
        assert_eq!(
            extension_for_content_type("text/html;charset=UTF-8"),
            "html",
        );
    }

    #[test]
    fn extension_case_insensitive() {
        assert_eq!(extension_for_content_type("Application/JSON"), "json");
        assert_eq!(extension_for_content_type("TEXT/HTML"), "html");
    }

    #[test]
    fn extension_structured_syntax_suffix() {
        assert_eq!(
            extension_for_content_type("application/ld+json"),
            "json",
        );
        assert_eq!(extension_for_content_type("application/soap+xml"), "xml");
        assert_eq!(extension_for_content_type("application/foo+yaml"), "yaml");
    }

    #[test]
    fn extension_unknown_falls_back_to_txt() {
        assert_eq!(extension_for_content_type(""), "txt");
        assert_eq!(extension_for_content_type("application/octet-stream"), "txt");
        assert_eq!(extension_for_content_type("image/png"), "txt");
    }
}
