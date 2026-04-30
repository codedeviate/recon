// Demo: prettify HTTP responses inside Rhai scripts.
//
// Two opts-map keys are wired in: `prettify` (auto-detect via Content-Type
// or body sniffing) and `prettify_as` (force a specific format — useful when
// the server returns the wrong Content-Type or no header at all). Setting
// `prettify_as` implicitly turns on `prettify`.

// Auto-detect: works when the server sets Content-Type correctly.
let r1 = http("https://httpbin.org/get", #{
    prettify: true,
});
print(r1.body);

// Forced format: overrides Content-Type / body sniffing.
let r2 = http("https://httpbin.org/get", #{
    prettify_as: "json",
});
print(r2.body);
