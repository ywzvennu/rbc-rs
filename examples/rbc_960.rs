//! Set up an RBC game with a Chess960 starting position.
//!
//! Run with `cargo run --example rbc_960`.

use rbc_rs::{Game, GameConfig};

fn main() {
    // Mirrored Chess960 — both sides start with the same back rank.
    // SP-ID 518 is the canonical FIDE starting position; pick any
    // other 0..960 for a shuffle variant.
    let game = Game::new_rbc_960(518, GameConfig::default()).expect("valid SP-ID");
    println!("rbc-960 sp_id=518 (FIDE): {}", game.to_fen());

    let shuffled = Game::new_rbc_960(0, GameConfig::default()).expect("valid SP-ID");
    println!("rbc-960 sp_id=0:        {}", shuffled.to_fen());

    // Random, deterministic in seed.
    let random = Game::new_rbc_960_random(0xC0FFEE, GameConfig::default());
    println!("rbc-960 random(0xC0FFEE): {}", random.to_fen());

    // Squared: white and black draw independently.
    let squared = Game::new_rbc_960_squared(0, 959, GameConfig::default()).expect("valid SP-IDs");
    println!("rbc-960² white=0 black=959: {}", squared.to_fen());
}
