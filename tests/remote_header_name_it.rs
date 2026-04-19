#[tokio::test(flavor = "multi_thread")]
async fn remote_header_name_uses_content_disposition() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("Content-Disposition", r#"attachment; filename="from_header.txt""#)
                .set_body_string("body"),
        )
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-O")
        .arg("--remote-header-name")
        .arg("--output-dir").arg(tmp.path())
        .arg(format!("{}/ignored-url-basename", server.uri()))
        .status()
        .unwrap();

    assert!(status.success());
    assert!(tmp.path().join("from_header.txt").exists());
    assert!(!tmp.path().join("ignored-url-basename").exists());
    assert_eq!(std::fs::read_to_string(tmp.path().join("from_header.txt")).unwrap(), "body");
}
