# Runtime Foundation And Extraction Checklist

## Scope

Use this file for the runtime-core extraction work: types, lifecycle, extension chain, and event fabric.

## Implementation checklist

- [ ] Define the runtime ownership boundary before changing code.
  - Runtime crate owns heartbeat, process supervision, cancellation, event distribution, and durable runtime-facing state transitions.
  - Orchestrator crate owns plan DAG execution, recovery, resource budgets, and worktree scheduling.
  - CLI crate owns command parsing, user I/O, and temporary compatibility shims.
- [ ] Audit and document the current type inventory.
  - Map existing runtime types in `crates/roko-runtime/src/`.
  - Map overlapping state in `crates/roko-orchestrator/src/executor/` and `crates/roko-cli/src/orchestrate.rs`.
  - Identify which existing structs become canonical instead of inventing duplicates.
- [ ] Introduce or formalize the minimum runtime types.
  - `CognitiveTier`
  - `ExtensionLayer`
  - `DomainProfile`
  - `RuntimeEvent`
  - `HeartbeatPipeline`
  - any replacement for ad hoc agent lifecycle state currently held in the CLI
- [ ] Reuse existing runtime/event machinery where possible.
  - Extend `crates/roko-runtime/src/event_bus.rs` instead of adding a second bus.
  - Add filtered subscriptions and typed events instead of stringly typed fan-out.
- [ ] Define the extension contract against the current workspace.
  - One trait for lifecycle hooks.
  - Explicit ordering by `ExtensionLayer`.
  - Error contract for hook failure, timeout, and no-op behavior.
  - Clear statement of which hooks are synchronous vs async.
- [ ] Decide the first extraction target from `orchestrate.rs`.
  - Heartbeat tick assembly
  - context assembly
  - agent dispatch wrapper
  - learning feedback writeback
  - post-run conductor notifications
- [ ] Create extension-chain assembly rules.
  - deterministic ordering;
  - duplicate detection;
  - per-domain enable/disable;
  - feature-flagged experimental extensions.
- [ ] Define domain profile loading.
  - Start from `roko.toml` plus existing agent/task config structures.
  - Allow a profile to select tools, gates, context mix, routing defaults, and extension set.
- [ ] Keep crate creation disciplined.
  - Do not add `roko-ext-*` crates until the extension trait, dependency direction, and one real extracted extension are all agreed.
  - First prove the pattern inside existing crates or with one new crate only.

## Concrete file touchpoints

- `crates/roko-runtime/src/lib.rs`
- `crates/roko-runtime/src/event_bus.rs`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/process.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/agent_spawn.rs`
- `crates/roko-cli/src/agent_exec.rs`
- `crates/roko-orchestrator/src/executor/mod.rs`

## Verification checklist

- [ ] `cargo check -p roko-runtime -p roko-cli -p roko-orchestrator`
- [ ] Runtime events can be subscribed to without pulling CLI-only types into `roko-runtime`.
- [ ] No circular dependency is introduced between runtime, orchestrator, agent, and CLI crates.
- [ ] At least one extracted extension runs through the same lifecycle on repeated executions.

## Acceptance criteria

- A fresh engineer can point to one canonical runtime API surface.
- `orchestrate.rs` is smaller because real ownership moved, not because logic was merely re-exported.
- Extension ordering is deterministic and test-covered.
- Domain profile behavior is explicit enough to support later code/chain/research specializations.
