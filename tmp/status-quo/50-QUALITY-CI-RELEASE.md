# Quality, CI, Release

> Status-quo audit · re-verified 2026-07-08 against git HEAD `5852c93c05` on `main`.
> Companion files: [71-CI-RELEASE-PROOF-GAPS.md](71-CI-RELEASE-PROOF-GAPS.md) (workflow-by-workflow gap table),
> [10-TESTING-STATUS.md](10-TESTING-STATUS.md) (per-crate census), [74-TEST-AND-PROOF-INVENTORY.md](74-TEST-AND-PROOF-INVENTORY.md)
> (volume-vs-proof ledger), [64-PARITY-TEST-MATRIX.md](64-PARITY-TEST-MATRIX.md) (parity tests to add).

## TL;DR

- **7 workflows** exist (`ci`, `coverage`, `msrv`, `release`, `docker-publish`, `deploy-fly`, `tui-parity-dry-run`).
  Only `ci.yml` is a real gate: `clippy -D warnings` + `cargo test --workspace` + nightly `fmt --check` + `layer-check`.
- **~9,968** test attributes across `crates`/`apps`/`tests` (8,285 `#[test]` + 1,777 `#[tokio::test]`). Volume ≠ proof.
- **P0 semantic gap (confirmed at source):** the *default* `roko plan run` uses `PlanEngine::Graph`
  (`crates/roko-cli/src/main.rs:1299,2699`), and its task cell is a **stub**
  (`crates/roko-graph/src/cells/task_executor.rs:1-95`): `TaskExecutorCell::default()` sets `dry_run: true`,
  and even the `dry_run: false` branch is "live dispatch not yet implemented; using dry-run fallback" and emits
  a synthetic `task-output:stub:{task}` engram. **The default plan-run path never dispatches an agent.**
  Real dispatch lives only behind `roko plan run --engine runner-v2` (`crates/roko-cli/src/runner/`).
- Tests that mock dispatch (MockToolDispatcher, MockAgent, wiremock) can pass while the default runtime path is inert.
- No CI gate exercises: frontend, Foundry contracts, `cargo deny`, a runtime dispatch smoke, Docker `/health`,
  the feature matrix, or the default-plan-run stub-detection.

## Current Reality (verified this pass)

| Fact | Value | Evidence |
|---|---|---|
| Workflows | 7 | `.github/workflows/*.yml` |
| Enforcing gates | clippy, workspace test, nightly fmt, layer-check, MSRV `check` | `ci.yml`, `msrv.yml` |
| Test attributes (crates+apps+tests) | 9,968 | `rg '#\[(tokio::)?test\]'` |
| Test attributes (crates only) | 9,521 | same, `crates/` scope |
| Real `#[ignore = ...]` | 11 | `rg '^\s*#\[ignore' crates/` |
| `deny.toml` | present, **unwired** | `deny.toml` exists; no workflow runs `cargo deny` |
| MSRV drift | **workspace `1.85` vs CI `1.91`** | `Cargo.toml:93` (`rust-version = "1.85"`) vs `msrv.yml` (`toolchain: "1.91"`) |
| roko-serve routes | 288 `.route(` | `rg '\.route\(' crates/roko-serve/src/` (CLAUDE.md still says ~85) |
| orchestrate.rs | 23,676 LOC / 85 inline tests (~3.6/KLOC) | `crates/roko-cli/src/orchestrate.rs`; **feature-gated off by default** (`legacy-orchestrate`) |
| Default `cargo test` scope | 3 default-members (`roko-cli`, `roko-mcp-code`, `roko-mcp-github`) | `Cargo.toml:84`; bare `cargo test` is narrower than CI's `--workspace` |

Verified during this audit: workflow files, `Cargo.toml` MSRV/members, route count, orchestrate LOC/tests,
Graph task-executor stub, `PlanEngine` default. **Not executed:** `cargo test --workspace`, frontend build,
Foundry `forge test`, `cargo deny check`, Docker/Fly deploy smoke, live-agent default plan run.

## Quality Risks

- **Honesty risk (P0):** the flagship "self-hosts" claim rests on the default plan-run path, which is a synthetic
  no-op. A green `roko plan run` proves nothing about agent execution. Only `--engine runner-v2` is real, and its
  entrypoint is undertested (see `74`/`10`).
- **False-passing tests:** mocked-dispatch tests (roko-std `mock_dispatcher.rs`, roko-agent `mock.rs`, wiremock)
  and route tests that don't hit real handlers can stay green while the real path is broken (the `/search`
  precedent from wave-2). Coverage numbers inflate correspondingly.
- **Coverage is informational, not a gate:** `coverage.yml` runs `--ignore-run-fail` with no threshold, so failing
  tests still produce green artifacts.
- **MSRV inconsistency:** `Cargo.toml` says `1.85`, CI pins `1.91`; CLAUDE.md tells contributors to `rustup update`.
  Three sources of truth.
- **Release ships untested:** `release.yml` builds `roko-cli`+`roko-mcp-code` binaries on tag with **no** pre-build
  `cargo test`/clippy/smoke. `docker-publish.yml` pushes images without booting them.
- **Doc-status drift:** CLAUDE.md and older status docs describe subsystems as "wired" that are stub/partial;
  doc-driven work re-imports those assumptions.
- **`.route(` count 3× CLAUDE.md** — either nested routers or a stale doc; either way route coverage is unproven
  (8 integration files vs 288 routes).

## CI/Release Checklist

- [ ] **[P0]** Default-plan-run smoke test that asserts on real agent output and **fails on `task-output:dry-run:`/`:stub:`** synthetic markers — verify: `cargo test -p roko-cli default_plan_run_not_stub`.
- [ ] **[P0]** Run `cargo test --workspace` (or at minimum `-p roko-cli -p roko-serve`) in `release.yml` before building binaries — verify: `grep 'cargo test' .github/workflows/release.yml`.
- [ ] **[P0]** Reconcile MSRV to one value across `Cargo.toml`, `msrv.yml`, Docker, CLAUDE.md — verify: `rg 'rust-version|1\.9?1|1\.85' Cargo.toml .github/workflows/msrv.yml`.
- [ ] **[P1]** `roko resume` smoke test (snapshot-capable engine, skips completed tasks).
- [ ] **[P1]** Route-contract test from `build_router` to a generated manifest; frontend endpoint contracts for ISFR, dreams, bench, terminal, relay.
- [ ] **[P1]** roko-graph engine tests (cell exec, error propagation, budget, cycle) + Graph example-load test for every `examples/graphs/*.toml`.
- [ ] **[P1]** Wire `cargo deny check` (`deny.toml` already exists).
- [ ] **[P1]** ACP permission-flow integration test for write/bash/fetch tools; safety adversarial suite.
- [ ] **[P2]** Drop or threshold `coverage.yml`'s `--ignore-run-fail`.
- [ ] **[P2]** Foundry `forge test` (`contracts/`) + frontend `npm ci`/build/Playwright, or explicitly mark non-release.
- [ ] **[P2]** Deterministic-provider runtime smoke + Docker boot `/health`/`/ready` checks in a release gate.
- [ ] **[P2]** Feature-matrix job (serve `hdc/otlp`, index `sqlite/rkyv`, lang `tree-sitter`, mirage, cli `legacy-orchestrate`).
- [ ] **[P3]** Require proof-gate updates when docs claim a subsystem is "wired".

## Release Gate

A release candidate must **not** advertise "self-hosting" or "v2 Graph plan execution as live" until:

- the default plan run either does real dispatch or hard-errors "unsupported" (no synthetic success),
- the Graph `TaskExecutorCell` live branch is implemented **or** `--engine runner-v2` is made the default,
- `roko resume` works against a snapshot-capable engine,
- `release.yml` depends on a passing test/clippy gate,
- MSRV is unified,
- route/API/frontend contracts pass,
- the proof gaps in [71](71-CI-RELEASE-PROOF-GAPS.md) are fixed or explicitly accepted for a non-production build.
