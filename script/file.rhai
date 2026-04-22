// Usage: recon --script file [PATH]
//
// Read a file as a Blob and report its size + sha256.

let path = if args.len() > 1 { args[1] } else { "Cargo.toml" };
let bytes = file_read(path);
print(`${path}: ${bytes.len()} bytes`);
print(`sha256: ${sha256(bytes)}`);
return 0;
