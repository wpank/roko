# 230 — Feature-Gate Chain/Alloy from the Default CLI Build

> **Status: SOURCE-IMPLEMENTED / BUILD GRAPH VERIFIED, REAL BENCHMARK EVIDENCE OPEN** (2026-08-31,
> `88c724744` + `6af235c0f`). The default development graph is lean; full provider/chain/ACP/
> embedded-frontend behavior is selected explicitly by release, Docker, CI, and dedicated targets.
> Fixed-SHA cold/warm automation is source-complete in `d1b94b139`, and the protected cache lane
> (`97f897200` + `8c82c5b1b`) avoids erasing warm dependencies outside explicit cleanup. The final
> checkpoint verified the lean dependency tree, no-default serve check, explicit Alloy/ACP CLI
> check, and current default CLI build. Representative before/after repetitions, the complete
> all-feature test matrix, and release jobs remain open.

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
