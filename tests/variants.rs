//! Integration tests for the variant-driven starting positions.

use chess_startpos_rs::chess as csp;
use rbc_rs::{Game, GameConfig};

const STANDARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[test]
fn rbc_960_sp_id_518_is_standard_starting_position() {
    // Chess960 SP-ID 518 is the canonical FIDE starting position.
    let game = Game::new_rbc_960(518, GameConfig::default()).unwrap();
    assert_eq!(game.to_fen(), STANDARD_FEN);
}

#[test]
fn rbc_960_sp_id_out_of_range_rejected() {
    let err = Game::new_rbc_960(960, GameConfig::default());
    assert!(err.is_err());
}

#[test]
fn rbc_960_random_is_deterministic_in_seed() {
    let a = Game::new_rbc_960_random(0xDEAD_BEEF, GameConfig::default()).to_fen();
    let b = Game::new_rbc_960_random(0xDEAD_BEEF, GameConfig::default()).to_fen();
    assert_eq!(a, b);
}

#[test]
fn rbc_960_squared_distinct_ids_yield_distinct_ranks() {
    // SP-ID 518 = standard; SP-ID 0 = a different arrangement.
    let game = Game::new_rbc_960_squared(0, 518, GameConfig::default()).unwrap();
    let fen = game.to_fen();
    let placement = fen.split_whitespace().next().unwrap();
    let ranks: Vec<&str> = placement.split('/').collect();
    let rank_8_lower = ranks[0].to_lowercase();
    let rank_1_lower = ranks[7].to_lowercase();
    assert_ne!(
        rank_8_lower, rank_1_lower,
        "white ≠ black rank for distinct ids"
    );
}

#[test]
fn rbc_960_mirrored_makes_rank_1_equal_rank_8_modulo_case() {
    let game = Game::new_rbc_960(0, GameConfig::default()).unwrap();
    let fen = game.to_fen();
    let placement = fen.split_whitespace().next().unwrap();
    let ranks: Vec<&str> = placement.split('/').collect();
    assert_eq!(ranks[0], ranks[7].to_lowercase());
}

#[test]
fn rbc_960_from_backrank_rejects_invalid_arrangement() {
    // Two queens, no king — not a valid Chess960 setup.
    let bad = [
        csp::Piece::Queen,
        csp::Piece::Queen,
        csp::Piece::Rook,
        csp::Piece::Rook,
        csp::Piece::Bishop,
        csp::Piece::Bishop,
        csp::Piece::Knight,
        csp::Piece::Knight,
    ];
    let err = Game::new_rbc_960_from_backrank(bad, GameConfig::default());
    assert!(err.is_err());
}

#[test]
fn rbc_960_from_backrank_accepts_chess_960_position() {
    let backrank = csp::chess_960().sp_id(518).unwrap();
    let arr: [csp::Piece; 8] = backrank.try_into().unwrap();
    let game = Game::new_rbc_960_from_backrank(arr, GameConfig::default()).unwrap();
    assert_eq!(game.to_fen(), STANDARD_FEN);
}

#[test]
fn rbc_2880_random_is_deterministic_in_seed() {
    let a = Game::new_rbc_2880_random(42, GameConfig::default())
        .unwrap()
        .to_fen();
    let b = Game::new_rbc_2880_random(42, GameConfig::default())
        .unwrap()
        .to_fen();
    assert_eq!(a, b);
}

#[test]
fn rbc_2880_index_out_of_range_rejected() {
    assert!(Game::new_rbc_2880(2880, GameConfig::default()).is_err());
}

#[test]
fn rbc_shuffle_index_out_of_range_rejected() {
    assert!(Game::new_rbc_shuffle(5040, GameConfig::default()).is_err());
}

#[test]
fn rbc_shuffle_squared_random_yields_two_full_ranks() {
    let game = Game::new_rbc_shuffle_squared_random(7, GameConfig::default()).unwrap();
    let fen = game.to_fen();
    let placement = fen.split_whitespace().next().unwrap();
    let ranks: Vec<&str> = placement.split('/').collect();
    assert_eq!(ranks.len(), 8);
    // Rank 1 (white back rank) and rank 8 (black) each have exactly
    // 8 pieces (no empty squares on the back rank for a valid
    // arrangement).
    assert_eq!(ranks[0].len(), 8); // black
    assert_eq!(ranks[7].len(), 8); // white
}

#[test]
fn standard_variant_default_unchanged() {
    let game = Game::new(GameConfig::default());
    assert_eq!(game.to_fen(), STANDARD_FEN);
}
