# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- `rbc960` game mode — Reconnaissance Blind Chess played from Chess960
  (Fischer Random) starting positions.
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
