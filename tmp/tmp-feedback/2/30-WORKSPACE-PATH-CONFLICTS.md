# Workspace Path Conflicts: plans/, sessions/, orphaned tmp files

## Problem

Multiple inconsistencies in where roko looks for files:

1. `plans/` directory location conflict between `plan.rs` and `main.rs`
2. `sessions/` directory mismatch
3. 11 orphaned `.tmp` files in `.roko/learn/`

## Root Cause

### A. Plans directory conflict

**File:** `crates/roko-cli/src/commands/plan.rs`

```rust
// plan.rs prefers top-level:
let plans_dir = workdir.join("plans");
```

**File:** `crates/roko-cli/src/main.rs`

```rust
// main.rs sometimes prefers .roko:
let plans_dir = workdir.join(".roko/plans");
```

Both locations exist in the repository:
```
./plans/                  ← 11 plan directories (used by plan.rs)
./.roko/plans/            ← sometimes created by other code paths
```

When `roko plan list` looks in `./plans/` but `roko prd plan` writes to `.roko/plans/`,
the generated plan is invisible to `plan list` and `plan run`.

### B. Sessions directory mismatch

Similar issue with session snapshots:
```
./.roko/state/            ← where orchestrate.rs writes executor.json
./.roko/sessions/         ← where some code paths look for session data
```

### C. Orphaned tmp files

**Directory:** `.roko/learn/`

```
.roko/learn/cascade-router.json.tmp
.roko/learn/cascade-router.json.tmp.2
.roko/learn/experiments.json.tmp
.roko/learn/gate-thresholds.json.tmp
... (11 total)
```

These are created by atomic write operations that failed mid-write (crash or interrupt).
The pattern is: write to `.tmp`, rename to final path. If the rename fails, the `.tmp`
file is orphaned.

## Fix

### Fix 1: Standardize plans directory (~10 min)

**Decision needed:** Use `./plans/` (top-level) as the canonical location.

**Files to update:**
- `crates/roko-cli/src/commands/plan.rs` — already uses `./plans/`
- `crates/roko-cli/src/commands/prd.rs` — check where `prd plan` writes
- `crates/roko-cli/src/main.rs` — remove `.roko/plans/` references
- `crates/roko-acp/src/bridge_events.rs` — check slash command plan paths

### Fix 2: Standardize sessions directory (~5 min)

Use `.roko/state/` as canonical. Remove any `.roko/sessions/` references.

### Fix 3: Clean up orphaned tmp files (~5 min)

Add cleanup to `roko doctor`:
```rust
// In doctor.rs:
fn check_orphaned_tmp_files(workdir: &Path) -> Vec<Diagnostic> {
    let learn_dir = workdir.join(".roko/learn");
    let mut diagnostics = vec![];
    for entry in fs::read_dir(&learn_dir).into_iter().flatten() {
        let path = entry.path();
        if path.extension() == Some("tmp".as_ref()) {
            diagnostics.push(Diagnostic::warning(
                format!("Orphaned tmp file: {}", path.display()),
                "Run `roko doctor --fix` to clean up",
            ));
        }
    }
    diagnostics
}
```

### Fix 4: Add tmp file cleanup to atomic write (~5 min)

**File:** `crates/roko-learn/src/persistence.rs` (or wherever atomic writes happen)

Add a cleanup step at startup:
```rust
pub fn cleanup_orphaned_tmp(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension() == Some("tmp".as_ref()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/commands/prd.rs` | Standardize plan output to `./plans/` |
| `crates/roko-cli/src/main.rs` | Remove `.roko/plans/` references |
| `crates/roko-cli/src/commands/doctor.rs` | Add orphaned tmp file check |

## Priority

**P2** — Path confusion can cause plans to be "lost" (written to one path, searched in
another). The fix is straightforward standardization.
