// netrc.rhai — .netrc-backed Basic auth.
//
// Usage: recon --script netrc [URL]
//
// `~/.netrc` entry format:
//   machine api.example.com
//   login alice
//   password s3cr3t
//
//   default
//   login guest
//   password guest
//
// Script opts:
//   netrc          : bool  — require the file exists (fail otherwise)
//   netrc_file     : path  — alternative netrc path
//   netrc_optional : bool  — silent fallback if the file doesn't exist

let url = if args.len() > 1 { args[1] } else { "https://httpbin.org/basic-auth/alice/s3cr3t" };

// Try optional first — works whether or not the user has a .netrc.
let r = http(url, #{ netrc_optional: true });
print(`optional netrc: ${r.status}`);

// Explicit netrc file — good for CI where the creds live alongside
// the job.
// let r = http(url, #{ netrc_file: "./ci.netrc" });
// print(`ci netrc: ${r.status}`);

// Require netrc — fails cleanly if missing.
// let r = http(url, #{ netrc: true });

return 0;
