// Usage: recon --script ntp [HOST]
//
// Query an NTP server and report clock offset + round-trip delay.

let host = if args.len() > 1 { args[1] } else { "pool.ntp.org" };
let r = ntp(host);
print(`${r.host}:${r.port}  stratum ${r.stratum} (${r.ref_id})`);
print(`  offset  ${r.offset_ms} ms`);
print(`  delay   ${r.delay_ms} ms`);
return 0;
