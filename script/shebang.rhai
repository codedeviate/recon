#!/usr/bin/env -S recon --script
//
// Usage: ./script/shebang.rhai [HOST]
//   or:  recon --script script/shebang.rhai [HOST]
//
// Demonstrates shebang support (0.68.0). When the file is executable
// (chmod +x) the kernel invokes recon directly; the #! line is silently
// converted to a // comment before Rhai sees it.
//
// To try the shebang mode:
//   chmod +x script/shebang.rhai
//   ./script/shebang.rhai example.com

let host = if args.len() > 1 { args[1] } else { "example.com" };
let url  = `https://${host}`;

let r = http(url);
print(`${r.status} ${r.final_url} (${r.duration_ms}ms)`);

if r.status == 200 {
    print(`body: ${r.body.len()} bytes`);
    return 0;
} else {
    eprint(`unexpected status ${r.status}`);
    return 1;
}
