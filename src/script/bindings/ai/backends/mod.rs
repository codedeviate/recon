//! Per-backend implementations of `AiBackend`. Registered into the
//! backend dispatcher in `super::backend::Registry::built_in()`.
//!
//! Each backend lives in its own file under this module.

// Stub — populated in later tasks.
pub mod claude;
pub mod cmd;
pub mod codex;
pub mod gemini;
