// Demo: read, transform, write back to clipboard.
//
// Common workflow: copy something to your clipboard, run a recon script
// that processes it, get the result back in your clipboard.

let original = clipboard::get();
print("Read from clipboard: " + original);

let transformed = original.trim().to_upper();
clipboard::set(transformed);

print("Wrote back: " + transformed);
