// Usage: recon --script netstatus
//
// Run the default probe set and print aggregate status.

let r = netstatus::check();
print(`${r.status} (${r.passed}/${r.total})`);
for p in r.probes {
    let icon = if p.passed { "✓" } else { "✗" };
    print(`  ${icon} ${p.label} — ${p.detail}`);
}
return if r.status == "ONLINE" { 0 } else { 1 };
