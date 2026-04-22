// Usage: recon --script encrypt
//
// Generate a keypair, write identity to a tempfile, encrypt + decrypt.

let keys = encrypt::keygen();
let pub_key = keys["public"];
let priv_key = keys["private"];
print(`public:  ${pub_key}`);

// age identities must be loaded from file paths (see encrypt module docs).
let id_path = "/tmp/recon-encrypt-demo.key";
let db = sqlite(":memory:");
// We don't have file_write; script demonstrates the guard + keygen path.
// To complete a real round-trip, run recon --encrypt / --decrypt from the CLI
// or use a shell:
//   echo "$PRIV" > id.key && recon --encrypt -r $PUB ...
print(`(demo only) keys generated. For a full round-trip, use:`);
print(`  echo '${priv_key}' > ${id_path}`);
print(`  recon --encrypt -r '${pub_key}' plaintext.txt -o out.age`);
print(`  recon --decrypt --identity ${id_path} out.age`);
return 0;
