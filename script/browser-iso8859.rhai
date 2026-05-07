// Usage: recon --script browser-iso8859 [HOST]
//
// Talk to a legacy ISO-8859-1 service with a browser() session.
// Demonstrates two distinct flows:
//
//   1. Outbound transcode — declaring `charset=iso-8859-1` on the
//      Content-Type tells recon to transcode the request body from
//      UTF-8 (the shell default) to ISO-8859-1 before it hits the wire.
//
//   2. Inbound decode — `text::decode(blob, charset)` round-trips a
//      Latin-1 byte stream back to UTF-8.
//
// Note on httpbin as the test server: httpbin always parses incoming
// bodies as UTF-8 regardless of the declared charset, so the Latin-1
// bytes recon sent get mangled into U+FFFD ("replacement character")
// inside httpbin's JSON echo. That mangling is *itself* the proof
// that recon transcoded correctly — if recon had sent UTF-8 bytes,
// httpbin would have echoed back valid characters. The strong proof
// is the `Content-Length` header httpbin parroted back: 24 bytes for
// the Latin-1 encoding vs 26 for the UTF-8 encoding of the same text.

let host = if args.len() > 1 { args[1] } else { "https://httpbin.org" };

let b = browser();
b.set_header("Content-Type", "application/x-www-form-urlencoded; charset=iso-8859-1");

let body_utf8  = "name=Jörg&city=København";
let utf8_bytes = text::encode(body_utf8, "utf-8").len();         // 26
let lat1_bytes = text::encode(body_utf8, "iso-8859-1").len();    // 24

let r = b.post(`${host}/post`, body_utf8);
print(`posted ${utf8_bytes}-byte UTF-8 source (status ${r.status})`);

let explicit = text::decode(r.body_bytes, r.charset ?? "utf-8");
print(`response charset: ${r.charset ?? "()"}, body ${r.body_bytes.len()} bytes`);

// 1. Strong proof of outbound transcode: the echoed Content-Length is
//    the Latin-1 byte count, not the UTF-8 byte count.
let want_cl = `"Content-Length": "${lat1_bytes}"`;
if explicit.contains(want_cl) {
    print(`✓ outbound transcode: server saw ${lat1_bytes} bytes (matches ISO-8859-1; UTF-8 would have been ${utf8_bytes})`);
} else {
    print(`✗ outbound transcode: did not find ${want_cl} in the echoed headers`);
    return 1;
}

// 2. Confirm httpbin mangled the Latin-1 bytes when re-decoding as
//    UTF-8 — its echo should contain U+FFFD literally JSON-escaped as
//    six characters: backslash, u, f, f, f, d. Note: must be a regular
//    string `"\\ufffd"` (escaped backslash) and NOT a backtick template
//    `` `\\ufffd` `` — Rhai's backtick literals don't process `\\` as a
//    single backslash.
let needle = "\\ufffd";
if explicit.contains(needle) {
    print(`✓ httpbin echoed ${needle} for the accented chars — confirms the bytes weren't UTF-8`);
} else {
    print(`✗ no ${needle} in echo — server may actually be Latin-1-aware (rare)`);
}

// 3. Round-trip primitive demo (independent of the test server): show
//    that text::encode → text::decode preserves the original.
let blob_lat1 = text::encode(body_utf8, "iso-8859-1");
let back      = text::decode(blob_lat1, "iso-8859-1");
if back == body_utf8 {
    print(`✓ text::encode/decode round-trip preserves "Jörg" and "København"`);
} else {
    print(`✗ round-trip mismatch: got "${back}"`);
    return 1;
}

return 0;
