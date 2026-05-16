# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Game::new` now dispatches on `config.variant` to assemble the
  starting position. Existing default-config calls produce the
  classical FIDE start, unchanged.
- 18 convenience constructors on `Game`, six per shuffle family
  (`rbc_960`, `rbc_2880`, `rbc_shuffle`):
  - `Game::new_rbc_960(sp_id, cfg)` — canonical Chess960 SP-ID
    (0..=959); SP-ID 518 is the FIDE standard position.
  - `Game::new_rbc_960_random(seed, cfg)` — uniform random,
    deterministic in seed.
  - `Game::new_rbc_960_from_backrank(arr, cfg)` — explicit back rank
    (validated against the Chess960 constraints).
  - `_squared`, `_squared_random`, `_squared_from_backranks` —
    independent draws per side.
  - Same 6-method family for `rbc_2880` and `rbc_shuffle`, using
    lexicographic indices (0..2880, 0..5040) since neither has a
    canonical SP-ID upstream.
- New `examples/rbc_960.rs` demonstrating mirrored, random, and
  squared Chess960 game construction.
- New `tests/variants.rs` integration test suite covering each
  constructor family.

### Changed

- `CastlingRights` (internal) now stores each direction's right as
  `Option<u8>` — the rook's starting file — instead of a plain
  `bool`. Standard-chess castling still works exactly as before
  (rook files always 0 / 7); Chess960 / X-FEN positions with
  non-standard rook files are now representable end-to-end.
- FEN castling-field parser accepts both the standard `KQkq` form
  (rook file inferred relative to the king) and the Shredder-FEN
  form `AHah` (explicit rook file letters). FEN emitter uses the
  standard form when every rook is on file 0 or 7, otherwise emits
  the Shredder form.
- Castling move generation and validation use Chess960 path-clear
  semantics: every square between the king's start and target, and
  between the rook's start and target, must be empty (except for
  the king and rook themselves). Standard chess is the special case
  where rook files are 0 / 7.

### Added

- New dependency on [`chess-startpos-rs`](https://crates.io/crates/chess-startpos-rs)
  (`= "0.1"`). Provides the constraint engine that drives the shuffle
  variants.
- New `Variant` enum on `GameConfig` storing the **already-sampled**
  back-rank arrangement for each named family:
  - `Standard` (default — classical FIDE chess).
  - `Rbc960 { backrank }` / `Rbc960Squared { white, black }` — Chess960
    (bishops opposite, king between rooks).
  - `Rbc2880 { backrank }` / `Rbc2880Squared { white, black }` —
    Chess2880 (bishops opposite, no king-between-rooks).
  - `RbcShuffle { backrank }` / `RbcShuffleSquared { white, black }` —
    unconstrained KQRRBBNN shuffle.
  - `#[non_exhaustive]`. `Copy + Eq + Hash` (the back-rank arrays are
    fully baked, no upstream constraint problem).
- New `CastlingPolicy` struct on `GameConfig`: per-side, per-direction
  toggles applied as an intersection with the structural rights
  derived from the chosen back rank. Defaults to all four directions
  allowed.
- `GameConfig` is now `#[non_exhaustive]` — construct via `Default`
  and mutate, no struct-literal from external crates.
- Re-exports: `Variant`, `CastlingPolicy`.

### Planned

- Game::new wiring: assemble the starting FEN from `config.variant`.
- X-FEN castling support so non-standard rook files survive FEN
  round-trips.
- Convenience constructors `Game::new_rbc_960` / `new_rbc_2880` /
  `new_rbc_shuffle` and their mirrored / squared / `_random` /
  `_from_backrank(s)` shapes (18 total).
- `rbc-setup` game mode — RBC played from user-configured starting
  positions, similar to setup chess on chess.com.

## [0.1.0] — Initial development

### Added

- Crate-owned public game model: `Game`, `Move`, `Square`, `Piece`,
  `PieceKind`, `Color`, `SenseResult`, `MoveOutcome`, `HistoryEntry`,
  game-status / result / win-reason / draw-reason types, and an
  `Error` enum.
- Standard board setup, FEN import/export (including RBC-specific
  positions such as post-king-capture states and positions with the
  side-to-move's king in check), and a 3×3 sense window clipped at
  the board edges.
- Move action generation from the acting player's own-piece view,
  including pawn capture requests against unseen opponent pieces and
  omitted-promotion variants on the back rank.
- Move resolution with slider revision, castling that ignores check,
  en-passant captures (with the captured-pawn square reported),
  promotion (with omitted promotions normalised to queen), and king
  capture as a game-ending event.
- Game lifecycle: pass moves, illegal-move turn consumption, capture
  notifications via `opponent_capture_square`, resignation,
  externally declared timeouts, reversible-move draw limit, and
  full-turn draw limit.
- Serde-backed serialization for `Game` and history through a
  FEN-anchored wire representation.
- Internal bitboard board representation with per-color and per-kind
  bitboards, precomputed attack tables, and ray-table-with-blocker
  helpers for slider generation, revision, and validation.
- Direct move validator on the `apply_move` hot path that avoids
  rebuilding `move_actions()` per call.
- Criterion benchmark harness covering move generation, application,
  sensing, FEN round-tripping, slider-heavy positions, and a full-
  game replay of Morphy vs Duke of Brunswick (Paris 1858).
- Conformance and parity tests covering RBC-specific FEN states,
  hidden-information move actions, slider revision, castling
  variants, en passant, promotions, capture reporting, and terminal
  positions. Behavior is checked against the upstream Python
  `reconchess` package.
