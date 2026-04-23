// Usage: recon --script file [PATH]
//
// Demo of the file-I/O bindings:
//  - whole-file: file_read, file_write_all, file_append_all,
//                file_exists, file_size, file_delete
//  - streaming:  file_open, file_read(h, n), file_read_all(h),
//                file_write(h, data), file_seek(h, pos, whence),
//                file_tell(h), file_flush(h), file_close(h)

let path = if args.len() > 1 { args[1] } else { "Cargo.toml" };

// Whole-file convenience — keeps the old smoke-test intact.
let bytes = file_read(path);
print(`${path}: ${bytes.len()} bytes`);
print(`sha256: ${sha256(bytes)}`);

// Streaming roundtrip into a scratch file.
let scratch = "/tmp/recon-file-demo.txt";
let h = file_open(scratch, "rwc");
file_write(h, "hello ");
file_write(h, "streaming world\n");
file_flush(h);
file_seek(h, 0, "start");
let head = file_read(h, 5);
print(`read(5) → "${head}"`);
print(`tell → ${file_tell(h)}`);
file_close(h);

// Append + inspect + clean up.
file_append_all(scratch, "appended\n");
print(`size → ${file_size(scratch)}`);
print(`exists → ${file_exists(scratch)}`);
file_delete(scratch);
print(`exists after delete → ${file_exists(scratch)}`);

return 0;
