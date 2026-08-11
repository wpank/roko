# E13-T03: Cell vs Block Naming Decision

> Decision document -- no Rust code changes are performed in this task.
> Created 2026-08-03 for E13-SPEC-DEBT-V2.

---

## 1. Background

The v2 specification restructure (version 2.0, 2026-04-24) renamed the execution primitive
from **Block** to **Cell**. The rationale recorded in `docs/v2/00-INDEX.md:258` is:

> Cell (not Module/Block) -- Composable, small, pluggable -- Eurorack module, Scratch block.

The rename landed in all Rust code and in the top-level v2 docs (`docs/v2/02-CELL.md`). However,
the deep-dive directory retains the old name (`docs/v2-depth/02-block/`, 16 files), and the two
Cell traits that exist in the codebase diverged during parallel development. No `Block` trait,
struct, or enum exists anywhere in `crates/` (verified: `rg 'pub trait Block|struct Block|enum Block' crates/` = 0 matches; the `BlockedReason`/`BlockedTask` types in `runner/task_dag.rs` and the `block_watcher`/`phase2` chain types are unrelated domain concepts).

---

## 2. The Two Cell Traits

### 2.1 roko-core Cell (`crates/roko-core/src/cell.rs:91`)

The **protocol supertrait** described by the v2 spec. All six protocol traits (Substrate, Scorer,
Gate, Router, Composer, Policy) require `Cell` as a supertrait.

```rust
pub trait Cell: Send + Sync + 'static {
    fn cell_id(&self) -> &str;
    fn cell_name(&self) -> &str;
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &[] }
    fn estimated_cost(&self) -> Option<f64> { None }
    fn estimated_duration(&self) -> Option<Duration> { None }
    fn input_schema(&self) -> Option<&TypeSchema> { None }     // v2 addition
    fn output_schema(&self) -> Option<&TypeSchema> { None }    // v2 addition
    async fn execute(&self, input: Vec<Engram>, ctx: &CellContext) -> Result<Vec<Engram>> {
        // Default: returns error
    }
}
```

**CellContext** fields: `bus` (`Arc<dyn BusErased>`), `store` (`Arc<dyn Substrate>`),
`cancel` (`CancellationToken`), `trace_id`, `run_id`, `budget_remaining`.

### 2.2 roko-graph Cell (`crates/roko-graph/src/cell.rs:74`)

The **graph-node execution** shape used by the `GraphEngine`.

```rust
pub trait Cell: Send + Sync + 'static {
    fn cell_id(&self) -> &str;
    fn cell_name(&self) -> &str;
    fn cell_version(&self) -> CellVersion { (0, 1, 0) }
    fn protocols(&self) -> &[&str] { &[] }
    fn estimated_cost(&self) -> Option<f64> { None }
    fn estimated_duration(&self) -> Option<Duration> { None }
    async fn execute(&self, input: Vec<Engram>, ctx: &CellContext) -> Result<Vec<Engram>>;
    // execute is required (no default impl)
}
```

**CellContext** fields: `trace_id`, `run_id`, `budget_remaining` only (no `bus`, `store`, or
`cancel`).

### 2.3 Incompatibilities

| Aspect | roko-core | roko-graph | Impact |
|--------|-----------|------------|--------|
| `execute` default | Has default (returns error) | Required (no default) | Structural: types implementing one trait do not automatically satisfy the other |
| `CellContext` | Has `bus`, `store`, `cancel` | Does not have them | Structural: roko-graph cells cannot access Bus/Store; roko-core cells expect them |
| `input_schema` / `output_schema` | Present (v2 additions) | Absent | Minor: roko-graph cells cannot declare typed I/O for edge validation |
| `TypeSchema` | Defined and used | Not available | Minor: downstream of `input_schema`/`output_schema` absence |
| Crate dependency | Kernel (layer 0) | Layer 2 (depends on roko-core) | roko-graph can depend on roko-core but not vice versa |

---

## 3. Canonical Decision

### Cell is canonical. Block is retired.

The term **Cell** is the canonical name for the universal execution primitive in roko. This
aligns with:

- The v2 spec naming table (`docs/v2/00-INDEX.md:258`): "Cell (not Module/Block)"
- All Rust code: both traits are named `Cell`, zero `Block` traits exist
- The v2 top-level docs (`02-CELL.md`): exclusively uses "Cell"
- The `CellId`, `CellVersion`, `CellContext`, `CellRegistry` types across both crates

**Block** survives only as:

1. The directory name `docs/v2-depth/02-block/` (16 files) -- a stale artifact of the pre-v2
   naming. The files inside already use "Cell" terminology in their prose.
2. Pseudo-code variable names in 3 depth-doc files (`protocol-algebra.md`, `verify-as-universal-oracle.md`)
   where `block_id` / `block_ref` / `block` appear in illustrative code snippets that predate
   the rename.

Neither of these is load-bearing. No consumer imports `Block` from anywhere.

### Aliases and deprecations

- **Block**: Deprecated. No Rust alias needed (it was never a Rust type). The `02-block/`
  directory name should be renamed to `02-cell/` in a follow-up task.
- **Module**: Not used. No action needed.
- **Node**: Used in roko-graph as `Node` (graph topology, `types.rs:56`), not as a synonym for
  Cell. A Node *contains* a cell_type string that resolves to a Cell via `CellRegistry`. This
  distinction is correct and should be preserved: Node is the graph-structural wrapper, Cell is
  the computation unit.

---

## 4. The Two-Trait Problem: Migration Shape

The two Cell traits must eventually converge into one. This section records the recommended
migration shape; **no code changes are made in this task**.

### 4.1 Recommended direction

**roko-core Cell is the canonical trait.** roko-graph should re-export and use it rather than
defining its own.

Rationale:
- roko-core is the kernel crate (layer 0); roko-graph depends on it (layer 2). The dependency
  direction is correct for roko-graph to import from roko-core.
- roko-core Cell already has the superset of functionality (TypeSchema, richer CellContext).
- All six protocol traits already use roko-core Cell as their supertrait.
- The v2 spec Cell trait (`docs/v2/02-CELL.md:20-58`) matches roko-core's shape more closely.

### 4.2 Migration steps (future task, not this one)

1. **Add missing CellContext fields to roko-graph**: Either import `roko_core::cell::CellContext`
   directly, or add `bus`/`store`/`cancel` to roko-graph's version and then unify.

2. **Resolve the `execute` default**: The roko-core version provides a default that returns an
   error. The roko-graph version makes it required. The roko-core approach is slightly more
   ergonomic (protocol-only cells like Scorer need not implement execute), but the roko-graph
   approach is safer (compile-time enforcement that execution cells actually implement execute).
   Recommendation: keep the default in roko-core (backwards compatible), but lint/doc that
   graph-node cells MUST override it.

3. **Replace `roko_graph::cell::Cell` with a re-export**: Change
   `crates/roko-graph/src/cell.rs` to `pub use roko_core::cell::{Cell, CellContext, CellVersion};`
   and update all graph-internal consumers. The `CellRegistry`, `GraphEngine`, `ShellCell`,
   `NoopCell`, `PassthroughCell`, `TaskExecutorCell`, and test types would all switch to the
   roko-core Cell trait.

4. **Reconcile the `NodeOutput`-based cells**: Three graph cells (`AgentCell`, `ComposeCell`,
   `GraduationCell`) use a separate `execute(&self, node_id, &[NodeOutput]) -> NodeOutput`
   signature that is incompatible with both Cell traits. These must be migrated to the
   `Cell::execute(Vec<Engram>, &CellContext) -> Result<Vec<Engram>>` signature, with NodeOutput
   data packed into Engrams.

### 4.3 Scope guard

The migration described in 4.2 is a separate task (likely in E01-execution-engine or a
dedicated E13 follow-up). It should be gated on:
- roko-graph GraphEngine being actively used (currently dry-run only)
- Runner v2 / roko-orchestrator relationship being settled (see `37-RUNNER-V2-AND-GRAPH.md`)

Premature unification risks breaking the working Runner v2 path for no user-visible benefit.

---

## 5. Documentation Follow-ups

| Item | Action | Priority |
|------|--------|----------|
| Rename `docs/v2-depth/02-block/` to `docs/v2-depth/02-cell/` | Directory rename + update INDEX.md cross-refs | Low (cosmetic) |
| Update `block_id`/`block_ref` pseudo-code in 3 depth docs | Replace with `cell_id`/`cell_ref` | Low (cosmetic) |
| Add Cell trait migration to GAPS.md | Record the two-trait convergence as a tracked gap | Medium |

---

## 6. Decision Summary

| Question | Answer |
|----------|--------|
| Canonical term for the execution primitive | **Cell** |
| Is Block deprecated? | Yes -- it was the pre-v2 name; no Rust types use it |
| Which Cell trait is canonical? | `roko_core::cell::Cell` (kernel layer, supertrait of all protocols) |
| Is `roko_graph::cell::Cell` wrong? | Not wrong, but redundant -- it should eventually re-export from roko-core |
| Does this task perform any Rust rename? | **No.** Decision-only. |
| Does this task merge the two traits? | **No.** Records the migration shape for a follow-up task. |
