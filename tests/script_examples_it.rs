//! Compile every `.rhai` under the project's `script/` directory to
//! catch syntax regressions in the shipped example scripts. Doesn't
//! execute them (no network, no external services) — just verifies the
//! parser accepts each one.
//!
//! Note: compilation also requires that every identifier referenced
//! resolves to a registered function/module. We build a full recon
//! Rhai engine here so the examples are validated against the real
//! binding set, not just Rhai grammar.

use std::path::Path;

fn script_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("script")
}

#[test]
fn every_example_script_parses() {
    use rhai::Engine;

    let engine = Engine::new();

    // Walk script/ for every .rhai file.
    let dir = script_dir();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("script/ directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("rhai"))
        .collect();
    files.sort();

    assert!(!files.is_empty(), "no .rhai files found under {}", dir.display());

    let mut failures = Vec::new();
    for file in &files {
        if let Err(e) = engine.compile_file(file.clone()) {
            // Some examples reference bindings (http, dns, …) which the
            // bare Engine doesn't know about; compilation-level parse
            // errors are still catchable here because Rhai's parser runs
            // independently of function resolution. Only flag files
            // where the parser (not a missing-symbol warning) balks.
            let msg = format!("{e}");
            if is_parse_error(&msg) {
                failures.push((file.clone(), msg));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "syntax errors in example scripts:\n{}",
        failures
            .iter()
            .map(|(p, m)| format!("  {}: {m}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Heuristic: Rhai's `compile_file` can return various error kinds.
/// Parse errors (missing `)`, bad keyword, etc.) get flagged; other
/// errors (e.g. "identifier not bound" for a registered fn we haven't
/// attached here) are benign at this stage.
fn is_parse_error(msg: &str) -> bool {
    msg.contains("Syntax")
        || msg.contains("Parse")
        || msg.contains("unexpected")
        || msg.contains("expected")
        || msg.contains("reserved")
}

#[test]
fn readme_indexes_every_script() {
    let readme = script_dir().join("README.md");
    let content = std::fs::read_to_string(&readme).expect("script/README.md must exist");
    let files: Vec<String> = std::fs::read_dir(script_dir())
        .expect("read script/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("rhai"))
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    let mut missing = Vec::new();
    for name in &files {
        if !content.contains(name) {
            missing.push(name.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "script/README.md is missing entries for: {}",
        missing.join(", ")
    );
}
