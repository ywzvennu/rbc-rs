//! Set up an RBC game with a Chess960 starting position by combining
//! [`chess_startpos_rs`] with [`rbc_rs`].
//!
//! `rbc-rs` does not depend on `chess-startpos-rs` directly — the
//! glue layer (this file) imports both and converts at the boundary.
//! In a real game server, that conversion + matchmaking logic lives
//! in your server crate.
//!
//! Run with `cargo run --example rbc_960`.

use chess_startpos_rs::chess as csp;
use rbc_rs::{Game, GameConfig, PieceKind};

fn convert(p: csp::Piece) -> PieceKind {
    match p {
        csp::Piece::King => PieceKind::King,
        csp::Piece::Queen => PieceKind::Queen,
        csp::Piece::Rook => PieceKind::Rook,
        csp::Piece::Bishop => PieceKind::Bishop,
        csp::Piece::Knight => PieceKind::Knight,
    }
}

fn convert_arr(arr: Vec<csp::Piece>) -> [PieceKind; 8] {
    let mut out = [PieceKind::King; 8];
    for (i, &p) in arr.iter().enumerate() {
        out[i] = convert(p);
    }
    out
}

fn main() {
    // Chess960 SP-ID 518 = the canonical FIDE starting position.
    let mut config = GameConfig::default();
    config.white_backrank = convert_arr(csp::chess_960().sp_id(518).unwrap());
    config.black_backrank = config.white_backrank;
    let fide = Game::new(config);
    println!("rbc-960 sp_id=518 (FIDE): {}", fide.to_fen());

    // SP-ID 0 — a non-standard arrangement.
    let mut config = GameConfig::default();
    config.white_backrank = convert_arr(csp::chess_960().sp_id(0).unwrap());
    config.black_backrank = config.white_backrank;
    let mirrored = Game::new(config);
    println!("rbc-960 sp_id=0 mirrored:   {}", mirrored.to_fen());

    // Random arrangement, deterministic in the seed your server picks.
    let mut config = GameConfig::default();
    let arr = convert_arr(csp::chess_960().sample(0xC0FFEE));
    config.white_backrank = arr;
    config.black_backrank = arr;
    let random = Game::new(config);
    println!("rbc-960 random(0xC0FFEE):   {}", random.to_fen());

    // Squared (RBC²) — white and black draw independently.
    let mut config = GameConfig::default();
    config.white_backrank = convert_arr(csp::chess_960().sp_id(0).unwrap());
    config.black_backrank = convert_arr(csp::chess_960().sp_id(959).unwrap());
    let squared = Game::new(config);
    println!("rbc-960² white=0 black=959: {}", squared.to_fen());
}
