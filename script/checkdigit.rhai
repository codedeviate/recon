// Usage: recon --script checkdigit [ALGO [INPUT]]
//
// Verify or create a check-digit value.

let algo = if args.len() > 1 { args[1] } else { "luhn" };
let input = if args.len() > 2 { args[2] } else { "4111111111111111" };
let info = checkdigit::inspect(algo, input);
if info.valid {
    print(`${algo} ${info.formatted}: valid`);
    if "comment" in info { print(`  ${info.comment}`); }
    return 0;
}
print(`${algo} ${input}: invalid (${info.reason})`);
return 1;
