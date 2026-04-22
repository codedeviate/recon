// Usage: recon --script text [URL]
//
// Demonstrates the text::* module: fetch a page, inspect the declared /
// sniffed charset, decode bytes explicitly, and hash both representations.

let url = if args.len() > 1 { args[1] } else { "https://example.com/" };

let r = http(url);
print(`url:          ${r.url}`);
print(`charset:      ${r.charset ?? "(undetectable)"}`);
print(`body size:    ${r.body_bytes.len()} bytes`);

// Re-decode the raw bytes with an explicit charset. When the server sent
// UTF-8 this is redundant; with a Latin-1 server it recovers accented
// characters that `r.body` mangled.
let explicit = text::decode(r.body_bytes, r.charset ?? "utf-8");
let same = explicit == r.body;
print(`body match:   ${same}  (re-decoded == lossy body)`);

// Round-trip: encode to Latin-1, back to UTF-8, compare.
let round_trip_bytes = text::encode(explicit, "iso-8859-1");
let round_trip = text::decode(round_trip_bytes, "iso-8859-1");
print(`round-trip:   ${round_trip.len()} chars (Latin-1 roundtrip)`);

// BOM detection snippet.
let bom = blob();
bom.push(0xEF); bom.push(0xBB); bom.push(0xBF); bom.push(0x68); bom.push(0x69);
let d = text::detect(bom);
print(`BOM detect:   ${d.charset} (had_bom=${d.had_bom})`);

// Line-ending normalisation.
let mixed = "a\r\nb\rc\nd";
let lf = text::normalize_newlines(mixed, "lf");
print(`nl normalise: ${lf.len()} bytes LF-only (was ${mixed.len()})`);

return 0;
