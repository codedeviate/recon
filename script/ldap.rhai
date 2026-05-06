// Usage: recon --script ldap [URL]
//
// Anonymous bind + RootDSE query.

let url = if args.len() > 1 { args[1] } else { "ldap://ldap.forumsys.com" };
let r = ldap(url);
print(`${r.url}  ${r.connect_ms} ms`);
for key in r.attrs.keys() {
    let values = r.attrs[key];
    print(`  ${key}: ${values.len()} value(s)`);
}
return 0;
