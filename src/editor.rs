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

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Monotonic counter bumped by every call to `temp_path_for` so that rapid
/// successive calls within the same millisecond still produce unique paths.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Build a fresh temp file path of the form `/tmp/recon-<unix-ms><counter>.<ext>`.
/// Pure path construction — does not touch the filesystem.
pub fn temp_path_for(extension: &str) -> PathBuf {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    // Merge the counter as extra low-order digits so chronological sort stays stable.
    let stem = now_ms.saturating_mul(1_000).saturating_add(seq % 1_000);
    PathBuf::from(format!("/tmp/recon-{stem}.{extension}"))
}

use std::fs::OpenOptions;
use std::io::Write;

/// Create a fresh temp file under `/tmp/recon-*` with the given extension and
/// payload, with 0600 permissions on Unix. Returns the path written.
pub fn create_temp_file(extension: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let path = temp_path_for(extension);
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(&path)?;
    file.write_all(bytes)?;
    file.flush()?;
    Ok(path)
}

use std::process::{Command, Stdio};

/// Launch the resolved editor on `path`, fire-and-forget.
/// Returns an error if the process fails to spawn.
pub fn spawn_editor(resolved: &ResolvedEditor, path: &std::path::Path) -> std::io::Result<()> {
    match resolved {
        ResolvedEditor::Argv { program } => {
            Command::new(program)
                .arg(path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
        }
        ResolvedEditor::Shell { command } => {
            let full = format!("{} {}", command, shell_quote(path));
            Command::new("sh")
                .arg("-c")
                .arg(&full)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
        }
    }
    Ok(())
}

/// Load `[editor]` defaults and aliases from `~/.recon/config.toml`.
///
/// Returns `(default, aliases)`. A missing or malformed config is not fatal
/// for `--editor`: the flag can still resolve built-in aliases and raw
/// commands without it.
pub fn load_editor_config() -> (Option<String>, HashMap<String, String>) {
    match crate::config::load() {
        Ok(cfg) => match cfg.editor {
            Some(e) => (e.default, e.aliases),
            None => (None, HashMap::new()),
        },
        Err(_) => (None, HashMap::new()),
    }
}

/// Single-quote a path for safe inclusion in a POSIX `sh -c` string. Embedded
/// single quotes are escaped by breaking and re-opening the quoted run.
fn shell_quote(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    let escaped = s.replace('\'', r"'\''");
    format!("'{escaped}'")
}

/// Delete every file in `/tmp` whose filename starts with `recon-`. Returns
/// the count of files removed. Unlink errors for individual files are printed
/// to stderr and do not abort the sweep.
pub fn cleanup_temp_files() -> std::io::Result<usize> {
    let mut removed = 0usize;
    for entry in std::fs::read_dir("/tmp")? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("recon-") {
            continue;
        }
        let path = entry.path();
        // Only touch plain files (not directories, symlinks, etc. we created).
        match entry.file_type() {
            Ok(ft) if ft.is_file() => {}
            _ => continue,
        }
        match std::fs::remove_file(&path) {
            Ok(_) => removed += 1,
            Err(e) => eprintln!("warning: failed to remove {}: {e}", path.display()),
        }
    }
    Ok(removed)
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

    #[test]
    fn temp_path_format() {
        let p = temp_path_for("json");
        let s = p.to_string_lossy();
        assert!(s.starts_with("/tmp/recon-"), "got {s}");
        assert!(s.ends_with(".json"), "got {s}");
        // Expect /tmp/recon-<digits>.json
        let stem = s
            .strip_prefix("/tmp/recon-")
            .and_then(|t| t.strip_suffix(".json"))
            .expect("unexpected format");
        assert!(stem.chars().all(|c| c.is_ascii_digit()), "stem was {stem}");
    }

    #[test]
    fn temp_paths_are_unique_across_rapid_calls() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for _ in 0..50 {
            let p = temp_path_for("txt");
            assert!(seen.insert(p), "duplicate path generated");
        }
    }

    #[test]
    fn create_temp_file_writes_contents() {
        let p = create_temp_file("txt", b"hello world").expect("write");
        let got = std::fs::read_to_string(&p).expect("read");
        assert_eq!(got, "hello world");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn shell_quote_plain_path() {
        let p = std::path::PathBuf::from("/tmp/recon-123.json");
        assert_eq!(shell_quote(&p), "'/tmp/recon-123.json'");
    }

    #[test]
    fn shell_quote_embedded_single_quote() {
        let p = std::path::PathBuf::from("/tmp/foo'bar.txt");
        assert_eq!(shell_quote(&p), "'/tmp/foo'\\''bar.txt'");
    }

    #[test]
    fn cleanup_removes_only_recon_temp_files() {
        // Create a recon temp file and an unrelated /tmp file.
        let ours = create_temp_file("txt", b"x").unwrap();
        let theirs = std::path::PathBuf::from(format!(
            "/tmp/not-recon-{}.txt",
            std::process::id()
        ));
        std::fs::write(&theirs, b"y").unwrap();

        let n = cleanup_temp_files().unwrap();
        assert!(n >= 1, "expected to remove at least our file");
        assert!(!ours.exists(), "our file should be deleted");
        assert!(theirs.exists(), "unrelated file should remain");

        let _ = std::fs::remove_file(&theirs);
    }
}
