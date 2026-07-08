# Pre-Commit Gate

> What must pass before a commit is allowed. Fast, developer-local, blocking.

**Status**: Shipping
**Depends on**: [../tiers/01-unit-tests.md](../tiers/01-unit-tests.md), [../tiers/03-property-tests.md](../tiers/03-property-tests.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The pre-commit gate runs in < 60 seconds on a developer laptop. It catches compilation errors, formatting violations, and fast unit test failures before code reaches the PR stage.

---

## What Runs

```bash
# Installed via: cargo install cargo-husky (or equivalent git hook)
# Runs on: git commit

1. cargo fmt --check          # format check (fail fast)
2. cargo clippy -- -D warnings # lint (fail fast)
3. cargo test --workspace      # unit tests (fast subset; E2E excluded)
4. cargo test proptest         # fast proptest subset (PROPTEST_CASES=64)
```

Expected runtime: < 60 seconds.

---

## What Does NOT Run

- Integration tests (too slow for pre-commit)
- Regression golden tests (only run in CI)
- E2E tests (minutes; CI only)
- Coverage (CI only)
- Benchmarks (CI only)

---

## Configuration

```bash
# .cargo-husky/hooks/pre-commit
#!/bin/sh
set -e
cargo fmt --check
cargo clippy -- -D warnings
PROPTEST_CASES=64 cargo test --workspace --exclude roko-e2e
```

The hook is installed automatically via `cargo-husky` on `cargo build`.

---

## Bypassing (Emergency Only)

```bash
git commit --no-verify -m "WIP: emergency fix"
```

Bypassing is recorded in the git log. A PR with `--no-verify` commits requires an explanation in the PR description.

---

## See also

- [02-pr-checks.md](02-pr-checks.md) — the next gate after commit
