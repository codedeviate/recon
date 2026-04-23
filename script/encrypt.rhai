// Usage: recon --script encrypt
//
// Demonstrates the encrypt::* static module:
//   - keygen          -> fresh age X25519 keypair
//   - encrypt/decrypt -> round-trip against a blob
//   - rekey           -> rotate from one recipient to another
//   - detect_backend  -> dispatch age vs pgp from recipient format
//
// age identities must load from file paths. Scripts can't write
// files directly, so this demo prints the shell one-liners needed to
// persist the key. A full in-script rekey round-trip is possible when
// the identity files already exist (see the try/catch block).

let k1 = encrypt::keygen();
let k2 = encrypt::keygen();
print(`old public: ${k1["public"]}`);
print(`new public: ${k2["public"]}`);

// In-memory encrypt demo (no file I/O needed).
let cipher = encrypt::encrypt("the quick brown fox".to_blob(), [k1["public"]]);
print(`encrypted: ${cipher.len()} bytes (binary age format)`);

let armored = encrypt::encrypt_armored("the quick brown fox".to_blob(), [k1["public"]]);
print(`armored:   ${armored.len()} bytes (ASCII)`);

// rekey needs identity files on disk to decrypt. Uncomment the shell
// one-liners below, then re-run this script, to exercise the rotation.
let id1 = "/tmp/recon-encrypt-demo-old.key";
let id2 = "/tmp/recon-encrypt-demo-new.key";
print(`# Persist the keys once:`);
print(`#   echo '${k1["private"]}' > ${id1} && echo '${k2["private"]}' > ${id2}`);

try {
    let c1 = encrypt::encrypt("rotate this".to_blob(), [k1["public"]]);
    let c2 = encrypt::rekey(c1, [id1], [k2["public"]]);
    let back = encrypt::decrypt(c2, [id2]);
    if back == "rotate this".to_blob() {
        print("✓ rekey round-trip: c1 decrypted with id1, re-encrypted, decrypted with id2");
    }
} catch (e) {
    print(`(rekey round-trip skipped; create the .key files to exercise it)`);
}

// Backend dispatch helper.
for r in [k1["public"], "0xDEADBEEF", "alice@example.com"] {
    let b = encrypt::detect_backend(r);
    print(`  ${b}  <- ${r}`);
}

return 0;
