//! Reconnaissance Blind Chess game logic.
//!
//! This crate intentionally exposes crate-owned game types and keeps the
//! underlying chess implementation as an internal detail.

/// Crate version from Cargo metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
