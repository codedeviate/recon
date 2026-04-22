// Usage: recon --script browser-persist [JAR-NAME]
//
// Persistent sessions survive across script runs. The cookie jar
// lives at ~/.recon/jars/NAME.db — the same file format used by
// --cookiejar and sqlite("cookiejar:NAME").
//
// Run twice to see the jar's contents accumulate. Clear with
//   sqlite3 ~/.recon/jars/NAME.db "DELETE FROM cookies;"

let jar = if args.len() > 1 { args[1] } else { "recon-demo" };

let b = browser();
b.use_persistent_session(jar);
print(`session: ${b.session_name()}`);
print(`jar path: ~/.recon/jars/${jar}.db`);

// Record a timestamp cookie on each run.
let ts = now();
b.get(`https://httpbin.org/cookies/set/run_${ts}/ok`);

let cookies = b.cookies();
print(`jar now holds ${cookies.len()} cookie(s):`);
for c in cookies {
    print(`  ${c.name} = ${c.value}   (domain ${c.domain})`);
}

// Clear programmatically when you're done testing.
if args.len() > 2 && args[2] == "clear" {
    b.clear_cookies();
    print("cleared");
}

return 0;
