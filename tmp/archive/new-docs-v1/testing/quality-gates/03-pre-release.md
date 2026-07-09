# Pre-Release Gate

> What must pass before a release tag is created. Includes E2E tests and performance benchmarks.

**Status**: Shipping
**Depends on**: all test tiers including E2E and performance
**Last reviewed**: 2026-04-19

---

## TL;DR

A release requires all PR checks PLUS the E2E self-hosting loop test and the full performance benchmark suite. Performance regressions must be justified or fixed before tagging.

---

## Required Checks (Blocking)

All PR checks plus:

| Check | What it runs | Failure threshold |
|---|---|---|
| `test-e2e` | `cargo test -p roko-e2e` (full self-hosting loop) | Any failure |
| `bench-regression` | `cargo bench -- --save-baseline release_<tag>` + diff vs last release | > 20% regression on any hot-path |
| `coverage-final` | `cargo llvm-cov --fail-under-lines 80` | Any crate < 80% |
| `full-proptest` | `PROPTEST_CASES=2048 cargo test proptest` | Any failure |

---

## E2E Tests in Pre-Release

The E2E suite (see [../tiers/05-end-to-end-tests.md](../tiers/05-end-to-end-tests.md)) verifies:
1. Full `roko prd → plan run → gate → persist → resume` loop.
2. Crash-and-resume correctness.
3. Content-addressing integrity of all persisted Engrams.
4. Learning updates are applied correctly.

All E2E tests must pass with zero failures (no retries counted as passing).

---

## Benchmark Baseline

The pre-release CI saves a benchmark baseline named `release_<semver>`:

```bash
cargo bench -- --save-baseline release_0.4.2
```

On the next release, the new benchmarks are compared against the previous baseline:
```bash
cargo bench -- --baseline release_0.4.1
```

Regressions > 20% on any hot-path (content hashing, gate pipeline, HDC search) block the release.

---

## Release Checklist

Before tagging a release:
- [ ] All PR checks pass on `main`.
- [ ] `test-e2e` passes.
- [ ] `bench-regression` shows no regressions > 20%.
- [ ] `CHANGELOG.md` updated with all changes since last release.
- [ ] `status/status.md` updated with current implementation tiers.
- [ ] No open P0 or P1 bugs.

---

## See also

- [04-post-deploy.md](04-post-deploy.md) — post-release monitoring
- [../tiers/05-end-to-end-tests.md](../tiers/05-end-to-end-tests.md) — E2E test details
- [../tiers/07-performance-tests.md](../tiers/07-performance-tests.md) — benchmark setup
