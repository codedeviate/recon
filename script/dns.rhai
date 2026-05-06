// Usage: recon --script dns [HOST [TYPE,TYPE,...]]
//
// DNS lookup with either the default type bundle or a comma-separated
// list of specific types.

let host = if args.len() > 1 { args[1] } else { "example.com" };
let r = if args.len() > 2 {
    // `args` is pushed into Rhai's Scope as a constant, so non-pure
    // methods (split, replace, …) can't be called on `args[N]` directly.
    // Bind to a local first.
    let raw = args[2];
    let types = raw.split(",");
    dns(host, types)
} else {
    dns(host)
};
print(`${r.host} (${r.duration_ms}ms)`);
for typ in r.records.keys() {
    let records = r.records[typ];
    if records.len() > 0 {
        print(`  ${typ}: ${records.len()} record${if records.len() == 1 { "" } else { "s" }}`);
    }
}
return 0;
