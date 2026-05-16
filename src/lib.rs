//! Reconnaissance Blind Chess (RBC) game logic in Rust.
//!
//! `rbc-rs` is the Rust home for the RBC rules layer. It is behaviourally
//! equivalent to the upstream Python [`reconchess`] package and implements
//! board state, FEN parsing, move generation, move resolution, sensing,
//! history, and result adjudication in-tree, with no external chess engine
//! dependency. The full game rules live at <https://rbc.jhuapl.edu/gameRules>.
//!
//! # Scope
//!
//! The crate provides the game-playing core: [`Game`], sense actions, move
//! generation and resolution, capture and king-capture handling, and
//! draw/win/timeout adjudication. Bots, networking, clocks, and server
//! integration are intentionally out of scope and belong to downstream
//! crates.
//!
//! # Quick start
//!
//! ```
//! use rbc_rs::{Color, Game, GameConfig, Move, Square};
//!
//! let mut game = Game::new(GameConfig::default());
//!
//! // Sense a 3×3 window centred on e4 from the side to move.
//! let center = Square::from_coords(4, 3).expect("valid square");
//! let sense_result = game.sense(Some(center));
//! assert_eq!(sense_result.squares.len(), 9);
//!
//! // Generate the candidate move requests for the acting player.
//! let actions = game.move_actions();
//! assert!(!actions.is_empty());
//!
//! // Apply 1. e2-e4. The crate revises blocked slider moves and reports
//! // any capture; here it accepts the move unchanged and flips the turn.
//! let e2_e4 = Move {
//!     from: Square::from_coords(4, 1).expect("valid square"),
//!     to: Square::from_coords(4, 3).expect("valid square"),
//!     promotion: None,
//! };
//! let outcome = game.apply_move(Some(e2_e4)).expect("legal request");
//! assert!(outcome.taken.is_some());
//! assert_eq!(game.turn(), Some(Color::Black));
//! ```
//!
//! For a longer worked example, see `examples/quickstart.rs` and run it
//! with `cargo run --example quickstart`.
//!
//! # Serialization
//!
//! The default feature set includes `serde`, which provides `Serialize`
//! and `Deserialize` for [`Game`] and the surrounding types. Disable
//! default features to drop the serde dependency entirely:
//!
//! ```toml
//! [dependencies]
//! rbc-rs = { version = "0.1", default-features = false }
//! ```
//!
//! [`reconchess`]: https://github.com/reconnaissanceblindchess/reconchess

#![warn(missing_docs)]

mod attack_tables;

mod types;

mod position;

mod game;

pub use game::Game;
pub use types::{
    Capture, CastlingPolicy, Color, DrawReason, Error, GameConfig, GameResult, GameStatus,
    HistoryEntry, Move, MoveOutcome, MoveStatus, Piece, PieceKind, SenseResult, SensedSquare,
    Square, Variant, WinReason,
};

/// Crate version from Cargo metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
