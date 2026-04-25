// form.rhai — multipart form uploads via -F/--form semantics.
//
// Usage: recon --script form
//
// The http() opts map accepts `form`, `form_string`, and
// `form_escape` with the same curl-compatible grammar as the CLI.
// Each element of `form` follows:
//   name=value            (literal)
//   name=@file            (file contents; filename attached)
//   name=@file;type=MIME;filename=NAME
//   name=<file            (file content, no filename)
//   name=<-               (content from stdin)
//
// `form_string` entries are always literal (no @ or < interpretation).

let r = http("https://httpbin.org/post", #{
    method: "POST",
    form: [
        "name=alice",
        "bio=@Cargo.toml;type=text/plain;filename=me.toml",
    ],
    form_string: [
        "literal_at=@not-a-path",
    ],
});
print(`status: ${r.status}`);

// httpbin echoes form fields back; dig through the JSON response.
let payload = json_parse(r.body);
if "form" in payload {
    print("form fields received:");
    for k in payload.form.keys() {
        print(`  ${k} = ${payload.form[k]}`);
    }
}

return 0;
