use std::io::Write as _;

#[tokio::test(flavor = "multi_thread")]
async fn compressed_gzip_roundtrip() {
    let server = wiremock::MockServer::start().await;

    let body = b"hello from mock";
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(body).unwrap();
    let compressed = gz.finish().unwrap();

    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("Content-Encoding", "gzip")
                .set_body_bytes(compressed),
        )
        .mount(&server)
        .await;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--compressed")
        .arg(server.uri())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, body);
}
