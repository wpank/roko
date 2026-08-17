# Roko Codebase Refactoring — Parallel Execution Prompt

> Copy this entire file into a fresh Claude Code session.

You are the **team lead** for refactoring a Rust codebase called Roko (agent toolkit, 18 crates, ~177K LOC). You have 5 independent refactoring tracks. **Tracks A–D touch completely different files and MUST run in parallel** using subagents in isolated worktrees. Track E runs after A–D merge.

## Your Job as Team Lead

1. Read `CLAUDE.md` at the project root
2. Create a team using `TeamCreate`
3. Spawn 4 parallel agents (one per track A–D) using the Agent tool with `isolation: "worktree"`
4. Monitor their progress, help if they get stuck
5. After all 4 complete, merge their branches into one
6. Run Track E yourself (it touches all crates, needs the merged state)
7. Final verification

## Critical Rules for ALL Agents

1. **Read before writing.** Always read a file before modifying it.
2. **Never push to main.**
3. **Verify after every change**: `cargo check --workspace`
4. **Before committing**: `cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace`
5. **Preserve all behavior exactly.** Zero functional changes.
6. **Rustc 1.91+ required.** `rustup update stable` if needed.

## File Ownership (NO OVERLAP between A–D)

| Track | Crate | Files Touched | Branch |
|-------|-------|---------------|--------|
| A | roko-cli | `src/main.rs`, `src/commands/` (new), `src/lib.rs` (add mod) | `refactor/main-rs` |
| B | roko-core | `src/config/schema.rs`, `src/config/*.rs` (new), `src/config/mod.rs` | `refactor/config` |
| C | roko-learn | `src/cascade_router.rs`, `src/cascade/` (new), `src/lib.rs` | `refactor/cascade` |
| D | roko-serve | `src/routes/status.rs` → `status/`, `src/routes/learning.rs` → `learning/`, `src/routes/mod.rs` | `refactor/routes` |
| E | ALL crates | `roko-core/src/traits.rs`, `roko-core/src/cell.rs` (new), ~20 files across crates | `refactor/cell-renames` |

## Execution Plan

```
Time 0:    Spawn agents A, B, C, D in parallel (worktree isolation)
           Each works independently — no coordination needed

Time ~45m: Agents complete, merge branches:
           git merge refactor/main-rs
           git merge refactor/config
           git merge refactor/cascade
           git merge refactor/routes

Time ~50m: Start Track E (Cell trait + renames) on merged code
Time ~90m: Track E complete. Final verification.
```

---

# AGENT PROMPTS (copy each into the Agent tool)

## Agent A: main.rs Decomposition

Spawn with: `Agent(subagent_type="general-purpose", isolation="worktree", prompt=<below>)`

```
You are refactoring crates/roko-cli/src/main.rs (12,690 lines, 43 cmd_* functions) into
focused subcommand modules. Zero behavior change — pure extraction.

Read CLAUDE.md first, then crates/roko-cli/src/main.rs and crates/roko-cli/src/lib.rs.

IMPORTANT: lib.rs re-exports `pub use orchestrate::{OrchestrationReport, PlanRunReport, PlanRunner}` — preserve this.
IMPORTANT: The `plan run` section (~lines 5770-5830) may be modified by a concurrent branch wp-runner-v2. If conflicts arise, keep the runner v2 version.
IMPORTANT: Clap #[derive(Parser)] structs and enum SubCommand stay in main.rs. Only move the cmd_* handler function bodies.

Steps:
1. Create crates/roko-cli/src/commands/ directory
2. Create commands/mod.rs with pub mod declarations for each submodule
3. Add `pub mod commands;` to lib.rs

4. Create commands/plan.rs — move cmd_plan() and all plan-related handlers (~line 5587: cmd_plan, cmd_plan_dry_run at ~6103, cmd_plan_validate at ~6275). Make functions pub(crate). In main.rs, replace inline logic with commands::plan::cmd_plan(...) calls.

5. Create commands/agent.rs — move cmd_agent() (~line 2583) and all agent handlers.

6. Create commands/prd.rs — move cmd_prd() (~line 8645) and all PRD handlers.

7. Create commands/research.rs — move cmd_research() (~line 6398) and all research handlers.

8. Create commands/config.rs — move all config-related cmd_* functions. Search for ConfigCmd handling.

9. Create commands/job.rs — move cmd_job() (~line 7225).

10. Create commands/learn.rs — move cmd_learn() (~line 9974).

11. Create commands/deploy.rs — move cmd_deploy() (~line 9735), cmd_deploy_fly() (~10455), cmd_deploy_docker() (~10464), cmd_deploy_railway() (~10503).

12. Create commands/server.rs — move cmd_daemon() (~line 2543) and serve handler.

13. Create commands/util.rs — move remaining: run, status, doctor, init, dashboard, replay, inject, index, new, explain, completions, chat.

14. Clean up main.rs — should be ~500-1000 lines: arg parsing + match dispatch.

Verify after each file:
  cargo check -p roko-cli

Final:
  wc -l crates/roko-cli/src/main.rs  # target: <1000
  cargo check --workspace && cargo test -p roko-cli
  cargo +nightly fmt --all
  cargo clippy --workspace --no-deps -- -D warnings

Commit: `refactor(main): extract command handlers to commands/ modules`
```

## Agent B: config/schema.rs Decomposition

Spawn with: `Agent(subagent_type="general-purpose", isolation="worktree", prompt=<below>)`

```
You are splitting crates/roko-core/src/config/schema.rs (6,061 lines, 30+ config structs)
into focused section files. Zero behavior change.

Read CLAUDE.md first, then crates/roko-core/src/config/schema.rs and config/mod.rs.

IMPORTANT: config/ already has: mod.rs, schema.rs, compat.rs, hot_reload.rs, presets.rs. Don't overwrite these.

The main config structs with approximate line numbers:
  RokoConfig (44), ToolsConfig (1346), ChainConfig (1400), RelayConfig (1442),
  ProviderConfig (1656), PrdConfig (1878), ProjectConfig (1892), AgentConfig (1935),
  DataLlmConfig (2138), GatesConfig (2297), PipelineConfig (2512), RoutingConfig (2677),
  BudgetConfig (2748), ConductorConfig (2787), LearningConfig (2899),
  DemurrageConfig (2999), AttentionConfig (3077), TuiConfig (3291), ServeConfig (3313),
  SchedulerConfig (3341)

Steps:
1. Create these new files in crates/roko-core/src/config/:
   - agent.rs: AgentConfig, DataLlmConfig + all nested types + impls + default fns
   - server.rs: ServeConfig, TuiConfig, SchedulerConfig + nested types + impls
   - budget.rs: BudgetConfig + impls
   - learning.rs: LearningConfig, RoutingConfig, ConductorConfig, AttentionConfig, DemurrageConfig, GatesConfig, PipelineConfig + impls
   - deploy.rs: ChainConfig, RelayConfig + impls
   - providers.rs: ProviderConfig, ToolProfileConfig, ToolsConfig + impls
   - project.rs: ProjectConfig, PrdConfig, GoalsConfig, EnergyConfig, ImmuneConfig, TemporalConfig, OneirographyConfig + impls

2. For each struct being moved, ALSO move:
   - All impl blocks
   - All fn default_*() functions used by #[serde(default = "...")]
   - All helper enums/structs only used by that config section
   - Keep pub visibility

3. Each new file needs appropriate use/imports. Use `use super::*;` or specific imports.

4. Update config/mod.rs to declare new submodules and re-export all public types:
   pub mod agent; pub mod server; pub mod budget; etc.
   pub use agent::*; pub use server::*; etc.

5. Keep in schema.rs ONLY:
   - RokoConfig struct (with fields importing from submodules)
   - from_toml(), to_toml(), to_toml_pretty(), is_stale()
   - effective_providers(), effective_models()
   - CURRENT_SCHEMA_VERSION, CURRENT_CONFIG_VERSION
   - ConfigChangeReport

6. Move #[cfg(test)] blocks to the submodule that owns the tested types.

Verify after each file move:
  cargo check -p roko-core

Final:
  wc -l crates/roko-core/src/config/schema.rs  # target: <1000
  cargo check --workspace && cargo test -p roko-core -- config
  cargo +nightly fmt --all
  cargo clippy --workspace --no-deps -- -D warnings

Commit: `refactor(config): split schema.rs into focused section modules`
```

## Agent C: cascade_router.rs Refactor

Spawn with: `Agent(subagent_type="general-purpose", isolation="worktree", prompt=<below>)`

```
You are splitting crates/roko-learn/src/cascade_router.rs (5,197 lines) into focused
submodules. Zero behavior change — pure extraction.

Read CLAUDE.md first, then crates/roko-learn/src/cascade_router.rs and lib.rs.
Also skim crates/roko-learn/src/model_router.rs (2,323 lines) for duplication.

The file has these sections:
  Lines 63-530: Type definitions (CascadeStage, CascadeModel, CascadeSelection, etc.)
  Lines 530-1080: Helper functions (slug_family, slug_matches, is_premium_model, etc.)
  Lines 1080-3200: CascadeRouter struct + 40 methods (select, observe, route, bias)
  Lines 3200-4000: LinUCB integration (arm management, UCB scores)
  Lines 4000-4500: Persistence (save, load, snapshot)
  Lines 4500-5000: Explanation generation (explain_route)
  Lines 5000-5197: Tests

Steps:
1. Create crates/roko-learn/src/cascade/ directory with mod.rs

2. Create cascade/types.rs — move all structs/enums defined before CascadeRouter:
   CascadeStage, StageTransition, CascadeModel, CascadeSelection,
   CascadeCandidateScore, CascadeRouteExplanation, CascadeRoutingExplanation,
   CascadeRoutingCandidate, RoutingBias, KnowledgeHint, KnowledgeRoutingAdvice,
   PerplexityObservation, GeminiContextTier, GeminiObservation, CascadeObservationStats

3. Create cascade/helpers.rs — move free functions:
   slug_family(), slug_matches(), is_premium_model(), and others before line 1080

4. Create cascade/arms.rs — move LinUCB-related code:
   LinUCB struct/impl, arm stats, UCB score computation, observation update methods

5. Create cascade/persistence.rs — move save/load/snapshot:
   CascadeRouter::save(), load(), snapshot serialization, JSONL helpers

6. Create cascade/explain.rs — move explanation generation:
   explain_route(), apply_knowledge_advice(), format_candidates()

7. Keep in cascade_router.rs:
   - CascadeRouter struct definition
   - Constructor (new, with_role_table, with_linucb, etc.)
   - Core routing: select(), select_for_frequency(), select_for_frequency_among()
   - Bias methods: apply_bias(), apply_cost_pressure()
   - Imports from cascade/ submodules

8. Update crates/roko-learn/src/lib.rs:
   - Add `pub mod cascade;`
   - Ensure all previously-public types are re-exported

9. Check model_router.rs: if it duplicates cascade_router logic, add a doc comment:
   `/// Legacy router. Prefer CascadeRouter for new code.`

Verify after each step:
  cargo check -p roko-learn

Final:
  wc -l crates/roko-learn/src/cascade_router.rs  # target: <2000
  cargo check --workspace && cargo test -p roko-learn
  cargo +nightly fmt --all
  cargo clippy --workspace --no-deps -- -D warnings

Commit: `refactor(learn): split cascade_router into focused submodules`
```

## Agent D: Serve Routes Consolidation

Spawn with: `Agent(subagent_type="general-purpose", isolation="worktree", prompt=<below>)`

```
You are splitting oversized route files in crates/roko-serve/src/routes/. Zero behavior change.

Read CLAUDE.md first, then:
  crates/roko-serve/src/routes/mod.rs — build_router() function
  crates/roko-serve/src/routes/status.rs — 2,490 lines (worst offender)

Steps:
1. Convert status.rs to a status/ directory:
   a. Create crates/roko-serve/src/routes/status/ directory
   b. Create status/mod.rs with pub fn routes() that merges sub-routers
   c. Create status/health.rs — move: health(), relay_health(), parity_handler(), retention_handler(), statehub_snapshot()
   d. Create status/metrics.rs — move: metrics(), metrics_summary(), success_rate(), engagement(), c_factor_metrics(), model_efficiency(), gate_rate(), experiments_metric(), feedback_latency(), velocity(), coverage(), prometheus_metrics()
   e. Create status/episodes.rs — move: episodes(), signals()
   f. Create status/gates.rs — move: gate_summary(), gates_history(), gate_history()
   g. Create status/dashboard.rs — move: dashboard(), session_status(), operation_status(), truth_map_handler()
   h. Delete the old status.rs file
   i. Update routes/mod.rs: change `mod status;` — it now points to status/mod.rs

2. Convert learning.rs to a learning/ directory:
   a. Create learning/ directory with mod.rs
   b. Create learning/router_state.rs — cascade router state endpoints
   c. Create learning/experiments.rs — A/B experiment endpoints
   d. Keep remaining (efficiency, c-factor, playbooks) in learning/mod.rs

3. Convert plans.rs to a plans/ directory:
   a. Create plans/ directory with mod.rs
   b. Create plans/execution.rs — plan execution endpoints
   c. Keep CRUD (list, show, create) in plans/mod.rs

4. Add missing endpoints (from tmp/dogfood/01-endpoint-audit.md):
   - GET /api/plans/:id — return plan state (add to plans/mod.rs)
   - GET /api/plans/:id/tasks — return task list (add to plans/mod.rs)
   - GET /api/learn/router — return cascade router snapshot (add to learning/router_state.rs)
   - GET /api/executor/state — return executor.json contents (add to status/dashboard.rs)

5. Update routes/mod.rs build_router() for the new module structure.

Verify after each step:
  cargo check -p roko-serve

Final:
  cargo check --workspace && cargo test -p roko-serve
  cargo +nightly fmt --all
  cargo clippy --workspace --no-deps -- -D warnings

Commit: `refactor(serve): split oversized route files into focused modules`
```

---

# AFTER AGENTS A–D COMPLETE

## Merge Phase (you, the team lead)

```bash
# Check all agent worktrees completed
# Merge each branch (resolve any conflicts — there should be none since files don't overlap)
git merge refactor/main-rs --no-edit
git merge refactor/config --no-edit
git merge refactor/cascade --no-edit
git merge refactor/routes --no-edit

# Verify merged state
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

---

## Track E: Cell Trait + Protocol Renames (run yourself, NOT as subagent)

This touches files across ALL crates. Run it on the merged code.

Read first:
- `crates/roko-core/src/traits.rs` — 7 current trait definitions at lines 36, 101, 165, 212, 240, 283, 337, 383
- `tmp/unified/02-CELL.md` — Cell trait spec

**The traits are NOT async. Don't add async_trait.**
**A Bus trait already exists at traits.rs:383. Don't duplicate it.**

### E1: Define Cell trait

Create `crates/roko-core/src/cell.rs`:

```rust
//! Universal computation unit. All protocol traits extend Cell.
use std::time::Duration;

pub type CellId = String;
pub type CellVersion = (u32, u32, u32);

/// Base trait for all protocol implementations.
pub trait Cell: Send + Sync + 'static {
    fn cell_id(&self) -> &str;
    fn cell_name(&self) -> &str;
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &[] }
    fn estimated_cost(&self) -> Option<f64> { None }
    fn estimated_duration(&self) -> Option<Duration> { None }
}
```

Add `pub mod cell;` to lib.rs, re-export `pub use cell::*;`.

### E2–E7: Rename traits (one at a time, verify after each)

For each rename:
1. Change the trait name in `traits.rs`
2. Search for ALL uses: `grep -rn 'impl OldName\|: OldName\|dyn OldName\|OldName for' crates/ --include='*.rs' | grep -v target`
3. Update each use
4. `cargo check --workspace` after each rename

| Old | New | Expected grep hits |
|-----|-----|-------------------|
| `Substrate` | `Store` | ~15 |
| `ColdSubstrate` | `ColdStore` | ~5 |
| `Scorer` | `Score` | ~10 |
| `Gate` (the trait only, not GateResult etc.) | `Verify` | ~25 |
| `Router` | `Route` | ~10 |
| `Composer` | `Compose` | ~10 |
| `Policy` | `React` | ~15 |

**Search carefully**: `impl Gate ` (with space) to avoid matching `impl GateResult`. The word `Gate` appears in many non-trait contexts (GateResult, GatePayload, gate_runner, etc.) — only rename the TRAIT and its implementors.

### E8: Define new protocol stubs

Add to traits.rs:
```rust
pub trait Observe: Cell {
    fn observe(&self) -> Vec<crate::Engram>;
}

pub trait Connect: Cell {
    fn connect(&self) -> anyhow::Result<()>;
    fn health(&self) -> bool;
    fn disconnect(&self) -> anyhow::Result<()>;
}

pub trait Trigger: Cell {
    fn arm(&self) -> anyhow::Result<()>;
    fn disarm(&self) -> anyhow::Result<()>;
}
```

### E9: Add Cell impls (high-value types only)

Add `impl Cell for X` to these (return static metadata):
- All gates in `crates/roko-gate/src/` (CompileGate, TestGate, ClippyGate, DiffGate, etc.)
- `MemorySubstrate` → `MemoryStore` in `crates/roko-std/src/memory.rs`
- `FileSubstrate` → `FileStore` in `crates/roko-fs/src/`
- `NoOpGate` in `crates/roko-std/src/noop.rs`
- `CascadeRouter` in `crates/roko-learn/src/cascade_router.rs`

Leave remaining ~80 implementations for a follow-up pass.

### E10: Backwards compat aliases

In `crates/roko-core/src/compat.rs` (already exists in config/, may need a top-level one):
```rust
pub use crate::Store as Substrate;
pub use crate::ColdStore as ColdSubstrate;
pub use crate::Score as Scorer;
pub use crate::Verify as Gate;
pub use crate::Route as Router;
pub use crate::Compose as Composer;
pub use crate::React as Policy;
```

### E-final: Verify

```bash
cargo check --workspace
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# Size checks
wc -l crates/roko-cli/src/main.rs                    # <1000 (was 12,690)
wc -l crates/roko-core/src/config/schema.rs           # <1000 (was 6,061)
wc -l crates/roko-learn/src/cascade_router.rs         # <2000 (was 5,197)
```
