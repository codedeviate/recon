mod cert;
mod cli;
mod client;
mod compression;
mod config;
mod cookiejar;
mod dict_probe;
mod dns;
mod editor;
mod email;
mod encode;
mod checkdigit;
mod encrypt;
mod examples;
mod fail;
mod file_url;
mod hash;
mod help;
mod jwt;
mod ldap_probe;
mod lorem;
mod memcached_probe;
mod metrics;
mod mqtt;
mod netstatus;
mod ntp_probe;
mod output;
mod ping;
mod prettify;
mod redis_probe;
mod remote_name;
mod rtsp_probe;
mod writeout;
mod sampledata;
mod scp;
mod serve;
mod source;
mod ssh;
mod ssh_auth;
mod tcp_probe;
mod telnet;
mod tls_probe;
mod traceroute;
mod udp_probe;
mod util;
mod version;
mod whois;
mod ws_probe;

use clap::{CommandFactory, Parser};
use cli::Args;

fn main() {
    // ── --help [topic] interception (before clap parses) ─────────────────────
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--help" || a == "-h") {
            let next = args.get(pos + 1);
            match next {
                Some(topic) if !topic.starts_with('-') => {
                    if !help::print_topic(topic) {
                        help::print_unknown_topic(topic);
                    }
                    return;
                }
                _ => {
                    let mut cmd = Args::command();
                    let _ = cmd.print_help();
                    println!();
                    help::print_topic_footer();
                    return;
                }
            }
        }
    }

    // --version / -V / --version-short don't require a URL; intercept before
    // clap validates required args. --version-short takes precedence if both
    // are passed.
    {
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|a| a == "--version-short") {
            version::print_short();
            return;
        }
        if args.iter().any(|a| a == "--version" || a == "-V") {
            version::print_full();
            return;
        }
    }

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

    // ── JWT operations (no HTTP request needed) ───────────────────────────────
    if args.has_jwt() {
        let result = run_jwt(&args);
        if let Err(err) = result {
            if args.full_errors {
                eprintln!("error: {err:#}");
            } else {
                eprintln!("error: {}", friendly_message(&err));
            }
            std::process::exit(1);
        }
        return;
    }

    // ── Editor temp file cleanup (no HTTP request needed) ────────────────────
    if args.editor_cleanup {
        match editor::cleanup_temp_files() {
            Ok(n) => {
                println!("removed {n} file{}", if n == 1 { "" } else { "s" });
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    // ── Sample data: list available samples ──────────────────────────────────
    if args.sample_list {
        let cfg_map = match load_sampledata_config() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        };
        print_sample_list(&sampledata::list_samples(&cfg_map));
        return;
    }

    // ── Hash: list supported algorithms ──────────────────────────────────────
    if args.hash_list {
        if let Err(e) = hash::print_list(&mut std::io::stdout().lock()) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── Hash: compute digest of the input source ─────────────────────────────
    if args.hash.is_some() {
        if args.remote_name {
            eprintln!("error: --hash and -O/--remote-name are mutually exclusive");
            std::process::exit(1);
        }
        if args.encode.is_some() {
            eprintln!("error: --encode and --hash are mutually exclusive");
            std::process::exit(1);
        }
        if let Err(err) = hash::run(&args) {
            if args.full_errors {
                eprintln!("error: {err:#}");
            } else {
                eprintln!("error: {}", friendly_message(&err));
            }
            std::process::exit(1);
        }
        return;
    }

    // ── Compression: list supported algorithms ───────────────────────────────
    if args.compress_list {
        if let Err(e) = compression::print_list(&mut std::io::stdout().lock()) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── Compression: compress / decompress the input source ─────────────────
    if args.compress.is_some() || args.decompress.is_some() {
        // Mutual exclusions.
        if args.compress.is_some() && args.decompress.is_some() {
            eprintln!("error: --compress and --decompress are mutually exclusive");
            std::process::exit(1);
        }
        if args.hash.is_some() {
            eprintln!("error: --compress and --hash are mutually exclusive");
            std::process::exit(1);
        }
        if args.remote_name {
            eprintln!("error: --compress and -O/--remote-name are mutually exclusive");
            std::process::exit(1);
        }
        if args.upload_file.is_some() {
            eprintln!("error: --compress and -T/--upload-file are mutually exclusive");
            std::process::exit(1);
        }
        if args.data.is_some() {
            eprintln!("error: --compress and -d/--data are mutually exclusive");
            std::process::exit(1);
        }
        if args.editor.is_some() {
            eprintln!("error: --compress and --editor are mutually exclusive");
            std::process::exit(1);
        }
        if args.sample.is_some() {
            eprintln!("error: --compress and --sample are mutually exclusive");
            std::process::exit(1);
        }

        if let Err(err) = compression::run(&args) {
            if args.full_errors {
                eprintln!("error: {err:#}");
            } else {
                eprintln!("error: {}", friendly_message(&err));
            }
            std::process::exit(1);
        }
        return;
    }

    // ── Encode: list supported formats ───────────────────────────────────────
    if args.encode_list {
        if let Err(e) = encode::print_list(&mut std::io::stdout().lock()) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── Encode: QR / DataMatrix / 1D barcode ────────────────────────────────
    if args.encode.is_some() {
        // Mutual exclusions.
        if args.remote_name {
            eprintln!("error: --encode and -O/--remote-name are mutually exclusive");
            std::process::exit(1);
        }
        if args.hash.is_some() {
            eprintln!("error: --encode and --hash are mutually exclusive");
            std::process::exit(1);
        }
        if args.compress.is_some() || args.decompress.is_some() {
            eprintln!("error: --encode and --compress/--decompress are mutually exclusive");
            std::process::exit(1);
        }
        if args.sample.is_some() {
            eprintln!("error: --encode and --sample are mutually exclusive");
            std::process::exit(1);
        }
        if args.data.is_some() {
            eprintln!("error: --encode and -d/--data are mutually exclusive");
            std::process::exit(1);
        }
        if args.upload_file.is_some() {
            eprintln!("error: --encode and -T/--upload-file are mutually exclusive");
            std::process::exit(1);
        }
        if args.editor.is_some() {
            eprintln!("error: --encode and --editor are mutually exclusive");
            std::process::exit(1);
        }

        if let Err(err) = encode::run(&args) {
            if args.full_errors {
                eprintln!("error: {err:#}");
            } else {
                eprintln!("error: {}", friendly_message(&err));
            }
            std::process::exit(1);
        }
        return;
    }

    // ── Encrypt: generate a key pair (standalone action) ─────────────────────
    if args.encrypt_keygen {
        if let Err(err) = encrypt::run_keygen(&args) {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
        return;
    }

    // ── Encrypt / decrypt the input source ───────────────────────────────────
    if args.encrypt || args.decrypt {
        if args.encrypt && args.decrypt {
            eprintln!("error: --encrypt and --decrypt are mutually exclusive");
            std::process::exit(1);
        }
        if args.armor && args.decrypt {
            eprintln!("error: --armor only applies to --encrypt; --decrypt auto-detects");
            std::process::exit(1);
        }
        if args.remote_name {
            eprintln!("error: --encrypt and -O/--remote-name are mutually exclusive");
            std::process::exit(1);
        }
        if args.hash.is_some() {
            eprintln!("error: --encrypt and --hash are mutually exclusive");
            std::process::exit(1);
        }
        if args.compress.is_some() || args.decompress.is_some() {
            eprintln!("error: --encrypt and --compress/--decompress are mutually exclusive");
            std::process::exit(1);
        }
        if args.encode.is_some() {
            eprintln!("error: --encrypt and --encode are mutually exclusive");
            std::process::exit(1);
        }
        if args.sample.is_some() {
            eprintln!("error: --encrypt and --sample are mutually exclusive");
            std::process::exit(1);
        }
        if args.data.is_some() {
            eprintln!("error: --encrypt and -d/--data are mutually exclusive");
            std::process::exit(1);
        }
        if args.upload_file.is_some() {
            eprintln!("error: --encrypt and -T/--upload-file are mutually exclusive");
            std::process::exit(1);
        }
        if args.editor.is_some() {
            eprintln!("error: --encrypt and --editor are mutually exclusive");
            std::process::exit(1);
        }

        if let Err(err) = encrypt::run(&args) {
            if args.full_errors {
                eprintln!("error: {err:#}");
            } else {
                eprintln!("error: {}", friendly_message(&err));
            }
            std::process::exit(1);
        }
        return;
    }

    // ── --checkdigit-list ────────────────────────────────────────────────────
    if args.checkdigit_list {
        if args.checkdigit.is_some() || args.checkdigit_create.is_some() {
            eprintln!("recon: --checkdigit-list is standalone; do not combine with --checkdigit / --checkdigit-create");
            std::process::exit(2);
        }
        checkdigit::print_list();
        return;
    }

    // ── --checkdigit <NAME> ─────────────────────────────────────────────────
    if let Some(name) = &args.checkdigit.clone() {
        let mutex: &[(&str, bool)] = &[
            ("--hash", args.hash.is_some()),
            ("--compress", args.compress.is_some()),
            ("--decompress", args.decompress.is_some()),
            ("--encode", args.encode.is_some()),
            ("--encrypt", args.encrypt),
            ("--decrypt", args.decrypt),
            ("-O/--remote-name", args.remote_name),
            ("--sample", args.sample.is_some()),
            ("--editor", args.editor.is_some()),
            ("--checkdigit-create", args.checkdigit_create.is_some()),
        ];
        for (other, present) in mutex {
            if *present {
                eprintln!("recon: --checkdigit and {} are mutually exclusive", other);
                std::process::exit(2);
            }
        }
        match checkdigit::run_verify(name, &args) {
            Ok(()) => return,
            Err(e) => {
                eprintln!("recon: --checkdigit: {}", e);
                std::process::exit(1);
            }
        }
    }

    // ── --checkdigit-create <NAME> ──────────────────────────────────────────
    if let Some(name) = &args.checkdigit_create.clone() {
        let mutex: &[(&str, bool)] = &[
            ("--hash", args.hash.is_some()),
            ("--compress", args.compress.is_some()),
            ("--decompress", args.decompress.is_some()),
            ("--encode", args.encode.is_some()),
            ("--encrypt", args.encrypt),
            ("--decrypt", args.decrypt),
            ("-O/--remote-name", args.remote_name),
            ("--sample", args.sample.is_some()),
            ("--editor", args.editor.is_some()),
        ];
        for (other, present) in mutex {
            if *present {
                eprintln!("recon: --checkdigit-create and {} are mutually exclusive", other);
                std::process::exit(2);
            }
        }
        match checkdigit::run_create(name, &args) {
            Ok(()) => return,
            Err(e) => {
                eprintln!("recon: --checkdigit-create: {}", e);
                std::process::exit(1);
            }
        }
    }

    // ── Sample data: fetch / generate ────────────────────────────────────────
    if args.sample.is_some() {
        let result = run_sample(&args);
        if let Err(err) = result {
            if args.full_errors {
                eprintln!("error: {err:#}");
            } else {
                eprintln!("error: {}", friendly_message(&err));
            }
            std::process::exit(1);
        }
        return;
    }

    // ── Network status ───────────────────────────────────────────────────────
    if args.netstatus {
        let cfg = config::load();
        let cfg = match cfg {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        };
        let ns_config = match cfg.netstatus {
            Some(c) => c,
            None => {
                eprintln!("error: no [netstatus] section found in ~/.recon/config.toml");
                std::process::exit(1);
            }
        };
        if let Err(e) = ns_config.validate() {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        if let Err(e) = netstatus::run(&ns_config, args.silent) {
            if !args.silent {
                eprintln!("error: {e}");
            }
            std::process::exit(1);
        }
        return;
    }

    // ── Serve mode ───────────────────────────────────────────────────────────
    if args.has_serve() {
        if args.has_exclusive() || args.has_composable() {
            eprintln!("error: --serve and --serve-tls cannot be combined with other features");
            std::process::exit(1);
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let recon_dir = std::path::PathBuf::from(&home).join(".recon");

        let http_port = args.serve.as_ref().and_then(|p| p.parse::<u16>().ok());
        let https_port = args.serve_tls.as_ref().and_then(|p| p.parse::<u16>().ok())
            .or(if !args.serve_sni.is_empty() { Some(443) } else { None });

        let config = serve::ServeConfig {
            http_port,
            https_port,
            http_version: args.http_version.clone(),
            cert_path: args.serve_cert.clone().unwrap_or_else(|| recon_dir.join("cert.pem")),
            key_path: args.serve_key.clone().unwrap_or_else(|| recon_dir.join("key.pem")),
            log_file: args.serve_log.clone(),
            root_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            sni_mappings: args.serve_sni.clone(),
        };

        let result = serve::run(&config);
        if let Err(err) = result {
            if args.full_errors {
                eprintln!("error: {err:#}");
            } else {
                eprintln!("error: {}", friendly_message(&err));
            }
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

    // ── -O / -o mutual exclusion + filename substitution ─────────────────────
    let mut args = args;
    if args.remote_name && args.output.is_some() {
        eprintln!("error: -O/--remote-name and -o/--output are mutually exclusive");
        std::process::exit(1);
    }
    if args.remote_name && !args.remote_header_name {
        match util::filename_from_url(args.target_url()) {
            Ok(name) => {
                args.output = Some(std::path::PathBuf::from(name));
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }

    if args.upload_file.is_some() && args.data.is_some() {
        eprintln!("error: -T/--upload-file and -d/--data are mutually exclusive");
        std::process::exit(1);
    }

    if let Some(m) = args.max_time {
        if !m.is_finite() || m < 0.0 {
            eprintln!("error: --max-time must be a non-negative finite number");
            std::process::exit(2);
        }
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
    } else if args.target_url().starts_with("dict://") {
        dict_probe::run(args.target_url(), args.timeout)
    } else if let Some(rest) = dns_scheme_rest(args.target_url()) {
        let normalized = format!("dns://{rest}");
        parse_dns_url(&normalized).and_then(|(host, path_types)| {
            let types = if !args.dns_type.is_empty() {
                args.dns_type.clone()
            } else {
                path_types
            };
            dns::run(&host, &types)
        })
    } else if args.target_url().starts_with("file://") {
        file_url::run(args.target_url(), &args)
    } else if args.target_url().starts_with("mqtt://")
        || args.target_url().starts_with("mqtts://")
    {
        mqtt::run(args.target_url(), &args)
    } else if args.target_url().starts_with("ntp://") {
        ntp_probe::run(args.target_url(), args.timeout)
    } else if args.target_url().starts_with("ldap://")
        || args.target_url().starts_with("ldaps://")
    {
        ldap_probe::run(args.target_url(), args.timeout)
    } else if args.target_url().starts_with("memcached://") {
        memcached_probe::run(args.target_url(), args.timeout)
    } else if args.target_url().starts_with("redis://") {
        redis_probe::run(args.target_url(), args.timeout)
    } else if args.target_url().starts_with("ping://") {
        parse_plain_host(args.target_url())
            .and_then(|host| ping::run(&host, args.ping_count))
    } else if args.target_url().starts_with("rtsp://")
        || args.target_url().starts_with("rtsps://")
    {
        rtsp_probe::run(args.target_url(), args.insecure, args.timeout)
    } else if args.target_url().starts_with("scp://") {
        scp::download(args.target_url(), &args)
    } else if args.target_url().starts_with("ssh://") {
        ssh::connect(args.target_url(), &args)
    } else if args.target_url().starts_with("tcp://") {
        tcp_probe::run(args.target_url(), args.timeout)
    } else if args.target_url().starts_with("telnet://") {
        telnet::connect(args.target_url(), &args)
    } else if args.target_url().starts_with("tls://") {
        let rewritten = rewrite_tls_scheme(args.target_url());
        cert::fetch_and_print(&rewritten)
    } else if args.target_url().starts_with("traceroute://") {
        parse_plain_host(args.target_url())
            .and_then(|host| traceroute::run(&host, args.max_hops))
    } else if args.target_url().starts_with("udp://") {
        udp_probe::run(args.target_url(), &args)
    } else if args.target_url().starts_with("whois://") {
        parse_plain_host(args.target_url()).and_then(|host| whois::run(&host))
    } else if args.target_url().starts_with("ws://")
        || args.target_url().starts_with("wss://")
    {
        ws_probe::run(args.target_url(), args.timeout)
    } else {
        let t0 = std::time::Instant::now();
        client::execute(&args).and_then(|(response, mut metrics)| -> anyhow::Result<()> {
            if args.verbose >= 2 {
                eprintln!("* Elapsed: {:.3}s", t0.elapsed().as_secs_f64());
            }
            let result = if args.editor.is_some() {
                run_with_editor(response, &args, &mut metrics)
            } else {
                output::write_response(response, &args, &mut metrics)
            };
            // -w / --write-out: render after the response has been handled so
            // metrics.response_end and size_download are final.
            if let Some(fmt_arg) = &args.write_out {
                let format = writeout::load_format(fmt_arg)?;
                let tokens = writeout::parse(&format);
                writeout::render(&tokens, &metrics)?;
            }
            result
        })
    };

    if let Err(err) = result {
        if args.full_errors {
            eprintln!("error: {err:#}");
        } else {
            eprintln!("error: {}", friendly_message(&err));
        }
        std::process::exit(exit_code_for_http_error(&err));
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

/// Returns the curl-compatible exit code for a given error.
/// - 7   CURLE_COULDNT_CONNECT
/// - 28  CURLE_OPERATION_TIMEDOUT
/// - 67  CURLE_LOGIN_DENIED (MQTT auth failure)
/// - 130 Ctrl-C (SIGINT convention)
/// - 1   generic failure (default)
///
/// HTTP errors (reqwest) are matched by iterating the `StdError` chain and
/// downcasting. Protocol errors carry a `ProtocolExitCode` tag attached via
/// `anyhow::Error::context(...)`; anyhow's own `downcast_ref` sees through
/// context wrappers, while the `&dyn StdError` chain iterator does not —
/// so we must call `downcast_ref` on the `anyhow::Error` directly for the
/// tag, and fall back to the chain for the reqwest error.
fn exit_code_for_http_error(e: &anyhow::Error) -> i32 {
    // ProtocolExitCode implements StdError, so the anyhow chain exposes it —
    // search every frame (not just the top). Robust against future code that
    // adds another `.context(...)` after the tag, which would push the tag
    // out of the top slot.
    if let Some(code) = protocol_exit_code(e) {
        return code as i32;
    }
    for cause in e.chain() {
        if let Some(rq_err) = cause.downcast_ref::<reqwest::Error>() {
            if rq_err.is_timeout() {
                return 28;
            }
            if rq_err.is_connect() {
                return 7;
            }
        }
    }
    1
}

/// Find the first `ProtocolExitCode` tag anywhere in the error chain.
fn protocol_exit_code(e: &anyhow::Error) -> Option<crate::mqtt::ProtocolExitCode> {
    // Check the top-level anyhow error first — `.context(...)` wrappers
    // sometimes obscure the tag from the `.chain()` iterator.
    if let Some(c) = e.downcast_ref::<crate::mqtt::ProtocolExitCode>() {
        return Some(*c);
    }
    for cause in e.chain() {
        if let Some(c) = cause.downcast_ref::<crate::mqtt::ProtocolExitCode>() {
            return Some(*c);
        }
    }
    None
}

fn friendly_message(err: &anyhow::Error) -> String {
    // If the top-level anyhow error IS a ProtocolExitCode tag, its Display
    // impl ("exit-N") is not useful to the user. Skip to the wrapped
    // source to get the real "mqtt probe: ..." message. Using the typed
    // downcast (not a string-prefix check) means a future rename of the
    // Display impl won't silently leak the tag into user output.
    let msg = if protocol_exit_code(err).is_some()
        && err.downcast_ref::<crate::mqtt::ProtocolExitCode>().is_some()
    {
        err.source()
            .map(|s| s.to_string())
            .unwrap_or_else(|| err.to_string())
    } else {
        err.to_string()
    };
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
        || msg.starts_with("SSH URL missing")
        || msg.starts_with("Invalid SSH URL")
        || msg.starts_with("Telnet URL missing")
        || msg.starts_with("Invalid Telnet URL")
        || msg.starts_with("TLS certificate not found")
        || msg.starts_with("TLS private key not found")
        || msg.starts_with("--jwt-secret")
        || msg.starts_with("--jwt-validate requires")
        || msg.starts_with("--jwt-view, --jwt-sign")
        || msg.starts_with("Unsupported algorithm")
        || msg.starts_with("--jwt-validate-iss")
        || msg.starts_with("--jwt-validate-sub")
        || msg.starts_with("--jwt-validate-aud")
        || msg.starts_with("--jwt-validate-jti")
        || msg.starts_with("Could not parse input as")
        || msg.starts_with("No input provided")
        || msg.starts_with("file:")
        || msg.starts_with("dict:")
        || msg.starts_with("ldap:")
        || msg.starts_with("memcached:")
        || msg.starts_with("redis:")
        || msg.starts_with("rtsp:")
        || msg.starts_with("ws:")
        || msg.starts_with("tcp:")
        || msg.starts_with("udp:")
        || msg.starts_with("ntp:")
        || msg.starts_with("mqtt:")
        || msg.starts_with("mqtt probe")
        || msg.starts_with("mqtt publish")
        || msg.starts_with("mqtt subscribe")
        || msg.starts_with("unsupported scheme for mqtt URL")
        || msg.starts_with("malformed mqtt URL")
        || msg.starts_with("mqtt URL missing host")
        || msg == "interrupted"
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

fn run_jwt(args: &Args) -> anyhow::Result<()> {
    let ops = [args.jwt_view, args.jwt_sign, args.jwt_validate]
        .iter()
        .filter(|&&v| v)
        .count();
    if ops > 1 {
        anyhow::bail!("--jwt-view, --jwt-sign, and --jwt-validate are mutually exclusive");
    }
    if args.jwt_view {
        jwt::view(args)
    } else if args.jwt_sign {
        jwt::sign(args)
    } else {
        jwt::validate(args)
    }
}

/// Parse `protocol://host[:port]/...` → host only. Used by ping:// and
/// traceroute:// where the port is meaningless (ICMP).
/// Match `dns://`, `dig://`, `drill://` and return the rest (after `scheme://`).
fn dns_scheme_rest(url: &str) -> Option<&str> {
    for scheme in ["dns://", "dig://", "drill://"] {
        if let Some(rest) = url.strip_prefix(scheme) {
            return Some(rest);
        }
    }
    None
}

/// Parse `dns://host[/TYPE[,TYPE…]]`. Returns (host, types-from-path).
fn parse_dns_url(url: &str) -> anyhow::Result<(String, Vec<String>)> {
    use anyhow::Context;
    let parsed = url::Url::parse(url)
        .with_context(|| format!("malformed URL: {url}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL missing host: {url}"))?
        .to_string();
    let path = parsed.path().trim_start_matches('/');
    let types = if path.is_empty() {
        Vec::new()
    } else {
        path.split(',').map(|s| s.to_string()).collect()
    };
    Ok((host, types))
}

fn parse_plain_host(url: &str) -> anyhow::Result<String> {
    use anyhow::Context;
    let parsed = url::Url::parse(url)
        .with_context(|| format!("malformed URL: {url}"))?;
    parsed
        .host_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("URL missing host: {url}"))
}

/// Rewrite `tls://host[:port]/...` → `https://host[:port]/` so
/// cert::fetch_and_print (which only accepts https:// or bare host)
/// accepts the target.
fn rewrite_tls_scheme(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("tls://") {
        format!("https://{rest}")
    } else {
        url.to_string()
    }
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

fn run_with_editor(
    response: reqwest::blocking::Response,
    args: &Args,
    metrics: &mut metrics::RequestMetrics,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Resolve the editor spec up front so we fail fast if misconfigured.
    let flag_value = args.editor.as_deref().unwrap_or("");
    let (cfg_default, user_aliases) = load_editor_config();

    let resolved = editor::resolve_editor(flag_value, cfg_default.as_deref(), &user_aliases)
        .map_err(|_| anyhow::anyhow!(
            "--editor: no value given and no [editor] default in ~/.recon/config.toml"
        ))?;

    // Capture the extension from Content-Type BEFORE consuming the response.
    let extension = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(editor::extension_for_content_type)
        .unwrap_or("txt")
        .to_string();

    // Pick sink: buffer only by default, tee to stdout at -vv+.
    let mut sink = if args.verbose >= 2 {
        output::StdoutSink::Tee(Vec::new())
    } else {
        output::StdoutSink::Buffer(Vec::new())
    };

    output::write_response_to(response, args, &mut sink, metrics)?;
    let bytes = sink.into_bytes().unwrap_or_default();

    let path = editor::create_temp_file(&extension, &bytes)
        .context("failed to write editor temp file")?;

    if args.verbose >= 1 {
        eprintln!("* editor temp file: {}", path.display());
    }

    editor::spawn_editor(&resolved, &path)
        .with_context(|| format!("failed to launch editor for {}", path.display()))?;

    Ok(())
}

fn load_editor_config() -> (Option<String>, std::collections::HashMap<String, String>) {
    match config::load() {
        Ok(cfg) => match cfg.editor {
            Some(e) => (e.default, e.aliases),
            None => (None, std::collections::HashMap::new()),
        },
        // Missing or malformed config is not fatal for --editor: the flag can
        // still resolve built-in aliases and raw commands without it.
        Err(_) => (None, std::collections::HashMap::new()),
    }
}

/// Load `[sampledata.*]` from ~/.recon/config.toml. Returns an empty map
/// when the config file simply doesn't exist (fine — built-ins always
/// work). Propagates real parse/IO errors so the user sees them.
fn load_sampledata_config() -> anyhow::Result<std::collections::HashMap<String, config::SampleDataConfig>> {
    let path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".recon")
        .join("config.toml");
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }
    let cfg = config::load()?;
    Ok(cfg.sampledata)
}

fn run_sample(args: &Args) -> anyhow::Result<()> {
    use sampledata::SampleMode;

    let raw = args.sample.as_deref().unwrap_or("");
    let parsed = sampledata::parse_sample_arg(raw).map_err(|e| anyhow::anyhow!("{e}"))?;

    let format_override = args
        .sample_format
        .as_deref()
        .or(parsed.format.as_deref());
    let count_override = match &args.sample_count {
        Some(s) => Some(sampledata::parse_count(s).map_err(|e| anyhow::anyhow!("{e}"))?),
        None => parsed.count,
    };

    let cfg_map = load_sampledata_config()?;

    let resolved = sampledata::resolve(&parsed.name, format_override, count_override, &cfg_map)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if args.sample_seed.is_some() && resolved.spec.mode != sampledata::SampleMode::Local {
        anyhow::bail!("--sample-seed only applies to the lorem sample");
    }

    // Early guard: --sample-file and --output are mutually exclusive.
    if args.sample_file.is_some() && args.output.is_some() {
        anyhow::bail!("-o and --sample-file are mutually exclusive");
    }

    match resolved.spec.mode {
        SampleMode::Local => run_sample_local(&resolved, args),
        SampleMode::Bulk => run_sample_bulk(&resolved, args),
        SampleMode::PerItem => run_sample_per_item(&resolved, args),
    }
}

fn run_sample_local(resolved: &sampledata::ResolvedSample, args: &Args) -> anyhow::Result<()> {
    use anyhow::Context;

    let seed = args.sample_seed.unwrap_or_else(seed_from_clock);
    let bytes = crate::lorem::generate(resolved.count, seed).into_bytes();

    if let Some(sf) = &args.sample_file {
        let path = resolve_sample_file_path(sf, &resolved.name, &resolved.format, None)?;
        std::fs::write(&path, &bytes)
            .with_context(|| format!("failed to write {}", path.display()))?;
        if !args.silent {
            eprintln!("Saved to {}", path.display());
        }
        return Ok(());
    }

    if args.editor.is_some() {
        let (cfg_default, user_aliases) = load_editor_config();
        let flag_value = args.editor.as_deref().unwrap_or("");
        let ed = editor::resolve_editor(flag_value, cfg_default.as_deref(), &user_aliases)
            .map_err(|_| anyhow::anyhow!(
                "--editor: no value given and no [editor] default in ~/.recon/config.toml"
            ))?;
        let path = editor::create_temp_file(&resolved.format, &bytes)
            .context("failed to write editor temp file")?;
        editor::spawn_editor(&ed, &path)
            .with_context(|| format!("failed to launch editor for {}", path.display()))?;
        return Ok(());
    }

    std::io::Write::write_all(&mut std::io::stdout(), &bytes)?;
    Ok(())
}

/// Resolve a `--sample-file` template to an actual filesystem path.
/// `iteration` is `Some(i)` in per_item mode, `None` otherwise.
fn resolve_sample_file_path(
    template: &str,
    name: &str,
    format: &str,
    iteration: Option<u32>,
) -> anyhow::Result<std::path::PathBuf> {
    let tpl = if template.is_empty() {
        // Default template: per_item uses {{n}}, others do not.
        if iteration.is_some() {
            "sample-{{name}}-{{n}}.{{format}}".to_string()
        } else {
            "sample-{{name}}.{{format}}".to_string()
        }
    } else {
        template.to_string()
    };

    let mut vars: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    vars.insert("name", name.to_string());
    vars.insert("format", format.to_string());
    if let Some(i) = iteration {
        vars.insert("n", i.to_string());
    }
    let s = sampledata::expand_template(&tpl, &vars).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(std::path::PathBuf::from(s))
}

fn run_sample_bulk(resolved: &sampledata::ResolvedSample, args: &Args) -> anyhow::Result<()> {
    use anyhow::Context;

    if resolved.spec.count_ignored && args.sample_count.is_some() {
        eprintln!("warning: --sample-count ignored for sample '{}'", resolved.name);
    }

    let url = sampledata::expand_sample_url(resolved, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if args.verbose >= 1 {
        eprintln!("* sample: {} {} ({} mode, format={})",
            resolved.name, url, mode_label(resolved.spec.mode), resolved.format);
    }

    let client = sampledata::build_client(args.timeout, args.insecure)?;
    let req = sampledata::build_request(&client, resolved, &url, args.timeout)?;
    let response = req.send().context("sample fetch failed")?;

    // --sample-file path: save response bytes to the templated filename and stop.
    if let Some(sf) = &args.sample_file {
        let path = resolve_sample_file_path(sf, &resolved.name, &resolved.format, None)?;
        let bytes = response.bytes().context("failed to read sample response")?;
        std::fs::write(&path, &bytes)
            .with_context(|| format!("failed to write {}", path.display()))?;
        if !args.silent {
            eprintln!("Saved to {}", path.display());
        }
        return Ok(());
    }

    // --editor path: buffer bytes, extension from sample format (not Content-Type).
    if args.editor.is_some() {
        let (cfg_default, user_aliases) = load_editor_config();
        let flag_value = args.editor.as_deref().unwrap_or("");
        let ed = editor::resolve_editor(flag_value, cfg_default.as_deref(), &user_aliases)
            .map_err(|_| anyhow::anyhow!(
                "--editor: no value given and no [editor] default in ~/.recon/config.toml"
            ))?;
        let mut sink = if args.verbose >= 2 {
            output::StdoutSink::Tee(Vec::new())
        } else {
            output::StdoutSink::Buffer(Vec::new())
        };
        // Sample paths don't participate in -w rendering, so a throwaway
        // RequestMetrics is acceptable here (shortcut).
        let mut dummy = metrics::RequestMetrics::default();
        output::write_response_to(response, args, &mut sink, &mut dummy)?;
        let bytes = sink.into_bytes().unwrap_or_default();
        let path = editor::create_temp_file(&resolved.format, &bytes)
            .context("failed to write editor temp file")?;
        editor::spawn_editor(&ed, &path)
            .with_context(|| format!("failed to launch editor for {}", path.display()))?;
        return Ok(());
    }

    // Default: route through the normal output pipeline.
    // Sample paths don't participate in -w rendering; use a throwaway metrics.
    let mut dummy = metrics::RequestMetrics::default();
    output::write_response(response, args, &mut dummy)
}

fn run_sample_per_item(
    resolved: &sampledata::ResolvedSample,
    args: &Args,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let count = resolved.count.n;

    // Editor + count > 1 is unsupported.
    if args.editor.is_some() && count > 1 {
        anyhow::bail!("--editor with per_item sample requires count == 1");
    }

    // --sample-file is required when count > 1.
    if count > 1 && args.sample_file.is_none() {
        anyhow::bail!(
            "--sample-file required for per_item sample '{}' with count > 1",
            resolved.name
        );
    }

    // Filename must include {{n}} when count > 1. Empty template means
    // "use the default", which already contains {{n}} in per_item mode.
    if count > 1 {
        let sf = args.sample_file.as_deref().unwrap();
        if !sf.is_empty() && !sf.contains("{{n}}") {
            anyhow::bail!(
                "--sample-file '{sf}' must include {{{{n}}}} when count > 1"
            );
        }
    }

    let client = sampledata::build_client(args.timeout, args.insecure)?;

    for i in 1..=count {
        let url = sampledata::expand_sample_url(resolved, Some(i))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        if args.verbose >= 1 {
            eprintln!("* fetching {i}/{count} ({}): {url}", resolved.name);
        }

        let req = sampledata::build_request(&client, resolved, &url, args.timeout)?;
        let response = req
            .send()
            .with_context(|| format!("sample fetch failed at iteration {i}/{count}"))?;
        let bytes = response
            .bytes()
            .with_context(|| format!("failed to read response at iteration {i}/{count}"))?;

        // Destination: --sample-file template (with {{n}} when count > 1), or
        // stdout if count == 1 and no --sample-file.
        match (count, &args.sample_file) {
            (1, None) => {
                if args.editor.is_some() {
                    let (cfg_default, user_aliases) = load_editor_config();
                    let flag_value = args.editor.as_deref().unwrap_or("");
                    let ed = editor::resolve_editor(
                        flag_value,
                        cfg_default.as_deref(),
                        &user_aliases,
                    ).map_err(|_| anyhow::anyhow!(
                        "--editor: no value given and no [editor] default in ~/.recon/config.toml"
                    ))?;
                    let path = editor::create_temp_file(&resolved.format, &bytes)
                        .context("failed to write editor temp file")?;
                    editor::spawn_editor(&ed, &path)
                        .with_context(|| format!("failed to launch editor for {}", path.display()))?;
                } else {
                    std::io::Write::write_all(&mut std::io::stdout(), &bytes)?;
                }
            }
            (_, Some(sf)) => {
                let path = resolve_sample_file_path(
                    sf,
                    &resolved.name,
                    &resolved.format,
                    Some(i),
                )?;
                std::fs::write(&path, &bytes)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                if !args.silent {
                    eprintln!("Saved to {}", path.display());
                }
            }
            _ => unreachable!("count > 1 without --sample-file is rejected above"),
        }
    }
    Ok(())
}

fn mode_label(mode: sampledata::SampleMode) -> &'static str {
    match mode {
        sampledata::SampleMode::Bulk => "bulk",
        sampledata::SampleMode::PerItem => "per_item",
        sampledata::SampleMode::Local => "local",
    }
}

fn print_sample_list(entries: &[sampledata::SampleListEntry]) {
    use sampledata::{SampleMode, SampleSource};

    for e in entries {
        let mode = match e.mode {
            SampleMode::Bulk => "bulk",
            SampleMode::PerItem => "per_item",
            SampleMode::Local => "local",
        };
        let tag = match e.source_tag {
            SampleSource::BuiltIn => "[built-in]",
            SampleSource::Config => "[config]",
            SampleSource::Overridden => "[overridden]",
        };
        println!("{}    {}", e.name, e.description);
        println!(
            "            mode={mode}   default format={}   formats={}",
            e.default_format,
            e.formats.join(","),
        );
        println!(
            "            default count={}                           source={tag}",
            e.count,
        );
        println!();
    }
}

fn seed_from_clock() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .ok()
        .filter(|&n| n != 0)
        .unwrap_or(1)
}
