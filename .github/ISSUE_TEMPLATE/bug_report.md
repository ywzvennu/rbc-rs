---
name: Bug report
about: Report a defect in game logic, FEN parsing, history serialization, or build
title: ''
labels: bug
assignees: ''
---

## Summary

A clear, single-paragraph description of what's wrong.

## Reproduction

The smallest reproduction you can produce. Where possible please include:

- Starting FEN (or `Game::new`)
- Sequence of `sense` / `apply_move` calls
- Expected outcome
- Actual outcome

```rust
// minimal example
```

## Environment

- `rbc-rs` version (Cargo.toml or commit SHA):
- Rust toolchain (`rustc --version`):
- OS / arch:

## Upstream parity

If the bug relates to behaviour that diverges from the upstream
Python [`reconchess`](https://github.com/reconnaissanceblindchess/reconchess)
package, please reference the corresponding upstream behaviour (file +
line, or a brief paste).

## Additional context

Anything else that would help reproduction or triage.
