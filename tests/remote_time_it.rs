#[tokio::test(flavor = "multi_thread")]
async fn remote_time_applies_last_modified() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("Last-Modified", "Sun, 06 Nov 1994 08:49:37 GMT")
                .set_body_string("old content"),
        )
        .mount(&server)
        .await;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--remote-time")
        .arg("-o").arg(tmp.path())
        .arg(server.uri())
        .status()
        .unwrap();
    assert!(status.success());

    let meta = std::fs::metadata(tmp.path()).unwrap();
    let mtime = meta.modified().unwrap();
    let secs = mtime.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    // Sun, 06 Nov 1994 08:49:37 GMT = 784111777
    assert_eq!(secs, 784111777);
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_time_silently_ignores_missing_header() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("no lm"))
        .mount(&server)
        .await;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--remote-time")
        .arg("-o").arg(tmp.path())
        .arg(server.uri())
        .status()
        .unwrap();
    assert!(status.success());
}
