use reconchess_rs::{Game, GameConfig, Move, MoveStatus, Square};

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

/// Replays Morphy vs Duke of Brunswick (Opera Game, Paris 1858) move by
/// move and asserts each request was taken (or revised) — never illegal.
/// Keeps the parallel sequence in benches/game.rs honest: if any move
/// fails to apply this test fails before the bench masks the issue.
#[test]
fn opera_game_completes_without_illegal_moves() {
    let mut game = Game::new(GameConfig::default());

    let moves = [
        ("1. e4", mv((4, 1), (4, 3))),
        ("1... e5", mv((4, 6), (4, 4))),
        ("2. Nf3", mv((6, 0), (5, 2))),
        ("2... d6", mv((3, 6), (3, 5))),
        ("3. d4", mv((3, 1), (3, 3))),
        ("3... Bg4", mv((2, 7), (6, 3))),
        ("4. dxe5", mv((3, 3), (4, 4))),
        ("4... Bxf3", mv((6, 3), (5, 2))),
        ("5. Qxf3", mv((3, 0), (5, 2))),
        ("5... dxe5", mv((3, 5), (4, 4))),
        ("6. Bc4", mv((5, 0), (2, 3))),
        ("6... Nf6", mv((6, 7), (5, 5))),
        ("7. Qb3", mv((5, 2), (1, 2))),
        ("7... Qe7", mv((3, 7), (4, 6))),
        ("8. Nc3", mv((1, 0), (2, 2))),
        ("8... c6", mv((2, 6), (2, 5))),
        ("9. Bg5", mv((2, 0), (6, 4))),
        ("9... b5", mv((1, 6), (1, 4))),
        ("10. Nxb5", mv((2, 2), (1, 4))),
        ("10... cxb5", mv((2, 5), (1, 4))),
        ("11. Bxb5+", mv((2, 3), (1, 4))),
        ("11... Nbd7", mv((1, 7), (3, 6))),
        ("12. O-O-O", mv((4, 0), (2, 0))),
        ("12... Rd8", mv((0, 7), (3, 7))),
        ("13. Rxd7", mv((3, 0), (3, 6))),
        ("13... Rxd7", mv((3, 7), (3, 6))),
        ("14. Rd1", mv((7, 0), (3, 0))),
        ("14... Qe6", mv((4, 6), (4, 5))),
        ("15. Bxd7+", mv((1, 4), (3, 6))),
        ("15... Nxd7", mv((5, 5), (3, 6))),
        ("16. Qb8+", mv((1, 2), (1, 7))),
        ("16... Nxb8", mv((3, 6), (1, 7))),
        ("17. Rd8#", mv((3, 0), (3, 7))),
    ];

    for (label, requested) in moves {
        let outcome = game
            .apply_move(Some(requested))
            .unwrap_or_else(|err| panic!("{label}: apply_move returned {err:?}"));
        assert!(
            matches!(outcome.status, MoveStatus::Taken | MoveStatus::Revised),
            "{label}: status was {:?}",
            outcome.status,
        );
    }
}
