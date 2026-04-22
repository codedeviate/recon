// Usage: recon --script dict [URL]
//
// RFC 2229 DICT client. Bare `dict://host/` runs SHOW SERVER +
// SHOW DATABASES + SHOW STRATEGIES; /d:WORD defines.

let url = if args.len() > 1 { args[1] } else { "dict://dict.dict.org/d:recon" };
let r = dict(url);
print(`${r.host}:${r.port} — ${r.banner}`);
for resp in r.responses {
    print(`[${resp.command}] ${resp.lines.len()} lines, status=${resp.final_status}`);
}
return 0;
