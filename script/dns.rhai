// Usage: recon --script dns [HOST [TYPE,TYPE,...]]
//
// DNS lookup with either the default type bundle or a comma-separated
// list of specific types.

let host = if args.len() > 1 { args[1] } else { "example.com" };
let r = if args.len() > 2 {
    let types = args[2].split(",");
    dns(host, types)
} else {
    dns(host)
};
print(`${r.host} (${r.duration_ms}ms)`);
for (records, typ) in r.records {
    if records.len() > 0 {
        print(`  ${typ}: ${records.len()} record${if records.len() == 1 { "" } else { "s" }}`);
    }
}
return 0;
