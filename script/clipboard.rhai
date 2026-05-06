// Demo: read, transform, write back to clipboard.
//
// Common workflow: copy something to your clipboard, run a recon script
// that processes it, get the result back in your clipboard.

let original = clipboard::get();
print("Read from clipboard: " + original);

// Rhai's String::trim is mutating (returns ()), so it can't be chained.
// Apply trim in-place on a local mutable copy, then convert via to_upper.
let buf = original;
buf.trim();
let transformed = buf.to_upper();
clipboard::set(transformed);

print("Wrote back: " + transformed);
