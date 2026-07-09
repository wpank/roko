# CI Integration

> How tests run in CI: pipeline, caching, parallelism, artifact management, and flakiness policy.

**Status**: Shipping
**Depends on**: [01-test-harness.md](01-test-harness.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The CI pipeline runs unit and property tests on every push, integration and regression tests on every PR, and the E2E suite only on merges to `main`. Tests run in parallel using `cargo-nextest` with a 16-thread job executor. Flaky tests are quarantined after two failures in 30 days.

---

## Pipeline Structure

```yaml
# .github/workflows/test.yml (conceptual)

on: [push, pull_request]

jobs:
  unit-and-property:
    runs-on: ubuntu-latest
    steps:
      - cargo nextest run --test-threads 16 --no-fail-fast
      - cargo test --doc

  integration:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - cargo nextest run --test-threads 8 --test '*_integration*'

  e2e:
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - cargo test -p roko-e2e --test-threads 2

  coverage:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - cargo llvm-cov nextest --lcov --output-path coverage.lcov
      - Upload coverage report as artifact
```

---

## Test Runner: cargo-nextest

Roko uses `cargo-nextest` instead of `cargo test` for CI because:
- **Better parallelism**: nextest runs each test function as a separate process, avoiding shared state.
- **Retry support**: nextest retries flaky tests up to 2 times before failing.
- **Test filtering**: nextest supports fast regex-based test selection.
- **Timing**: nextest tracks per-test timing for flakiness detection.

Install: `cargo install cargo-nextest`

CI command:
```bash
cargo nextest run --test-threads 16 --retries 2 --no-fail-fast
```

---

## Caching

The CI pipeline caches:
1. `~/.cargo/registry` — Cargo registry
2. `target/` — build artifacts (keyed by `Cargo.lock` hash)
3. `proptest-regressions/` — committed to repo; no separate caching needed
4. Tape fixtures — committed to repo; no separate caching needed

Cache invalidation: keyed by `Cargo.lock` checksum + Rust toolchain version.

---

## Test Parallelism

| Test tier | Threads in CI | Notes |
|---|---|---|
| Unit | 16 | Fully parallelisable |
| Property | 8 | Reduced due to RAM usage |
| Integration | 8 | Each test uses its own temp dir |
| E2E | 2 | Slow; limited parallelism to reduce noise |
| Benchmarks | 1 | Serial for statistical reliability |

---

## Flakiness Policy

1. A test that fails more than twice in any 30-day window is quarantined.
2. Quarantined tests run in a separate `quarantine` CI job (non-blocking).
3. A quarantined test is re-admitted only after:
   - Root cause identified and documented.
   - Fix landed.
   - 20 consecutive clean CI runs.
4. Quarantine status is tracked in `tests/QUARANTINE.md`.

---

## Artifact Management

CI uploads as artifacts:
- Test results (JUnit XML format via `cargo-nextest --junit`).
- Coverage report (LCOV format).
- Benchmark results (JSON via `criterion`).
- Proptest regression files (new additions from this run, if any).

Artifacts are retained for 90 days.

---

## Environment Variables in CI

| Variable | Value | Purpose |
|---|---|---|
| `PROPTEST_CASES` | `512` | More cases than developer builds |
| `ROKO_TEST_LOG` | `debug` | Verbose test logging |
| `ROKO_RECORD_TAPE` | unset | Tape replay only (never recording) |
| `RUST_BACKTRACE` | `1` | Full backtraces on panics |

---

## See also

- [../quality-gates/02-pr-checks.md](../quality-gates/02-pr-checks.md) — what must pass for merge
- [05-coverage-tooling.md](05-coverage-tooling.md) — coverage setup
