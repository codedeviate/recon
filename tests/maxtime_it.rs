#[tokio::test(flavor = "multi_thread")]
async fn max_time_exits_28_on_slow_server() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(10))
                .set_body_string("late"),
        )
        .mount(&server)
        .await;

    let start = std::time::Instant::now();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("--max-time").arg("0.5")
        .arg(server.uri())
        .output()
        .unwrap();
    let elapsed = start.elapsed();

    assert_eq!(output.status.code(), Some(28), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(elapsed.as_secs_f64() < 3.0, "elapsed {} too long", elapsed.as_secs_f64());
}
