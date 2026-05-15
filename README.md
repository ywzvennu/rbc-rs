# reconchess-rs

Reconnaissance Blind Chess game logic for Rust.

This crate provides the game-playing core only: board state, sensing, move
resolution, game results, and serializable history. It does not include bots,
networking, matchmaking, accounts, clocks, or server code.

The public API is Rust-native and crate-owned. The crate has no external
dependency — board state, FEN parsing, move generation, and move resolution
are all implemented in-tree.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
