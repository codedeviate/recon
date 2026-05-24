// jq.rhai — apply jq-style filters to any Map or Array.
// Usage: recon --script jq.rhai

let prs = [
    #{ number: 1, title: "Add foo", state: "open",   author: "alice" },
    #{ number: 2, title: "Fix bar", state: "closed", author: "bob"   },
    #{ number: 3, title: "Tweak",   state: "open",   author: "alice" },
];

// `.jq(filter)` returns the first result, or () if nothing matches.
print(prs.jq(".[0].title"));                                // "Add foo"
print(prs.jq(".[] | select(.state == \"closed\") | .title")); // "Fix bar"
print(prs.jq(".[] | select(.state == \"merged\") | .title")); // ()

// `.jq_all(filter)` returns every result as an Array.
print(prs.jq_all(".[] | select(.state == \"open\") | .number"));   // [1, 3]
print(prs.jq_all(".[] | select(.author == \"alice\") | .title"));  // ["Add foo", "Tweak"]
print(json_stringify(prs.jq_all(".[] | .author")));            // "[\"alice\",\"bob\",\"alice\"]"

// Free-function form is also available.
print(jq(prs, ".[0].state"));   // "open"
print(jq_all(prs, ".[].state")); // ["open", "closed", "open"]

// Combines with json_parse for raw JSON text.
let raw = `{"items":[{"k":"a","v":1},{"k":"b","v":2}]}`;
print(json_parse(raw).jq(".items[0].k"));   // "a"
print(json_parse(raw).jq_all(".items[].v")); // [1, 2]

// Filter parse errors throw — wrap in try/catch to recover.
try {
    prs.jq("invalid syntax (");
} catch (e) {
    print(`caught: ${e}`);
}

// Return cleanly so the script exits 0 (recon maps a trailing int
// to the exit code).
()
