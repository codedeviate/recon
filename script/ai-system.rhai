//! ai-system.rhai — how .system() shapes the answer.
//!
//! Sends the same prompt twice with two different system prompts to
//! show how the system message steers tone and format. Both requests
//! are pinned to claude/sonnet.
//!
//! Token cost: two short prompts, ~1-2 sentence answers.

let req = ai::request()
    .backend("claude")
    .model("sonnet");

try {
    let formal = req
        .system("Reply formally in one sentence.")
        .prompt("What is TLS?")
        .send();
    print("formal: " + formal);

    let casual = req
        .system("Reply in three words or fewer, all lowercase.")
        .prompt("What is TLS?")
        .send();
    print("casual: " + casual);
} catch (e) {
    print("ai unavailable: " + e);
}
