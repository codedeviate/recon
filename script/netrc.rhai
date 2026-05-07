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

// 1. `netrc_optional`: if the user happens to have a matching entry in
//    ~/.netrc, creds are picked up; otherwise the request goes out
//    unauthenticated. Against httpbin's basic-auth/<user>/<pass> route
//    that means a 401 unless the user really has an `httpbin.org` entry.
let r = http(url, #{ netrc_optional: true });
print(`optional netrc (no file required): ${r.status}` +
      if r.status == 401 { "  ← expected when ~/.netrc has no httpbin.org entry" } else { "" });

// 2. Synthesize a minimal netrc in /tmp and exercise the success path
//    via `netrc_file`. This is the CI pattern: drop creds next to the
//    job, point the request at the file, no global state needed.
let netrc_path = "/tmp/recon-demo.netrc";
file_write_all(netrc_path, "machine httpbin.org\nlogin alice\npassword s3cr3t\n");
let r2 = http(url, #{ netrc_file: netrc_path });
print(`explicit netrc_file=${netrc_path}: ${r2.status}`);
file_delete(netrc_path);

// 3. `netrc: true` requires ~/.netrc to exist — fails cleanly when
//    missing. Left commented because the failure noise distracts from
//    the demo; uncomment to see the strict-mode error message.
// let r3 = http(url, #{ netrc: true });
// print(`strict netrc: ${r3.status}`);

return 0;
