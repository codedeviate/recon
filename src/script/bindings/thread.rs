//! Script threading primitives — `spawn`, `join`, `channel`, `send`,
//! `recv`, `tid`.
//!
//! Relies on rhai's `sync` feature (enabled in Cargo.toml) which makes
//! the engine Send+Sync. `spawn(fn_ptr, args)` copies the current
//! engine + AST into a new OS thread, evaluates the closure, and
//! returns a `ThreadHandle` that `join(h)` blocks on.
//!
//! Channels are MPSC: one receiver shared across multiple senders.
//! `channel()` is unbounded; `channel_bounded(capacity)` gives you
//! back-pressure via a bounded `sync_channel`.

use crate::script::convert::err;
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, FnPtr, Shared, AST};
use std::sync::{
    mpsc::{self, Receiver, Sender, SyncSender},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

type ThreadResult = Result<Dynamic, String>;
type ThreadInner = Arc<Mutex<Option<JoinHandle<ThreadResult>>>>;

#[derive(Clone)]
pub struct ThreadHandle {
    inner: ThreadInner,
}

#[derive(Clone)]
pub struct RhaiSender {
    kind: SenderKind,
}

#[derive(Clone)]
enum SenderKind {
    Unbounded(Sender<Dynamic>),
    Bounded(SyncSender<Dynamic>),
}

#[derive(Clone)]
pub struct RhaiReceiver {
    inner: Arc<Mutex<Receiver<Dynamic>>>,
}

pub fn register(engine: &mut Engine, ast: Shared<AST>, defaults: ScriptDefaults) {
    let defaults = Arc::new(defaults);
    engine.register_type_with_name::<ThreadHandle>("ThreadHandle");
    engine.register_type_with_name::<RhaiSender>("RhaiSender");
    engine.register_type_with_name::<RhaiReceiver>("RhaiReceiver");

    // tid() — current thread ID (rust internal; just for logging).
    engine.register_fn("tid", || -> i64 {
        // ThreadId's internal u64 isn't stable across releases; hash it.
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        thread::current().id().hash(&mut h);
        (h.finish() as i64).abs()
    });

    engine.register_fn("sleep", |ms: i64| {
        if ms > 0 {
            thread::sleep(Duration::from_millis(ms as u64));
        }
    });

    // ── spawn ──────────────────────────────────────────────────────
    //
    // The engine isn't Clone in rhai 1.x even with `sync`, but we can
    // build a fresh engine per spawn that reuses the same AST. The AST
    // is Shared (Arc) under the sync feature, so passing it across
    // threads is cheap.
    let ast_for_spawn = ast.clone();
    let d = defaults.clone();
    engine.register_fn(
        "thread_spawn",
        move |fn_ptr: FnPtr| -> Result<ThreadHandle, Box<EvalAltResult>> {
            Ok(spawn_closure(ast_for_spawn.clone(), d.clone(), fn_ptr, Vec::new()))
        },
    );

    let ast_for_spawn = ast.clone();
    let d = defaults.clone();
    engine.register_fn(
        "thread_spawn",
        move |fn_ptr: FnPtr, arg: Dynamic| -> Result<ThreadHandle, Box<EvalAltResult>> {
            Ok(spawn_closure(ast_for_spawn.clone(), d.clone(), fn_ptr, vec![arg]))
        },
    );

    let ast_for_spawn = ast.clone();
    let d = defaults.clone();
    engine.register_fn(
        "thread_spawn",
        move |fn_ptr: FnPtr, args: Array| -> Result<ThreadHandle, Box<EvalAltResult>> {
            Ok(spawn_closure(ast_for_spawn.clone(), d.clone(), fn_ptr, args))
        },
    );

    register_join_and_channels(engine);
}

/// REPL-mode variant of `register`. Registers everything `register` does
/// **except** `thread_spawn`, which is replaced with a stub that returns
/// a clear error. The script engine's spawn needs a `Shared<AST>` handle
/// to dispatch into; the REPL has no single static AST, so threading is
/// disabled there. Channels, sleep, tid, and join still work for any
/// pre-existing `ThreadHandle`s (in practice unreachable, but harmless).
pub fn register_repl_stub(engine: &mut Engine) {
    // Same type registrations as `register`.
    engine.register_type_with_name::<ThreadHandle>("ThreadHandle");
    engine.register_type_with_name::<RhaiSender>("RhaiSender");
    engine.register_type_with_name::<RhaiReceiver>("RhaiReceiver");

    // tid + sleep don't need the AST — register as-is.
    engine.register_fn("tid", || -> i64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        thread::current().id().hash(&mut h);
        (h.finish() as i64).abs()
    });
    engine.register_fn("sleep", |ms: i64| {
        if ms > 0 {
            thread::sleep(Duration::from_millis(ms as u64));
        }
    });

    // thread_spawn — all three overloads error out identically.
    engine.register_fn(
        "thread_spawn",
        |_f: FnPtr| -> Result<ThreadHandle, Box<EvalAltResult>> {
            Err(err("thread_spawn is not available in REPL mode (script-only)"))
        },
    );
    engine.register_fn(
        "thread_spawn",
        |_f: FnPtr, _a: Dynamic| -> Result<ThreadHandle, Box<EvalAltResult>> {
            Err(err("thread_spawn is not available in REPL mode (script-only)"))
        },
    );
    engine.register_fn(
        "thread_spawn",
        |_f: FnPtr, _a: Array| -> Result<ThreadHandle, Box<EvalAltResult>> {
            Err(err("thread_spawn is not available in REPL mode (script-only)"))
        },
    );

    // join, channel, send, recv — these don't need the AST either.
    register_join_and_channels(engine);
}

/// Helper shared by `register` and `register_repl_stub`. Registers the
/// AST-independent surface: join, channel, send, recv. Both entry points
/// call this so the surface stays in sync.
fn register_join_and_channels(engine: &mut Engine) {
    engine.register_fn(
        "join",
        |h: &mut ThreadHandle| -> Result<Dynamic, Box<EvalAltResult>> {
            let handle = {
                let mut guard = h
                    .inner
                    .lock()
                    .map_err(|_| err("join: thread-handle mutex poisoned"))?;
                guard.take()
            };
            match handle {
                None => Err(err("join: handle already joined")),
                Some(jh) => match jh.join() {
                    Err(_) => Err(err("join: spawned thread panicked")),
                    Ok(Err(msg)) => Err(err(format!("spawned task error: {msg}"))),
                    Ok(Ok(v)) => Ok(v),
                },
            }
        },
    );

    engine.register_fn("channel", || -> Array {
        let (tx, rx) = mpsc::channel::<Dynamic>();
        vec![
            Dynamic::from(RhaiSender {
                kind: SenderKind::Unbounded(tx),
            }),
            Dynamic::from(RhaiReceiver {
                inner: Arc::new(Mutex::new(rx)),
            }),
        ]
    });

    engine.register_fn("channel_bounded", |capacity: i64| -> Array {
        let cap = capacity.max(0) as usize;
        let (tx, rx) = mpsc::sync_channel::<Dynamic>(cap);
        vec![
            Dynamic::from(RhaiSender {
                kind: SenderKind::Bounded(tx),
            }),
            Dynamic::from(RhaiReceiver {
                inner: Arc::new(Mutex::new(rx)),
            }),
        ]
    });

    engine.register_fn(
        "send",
        |s: &mut RhaiSender, val: Dynamic| -> Result<(), Box<EvalAltResult>> {
            match &s.kind {
                SenderKind::Unbounded(tx) => tx
                    .send(val)
                    .map_err(|e| err(format!("send: channel closed ({e})"))),
                SenderKind::Bounded(tx) => tx
                    .send(val)
                    .map_err(|e| err(format!("send: channel closed ({e})"))),
            }
        },
    );

    engine.register_fn(
        "try_send",
        |s: &mut RhaiSender, val: Dynamic| -> Result<bool, Box<EvalAltResult>> {
            match &s.kind {
                SenderKind::Unbounded(tx) => {
                    tx.send(val)
                        .map_err(|e| err(format!("try_send: channel closed ({e})")))?;
                    Ok(true)
                }
                SenderKind::Bounded(tx) => match tx.try_send(val) {
                    Ok(()) => Ok(true),
                    Err(mpsc::TrySendError::Full(_)) => Ok(false),
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        Err(err("try_send: channel closed"))
                    }
                },
            }
        },
    );

    engine.register_fn(
        "recv",
        |r: &mut RhaiReceiver| -> Result<Dynamic, Box<EvalAltResult>> {
            let guard = r
                .inner
                .lock()
                .map_err(|_| err("recv: receiver mutex poisoned"))?;
            guard.recv().map_err(|_| err("recv: all senders dropped"))
        },
    );

    engine.register_fn(
        "recv",
        |r: &mut RhaiReceiver, timeout_ms: i64| -> Result<Dynamic, Box<EvalAltResult>> {
            let guard = r
                .inner
                .lock()
                .map_err(|_| err("recv: receiver mutex poisoned"))?;
            guard
                .recv_timeout(Duration::from_millis(timeout_ms.max(0) as u64))
                .map_err(|e| err(format!("recv: timeout or closed ({e})")))
        },
    );

    engine.register_fn("try_recv", |r: &mut RhaiReceiver| -> Dynamic {
        let Ok(guard) = r.inner.lock() else {
            return Dynamic::UNIT;
        };
        guard.try_recv().unwrap_or(Dynamic::UNIT)
    });
}

fn spawn_closure(
    ast: Shared<AST>,
    defaults: Arc<ScriptDefaults>,
    fn_ptr: FnPtr,
    args: Vec<Dynamic>,
) -> ThreadHandle {
    let jh = thread::spawn(move || -> Result<Dynamic, String> {
        // Build a fresh engine for the worker and re-register the
        // threading bindings so nested spawn / send / recv calls work.
        // Binding registration is relatively cheap compared to whatever
        // the user script is doing inside the spawned closure.
        // Inheriting `defaults` ensures http/tcp/etc. probes see the
        // same CLI-flag inheritance chain as the parent.
        let mut engine = crate::script::engine::build_engine(&defaults);
        register(&mut engine, ast.clone(), (*defaults).clone());
        fn_ptr
            .call::<Dynamic>(&engine, &ast, args)
            .map_err(|e| e.to_string())
    });
    ThreadHandle {
        inner: Arc::new(Mutex::new(Some(jh))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_engine_with_threads() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        let empty = Shared::new(AST::empty());
        register(&mut e, empty, ScriptDefaults::default());
        e
    }

    #[test]
    fn tid_is_an_integer() {
        let e = build_engine_with_threads();
        let t: i64 = e.eval("tid()").expect("eval");
        assert!(t > 0);
    }

    #[test]
    fn channel_send_recv() {
        let e = build_engine_with_threads();
        let v: i64 = e
            .eval(
                r#"
let c = channel();
let tx = c[0];
let rx = c[1];
send(tx, 42);
recv(rx)
"#,
            )
            .expect("eval");
        assert_eq!(v, 42);
    }

    #[test]
    fn bounded_channel_try_send_fills() {
        let e = build_engine_with_threads();
        let ok: bool = e
            .eval(
                r#"
let c = channel_bounded(1);
let tx = c[0];
if !try_send(tx, 1) { return false; }
// Second should return false (channel full).
!try_send(tx, 2)
"#,
            )
            .expect("eval");
        assert!(ok);
    }

    #[test]
    fn recv_with_timeout_errors_when_empty() {
        let e = build_engine_with_threads();
        let res: Result<Dynamic, _> = e.eval(
            r#"
let c = channel();
let rx = c[1];
recv(rx, 10)
"#,
        );
        assert!(res.is_err());
    }
}

#[cfg(test)]
mod repl_stub_tests {
    use super::*;
    use rhai::Engine;

    #[test]
    fn thread_spawn_errors_in_repl_mode() {
        let mut engine = Engine::new();
        register_repl_stub(&mut engine);
        let err = engine
            .eval::<rhai::Dynamic>(r#"thread_spawn(|| { 1 })"#)
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not available in REPL mode"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn sleep_still_works_in_repl_mode() {
        let mut engine = Engine::new();
        register_repl_stub(&mut engine);
        engine.eval::<()>("sleep(1)").expect("sleep should work");
    }
}
