# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — per-token sense reveal mode (#84)

- New [`SenseRevealMode`] enum (`Immediate` default,
  `Deferred`). `SenseToken` gains a `reveal_mode` field with a
  builder-style `SenseToken::with_reveal_mode(m)` setter. Default
  is `Immediate` so existing behaviour is preserved exactly.
- **Breaking**: `Game::sense_with` now returns
  `Result<Option<SenseResult>, Error>`. For
  [`Immediate`](SenseRevealMode::Immediate) tokens it returns
  `Ok(Some(result))` (same payload as before, just wrapped in
  `Some`); for [`Deferred`](SenseRevealMode::Deferred) tokens it
  buffers the result inside the engine and returns `Ok(None)`.
  Callsite migration is one extra `.expect("immediate")` or one
  `if let Some(result) = ...` branch.
- New `Game::reveal_senses() -> Vec<SenseResult>` — drains the
  deferred buffer for the current player and returns the
  just-revealed results. Idempotent: returns an empty `Vec` once
  the buffer is empty.
- `Game::apply_move` auto-reveals any deferred senses the player
  forgot to commit. They still land in `HistoryEntry.senses`;
  they just weren't visible in time to inform this turn's move.
- Closes #84.

### Added — per-token sense visibility (#83)

- New [`SenseVisibility`] enum with six levels of opponent
  disclosure: `Private` (default — vanilla RBC, opponent learns
  nothing), `Existence` (opponent knows a sense happened),
  `Shape` (shape only, no center), `Center` (center only, no
  shape), `Board` (center + shape + sensed squares but no piece
  data) and `Full` (everything, including piece data).
- `SenseToken` gains a `visibility: SenseVisibility` field with
  a builder-style `SenseToken::with_visibility(v) -> Self` setter.
  Default is `Private` so existing behaviour is preserved.
- `SenseResult` gains `visibility: SenseVisibility` and `shape:
  SenseShape` fields, snapshotted from the token at sense time —
  so revoking or mutating the token later does not retroactively
  change historical projection.
- New [`SenseObservation`] enum and
  `SenseResult::observation() -> Option<SenseObservation>`
  accessor. Returns `None` for `Private` senses (filtered out of
  the opponent's view); otherwise returns the appropriate
  variant. The server walks `Game::history()` and calls
  `observation()` per sense to compose per-viewer history.
- New `SensePolicy::from_tokens(Vec<SenseToken>)` constructor for
  building multi-token / non-default-visibility policies (since
  `SensePolicy` is `#[non_exhaustive]`).
- Closes #83.

### Added — mid-game sense token grants and revocations

- `Game::grant_sense_token(color, token) -> SenseTokenId` — adds a
  token to a side's runtime sense policy and returns its new opaque
  ID. Useful for server-side economies where players acquire
  additional sense capabilities mid-game.
- `Game::revoke_sense_token(color, id) -> bool` — removes a token.
  Returns `true` if the ID existed (whether used this turn or
  not), `false` otherwise. Revocation is permanent; subsequent
  `sense_with` calls with the revoked ID return
  `Err(Error::InvalidSense)`.
- IDs are monotonic per side: revoking does not free the ID for
  reuse.
- Closes #87.

### Changed — sense API refactor

- **`Game::sense(center)` → `Game::sense_with(action)`**. The center is
  no longer passed directly; the player picks an action from
  `Game::sense_actions()`, which now returns `Vec<SenseAction>`
  (a `{ token, center }` pair) instead of `Vec<Square>`.
- New opaque types: `SenseTokenId` (the engine's handle for a
  token), `SenseAction` (token + center), `SenseToken` (`{ shape }`,
  `#[non_exhaustive]`), `SensePolicy` (`{ tokens: Vec<SenseToken> }`,
  `#[non_exhaustive]`).
- `GameConfig.{white,black}_sense_shape` → `{white,black}_sense_policy`
  (a `SensePolicy` containing exactly one token in the default).
- `SenseResult.center: Option<Square>` → `SenseResult.action: SenseAction`.
  Passing on sense is no longer represented as a `None` center —
  the player simply doesn't call `sense_with` and the recorded
  history entry's `senses` vector is empty.
- `HistoryEntry.sense: SenseResult` → `HistoryEntry.senses: Vec<SenseResult>`
  (today: 0 or 1 element; future multi-token policies make N > 1
  possible).
- New `Error::InvalidSense` variant for unknown / used / depleted
  token IDs.
- New `Game::token_shape(color, token_id)` accessor for UI.

Default behaviour is **identical** to before the refactor — one 3×3
sense per turn per side, optional. The new types make multi-token
budgets (#86) and mid-game grants (#87) purely additive going
forward: `SenseToken` and `SensePolicy` are `#[non_exhaustive]` so
new fields land without API breakage; `Vec<SenseToken>` already
holds more than one entry; `SenseTokenId` is opaque.

### Added (kept from earlier round)

- New `SenseShape` type and `white_sense_shape` / `black_sense_shape`
  fields on `GameConfig`. Default is `SenseShape::window(1)` — the
  standard 3×3 RBC window — so existing behaviour is preserved.
  Variants can use:
  - `SenseShape::window(half_width)` — square window
  - `SenseShape::rectangle(half_w, half_h)` — rectangle
  - `SenseShape::cross(arm)` — plus shape
  - `SenseShape::point()` — single square
  - `SenseShape::full_board()` — all 64 squares
  - `SenseShape::empty()` — zero squares
  - `SenseShape::custom(offsets)` — arbitrary `(dx, dy)` offsets
- Per-side configuration: `white_sense_shape` and `black_sense_shape`
  can differ for asymmetric / handicap variants.
- Closes #85; tracker #82 (multi-token budgets and mid-game grants
  remain open as #86 / #87).

### Changed (pre-v0.1.0 simplification)

- **Removed `Variant` enum from the public API.** The family label
  (Chess960 / Chess-2880 / shuffle, mirrored vs squared) is
  matchmaking metadata for downstream consumers (a game server, a
  matchmaking layer), not engine state. `rbc-rs` no longer carries
  it.
- **`GameConfig` now has `white_backrank: [PieceKind; 8]` and
  `black_backrank: [PieceKind; 8]` fields**, defaulting to
  [`STANDARD_BACK_RANK`] (the FIDE arrangement). `Game::new` reads
  these directly to assemble the starting position. Setting both to
  the same array gives a mirrored shuffle; setting them to different
  arrays gives a squared (RBC²) game.
- **Removed the 18 family-specific constructors** on `Game`
  (`new_rbc_960`, `new_rbc_960_random`, `new_rbc_960_from_backrank`,
  `_squared` variants, and the same for `rbc_2880` and
  `rbc_shuffle`). Consumers that want a shuffle variant sample a
  back rank via `chess-startpos-rs`, convert to `PieceKind`, and
  set the config fields directly. See `examples/rbc_960.rs`.
- **`chess-startpos-rs` moved from `[dependencies]` to
  `[dev-dependencies]`.** `rbc-rs`'s public API no longer references
  the upstream type — `PieceKind` (rbc-rs's own) is used throughout.
  The `serde` feature no longer forwards to
  `chess-startpos-rs/serde`.

### Added

- New exported const `STANDARD_BACK_RANK: [PieceKind; 8]` — the FIDE
  arrangement (`RNBQKBNR`).

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
