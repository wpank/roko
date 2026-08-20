# 42 — Duplicate Type Consolidation

**Priority**: P3 — Maintenance burden; identical structs must be kept in sync independently
**Size**: S (1 day)
**Crates**: `roko-core` (`crates/roko-core/`), `roko-runtime` (`crates/roko-runtime/`), `roko-agent` (`crates/roko-agent/`), `roko-cli` (`crates/roko-cli/`), `roko-agent-server` (`crates/roko-agent-server/`), `roko-compose` (`crates/roko-compose/`)
**Depends on**: None

---

## Background

In a large codebase that grew through parallel development, the same types sometimes get defined independently in multiple crates when developers cannot use a shared dependency without risking circular imports. The result is duplicated maintenance: any change to the type (adding a field, changing a derive, fixing serialization) must be applied in multiple places, and there is no compile-time guarantee the copies remain consistent.

This codebase has three such clusters. Each cluster has a clear resolution: either move the canonical type to a shared dependency crate that both users already depend on, or rename the conflicting types so they are unambiguous.

The `roko-core` crate holds shared primitives and is already a dependency of both `roko-runtime` and `roko-agent`, so it is the correct home for shared types that currently live in the other two. Neither `roko-runtime` nor `roko-agent` depends on the other (no cycle), and both already import from `roko-core`.

## Current State

1. **`GitOpsConfig` (9 fields), `GitOpsRetryPolicy` (4 fields), and `ConfigDrift` (3-variant enum) are defined identically in two files:**
   - `crates/roko-runtime/src/lifecycle.rs` lines 310-390 — authoritative copy, re-exported from `crates/roko-runtime/src/lib.rs:98`
   - `crates/roko-agent/src/lifecycle.rs` lines 2396-2472 — duplicate copy, used only inside `roko-agent`
   - `roko-agent` does NOT depend on `roko-runtime` (confirmed: no `roko-runtime` entry in `crates/roko-agent/Cargo.toml`), so the types were duplicated rather than shared. Both crates already depend on `roko-core`.
   - The GitOps types are NOT used cross-crate outside of these two files. No external crate imports them from either location.

2. **Three separate enums all named `DispatchError` with no relation to each other:**
   - `crates/roko-core/src/dispatch_plan.rs:272` — provider-selection failures (`MissingAuth`, `UnsupportedProvider`, `CapabilityMismatch`, `AmbiguousProvider`, `AmbiguousModel`, `ProviderFailure`, `Cancelled`, `BudgetExceeded`, …)
   - `crates/roko-cli/src/dispatch/outcome.rs:77` — pre-spawn runner rejections (`BudgetExceeded`, `NoModelAvailable`, `PreValidationFailed`, `SpawnFailed`)
   - `crates/roko-agent-server/src/state.rs:40` — sidecar message dispatch failures (`NotConfigured`, `DispatchFailed`)
   - No cross-crate usage found: none of the three is imported under the `DispatchError` name in any other crate. All three are crate-local.

3. **Two separate traits both named `ContextBidder` with different method signatures:**
   - `crates/roko-compose/src/context_provider.rs:682` — compose-time bidder: `fn propose_context(&self, ...) -> Vec<ContextCandidate>`
   - `crates/roko-runtime/src/heartbeat_attention.rs:665` — runtime auction bidder: `fn generate_candidates(&self, ctx: &BidderContext) -> Vec<ContextCandidate>`
   - The runtime trait has six implementations (`NeuroBidder`, `DaimonBidder`, `IterationMemoryBidder`, `CodeIntelligenceBidder`, `PlaybookRulesBidder`, `ResearchArtifactsBidder`, `TaskContextBidder`)
   - The compose-time trait has a `ContextBidderRegistry` at line 696 with a `LearningContextBidder` (line 447) implementation

## Implementation Plan

### Step 1: Move GitOps types to `roko-core`

Create `crates/roko-core/src/gitops.rs` with the three types copied from `roko-runtime/src/lifecycle.rs` (lines 308-410). Use the `roko-runtime` version as the source of truth for doc comments.

Add `pub mod gitops;` to `crates/roko-core/src/lib.rs` and re-export the types at the crate root (`pub use gitops::{GitOpsConfig, GitOpsRetryPolicy, ConfigDrift};`).

In `crates/roko-runtime/src/lifecycle.rs`, replace the three struct/enum definitions with `use roko_core::{GitOpsConfig, GitOpsRetryPolicy, ConfigDrift};`. Update `crates/roko-runtime/src/lib.rs:98` to re-export from `roko_core` instead of `lifecycle`.

In `crates/roko-agent/src/lifecycle.rs`, delete the duplicate struct/enum definitions (lines 2394-2475). Replace any usage in that file with `use roko_core::{GitOpsConfig, GitOpsRetryPolicy, ConfigDrift};`. Since `roko-agent` already depends on `roko-core`, no `Cargo.toml` change is needed.

Estimated diff: ~120 lines removed from two files, ~80 lines added to `roko-core/src/gitops.rs`, ~10 lines of `use` statements added.

### Step 2: Rename the three `DispatchError` enums

Rename each to reflect its layer and purpose:

- `crates/roko-core/src/dispatch_plan.rs:272`: rename `DispatchError` → `ProviderDispatchError`. Update all uses in that file and add `pub type DispatchError = ProviderDispatchError;` temporarily if the old name is used in tests.
- `crates/roko-cli/src/dispatch/outcome.rs:77`: rename `DispatchError` → `RunnerDispatchError`. Update all uses within `roko-cli`.
- `crates/roko-agent-server/src/state.rs:40`: rename `DispatchError` → `SidecarDispatchError`. Update all uses within `roko-agent-server`.

Check for tests that use the old name and update them. Since none are imported cross-crate, this is a local rename in each crate.

Estimated diff: ~15 name occurrences across 3 files.

### Step 3: Rename `ContextBidder` in `roko-compose`

In `crates/roko-compose/src/context_provider.rs`, rename:
- `trait ContextBidder` → `trait ComposeBidder`
- `ContextBidderRegistry` → `ComposeBidderRegistry`

Update all implementations and usages within `roko-compose`. The runtime trait in `roko-runtime/src/heartbeat_attention.rs` keeps the name `ContextBidder` (it has more implementations and is the primary usage).

Estimated diff: ~20 name occurrences in one file.

## Acceptance Criteria

1. `grep -rn 'struct GitOpsConfig' crates/ --include='*.rs' | grep -v target/` returns exactly 1 hit, located in `crates/roko-core/src/gitops.rs`.
2. `grep -rn 'enum DispatchError' crates/ --include='*.rs' | grep -v target/` returns zero hits; the three enums exist under unique names.
3. `grep -rn 'trait ContextBidder' crates/ --include='*.rs' | grep -v target/` returns exactly 1 hit, in `crates/roko-runtime/src/heartbeat_attention.rs`; the compose-time trait has been renamed to `ComposeBidder`.
4. `cargo build --workspace` passes with no errors.
5. `cargo test --workspace` passes with no regressions.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] `grep -rn 'struct GitOpsConfig' crates/ --include='*.rs' | grep -v target/` shows exactly 1 result in `roko-core`
- [ ] `grep -rn 'enum DispatchError' crates/ --include='*.rs' | grep -v target/` shows 0 results
- [ ] `grep -rn 'trait ContextBidder' crates/ --include='*.rs' | grep -v target/` shows 1 result in `roko-runtime`
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` is clean

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-core/src/gitops.rs` (new) | Move `GitOpsConfig`, `GitOpsRetryPolicy`, `ConfigDrift` here from `roko-runtime` |
| `crates/roko-core/src/lib.rs` | Add `pub mod gitops;` and re-export the three types |
| `crates/roko-runtime/src/lifecycle.rs` | Delete duplicate definitions (lines 308-410); add `use roko_core::{...}` |
| `crates/roko-runtime/src/lib.rs` | Update re-exports to use `roko_core` source |
| `crates/roko-agent/src/lifecycle.rs` | Delete duplicate definitions (lines 2394-2475); add `use roko_core::{...}` |
| `crates/roko-core/src/dispatch_plan.rs` | Rename `DispatchError` → `ProviderDispatchError` (line 272) |
| `crates/roko-cli/src/dispatch/outcome.rs` | Rename `DispatchError` → `RunnerDispatchError` (line 77) |
| `crates/roko-agent-server/src/state.rs` | Rename `DispatchError` → `SidecarDispatchError` (line 40) |
| `crates/roko-compose/src/context_provider.rs` | Rename `ContextBidder` → `ComposeBidder`, `ContextBidderRegistry` → `ComposeBidderRegistry` (line 682) |
