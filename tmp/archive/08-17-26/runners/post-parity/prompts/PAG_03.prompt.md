# PAG_03: Delete duplicate code_context_for_task from prompt_helpers.rs

## Task
Remove the dead-code copy of `code_context_for_task()` from `prompt_helpers.rs` — the canonical version lives in `dispatch_helpers.rs`.

## Runner Context
Runner PAG (Cognitive Cleanup), batch 3 of 3. No dependencies.

## Problem
`code_context_for_task()` is defined identically in two files:
- `dispatch_helpers.rs:699-773` (75 lines, **active** — imported by orchestrate.rs)
- `prompt_helpers.rs:206-281` (76 lines, **dead code** — never imported or called)

The prompt_helpers copy is dead code. Any future changes to the function would only be made in dispatch_helpers (where it's actually used), making the copies drift silently.

## Current Code

**Canonical version** — `crates/roko-cli/src/dispatch_helpers.rs:699-773`:
```rust
pub(crate) fn code_context_for_task(
    workdir: &Path,
    task_description: &str,
    cached_index: Option<&roko_index::WorkspaceIndex>,
) -> Vec<String> {
    // ... 75 lines: index loading, keyword extraction, hybrid search, token budgeting
}
```

**Dead copy** — `crates/roko-cli/src/prompt_helpers.rs:206-281`:
```rust
pub(crate) fn code_context_for_task(
    workdir: &Path,
    task_description: &str,
    cached_index: Option<&roko_index::WorkspaceIndex>,
) -> Vec<String> {
    // ... 76 lines: identical logic, one extra comment at line 215
}
```

**Callers** (all use dispatch_helpers version):
- `orchestrate.rs:181` — `use crate::dispatch_helpers::code_context_for_task;`
- `orchestrate.rs:15044` — call site 1
- `orchestrate.rs:17196` — call site 2

**Zero callers** of the prompt_helpers version.

## Exact Changes

### Step 1: Delete the dead copy from prompt_helpers.rs

In `crates/roko-cli/src/prompt_helpers.rs`, delete lines 206-281 (the entire `code_context_for_task` function).

### Step 2: Check for any imports of the deleted function

```bash
grep -rn 'prompt_helpers::code_context_for_task' crates/ --include='*.rs' | grep -v target/
```

If any exist (none expected), change them to:
```rust
use crate::dispatch_helpers::code_context_for_task;
```

### Step 3: Remove any now-unused imports in prompt_helpers.rs

After deleting the function, check if any `use` statements at the top of prompt_helpers.rs are now unused (e.g., `roko_index::WorkspaceIndex`). Remove them.

## Write Scope
- `crates/roko-cli/src/prompt_helpers.rs` (delete lines 206-281 + cleanup unused imports)

## Read-Only Context
- `crates/roko-cli/src/dispatch_helpers.rs:699-773` (canonical version, don't touch)
- `crates/roko-cli/src/orchestrate.rs:181,15044,17196` (existing callers)

## Verify
```bash
cargo build -p roko-cli 2>&1 | head -30
cargo test -p roko-cli 2>&1 | tail -20
```

## Acceptance Criteria
- `code_context_for_task` exists only in `dispatch_helpers.rs`
- No dead code in `prompt_helpers.rs`
- All existing callers (orchestrate.rs) still compile
- `cargo build --workspace` passes
- No behavioral change

## Do NOT
- Modify the canonical version in dispatch_helpers.rs
- Change the function signature
- Move it to a different crate
- Add new callers
