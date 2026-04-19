#[tokio::test(flavor = "multi_thread")]
async fn fail_with_body_writes_body_and_exits_nonzero() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(404).set_body_string("not found details"))
        .mount(&server)
        .await;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--fail-with-body")
        .arg("-o").arg(tmp.path())
        .arg(format!("{}/missing", server.uri()))
        .output()
        .unwrap();

    assert!(!output.status.success(), "should exit non-zero; stderr: {}", String::from_utf8_lossy(&output.stderr));
    let saved = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(saved, "not found details");
}

#[tokio::test(flavor = "multi_thread")]
async fn fail_without_body_suppresses_body_on_error() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-f")
        .arg("-o").arg(tmp.path())
        .arg(format!("{}/err", server.uri()))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let saved_len = std::fs::metadata(tmp.path()).unwrap().len();
    assert_eq!(saved_len, 0);
}
