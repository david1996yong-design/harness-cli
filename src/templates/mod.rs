//! Embedded template files for all supported AI platforms.
//!
//! Templates are embedded at compile time using `rust-embed` so the binary is
//! fully self-contained.

pub mod extract;

pub mod antigravity;
pub mod claude;
pub mod codebuddy;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod gemini;
pub mod harness_cli;
pub mod iflow;
pub mod kilo;
pub mod kiro;
pub mod markdown;
pub mod opencode;
pub mod qoder;
pub mod windsurf;
