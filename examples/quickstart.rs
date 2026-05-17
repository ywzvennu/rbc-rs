//! Minimal end-to-end demonstration of driving an RBC game with `rbc-rs`.
//!
//! Run with `cargo run --example quickstart`.

use rbc_rs::{Color, Game, GameConfig, Move, SenseAction, Square};

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).expect("valid square")
}

/// Pick the action for the given center from the current player's
/// available actions. Today every default GameConfig has exactly one
/// sense token per side, so each center appears in `sense_actions()`
/// at most once.
fn sense_at(game: &mut Game, center: Square) -> rbc_rs::SenseResult {
    let action: SenseAction = game
        .sense_actions()
        .into_iter()
        .find(|a| a.center == center)
        .expect("center available");
    game.sense_with(action).expect("valid action")
}

fn main() {
    // 1. Start a fresh game from the standard RBC starting position.
    let mut game = Game::new(GameConfig::default());
    println!("starting FEN: {}", game.to_fen());

    // 2. The side to move can sense any 3×3 window. Sensing the centre of
    //    the board returns a full nine-square window; sensing a corner
    //    returns a clipped window. Today's default config has one sense
    //    token per side; multi-token variants land in a future minor.
    let centre_window = sense_at(&mut game, sq(4, 3));
    println!("centre window squares: {}", centre_window.squares.len());
    // Note: the per-turn token has been used, so we can't sense again
    // this turn. To sense a corner too we'd need black's turn (or a
    // future multi-token policy).

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
