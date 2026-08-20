# 75 — Graph Example Schema Drift (3 Broken Examples)

**Priority**: P0 — blocks new users; 3 of 8 graph examples fail to parse
**Size**: S (half day)
**Crates**: `crates/roko-graph/src/loader.rs`, `crates/roko-graph/src/` (test addition)
**Depends on**: None

---

## Background

The `examples/graphs/` directory contains 8 TOML files that serve as the primary reference
for users writing roko graph definitions. Three of these files currently fail to parse
because they were written against the graph engine's internal runtime types rather than the
TOML loader's deserialization types. A new user who copies any of these examples will receive
a cryptic parse error and no working graph.

The five working examples (`single-gate.toml`, `linear-gates.toml`, `score-compose.toml`,
`cognitive-loop.toml`, `observed-cost.toml`) all correctly use `[graph]` tables and
`RawEdgeCondition` variants. The three broken examples were written against `condition::Condition`
(the runtime type used after loading) rather than `RawEdgeCondition` (the TOML-facing type
that the serde parser actually reads).

The fix is purely in the example TOML files (no Rust code changes required for the basic
fix) plus an optional loader enhancement for the `when` condition and a CI regression test.

## Current State

1. **TOML loader type:** `RawEdgeCondition` is defined at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/loader.rs` lines 67-78.
   It is a `#[serde(tag = "type")]` enum with exactly four variants:
   - `Success` (alias `"success"`)
   - `Failure` (alias `"failure"`)
   - `Always` (alias `"always"`)
   - `OutputEquals { key: String, value: String }` (alias `"output_equals"`)

   The variants are case-insensitive in the lowercase forms (via serde alias). The
   `Success` variant is serialized as `"Success"` but also accepted as `"success"`. The
   example files must use one of these four variant names in `type = "..."`.

2. **`parallel-gates.toml`** at
   `/Users/will/dev/nunchi/roko/roko/examples/graphs/parallel-gates.toml`:
   - Uses bare top-level `name = "parallel-gates"` and `description = "..."` (lines 8-9)
     instead of a `[graph]` table. The TOML loader's `RawGraphFile` struct (line 14-23 of
     `loader.rs`) requires a `graph:` field at the top level.
   - All edges in this file already use `type = "always"` (correct). The `cell_type = "gate"`
     values at lines 33 and 44 use a specific gate type key per node config (`gate_type =
     "compile"`, `gate_type = "clippy"`, `gate_type = "test"`), not a bare `"gate"` cell
     type, so no cell_type issue once the `[graph]` table is added.
   - **Error:** `missing field 'graph'`

3. **`task-execution.toml`** at
   `/Users/will/dev/nunchi/roko/roko/examples/graphs/task-execution.toml`:
   - Uses `type = "on_success"` (lines 124, 132, 138) and `type = "on_failure"` (line 147)
     in edge condition blocks.
   - `"on_success"` and `"on_failure"` are not variants of `RawEdgeCondition`. The TOML
     loader will fail with `unknown variant 'on_success'`.
   - The correct variants are `type = "success"` and `type = "failure"`.
   - All other parts of the file are structurally correct.

4. **`conditional-branch.toml`** at
   `/Users/will/dev/nunchi/roko/roko/examples/graphs/conditional-branch.toml`:
   - Line 85 uses `type = "on_success"` (one instance), which has the same issue as
     `task-execution.toml`.
   - Lines 93-100, 107-112, 119-124 use `type = "when"` with `field`, `op` (Gte/Lt), and
     `[edges.condition.value]` blocks. The `"when"` variant is not in `RawEdgeCondition` at
     all; it exists in the internal `condition::Condition::When { field, op, value }` runtime
     type but is not reachable from TOML.
   - **Error:** `unknown variant 'on_success'` (first condition hit before the `when` issues)

5. **Working examples use `[graph]` tables:** For example, `single-gate.toml` (lines 9-11)
   has `[graph]` as a proper TOML table with `name` and `description` inside it. The broken
   `parallel-gates.toml` puts `name` and `description` at the top level without a table.

6. **No regression test exists** that validates all example graphs parse successfully. Schema
   drift can silently reoccur after any loader change.

## Implementation Plan

### Step 1: Fix `parallel-gates.toml`

In `/Users/will/dev/nunchi/roko/roko/examples/graphs/parallel-gates.toml`, wrap the bare
top-level metadata in a `[graph]` table. The current top of the file is:

```toml
name = "parallel-gates"
description = "Run multiple validation gates in parallel after prompt assembly"
```

Change to:

```toml
[graph]
name = "parallel-gates"
description = "Run multiple validation gates in parallel after prompt assembly"
```

Everything else in the file is correct (edges use `type = "always"`, node configs use
specific `gate_type` keys).

### Step 2: Fix `task-execution.toml`

In `/Users/will/dev/nunchi/roko/roko/examples/graphs/task-execution.toml`, replace all
occurrences of `type = "on_success"` with `type = "success"` and all occurrences of
`type = "on_failure"` with `type = "failure"`.

Specific lines to change (as of the current file):
- Line 124: `type = "on_success"` -> `type = "success"`
- Line 132: `type = "on_success"` -> `type = "success"`
- Line 138: `type = "on_success"` -> `type = "success"`
- Line 147: `type = "on_failure"` -> `type = "failure"`

### Step 3: Fix `conditional-branch.toml`

**Sub-step 3a:** Fix the `on_success` instance at line 85:
```toml
type = "on_success"
```
Change to:
```toml
type = "success"
```

**Sub-step 3b:** Fix the `when` condition edges (lines 88-124). Two options:

**Option A (simpler, recommended):** Replace the three `type = "when"` conditional edges
with `type = "always"` edges and add a comment explaining that the conditional routing is
illustrative and requires either the `When` loader extension or external orchestration logic.
This is the minimal fix to make the example parse and run.

**Option B (more powerful):** Add `When` to `RawEdgeCondition` in
`/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/loader.rs` and wire it to
`condition::Condition::When`. This requires:

1. Add the variant to `RawEdgeCondition` (around line 67):
```rust
#[serde(alias = "when")]
When {
    field: String,
    op: String,   // "Eq", "Neq", "Gt", "Lt", "Gte", "Lte"
    value: toml::Value,
},
```

2. Add the `From<RawEdgeCondition>` arm (around line 81):
```rust
RawEdgeCondition::When { field, op, value } => {
    let compare_op = op.parse::<CompareOp>()
        .unwrap_or(CompareOp::Eq);
    let cond_value = toml_value_to_condition_value(value);
    Self::When { field, op: compare_op, value: cond_value }
}
```

3. Add a `toml_value_to_condition_value` conversion helper.

4. Import `crate::condition::{CompareOp, ConditionValue}` in `loader.rs`.

Option A unblocks new users immediately without loader changes. Option B adds richer
conditional routing capability but touches production Rust code. **Choose Option A for the
initial fix**, and file Option B as a separate improvement task.

### Step 4: Add a CI regression test

In `crates/roko-graph/` (or a new `tests/` file), add a test that validates all
`examples/graphs/*.toml` files parse successfully. This prevents future schema drift.

Add a file `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/tests/example_graphs.rs`:

```rust
#[test]
fn all_example_graphs_parse_successfully() {
    let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()  // crates/
        .unwrap()
        .parent()  // workspace root
        .unwrap()
        .join("examples")
        .join("graphs");

    let mut failed = Vec::new();
    for entry in std::fs::read_dir(&examples_dir)
        .expect("examples/graphs/ must exist")
    {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            if let Err(e) = roko_graph::loader::load_from_str(&content) {
                failed.push(format!("{}: {}", path.display(), e));
            }
        }
    }

    if !failed.is_empty() {
        panic!(
            "{} example graph(s) failed to parse:\n{}",
            failed.len(),
            failed.join("\n")
        );
    }
}
```

Note: `roko_graph::loader::load_from_str` must be `pub` for this to work. Verify it is
already public (it is, at line 96 of `loader.rs`).

## Acceptance Criteria

1. `roko graph validate examples/graphs/parallel-gates.toml` exits 0 with no errors.
2. `roko graph validate examples/graphs/task-execution.toml` exits 0 with no errors.
3. `roko graph validate examples/graphs/conditional-branch.toml` exits 0 with no errors.
4. All 8 example graphs pass validation: `for f in examples/graphs/*.toml; do roko graph validate "$f"; done`
5. The new regression test in `crates/roko-graph/tests/example_graphs.rs` passes:
   `cargo test -p roko-graph all_example_graphs_parse_successfully`
6. `cargo test --workspace` passes.
7. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] Run `roko graph validate examples/graphs/parallel-gates.toml` and confirm 0 errors
- [ ] Run `roko graph validate examples/graphs/task-execution.toml` and confirm 0 errors
- [ ] Run `roko graph validate examples/graphs/conditional-branch.toml` and confirm 0 errors
- [ ] Run `cargo test -p roko-graph all_example_graphs_parse_successfully` and confirm pass
- [ ] Run all 8 validations in a loop and confirm none fail
- [ ] Confirm `[graph]` table is present in all example `.toml` files that need it

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/examples/graphs/parallel-gates.toml` | Wrap `name` and `description` in a `[graph]` table (2-line change) |
| `/Users/will/dev/nunchi/roko/roko/examples/graphs/task-execution.toml` | Replace `on_success` with `success` and `on_failure` with `failure` in all edge condition `type` fields (4 occurrences) |
| `/Users/will/dev/nunchi/roko/roko/examples/graphs/conditional-branch.toml` | Replace `on_success` with `success`; replace `when` condition edges with `always` + explanatory comment (Option A) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/tests/example_graphs.rs` (new) | Regression test that validates all 8 example TOML files parse via `load_from_str` |
