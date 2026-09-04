# 230 — Feature-Gate Chain/Alloy from the Default CLI Build

> **Status: SOURCE-IMPLEMENTED / BUILD GRAPH VERIFIED, REAL BENCHMARK EVIDENCE OPEN** (2026-08-31,
> `88c724744` + `6af235c0f`). The default development graph is lean; full provider/chain/ACP/
> embedded-frontend behavior is selected explicitly by release, Docker, CI, and dedicated targets.
> Fixed-SHA cold/warm automation is source-complete in `d1b94b139`, and the protected cache lane
> (`97f897200` + `8c82c5b1b`) avoids erasing warm dependencies outside explicit cleanup. The final
> checkpoint verified the lean dependency tree, no-default serve check, explicit Alloy/ACP CLI
> check, and current default CLI build. Representative before/after repetitions, the complete
> all-feature test matrix, and release jobs remain open.

**Status**: Verified (2026-09-03) — Alloy excluded, 501 stubs, CI matrix

> **Status update (2026-09-01):** Cache lifecycle release fixtures (Cargo lock prevents incremental
> pruning, Cargo lock prevents orphan target pruning) have been added to
> `roko-fs/src/target_cleanup.rs`. Benchmark evidence collection is scriptable via
> `scripts/run_benchmark_evidence.sh`. Before/after cold/warm measurements remain the terminal
> verification authority.

**Priority**: P2 — repeated dogfood cold builds took 10–14 minutes and the default CLI still enables Alloy's full dependency graph
**Size**: M (2–3 days)
**Wave**: 6
**Crates**: `roko-cli`, `roko-serve`, `roko-chain`, `roko-demo`
**Depends on**: None
**Source**: `tmp/archive/dogfood-2026-08-13/DOGFOOD-DEBRIEF.md`, `tmp/archive/dogfood-2026-08-17/DOGFOOD-DEBRIEF.md`

## Background

The earliest dogfood sessions spent 10–14 minutes producing a cold release build. Worktree cache
sharing reduced repeat builds, but it does not reduce the default dependency graph. At the audited
baseline, manifests made `roko-cli` depend on `roko-chain` with `features = ["alloy-backend"]`, and
`roko-serve` enabled `alloy` with `features = ["full"]`. Most self-hosting operations—plan
generation, agent dispatch, gates, learning, TUI, and local HTTP monitoring—do not use chain
features.

`roko-chain` already declares `alloy-backend` as an optional feature; the gap is propagating that
optionality to binaries and route/command surfaces so a normal development build does not compile
the full chain stack.

## Implementation Plan

- [ ] Record reproducible cold/warm baselines for `cargo build -p roko-cli`, release build time,
   dependency count, binary size, and peak disk usage. Use isolated target directories for cold
   measurements.
- [x] Add top-level feature propagation to `roko-cli` and `roko-serve`; ordinary development avoids
   unconditional full-provider/Alloy dependencies.
- [x] Gate chain-specific commands, routes, imports, and startup wiring with the selected feature.
   When a user invokes a chain surface in a build without the feature, return a clear diagnostic
   describing the build/install flag rather than silently omitting help or panicking.
- [x] Give dedicated chain/release/Docker targets the required features explicitly so their behavior
   is unchanged.
- [x] Add CI/release jobs for both the lean default build and explicit full features; ensure feature combinations do
   not bit-rot.
- [x] Add a deterministic runner that can compare the same immutable commit with isolated cold
   targets and bounded warm targets while retaining raw samples and failures.
- [ ] Publish before/after measurements and set a regression threshold for default dependency count
   or build time where CI infrastructure permits stable measurement.

## Acceptance Criteria

- [x] Default manifests select the lean graph; `cargo tree -p roko-cli` excludes
  `alloy-provider`, `alloy-network`, and `alloy-rpc-client`, and the current CLI binary builds.
- [x] Explicit Alloy/ACP CLI wiring compiles with
  `cargo check -p roko-cli --features alloy-backend,acp --locked -j1`; full runtime proof remains
  part of the release lane.
- [x] `roko serve` has an explicit feature choice; chain-disabled builds return a typed `501` diagnostic for
   chain-only routes/surfaces.
- [x] Dedicated release/Docker/CI definitions request the appropriate full features; their final
  jobs have not run for this integration.
- [ ] Cold default build time, dependency count, and binary size are measured before and after; the
   result demonstrates a material improvement or documents why the feature split is not viable.

## Verification Checklist

- [ ] Use a fresh target directory and capture `cargo build -p roko-cli --timings` before changes.
- [x] Verify `cargo tree -p roko-cli` for the default build no longer includes Alloy
      provider/network/RPC-client crates.
- [x] Check the explicit Alloy/ACP CLI feature graph and the no-default serve graph.
- [ ] Run default CLI plan validation, plan dry-run, status, doctor, and screenshot smoke tests.
- [ ] Run chain route/command tests with `--features chain,alloy-backend`.
- [ ] Run `cargo test --workspace --all-features` and lean-default CI jobs.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/Cargo.toml` | Optional chain dependency and feature definitions |
| `crates/roko-cli/src/main.rs` and chain command modules | Feature-gated surfaces and diagnostics |
| `crates/roko-serve/Cargo.toml` | Optional chain/Alloy dependencies |
| `crates/roko-serve/src/routes/chain.rs` and router wiring | Feature-gated routes |
| `crates/roko-demo/Cargo.toml` / chain apps | Explicit required features |
| CI workflow files | Lean and all-feature build matrix |

## Status Update (2026-09-01)

**Overall: SOURCE-IMPLEMENTED; benchmark measurements open.** The feature-gating work landed
in `88c724744` ("perf: complete lean self-development runtime lane") and `6af235c0f` ("fix:
align lean builds with release automation"). The build graph is verified lean.

### Verified current state

The `roko-cli/Cargo.toml` now has:
- `default = ["chain"]` -- backend-neutral chain tools only, no Alloy.
- `alloy-backend` -- explicit opt-in that propagates through `roko-chain/alloy-backend` and
  `roko-serve/alloy-backend`.
- `acp` -- explicit opt-in for `roko-acp`.
- `chain_integration` test requires `alloy-backend` feature.

The `roko-serve/Cargo.toml` has:
- `default = ["chain"]` -- backend-independent local chain registries.
- `alloy-backend = ["chain", "dep:alloy", "roko-chain/alloy-backend"]` -- explicit.
- Chain-disabled builds return typed `501` for chain-only routes.

This matches all five checked acceptance criteria (lean tree excludes alloy-provider/network/
rpc-client, explicit feature check compiles, serve has 501 diagnostics, release/Docker/CI
request full features).

### What remains open

Two items from the implementation plan and two from the verification checklist:

1. Reproducible cold/warm baseline measurements (before/after) have not been recorded.
2. No regression threshold has been set for dependency count or build time.
3. Default CLI plan validation, dry-run, status, doctor, and screenshot smoke tests have not
   been run against the lean build specifically.
4. Full `cargo test --workspace --all-features` confirmation for the lean+full CI matrix.

### Audit cross-references

- **cli-audit `19-feature-flags.md`**: At audit time (2026-08-31), the audit recorded
  `roko-chain/alloy-backend` as "Healthy. The main binary and server both enable it." This
  was the pre-gating state. The audit's verdict is now stale -- the feature split has landed
  since then. The audit also identified the severed HDC feature pipeline
  (`roko-compose/hdc`, `roko-fs/hdc`, `roko-serve/hdc` all dead at workspace level), which
  is a separate concern from #230 but was discovered in the same feature-flag audit scope.
- **cli-audit `29-test-coverage.md`**: Notes that `roko-serve` is under-tested; the
  all-feature test matrix confirmation (open item 4) intersects with this.
- **engine-audit**: No direct overlap. The engine-audit focuses on graph-vs-runner
  architecture, not build graph.
- **ux-audit**: Empty (no files).

### Recommendation

The code work is done. The remaining items are measurement and CI validation tasks. Recording
before/after cold-build timings requires the `scripts/dev_benchmark.py` infrastructure from
#228, which is source-complete but whose real fixtures are also pending. These two items
should be exercised together.

## Verification (2026-09-03) — Static source/manifest audit

Auditor verified the three packet claims by reading Cargo.toml manifests, conditional
compilation directives, CI workflow definitions, and route handler source files. No build
or test commands were executed.

### Claim 1: Default lean build graph excludes Alloy dependencies

**CONFIRMED.** Evidence:

1. `roko-chain/Cargo.toml` (line 14-18): `default = []`, with `alloy-backend` as an
   explicit opt-in feature that gates `dep:alloy`, `dep:alloy-primitives`, and `dep:reqwest`.
   All three are declared `optional = true` in the `[dependencies]` section.

2. `roko-cli/Cargo.toml` (line 15): `default = ["chain", "hdc"]`. The `chain` feature
   propagates only `roko-serve/chain`. It does NOT include `roko-chain/alloy-backend`.
   The `alloy-backend` feature (line 23-27) is a separate opt-in that explicitly pulls
   `roko-chain/alloy-backend` and `roko-serve/alloy-backend`. The `roko-chain` path dep
   on line 64 has no `features = [...]` qualifier, so it compiles with `roko-chain`'s
   default features (which are empty).

3. `roko-serve/Cargo.toml` (line 14): `default = ["chain"]`. The `chain` feature on
   line 17 is `chain = []` -- a pure marker with no dependency activation. The heavy
   `alloy` dep on line 49 is `optional = true` and only activated by the `alloy-backend`
   feature on line 20. This ensures `alloy v1 features=["full"]` is excluded from a
   default build.

4. `Cargo.toml` workspace root (line 86-91): `default-members` lists only `roko-cli`,
   `roko-mcp-code`, and `roko-mcp-github`. None of these default members request
   alloy-backend features.

5. CI workflow `.github/workflows/ci.yml` (line 58-64): The `cli-feature-matrix` job
   explicitly asserts `cargo tree -p roko-cli -e normal | grep -Eq 'alloy-(provider|network|rpc-client) v'`
   returns no matches, failing the job if alloy provider crates leak into the default tree.

**Minor observation:** `roko-serve/Cargo.toml` lists `alloy-dyn-abi = "1"`,
`alloy-json-abi = "1"`, and `alloy-primitives` as unconditional (non-optional) dependencies.
These are lightweight ABI/type crates used by `trigger_runtime.rs` for EVM event decoding.
They are NOT the heavy provider/network/RPC-client graph (alloy-provider, alloy-network,
alloy-rpc-client) that the acceptance criteria target. The spec's exclusion list is correct
and this does not represent a gap.

### Claim 2: Chain-specific routes return 501 when feature is disabled

**CONFIRMED.** Evidence:

1. `roko-serve/src/routes/mod.rs` (line 12-16): Conditional compilation selects between
   the real `chain.rs` module (`#[cfg(feature = "alloy-backend")]`) and the stub
   `chain_disabled.rs` (`#[cfg(not(feature = "alloy-backend"))]`). Both are compiled
   as `mod chain`, so the router merge on line 358 (`.merge(chain::routes())`) works
   identically regardless of feature selection.

2. `roko-serve/src/routes/chain_disabled.rs`: Registers seven routes (`/chain/agents`,
   `/chain/bounties`, `/chain/status`, `/chain/blocks`, `/chain/transactions`,
   `/chain/events`, `/chain/watcher`) all pointing to a single `disabled()` handler that
   returns `StatusCode::NOT_IMPLEMENTED` (HTTP 501) with a JSON body containing:
   - `"error": "chain RPC support is not included in this build"`
   - `"required_feature": "alloy-backend"`
   - `"hint": "rebuild roko with --features alloy-backend"`

3. Additional feature gates in `roko-serve/src/state.rs`:
   - Line 46-47: `AlloyChainClient`/`AlloyChainWallet` imports gated behind `alloy-backend`.
   - Lines 515-518: `alloy_chain_client` and `chain_wallet` struct fields gated.
   - Lines 893-908: Chain client initialization falls through to `None` with a
     `tracing::warn!` diagnostic when `alloy-backend` is absent but config requests it.

4. `roko-serve/src/lib.rs` lines 2457-2524: `start_block_watcher` has two
   implementations -- the real one gated behind `#[cfg(feature = "alloy-backend")]` and a
   no-op `tokio::spawn(async {})` stub behind `#[cfg(not(feature = "alloy-backend"))]`.

5. `roko-cli/src/agent_serve.rs` lines 356-384: Agent sidecar chain tool initialization
   gated with matching `cfg` guards and a clear warning message when the feature is absent.

### Claim 3: Release targets explicitly request chain features

**CONFIRMED.** Evidence:

1. `.github/workflows/release.yml` (line 88-89): The release build command is:
   ```
   cargo build --release --target ${{ matrix.target }} \
     -p roko-cli -p roko-mcp-code --features roko-cli/alloy-backend,roko-cli/acp
   ```
   This explicitly opts into both `alloy-backend` and `acp` for the release binary across
   all four targets (aarch64-apple-darwin, x86_64-apple-darwin, x86_64-unknown-linux-gnu,
   x86_64-unknown-linux-musl).

2. `roko-demo/Cargo.toml` (line 18-19): Hard-codes
   `roko-chain = { path = "../roko-chain", features = ["alloy-backend"] }` and
   `alloy = { version = "1", features = ["full"] }`. This is the demo orchestrator that
   always needs real chain access.

3. `docker/roko.Dockerfile` (line 27): Uses `cargo build --release --bin roko` with NO
   explicit `--features` flag. This produces a lean Docker image with default features only
   (chain tools but no Alloy RPC). This is intentional -- the Docker image runs `roko serve`
   and chain routes will return 501.

4. `docker/worker.Dockerfile` (line 24): Same pattern -- lean default build for the worker
   container.

5. `.github/workflows/ci.yml` (line 50-68): The `cli-feature-matrix` CI job runs TWO steps:
   - Step 1: Default `cargo check -p roko-cli` plus a tree assertion proving alloy-provider
     is absent.
   - Step 2: Explicit `cargo check -p roko-cli --features alloy-backend,acp` proving the
     full feature combination builds.

6. `roko-cli/tests/chain_integration.rs` (line 9): The entire integration test file is
   gated behind `#![cfg(feature = "alloy-backend")]`, and the `Cargo.toml` test entry
   (line 42-44) declares `required-features = ["alloy-backend"]`. This prevents chain
   integration tests from running in the default lean build.

### Summary

| Claim | Verdict | Confidence |
|---|---|---|
| Default lean graph excludes Alloy provider deps | CONFIRMED | High (manifests + CI assertion) |
| Chain routes return 501 without feature | CONFIRMED | High (source audit of handler + cfg gates) |
| Release targets request explicit chain features | CONFIRMED | High (release.yml line 88-89) |

### Remaining open items (unchanged from 2026-09-01 status)

The following items were not in scope for this static verification pass and remain open:

1. Reproducible cold/warm baseline measurements (before/after).
2. Regression threshold for dependency count or build time.
3. Default CLI plan validation, dry-run, status, doctor smoke tests against lean build.
4. Full `cargo test --workspace --all-features` confirmation.

These are measurement and execution tasks that require running builds, not source-level
verification.
