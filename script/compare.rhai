// compare.rhai — in-script diff of two byte buffers or strings.
//
// Usage:
//   recon --script compare.rhai
//
// compare(a, b) returns a map:
//   identical   : bool
//   a_bytes     : int
//   b_bytes     : int
//   added       : int    // lines inserted (0 if binary)
//   removed     : int    // lines deleted (0 if binary)
//   binary      : bool
//   diff        : string // unified diff (empty if identical)

let a = "one\ntwo\nthree\n";
let b = "one\ntwo plus\nthree\nfour\n";

let r = compare(a, b);
print(`identical: ${r.identical}`);
print(`+${r.added} / -${r.removed}`);
print("--- diff ---");
print_raw(r.diff);
