// Usage: recon --script sample
//
// Enumerate built-in sample sources.

let samples = sample::list();
print(`${samples.len()} built-in samples:`);
for s in samples {
    print(`  ${s.name} (${s.default_format}): ${s.description}`);
}
return 0;
