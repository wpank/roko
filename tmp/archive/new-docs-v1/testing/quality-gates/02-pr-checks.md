# PR Checks

> What must pass for a PR to be mergeable. CI-enforced, blocking.

**Status**: Shipping
**Depends on**: all test tiers except E2E
**Last reviewed**: 2026-04-19

---

## TL;DR

A PR requires all unit tests, property tests (full), integration tests, and regression tests to pass. Coverage must not drop below 80% for any shipping crate. Performance regressions > 20% generate warnings but are not blocking.

---

## Required Checks (Blocking)

All of the following must be green for merge:

| Check | What it runs | Failure threshold |
|---|---|---|
| `test-unit` | `cargo nextest run` (unit tests only) | Any failure |
| `test-property` | `cargo nextest run proptest` (PROPTEST_CASES=512) | Any failure |
| `test-integration` | `cargo nextest run --test '*_integration*'` | Any failure |
| `test-regression` | `cargo nextest run --test '*_golden*'` | Any diff |
| `format` | `cargo fmt --check` | Any failure |
| `lint` | `cargo clippy -- -D warnings` | Any warning |
| `coverage` | `cargo llvm-cov --fail-under-lines 80` | Any crate < 80% |
| `security-audit` | `cargo audit` | Any critical CVE |

---

## Optional Checks (Non-Blocking Warnings)

| Check | Condition for warning |
|---|---|
| `performance` | Benchmark regression > 5% on any hot-path |
| `coverage-diff` | Lines added in this PR that are not covered |
| `doc-check` | `cargo doc --no-deps --document-private-items` (broken links) |

Performance regressions > 20% require explicit maintainer acknowledgment before merge.

---

## PR Size Policy

PRs that change:
- > 500 lines in a single crate: require a test coverage review comment.
- > 1,000 lines total: split encouraged (suggested in PR review, not required).
- New public APIs: require tests demonstrating the new API before merge.

---

## Review Requirements

- At least 1 approving review from a maintainer.
- No unresolved conversations.
- All CI checks green (or explicitly waived with a comment).

---

## What PRs Do NOT Need

- E2E test passage (E2E only runs on `main`).
- Full benchmark suite passage (only pre-release).

---

## See also

- [03-pre-release.md](03-pre-release.md) — additional checks for releases
- [../tools-and-harness/04-ci-integration.md](../tools-and-harness/04-ci-integration.md) — CI configuration
