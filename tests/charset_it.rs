//! End-to-end tests for the 0.43.0 text-encoding flags: --output-charset,
//! --source-charset, --request-charset, and --iconv. Wiremock + the release
//! binary so we exercise the full pipeline.

/// `café` encoded as ISO-8859-1 (Latin-1).
const LATIN1_CAFE: &[u8] = &[0x63, 0x61, 0x66, 0xE9];
/// `café` encoded as UTF-8.
const UTF8_CAFE: &[u8] = &[0x63, 0x61, 0x66, 0xC3, 0xA9];

#[tokio::test(flavor = "multi_thread")]
async fn output_charset_transcodes_latin1_response_to_utf8() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain; charset=iso-8859-1")
                .set_body_bytes(LATIN1_CAFE),
        )
        .mount(&server)
        .await;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--to-utf8")
        .arg(server.uri())
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, UTF8_CAFE);
}

#[tokio::test(flavor = "multi_thread")]
async fn no_output_charset_keeps_raw_bytes() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain; charset=iso-8859-1")
                .set_body_bytes(LATIN1_CAFE),
        )
        .mount(&server)
        .await;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg(server.uri())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, LATIN1_CAFE);
}

#[tokio::test(flavor = "multi_thread")]
async fn source_charset_overrides_server_declaration() {
    // Server mis-declares the body as UTF-8 but the bytes are actually
    // Latin-1. --source-charset lets the user correct it.
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain; charset=utf-8")
                .set_body_bytes(LATIN1_CAFE),
        )
        .mount(&server)
        .await;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--source-charset")
        .arg("iso-8859-1")
        .arg("--output-charset")
        .arg("utf-8")
        .arg(server.uri())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, UTF8_CAFE);
}

#[tokio::test(flavor = "multi_thread")]
async fn request_body_transcodes_to_iso8859_1_via_content_type() {
    use std::sync::Arc;
    use std::sync::Mutex;

    // Capture the raw request body and assert it was Latin-1-encoded.
    let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
    let captured_for_handler = Arc::clone(&captured);

    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |req: &wiremock::Request| {
            captured_for_handler.lock().unwrap().extend_from_slice(&req.body);
            wiremock::ResponseTemplate::new(200)
        })
        .mount(&server)
        .await;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-X").arg("POST")
        .arg("-H").arg("Content-Type: text/plain; charset=iso-8859-1")
        .arg("-d").arg("café")
        .arg(server.uri())
        .output()
        .unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let captured_body = captured.lock().unwrap().clone();
    assert_eq!(captured_body, LATIN1_CAFE);
}

#[tokio::test(flavor = "multi_thread")]
async fn request_charset_passthrough_preserves_utf8_bytes() {
    use std::sync::{Arc, Mutex};
    let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
    let captured_for_handler = Arc::clone(&captured);

    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |req: &wiremock::Request| {
            captured_for_handler.lock().unwrap().extend_from_slice(&req.body);
            wiremock::ResponseTemplate::new(200)
        })
        .mount(&server)
        .await;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-X").arg("POST")
        .arg("-H").arg("Content-Type: text/plain; charset=iso-8859-1")
        .arg("--request-charset-passthrough")
        .arg("-d").arg("café")
        .arg(server.uri())
        .output()
        .unwrap();
    assert!(output.status.success());

    assert_eq!(captured.lock().unwrap().clone(), UTF8_CAFE);
}

#[test]
fn iconv_latin1_to_utf8_file_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.txt");
    let out_path = dir.path().join("out.txt");
    std::fs::write(&input, LATIN1_CAFE).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--iconv").arg("iso-8859-1:utf-8")
        .arg("-o").arg(&out_path)
        .arg(&input)
        .status()
        .unwrap();
    assert!(status.success());

    let written = std::fs::read(&out_path).unwrap();
    assert_eq!(written, UTF8_CAFE);
}

#[test]
fn iconv_auto_detect_utf8_bom_stdin() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let bom = b"\xEF\xBB\xBFhello";
    let mut child = Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--iconv").arg(":utf-8")
        .arg("--silent")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.as_mut().unwrap().write_all(bom).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    // Auto-detect kept the BOM intact during UTF-8 → UTF-8 transcoding.
    assert_eq!(output.stdout, bom);
}
