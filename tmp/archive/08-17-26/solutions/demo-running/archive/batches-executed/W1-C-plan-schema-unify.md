# W1-C: Unify Plan Schema Parsing (validate vs run)

**Priority**: P0 — blocks demo pipeline
**Effort**: 1-2 hours
**Files to modify**: 2-3 files
**Dependencies**: None

## Problem

`plan validate` and `plan run` use different parsers with different required fields. A tasks.toml that passes `plan validate` can fail at `plan run` (and vice versa). Specifically:
- `[meta]` without `plan` field: validate passes, run fails
- `[[task]]` without `role` field: validate passes, run fails (actually validate DOES check role — the mismatch is in OTHER fields)
- Different error messages for the same problem

## Root Cause

Two independent TOML parsing paths:
1. **plan validate**: `crates/roko-cli/src/plan_validate.rs` (1517 lines) — uses its own `TaskEntry` struct with `Option<String>` for most fields, then checks required fields post-parse (lines 334-355: checks `id`, `title`, `role`)
2. **plan run**: `crates/roko-orchestrator/src/plan_discovery.rs` uses `PlanFrontmatter` (lines 29-70) which has ALL fields as `Option` or `Vec` with defaults. The actual task parsing happens through `crate::task_parser::TasksFile` which is a separate struct.

The plan_validate.rs is actually a submodule of prd.rs (line 17-18 in prd.rs: `#[path = "plan_validate.rs"] mod plan_validate;`).

## What Needs to Happen

Both `plan validate` and `plan run` must call the **same** parsing function. The canonical schema should live in one place.

### Step 1: Find the task parser used by `plan run`

```bash
grep -rn 'TasksFile\|TaskDef\|parse_tasks' crates/roko-cli/src/task_parser.rs | head -30
grep -rn 'TasksFile\|TaskDef' crates/roko-cli/src/ --include='*.rs' | head -30
```

Look at `crates/roko-cli/src/task_parser.rs` — this is where the runtime task struct is defined. The key fields that are REQUIRED at runtime are the ones we need to enforce in validation too.

### Step 2: Make plan_validate.rs use the runtime parser

In `plan_validate.rs`, the current approach parses into a lenient `TaskEntry` struct (all fields Optional), then manually checks required fields. Instead:

1. Try parsing with the runtime parser first
2. If it fails, report the deserialization error as a validation diagnostic
3. If it succeeds, run the additional lint checks (dependency cycles, model availability, etc.)

```rust
// In validate_plan_file():
// Instead of parsing into lenient TaskEntry, try the runtime parser first
match crate::task_parser::TasksFile::from_str(&content) {
    Ok(parsed) => {
        // Runtime parser accepts it — now run lint checks
        run_lint_checks(&parsed, &mut diagnostics, &plan_id, models);
    }
    Err(e) => {
        // Runtime parser rejects it — this means plan run would also fail
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule_id: "schema/parse".to_string(),
            plan_id: Some(plan_id.clone()),
            task_id: None,
            message: format!("plan would fail at runtime: {e}"),
        });
        // Still try lenient parse for additional diagnostics
    }
}
```

### Step 3: Ensure required fields match

The canonical required fields should be:
- `[meta]` section with `plan` field (string, non-empty)
- `[[task]]` entries with:
  - `id` (string, non-empty, unique within plan)
  - `title` (string, non-empty)
  - `role` (string, non-empty)
  - `prompt` (string, non-empty)

Everything else is optional with defaults.

### Step 4: Test both paths agree

Create test cases:

```toml
# test-minimal-valid.toml — should pass BOTH validate and run
[meta]
plan = "test"

[[task]]
id = "t1"
title = "Test"
role = "engineer"
prompt = "Do the thing"

# test-missing-plan.toml — should fail BOTH validate and run
[meta]
version = 1

[[task]]
id = "t1"
title = "Test"
role = "engineer"
prompt = "Do the thing"

# test-missing-role.toml — should fail BOTH validate and run
[meta]
plan = "test"

[[task]]
id = "t1"
title = "Test"
prompt = "Do the thing"
```

## Files to Modify

1. **`crates/roko-cli/src/plan_validate.rs`** — Add runtime parser check before/alongside lenient validation
2. **`crates/roko-cli/src/task_parser.rs`** — May need to make `TasksFile::from_str` public or add a `parse()` classmethod
3. **`crates/roko-cli/src/commands/plan.rs`** — If the validate command's error formatting needs updating

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W1-C-plan-schema-unify.md and implement all changes described in it. Read crates/roko-cli/src/task_parser.rs and crates/roko-cli/src/plan_validate.rs first to understand both parsers. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Just make the code changes and mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 1 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After compilation: `plan validate` and `plan run` should accept/reject the same inputs.

## Checklist

- [x] Read `crates/roko-cli/src/task_parser.rs` to understand runtime required fields
- [x] Read `crates/roko-cli/src/plan_validate.rs` to understand current validation
- [x] Add runtime parser check in `plan_validate.rs` (try runtime parse first)
- [x] Ensure required fields match between validate and run
- [ ] Test: valid plan passes both validate and run
- [ ] Test: invalid plan (missing role) fails both validate and run
- [ ] Test: invalid plan (missing meta.plan) fails both validate and run
- [ ] Pre-commit checks pass
