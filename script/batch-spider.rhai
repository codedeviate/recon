// batch-spider.rhai — bulk link check with retry + rate limit.
//
// Usage: recon --script batch-spider [URLS_FILE]
//
// Rather than the CLI --input-file + --spider combo, this script
// reads the URL list itself and runs `http()` with spider opts per
// URL. Lets the script add its own per-result logic: count broken
// links, write a report, emit JSON summary, etc.

let path = if args.len() > 1 { args[1] } else { "/tmp/urls.txt" };

if !file_exists(path) {
    eprint(`url list not found: ${path}`);
    eprint("create one with e.g.:");
    eprint("  echo 'https://example.com/' > /tmp/urls.txt");
    eprint("  echo 'https://httpbin.org/status/200' >> /tmp/urls.txt");
    eprint("  echo 'https://httpbin.org/status/404' >> /tmp/urls.txt");
    return 2;
}

let urls = file_read(path).to_string().split('\n');

let ok = 0;
let broken = [];
let rate_delay_ms = 500;   // 2 requests per second

for raw in urls {
    let url = raw.trim();
    if url.len() == 0 || url.starts_with('#') { continue; }
    sleep_ms(rate_delay_ms);

    // HEAD check with 2 retries on transient failures.
    let r = http(url, #{
        spider: true,
        retry: 2,
        retry_delay: 1,
    });
    if r.status >= 200 && r.status < 400 {
        ok += 1;
    } else {
        broken.push(#{ url: url, status: r.status });
    }
    print(`${r.status} ${url}`);
}

print("");
print(`summary: ${ok} OK, ${broken.len()} broken`);
if broken.len() > 0 {
    print("broken:");
    for b in broken {
        print(`  ${b.status} ${b.url}`);
    }
    return 1;
}
return 0;
