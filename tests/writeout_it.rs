//! Integration tests for `-w / --write-out` rendering.

async fn setup_server(
    body: &'static str,
    status: u16,
    extra_headers: &[(&str, &str)],
) -> wiremock::MockServer {
    let server = wiremock::MockServer::start().await;
    // Pick content-type from extra_headers when provided, else fall back to
    // text/plain (same as set_body_string). This avoids set_body_string
    // stomping our explicit Content-Type.
    let ct = extra_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| *v)
        .unwrap_or("text/plain");
    let mut tmpl = wiremock::ResponseTemplate::new(status)
        .set_body_raw(body.as_bytes().to_vec(), ct);
    for (k, v) in extra_headers {
        if k.eq_ignore_ascii_case("content-type") {
            continue; // already applied via set_body_raw
        }
        tmpl = tmpl.insert_header(*k, *v);
    }
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(tmpl)
        .mount(&server)
        .await;
    server
}

#[tokio::test(flavor = "multi_thread")]
async fn writeout_basic_variables() {
    let server = setup_server(
        r#"{"x":1}"#,
        200,
        &[("Content-Type", "application/json")],
    )
    .await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-o")
        .arg("/dev/null")
        .arg("-w")
        .arg("%{http_code} %{content_type}\n")
        .arg(server.uri())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("200 application/json"), "stdout: {stdout}");
}

#[tokio::test(flavor = "multi_thread")]
async fn writeout_json_mode() {
    let server = setup_server("ok", 201, &[]).await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-o")
        .arg("/dev/null")
        .arg("-w")
        .arg("%{json}")
        .arg(server.uri())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout is valid JSON");
    // `http_code` is rendered as a JSON number; accept either int or float form.
    assert_eq!(parsed["http_code"].as_f64(), Some(201.0));
}

#[tokio::test(flavor = "multi_thread")]
async fn writeout_header_extraction() {
    let server = setup_server("body", 200, &[("X-Custom", "hello")]).await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-o")
        .arg("/dev/null")
        .arg("-w")
        .arg("%{header{x-custom}}\n")
        .arg(server.uri())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello"), "stdout: {stdout}");
}

#[tokio::test(flavor = "multi_thread")]
async fn writeout_unknown_variable_preserved() {
    let server = setup_server("x", 200, &[]).await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-o")
        .arg("/dev/null")
        .arg("-w")
        .arg("%{not_a_var}")
        .arg(server.uri())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "%{not_a_var}");
}

#[tokio::test(flavor = "multi_thread")]
async fn writeout_stderr_switch() {
    let server = setup_server("x", 200, &[]).await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recon"))
        .arg("-o")
        .arg("/dev/null")
        .arg("-w")
        .arg("out-text%{stderr}err-text")
        .arg(server.uri())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("out-text"));
    assert!(String::from_utf8(output.stderr).unwrap().contains("err-text"));
}
