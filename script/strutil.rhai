// strutil.rhai — PHP-style string helpers.
// Usage: recon --script strutil.rhai

// trim / ltrim / rtrim — whitespace by default, custom mask optional.
print(trim("   hello   "));            // "hello"
print(ltrim("...path", "."));          // "path"
print(rtrim("file.log", ".log"));      // "file"

// strrev — Unicode-aware character reverse.
print(strrev("hello"));                // "olleh"
print(strrev("café"));                 // "éfac"

// HTML helpers.
print(strip_html("<p>plain <b>text</b></p>"));   // "plain text"
print(nl2br("line1\nline2"));                    // "line1<br>\nline2"
print(br2nl("line1<br>line2"));                  // "line1\nline2"

// Regex (PHP-style delimiters optional).
let caps = preg_match("/^Host:\\s*(.+)$/i", "Host: example.com");
print(caps);                                      // ["Host: example.com", "example.com"]
print(preg_replace("\\s+", "-", "a  b   c"));     // "a-b-c"

// Array join — also callable as join(arr, sep).
print(["a", "b", "c"].join(", "));               // "a, b, c"
print([1, "two", 3.5].join("-"));                 // "1-two-3.5"

// sprintf / printf — supports %s %d %f %x %X %o %b %c %% with flags + width.
print(sprintf("%-10s %5d", ["alpha", 42]));      // "alpha           42"
print(sprintf("hex=%#x bin=%08b", [255, 10]));    // "hex=0xff bin=00001010"
print(sprintf("pi=%.4f", 3.14159265));            // "pi=3.1416"
printf("hello %s\n", "world");                    // writes to stdout

// URL encoding.
print(urlencode("hello world & friends?"));       // "hello%20world%20%26%20friends%3F"
print(urldecode("name%3DJ%C3%B6rg"));             // "name=Jörg"

// Base64 — accepts string or Blob, decode returns a Blob.
print(base64_encode("hello"));                    // "aGVsbG8="
let b = base64_decode("aGVsbG8=");
print(text::decode(b, "utf-8"));                  // "hello"

// HTML entity decode — companion to strip_html.
print(html_entity_decode("&lt;b&gt;A &amp; B&lt;/b&gt;"));   // "<b>A & B</b>"

// Padding — str_pad takes optional side ("left" / "right" / "both"),
// lpad / rpad are the bare-name aliases.
print(str_pad("42", 6, "0", "left"));             // "000042"
print(rpad("hi", 5, "."));                        // "hi..."
print(str_pad("hi", 6, "-", "both"));             // "--hi--"

// Path helpers — POSIX semantics.
print(dirname("/var/log/recon.log"));             // "/var/log"
print(basename("/var/log/recon.log", ".log"));    // "recon"

// Date formatting — chrono strftime spec; pass "local" as third arg
// to use the system timezone instead of UTC.
print(date_format(1700000000, "%Y-%m-%dT%H:%M:%SZ"));   // "2023-11-14T22:13:20Z"
print(date_format(now_ms() / 1000, "%a %d %b %Y", "local"));

// printf returns the byte count; discard it so the script's exit
// code stays 0 (recon maps a trailing integer to its exit status).
()
