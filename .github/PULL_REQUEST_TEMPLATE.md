<!-- If this PR closes an issue, reference it here (e.g. "Closes #123"). -->

## Summary

What this PR changes and why, in one short paragraph.

## Details

- Bullet-pointed list of the substantive changes.
- Notable design decisions, if any.

## Test plan

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-features`
- [ ] Public API changes documented with rustdoc
- [ ] Performance-sensitive changes include before/after numbers from
      `cargo bench --bench game`
- [ ] Behaviour-changing PRs include or extend parity tests in
      `tests/conformance.rs`

## Notes for reviewers

Anything reviewers should pay extra attention to: tricky invariants,
follow-up issues spawned, deferred work, etc.
