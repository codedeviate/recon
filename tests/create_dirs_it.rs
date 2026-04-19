#[tokio::test(flavor = "multi_thread")]
async fn create_dirs_creates_missing_parents() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("hello"))
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("a/b/c/file.txt");
    assert!(!nested.parent().unwrap().exists());

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--create-dirs")
        .arg("-o").arg(&nested)
        .arg(server.uri())
        .status()
        .unwrap();

    assert!(status.success());
    assert!(nested.exists());
    assert_eq!(std::fs::read_to_string(&nested).unwrap(), "hello");
}

#[tokio::test(flavor = "multi_thread")]
async fn output_dir_prefixes_output_path() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("world"))
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--output-dir").arg(tmp.path())
        .arg("-o").arg("inside.txt")
        .arg(server.uri())
        .status()
        .unwrap();

    assert!(status.success());
    let expected = tmp.path().join("inside.txt");
    assert_eq!(std::fs::read_to_string(&expected).unwrap(), "world");
}
