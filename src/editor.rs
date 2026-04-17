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

use std::collections::HashMap;

/// Built-in editor aliases: alias → argv[0] command name.
const BUILTIN_ALIASES: &[(&str, &str)] = &[
    ("zed", "zed"),
    ("code", "code"),
    ("cursor", "cursor"),
    ("subl", "subl"),
    ("vim", "vim"),
    ("nvim", "nvim"),
    ("nano", "nano"),
    ("emacs", "emacs"),
];

/// The resolved form of an `--editor` argument. Determines how the editor is
/// spawned: `Argv` uses direct exec (argv[0] + tempfile), `Shell` uses
/// `sh -c "<cmd> <tempfile>"` so user-supplied flags work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedEditor {
    /// Built-in alias; spawn with argv `[program, tempfile]`.
    Argv { program: String },
    /// User alias or raw command; spawn with `sh -c "<cmd> <quoted-tempfile>"`.
    Shell { command: String },
}

/// Error returned by `resolve_editor` when no value was given and no config
/// default is set.
#[derive(Debug, PartialEq, Eq)]
pub struct NoEditorDefault;

/// Resolve the `--editor` argument to a spawn recipe.
///
/// `flag_value` is:
/// - `Some("")` if `--editor` was provided with no value (use config default);
/// - `Some(non-empty)` otherwise.
///
/// Resolution order when a value is present:
///   1. User alias from `[editor.aliases]` (overrides built-ins).
///   2. Built-in alias.
///   3. Raw command (shell-interpreted).
pub fn resolve_editor(
    flag_value: &str,
    config_default: Option<&str>,
    user_aliases: &HashMap<String, String>,
) -> Result<ResolvedEditor, NoEditorDefault> {
    let effective: &str = if flag_value.is_empty() {
        config_default.ok_or(NoEditorDefault)?
    } else {
        flag_value
    };

    if let Some(cmd) = user_aliases.get(effective) {
        return Ok(ResolvedEditor::Shell {
            command: cmd.clone(),
        });
    }
    if let Some((_, program)) = BUILTIN_ALIASES.iter().find(|(k, _)| *k == effective) {
        return Ok(ResolvedEditor::Argv {
            program: (*program).to_string(),
        });
    }
    Ok(ResolvedEditor::Shell {
        command: effective.to_string(),
    })
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

    #[test]
    fn resolve_builtin_alias() {
        let aliases = HashMap::new();
        let got = resolve_editor("zed", None, &aliases).unwrap();
        assert_eq!(got, ResolvedEditor::Argv { program: "zed".into() });
    }

    #[test]
    fn resolve_user_alias_overrides_builtin() {
        let mut aliases = HashMap::new();
        aliases.insert("zed".to_string(), "zed --dev".to_string());
        let got = resolve_editor("zed", None, &aliases).unwrap();
        assert_eq!(got, ResolvedEditor::Shell { command: "zed --dev".into() });
    }

    #[test]
    fn resolve_raw_command() {
        let aliases = HashMap::new();
        let got = resolve_editor("code --wait", None, &aliases).unwrap();
        assert_eq!(got, ResolvedEditor::Shell { command: "code --wait".into() });
    }

    #[test]
    fn resolve_empty_uses_config_default_alias() {
        let aliases = HashMap::new();
        let got = resolve_editor("", Some("zed"), &aliases).unwrap();
        assert_eq!(got, ResolvedEditor::Argv { program: "zed".into() });
    }

    #[test]
    fn resolve_empty_uses_config_default_raw() {
        let aliases = HashMap::new();
        let got = resolve_editor("", Some("code --wait"), &aliases).unwrap();
        assert_eq!(got, ResolvedEditor::Shell { command: "code --wait".into() });
    }

    #[test]
    fn resolve_empty_without_default_errors() {
        let aliases = HashMap::new();
        let err = resolve_editor("", None, &aliases).unwrap_err();
        assert_eq!(err, NoEditorDefault);
    }
}
