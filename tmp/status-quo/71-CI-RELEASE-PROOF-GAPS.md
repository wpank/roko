# CI And Release Proof Gaps

> Re-verified 2026-07-08 against git HEAD `5852c93c05`. Expands [50-QUALITY-CI-RELEASE.md](50-QUALITY-CI-RELEASE.md)
> and [64-PARITY-TEST-MATRIX.md](64-PARITY-TEST-MATRIX.md) with the workflow/scripts audit.

## Current CI Coverage (7 workflows)

| Workflow | Trigger | What it actually runs (verified) | Gate? | Gap |
|---|---|---|---|---|
| `ci.yml` | push main, PR | job **test**: `cargo clippy --workspace --no-deps -- -D warnings` then `cargo test --workspace` (stable, `RUSTFLAGS=-D warnings`, 60 min). job **fmt**: `cargo fmt --all --check` on **nightly**. job **layer-check**: `cargo build -p roko-cli` + `cargo run -p roko-cli -- layer-check`. | **Yes** | The only real Rust gate. No frontend, Foundry, `deny`, route contract, runtime dispatch smoke, Docker health, feature matrix, or default-plan-run stub detection. |
| `coverage.yml` | push main, PR | `cargo llvm-cov --html/--json` with `--ignore-run-fail` + `--ignore-filename-regex`; uploads artifacts. | **No** | `--ignore-run-fail` means failing tests still go green; **no threshold**. Informational only. |
| `msrv.yml` | push main, PR | `cargo check --workspace` on pinned **1.91**. | Yes (check only) | Value drifts: `Cargo.toml:93` says `rust-version = "1.85"`; comment claims it "must match workspace". No test/clippy on MSRV. |
| `release.yml` | tag `v[0-9]+.*`, dispatch | 4-target matrix **build only** (`roko-cli`, `roko-mcp-code`) → GitHub Release with changelog. | **No** | Ships binaries with **no** pre-release `cargo test`/clippy/smoke/Docker-health. |
| `docker-publish.yml` | push main, tags `v*` | Builds/pushes 3 images (`roko`, `roko-worker`, `mirage`) to ghcr. | **No** | Never boots a container to curl `/health`/`/ready`. |
| `deploy-fly.yml` | push main, dispatch | Generates `fly.toml`, `flyctl deploy --remote-only`. | Deploy | Deploys straight off main; the `[[http_service.checks]]` `/api/health` is Fly-side post-deploy, not a CI gate. |
| `tui-parity-dry-run.yml` | PR touching `tmp/tui-parity/**`, `tmp/ux-followup-runner/**` | `--dry-run` of two tmp shell scripts. | Narrow | tmp-specific; the referenced runner scripts should be confirmed tracked (doc 74). |

## P0 Proof Gap — default plan run is a stub (verified at source)

The default `roko plan run` selects `PlanEngine::Graph` (`crates/roko-cli/src/main.rs:1299,2699`). Its task cell is a
placeholder: `crates/roko-graph/src/cells/task_executor.rs:1-95`.

- `TaskExecutorCell::default()` → `dry_run: true` → emits `task-output:dry-run:{task}` and logs
  "dry-run: skipping LLM dispatch" (lines 30-34, 70-79).
- The `dry_run: false` branch: "Live mode: not yet implemented. Fall back to dry-run behavior with a warning."
  → emits `task-output:stub:{task}` (lines 80-92).

So **no Graph plan run ever dispatches an agent**. Real dispatch exists only behind `--engine runner-v2`
(`crates/roko-cli/src/runner/`, entry `RunnerV2` at `main.rs:1302`). CI has **no test** that fails when the default
path produces a synthetic engram. This is the single highest-leverage missing gate.

## Missing Gates

- `cargo deny check` — `deny.toml` **exists** but no workflow invokes it.
- `forge test` for `contracts/test` (10 files / 52 cases per doc 74) — present, unwired.
- `npm ci` + `npm run build` + Playwright E2E for `demo/demo-app` (15 spec files / 112 `test(...)`) — present, unwired.
- Runtime smoke: `tests/security-smoke.sh`, `tests/endpoint_smoke.py`, `tests/proof/mori-diffs/prove-runtime-end-to-end.sh` — exist, no workflow.
- Default-plan-run stub-detection test (see P0).
- Feature-matrix tests: `roko-serve` `hdc/otlp`, `roko-index` `sqlite/rkyv`, `roko-lang-rust` `tree-sitter`, `mirage-rs`, `roko-cli` `legacy-orchestrate`.
- Release preflight for the exact artifacts shipped; Docker container boot + `/health`/`/ready` curl.
- roko-graph engine unit tests (`engine.rs` has ~4 inline tests; `cell.rs`/`error.rs` 0).

## Runtime Proof Script Risk

`tests/proof/mori-diffs/prove-runtime-end-to-end.sh` can accept missing credentials / auth / rate-limit outcomes and
still succeed — fine for local dev, insufficient for release. Add either: a deterministic mock provider that proves the
full dispatch path, or a hard assertion that at least one configured live provider completed a real turn.

## Ignored / Slow / External Tests

- **11 real `#[ignore = ...]`** attributes (all with reason strings — good hygiene), including the flagship
  self-hosting e2e `crates/roko-cli/tests/e2e_self_host.rs` (needs a `ROKO_DISPATCHER` fixture). Mirage timing,
  ACP harnesses, Cursor CLI, file watchers, MCP stdio, exec parity are also ignored.
- External/env-gated tests depend on `ROKO_TEST_OLLAMA`, `ZAI_API_KEY`, `OPENAI_API_KEY`, `ROKO_TEST_RPC_URL`, and
  local Mirage/devnet state (`127.0.0.1:7878`, Railway devnet).
- The `.roko/plans/ignored-tests.md` ledger that `roko-compose/src/templates/common.rs:231` instructs agents to
  maintain **does not exist**.

## Recommended CI Staging (ordered)

1. **[P0]** Default-plan-run stub-detection test + make `release.yml` depend on `cargo test`.
2. **[P0]** Unify MSRV; add `cargo deny check`.
3. **[P1]** Frontend build + route-contract test; roko-graph engine tests.
4. **[P1]** Un-ignore self-hosting e2e via a mock `ROKO_DISPATCHER`; wire security/endpoint smoke.
5. **[P2]** Foundry tests; runtime smoke with deterministic provider; Docker boot health checks.
6. **[P2]** Targeted feature matrix; drop coverage `--ignore-run-fail` or add a floor.
7. **[P3]** Harden release/docker workflows to depend on all proof gates.

## Command Matrix

```bash
# What ci.yml gates today
cargo +nightly fmt --all --check
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
cargo build -p roko-cli && cargo run -p roko-cli -- layer-check

# Gates that SHOULD exist but don't
cargo deny check                                   # deny.toml exists, unwired
cargo +1.91 check --workspace                      # msrv (unify with Cargo.toml 1.85)
cargo test --workspace --all-features
cargo test -p roko-cli --features legacy-orchestrate
cargo test -p roko-serve --features hdc,otlp
cargo test -p roko-index --features sqlite,rkyv
cargo test -p roko-lang-rust --features tree-sitter
cargo test -p mirage-rs --features roko,sim-gas

# Runtime / contract / frontend proof (all currently unwired)
ROKO_BIN=target/debug/roko tests/security-smoke.sh
python3 tests/endpoint_smoke.py --roko-bin target/debug/roko
tests/proof/mori-diffs/prove-runtime-end-to-end.sh
(cd demo/demo-app && npm ci && npm run build && npm run e2e)
(cd contracts && forge test)
```
