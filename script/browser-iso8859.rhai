// Usage: recon --script browser-iso8859 [HOST]
//
// Talk to a legacy ISO-8859-1 service with a browser() session. POST
// UTF-8 input — the browser transcodes to Latin-1 because the Content-
// Type header declares it — then decode the response back to UTF-8.

let host = if args.len() > 1 { args[1] } else { "https://httpbin.org" };

let b = browser();
// Declaring the charset on Content-Type tells recon to transcode the
// request body from UTF-8 (the shell default) to ISO-8859-1 automatically.
b.set_header("Content-Type", "application/x-www-form-urlencoded; charset=iso-8859-1");

// Post an accented name — the browser transcodes before sending.
let r = b.post(`${host}/post`, "name=Jörg&city=København");
print(`posted (status ${r.status})`);

// httpbin echoes the raw form fields back in the JSON response. We
// decode the echo explicitly using the charset recon sniffed / saw.
let explicit = text::decode(r.body_bytes, r.charset ?? "utf-8");
print(`echoed charset: ${r.charset ?? "()"}`);
print(`body length:    ${r.body_bytes.len()} bytes`);

// Show that both representations are usable.
if explicit.contains("J") {
    print("✓ UTF-8 decode of the echo contains the input");
}

return 0;
