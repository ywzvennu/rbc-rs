//! Reconnaissance Blind Chess game logic.
//!
//! This crate intentionally exposes crate-owned game types and keeps the
//! underlying chess implementation as an internal detail.

mod attack_tables;

mod types;

mod position;

mod game;

pub use game::Game;
pub use types::{
    Capture, Color, DrawReason, Error, GameConfig, GameResult, GameStatus, HistoryEntry, Move,
    MoveOutcome, MoveStatus, Piece, PieceKind, SenseResult, SensedSquare, Square, WinReason,
};

/// Crate version from Cargo metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
