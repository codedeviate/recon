//! ai-context.rhai — accumulating .context() blocks.
//!
//! Builds context from multiple pieces (here: a hand-rolled fake
//! response — in real scripts you'd build it from prior http(),
//! tls(), or dns() probes) and asks one question about it.
//! Backend pinned to claude/sonnet.
//!
//! Token cost: ~50 input tokens + a one-word answer.

let req = ai::request()
    .backend("claude")
    .model("sonnet")
    .system("Reply yes or no only.");

req.context("Status: 200 OK");
req.context("Content-Type: application/json");
req.context("Server: nginx/1.27.0");
req.prompt("Is this an HTTP success response?");

try {
    print("verdict: " + req.send());
} catch (e) {
    print("ai unavailable: " + e);
}
