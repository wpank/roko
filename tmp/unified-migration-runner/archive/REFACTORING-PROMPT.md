# Roko Codebase Refactoring — Implementation Prompt

> Copy this entire file into a fresh Claude Code session.

You are refactoring a Rust codebase called Roko (agent toolkit, 18 crates, ~177K LOC). You have 5 independent refactoring tracks to execute. Each is a mechanical extraction — zero behavior change, just moving code into focused modules.

## Critical Rules

1. **Read `CLAUDE.md` at project root first.** It has mandatory project instructions.
2. **Read before writing.** Always read a file before modifying it.
3. **Never push to main.** Create a branch per track.
4. **Ask before git actions.** Committing, pushing, creating PRs — ask first.
5. **Verify after every change.** `cargo check --workspace` must pass.
6. **Before final commit, run all three:**
   ```bash
   cargo +nightly fmt --all
   cargo clippy --workspace --no-deps -- -D warnings
   cargo test --workspace
   ```
7. **Never reimplement what exists.** Search before writing: `grep -rn 'Name' crates/ --include='*.rs' | grep -v target/`
8. **Preserve all behavior exactly.** These are structural refactors, not feature changes.
9. **Rustc 1.91+ required.** Run `rustup update stable` if needed (alloy deps).
10. **Preserve existing re-exports.** `crates/roko-cli/src/lib.rs` re-exports `pub use orchestrate::{OrchestrationReport, PlanRunReport, PlanRunner};` — do not break this.

## Project Layout

```
/Users/will/dev/nunchi/roko/roko/          — workspace root
  crates/roko-core/src/                    — kernel: types, config, traits
  crates/roko-cli/src/                     — CLI binary: main.rs, orchestrate.rs, TUI
  crates/roko-learn/src/                   — learning: episodes, routing, experiments
  crates/roko-serve/src/                   — HTTP server: routes, state, events
  crates/roko-gate/src/                    — verification gates
  crates/roko-agent/src/                   — LLM backends, dispatch
  crates/roko-orchestrator/src/            — plan DAG, executor
  crates/roko-compose/src/                 — prompt assembly
  crates/roko-fs/src/                      — file storage
  crates/roko-std/src/                     — defaults, mock dispatcher
  tmp/unified/                             — unified architecture spec
  tmp/dogfood/                             — known issues
  CLAUDE.md                                — project instructions (READ THIS)
```

## Execution Order

These tracks touch different files and can run sequentially in one session:

1. **Track A: main.rs decomposition** (~1 hour) — branch `refactor/main-rs-decompose`
2. **Track B: config/schema.rs decomposition** (~1 hour) — branch `refactor/config-schema`
3. **Track C: cascade_router.rs refactor** (~1 hour) — branch `refactor/cascade-router`
4. **Track D: serve routes consolidation** (~1 hour) — branch `refactor/serve-routes`
5. **Track E: Cell trait + protocol renames** (~2 hours) — branch `refactor/cell-trait-renames`

Do them in order. Commit each track before starting the next. Track E touches many crates and should go last to minimize conflicts.

---

# Track A: main.rs Decomposition

**Goal**: Split `crates/roko-cli/src/main.rs` (12,690 lines, 43 `cmd_*` functions) into subcommand modules.

**Read first**:
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/lib.rs`

## Step A1: Create commands/ directory

```bash
mkdir -p crates/roko-cli/src/commands
```

Create `crates/roko-cli/src/commands/mod.rs` with module declarations.
Add `pub mod commands;` to `crates/roko-cli/src/lib.rs`.

**Note**: Clap arg definitions (`#[derive(Parser)]` structs, `enum SubCommand`, etc.) stay in main.rs. Only move the `cmd_*` handler functions. The match dispatch stays in main.rs too — it just calls into the command modules instead of inlining the logic.

**Note**: `lib.rs` re-exports `pub use orchestrate::{OrchestrationReport, PlanRunReport, PlanRunner}`. Do NOT break this — other crates and the runner v2 may depend on it.

## Step A2: Extract plan commands

**IMPORTANT**: The `plan run` section of `cmd_plan` (~lines 5770-5830) may have been
modified by a concurrent runner v2 session on branch `wp-runner-v2`. If that branch
exists and has changes to main.rs, coordinate: either rebase after, or skip moving
the `plan run` handler and just move the other plan subcommands.

The plan command handler starts at main.rs line ~5587 (`async fn cmd_plan`). It contains:
- `cmd_plan()` — match dispatcher for plan subcommands
- `cmd_plan_dry_run()` — line ~6103
- `cmd_plan_validate()` — line ~6275
- The `plan run` handler at ~5770-5830

Move ALL plan-related `cmd_*` functions to `crates/roko-cli/src/commands/plan.rs`.
Make them `pub(crate)`. Replace the inline implementations in main.rs with calls:
```rust
PlanCmd::List { .. } => commands::plan::cmd_plan_list(...).await,
```

**Verify**: `cargo check -p roko-cli && cargo run -p roko-cli -- plan --help`

## Step A3: Extract agent commands

Agent handler at main.rs line ~2583 (`async fn cmd_agent`). Move to `commands/agent.rs`.

## Step A4: Extract PRD commands

PRD handler at main.rs line ~8645 (`async fn cmd_prd`). Move to `commands/prd.rs`.

## Step A5: Extract research commands

Research handler at main.rs line ~6398 (`async fn cmd_research`). Move to `commands/research.rs`.

## Step A6: Extract config commands

Config handling is scattered through main.rs. Search for `cmd_config`, `ConfigCmd`, and related match arms. Move to `commands/config.rs`.

## Step A7: Extract remaining command groups

Move each remaining group to its own file:
- `commands/job.rs` — line ~7225 (`cmd_job`)
- `commands/learn.rs` — line ~9974 (`cmd_learn`)
- `commands/deploy.rs` — line ~9735 (`cmd_deploy`, `cmd_deploy_fly`, `cmd_deploy_docker`, `cmd_deploy_railway`)
- `commands/server.rs` — `cmd_daemon` (line ~2543), serve handler
- `commands/util.rs` — remaining: run, status, doctor, init, dashboard, replay, inject, index, new, explain, completions, chat

## Step A8: Final cleanup

main.rs should now be ~500-1000 lines: clap arg definitions + match arms calling `commands::*`.
Move any stray helper functions out of main.rs.

**Verify**:
```bash
wc -l crates/roko-cli/src/main.rs  # target: <1000
cargo check --workspace
cargo test -p roko-cli
```

---

# Track B: config/schema.rs Decomposition

**Goal**: Split `crates/roko-core/src/config/schema.rs` (6,061 lines) into focused section files.

**Read first**:
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-core/src/config/mod.rs`

The file contains 30+ config struct types. Here are the main ones with approximate line ranges:

| Struct | Line | Section |
|--------|------|---------|
| `RokoConfig` | 44 | root |
| `ToolsConfig` | 1346 | tools |
| `ChainConfig` | 1400 | chain |
| `RelayConfig` | 1442 | relay |
| `ProviderConfig` | 1656 | providers |
| `PrdConfig` | 1878 | prd |
| `ProjectConfig` | 1892 | project |
| `AgentConfig` | 1935 | agent |
| `DataLlmConfig` | 2138 | data_llm |
| `GatesConfig` | 2297 | gates |
| `PipelineConfig` | 2512 | pipeline |
| `RoutingConfig` | 2677 | routing |
| `BudgetConfig` | 2748 | budget |
| `ConductorConfig` | 2787 | conductor |
| `LearningConfig` | 2899 | learning |
| `DemurrageConfig` | 2999 | demurrage |
| `AttentionConfig` | 3077 | attention |
| `TuiConfig` | 3291 | tui |
| `ServeConfig` | 3313 | serve |
| `SchedulerConfig` | 3341 | scheduler |

## Step B1: Create submodule files

**Note**: The `config/` directory already has: `mod.rs`, `schema.rs`, `compat.rs`, `hot_reload.rs`, `presets.rs`. Don't overwrite these — add new files alongside them.

Create these files in `crates/roko-core/src/config/`:
- `agent.rs` — AgentConfig + DataLlmConfig + related nested types + their impls
- `server.rs` — ServeConfig + TuiConfig + SchedulerConfig + related impls
- `budget.rs` — BudgetConfig + related impls
- `learning.rs` — LearningConfig + RoutingConfig + ConductorConfig + AttentionConfig + DemurrageConfig + GatesConfig + PipelineConfig
- `deploy.rs` — ChainConfig + RelayConfig + related impls
- `providers.rs` — ProviderConfig + ToolProfileConfig + ToolsConfig + related impls
- `project.rs` — ProjectConfig + PrdConfig + GoalsConfig + related impls

## Step B2: Move types with their impls, default functions, and serde helpers

For each struct being moved:
1. Move the struct definition
2. Move all `impl` blocks for that struct
3. Move any `fn default_*()` functions used by `#[serde(default = "...")]`
4. Move any helper enums used only by that struct
5. Add `use super::*;` or specific imports in the new file
6. In the new file: `pub use` everything that was public

## Step B3: Update schema.rs

Keep in `schema.rs`:
- `RokoConfig` struct (with field types imported from submodules)
- `from_toml()`, `to_toml()`, `to_toml_pretty()`, `is_stale()`
- `effective_providers()`, `effective_models()`
- `CURRENT_SCHEMA_VERSION`, `CURRENT_CONFIG_VERSION`
- `ConfigChangeReport`

## Step B4: Update mod.rs re-exports

In `config/mod.rs`, re-export all public types from submodules so external crates see no change.

## Step B5: Move tests

Move `#[cfg(test)]` blocks to the submodule that owns the types being tested.

**Verify**:
```bash
wc -l crates/roko-core/src/config/schema.rs  # target: <1000
cargo check --workspace
cargo test -p roko-core -- config
```

---

# Track C: cascade_router.rs Refactor

**Goal**: Split `crates/roko-learn/src/cascade_router.rs` (5,197 lines) into focused submodules.

**Read first**:
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/model_router.rs` (legacy, 2,323 lines — check for duplication)
- `crates/roko-learn/src/lib.rs`

The file has these main sections:

| Section | Lines | Content |
|---------|-------|---------|
| Type definitions | 63-530 | CascadeStage, CascadeModel, CascadeSelection, CascadeRouteExplanation, etc. |
| Helper functions | 530-1080 | slug_family(), slug_matches(), is_premium_model(), etc. |
| CascadeRouter struct + methods | 1080-3200 | 40+ methods: select(), observe(), route(), bias application |
| LinUCB integration | 3200-4000 | LinUCB arm management, UCB score computation |
| Persistence | 4000-4500 | save(), load(), snapshot serialization |
| Explanation | 4500-5000 | explain_route(), format candidates |
| Tests | 5000-5197 | Test module |

## Step C1: Create cascade/ subdirectory

```bash
mkdir -p crates/roko-learn/src/cascade
```

Create `crates/roko-learn/src/cascade/mod.rs`.

## Step C2: Extract type definitions

Move all structs/enums defined BEFORE the `CascadeRouter` struct (lines ~63-530) to `cascade/types.rs`:
- `CascadeStage`, `StageTransition`, `CascadeModel`, `CascadeSelection`
- `CascadeCandidateScore`, `CascadeRouteExplanation`, `CascadeRoutingExplanation`
- `CascadeRoutingCandidate`, `RoutingBias`, `KnowledgeHint`, `KnowledgeRoutingAdvice`
- `PerplexityObservation`, `GeminiObservation`, `CascadeObservationStats`

## Step C3: Extract helper functions

Move free functions (lines ~530-1080) to `cascade/helpers.rs`:
- `slug_family()`, `slug_matches()`, `is_premium_model()`
- Any other standalone utility functions

## Step C4: Extract LinUCB arm management

Move LinUCB-related code to `cascade/arms.rs`:
- LinUCB struct and its methods
- UCB score computation
- Arm observation updates

## Step C5: Extract persistence

Move save/load/snapshot code to `cascade/persistence.rs`:
- `save()`, `load()`, snapshot types
- JSONL read/write helpers
- Atomic write for cascade state

## Step C6: Extract explanation

Move explanation generation to `cascade/explain.rs`:
- `explain_route()`, `CascadeRouteExplanation` formatting
- Human-readable routing rationale

## Step C7: Add ModelSelector trait (OPTIONAL — behavior change)

**NOTE**: This step introduces a new abstraction, not just extraction. Skip if time-constrained.

Create `cascade/selector.rs` with a trait that abstracts model selection:
```rust
pub trait ModelSelector: Send + Sync {
    fn select(&self, candidates: &[ModelCandidate], context: &[f64]) -> ModelSelection;
    fn observe(&mut self, model: &str, reward: f64);
}
```
Make LinUCB implement this trait. Update CascadeRouter to use `Box<dyn ModelSelector>`.
This prepares for EFE routing (Phase 1) by making the selection algorithm pluggable.

## Step C8: Audit model_router.rs

Read `model_router.rs` and compare with cascade_router. If it's a legacy duplicate:
- Add `#[deprecated(note = "use CascadeRouter instead")]` to its public API
- Add doc comment explaining the relationship

## Step C9: Update cascade_router.rs

Keep in `cascade_router.rs`:
- `CascadeRouter` struct
- Core routing methods: `select()`, `select_for_frequency()`, `select_tier_with_active_inference()`
- Bias application methods
- Constructor and builder methods
- Import types/helpers from submodules

## Step C10: Update lib.rs re-exports

Ensure `crates/roko-learn/src/lib.rs` re-exports everything that was public.

**Verify**:
```bash
wc -l crates/roko-learn/src/cascade_router.rs  # target: <2000
cargo check --workspace
cargo test -p roko-learn
```

---

# Track D: Serve Routes Consolidation

**Goal**: Split 6 oversized route files in `crates/roko-serve/src/routes/`.

**Read first**:
- `crates/roko-serve/src/routes/mod.rs` — `build_router()` function
- `crates/roko-serve/src/routes/status.rs` — 2,490 lines, worst offender

## Step D1: Split status.rs → status/ directory

Create `crates/roko-serve/src/routes/status/mod.rs`. Extract:

- `status/health.rs` — `health()`, `relay_health()`, `parity_handler()`, `retention_handler()`, `statehub_snapshot()`
- `status/metrics.rs` — `metrics()`, `metrics_summary()`, `success_rate()`, `engagement()`, `c_factor_metrics()`, `model_efficiency()`, `gate_rate()`, `experiments_metric()`, `feedback_latency()`, `velocity()`, `coverage()`, `prometheus_metrics()`
- `status/episodes.rs` — `episodes()`, `signals()`
- `status/gates.rs` — `gate_summary()`, `gates_history()`, `gate_history()`
- `status/dashboard.rs` — `dashboard()`, `session_status()`, `operation_status()`, `truth_map_handler()`

Keep `status/mod.rs` with `pub fn routes()` that merges all sub-routers.

## Step D2: Split learning.rs → learning/ directory

- `learning/router_state.rs` — cascade router state, routing decision endpoints
- `learning/experiments.rs` — A/B experiment endpoints
- Keep `learning/mod.rs` with remaining efficiency/c-factor/playbook endpoints

## Step D3: Split plans.rs → plans/ directory

- `plans/execution.rs` — plan run execution, task dispatch
- Keep `plans/mod.rs` with CRUD (list, show, create)

## Step D4: Add missing endpoints

From `tmp/dogfood/01-endpoint-audit.md`:

In `status/dashboard.rs` or a new file:
- `GET /api/executor/state` — read `.roko/state/executor.json`, return contents
- `GET /api/learn/router` — read `.roko/learn/cascade-router.json`, return contents

In `crates/roko-serve/src/routes/neuro.rs` (may already exist):
- `GET /api/knowledge` — list neuro store entries
- `GET /api/knowledge?query=<topic>` — search

In plans:
- `GET /api/plans/:id` — individual plan state
- `GET /api/plans/:id/tasks` — task list from tasks.toml

## Step D5: Update routes/mod.rs

Update `build_router()` to import from new submodule directories.

**Verify**:
```bash
cargo check -p roko-serve
cargo test -p roko-serve
```

---

# Track E: Cell Trait + Protocol Renames

**Goal**: Define `Cell` trait, rename 6 existing traits to unified spec names, add backwards compatibility aliases.

**Read first**:
- `crates/roko-core/src/traits.rs` — current trait definitions (lines 36, 101, 165, 212, 240, 283, 337, 383)
- `crates/roko-core/src/lib.rs` — re-exports
- `tmp/unified/02-CELL.md` — spec for Cell trait and 9 protocols
- `crates/roko-core/Cargo.toml` — current dependencies

**IMPORTANT**: There are ~100 trait implementations across the workspace. Each rename must update all of them. Use `grep -rn 'impl OldName' crates/ --include='*.rs' | grep -v target` to find them all.

**IMPORTANT**: The traits are NOT async. Do NOT add `async_trait`. Keep the existing sync signatures.

**IMPORTANT**: A `Bus` trait already exists at traits.rs:383. Don't create a duplicate.

## Step E1: Define Cell trait

Create `crates/roko-core/src/cell.rs`:

```rust
//! Universal computation unit. All protocol traits extend Cell.

use std::time::Duration;

/// Unique cell identifier (content-addressed in future).
pub type CellId = String;

/// Semantic version tuple.
pub type CellVersion = (u32, u32, u32);

/// Base trait for all protocol implementations.
///
/// Every Gate, Router, Scorer, Composer, Substrate, and Policy is a Cell.
/// This trait provides identity, versioning, and metadata.
pub trait Cell: Send + Sync + 'static {
    /// Unique identifier for this cell.
    fn cell_id(&self) -> &str;
    /// Human-readable name.
    fn cell_name(&self) -> &str;
    /// Semantic version.
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    /// Protocol names this cell conforms to (e.g., ["verify", "score"]).
    fn protocols(&self) -> &[&str] { &[] }
    /// Estimated USD cost per invocation, if known.
    fn estimated_cost(&self) -> Option<f64> { None }
    /// Estimated wall-clock duration, if known.
    fn estimated_duration(&self) -> Option<Duration> { None }
}
```

Add `pub mod cell;` to lib.rs. Re-export `pub use cell::*;`.

**Verify**: `cargo check -p roko-core`

## Step E2: Rename Substrate → Store

1. In `traits.rs`: rename `pub trait Substrate` to `pub trait Store`
2. Also rename `pub trait ColdSubstrate` to `pub trait ColdStore`
3. Find all implementations:
   ```bash
   grep -rn 'impl Substrate\|impl ColdSubstrate\|: Substrate\|: ColdSubstrate\|dyn Substrate\|dyn ColdSubstrate' crates/ --include='*.rs' | grep -v target
   ```
4. Update each to use `Store` / `ColdStore`
5. For backwards compat, keep the old name as a trait alias in `compat.rs`:
   ```rust
   // In crates/roko-core/src/compat.rs (already exists):
   /// Deprecated: renamed to Store per unified spec.
   pub use crate::Store as Substrate;
   pub use crate::ColdStore as ColdSubstrate;
   ```
   Note: Rust doesn't support `#[deprecated]` on `pub use` re-exports of traits cleanly.
   Just add a doc comment. The old names work but new code should use the new names.
6. Update lib.rs re-exports

**Verify**: `cargo check --workspace`

## Step E3: Rename Scorer → Score

Same pattern. Find all uses:
```bash
grep -rn 'impl Scorer\|: Scorer\|dyn Scorer' crates/ --include='*.rs' | grep -v target
```

## Step E4: Rename Gate → Verify

**This is the largest rename** — `roko-gate` has 11+ implementations. Also note:
- `GateResult`, `GateVerdict`, `GatePayload` types reference "Gate" — decide whether to rename these too or just the trait
- The `roko-gate` crate name stays as-is (crate renames are a separate effort)
- String literals like `"gate"` in log messages don't need changing

Find all uses:
```bash
grep -rn 'impl Gate ' crates/ --include='*.rs' | grep -v target
```

Note: search for `impl Gate ` (with trailing space) to avoid matching `impl GateResult` etc.

## Step E5: Rename Router → Route

```bash
grep -rn 'impl Router\|: Router\|dyn Router' crates/ --include='*.rs' | grep -v target
```

## Step E6: Rename Composer → Compose

```bash
grep -rn 'impl Composer\|: Composer\|dyn Composer' crates/ --include='*.rs' | grep -v target
```

## Step E7: Rename Policy → React

```bash
grep -rn 'impl Policy\|: Policy\|dyn Policy' crates/ --include='*.rs' | grep -v target
```

Note: just rename the trait, do NOT change method signatures yet. The breaking change (Engram → Pulse input) happens later.

## Step E8: Define new protocol trait stubs

Add to `crates/roko-core/src/traits.rs` (or `cell.rs`):

```rust
/// Read-only observation. Produces Signals without side effects.
pub trait Observe: Cell {
    fn observe(&self) -> Vec<crate::Engram>;
}

/// Lifecycle-managed connection to an external system.
pub trait Connect: Cell {
    fn connect(&self) -> anyhow::Result<()>;
    fn health(&self) -> bool;
    fn disconnect(&self) -> anyhow::Result<()>;
}

/// Event-driven trigger that fires Graphs.
pub trait Trigger: Cell {
    fn arm(&self) -> anyhow::Result<()>;
    fn disarm(&self) -> anyhow::Result<()>;
}
```

No implementations yet — just trait definitions.

## Step E9: Add Cell implementations to existing types

For EACH existing trait implementation (there are ~100), add a corresponding `impl Cell`:

```rust
impl Cell for CompileGate {
    fn cell_id(&self) -> &str { "builtin:gate:compile" }
    fn cell_name(&self) -> &str { "CompileGate" }
    fn protocols(&self) -> &[&str] { &["verify"] }
}
```

This is mechanical but there are ~100 implementations. **Do the high-value ones only**:
- All gates in `crates/roko-gate/src/` (11+ structs: CompileGate, TestGate, ClippyGate, DiffGate, etc.)
- `MemorySubstrate` in `crates/roko-std/src/memory.rs`
- `FileSubstrate` in `crates/roko-fs/src/`
- `NoOpGate` in `crates/roko-std/src/noop.rs`
- `CascadeRouter` in `crates/roko-learn/src/cascade_router.rs`
- `RecordingGate` in `crates/roko-cli/src/gate_runner.rs`

Leave the remaining ~80 implementations for a follow-up pass. The important thing
is that the trait compiles and the pattern is established.

**Note**: If tests reference old trait names in assertion strings or log messages,
those don't need updating — they're just strings, not code.

## Step E10: Update compat.rs

`crates/roko-core/src/config/compat.rs` already exists. Add trait aliases there
(or create `crates/roko-core/src/compat.rs` for trait-level compat — check which
location is more appropriate by reading the existing compat.rs).

Add re-exports for all 6 renamed traits:
```rust
/// Backwards compatibility: use Store instead.
pub use crate::Store as Substrate;
/// Backwards compatibility: use ColdStore instead.
pub use crate::ColdStore as ColdSubstrate;
/// Backwards compatibility: use Score instead.
pub use crate::Score as Scorer;
/// Backwards compatibility: use Verify instead.
pub use crate::Verify as Gate;
/// Backwards compatibility: use Route instead.
pub use crate::Route as Router;
/// Backwards compatibility: use Compose instead.
pub use crate::Compose as Composer;
/// Backwards compatibility: use React instead.
pub use crate::React as Policy;
```

**Note**: `#[deprecated]` on `pub use` trait re-exports may cause issues with
`-D warnings` (clippy treats all uses as deprecated). If this causes too many
warnings across the workspace, just use doc comments instead of `#[deprecated]`.
The old names should work without triggering errors — the goal is gradual migration.

**Verify**:
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
# If deprecation warnings cause -D warnings to fail, add
# #[allow(deprecated)] to the compat module itself
```

---

# Final Verification Checklist

After all 5 tracks are complete:

```bash
# Build
cargo check --workspace

# Format
cargo +nightly fmt --all

# Lint
cargo clippy --workspace --no-deps -- -D warnings

# Test
cargo test --workspace

# Size checks
wc -l crates/roko-cli/src/main.rs                    # target: <1000 (was 12,690)
wc -l crates/roko-core/src/config/schema.rs           # target: <1000 (was 6,061)
wc -l crates/roko-learn/src/cascade_router.rs         # target: <2000 (was 5,197)
wc -l crates/roko-serve/src/routes/status.rs 2>/dev/null  # should not exist (now status/mod.rs)
```

All CLI commands should still work:
```bash
cargo run -p roko-cli -- --help
cargo run -p roko-cli -- plan --help
cargo run -p roko-cli -- status
cargo run -p roko-cli -- config show
```
