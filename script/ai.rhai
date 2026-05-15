//! ai.rhai — minimal demo of the ai::* bindings.
//!
//! Requires one of these CLIs on PATH AND a backend selection:
//!   export RECON_AI_BACKEND=claude   # or codex, gemini
//!
//! Or via ~/.recon/config.toml:
//!   [ai]
//!   default_backend = "claude"
//!
//! Skipped at parse-check time only; real execution requires the CLI.

let req = ai::request();
req.system("You are a concise TLS expert.");
req.context("Certificate subject CN: example.com");
req.context("Issuer: Let's Encrypt R3");
req.prompt("Is this a typical commercial cert?");
req.timeout(30);

// The actual call is wrapped in try so the script also runs without
// any AI backend configured — it just prints the error in that case.
try {
    let answer = req.send();
    print("answer: " + answer);
} catch (e) {
    print("ai not available: " + e);
}
