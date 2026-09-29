# Execution Path Design

*Plan: 02-backend-plan-execution — T01 design document*

---

## 1. Current call chain (broken)

The serve path for `POST /api/plans/:id/execute` never reaches the real graph engine.
Each hop is documented below with exact file and line.

### Hop 1 — Route handler

**File:** `crates/roko-serve/src/routes/plans.rs`, lines 249–317
**Function:** `execute_plan`

```rust
async fn execute_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError>
```

Line 288 spawns a `tokio::spawn` task and calls:

```rust
runtime.run_plan(&workdir, &plan_file).await
```

`runtime` is `Arc<dyn CliRuntime>` from `AppState`.

---

### Hop 2 — `CliRuntime` trait method

**File:** `crates/roko-serve/src/runtime.rs`, lines 296–312
**Method:**

```rust
async fn run_plan(
    &self,
    workdir: &std::path::Path,
    plan_target: &std::path::Path,
) -> anyhow::Result<PlanExecutionResult>;
```

The trait default implementation falls back to `run_once()` with a synthesized prompt.
The real CLI implementation overrides it.

---

### Hop 3 — `RokoCliRuntime::run_plan` — reaches the deprecated stub

**File:** `crates/roko-cli/src/serve_runtime.rs`, lines 233–258 (method entry), 537–580 (body of `run_plan_on_local_runtime`)

The method calls `tokio::task::spawn_blocking` which internally creates a single-threaded
Tokio runtime and executes:

```rust
// serve_runtime.rs ~line 571–572
#[allow(deprecated)]
let report = crate::runner::run(plans, &run_config, &state_hub, cancel).await?;
```

---

### Hop 4 — Deprecated stub

**File:** `crates/roko-cli/src/runner/mod.rs`, lines 58–84
**Function:**

```rust
#[deprecated(note = "Runner-v2 has been removed; use the Graph engine instead.")]
pub async fn run(
    _plans: Vec<Plan>,
    _config: &RunConfig,
    _state_hub: &crate::state_hub::StateHub,
    _cancel: tokio_util::sync::CancellationToken,
) -> anyhow::Result<RunReport> {
    anyhow::bail!(
        "the legacy Runner-v2 event loop has been removed. \
         Use the Graph engine (the default) instead."
    )
}
```

**This stub always returns an error.** Plan execution through the HTTP API is completely
broken today. Issue #342 tracks migrating the four remaining call sites.

---

### Hop 5 — Real graph entry point (binary-only today)

**File:** `crates/roko-cli/src/commands/plan.rs`, lines 2581–2598
**Function:** `cmd_plan_run_engine` (private, binary crate)

```rust
async fn cmd_plan_run_engine(
    plans_dir: &std::path::Path,    // (1)
    workdir: &std::path::Path,      // (2)
    cli: &Cli,                      // (3) ← private binary type
    resume_plan: Option<&std::path::Path>, // (4)
    fresh: bool,                    // (5)
    force_resume: bool,             // (6)
    max_retries: Option<u32>,       // (7)
    max_tasks: usize,               // (8)
    budget_override: Option<f64>,   // (9)
    no_budget: bool,                // (10)
    cli_model_override: Option<String>, // (11)
    dangerously_skip_permissions: bool, // (12)
    log_file: Option<&std::path::Path>, // (13)
    worktree_per_task: bool,        // (14)
    rich_topology: bool,            // (15)
    no_tui: bool,                   // (16)
) -> Result<i32>
```

---

## 2. The crate-boundary problem

`commands/plan.rs` is declared via `mod commands;` in `crates/roko-cli/src/main.rs`, so
it belongs to the **binary** crate. `serve_runtime.rs` is declared via
`pub mod serve_runtime;` in `crates/roko-cli/src/lib.rs`, so it belongs to the
**library** crate. The binary depends on the library; the library cannot call anything in
the binary. Making `cmd_plan_run_engine` public does not help — the type `Cli` it
receives is also in the binary crate.

---

## 3. The extraction design

The fix is the same inversion that `serve_runtime.rs` applied to the runtime trait one
layer up: extract the execution logic into the library, and have the binary supply a
pre-built parameter struct rather than its own private types.

### 3a. New parameter struct — `GraphPlanRunParams`

Lives in **`crates/roko-cli/src/graph_execution/mod.rs`** (or a new sub-module
`graph_execution/run_params.rs` re-exported from `mod.rs`). Every field uses types
already visible to the library crate.

| `cmd_plan_run_engine` parameter | Disposition in struct |
|---|---|
| `plans_dir: &Path` | Field `plans_dir: PathBuf` |
| `workdir: &Path` | Field `workdir: PathBuf` |
| `cli: &Cli` | **Dropped** — the struct exposes the individual settings `Cli` was consulted for (see fields below) |
| `resume_plan: Option<&Path>` | Field `resume_plan: Option<PathBuf>` |
| `fresh: bool` | Field `fresh: bool` |
| `force_resume: bool` | Field `force_resume: bool` |
| `max_retries: Option<u32>` | Field `max_retries: Option<u32>` |
| `max_tasks: usize` | Field `max_tasks: usize` |
| `budget_override: Option<f64>` | Field `budget_override: Option<f64>` |
| `no_budget: bool` | Field `no_budget: bool` |
| `cli_model_override: Option<String>` | Field `model_override: Option<String>` |
| `dangerously_skip_permissions: bool` | Field `dangerously_skip_permissions: bool` |
| `log_file: Option<&Path>` | Field `log_file: Option<PathBuf>` |
| `worktree_per_task: bool` | Field `worktree_per_task: bool` |
| `rich_topology: bool` | Field `rich_topology: bool` |
| `no_tui: bool` | Field `no_tui: bool` |

The `&Cli` parameter was the sole source of binary-private contamination. Anything the
body reads from `cli` (global flags, model/effort settings, provider config already
resolved at CLI parse time) is instead passed as explicit typed fields. The binary's
`commands/plan.rs` constructs the struct by transferring values it has already parsed
from `&Cli` before calling the library function.

Suggested definition skeleton:

```rust
/// Parameters for a graph-engine plan execution, owned entirely by the library crate.
///
/// Constructed by `commands/plan.rs` in the binary before calling
/// [`run_graph_plan`][`crate::graph_execution::run_graph_plan`].
#[derive(Debug, Clone)]
pub struct GraphPlanRunParams {
    pub plans_dir: std::path::PathBuf,
    pub workdir: std::path::PathBuf,
    pub resume_plan: Option<std::path::PathBuf>,
    pub fresh: bool,
    pub force_resume: bool,
    pub max_retries: Option<u32>,
    pub max_tasks: usize,
    pub budget_override: Option<f64>,
    pub no_budget: bool,
    pub model_override: Option<String>,
    pub dangerously_skip_permissions: bool,
    pub log_file: Option<std::path::PathBuf>,
    pub worktree_per_task: bool,
    pub rich_topology: bool,
    pub no_tui: bool,
    // Any additional fields read from &Cli that the body needs:
    pub config: crate::config::Config,
    pub repo_registry: crate::config::RepoRegistry,
    pub state_hub: crate::state_hub::SharedStateHub,
}
```

`config`, `repo_registry`, and `state_hub` are already library types — `serve_runtime.rs`
holds them as fields and can supply them directly.

### 3b. Library entry point signature

```rust
/// Execute a plan directory through the Graph engine.
///
/// This is the library-crate counterpart of `cmd_plan_run_engine` in the binary.
/// It carries no dependency on `Cli` or any other binary-private type.
pub async fn run_graph_plan(
    params: GraphPlanRunParams,
) -> anyhow::Result<PlanExecutionResult>
```

Return type `PlanExecutionResult` is already defined in `roko-serve/src/runtime.rs` and
is imported by `serve_runtime.rs` today.

Function lives at:
`crates/roko-cli/src/graph_execution/mod.rs` (re-exported)
or a dedicated sub-module `crates/roko-cli/src/graph_execution/run.rs` re-exported as
`pub use run::run_graph_plan;` from `mod.rs`.

### 3c. What moves to the library vs. what stays in the binary

| Concern | Destination |
|---|---|
| Graph engine bootstrap (build `WorkflowGraphController` / `GraphExecutor`) | Library (`run_graph_plan` body) |
| `WorktreeExecutionWorkspaceProvider` construction | Library (already in `graph_execution/workspaces.rs`) |
| `CliCompletionDeliveryService` construction | Library (already in `graph_execution/delivery.rs`) |
| `build_settler` for feedback | Library (already in `graph_execution/feedback.rs`) |
| `GraphIdentityMap` construction | Library (already in `graph_execution/identity_map.rs`) |
| `GraphRuntimeEventAdapter` | Library (already in `graph_execution/runtime_event_adapter.rs`) |
| `GraphExecutionControlAdapter` | Library (already in `graph_execution/control_adapter.rs`) |
| Argument parsing (`--plans-dir`, `--resume`, etc.) | Stays in binary (`commands/plan.rs`) |
| TUI launch (`roko dashboard` tab wiring) | Stays in binary (`commands/plan.rs`; `no_tui` flag plumbed through params) |
| Terminal progress printing (spinners, status lines) | Stays in binary (binary side can subscribe to a `watch::Receiver<GraphStatusSummary>` that the library exposes as part of its return value or via a progress callback) |
| `SharedStateHub` construction from config | Binary constructs and passes via params |

The body of the current `cmd_plan_run_engine` that is **not** TUI/progress-printing logic
moves verbatim into `run_graph_plan`. The binary's `cmd_plan_run_engine` becomes a thin
shim: build `GraphPlanRunParams` from `&Cli` + parsed flags, call
`crate::graph_execution::run_graph_plan(params).await`, map the result to an exit code.

---

## 4. Migration of `serve_runtime.rs`

After the library function exists, `RokoCliRuntime::run_plan` replaces its entire
`run_plan_on_local_runtime` block with:

```rust
async fn run_plan(
    &self,
    workdir: &Path,
    plan_target: &Path,
) -> anyhow::Result<PlanExecutionResult> {
    let params = GraphPlanRunParams {
        plans_dir: plan_target.parent().unwrap_or(plan_target).to_path_buf(),
        workdir: workdir.to_path_buf(),
        config: self.config.clone(),
        repo_registry: self.repo_registry.clone(),
        state_hub: self.state_hub.clone(),
        // safe serve-side defaults:
        no_tui: true,
        fresh: false,
        force_resume: false,
        resume_plan: None,
        max_retries: None,
        max_tasks: usize::MAX,
        budget_override: None,
        no_budget: false,
        model_override: None,
        dangerously_skip_permissions: false,
        log_file: None,
        worktree_per_task: false,
        rich_topology: false,
    };
    crate::graph_execution::run_graph_plan(params).await
}
```

No `#[allow(deprecated)]`, no `spawn_blocking`, no phantom Runner-v2 call.

---

## 5. Other deprecated call sites (#342)

The stub in `runner/mod.rs` has four call sites. Only `serve_runtime.rs` is in scope for
this plan. The remaining three (`prd`, `worker/cloud`, `do_cmd`) can be migrated in later
plans using the same `GraphPlanRunParams` struct.

---

## 6. File change summary for T02 onward

| File | Change |
|---|---|
| `crates/roko-cli/src/graph_execution/mod.rs` | Add `pub mod run;` + `pub use run::{GraphPlanRunParams, run_graph_plan};` |
| `crates/roko-cli/src/graph_execution/run.rs` | **New file** — struct + async fn |
| `crates/roko-cli/src/serve_runtime.rs` | Replace `run_plan_on_local_runtime` with the slim shim above |
| `crates/roko-cli/src/commands/plan.rs` | `cmd_plan_run_engine` becomes a thin wrapper; no logic removed, only re-routed |
| `crates/roko-cli/src/runner/mod.rs` | Unchanged in this task (stub remains; removal tracked by #342) |
