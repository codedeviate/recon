//! Engine setup and script execution.

use super::convert::{clear_protocol_exit_code, take_protocol_exit_code};
use super::defaults::ScriptDefaults;
use crate::cli::Args;
use std::path::Path;

/// Execute a script file. Returns the process exit code.
pub fn run_file(path: &Path, args: &Args) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read script '{}': {e}", path.display());
            return 1;
        }
    };

    clear_protocol_exit_code();
    let defaults = ScriptDefaults::from_args(args);
    let engine = build_engine(&defaults);

    // Expose `args` (array) and `flags` (map) as read-only top-level
    // constants via a Scope. Constants because scripts should observe the
    // CLI invocation, not mutate it.
    let mut scope = rhai::Scope::new();
    scope.push_constant("args", super::bindings::cli::build_args_array(args));
    scope.push_constant("flags", super::bindings::cli::build_flags_map(args));

    // Compile with an explicit source so Rhai's default FileModuleResolver
    // can resolve `import "name"` relative to the script's directory.
    // Without set_source, the resolver has no "source path" and imports
    // fail even for sibling files.
    let mut ast = match engine.compile_with_scope(&scope, &source) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    ast.set_source(path.to_string_lossy().into_owned());

    match engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast) {
        Ok(val) => {
            if let Ok(n) = val.as_int() {
                (n & 0xff) as i32
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            take_protocol_exit_code().unwrap_or(1)
        }
    }
}

/// Build a Rhai engine with recon helpers registered. Protocol probe
/// bindings are layered on in subsequent tasks; each probe module takes
/// the `ScriptDefaults` so it can read CLI flag inheritance at call time.
pub fn build_engine(defaults: &ScriptDefaults) -> rhai::Engine {
    let mut engine = rhai::Engine::new();
    install_module_resolver(&mut engine);
    super::bindings::agent_browser::register(&mut engine);
    super::bindings::archive::register(&mut engine);
    super::bindings::browser::register(&mut engine, defaults.clone());
    super::bindings::checkdigit::register(&mut engine);
    super::bindings::compression::register(&mut engine);
    super::bindings::email::register(&mut engine);
    super::bindings::encode::register(&mut engine);
    super::bindings::encrypt::register(&mut engine);
    super::bindings::helpers::register(&mut engine);
    super::bindings::imap::register(&mut engine, defaults.clone());
    super::bindings::jwt::register(&mut engine);
    super::bindings::netstatus::register(&mut engine);
    super::bindings::sample::register(&mut engine);
    super::bindings::dict::register(&mut engine, defaults.clone());
    super::bindings::dns::register(&mut engine);
    super::bindings::file::register(&mut engine);
    super::bindings::ftp::register(&mut engine, defaults.clone());
    super::bindings::gopher::register(&mut engine, defaults.clone());
    super::bindings::hash::register(&mut engine);
    super::bindings::http::register(&mut engine, defaults.clone());
    super::bindings::ldap::register(&mut engine, defaults.clone());
    super::bindings::memcached::register(&mut engine, defaults.clone());
    super::bindings::mqtt::register(&mut engine, defaults.clone());
    super::bindings::ntp::register(&mut engine, defaults.clone());
    super::bindings::ping::register(&mut engine, defaults.clone());
    super::bindings::pop3::register(&mut engine, defaults.clone());
    super::bindings::redis::register(&mut engine, defaults.clone());
    super::bindings::rtsp::register(&mut engine, defaults.clone());
    super::bindings::sftp::register(&mut engine, defaults.clone());
    super::bindings::smtp::register(&mut engine, defaults.clone());
    super::bindings::sqlite::register(&mut engine);
    super::bindings::tcp::register(&mut engine, defaults.clone());
    super::bindings::text::register(&mut engine);
    super::bindings::tftp::register(&mut engine, defaults.clone());
    super::bindings::tls::register(&mut engine);
    super::bindings::whois::register(&mut engine);
    super::bindings::ws::register(&mut engine, defaults.clone());
    engine
}

/// Install Rhai's module resolver so `import "name"` statements work.
/// Two resolvers chained: (1) Rhai default — resolves relative to the
/// importing script's directory; (2) `FileModuleResolver` rooted at
/// `~/.recon/script/` — picks up shared modules for scripts that live
/// outside the global dir. Both auto-append `.rhai`. If `$HOME` is
/// unset, only the default resolver is registered.
fn install_module_resolver(engine: &mut rhai::Engine) {
    use rhai::module_resolvers::{FileModuleResolver, ModuleResolversCollection};

    let mut resolvers = ModuleResolversCollection::new();
    resolvers.push(FileModuleResolver::new());
    if let Some(dir) = super::script_dir() {
        resolvers.push(FileModuleResolver::new_with_path(dir));
    }
    engine.set_module_resolver(resolvers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_script(body: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(body.as_bytes()).expect("write");
        f
    }

    fn dummy_args() -> Args {
        use clap::Parser;
        Args::try_parse_from(["recon", "--script", "/dev/null"]).expect("parse")
    }

    #[test]
    fn args_and_flags_exposed_to_script() {
        // Script exits 42 only if args[0] == "myscript" AND flags.insecure is true.
        let a = Args::parse_with_script_split([
            "recon", "-k", "--script", "myscript", "one", "two",
        ])
        .unwrap();
        let script = r#"
assert(args[0] == "myscript", `args[0] was ${args[0]}`);
assert(args.len() == 3, `len was ${args.len()}`);
assert(args[1] == "one", `args[1] was ${args[1]}`);
assert(args[2] == "two", `args[2] was ${args[2]}`);
assert(flags.insecure == true, "insecure");
assert(flags.connect_timeout == 30, "timeout");
return 42;
"#;
        let f = write_script(script);
        let code = run_file(f.path(), &a);
        assert_eq!(code, 42);
    }

    #[test]
    fn args_immutable_from_script() {
        let a = Args::parse_with_script_split(["recon", "--script", "x"]).unwrap();
        // Attempting to mutate a constant is a Rhai error.
        let f = write_script(r#"args.push("boom"); return 0;"#);
        let code = run_file(f.path(), &a);
        assert_ne!(code, 0, "expected non-zero exit from mutation attempt");
    }

    #[test]
    fn returns_zero_for_empty_script() {
        let f = write_script("");
        let code = run_file(f.path(), &dummy_args());
        assert_eq!(code, 0);
    }

    #[test]
    fn return_integer_maps_to_exit_code() {
        let f = write_script("return 3;");
        let code = run_file(f.path(), &dummy_args());
        assert_eq!(code, 3);
    }

    #[test]
    fn missing_file_reports_error() {
        let code = run_file(Path::new("/nonexistent/path/xyz.rhai"), &dummy_args());
        assert_eq!(code, 1);
    }

    #[test]
    fn syntax_error_returns_one() {
        let f = write_script("this is not valid rhai @#$");
        let code = run_file(f.path(), &dummy_args());
        assert_eq!(code, 1);
    }

    #[test]
    fn import_resolves_sibling_script() {
        // Two tempfiles in the same dir. a imports b.
        let dir = tempfile::tempdir().unwrap();
        let lib_path = dir.path().join("lib.rhai");
        std::fs::write(&lib_path, "fn salute(n) { `hi ${n}` }").unwrap();
        let main_path = dir.path().join("main.rhai");
        std::fs::write(
            &main_path,
            r#"import "lib" as lib;
let s = lib::salute("world");
if s == "hi world" { 42 } else { 1 }"#,
        )
        .unwrap();

        let mut args = dummy_args();
        args.script = Some(main_path.clone());
        let code = run_file(&main_path, &args);
        assert_eq!(code, 42);
    }

    #[test]
    fn missing_module_returns_nonzero() {
        let f = write_script(r#"import "definitely_does_not_exist" as x; return 0;"#);
        let code = run_file(f.path(), &dummy_args());
        assert_ne!(code, 0);
    }
}
