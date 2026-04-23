//! `sqlite(spec [, mode])` script binding.
//!
//! Returns a `SqliteHandle` with four methods for querying and executing
//! SQL: `query`, `query_one`, `query_value`, `exec`. `spec` is either a
//! filesystem path (contains `/`, `\`, or ends with `.db`), an alias for
//! an internal recon database (`cookiejar`, `cookiejar:NAME`), or
//! `":memory:"` for an ephemeral in-memory database.
//!
//! Default mode is `"rw"` for both paths and aliases; override with
//! `"ro"` (read-only), `"rwc"` (read-write + create on missing).
//!
//! Parameters bind positionally via `?` placeholders; each is a Rhai
//! value mapped to the closest SQLite affinity (() → NULL, bool → INT,
//! i64 → INT, f64 → REAL, String → TEXT, Blob → BLOB). Rows come back
//! as Rhai maps keyed by column name.

use crate::script::convert::err;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult, Map};
use rusqlite::{types::ValueRef, Connection, OpenFlags, ToSql};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SqliteHandle {
    conn: Arc<Mutex<Connection>>,
    path: Arc<PathBuf>,
}

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<SqliteHandle>("SqliteHandle");

    engine.register_fn("sqlite", |spec: &str| -> Result<SqliteHandle, Box<EvalAltResult>> {
        open_spec(spec, None)
    });
    engine.register_fn(
        "sqlite",
        |spec: &str, mode: &str| -> Result<SqliteHandle, Box<EvalAltResult>> {
            open_spec(spec, Some(mode))
        },
    );

    // query(sql) / query(sql, params)
    engine.register_fn(
        "query",
        |h: &mut SqliteHandle, sql: &str| -> Result<Array, Box<EvalAltResult>> {
            run_query(h, sql, Vec::new())
        },
    );
    engine.register_fn(
        "query",
        |h: &mut SqliteHandle, sql: &str, params: Array| -> Result<Array, Box<EvalAltResult>> {
            let bound = bind_params(params)?;
            run_query(h, sql, bound)
        },
    );

    // query_one(sql) / query_one(sql, params)
    engine.register_fn(
        "query_one",
        |h: &mut SqliteHandle, sql: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_query_one(h, sql, Vec::new())
        },
    );
    engine.register_fn(
        "query_one",
        |h: &mut SqliteHandle, sql: &str, params: Array| -> Result<Dynamic, Box<EvalAltResult>> {
            let bound = bind_params(params)?;
            run_query_one(h, sql, bound)
        },
    );

    // query_value(sql) / query_value(sql, params)
    engine.register_fn(
        "query_value",
        |h: &mut SqliteHandle, sql: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            run_query_value(h, sql, Vec::new())
        },
    );
    engine.register_fn(
        "query_value",
        |h: &mut SqliteHandle, sql: &str, params: Array| -> Result<Dynamic, Box<EvalAltResult>> {
            let bound = bind_params(params)?;
            run_query_value(h, sql, bound)
        },
    );

    // exec(sql) / exec(sql, params) → i64 rows affected
    engine.register_fn(
        "exec",
        |h: &mut SqliteHandle, sql: &str| -> Result<i64, Box<EvalAltResult>> {
            run_exec(h, sql, Vec::new())
        },
    );
    engine.register_fn(
        "exec",
        |h: &mut SqliteHandle, sql: &str, params: Array| -> Result<i64, Box<EvalAltResult>> {
            let bound = bind_params(params)?;
            run_exec(h, sql, bound)
        },
    );
}

// ── Spec resolution ───────────────────────────────────────────────────────

enum Resolved {
    Memory,
    File(PathBuf),
}

fn resolve_spec(spec: &str) -> Result<Resolved, Box<EvalAltResult>> {
    if spec == ":memory:" {
        return Ok(Resolved::Memory);
    }
    if spec.contains('/') || spec.contains('\\') || spec.ends_with(".db") {
        return Ok(Resolved::File(PathBuf::from(spec)));
    }
    // Alias form: `name` or `name:arg`
    let (name, arg) = match spec.split_once(':') {
        Some((n, a)) => (n, Some(a)),
        None => (spec, None),
    };
    match name {
        "cookiejar" => {
            let jar = arg.unwrap_or("default");
            Ok(Resolved::File(cookiejar_path(jar)))
        }
        other => Err(err(format!(
            "sqlite: unknown spec '{other}' — expected a path or known alias (cookiejar[:NAME], :memory:)"
        ))),
    }
}

fn cookiejar_path(name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".recon").join("jars").join(format!("{name}.db"))
}

fn open_spec(spec: &str, mode: Option<&str>) -> Result<SqliteHandle, Box<EvalAltResult>> {
    let resolved = resolve_spec(spec)?;
    let mode_str = mode.unwrap_or("rw");
    let flags = parse_mode(mode_str)?;

    let (conn, path) = match resolved {
        Resolved::Memory => {
            let c = Connection::open_in_memory()
                .map_err(|e| err(format!("sqlite: open_in_memory: {e}")))?;
            (c, PathBuf::from(":memory:"))
        }
        Resolved::File(p) => {
            if let Some(parent) = p.parent() {
                if flags.contains(OpenFlags::SQLITE_OPEN_CREATE) && !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            let c = Connection::open_with_flags(&p, flags)
                .map_err(|e| err(format!("sqlite: open '{}': {e}", p.display())))?;
            (c, p)
        }
    };

    Ok(SqliteHandle {
        conn: Arc::new(Mutex::new(conn)),
        path: Arc::new(path),
    })
}

fn parse_mode(mode: &str) -> Result<OpenFlags, Box<EvalAltResult>> {
    match mode {
        "ro" => Ok(OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX),
        "rw" => Ok(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX),
        "rwc" => Ok(OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX),
        other => Err(err(format!(
            "sqlite: invalid mode '{other}' (expected \"ro\", \"rw\", or \"rwc\")"
        ))),
    }
}

// ── Parameter binding ─────────────────────────────────────────────────────

enum BoundValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl ToSql for BoundValue {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        use rusqlite::types::{ToSqlOutput, Value, ValueRef as VR};
        Ok(match self {
            BoundValue::Null => ToSqlOutput::Owned(Value::Null),
            BoundValue::Integer(i) => ToSqlOutput::Owned(Value::Integer(*i)),
            BoundValue::Real(f) => ToSqlOutput::Owned(Value::Real(*f)),
            BoundValue::Text(s) => ToSqlOutput::Borrowed(VR::Text(s.as_bytes())),
            BoundValue::Blob(b) => ToSqlOutput::Borrowed(VR::Blob(b)),
        })
    }
}

fn bind_params(params: Array) -> Result<Vec<BoundValue>, Box<EvalAltResult>> {
    let mut out = Vec::with_capacity(params.len());
    for (i, v) in params.into_iter().enumerate() {
        out.push(dynamic_to_bound(v, i)?);
    }
    Ok(out)
}

fn dynamic_to_bound(v: Dynamic, index: usize) -> Result<BoundValue, Box<EvalAltResult>> {
    if v.is_unit() {
        return Ok(BoundValue::Null);
    }
    if let Ok(b) = v.as_bool() {
        return Ok(BoundValue::Integer(if b { 1 } else { 0 }));
    }
    if let Ok(i) = v.as_int() {
        return Ok(BoundValue::Integer(i));
    }
    if let Ok(f) = v.as_float() {
        return Ok(BoundValue::Real(f));
    }
    if v.is_string() {
        return Ok(BoundValue::Text(v.into_string().unwrap_or_default()));
    }
    if v.is_blob() {
        return Ok(BoundValue::Blob(
            v.into_blob()
                .map_err(|_| err(format!("sqlite: param {index}: blob cast failed")))?,
        ));
    }
    Err(err(format!(
        "sqlite: unsupported param type at index {index}"
    )))
}

// ── Execution helpers ─────────────────────────────────────────────────────

fn map_err_path(path: &Path, e: rusqlite::Error) -> Box<EvalAltResult> {
    err(format!("sqlite: {} — {e}", path.display()))
}

fn run_query(
    h: &SqliteHandle,
    sql: &str,
    params: Vec<BoundValue>,
) -> Result<Array, Box<EvalAltResult>> {
    let conn = h.conn.lock().map_err(|_| err("sqlite: mutex poisoned"))?;
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| map_err_path(&h.path, e))?;
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();
    let mut rows = stmt
        .query(rusqlite::params_from_iter(param_refs))
        .map_err(|e| map_err_path(&h.path, e))?;
    let mut out = Array::new();
    while let Some(row) = rows.next().map_err(|e| map_err_path(&h.path, e))? {
        out.push(row_to_map(row, &col_names)?.into());
    }
    Ok(out)
}

fn run_query_one(
    h: &SqliteHandle,
    sql: &str,
    params: Vec<BoundValue>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let conn = h.conn.lock().map_err(|_| err("sqlite: mutex poisoned"))?;
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| map_err_path(&h.path, e))?;
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();
    let mut rows = stmt
        .query(rusqlite::params_from_iter(param_refs))
        .map_err(|e| map_err_path(&h.path, e))?;
    match rows.next().map_err(|e| map_err_path(&h.path, e))? {
        Some(row) => Ok(row_to_map(row, &col_names)?.into()),
        None => Ok(Dynamic::UNIT),
    }
}

fn run_query_value(
    h: &SqliteHandle,
    sql: &str,
    params: Vec<BoundValue>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let conn = h.conn.lock().map_err(|_| err("sqlite: mutex poisoned"))?;
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| map_err_path(&h.path, e))?;
    let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();
    let mut rows = stmt
        .query(rusqlite::params_from_iter(param_refs))
        .map_err(|e| map_err_path(&h.path, e))?;
    match rows.next().map_err(|e| map_err_path(&h.path, e))? {
        Some(row) => value_ref_to_dynamic(
            row.get_ref(0).map_err(|e| map_err_path(&h.path, e))?,
        ),
        None => Ok(Dynamic::UNIT),
    }
}

fn run_exec(
    h: &SqliteHandle,
    sql: &str,
    params: Vec<BoundValue>,
) -> Result<i64, Box<EvalAltResult>> {
    let conn = h.conn.lock().map_err(|_| err("sqlite: mutex poisoned"))?;
    let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();
    let n = conn
        .execute(sql, rusqlite::params_from_iter(param_refs))
        .map_err(|e| map_err_path(&h.path, e))?;
    Ok(n as i64)
}

// ── Row conversion ────────────────────────────────────────────────────────

fn row_to_map(
    row: &rusqlite::Row<'_>,
    col_names: &[String],
) -> Result<Map, Box<EvalAltResult>> {
    let mut m = Map::new();
    for (i, name) in col_names.iter().enumerate() {
        let vr = row
            .get_ref(i)
            .map_err(|e| err(format!("sqlite: read column {i}: {e}")))?;
        m.insert(name.as_str().into(), value_ref_to_dynamic(vr)?);
    }
    Ok(m)
}

fn value_ref_to_dynamic(v: ValueRef<'_>) -> Result<Dynamic, Box<EvalAltResult>> {
    Ok(match v {
        ValueRef::Null => Dynamic::UNIT,
        ValueRef::Integer(i) => Dynamic::from(i),
        ValueRef::Real(f) => Dynamic::from(f),
        ValueRef::Text(bytes) => {
            Dynamic::from(String::from_utf8_lossy(bytes).into_owned())
        }
        ValueRef::Blob(bytes) => {
            let b: Blob = bytes.to_vec();
            Dynamic::from(b)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        register(&mut e);
        // Helpers (for assert)
        super::super::helpers::register(&mut e);
        e
    }

    #[test]
    fn memory_db_crud_roundtrip() {
        let e = engine();
        let script = r#"
let db = sqlite(":memory:");
db.exec("CREATE TABLE t (id INTEGER, name TEXT)");
db.exec("INSERT INTO t VALUES (?, ?)", [1, "alice"]);
db.exec("INSERT INTO t VALUES (?, ?)", [2, "bob"]);
let all = db.query("SELECT * FROM t ORDER BY id");
assert(all.len() == 2, "len");
assert(all[0].name == "alice", "alice");
assert(all[1].name == "bob", "bob");
return 0;
"#;
        let code: i64 = e.eval(script).expect("eval");
        assert_eq!(code, 0);
    }

    #[test]
    fn query_one_returns_unit_for_empty() {
        let e = engine();
        let script = r#"
let db = sqlite(":memory:");
db.exec("CREATE TABLE t (id INTEGER)");
let r = db.query_one("SELECT id FROM t WHERE id = ?", [99]);
r == ()
"#;
        let eq: bool = e.eval(script).expect("eval");
        assert!(eq);
    }

    #[test]
    fn query_value_returns_scalar() {
        let e = engine();
        let script = r#"
let db = sqlite(":memory:");
db.exec("CREATE TABLE t (n INTEGER)");
db.exec("INSERT INTO t VALUES (1)");
db.exec("INSERT INTO t VALUES (2)");
db.exec("INSERT INTO t VALUES (3)");
db.query_value("SELECT COUNT(*) FROM t", [])
"#;
        let n: i64 = e.eval(script).expect("eval");
        assert_eq!(n, 3);
    }

    #[test]
    fn exec_returns_rows_affected() {
        let e = engine();
        let script = r#"
let db = sqlite(":memory:");
db.exec("CREATE TABLE t (id INTEGER)");
db.exec("INSERT INTO t VALUES (1)");
db.exec("INSERT INTO t VALUES (2)");
db.exec("DELETE FROM t WHERE id = ?", [1])
"#;
        let n: i64 = e.eval(script).expect("eval");
        assert_eq!(n, 1);
    }

    #[test]
    fn param_types_roundtrip() {
        let e = engine();
        let script = r#"
let db = sqlite(":memory:");
db.exec("CREATE TABLE t (i INTEGER, f REAL, s TEXT, n INTEGER, b BLOB)");
let blob_val = blob();
blob_val.push(0x01);
blob_val.push(0xff);
db.exec("INSERT INTO t VALUES (?, ?, ?, ?, ?)", [42, 3.14, "hi", (), blob_val]);
let row = db.query_one("SELECT * FROM t", []);
assert(row.i == 42, "i");
assert(row.s == "hi", "s");
assert(row.n == (), "null");
return 0;
"#;
        let code: i64 = e.eval(script).expect("eval");
        assert_eq!(code, 0);
    }

    #[test]
    fn unknown_alias_throws() {
        let e = engine();
        let res: Result<SqliteHandle, _> = e.eval(r#"sqlite("nosuchalias")"#);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_mode_throws() {
        let e = engine();
        let res: Result<SqliteHandle, _> = e.eval(r#"sqlite(":memory:", "bogus")"#);
        assert!(res.is_err());
    }

    #[test]
    fn sql_syntax_error_throws() {
        let e = engine();
        let res: Result<Array, _> = e.eval(r#"sqlite(":memory:").query("NOT VALID SQL")"#);
        assert!(res.is_err());
    }

    #[test]
    fn file_path_open_create() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let e = engine();
        let script = format!(
            r#"
let db = sqlite("{}", "rwc");
db.exec("CREATE TABLE t (n INTEGER)");
db.exec("INSERT INTO t VALUES (7)");
db.query_value("SELECT n FROM t", [])
"#,
            path.display()
        );
        let n: i64 = e.eval(&script).expect("eval");
        assert_eq!(n, 7);
    }

    #[test]
    fn file_path_rw_missing_errors() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("does_not_exist.db");
        let e = engine();
        let script = format!(r#"sqlite("{}")"#, path.display());
        let res: Result<SqliteHandle, _> = e.eval(&script);
        assert!(res.is_err(), "rw should fail on missing file");
    }

    #[test]
    fn resolve_spec_routes() {
        use super::resolve_spec;
        assert!(matches!(resolve_spec(":memory:").unwrap(), Resolved::Memory));
        assert!(matches!(
            resolve_spec("/tmp/foo.db").unwrap(),
            Resolved::File(_)
        ));
        assert!(matches!(resolve_spec("foo.db").unwrap(), Resolved::File(_)));
        assert!(matches!(resolve_spec("cookiejar").unwrap(), Resolved::File(_)));
        assert!(matches!(
            resolve_spec("cookiejar:session").unwrap(),
            Resolved::File(_)
        ));
        assert!(resolve_spec("bogusalias").is_err());
    }
}
