// Usage: recon --script email [DOMAIN]
//
// Runs SPF, DMARC, MTA-STS, TLS-RPT, BIMI in one aggregate call.

let host = if args.len() > 1 { args[1] } else { "example.com" };
let r = email::all(host);
for (v, k) in r {
    if v.contains_key("error") {
        print(`${k}: ERROR — ${v.error}`);
    } else {
        print(`${k}: ${v.verdict} — ${v.summary}`);
    }
}
return 0;
