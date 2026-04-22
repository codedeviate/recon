// Usage: recon --script whois [DOMAIN]
//
// Two-hop whois: IANA refer, then registry, plus registrar follow-up
// when present. Returns server + body.

let host = if args.len() > 1 { args[1] } else { "example.com" };
let r = whois(host);
print(`server: ${r.server}`);
print(`body:   ${r.body.len()} bytes`);
return 0;
