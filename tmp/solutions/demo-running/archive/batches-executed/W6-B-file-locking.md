# W6-B: Multi-Process Workspace File Locking

**Priority**: P1 — prevents data corruption
**Effort**: 1 hour
**Files to modify**: 2-3 files
**Dependencies**: None

## Problem

`RokoLayout` defines `.roko/runtime/roko.lock` but never creates or checks it. Two simultaneous `roko plan run` commands corrupt shared state files.

## Fix

Add advisory file lock via `fs2::FileExt::lock_exclusive()` at startup of mutating commands.

### Step 1: Add dependency

**File**: `crates/roko-cli/Cargo.toml`
```toml
[dependencies]
fs2 = "0.4"
```

### Step 2: Create lock helper

**File**: `crates/roko-cli/src/workspace_lock.rs` (new file)

```rust
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs2::FileExt;

/// Acquires an exclusive advisory lock on the workspace.
/// Returns a guard that releases the lock on drop.
/// Fails immediately if another process holds the lock.
pub fn acquire_workspace_lock(roko_dir: &Path) -> Result<WorkspaceLockGuard> {
    let lock_dir = roko_dir.join("runtime");
    fs::create_dir_all(&lock_dir)
        .with_context(|| format!("create lock dir: {}", lock_dir.display()))?;

    let lock_path = lock_dir.join("roko.lock");
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&lock_path)
        .with_context(|| format!("open lock file: {}", lock_path.display()))?;

    match file.try_lock_exclusive() {
        Ok(()) => {
            // Write PID for diagnostics
            let mut f = &file;
            let _ = writeln!(f, "{}", std::process::id());
            Ok(WorkspaceLockGuard { file })
        }
        Err(_) => {
            // Read PID of holder for better error message
            let holder_pid = fs::read_to_string(&lock_path)
                .unwrap_or_default()
                .trim()
                .to_string();
            bail!(
                "Another roko process is running in this workspace (PID {}).\n  \
                 hint: wait for it to finish, or kill it with `kill {}`",
                holder_pid, holder_pid
            );
        }
    }
}

/// RAII guard that releases the file lock on drop.
pub struct WorkspaceLockGuard {
    file: File,
}

impl Drop for WorkspaceLockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
```

### Step 3: Wire into mutating commands

**File**: `crates/roko-cli/src/commands/plan.rs`

In the `PlanCmd::Run` handler, after resolving workdir:
```rust
let roko_dir = wd.join(".roko");
let _lock = crate::workspace_lock::acquire_workspace_lock(&roko_dir)?;
// ... rest of plan run
```

**File**: `crates/roko-cli/src/orchestrate.rs`

At the top of the orchestration entry point (if plan.rs delegates here):
```rust
let _lock = crate::workspace_lock::acquire_workspace_lock(&workdir.join(".roko"))?;
```

Also add to: `roko serve` (serves mutable state), `prd plan` (writes plans), `prd draft new` (writes drafts).

Do NOT add to read-only commands: `status`, `plan validate`, `plan list`, `config show`, etc.

### Step 4: Register module

**File**: `crates/roko-cli/src/lib.rs` or `main.rs`
```rust
mod workspace_lock;
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W6-B-file-locking.md and implement all changes described in it. Create crates/roko-cli/src/workspace_lock.rs with acquire_workspace_lock() + WorkspaceLockGuard. Add fs2 = "0.4" to Cargo.toml. Wire into plan run, prd plan, prd draft new, roko serve. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 6 batches together. Do not commit individually.

## Checklist

- [x] Add `fs2 = "0.4"` to roko-cli Cargo.toml
- [x] Create `workspace_lock.rs` with `acquire_workspace_lock()` + `WorkspaceLockGuard`
- [x] Wire into `plan run` handler
- [x] Wire into `prd plan` handler
- [x] Wire into `prd draft new` handler
- [x] Wire into `roko serve`
- [x] Do NOT add to read-only commands
- [ ] Verify: two simultaneous plan runs → second fails with clear message
- [ ] Verify: lock released after process exits
- [ ] Pre-commit checks pass
