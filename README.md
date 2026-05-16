# rbc-rs

The Rust implementation of **Reconnaissance Blind Chess (RBC)** — a variant of
chess in which each player can see only their own pieces and must spend one
"sense" action per turn revealing a 3×3 window of the opponent's position
before committing to a move. The game was developed and is run as an annual
research tournament by the [Johns Hopkins University Applied Physics
Laboratory][jhuapl] at [rbc.jhuapl.edu][rbc].

`rbc-rs` aims to be the canonical Rust home for the rules layer: behaviourally
equivalent to the upstream Python [`reconchess`][reconchess-py] package, with
in-tree implementations of board state, FEN parsing, move generation, move
resolution, sensing, history, and result adjudication. No external chess
engine dependency.

## What this crate is (and isn't)

In scope:

- Game-playing core — `Game`, sense actions, move actions, move resolution,
  capture and king-capture handling, draw/win/timeout adjudication.
- Serializable history of every turn (sense result, requested move, taken
  move, capture square, FEN before and after).
- Standard RBC starting position and configurable reversible-move / full-turn
  draw limits.

Out of scope (by design):

- Bots, networking, matchmaking, accounts, clocks, or server code. Those
  belong to downstream crates that build on this one.
- Standard-chess legality enforcement. RBC explicitly permits positions and
  moves that standard chess forbids (e.g. moving into check, capturing the
  king); the crate preserves those.

## Install

Add to `Cargo.toml`:

```toml
[dependencies]
rbc-rs = "0.1"
```

Minimum supported Rust version: **stable (current)**. An explicit MSRV will
be declared in a follow-up release.

## Quick start

```rust
use rbc_rs::{Color, Game, GameConfig, Move, Square};

let mut game = Game::new(GameConfig::default());

// Sense a 3×3 window centred on e4 from the side to move.
let center = Square::from_coords(4, 3).unwrap();
let sense_result = game.sense(Some(center));
for sensed in &sense_result.squares {
    println!("{}: {:?}", sensed.square, sensed.piece);
}

// Generate the candidate move requests for the acting player.
let actions = game.move_actions();
assert!(!actions.is_empty());

// Apply a move. The crate revises blocked slider moves to the first
// blocking square and reports any capture.
let e2_e4 = Move {
    from: Square::from_coords(4, 1).unwrap(),
    to:   Square::from_coords(4, 3).unwrap(),
    promotion: None,
};
let outcome = game.apply_move(Some(e2_e4)).unwrap();
println!("{:?} -> {:?}", outcome.status, outcome.taken);

assert_eq!(game.turn(), Some(Color::Black));
```

The history of every turn — sense window, requested vs. taken move, capture,
FEN before and after — is accessible via `game.history()` and round-trips
through serde.

## Behavioural parity with upstream

`rbc-rs` is checked for equivalence with the upstream Python
[`reconchess`][reconchess-py] package on every PR. `tests/conformance.rs`
ports the upstream regression suite; `tests/opera_game.rs` replays a full
historical chess game (Morphy vs. Duke of Brunswick, Paris 1858) and asserts
every move resolves with the expected status. Reports of parity divergence
are treated as bugs.

The upstream Python package builds on the excellent
[`python-chess`][python-chess] library for its chess primitives. The Rust
crate has no equivalent dependency — all chess mechanics are crate-owned
and implemented in-tree.

## Speed

The crate uses a bitboard-backed `Position`, precomputed knight/king/pawn
attack tables, and a ray-table-with-blocker-subtract pattern for sliders.
The `cargo bench --bench game` harness exercises move generation, move
application, sensing, FEN round-tripping, and a full-game replay. Indicative
numbers on a recent x86_64 machine (criterion midpoints):

| Bench                       | Time        |
| --------------------------- | ----------- |
| `move_actions_start`        | ~440 ns     |
| `move_actions_midgame`      | ~600 ns     |
| `apply_move_sequence` (8×)  | ~6 µs       |
| `apply_opera_game` (33×)    | ~25 µs      |
| `sense/corner`              | ~25 ns      |
| `sense/center`              | ~25 ns      |
| `position_from_fen`         | ~330 ns     |
| `position_to_fen`           | ~340 ns     |

For comparison, the upstream Python `reconchess` is dominated by Python
interpreter overhead. The Rust crate is orders of magnitude faster on
per-operation cost, which makes it usable for simulation and search
workloads where the Python original is not.

## Roadmap

- `rbc960` — RBC played from Chess960 (Fischer Random) starting positions.
- `rbc-setup` — RBC played from user-configured starting positions, similar
  to setup chess on chess.com.

These variants will land as additive features without disturbing the
canonical RBC behaviour. Track progress on the [issues page][issues].

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo bench --bench game
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the contribution workflow,
parity policy, and PR conventions. Release history lives in
[`CHANGELOG.md`](CHANGELOG.md).

## License

Licensed under the [MIT License](LICENSE).

## Credits

- **Reconnaissance Blind Chess** — designed and hosted by the
  [Johns Hopkins University Applied Physics Laboratory][jhuapl];
  see [rbc.jhuapl.edu][rbc] for the rules, tournament, and academic work.
- **Upstream Python implementation** — the canonical reference for RBC
  rules, [`reconchess`][reconchess-py]. `rbc-rs` aims for behavioural
  parity with it.
- **`python-chess`** — the chess primitive library that the upstream
  Python `reconchess` package builds on. See [niklasf/python-chess][python-chess].

[jhuapl]: https://www.jhuapl.edu/
[rbc]: https://rbc.jhuapl.edu/
[reconchess-py]: https://github.com/reconnaissanceblindchess/reconchess
[python-chess]: https://github.com/niklasf/python-chess
[issues]: https://github.com/ywzvennu/rbc-rs/issues
