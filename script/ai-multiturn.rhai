//! ai-multiturn.rhai — manual multi-turn replay.
//!
//! Asks one question, then asks a follow-up that depends on the
//! first answer. Caller manually calls .assistant() to feed the
//! prior reply back into the request before .user() sets the new
//! question — .send() does not auto-remember (by design).
//! Backend pinned to claude/sonnet.
//!
//! Token cost: two short prompts, one-word answers.

let req = ai::request()
    .backend("claude")
    .model("sonnet")
    .system("Reply with one word only.")
    .prompt("Pick a colour: red or blue?");

try {
    let first = req.send();
    print("Q1 -> " + first);

    req.assistant(first);
    req.user("Now name a fruit of that colour.");

    let second = req.send();
    print("Q2 -> " + second);
} catch (e) {
    print("ai unavailable: " + e);
}
