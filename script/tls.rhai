// Usage: recon --script tls [HOST [PORT]]
//
// TLS certificate inspection: CN, issuer, expiry, days remaining.

let host = if args.len() > 1 { args[1] } else { "example.com" };
let port = if args.len() > 2 { parse_int(args[2]) } else { 443 };
let c = tls(host, port);
print(`${host}:${port}`);
print(`  CN:      ${c.subject.common_name}`);
print(`  issuer:  ${c.issuer.common_name}`);
print(`  expires: ${c.not_after} (${c.days_remaining} days)`);
if c.is_expired { return 2; }
if c.days_remaining < 30 {
    print(`  ⚠ EXPIRES IN < 30 DAYS`);
    return 1;
}
return 0;
