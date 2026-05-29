#![doc = include_str!("../README.md")]

mod cert;
mod cli;
mod client;
mod client_cert;
#[cfg(feature = "impersonate")]
mod impersonate;
mod clipboard;
mod compare;
mod compression;
mod decode;
mod docs;
mod docs_pdf;
mod flaglist;
mod config;
mod config_file;
mod config_resolver;
mod aliases;
mod cookiejar;
mod dict_probe;
mod dns;
mod dns_resolver;
mod editor;
mod email;
mod encode;
mod checkdigit;
mod encrypt;
mod examples;
mod fail;
mod file_url;
mod agent_browser;
mod archive;
mod hash;
mod help;
mod iface;
mod hsts;
mod init;
mod input_file;
mod pager;
mod ratelimit;
mod jwt;
mod ldap_probe;
mod lorem;
mod memcached_probe;
mod metrics;
mod mqtt;
mod netrc;
mod netstatus;
mod ntp_probe;
mod output;
mod pdf_export;
mod ping;
mod prettify;
mod proto_filter;
mod proxy;
mod redis_probe;
mod repl;
mod remote_name;
mod retry;
mod rtsp_probe;
mod writeout;
mod sampledata;
mod script;
mod scp;
mod serve;
mod source;
mod ssh;
mod ssh_auth;
mod ftp_probe;
mod gopher_probe;
mod imap_probe;
mod ipfs;
mod pop3_probe;
mod sftp_probe;
mod smtp_probe;
mod tcp_probe;
mod tftp_probe;
mod telnet;
mod text_encoding;
mod iconv;
mod tls_probe;
mod traceroute;
mod udp_probe;
mod unix_socket;
mod util;
mod version;
mod wget_filter;
mod whois;
mod ws_probe;

use clap::CommandFactory;
use cli::Args;

#[derive(Debug, PartialEq, Eq)]
enum ClipboardDir {
    In,
    Out,
    Both,
}

/// True when any flag is set that runs recon without needing a URL.
/// Mirrors what was previously in clap's `required_unless_present_any`.
fn any_no_url_mode_flag(args: &cli::Args) -> bool {
    args.url_flag.is_some()
        || args.cookies
        || args.cookie_delete.is_some()
        || args.cookie_set.is_some()
        || args.spf
        || args.dmarc
        || !args.dkim.is_empty()
        || args.mta_sts
        || args.bimi.is_some()
        || args.tls_rpt
        || args.serve.is_some()
        || args.serve_tls.is_some()
        || !args.serve_sni.is_empty()
        || args.jwt_view
        || args.jwt_sign
        || args.jwt_validate
        || args.netstatus
        || args.editor_cleanup
        || args.sample.is_some()
        || args.sample_list
        || args.hash.is_some()
        || args.hash_list
        || args.compress.is_some()
        || args.decompress.is_some()
        || args.compress_list
        || args.encode.is_some()
        || args.encode_list
        || args.encrypt
        || args.decrypt
        || args.encrypt_keygen
        || args.checkdigit.is_some()
        || args.checkdigit_create.is_some()
        || args.checkdigit_list
        || args.script.is_some()
        || args.init
        || args.browser_screenshot.is_some()
        || args.archive.is_some()
        || args.extract.is_some()
        || args.iconv.is_some()
        || args.list_charsets
        || args.compare.is_some()
        || args.decode.is_some()
        || args.decode_all.is_some()
        || args.md_to_html.is_some()
        || args.md_to_pdf.is_some()
        || args.html_to_pdf.is_some()
        || args.export_pdf_page.is_some()
        || args.input_file.is_some()
        || args.repl
        || args.show_config_paths
}

fn resolve_clipboard(args: &cli::Args) -> Option<ClipboardDir> {
    let raw = args.clipboard.as_deref()?;
    match raw {
        "in" => Some(ClipboardDir::In),
        "out" => Some(ClipboardDir::Out),
        "both" => Some(ClipboardDir::Both),
        "auto" => {
            let has_input = args.url.is_some()
                || args.url_flag.is_some()
                || args.stdin
                || args.from_clipboard;
            Some(if has_input { ClipboardDir::Out } else { ClipboardDir::In })
        }
        other => {
            eprintln!("error: --clipboard expects in|out|both, got '{other}'");
            std::process::exit(2);
        }
    }
}

fn main() {
    // ── SIGPIPE → default (terminate) ────────────────────────────────────────
    // Rust's std installs SIG_IGN for SIGPIPE on Unix so the first `write` to
    // a closed pipe surfaces as `BrokenPipe`, which `println!` then turns into
    // a panic. For a CLI that pipes long output (--examples, --flags, --help)
    // into pagers / `head`, that panic is just noise. Restoring SIG_DFL means
    // the reader closing causes a clean exit, the same as `cat /dev/urandom`
    // behaves when piped to `head`.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // ── --help [topic] interception (before clap parses) ─────────────────────
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--help" || a == "-h") {
            // Spawn the pager BEFORE emitting any output, so every println!
            // in help::* flows through it. Keep the Child until we call
            // finish() — that's what blocks until the user quits less.
            let pager_child = pager::activate(pager::no_pager_requested());
            let next = args.get(pos + 1);
            match next {
                Some(topic) if !topic.starts_with('-') => {
                    if !help::print_topic(topic) {
                        help::print_unknown_topic(topic);
                    }
                }
                _ => {
                    // Clap's own ANSI colouring auto-strips when stdout
                    // isn't a TTY. After activate()'s dup2, our stdout
                    // is a pipe — clap sees non-TTY and would emit mono.
                    // Force Always when paging so less -R gets real
                    // escape codes to render.
                    let mut cmd = Args::command();
                    if pager_child.is_some() {
                        cmd = cmd.color(clap::ColorChoice::Always);
                    }
                    let _ = cmd.print_help();
                    println!();
                    help::print_topic_footer();
                }
            }
            pager::finish(pager_child);
            return;
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
        let pager_child = pager::activate(pager::no_pager_requested());
        examples::print();
        pager::finish(pager_child);
        return;
    }

    // --flags lists every flag alphabetically (curl --help all style).
    // Same early-intercept treatment so clap doesn't demand a URL first.
    if std::env::args().any(|a| a == "--flags") {
        let pager_child = pager::activate(pager::no_pager_requested());
        flaglist::print_flags_listing();
        pager::finish(pager_child);
        return;
    }

    // Pre-split argv on `--script PATH` so trailing positional args after
    // the script path become `script_args` instead of being assigned to the
    // positional `url` by clap. Non-script invocations are unaffected.
    // Pre-expand -K/--config files into argv before clap parses. The
    // expanded tokens are spliced in at the -K position.
    let raw_argv: Vec<String> = std::env::args().collect();
    let expanded_argv = match config_file::expand_config_in_argv(raw_argv) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    };

    // Pre-clap pass: load the layered config (without yet honoring
    // CLI skip flags — those land via init_global later) so we can
    // resolve `--alias <name>` or `[aliases] default` against it and
    // rewrite short flags in argv before clap parses. If the user
    // disabled all config via `-q/--disable` or skipped both layers,
    // an empty TOML table is used; the alias step then becomes a
    // no-op unless `--alias` is explicit.
    let expanded_argv = {
        let disable_all = expanded_argv.iter().any(|t| t == "-q" || t == "--disable");
        let no_sys = expanded_argv.iter().any(|t| t == "--no-system-config");
        let no_usr = expanded_argv.iter().any(|t| t == "--no-user-config");
        let layered = if disable_all || (no_sys && no_usr) {
            toml::Value::Table(toml::value::Table::new())
        } else {
            let opts = config_resolver::LayerOpts::from_env()
                .merge_cli_flags(disable_all, no_sys, no_usr);
            match config_resolver::load_layered("config.toml", &opts) {
                Ok(v) => v,
                Err(_) => toml::Value::Table(toml::value::Table::new()),
            }
        };
        match aliases::apply_from_argv(expanded_argv, &layered) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: {e:#}");
                std::process::exit(2);
            }
        }
    };

    let mut args = match Args::parse_with_script_split(expanded_argv) {
        Ok(a) => a,
        Err(e) => {
            e.exit();
        }
    };

    // ── --editor URL-grabbing rescue ──
    // clap's num_args = 0..=1 on --editor greedily eats the next token, so
    // `recon --editor https://example.com` lands the URL on `--editor` and
    // leaves the positional `url` empty. If the editor value contains `://`
    // it's a URL, not an editor command — swap it onto `url` and let
    // `--editor` fall back to the configured default.
    if args.url.is_none() {
        if let Some(val) = args.editor.as_deref() {
            if val.contains("://") {
                args.url = Some(val.to_string());
                args.editor = Some(String::new());
            }
        }
    }

    // ── --tries validation ──
    if matches!(args.tries, Some(0)) {
        eprintln!("error: --tries: N must be ≥ 1 (use --retry-max-time as a ceiling for many retries)");
        std::process::exit(2);
    }

    // ── --prettify-as implies -p; validate format name early ──
    if args.prettify_as.is_some() {
        args.prettify = true;
    }
    if let Some(s) = &args.prettify_as {
        if let Err(e) = prettify::parse_format(s) {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }

    // --clipboard <DIR> resolution: populate the underlying flags.
    match resolve_clipboard(&args) {
        Some(ClipboardDir::In) => args.from_clipboard = true,
        Some(ClipboardDir::Out) => args.to_clipboard = true,
        Some(ClipboardDir::Both) => {
            args.from_clipboard = true;
            args.to_clipboard = true;
        }
        None => {}
    }

    // Auto-detect stdin: if no input source is given and stdin is piped,
    // treat as implicit --stdin.
    {
        use std::io::IsTerminal;
        let no_input = args.url.is_none()
            && !args.stdin
            && !args.from_clipboard
            && !any_no_url_mode_flag(&args);
        if no_input {
            if !std::io::stdin().is_terminal() {
                args.stdin = true;
            } else {
                eprintln!("error: missing URL or input flag (try --help)");
                std::process::exit(2);
            }
        }
    }

    // Output-sink mutex: --to-clipboard conflicts with -o and --editor.
    if args.to_clipboard && args.output.is_some() {
        eprintln!("error: --to-clipboard and -o/--output are mutually exclusive");
        std::process::exit(2);
    }
    if args.to_clipboard && args.editor.is_some() {
        eprintln!("error: --to-clipboard and --editor are mutually exclusive");
        std::process::exit(2);
    }

    // ── --input-file: batch URL fetch ──
    if let Some(path) = args.input_file.clone() {
        match input_file::load_urls(&path) {
            Ok(urls) => {
                // --wait wins over --rate when both are set.
                let inter_url_delay: Option<std::time::Duration> = if let Some(secs) = args.wait {
                    Some(std::time::Duration::from_secs(secs))
                } else {
                    args.rate
                        .as_deref()
                        .map(input_file::parse_rate)
                        .transpose()
                        .unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            std::process::exit(2);
                        })
                };

                // --accept / --reject suffix filter (wget-compat).
                let accept = args.accept.as_deref();
                let reject = args.reject.as_deref();
                let mut rejected = 0usize;
                let kept: Vec<String> = urls
                    .into_iter()
                    .filter(|u| {
                        if wget_filter::should_keep(u, accept, reject) {
                            true
                        } else {
                            if !args.silent {
                                eprintln!("# skip (filter): {u}");
                            }
                            rejected += 1;
                            false
                        }
                    })
                    .collect();

                let mut any_err = rejected > 0;
                for (i, url) in kept.iter().enumerate() {
                    if let Some(d) = inter_url_delay {
                        if i > 0 {
                            std::thread::sleep(d);
                        }
                    }
                    let mut per = args.clone();
                    per.input_file = None;
                    per.url = Some(url.clone());
                    per.url_flag = None;
                    if per.remote_name_all {
                        per.remote_name = true;
                    }
                    if !args.silent {
                        eprintln!("# {} ({}/{})", url, i + 1, kept.len());
                    }
                    match retry::execute_with_retry(&per) {
                        Ok((response, mut metrics)) => {
                            if per.spider {
                                let status = response.status().as_u16();
                                println!("{status} {url}");
                                if !response.status().is_success() {
                                    any_err = true;
                                }
                            } else if let Err(e) = output::write_response(response, &per, &mut metrics) {
                                eprintln!("  error: {e}");
                                any_err = true;
                            }
                        }
                        Err(e) => {
                            eprintln!("  error: {e}");
                            any_err = true;
                        }
                    }
                }
                if any_err {
                    std::process::exit(1);
                }
                return;
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(2);
            }
        }
    }

    // ── --proto-default: scheme injection for scheme-less URLs ──
    if let Some(scheme) = args.proto_default.clone() {
        for url_field in [&mut args.url, &mut args.url_flag] {
            if let Some(raw) = url_field.clone() {
                let rewritten = proto_filter::apply_default_scheme(&raw, Some(&scheme));
                if rewritten != raw {
                    *url_field = Some(rewritten);
                }
            }
        }
    }

    // ── --proto: allow-list check ──
    if let Some(spec) = &args.proto {
        match proto_filter::ProtoFilter::parse(spec) {
            Ok(filter) => {
                let url = args.target_url();
                if !url.is_empty() {
                    if let Err(e) = filter.validate_url(url) {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("error: --proto: {e}");
                std::process::exit(1);
            }
        }
    }

    // ── --accept / --reject: single-URL suffix filter (wget-compat) ──
    if args.accept.is_some() || args.reject.is_some() {
        let url = args.target_url();
        if !url.is_empty()
            && !wget_filter::should_keep(url, args.accept.as_deref(), args.reject.as_deref())
        {
            eprintln!("error: URL rejected by --accept/--reject filter: {url}");
            std::process::exit(1);
        }
    }

    // ── --stderr redirect (early so all subsequent error output routes through it) ──
    if let Some(path) = args.stderr_file.clone() {
        use std::os::unix::io::AsRawFd;
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(f) => {
                #[cfg(unix)]
                unsafe {
                    libc::dup2(f.as_raw_fd(), libc::STDERR_FILENO);
                }
                // Intentionally leak the File — stderr now owns the fd.
                std::mem::forget(f);
            }
            Err(e) => eprintln!("warning: --stderr: {e}"),
        }
    }

    // ── ipfs:// / ipns:// URL rewrite ─────────────────────────────────────────
    // Rewrite before any protocol dispatch so the URL flows through the
    // existing HTTP path. Gateway defaults to https://ipfs.io; override
    // via --ipfs-gateway or $RECON_IPFS_GATEWAY.
    if let Some(raw) = args.url.clone() {
        if let Some(rewritten) = ipfs::rewrite_url(&raw, args.ipfs_gateway.as_deref()) {
            args.url = Some(rewritten);
        }
    }
    if let Some(raw) = args.url_flag.clone() {
        if let Some(rewritten) = ipfs::rewrite_url(&raw, args.ipfs_gateway.as_deref()) {
            args.url_flag = Some(rewritten);
        }
    }

    // ── HSTS upgrade (http:// → https:// for cached hosts) ────────────────────
    if let Some(hsts_path) = args.hsts.clone() {
        match hsts::HstsStore::load(&hsts_path) {
            Ok(store) => {
                for url_field in [&mut args.url, &mut args.url_flag] {
                    if let Some(raw) = url_field.clone() {
                        if let Some(stripped) = raw.strip_prefix("http://") {
                            let host = stripped.split('/').next().unwrap_or(stripped);
                            let host = host.split(':').next().unwrap_or(host);
                            if store.matches(host) {
                                if !args.silent {
                                    eprintln!("* HSTS: upgrading http:// to https:// for {host}");
                                }
                                *url_field = Some(format!("https://{stripped}"));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("warning: --hsts: {e}");
            }
        }
    }

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

    // ── Archive: create (no HTTP request needed) ─────────────────────────────
    if args.archive.is_some() {
        if let Err(e) = archive::run_archive_cli(&args) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── Archive: extract (no HTTP request needed) ────────────────────────────
    if args.extract.is_some() {
        if let Err(e) = archive::run_extract_cli(&args) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── Document conversions: markdown / HTML → HTML / PDF ──────────────────
    if args.md_to_html.is_some() {
        if let Err(e) = docs::run_md_to_html(&args) {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }
    if args.md_to_pdf.is_some() {
        if let Err(e) = docs::run_md_to_pdf(&args) {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }
    if args.html_to_pdf.is_some() {
        if let Err(e) = docs::run_html_to_pdf(&args) {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }
    if args.export_pdf_page.is_some() {
        if let Err(e) = pdf_export::run_export_pdf_page_cli(&args) {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }

    // ── Decode a barcode image (no HTTP request needed) ─────────────────────
    if args.decode.is_some() {
        if let Err(e) = decode::run(&args) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }
    if args.decode_all.is_some() {
        if let Err(e) = decode::run_all(&args) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── Compare two sources (no HTTP request needed unless one is http(s)) ──
    if args.compare.is_some() {
        match compare::run(&args) {
            Ok(verdict) => std::process::exit(verdict.exit_code()),
            Err(e) => {
                eprintln!("error: {e:#}");
                std::process::exit(2);
            }
        }
    }

    // ── Browser screenshot convenience (no HTTP request needed) ──────────────
    if let Some(url) = args.browser_screenshot.clone() {
        let output = args.output.as_deref();
        if let Err(e) = agent_browser::run_screenshot_cli(&url, output) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── List charsets (no HTTP request needed) ───────────────────────────────
    if args.list_charsets {
        for label in text_encoding::common_labels() {
            println!("{label}");
        }
        return;
    }

    // ── Standalone iconv mode (no HTTP request needed) ───────────────────────
    if args.iconv.is_some() {
        let code = iconv::run_cli(&args);
        std::process::exit(code);
    }

    // ── Payload mode (stdin OR clipboard, no HTTP request needed) ────────────
    if args.stdin || args.from_clipboard {
        if args.stdin && args.from_clipboard {
            eprintln!("error: --stdin and --from-clipboard are mutually exclusive");
            std::process::exit(2);
        }
        if args.url.is_some() || args.url_flag.is_some() {
            eprintln!("error: --stdin/--from-clipboard and a URL are mutually exclusive");
            std::process::exit(2);
        }
        let code = run_payload_mode(&args);
        std::process::exit(code);
    }

    // ── Init: bootstrap ~/.recon/ layout (no HTTP request needed) ────────────
    if args.init {
        if let Err(e) = init::run() {
            eprintln!("error: {e}");
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

    // ── Encrypt / decrypt / rekey the input source ───────────────────────────
    if args.encrypt || args.decrypt || args.rekey {
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

    // ── Layered config resolver: wire CLI flags into the global OnceLock ────
    {
        let opts = crate::config_resolver::LayerOpts::from_env()
            .merge_cli_flags(args.disable_default_config, args.no_system_config, args.no_user_config);
        crate::config_resolver::init_global(opts);
    }

    // ── Show which config files the resolver picked ───────────────────────────
    if args.show_config_paths {
        let opts = crate::config_resolver::global();
        let resolved = crate::config_resolver::resolve_paths("config.toml", &opts);

        fn show(label: &str, p: &Option<std::path::PathBuf>, skipped: bool, why_none: &str) {
            match (skipped, p) {
                (true, _) => println!("{label}: (skipped)"),
                (_, Some(p)) => println!("{label}: {}", p.display()),
                (_, None) => println!("{label}: (none — {why_none})"),
            }
        }

        #[cfg(target_os = "macos")]
        let system_why = "no candidate in [$HOMEBREW_PREFIX/etc/recon, /opt/homebrew/etc/recon, /usr/local/etc/recon, /etc/recon] exists";
        #[cfg(not(target_os = "macos"))]
        let system_why = "/etc/recon/config.toml does not exist";

        show("system", &resolved.system, opts.skip_system, system_why);
        show("user",   &resolved.user,   opts.skip_user,
             "$HOME unset or ~/.recon/config.toml does not exist");

        let env = |k: &str| std::env::var(k).unwrap_or_else(|_| "(unset)".to_string());
        println!("$RECON_SYSTEM_CONFIG: {}", env("RECON_SYSTEM_CONFIG"));
        println!("$RECON_CONFIG:        {}", env("RECON_CONFIG"));
        println!("$HOMEBREW_PREFIX:     {}", env("HOMEBREW_PREFIX"));

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

    // ── REPL mode ────────────────────────────────────────────────────────────
    // `--repl` opens an interactive prompt backed by the script engine.
    // Mutually exclusive with --script (this routing order makes --repl win
    // if both are given).
    if args.repl {
        std::process::exit(repl::run(&args));
    }

    // ── Script mode ──────────────────────────────────────────────────────────
    // `--script PATH.rhai` runs an embedded Rhai script against the recon
    // probe API and exits with the script's return code. Mutually exclusive
    // with URL-based dispatch.
    if args.script.is_some() {
        std::process::exit(script::run(&args));
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
        redis_probe::run(args.target_url(), &args)
    } else if args.target_url().starts_with("ping://") {
        parse_plain_host(args.target_url())
            .and_then(|host| ping::run(&host, args.ping_count))
    } else if args.target_url().starts_with("rtsp://")
        || args.target_url().starts_with("rtsps://")
    {
        rtsp_probe::run(args.target_url(), args.insecure, args.timeout)
    } else if args.target_url().starts_with("smtp://")
        || args.target_url().starts_with("smtps://")
    {
        smtp_probe::run(args.target_url(), &args)
    } else if args.target_url().starts_with("ftp://")
        || args.target_url().starts_with("ftps://")
    {
        let fargs = ftp_probe::FtpArgs {
            user: args.user.as_deref().and_then(|s| s.split_once(':').map(|(u, _)| u)),
            pass: args.user.as_deref().and_then(|s| s.split_once(':').map(|(_, p)| p)),
            passive: !args.ftp_active,
            implicit_tls: args.ftps_implicit,
            insecure: args.insecure,
            timeout_secs: args.timeout,
            list_only: args.list_only,
            quote: args.quote.clone(),
            ftp_skip_pasv_ip: args.ftp_skip_pasv_ip,
            disable_epsv: args.disable_epsv,
            disable_eprt: args.disable_eprt,
            ftp_pasv: args.ftp_pasv,
            verbose: args.verbose,
        };
        ftp_probe::run(args.target_url(), &fargs, args.output.as_deref())
    } else if args.target_url().starts_with("sftp://") {
        sftp_probe::run(args.target_url(), &args)
    } else if args.target_url().starts_with("tftp://") {
        if args.tftp_no_options && args.verbose >= 1 {
            eprintln!("* TFTP: vanilla RFC 1350 mode (no RFC 2347 options) — --tftp-no-options confirmed");
        }
        tftp_probe::run(args.target_url(), args.timeout, args.tftp_blksize)
    } else if args.target_url().starts_with("gopher://")
        || args.target_url().starts_with("gophers://")
    {
        gopher_probe::run(args.target_url(), args.timeout, args.insecure)
    } else if args.target_url().starts_with("pop3://")
        || args.target_url().starts_with("pop3s://")
    {
        let pargs = pop3_probe::Pop3Args {
            user: args.user.as_deref().and_then(|s| s.split_once(':').map(|(u, _)| u)),
            pass: args.user.as_deref().and_then(|s| s.split_once(':').map(|(_, p)| p)),
            stls: args.stls,
            insecure: args.insecure,
            timeout_secs: args.timeout,
        };
        pop3_probe::run(args.target_url(), &pargs)
    } else if args.target_url().starts_with("imap://")
        || args.target_url().starts_with("imaps://")
    {
        let iargs = imap_probe::ImapArgs {
            user: args.user.as_deref().and_then(|s| s.split_once(':').map(|(u, _)| u)),
            pass: args.user.as_deref().and_then(|s| s.split_once(':').map(|(_, p)| p)),
            insecure: args.insecure,
            peek: args.imap_peek,
        };
        imap_probe::run(args.target_url(), &iargs)
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
    } else if args.unix_socket.is_some() {
        unix_socket::run(&args)
    } else {
        let t0 = std::time::Instant::now();
        retry::execute_with_retry(&args).and_then(|(response, mut metrics)| -> anyhow::Result<()> {
            if args.verbose >= 2 {
                eprintln!("* Elapsed: {:.3}s", t0.elapsed().as_secs_f64());
            }
            // --spider: print "<status> <url>" and exit. No body.
            if args.spider {
                let status = response.status().as_u16();
                let url = response.url().to_string();
                println!("{status} {url}");
                metrics.response_end = Some(std::time::Instant::now());
                metrics.size_download = 0;
                if !response.status().is_success() {
                    anyhow::bail!("--spider: {status} for {url}");
                }
                return Ok(());
            }
            let result = output::write_response(response, &args, &mut metrics);
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

fn run_payload_mode(args: &cli::Args) -> i32 {
    use std::io::Read;

    let buf: Vec<u8> = if args.from_clipboard {
        match clipboard::read_text() {
            Ok(s) => s.into_bytes(),
            Err(e) => {
                if args.full_errors {
                    eprintln!("error: {e:#}");
                } else {
                    eprintln!("error: {}", friendly_message(&e));
                }
                return 1;
            }
        }
    } else {
        let mut buf = Vec::new();
        if let Err(e) = std::io::stdin().lock().read_to_end(&mut buf) {
            eprintln!("error: failed to read stdin: {e}");
            return 1;
        }
        buf
    };

    let output_charset_label: Option<String> = if let Some(c) = &args.output_charset {
        Some(c.clone())
    } else if args.to_utf8 {
        Some("utf-8".to_string())
    } else {
        None
    };

    let mut stdout_lock_holder;
    let body_sink = if args.editor.is_some() {
        output::BodySink::Editor
    } else if args.to_clipboard {
        output::BodySink::Clipboard
    } else if let Some(p) = args.output.as_deref() {
        output::BodySink::File(p)
    } else {
        stdout_lock_holder = std::io::stdout().lock();
        output::BodySink::Writer(&mut stdout_lock_holder)
    };

    match output::write_processed_body(
        args,
        &buf,
        "", // no Content-Type — body sniffing kicks in
        output_charset_label.as_deref(),
        body_sink,
    ) {
        Ok(_) => 0,
        Err(e) => {
            if args.full_errors {
                eprintln!("error: {e:#}");
            } else {
                eprintln!("error: {}", friendly_message(&e));
            }
            1
        }
    }
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
        || msg.starts_with("--client-cert")
        || msg.starts_with("--key")
        || msg.starts_with("--cert-type")
        || msg.starts_with("--key-type")
        || msg.starts_with("--pass")
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
        || msg.contains("browser fingerprint impersonation")
        || msg.contains("impersonate profile")
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
        let (cfg_default, user_aliases) = editor::load_editor_config();
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
        let (cfg_default, user_aliases) = editor::load_editor_config();
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
                    let (cfg_default, user_aliases) = editor::load_editor_config();
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
