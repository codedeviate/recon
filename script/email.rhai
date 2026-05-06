// Usage: recon --script email [DOMAIN]
//
// Runs SPF, DMARC, MTA-STS, TLS-RPT, BIMI in one aggregate call.

let host = if args.len() > 1 { args[1] } else { "example.com" };
let r = email::all(host);
for k in r.keys() {
    let v = r[k];
    if "error" in v {
        print(`${k}: ERROR — ${v.error}`);
    } else {
        print(`${k}: ${v.verdict} — ${v.summary}`);
    }
}
return 0;
