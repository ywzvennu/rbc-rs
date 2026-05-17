//! Integration tests for the back-rank-driven starting positions.

use chess_startpos_rs::chess as csp;
use rbc_rs::{CastlingPolicy, Color, Game, GameConfig, PieceKind, STANDARD_BACK_RANK};

const STANDARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Converts a `chess_startpos_rs::chess::Piece` to rbc-rs's
/// [`PieceKind`]. Lives at the consumer boundary — rbc-rs's public
/// API does not depend on chess-startpos-rs.
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

#[test]
fn default_config_is_fide_standard() {
    let game = Game::new(GameConfig::default());
    assert_eq!(game.to_fen(), STANDARD_FEN);
}

#[test]
fn explicit_fide_backranks_match_default() {
    let mut config = GameConfig::default();
    config.white_backrank = STANDARD_BACK_RANK;
    config.black_backrank = STANDARD_BACK_RANK;
    let game = Game::new(config);
    assert_eq!(game.to_fen(), STANDARD_FEN);
}

#[test]
fn mirrored_back_ranks_match() {
    // BBQNNRKR — Chess960 SP-ID 0.
    let arr: [PieceKind; 8] = convert_arr(csp::chess_960().sp_id(0).unwrap());
    let mut config = GameConfig::default();
    config.white_backrank = arr;
    config.black_backrank = arr;
    let game = Game::new(config);
    let placement = game.to_fen().split_whitespace().next().unwrap().to_string();
    let ranks: Vec<&str> = placement.split('/').collect();
    assert_eq!(ranks[0], ranks[7].to_lowercase());
}

#[test]
fn independent_back_ranks_differ() {
    let mut config = GameConfig::default();
    config.white_backrank = convert_arr(csp::chess_960().sp_id(0).unwrap());
    config.black_backrank = convert_arr(csp::chess_960().sp_id(959).unwrap());
    let game = Game::new(config);
    let placement = game.to_fen().split_whitespace().next().unwrap().to_string();
    let ranks: Vec<&str> = placement.split('/').collect();
    assert_ne!(ranks[0], ranks[7].to_lowercase());
}

#[test]
fn chess_960_sp_id_0_yields_expected_fen() {
    // BBQNNRKR — bishops on a/b, queen on c, knights on d/e, rook-king-rook on f/g/h.
    let arr = convert_arr(csp::chess_960().sp_id(0).unwrap());
    let mut config = GameConfig::default();
    config.white_backrank = arr;
    config.black_backrank = arr;
    let game = Game::new(config);
    assert_eq!(
        game.to_fen(),
        "bbqnnrkr/pppppppp/8/8/8/8/PPPPPPPP/BBQNNRKR w HFhf - 0 1"
    );
}

#[test]
fn chess_960_sp_id_518_yields_standard_fen() {
    let arr = convert_arr(csp::chess_960().sp_id(518).unwrap());
    let mut config = GameConfig::default();
    config.white_backrank = arr;
    config.black_backrank = arr;
    let game = Game::new(config);
    assert_eq!(game.to_fen(), STANDARD_FEN);
}

#[test]
fn castling_policy_suppresses_white_kingside_right() {
    let mut config = GameConfig::default();
    config.castling_policy = CastlingPolicy {
        white_kingside: false,
        ..CastlingPolicy::default()
    };
    let game = Game::new(config);
    // White had a kingside rook (FIDE default) but the policy
    // suppressed the right — the castling field omits K.
    let fen = game.to_fen();
    let castling = fen.split_whitespace().nth(2).unwrap();
    assert!(!castling.contains('K'));
    assert!(castling.contains('Q'));
    assert!(castling.contains('k'));
    assert!(castling.contains('q'));
}

#[test]
fn castling_policy_all_false_yields_no_castling_field() {
    let mut config = GameConfig::default();
    config.castling_policy = CastlingPolicy {
        white_kingside: false,
        white_queenside: false,
        black_kingside: false,
        black_queenside: false,
    };
    let game = Game::new(config);
    let castling = game.to_fen().split_whitespace().nth(2).unwrap().to_string();
    assert_eq!(castling, "-");
}

#[test]
fn missing_king_in_backrank_rejected() {
    // No king anywhere — Position::from_starting_backranks should
    // refuse and Game::new will panic via expect. Use catch_unwind
    // to assert that's what happens.
    let mut config = GameConfig::default();
    config.white_backrank = [PieceKind::Pawn; 8];
    config.black_backrank = STANDARD_BACK_RANK;
    let result = std::panic::catch_unwind(|| Game::new(config));
    assert!(result.is_err());
}

#[test]
fn turn_is_white_after_new() {
    let game = Game::new(GameConfig::default());
    assert_eq!(game.turn(), Some(Color::White));
}
