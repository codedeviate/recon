//! Integration coverage for `--compare` that exercises the CLI binary.

use std::io::Write;
use std::process::{Command, Stdio};

fn recon_bin() -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Tests can run from either debug or release; pick whichever exists.
    let dbg = p.join("debug").join("recon");
    let rel = p.join("release").join("recon");
    if dbg.exists() {
        dbg.to_string_lossy().into_owned()
    } else {
        rel.to_string_lossy().into_owned()
    }
}

fn write_tmp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("recon-compare-it-{}-{}", std::process::id(), name));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(bytes).unwrap();
    path
}

#[test]
fn identical_files_exit_zero() {
    let a = write_tmp("same-a.txt", b"hello\nworld\n");
    let b = write_tmp("same-b.txt", b"hello\nworld\n");
    let out = Command::new(recon_bin())
        .args([
            "--compare",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--compare-format",
            "summary",
        ])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("identical"), "stdout: {stdout}");
    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn differing_files_exit_one_unified() {
    let a = write_tmp("diff-a.txt", b"hello\nworld\n");
    let b = write_tmp("diff-b.txt", b"hello\nearth\n");
    let out = Command::new(recon_bin())
        .args([
            "--compare",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("-world"), "stdout: {stdout}");
    assert!(stdout.contains("+earth"), "stdout: {stdout}");
    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn binary_reports_byte_count_delta() {
    let a = write_tmp("bin-a.bin", b"pre\0post-one");
    let b = write_tmp("bin-b.bin", b"pre\0post-two-longer");
    let out = Command::new(recon_bin())
        .args([
            "--compare",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--compare-format",
            "summary",
        ])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("binary"), "stdout: {stdout}");
    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn stdin_as_one_side() {
    let b = write_tmp("stdin-b.txt", b"one\ntwo\n");
    let mut child = Command::new(recon_bin())
        .args([
            "--compare",
            "-",
            b.to_str().unwrap(),
            "--compare-format",
            "summary",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    child.stdin.as_mut().unwrap().write_all(b"one\ntwo\n").unwrap();
    let out = child.wait_with_output().expect("wait");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("identical"), "stdout: {stdout}");
    let _ = std::fs::remove_file(&b);
}
