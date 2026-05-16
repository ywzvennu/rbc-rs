use rbc_rs::{
    Color, DrawReason, Error, Game, GameConfig, GameResult, GameStatus, Move, MoveStatus, Piece,
    PieceKind, Square, WinReason,
};

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).expect("valid square")
}

fn mv(from: (u8, u8), to: (u8, u8)) -> Move {
    Move {
        from: sq(from.0, from.1),
        to: sq(to.0, to.1),
        promotion: None,
    }
}

#[test]
fn sensing_preserves_window_order_at_edges_and_center() {
    let mut game = Game::new(GameConfig::default());

    let a8: Vec<_> = game
        .sense(Some(sq(0, 7)))
        .squares
        .into_iter()
        .map(|entry| entry.square)
        .collect();
    assert_eq!(a8, vec![sq(0, 7), sq(1, 7), sq(0, 6), sq(1, 6)]);

    let e4: Vec<_> = game
        .sense(Some(sq(4, 3)))
        .squares
        .into_iter()
        .map(|entry| entry.square)
        .collect();
    assert_eq!(
        e4,
        vec![
            sq(3, 4),
            sq(4, 4),
            sq(5, 4),
            sq(3, 3),
            sq(4, 3),
            sq(5, 3),
            sq(3, 2),
            sq(4, 2),
            sq(5, 2),
        ]
    );

    let h1: Vec<_> = game
        .sense(Some(sq(7, 0)))
        .squares
        .into_iter()
        .map(|entry| entry.square)
        .collect();
    assert_eq!(h1, vec![sq(6, 1), sq(7, 1), sq(6, 0), sq(7, 0)]);
}

#[test]
fn move_actions_include_hidden_information_requests() {
    let game = Game::new(GameConfig::default());
    assert!(game.move_actions().contains(&mv((0, 1), (1, 2))));
    assert!(!game.move_actions().contains(&mv((4, 1), (4, 4))));

    let black_to_move = Game::from_fen(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
        GameConfig::default(),
    )
    .unwrap();
    assert!(black_to_move.move_actions().contains(&mv((0, 6), (1, 5))));

    let rook_game =
        Game::from_fen("4k3/8/8/3R1p2/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
    assert!(rook_game.move_actions().contains(&mv((3, 4), (7, 4))));
}

#[test]
fn sliders_revise_to_the_first_opponent_piece() {
    let mut rook_game = Game::from_fen(
        "4k3/3p4/8/1p1R1p2/8/8/8/4K3 w - - 0 1",
        GameConfig::default(),
    )
    .unwrap();
    let rook_outcome = rook_game.apply_move(Some(mv((3, 4), (7, 4)))).unwrap();
    assert_eq!(rook_outcome.status, MoveStatus::Revised);
    assert_eq!(rook_outcome.taken, Some(mv((3, 4), (5, 4))));
    assert_eq!(rook_outcome.capture.unwrap().square, sq(5, 4));

    let mut bishop_game = Game::from_fen(
        "p5p1/4k3/8/3B4/8/8/p5p1/4K3 w - - 0 1",
        GameConfig::default(),
    )
    .unwrap();
    let bishop_outcome = bishop_game.apply_move(Some(mv((3, 4), (7, 0)))).unwrap();
    assert_eq!(bishop_outcome.status, MoveStatus::Revised);
    assert_eq!(bishop_outcome.taken, Some(mv((3, 4), (6, 1))));
    assert_eq!(bishop_outcome.capture.unwrap().square, sq(6, 1));
}

#[test]
fn pawn_double_step_revises_when_destination_is_occupied() {
    let mut game =
        Game::from_fen("4k3/8/8/8/3p4/8/3P4/4K3 w - - 0 1", GameConfig::default()).unwrap();
    let outcome = game.apply_move(Some(mv((3, 1), (3, 3)))).unwrap();
    assert_eq!(outcome.status, MoveStatus::Revised);
    assert_eq!(outcome.taken, Some(mv((3, 1), (3, 2))));
    assert_eq!(outcome.capture, None);
}

#[test]
fn castling_ignores_check_but_honors_path_and_rights() {
    let mut into_check =
        Game::from_fen("4k3/8/8/8/6q1/8/8/4K2R w K - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        into_check
            .apply_move(Some(mv((4, 0), (6, 0))))
            .unwrap()
            .status,
        MoveStatus::Taken
    );

    let mut blocked =
        Game::from_fen("4k3/8/8/8/8/8/8/4K1nR w K - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        blocked.apply_move(Some(mv((4, 0), (6, 0)))).unwrap().status,
        MoveStatus::Illegal
    );

    let mut no_rights =
        Game::from_fen("4k3/8/8/8/8/8/8/4K2R w - - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        no_rights.apply_move(Some(mv((4, 0), (6, 0)))),
        Err(Error::InvalidMove)
    );
}

#[test]
fn castling_variants_match_upstream_behavior() {
    let mut queenside =
        Game::from_fen("4k3/8/8/8/8/8/8/R3K2R w Q - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        queenside
            .apply_move(Some(mv((4, 0), (2, 0))))
            .unwrap()
            .status,
        MoveStatus::Taken
    );
    assert_eq!(queenside.to_fen(), "4k3/8/8/8/8/8/8/2KR3R b - - 1 1");

    let mut out_of_check =
        Game::from_fen("4k3/8/8/8/8/8/8/q3K2R w K - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        out_of_check
            .apply_move(Some(mv((4, 0), (6, 0))))
            .unwrap()
            .status,
        MoveStatus::Taken
    );

    let mut stays_in_check =
        Game::from_fen("4k3/8/8/8/8/6q1/8/4K2R w K - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        stays_in_check
            .apply_move(Some(mv((4, 0), (6, 0))))
            .unwrap()
            .status,
        MoveStatus::Taken
    );

    let mut queenside_blocked =
        Game::from_fen("4k3/8/8/8/8/8/8/R1n1K3 w Q - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        queenside_blocked
            .apply_move(Some(mv((4, 0), (2, 0))))
            .unwrap()
            .status,
        MoveStatus::Illegal
    );

    let mut queenside_no_rights =
        Game::from_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        queenside_no_rights.apply_move(Some(mv((4, 0), (2, 0)))),
        Err(Error::InvalidMove)
    );
}

#[test]
fn en_passant_reports_capture_square_for_both_colors() {
    let mut white_push =
        Game::from_fen("4k3/8/8/8/1p6/8/P7/4K3 w - - 0 1", GameConfig::default()).unwrap();
    white_push.apply_move(Some(mv((0, 1), (0, 3)))).unwrap();
    let black_capture = white_push.apply_move(Some(mv((1, 3), (0, 2)))).unwrap();
    assert_eq!(black_capture.capture.unwrap().square, sq(0, 3));

    let mut black_push =
        Game::from_fen("4k3/5p2/8/6P1/8/8/8/4K3 b - - 0 1", GameConfig::default()).unwrap();
    black_push.apply_move(Some(mv((5, 6), (5, 4)))).unwrap();
    let white_capture = black_push.apply_move(Some(mv((6, 4), (5, 5)))).unwrap();
    assert_eq!(white_capture.capture.unwrap().square, sq(5, 4));
}

#[test]
fn omitted_promotion_defaults_to_queen() {
    let mut game = Game::from_fen("7k/3P4/8/8/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
    let outcome = game.apply_move(Some(mv((3, 6), (3, 7)))).unwrap();
    assert_eq!(outcome.status, MoveStatus::Taken);
    assert_eq!(outcome.taken.unwrap().promotion, Some(PieceKind::Queen));
    assert_eq!(
        game.piece_at(sq(3, 7)),
        Some(Piece {
            color: Color::White,
            kind: PieceKind::Queen,
        })
    );
}

#[test]
fn omitted_promotion_capture_action_matches_upstream() {
    let mut game =
        Game::from_fen("1r5k/P7/8/8/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
    assert!(game.move_actions().contains(&mv((0, 6), (1, 7))));

    let outcome = game.apply_move(Some(mv((0, 6), (1, 7)))).unwrap();
    assert_eq!(outcome.status, MoveStatus::Taken);
    assert_eq!(outcome.taken.unwrap().promotion, Some(PieceKind::Queen));
}

#[test]
fn invalid_requests_match_move_action_contract() {
    let mut game = Game::new(GameConfig::default());
    assert_eq!(
        game.apply_move(Some(mv((4, 6), (4, 4)))),
        Err(Error::InvalidMove)
    );
    assert_eq!(
        game.apply_move(Some(mv((4, 3), (4, 4)))),
        Err(Error::InvalidMove)
    );
    assert_eq!(
        game.apply_move(Some(mv((1, 0), (1, 2)))),
        Err(Error::InvalidMove)
    );
}

#[test]
fn pass_moves_consume_turn_without_capture() {
    let mut game = Game::new(GameConfig::default());
    let outcome = game.apply_move(None).unwrap();
    assert_eq!(outcome.status, MoveStatus::Pass);
    assert_eq!(outcome.capture, None);
    assert_eq!(game.turn(), Some(Color::Black));
}

#[test]
fn opponent_capture_square_tracks_the_latest_opponent_turn() {
    let mut game =
        Game::from_fen("4k3/8/8/3pp3/3PP3/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
    game.apply_move(Some(mv((3, 3), (4, 4)))).unwrap();
    assert_eq!(game.opponent_capture_square(Color::Black), Some(sq(4, 4)));

    let _ = game.sense(Some(sq(4, 4)));
    assert_eq!(game.opponent_capture_square(Color::Black), Some(sq(4, 4)));

    game.apply_move(None).unwrap();
    assert_eq!(game.opponent_capture_square(Color::White), None);
}

#[test]
fn terminal_states_and_history_round_trip() {
    let mut capture_game =
        Game::from_fen("4k3/4Q3/8/8/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
    let _ = capture_game.sense(None);
    capture_game.apply_move(Some(mv((4, 6), (4, 7)))).unwrap();
    assert_eq!(
        capture_game.status(),
        &GameStatus::Won(GameResult {
            winner: Color::White,
            reason: WinReason::KingCapture,
        })
    );

    #[cfg(feature = "serde")]
    {
        let json = serde_json::to_string(&capture_game).unwrap();
        let decoded: Game = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.status(), capture_game.status());
        assert_eq!(decoded.history(), capture_game.history());
        assert_eq!(decoded.to_fen(), capture_game.to_fen());
    }

    let mut draw_config = GameConfig::default();
    draw_config.reversible_moves_limit = Some(2);
    draw_config.full_turn_limit = None;
    let mut draw_game = Game::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1", draw_config).unwrap();
    draw_game.apply_move(None).unwrap();
    draw_game.apply_move(None).unwrap();
    assert_eq!(
        draw_game.status(),
        &GameStatus::Draw {
            reason: DrawReason::MoveLimit,
        }
    );
}

#[test]
fn constructors_surface_upstream_terminal_conditions() {
    let no_black_king =
        Game::from_fen("8/8/8/8/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
    assert_eq!(
        no_black_king.status(),
        &GameStatus::Won(GameResult {
            winner: Color::White,
            reason: WinReason::KingCapture,
        })
    );

    let mut turn_limit_config = GameConfig::default();
    turn_limit_config.reversible_moves_limit = None;
    turn_limit_config.full_turn_limit = Some(1);
    let turn_limit = Game::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 2", turn_limit_config).unwrap();
    assert_eq!(
        turn_limit.status(),
        &GameStatus::Draw {
            reason: DrawReason::TurnLimit,
        }
    );
}
