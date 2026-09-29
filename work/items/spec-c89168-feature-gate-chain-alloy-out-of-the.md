+++
id = "spec-c89168"
kind = "spec"
title = "Feature-gate chain/Alloy out of the default CLI build: remaining benchmark evidence, all-feature test matrix and release jobs"
status = "open"
triage = "verified"
severity = "p2"
size = "M"
subsystem = ["workspace/build"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/230-chain-alloy-default-build-feature-gating.md"
anchors = ["crates/roko-cli/Cargo.toml::alloy-backend", "crates/roko-serve/Cargo.toml::alloy-backend", "crates/roko-chain/Cargo.toml::alloy-backend", ".github/workflows/ci.yml::cli-feature-matrix", ".github/workflows/release.yml", "crates/roko-serve/src/routes/chain_disabled.rs::disabled", "crates/roko-cli/tests/chain_integration.rs"]
goal = "tooling"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn chain_routes_return_501_without_alloy_backend' crates/roko-serve/ && grep -q -- '--all-features' .github/workflows/ci.yml && grep -Eq 'cargo test .*alloy-backend' .github/workflows/ci.yml && grep -q 'ROKO_CLI_MAX_NORMAL_DEPS' .github/workflows/ci.yml && ls benchmarks/feature-split/*.json >/dev/null 2>&1 && cargo test -p roko-serve chain_routes_return_501_without_alloy_backend"
+++

## Problem

The split that keeps Alloy's RPC/provider graph out of the default `roko-cli` build landed on 2026-08-31. The
spec's measurement and CI-coverage parts were never done:

1. **No before/after numbers.** Nobody has recorded cold and warm `cargo build -p roko-cli` time, dependency count,
   binary size or target-dir disk use for the lean default against the full Alloy graph. So it is unknown whether
   the split paid off. The motivating figure was 10–14 minute cold release builds in the August dogfood sessions.
2. **No regression threshold.** CI only asserts that `alloy-provider`, `alloy-network` and `alloy-rpc-client` are
   absent from the default tree. Any other growth goes unnoticed.
3. **Non-default features go untested.** CI's clippy and tests run with default features only, and no manifest
   enables `alloy-backend`. So the Alloy-gated code is only `cargo check`ed: lib and bin, no test targets, no
   clippy. The `tui-png`, `otlp` and `chaos` code is never compiled in CI. The lean-build 501 stub for chain
   routes has no test.
4. **The release build is unproven.** `release.yml` builds with `--features roko-cli/alloy-backend,roko-cli/acp`,
   but it triggers on `v*` tags and `git tag -l 'v*'` is empty. Whether a manual `workflow_dispatch` run ever
   happened is unknown.

## Why it matters

Goal `tooling` (developing roko with roko): cold builds dominate dogfood cycle time, and every agent worktree pays
for them. Without numbers the split is an unmeasured claim. Without an all-features job, the Alloy, OTLP and
TUI-PNG paths can rot unseen until the release build fails (goal `release`).

Related: `find-8cc7ac` (every workflow fails on `main` today, so new jobs give no CI signal until it lands);
`gap-2122bd` (also edits `release.yml`); `gap-09e478` (#228 evidence bundle; owns the dev-lane benchmark scripts).

## Where

- `crates/roko-cli/Cargo.toml` `[features]` sets `default = ["acp", "chain", "hdc"]`. `alloy-backend` turns on
  `roko-chain/alloy-backend` and `roko-serve/alloy-backend`. Also defines `tui-png`. The `chain_integration`
  test has `required-features = ["alloy-backend"]`.
- `crates/roko-serve/Cargo.toml` sets `default = ["chain", "hdc"]`. `chain` adds only the light `alloy-dyn-abi`,
  `alloy-json-abi` and `alloy-primitives`. `alloy-backend` adds `alloy` with `features = ["full"]`. Also defines `otlp`.
- `crates/roko-chain/Cargo.toml` sets `default = []` and `alloy-backend = ["dep:alloy", "dep:reqwest"]`.
- Alloy-gated code:
  - `crates/roko-chain/src/lib.rs`: the `alloy_impl` module.
  - `crates/roko-serve/src/routes/mod.rs`: picks `chain.rs` or `chain_disabled.rs` as `mod chain`.
  - `crates/roko-serve/src/state.rs` and `crates/roko-serve/src/routes/agents.rs`.
  - `crates/roko-serve/src/lib.rs::start_block_watcher`: a real version and a no-op one.
  - Tests: `crates/roko-chain/tests/alloy_live.rs`, `crates/roko-cli/tests/chain_integration.rs`.
- `crates/roko-serve/src/routes/chain_disabled.rs::disabled` is the lean-build handler. Seven `/chain/*` GET routes
  return 501 with `required_feature: "alloy-backend"`.
- `.github/workflows/ci.yml` has two relevant jobs. `test` (lines 13–24) runs default-feature clippy and tests.
  `cli-feature-matrix` (lines 50–68) runs the lean tree assertion plus `cargo check -p roko-cli --features alloy-backend,acp`.
- `.github/workflows/release.yml:88-89` is the full-feature release build. It triggers on `v[0-9]+.*` tags or
  `workflow_dispatch`.
- `crates/roko-serve/build.rs:63` builds the embedded frontend for release builds unless `SKIP_FRONTEND_BUILD` is
  set. Set it when timing the Rust compile.

## Current state

- **Source split done:** `88c724744` ("perf: complete lean self-development runtime lane") and `6af235c0f`
  ("fix: align lean builds with release automation"), both 2026-08-31. A static audit on 2026-09-03 confirmed the
  manifests, the 501 stub and the release flags.
- **`acp` is now a default feature** (`2261ab17b`, 2026-09-07). The spec's "explicit opt-in" is stale. The `,acp`
  in CI and release is redundant but harmless.
- **Nothing enables `alloy-backend`.** `caf81ad43` (2026-09-05) dropped `roko-demo`'s `roko-chain` dependency with
  `alloy-backend`, so audit claim 3.2 is stale. `roko-demo` still depends on `alloy` `full` directly.
  `git grep alloy-backend -- '*Cargo.toml'` finds only feature definitions. So `cargo test --workspace` never
  compiles the gated code.
- **Live tests.** `alloy_live.rs` (3 tests) skips when no RPC endpoint answers (`ROKO_TEST_RPC_URL`, default
  `http://127.0.0.1:18545`). `chain_integration.rs` (8 tests) calls the live `https://mirage-devnet.up.railway.app`
  and fails, not skips, when it is unreachable. So a plain `cargo test --workspace --all-features` is unsafe in CI.
- **No coverage of the 501 stub.** `grep -rn '"/chain/' crates/roko-serve/tests` is empty.
- **No measurements.** `.roko/benchmarks/` holds only a micro-benchmark run. `benchmarks/` is tracked in git;
  `.roko/` is ignored.
- **`scripts/dev_benchmark.py` is not a build timer.** It is the #228 dev-lane benchmark: it runs real LLM plan
  fixtures with a spend ceiling of up to $50. Do not use it here.

## Plan

1. Add `scripts/measure_feature_split.sh` (bash, no LLM calls). On one commit, for the feature sets `default` and
   `alloy-backend`, run `REPS` repetitions (default 3). Each repetition does this:
   - Cold build into a fresh `T=$(mktemp -d)`:
     `CARGO_TARGET_DIR=$T SKIP_FRONTEND_BUILD=1 cargo build -p roko-cli --locked --timings [--features alloy-backend]`.
     Record wall seconds.
   - Warm build: `touch crates/roko-cli/src/main.rs`, then rebuild into the same `$T`.
   - Record the size of `$T/debug/roko` and `du -sk $T`.
   - Record the normal dependency count, and how many of those crates are `alloy*`:
     `cargo tree -p roko-cli -e normal --prefix none [--features alloy-backend] | sed 's/ (\*)$//' | sort -u | wc -l`.
   Also build the release profile once per set; the 10–14 minute figure was a release build. Record host info
   (`uname -a`, CPU count, `rustc -V`) and the commit sha. Write raw samples, medians and a `conclusion` string to
   `benchmarks/feature-split/<YYYY-MM-DD>-<sha>.json`. Remove only the script's own `mktemp` dirs, never `target/`.
   `--features alloy-backend` on the same commit stands in for "before". It turns on exactly the pre-split graph:
   `roko-chain/alloy-backend` plus serve's `alloy` `full`. The pre-split commit would mix in hundreds of unrelated
   changes.
2. Run the script on an idle machine with about 10 GiB free and commit the JSON. If the lean cold median is not
   materially lower (under about 10%), say so in `conclusion`. The spec allows documenting why the split is not
   worth it.
3. Regression guard. In `cli-feature-matrix`, set `env: ROKO_CLI_MAX_NORMAL_DEPS: <lean count + ~5%>`. Add a step
   that computes the count with the same formula and fails above the ceiling. Use a count, not a time: hosted
   runners are too noisy.
4. All-features compile and lint. Add a job that runs
   `SKIP_FRONTEND_BUILD=1 cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings`. It
   compiles every test target with every feature on (including `alloy-backend`, `otlp`, `tui-png` and `chaos`)
   without running live tests. Fix what it reports.
5. Offline Alloy tests. Add a step, on one line:
   `cargo test -p roko-chain -p roko-serve --features roko-chain/alloy-backend,roko-serve/alloy-backend`.
   Leave `chain_integration` out. If it should run at all, give it a separate `workflow_dispatch`-only job with
   network access to the mirage devnet.
6. Add `crates/roko-serve/tests/chain_disabled.rs` with `#![cfg(not(feature = "alloy-backend"))]` and
   `fn chain_routes_return_501_without_alloy_backend`. For all seven routes, assert 501 and
   `required_feature == "alloy-backend"`. Build the router the way `route_coverage_matrix.rs::test_router` does.
7. Lean smoke test with the default debug binary:
   - `roko plan validate <small plan dir>`, `roko status` and `roko doctor`;
   - `roko serve`, then `curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:6677/chain/status`, expecting 501.
     Whether that route needs auth is unknown.
   Record the results in the JSON or the closure evidence.
8. Build the release profile locally:
   `SKIP_FRONTEND_BUILD=1 cargo build --release -p roko-cli -p roko-mcp-code --locked --features roko-cli/alloy-backend,roko-cli/acp`.
   Running `release.yml` itself (a tag push or `workflow_dispatch`) is a public GitHub action, so ask Will first.

## Done when

- `benchmarks/feature-split/*.json` is committed. For lean against `alloy-backend` it holds cold and warm debug
  medians, one release build each, dependency counts, binary and target sizes, host info and a conclusion.
- `ci.yml` has the `ROKO_CLI_MAX_NORMAL_DEPS` ceiling in `cli-feature-matrix`, an all-features clippy job, and a
  one-line `cargo test` step with `alloy-backend` for `roko-chain` and `roko-serve`. These pass locally, and in CI
  once `find-8cc7ac` makes `main` green.
- `chain_routes_return_501_without_alloy_backend` passes and the lean smoke results are recorded.
- Verify: `grep -rqw 'fn chain_routes_return_501_without_alloy_backend' crates/roko-serve/ && grep -q -- '--all-features' .github/workflows/ci.yml && grep -Eq 'cargo test .*alloy-backend' .github/workflows/ci.yml && grep -q 'ROKO_CLI_MAX_NORMAL_DEPS' .github/workflows/ci.yml && ls benchmarks/feature-split/*.json >/dev/null 2>&1 && cargo test -p roko-serve chain_routes_return_501_without_alloy_backend`

## Notes

- **Don't redesign the split.** The manifests are the decided shape. `acp` stays a default feature (`2261ab17b`).
  `Dockerfile:38` builds the full graph; `docker/roko.Dockerfile` and `docker/worker.Dockerfile` stay lean on purpose.
- **Measurement needs an idle machine.** Expect hours of CPU (at least 6 cold builds). Numbers taken while other
  sessions build are worthless.
- **File overlap.** `ci.yml` is shared with `find-8cc7ac` and `release.yml` with `gap-2122bd`. Otherwise this is
  safe in parallel: it adds one script, one test file and `benchmarks/feature-split/`.
- **Ask before public actions.** Editing workflows in a branch is fine. Pushing, tagging or dispatching workflows
  needs Will's approval.
- **The imported notes are partly stale.** They date from 2026-08-31 to 2026-09-03. This body was checked against
  HEAD on 2026-09-29.

## Original notes


Imported 2026-09-29 from `tmp/backlog/230-chain-alloy-default-build-feature-gating.md` (tmp/backlog is frozen). Not yet checked against current code: the text below is the spec as last written, including its own status notes.

## Original spec

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
