//! Minimal end-to-end demonstration of driving an RBC game with `rbc-rs`.
//!
//! Run with `cargo run --example quickstart`.

use rbc_rs::{Color, Game, GameConfig, Move, Square};

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).expect("valid square")
}

fn main() {
    // 1. Start a fresh game from the standard RBC starting position.
    let mut game = Game::new(GameConfig::default());
    println!("starting FEN: {}", game.to_fen());

    // 2. The side to move can sense any 3×3 window. Sensing the centre of
    //    the board returns a full nine-square window; sensing a corner
    //    returns a clipped window.
    let centre_window = game.sense(Some(sq(4, 3)));
    println!("centre window squares: {}", centre_window.squares.len());

    let corner_window = game.sense(Some(sq(0, 0)));
    println!("corner window squares: {}", corner_window.squares.len());

    // 3. `move_actions` returns every move the acting player can request
    //    given only their own-piece view. This includes pawn capture
    //    attempts against unseen opponent pieces.
    let actions = game.move_actions();
    println!("move actions at start: {}", actions.len());

    // 4. Apply 1. e2-e4. Sliders block as expected and captures are
    //    reported; here the request is accepted unchanged.
    let e2_e4 = Move {
        from: sq(4, 1),
        to: sq(4, 3),
        promotion: None,
    };
    let outcome = game.apply_move(Some(e2_e4)).expect("legal request");
    println!(
        "1. e4 — status={:?} taken={:?} capture={:?}",
        outcome.status, outcome.taken, outcome.capture,
    );
    assert_eq!(game.turn(), Some(Color::Black));

    // 5. The full turn history — sense result, requested vs taken move,
    //    capture, and FEN before/after — is available for every completed
    //    turn.
    let entry = game.history().last().expect("one turn recorded");
    println!("history[0].color = {:?}", entry.color);
    println!("history[0].fen_before_move = {}", entry.fen_before_move);
    println!("history[0].fen_after_move  = {}", entry.fen_after_move);
}
