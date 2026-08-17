# Duplicate Type Consolidation

**Priority**: P3
**Size**: S (1 day)

---

## Problem

Several types are defined identically (or near-identically) in multiple crates. Each copy
must be maintained independently, and callers in one crate cannot accept values from the
other without conversion. There are three distinct duplicate clusters:

### Cluster 1 — `GitOpsConfig` / `GitOpsRetryPolicy` / `ConfigDrift`

Byte-for-byte (modulo doc-comment wording) identical structs and enums defined twice:

| Location | Lines |
|---|---|
| `crates/roko-runtime/src/lifecycle.rs` | 310–374 |
| `crates/roko-agent/src/lifecycle.rs` | 2396–2460 |

Both define the same nine-field `GitOpsConfig`, the same four-field `GitOpsRetryPolicy`,
and the same `ConfigDrift` enum with `InSync` / `Diverged` / `Pending` variants and
identical `Default` impls. They exist because `roko-agent` cannot depend on
`roko-runtime` (cycle risk) so each crate defined its own copy.

### Cluster 2 — `DispatchError`

Three separate enums share the name `DispatchError` with no relation to each other:

| Location | Line | Purpose |
|---|---|---|
| `crates/roko-core/src/dispatch_plan.rs` | 272 | Provider-selection failures (MissingAuth, UnsupportedProvider, CapabilityMismatch, …) |
| `crates/roko-cli/src/dispatch/outcome.rs` | 77 | Pre-spawn runner rejections (BudgetExceeded, NoModelAvailable, SpawnFailed, …) |
| `crates/roko-agent-server/src/state.rs` | 40 | Sidecar message dispatch failures (NotConfigured, DispatchFailed) |

The name collision causes confusion when reading code that imports any of them, and means
`From` conversions or shared error-handling cannot be written without a fully-qualified path.

### Cluster 3 — `ContextBidder` trait

The same trait name is defined in two separate crates with different method signatures:

| Location | Line | Method signature |
|---|---|---|
| `crates/roko-compose/src/context_provider.rs` | 682 | `propose_context(&self, provider, request) -> Vec<ContextCandidate>` |
| `crates/roko-runtime/src/heartbeat_attention.rs` | 665 | `generate_candidates(&self, ctx: &BidderContext) -> Vec<ContextCandidate>` |

These represent two parallel systems for the same conceptual role (bidding for context
budget). Their existence as separate traits prevents a unified registry or shared
auction logic.

### What already exists

| Component | Location | Status |
|---|---|---|
| `GitOpsConfig` | `roko-runtime/src/lifecycle.rs:310` | EXISTS (authoritative) |
| `GitOpsConfig` | `roko-agent/src/lifecycle.rs:2396` | EXISTS (duplicate) |
| `GitOpsRetryPolicy` | `roko-runtime/src/lifecycle.rs:349` | EXISTS (authoritative) |
| `GitOpsRetryPolicy` | `roko-agent/src/lifecycle.rs:2435` | EXISTS (duplicate) |
| `ConfigDrift` | `roko-runtime/src/lifecycle.rs:374` | EXISTS (authoritative) |
| `ConfigDrift` | `roko-agent/src/lifecycle.rs:2460` | EXISTS (duplicate) |
| `DispatchError` (provider-selection) | `roko-core/src/dispatch_plan.rs:272` | EXISTS |
| `DispatchError` (runner pre-spawn) | `roko-cli/src/dispatch/outcome.rs:77` | EXISTS |
| `DispatchError` (sidecar) | `roko-agent-server/src/state.rs:40` | EXISTS |
| `ContextBidder` (compose-time) | `roko-compose/src/context_provider.rs:682` | EXISTS |
| `ContextBidder` (runtime auction) | `roko-runtime/src/heartbeat_attention.rs:665` | EXISTS |

### What is missing

1. **A shared home for the GitOps types.** `roko-core` already holds shared primitive
   types and is depended on by both `roko-runtime` and `roko-agent`. Moving
   `GitOpsConfig`, `GitOpsRetryPolicy`, and `ConfigDrift` there eliminates both copies.

2. **Distinct names for the three `DispatchError` enums.** Each covers a different
   layer; renaming them to `PlanDispatchError` (core), `AgentDispatchError` (cli), and
   `SidecarDispatchError` (agent-server) ends the ambiguity.

3. **Renamed or merged `ContextBidder` traits.** The compose-time trait should be
   renamed `ComposeBidder` (or similar) to make the distinction explicit. Whether the
   two systems are eventually merged is a product decision; the immediate fix is to
   give them distinct names so callers cannot accidentally import the wrong one.

---

## Proposed changes

### Change A: move GitOps types to `roko-core`

Add `pub mod gitops;` (or inline into `roko-core/src/types.rs`) and move the three
types from `roko-runtime/src/lifecycle.rs`. Update `roko-agent/src/lifecycle.rs` to
remove its copies and re-export or use the `roko-core` versions.

Estimated diff: ~120 lines removed, ~10 lines of `use roko_core::gitops::*` added.
Risk: low — `roko-core` has no runtime dep, both crates already depend on it.

### Change B: rename `DispatchError` variants

- `roko-core/src/dispatch_plan.rs`: rename `DispatchError` → `ProviderDispatchError`
- `roko-cli/src/dispatch/outcome.rs`: rename `DispatchError` → `RunnerDispatchError`
- `roko-agent-server/src/state.rs`: rename `DispatchError` → `SidecarDispatchError`

Add `pub type` aliases in each crate if any downstream code uses the old names publicly.

Estimated diff: ~15 lines across 3 files (renames + any re-export aliases).
Risk: low — all three types are crate-local; no cross-crate public usage confirmed.

### Change C: rename `ContextBidder` in `roko-compose`

Rename the compose-time trait from `ContextBidder` to `ComposeBidder` (or
`ColdStartBidder`). The runtime auction trait in `roko-runtime` is the primary user of
the name and has more implementations.

Estimated diff: ~20 lines (rename + `ContextBidderRegistry` → `ComposeBidderRegistry`).
Risk: low — the compose-time trait has no external implementations outside the crate.

---

## Acceptance criteria

1. `grep -rn 'struct GitOpsConfig' crates/ --include='*.rs' | grep -v target/` returns
   exactly 1 hit (in `roko-core`).
2. `grep -rn 'enum DispatchError' crates/ --include='*.rs' | grep -v target/` returns
   zero hits; each former use site has a unique name.
3. `grep -rn 'trait ContextBidder' crates/ --include='*.rs' | grep -v target/` returns
   at most 1 hit (the runtime auction trait); the compose-time trait has been renamed.
4. `cargo build --workspace` passes with no errors.
5. `cargo test --workspace` passes with no regressions.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## References

- `crates/roko-runtime/src/lifecycle.rs` — authoritative GitOps types (lines 310–374)
- `crates/roko-agent/src/lifecycle.rs` — duplicate GitOps types (lines 2396–2460)
- `crates/roko-core/src/dispatch_plan.rs:272` — `DispatchError` (provider-selection)
- `crates/roko-cli/src/dispatch/outcome.rs:77` — `DispatchError` (runner pre-spawn)
- `crates/roko-agent-server/src/state.rs:40` — `DispatchError` (sidecar)
- `crates/roko-compose/src/context_provider.rs:682` — `ContextBidder` (compose-time)
- `crates/roko-runtime/src/heartbeat_attention.rs:665` — `ContextBidder` (runtime auction)
