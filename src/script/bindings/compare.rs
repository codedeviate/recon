//! `compare(a, b)` script binding.
//!
//! Diffs two byte buffers (or strings) in-memory and returns a summary
//! map. Mirrors the `--compare` CLI feature but operates on content the
//! script already has in hand — no file or HTTP fetch is implied.

use rhai::{Blob, Engine, Map};
use similar::{ChangeTag, TextDiff};

pub fn register(engine: &mut Engine) {
    engine.register_fn("compare", |a: Blob, b: Blob| -> Map { diff_bytes(&a, &b) });

    engine.register_fn("compare", |a: &str, b: &str| -> Map {
        diff_bytes(a.as_bytes(), b.as_bytes())
    });
}

fn diff_bytes(a: &[u8], b: &[u8]) -> Map {
    let mut out = Map::new();
    out.insert("a_bytes".into(), (a.len() as i64).into());
    out.insert("b_bytes".into(), (b.len() as i64).into());

    if a == b {
        out.insert("identical".into(), true.into());
        out.insert("added".into(), 0i64.into());
        out.insert("removed".into(), 0i64.into());
        out.insert("diff".into(), "".into());
        out.insert("binary".into(), crate::compare::is_binary(a).into());
        return out;
    }

    let binary = crate::compare::is_binary(a) || crate::compare::is_binary(b);
    out.insert("identical".into(), false.into());
    out.insert("binary".into(), binary.into());

    if binary {
        out.insert("added".into(), 0i64.into());
        out.insert("removed".into(), 0i64.into());
        out.insert(
            "diff".into(),
            format!("binary delta: {} vs {} bytes", a.len(), b.len()).into(),
        );
        return out;
    }

    let ta = String::from_utf8_lossy(a).into_owned();
    let tb = String::from_utf8_lossy(b).into_owned();
    let diff = TextDiff::from_lines(ta.as_str(), tb.as_str());
    let mut added = 0i64;
    let mut removed = 0i64;
    for c in diff.iter_all_changes() {
        match c.tag() {
            ChangeTag::Insert => added += 1,
            ChangeTag::Delete => removed += 1,
            ChangeTag::Equal => {}
        }
    }
    let unified = diff
        .unified_diff()
        .context_radius(3)
        .header("a", "b")
        .to_string();

    out.insert("added".into(), added.into());
    out.insert("removed".into(), removed.into());
    out.insert("diff".into(), unified.into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_blobs_report_identical() {
        let mut e = Engine::new();
        register(&mut e);
        let m: Map = e
            .eval(r#"compare("same\n", "same\n")"#)
            .expect("eval");
        assert_eq!(m.get("identical").unwrap().as_bool().unwrap(), true);
        assert_eq!(m.get("added").unwrap().as_int().unwrap(), 0);
    }

    #[test]
    fn differing_strings_count_changes() {
        let mut e = Engine::new();
        register(&mut e);
        let m: Map = e
            .eval(r#"compare("one\ntwo\n", "one\nthree\n")"#)
            .expect("eval");
        assert_eq!(m.get("identical").unwrap().as_bool().unwrap(), false);
        assert_eq!(m.get("added").unwrap().as_int().unwrap(), 1);
        assert_eq!(m.get("removed").unwrap().as_int().unwrap(), 1);
        assert!(m.get("diff").unwrap().clone().into_string().unwrap().contains("three"));
    }

    #[test]
    fn binary_sources_flag_binary() {
        let mut e = Engine::new();
        register(&mut e);
        let script = r#"
let a = blob();
a += 0x68; a += 0x00; a += 0x69;
let b = blob();
b += 0x42;
compare(a, b)
"#;
        let m: Map = e.eval(script).expect("eval");
        assert_eq!(m.get("binary").unwrap().as_bool().unwrap(), true);
        assert_eq!(m.get("identical").unwrap().as_bool().unwrap(), false);
    }
}
