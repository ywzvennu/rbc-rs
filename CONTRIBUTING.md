# Contributing to rbc-rs

Thanks for your interest in improving the crate. This document covers the
day-to-day workflow and the bar for accepted changes.

## Scope

`rbc-rs` is the Rust game-logic core for Reconnaissance Blind Chess:
position state, sensing, move generation, move resolution, history, and
result adjudication. It deliberately does not include bots, networking,
matchmaking, accounts, clocks, or server code. Contributions that fit
inside that scope are welcome; please open an issue first if you're
unsure whether a change belongs here.

## Development environment

A stable Rust toolchain is sufficient. Required components:

```sh
rustup component add rustfmt clippy
```

All checks the CI runs locally:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo bench --bench game     # optional, slower
```

Tests must remain green and clippy must stay clean before submitting a
pull request.

## Reporting bugs and proposing changes

Open an issue first, especially for behavioral changes. Use the bug
report or feature request templates in `.github/ISSUE_TEMPLATE/`. Link
the issue from your pull request so the merge auto-closes it.

For changes to RBC move semantics, please reference upstream
[`reconchess`](https://github.com/reconnaissanceblindchess/reconchess)
behavior — the crate aims for behavioral parity with the canonical
Python package.

## Pull request guidelines

- Branch off `main`. Use `feature/<slug>`, `fix/<slug>`, `refactor/<slug>`,
  `bench/<slug>`, `test/<slug>`, or `chore/<slug>` naming.
- Keep PRs focused. Smaller, single-purpose PRs are easier to review and
  bisect later.
- Match the existing commit style: imperative subject under ~72
  characters, optional body explaining the why.
- Every PR must pass CI (`cargo fmt --check`, `cargo clippy ... -D
  warnings`, `cargo test --all-features`).
- Public API changes need rustdoc updates. Performance-sensitive changes
  should include before/after numbers from the criterion bench harness
  (`cargo bench --bench game`).

## Behavioral parity with upstream

When in doubt about move resolution edge cases, the upstream Python
`reconchess` is the reference. The `tests/conformance.rs` and
`tests/opera_game.rs` files exercise parity-critical paths; please
extend them when adding logic that could drift from upstream.

## License

By contributing, you agree that your contribution will be licensed
under the same [MIT License](LICENSE) that covers this project.
