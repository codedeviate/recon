//! ai-simple.rhai — minimal ai::request() builder demo.
//!
//! Sends a tiny prompt to claude/sonnet and prints the answer.
//! Backend is hardcoded so the script runs without any RECON_AI_*
//! env or config — change `.backend("claude")` if you've got a
//! different CLI installed (codex, copilot, gemini).
//!
//! Token cost: a few input tokens + a one-word answer.

try {
    let answer = ai::request()
        .backend("claude")
        .model("sonnet")
        .system("Reply with one word only.")
        .prompt("Is HTTP stateless?")
        .send();
    print("answer: " + answer);
} catch (e) {
    print("ai unavailable: " + e);
}
