# 14 — Plan Mutation Protocol

**Priority**: P2 — unlocks structured plan editing by agents and the HTTP dashboard; not blocking core self-hosting
**Size**: M (2–3 days)
**Crates**: `crates/roko-core/` (new types), `crates/roko-std/` (new tool definition), `crates/roko-cli/` (runner handler), `crates/roko-serve/` (HTTP endpoint)
**Depends on**: None

---

## Background

Roko executes plans represented as TOML files (`tasks.toml`). A plan is a directed acyclic graph (DAG) of tasks, where each task has an ID, a description, dependencies, and metadata. Plans are currently created by LLM agents during `roko prd plan <slug>` and are then executed as-is by the runner.

During execution, agents sometimes discover they need to adjust the plan: add a task for a newly found sub-problem, remove a task that turned out to be redundant, or insert a checkpoint after a risky step. Currently, agents have no structured way to express these changes. The only option is to emit raw TOML and hope the runtime reloads it, which is fragile: a syntactically valid TOML edit can still introduce a DAG cycle or a duplicate task ID, and there is no audit trail of what changed.

This item implements a typed `PlanMutation` protocol. The key insight is to separate _intent_ (what change the agent wants) from _representation_ (how it is stored). Mutations are validated before application, rejected mutations are reported back to the caller, and every applied batch is durably logged before the TOML file is written.

Note: neither `PlanMutation` nor a `plan_mutate` tool exists anywhere in the workspace today. The `plans.rs` serve route has a `POST /api/plans/:id/chat` endpoint (line 660) that uses natural language, but no structured mutation endpoint. Verified by grepping for `PlanMutation` and `plan_mutate` across the codebase — both return zero results.

## Current State

1. **No `PlanMutation` type exists** — search `grep -rn "PlanMutation" /Users/will/dev/nunchi/roko/roko/crates/` returns no results.
2. **No `plan_mutate` tool exists** — the builtin tools at `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/` do not include a `plan_mutate.rs` file.
3. **`dispatch_plan.rs`** at `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dispatch_plan.rs` exists and defines `DispatchPlan`, `DispatchAttempt`, etc. This is a sibling module location for `plan_mutation.rs`.
4. **`roko-core/src/lib.rs`** — lists all public modules at lines 62–200. A `plan_mutation` module would be added here.
5. **Serve routes** at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` — existing endpoints include `POST /api/plans/:id/execute` (line 201), `POST /api/plans/:id/chat` (line 660). The `POST /api/plans/:id/mutate` endpoint is missing.
6. **Builtin tool registration** — at `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/mod.rs`. New tools are registered here by adding a new file and a `pub mod` declaration.
7. **Runner tool-call handler** — in `event_loop.rs`. Tool calls from agents are handled in the `tokio::select!` branches; the implementation adds a new match arm for `"plan_mutate"`.
8. **Mutation log path** — `.roko/state/plan-mutations.jsonl` (new file, does not exist yet).

## Implementation Plan

### Step 1: Create `crates/roko-core/src/plan_mutation.rs`

Create `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/plan_mutation.rs`:

```rust
//! Typed plan mutation protocol.
//!
//! A [`PlanMutation`] represents a single structured change to a plan's task
//! graph. Changes are validated then applied via [`apply_mutations`]. Every
//! applied batch is appended to a durable JSONL log before the TOML file is
//! modified, enabling crash recovery and audit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Supporting types ────────────────────────────────────────────────────────

/// Patch fields for updating an existing task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TaskPatch {
    /// New description, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// New role, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// New complexity, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity: Option<String>,
    /// New domain, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// Patch fields for plan-level metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PlanMetaPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Minimal task specification for adding a new task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub complexity: String,
    #[serde(default)]
    pub domain: String,
    /// IDs of tasks this task depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

// ─── PlanMutation enum ───────────────────────────────────────────────────────

/// A single typed change to a plan's task graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanMutation {
    /// Add a new task. Optionally insert it after `after` in topological order.
    AddTask {
        task: TaskSpec,
        #[serde(skip_serializing_if = "Option::is_none")]
        after: Option<String>,
    },
    /// Remove an existing task and all its dependency edges.
    RemoveTask { id: String },
    /// Update fields on an existing task.
    UpdateTask { id: String, patch: TaskPatch },
    /// Add a dependency edge from `from` to `to` (meaning `to` depends on `from`).
    AddDependency { from: String, to: String },
    /// Remove a dependency edge.
    RemoveDependency { from: String, to: String },
    /// Reorder tasks (topological reorder hint; does not change dependencies).
    Reorder { task_ids: Vec<String> },
    /// Mark tasks as parallelizable (no ordering constraint between them).
    SetParallel { task_ids: Vec<String> },
    /// Insert a named checkpoint task after `after`.
    AddCheckpoint { after: String, name: String },
    /// Update plan-level metadata.
    UpdatePlanMeta { patch: PlanMetaPatch },
}

// ─── PlanMutationBatch ───────────────────────────────────────────────────────

/// A set of mutations applied atomically, with authorship metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanMutationBatch {
    pub plan_id: String,
    /// Who requested these mutations (agent ID, "dashboard", "cli", etc.).
    pub author: String,
    /// Session or run ID.
    #[serde(default)]
    pub session_id: String,
    pub mutations: Vec<PlanMutation>,
    pub requested_at: DateTime<Utc>,
}

// ─── MutationResult ──────────────────────────────────────────────────────────

/// Result of applying one batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationResult {
    pub plan_id: String,
    /// Mutations that were successfully applied.
    pub applied: Vec<PlanMutation>,
    /// Mutations that were rejected, with reasons.
    pub rejected: Vec<RejectedMutation>,
}

/// A mutation that failed validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedMutation {
    pub mutation: PlanMutation,
    pub reason: String,
}

impl MutationResult {
    /// Returns true if all mutations were applied with no rejections.
    pub fn all_applied(&self) -> bool {
        self.rejected.is_empty()
    }

    /// Returns true if every mutation was rejected.
    pub fn all_rejected(&self) -> bool {
        self.applied.is_empty() && !self.rejected.is_empty()
    }
}

// ─── Plan representation for validation ─────────────────────────────────────

/// Minimal in-memory representation of a plan for mutation validation.
/// The full `tasks.toml` structure from `roko-cli` is not imported here to
/// avoid circular dependencies. The caller reads the TOML and populates this.
#[derive(Debug, Clone, Default)]
pub struct PlanGraph {
    /// All task IDs in the plan.
    pub task_ids: std::collections::HashSet<String>,
    /// Dependency edges: (from, to) meaning `to` depends on `from`.
    pub edges: Vec<(String, String)>,
}

impl PlanGraph {
    /// Check whether adding a directed edge from `from` to `to` would create a cycle.
    ///
    /// Uses DFS from `to`, checking whether `from` is reachable (if so, adding
    /// the edge would close a cycle).
    pub fn would_cycle(&self, from: &str, to: &str) -> bool {
        // DFS from `to` through existing edges to see if we can reach `from`.
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![to.to_string()];
        while let Some(node) = stack.pop() {
            if node == from {
                return true;
            }
            if !visited.insert(node.clone()) {
                continue;
            }
            for (edge_from, edge_to) in &self.edges {
                if edge_from == &node {
                    stack.push(edge_to.clone());
                }
            }
        }
        false
    }
}

// ─── apply_mutations ─────────────────────────────────────────────────────────

/// Validate and apply a batch of mutations to a `PlanGraph`.
///
/// Mutations are processed in order. Each mutation is validated independently;
/// failures are collected in `rejected` and do not prevent subsequent mutations
/// in the same batch from being applied.
///
/// The caller is responsible for:
/// 1. Loading the `PlanGraph` from the TOML file before calling this.
/// 2. Persisting the updated graph back to TOML after this returns.
/// 3. Appending the batch and result to the mutation log.
pub fn apply_mutations(graph: &mut PlanGraph, batch: &PlanMutationBatch) -> MutationResult {
    let mut result = MutationResult {
        plan_id: batch.plan_id.clone(),
        applied: Vec::new(),
        rejected: Vec::new(),
    };

    for mutation in &batch.mutations {
        match validate_and_apply(graph, mutation) {
            Ok(()) => result.applied.push(mutation.clone()),
            Err(reason) => result.rejected.push(RejectedMutation {
                mutation: mutation.clone(),
                reason,
            }),
        }
    }

    result
}

fn validate_and_apply(graph: &mut PlanGraph, mutation: &PlanMutation) -> Result<(), String> {
    match mutation {
        PlanMutation::AddTask { task, .. } => {
            if graph.task_ids.contains(&task.id) {
                return Err(format!("task '{}' already exists", task.id));
            }
            graph.task_ids.insert(task.id.clone());
            for dep in &task.depends_on {
                if !graph.task_ids.contains(dep) {
                    return Err(format!(
                        "dependency '{}' for new task '{}' does not exist",
                        dep, task.id
                    ));
                }
                graph.edges.push((dep.clone(), task.id.clone()));
            }
            Ok(())
        }
        PlanMutation::RemoveTask { id } => {
            if !graph.task_ids.remove(id) {
                return Err(format!("task '{id}' does not exist"));
            }
            graph.edges.retain(|(from, to)| from != id && to != id);
            Ok(())
        }
        PlanMutation::UpdateTask { id, .. } => {
            if !graph.task_ids.contains(id) {
                return Err(format!("task '{id}' does not exist"));
            }
            Ok(()) // metadata update; no graph change to validate
        }
        PlanMutation::AddDependency { from, to } => {
            if !graph.task_ids.contains(from) {
                return Err(format!("source task '{from}' does not exist"));
            }
            if !graph.task_ids.contains(to) {
                return Err(format!("target task '{to}' does not exist"));
            }
            if graph.would_cycle(from, to) {
                return Err(format!(
                    "adding dependency {from}→{to} would introduce a cycle"
                ));
            }
            graph.edges.push((from.clone(), to.clone()));
            Ok(())
        }
        PlanMutation::RemoveDependency { from, to } => {
            let before = graph.edges.len();
            graph.edges.retain(|(f, t)| !(f == from && t == to));
            if graph.edges.len() == before {
                return Err(format!("dependency {from}→{to} does not exist"));
            }
            Ok(())
        }
        PlanMutation::Reorder { task_ids } => {
            for id in task_ids {
                if !graph.task_ids.contains(id) {
                    return Err(format!("task '{id}' in reorder list does not exist"));
                }
            }
            Ok(()) // reorder is a hint; no structural graph change
        }
        PlanMutation::SetParallel { task_ids } => {
            for id in task_ids {
                if !graph.task_ids.contains(id) {
                    return Err(format!("task '{id}' in parallel set does not exist"));
                }
            }
            Ok(())
        }
        PlanMutation::AddCheckpoint { after, name } => {
            if !graph.task_ids.contains(after) {
                return Err(format!("anchor task '{after}' does not exist"));
            }
            let checkpoint_id = format!("checkpoint-{name}");
            if graph.task_ids.contains(&checkpoint_id) {
                return Err(format!("checkpoint '{checkpoint_id}' already exists"));
            }
            graph.task_ids.insert(checkpoint_id.clone());
            graph.edges.push((after.clone(), checkpoint_id));
            Ok(())
        }
        PlanMutation::UpdatePlanMeta { .. } => Ok(()),
    }
}
```

### Step 2: Register the new module in `roko-core/src/lib.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs`, add after the `dispatch_plan` module line (around line 93):

```rust
pub mod plan_mutation;
```

Also add re-exports at the bottom of the `pub use` block:

```rust
pub use plan_mutation::{
    apply_mutations, MutationResult, PlanGraph, PlanMetaPatch, PlanMutation,
    PlanMutationBatch, RejectedMutation, TaskPatch, TaskSpec,
};
```

### Step 3: Create `crates/roko-std/src/tool/builtin/plan_mutate.rs`

Create `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/plan_mutate.rs`:

```rust
//! Built-in `plan_mutate` tool definition.
//!
//! Agents call this tool instead of writing TOML directly when they want to
//! modify the current plan. The runner handles the tool call, validates the
//! mutations, applies them to `tasks.toml`, and logs the batch.

use roko_core::tool::{ToolDef, ToolSchema};
use serde_json::json;

/// Returns the `plan_mutate` tool definition.
pub fn plan_mutate_tool() -> ToolDef {
    ToolDef {
        name: "plan_mutate".to_string(),
        description: "Apply structured mutations to the current plan's task graph. \
            Use this instead of editing tasks.toml directly. Mutations are validated \
            for DAG integrity (no cycles, no duplicate IDs) before being applied. \
            Each mutation batch is logged to .roko/state/plan-mutations.jsonl."
            .to_string(),
        schema: ToolSchema::Json(json!({
            "type": "object",
            "required": ["mutations"],
            "properties": {
                "mutations": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["kind"],
                        "properties": {
                            "kind": {
                                "type": "string",
                                "enum": [
                                    "add_task", "remove_task", "update_task",
                                    "add_dependency", "remove_dependency",
                                    "reorder", "set_parallel",
                                    "add_checkpoint", "update_plan_meta"
                                ]
                            }
                        }
                    }
                }
            }
        })),
        ..Default::default()
    }
}
```

Register the module in `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/mod.rs` — add:

```rust
pub mod plan_mutate;
pub use plan_mutate::plan_mutate_tool;
```

And include it in the set of default built-in tools wherever other tools like `read_file_tool()` are aggregated.

### Step 4: Handle `plan_mutate` tool calls in the runner

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, find where agent tool calls are dispatched (search for `"read_file"` or `ToolCall` handling). Add a match arm for `"plan_mutate"`:

```rust
"plan_mutate" => {
    // Parse the mutations from the tool call arguments.
    let mutations: Vec<roko_core::PlanMutation> =
        serde_json::from_value(tool_call.input.get("mutations")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])))
            .unwrap_or_default();

    let batch = roko_core::PlanMutationBatch {
        plan_id: current_plan_id.clone(),
        author: agent_id.clone(),
        session_id: session_id.clone(),
        mutations,
        requested_at: chrono::Utc::now(),
    };

    // Load current graph state from the tasks.toml (simplified: just IDs and edges).
    let mut graph = load_plan_graph(&plan_toml_path)?;

    // Log the batch BEFORE applying (so we can recover if the TOML write fails).
    append_mutation_log(&mutation_log_path, &batch)?;

    // Validate and apply.
    let result = roko_core::apply_mutations(&mut graph, &batch);

    // Persist the updated tasks.toml.
    if !result.applied.is_empty() {
        write_plan_graph(&plan_toml_path, &graph)?;
    }

    // Return the result to the agent as the tool output.
    let tool_result = serde_json::to_string(&result)?;
    // ... send tool_result back through the agent channel
}
```

You will need to implement `load_plan_graph`, `append_mutation_log`, and `write_plan_graph` helpers that translate between the existing `tasks.toml` TOML structure and the `PlanGraph` type. The existing TOML parsing is in `crates/roko-cli/src/task_parser.rs` — use that as the reference for field names.

**Mutation log append helper** (add as a free function near `record_gate_failure_reflection`):

```rust
fn append_mutation_log(
    path: &std::path::Path,
    batch: &roko_core::PlanMutationBatch,
) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(batch)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(file, "{line}")
}
```

### Step 5: Add `POST /api/plans/:id/mutate` to serve

In `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs`, add:

```rust
/// `POST /api/plans/:id/mutate` — apply typed mutations to a plan.
///
/// Returns 200 with `{applied: [...], rejected: [...]}` on success.
/// Returns 422 if the entire batch is invalid.
pub async fn mutate_plan(
    State(state): State<Arc<AppState>>,
    Path(plan_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mutations: Vec<roko_core::PlanMutation> = match serde_json::from_value(
        body.get("mutations").cloned().unwrap_or(serde_json::Value::Array(vec![]))
    ) {
        Ok(m) => m,
        Err(e) => {
            return (
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"error": format!("invalid mutations: {e}")})),
            ).into_response();
        }
    };

    // Load the plan graph from disk.
    let plan_dir = state.config.layout.plans_dir().join(&plan_id);
    let toml_path = plan_dir.join("tasks.toml");
    let mut graph = match load_plan_graph_from_path(&toml_path) {
        Ok(g) => g,
        Err(e) => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(json!({"error": format!("plan not found: {e}")})),
            ).into_response();
        }
    };

    let batch = roko_core::PlanMutationBatch {
        plan_id: plan_id.clone(),
        author: "dashboard".to_string(),
        session_id: String::new(),
        mutations,
        requested_at: chrono::Utc::now(),
    };

    // Log before applying.
    let log_path = state.config.layout.state_dir().join("plan-mutations.jsonl");
    let _ = append_mutation_log_serve(&log_path, &batch);

    let result = roko_core::apply_mutations(&mut graph, &batch);

    if result.all_rejected() {
        return (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::to_value(&result).unwrap_or_default()),
        ).into_response();
    }

    if !result.applied.is_empty() {
        let _ = write_plan_graph_to_path(&toml_path, &graph);
    }

    (axum::http::StatusCode::OK, Json(serde_json::to_value(&result).unwrap_or_default()))
        .into_response()
}
```

Register the route in `plans.rs`'s router function:
```rust
.route("/api/plans/:id/mutate", post(mutate_plan))
```

## Acceptance Criteria

1. `apply_mutations` rejects `AddTask` with a duplicate ID: the `MutationResult::rejected` contains that mutation with a reason containing "already exists"; other valid mutations in the same batch are in `applied`.
2. `apply_mutations` rejects `AddDependency` that would introduce a cycle: the rejection reason contains "cycle".
3. `PlanGraph::would_cycle` returns `true` for a graph with A→B→C when testing adding C→A, and `false` when testing adding D→A on a fresh graph.
4. Every applied batch is appended to `.roko/state/plan-mutations.jsonl` with `plan_id`, `author`, `requested_at`, and the full `mutations` list — verified by reading the file after a test call.
5. `POST /api/plans/:id/mutate` returns HTTP 200 with `{"applied": [...], "rejected": []}` for valid mutations.
6. `POST /api/plans/:id/mutate` returns HTTP 422 when all mutations are invalid.
7. `cargo test -p roko-core` passes with tests covering: add/remove roundtrip, cycle rejection, batch partial-apply, log append.
8. `cargo test --workspace` passes with no regressions.

## Verification Checklist

- [ ] Run `cargo build -p roko-core` — should compile with `plan_mutation` module
- [ ] Run `cargo test -p roko-core` — new unit tests should pass
- [ ] Write a test calling `apply_mutations` with a batch containing one valid `AddTask` and one duplicate `AddTask` — verify `applied.len() == 1` and `rejected.len() == 1`
- [ ] Write a test that adds A, B, C tasks, adds edges A→B and B→C, then tries to add C→A — verify the cycle rejection
- [ ] Run `cargo build -p roko-std` — should compile with `plan_mutate` tool
- [ ] Run `cargo build -p roko-serve` — should compile with new route registered
- [ ] Start `cargo run -p roko-cli -- serve` and run: `curl -X POST http://localhost:6677/api/plans/test/mutate -H 'Content-Type: application/json' -d '{"mutations": [{"kind": "add_task", "task": {"id": "t1", "description": "test task"}}]}'` — should return 200 or 404 (plan not found)
- [ ] Check `.roko/state/plan-mutations.jsonl` exists and contains a valid JSON line after the curl call
- [ ] Run `cargo test --workspace` to confirm no regressions

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/plan_mutation.rs` | Create new file with all types and `apply_mutations` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs` | Add `pub mod plan_mutation` and re-exports |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/plan_mutate.rs` | Create new file with `plan_mutate_tool()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/mod.rs` | Add `pub mod plan_mutate` and register the tool |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Add `"plan_mutate"` tool call handler; add `append_mutation_log`, `load_plan_graph`, `write_plan_graph` helpers |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` | Add `mutate_plan` handler and register `POST /api/plans/:id/mutate` route |
