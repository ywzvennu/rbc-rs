use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use reconchess_rs::{Game, GameConfig, Move, Square};
use std::hint::black_box;

const MIDGAME_FEN: &str = "r1bq1rk1/pp2bppp/2n1pn2/2pp4/3P4/2N1PN2/PPPBBPPP/R2Q1RK1 w - - 0 8";

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

criterion_group!(
    benches,
    bench_move_actions_start,
    bench_move_actions_midgame,
    bench_apply_move_sequence,
    bench_sense_window,
    bench_position_from_fen,
    bench_position_to_fen,
);
criterion_main!(benches);
