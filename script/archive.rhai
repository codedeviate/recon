// Usage: recon --script archive
//
// Create + extract a tiny zip round-trip in /tmp.

let dir = "/tmp/recon-archive-example";
// Prepare a source file.
let src_path = `${dir}/hello.txt`;
let archive_path = `${dir}/demo.zip`;
let out_dir = `${dir}/unpacked`;
// (No file-write binding — use a pre-existing file.)
let src = "Cargo.toml";

let n = archive::create(archive_path, [src]);
print(`archived ${n} file into ${archive_path}`);
let m = archive::extract(archive_path, out_dir);
print(`extracted ${m} file to ${out_dir}`);
return 0;
