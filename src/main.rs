mod cert;
mod cli;
mod client;
mod cookiejar;
mod dns;
mod email;
mod examples;
mod output;
mod ping;
mod prettify;
mod scp;
mod tls_probe;
mod traceroute;
mod util;
mod whois;

use clap::Parser;
use cli::Args;

fn main() {
    // --examples doesn't require a URL; intercept before clap validates required args
    if std::env::args().any(|a| a == "--examples") {
        examples::print();
        return;
    }

    let args = Args::parse();

    // ── Cookie jar management commands (no HTTP request needed) ───────────────
    let is_cookie_mgmt = args.cookies || args.cookie_delete.is_some() || args.cookie_set.is_some();
    if is_cookie_mgmt {
        let name = match &args.cookiejar {
            Some(n) => n.as_str(),
            None => {
                eprintln!("error: --cookies, --cookie-delete and --cookie-set require --cookiejar <name>");
                std::process::exit(1);
            }
        };
        let result = run_cookie_mgmt(&args, name);
        if let Err(err) = result {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
        return;
    }

    // ── Validate flag combinations ────────────────────────────────────────────
    if args.exclusive_count() > 1 {
        eprintln!("error: --ping, --traceroute, and --whois are mutually exclusive");
        std::process::exit(1);
    }
    if args.has_exclusive() && args.has_composable() {
        eprintln!("error: --ping, --traceroute, and --whois cannot be combined with domain-inspection flags");
        std::process::exit(1);
    }

    // ── Dispatch ──────────────────────────────────────────────────────────────
    let result = if args.traceroute {
        traceroute::run(args.target_url(), args.max_hops)
    } else if args.ping {
        ping::run(args.target_url(), args.ping_count)
    } else if args.whois {
        whois::run(args.target_url())
    } else if args.has_composable() {
        run_composable(&args)
    } else if args.target_url().starts_with("scp://") {
        scp::download(args.target_url(), &args)
    } else {
        let t0 = std::time::Instant::now();
        client::execute(&args).and_then(|response| {
            if args.verbose >= 2 {
                eprintln!("* Elapsed: {:.3}s", t0.elapsed().as_secs_f64());
            }
            output::write_response(response, &args)
        })
    };

    if let Err(err) = result {
        if args.full_errors {
            eprintln!("error: {err:#}");
        } else {
            eprintln!("error: {}", friendly_message(&err));
        }
        std::process::exit(1);
    }
}

fn run_composable(args: &Args) -> anyhow::Result<()> {
    if args.cert {
        cert::fetch_and_print(args.target_url())?;
    }

    if args.dns {
        dns::run(args.target_url(), &args.dns_type)?;
    }

    if args.has_email_checks() {
        let (host, _port) = util::parse_target(args.target_url());
        let checks = email::EmailChecks {
            spf: args.spf,
            dmarc: args.dmarc,
            dkim_selectors: args.dkim.clone(),
            mta_sts: args.mta_sts,
            bimi: args.bimi.clone(),
            tls_rpt: args.tls_rpt,
            insecure: args.insecure,
        };
        email::run(&host, checks)?;
    }

    Ok(())
}

fn run_cookie_mgmt(args: &Args, jar_name: &str) -> anyhow::Result<()> {
    use cookiejar::CookieJar;

    let jar = CookieJar::open(jar_name)?;

    if let Some(id) = args.cookie_delete {
        if jar.delete(id)? {
            eprintln!("Deleted cookie #{id}");
        } else {
            eprintln!("No cookie with ID {id}");
        }
    }

    if let Some(cookie_str) = &args.cookie_set {
        jar.set_from_str(cookie_str)?;
        eprintln!("Cookie saved");
    }

    // --cookies lists the jar; also shown automatically after --cookie-set / --cookie-delete
    if args.cookies || args.cookie_delete.is_some() || args.cookie_set.is_some() {
        let cookies = jar.list()?;
        eprintln!("Cookie jar: {}", jar.path.display());
        eprintln!();
        CookieJar::print_table(&cookies);
    }

    Ok(())
}

fn friendly_message(err: &anyhow::Error) -> String {
    let msg = err.to_string();
    let root = err.root_cause().to_string();

    if msg.starts_with("Could not connect to")
        || msg.starts_with("Could not resolve")
        || msg.starts_with("Invalid URL")
        || msg.starts_with("--cert")
        || msg.starts_with("TLS handshake")
        || msg.starts_with("Server did not")
        || msg.starts_with("ICMP ping requires")
        || msg.starts_with("Unknown DNS record type")
        || msg.starts_with("SSH handshake failed")
        || msg.starts_with("SSH host key")
        || msg.starts_with("All SSH authentication")
        || msg.starts_with("SCP failed")
        || msg.starts_with("SCP URL")
    {
        return msg;
    }

    if msg.contains("dns error") || root.contains("dns error") || root.contains("failed to lookup")
    {
        return format!("Could not resolve host: {}", extract_host(&msg));
    }
    if root.contains("Connection refused") || root.contains("connection refused") {
        return format!("Connection refused: {}", extract_host(&msg));
    }
    if root.contains("timed out") || root.contains("deadline has elapsed") {
        return "Connection timed out".to_string();
    }
    if root.contains("certificate") || root.contains("tls") || root.contains("TLS") {
        return "TLS/certificate error — the server's certificate could not be verified"
            .to_string();
    }
    if msg.contains("Invalid HTTP method")
        || msg.contains("Invalid header format")
        || msg.contains("Failed to read file")
        || msg.contains("HTTP error")
    {
        return msg;
    }
    if root.contains("No such file or directory") || root.contains("os error 2") {
        return format!("File not found: {}", extract_path(&msg));
    }
    if root.contains("Permission denied") {
        return format!("Permission denied: {}", extract_path(&msg));
    }

    msg.lines()
        .next()
        .unwrap_or("an unexpected error occurred")
        .to_string()
}

fn extract_host(msg: &str) -> &str {
    if let Some(start) = msg.find("https://").or_else(|| msg.find("http://")) {
        let rest = &msg[start..];
        return rest.split_whitespace().next().unwrap_or(rest);
    }
    "unknown host"
}

fn extract_path(msg: &str) -> &str {
    if let Some(pos) = msg.rfind(": ") {
        return &msg[pos + 2..];
    }
    "unknown path"
}
