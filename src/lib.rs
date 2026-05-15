//! Library surface for integration tests and downstream crates.
//!
//! The `recon-cli` crate is primarily a binary; this `lib.rs` exposes the
//! minimal internal paths required by `tests/script_ai_it.rs`. Nothing
//! here is part of a public API guarantee.
//!
//! We deliberately do NOT re-export the full `script` module tree because
//! that tree depends on `cli::Args` and hundreds of other internal modules.
//! Instead we mirror only the `script::bindings::ai` path the tests need,
//! using `#[path]` so Rust finds the source files in their real location
//! under `src/script/bindings/ai/`.

pub mod config;

// The real `ai/` module tree, pointed at the actual source files via
// `#[path]`. Compiled independently of the binary's module tree.
#[path = "script/bindings/ai/mod.rs"]
pub(crate) mod ai_impl;

/// Mirrors the `script::bindings::ai` import path used by integration tests.
pub mod script {
    pub mod bindings {
        pub mod ai {
            pub mod backend {
                pub use crate::ai_impl::backend::*;
            }
            pub mod request {
                pub use crate::ai_impl::request::*;
            }
            pub use crate::ai_impl::register_with_registry;
        }
    }
}
