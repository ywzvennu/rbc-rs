use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use rbc_rs::{Game, GameConfig, Move, Square};
use std::hint::black_box;

const MIDGAME_FEN: &str = "r1bq1rk1/pp2bppp/2n1pn2/2pp4/3P4/2N1PN2/PPPBBPPP/R2Q1RK1 w - - 0 8";

// Slider-heavy midgame: white queen, two rooks, bishop active on open files
// and diagonals. Used to exercise slider move generation.
const SLIDER_HEAVY_FEN: &str = "r3k2r/ppp2ppp/2n5/3qp3/3P4/2N2B2/PPP2PPP/R2Q1RK1 w kq - 0 10";

// Rook on a1, white king on e2, opponent pawn on e1, opponent king on e8.
// Apply Ra1->h1: the new revise_slider_move walks the ray (~7 squares) and
// revises to e1 where the opponent pawn blocks.
const ROOK_REVISE_FEN: &str = "4k3/8/8/8/8/8/4K3/R3p3 w - - 0 1";

// Rook on a1, white king on e2, opponent king on e8, otherwise empty.
// Apply Ra1->h1: full clear ray; both validation and revise walk to the
// end without finding a blocker.
const ROOK_CLEAR_FEN: &str = "4k3/8/8/8/8/8/4K3/R7 w - - 0 1";

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

fn opening_sequence() -> Vec<Move> {
    vec![
        mv((4, 1), (4, 3)),
        mv((4, 6), (4, 4)),
        mv((6, 0), (5, 2)),
        mv((1, 7), (2, 5)),
        mv((5, 0), (2, 3)),
        mv((5, 7), (2, 4)),
        mv((3, 1), (3, 2)),
        mv((3, 6), (3, 5)),
    ]
}

/// Morphy vs Duke of Brunswick & Count Isouard, Paris 1858 — the "Opera
/// Game". 33 plies; exercises pawn pushes/captures, knight moves,
/// bishop trades, queen activity, castling (queenside), and a series of
/// rook moves down an open file. A real chess game's worth of slider
/// activity, enough total work for criterion to surface deltas under
/// per-op noise.
fn opera_game() -> Vec<Move> {
    vec![
        mv((4, 1), (4, 3)), // 1. e4
        mv((4, 6), (4, 4)), // 1... e5
        mv((6, 0), (5, 2)), // 2. Nf3
        mv((3, 6), (3, 5)), // 2... d6
        mv((3, 1), (3, 3)), // 3. d4
        mv((2, 7), (6, 3)), // 3... Bg4
        mv((3, 3), (4, 4)), // 4. dxe5
        mv((6, 3), (5, 2)), // 4... Bxf3
        mv((3, 0), (5, 2)), // 5. Qxf3
        mv((3, 5), (4, 4)), // 5... dxe5
        mv((5, 0), (2, 3)), // 6. Bc4
        mv((6, 7), (5, 5)), // 6... Nf6
        mv((5, 2), (1, 2)), // 7. Qb3
        mv((3, 7), (4, 6)), // 7... Qe7
        mv((1, 0), (2, 2)), // 8. Nc3
        mv((2, 6), (2, 5)), // 8... c6
        mv((2, 0), (6, 4)), // 9. Bg5
        mv((1, 6), (1, 4)), // 9... b5
        mv((2, 2), (1, 4)), // 10. Nxb5
        mv((2, 5), (1, 4)), // 10... cxb5
        mv((2, 3), (1, 4)), // 11. Bxb5+
        mv((1, 7), (3, 6)), // 11... Nbd7
        mv((4, 0), (2, 0)), // 12. O-O-O
        mv((0, 7), (3, 7)), // 12... Rd8
        mv((3, 0), (3, 6)), // 13. Rxd7
        mv((3, 7), (3, 6)), // 13... Rxd7
        mv((7, 0), (3, 0)), // 14. Rd1
        mv((4, 6), (4, 5)), // 14... Qe6
        mv((1, 4), (3, 6)), // 15. Bxd7+
        mv((5, 5), (3, 6)), // 15... Nxd7
        mv((1, 2), (1, 7)), // 16. Qb8+
        mv((3, 6), (1, 7)), // 16... Nxb8
        mv((3, 0), (3, 7)), // 17. Rd8#
    ]
}

fn bench_move_actions_start(c: &mut Criterion) {
    let game = Game::new(GameConfig::default());
    c.bench_function("move_actions_start", |b| {
        b.iter(|| black_box(&game).move_actions())
    });
}

fn bench_move_actions_midgame(c: &mut Criterion) {
    let game = Game::from_fen(MIDGAME_FEN, GameConfig::default()).expect("valid FEN");
    c.bench_function("move_actions_midgame", |b| {
        b.iter(|| black_box(&game).move_actions())
    });
}

fn bench_apply_move_sequence(c: &mut Criterion) {
    let initial = Game::new(GameConfig::default());
    let moves = opening_sequence();
    c.bench_function("apply_move_sequence", |b| {
        b.iter_batched(
            || initial.clone(),
            |mut game| {
                for mv in &moves {
                    let _ = game.apply_move(Some(*mv));
                }
                game
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_sense_window(c: &mut Criterion) {
    let mut group = c.benchmark_group("sense");
    group.bench_function("corner", |b| {
        let mut game = Game::new(GameConfig::default());
        b.iter(|| black_box(&mut game).sense(Some(sq(0, 7))))
    });
    group.bench_function("center", |b| {
        let mut game = Game::new(GameConfig::default());
        b.iter(|| black_box(&mut game).sense(Some(sq(4, 3))))
    });
    group.finish();
}

fn bench_position_from_fen(c: &mut Criterion) {
    c.bench_function("position_from_fen", |b| {
        b.iter(|| Game::from_fen(black_box(MIDGAME_FEN), GameConfig::default()))
    });
}

fn bench_position_to_fen(c: &mut Criterion) {
    let game = Game::from_fen(MIDGAME_FEN, GameConfig::default()).expect("valid FEN");
    c.bench_function("position_to_fen", |b| b.iter(|| black_box(&game).to_fen()));
}

fn bench_move_actions_slider_heavy(c: &mut Criterion) {
    let game = Game::from_fen(SLIDER_HEAVY_FEN, GameConfig::default()).expect("valid FEN");
    c.bench_function("move_actions_slider_heavy", |b| {
        b.iter(|| black_box(&game).move_actions())
    });
}

fn bench_apply_rook_revised(c: &mut Criterion) {
    let base = Game::from_fen(ROOK_REVISE_FEN, GameConfig::default()).expect("valid FEN");
    let request = mv((0, 0), (7, 0));
    c.bench_function("apply_rook_revised", |b| {
        b.iter_batched(
            || base.clone(),
            |mut game| {
                let _ = game.apply_move(Some(request));
                game
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_apply_rook_clear(c: &mut Criterion) {
    let base = Game::from_fen(ROOK_CLEAR_FEN, GameConfig::default()).expect("valid FEN");
    let request = mv((0, 0), (7, 0));
    c.bench_function("apply_rook_clear", |b| {
        b.iter_batched(
            || base.clone(),
            |mut game| {
                let _ = game.apply_move(Some(request));
                game
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_apply_opera_game(c: &mut Criterion) {
    let initial = Game::new(GameConfig::default());
    let moves = opera_game();
    c.bench_function("apply_opera_game", |b| {
        b.iter_batched(
            || initial.clone(),
            |mut game| {
                for mv in &moves {
                    let _ = game.apply_move(Some(*mv));
                }
                game
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_move_actions_start,
    bench_move_actions_midgame,
    bench_move_actions_slider_heavy,
    bench_apply_move_sequence,
    bench_apply_opera_game,
    bench_apply_rook_revised,
    bench_apply_rook_clear,
    bench_sense_window,
    bench_position_from_fen,
    bench_position_to_fen,
);
criterion_main!(benches);
