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

// sprintf / printf — supports %s %d %f %x %X %o %b %c %% with flags + width.
print(sprintf("%-10s %5d", ["alpha", 42]));      // "alpha           42"
print(sprintf("hex=%#x bin=%08b", [255, 10]));    // "hex=0xff bin=00001010"
print(sprintf("pi=%.4f", 3.14159265));            // "pi=3.1416"
printf("hello %s\n", "world");                    // writes to stdout

// printf returns the byte count; discard it so the script's exit
// code stays 0 (recon maps a trailing integer to its exit status).
()
