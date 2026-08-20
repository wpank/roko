# 37 — Multi-Process Locking

**Priority**: P2 — workspace lock implementation is complete; the remaining work is auditing which entry points are missing locks and adding shared-lock semantics for read-only commands
**Size**: S (half day)
**Crates**: `crates/roko-cli/`
**Depends on**: None

---

## Background

Roko workspaces store mutable state in `.roko/`: the runner snapshot at `.roko/state/state-snapshot.json`, learning files at `.roko/learn/cascade-router.json` and `.roko/learn/gate-thresholds.json`, episode logs at `.roko/episodes.jsonl`, and PRD files at `.roko/prd/`. When multiple roko processes run concurrently against the same workspace, they can corrupt these files by interleaving writes.

The workspace lock infrastructure is **already implemented** in `crates/roko-cli/src/workspace_lock.rs`. The `acquire_workspace_lock(roko_dir: &Path) -> Result<WorkspaceLockGuard>` function creates `.roko/runtime/roko.lock`, acquires an exclusive `fs2` advisory lock, writes the current PID into the file, and returns an RAII guard that releases the lock and clears the PID on drop. The `fs2` crate is already a workspace dependency. The lock is automatically released on process exit, crash, or SIGINT because it is tied to the file descriptor lifetime.

The lock is already wired into several key entry points: `roko plan run` (via `commands/plan.rs` lines 206-207, 328, 946-947, 1161-1162), `roko serve` (via `commands/server.rs` line 11), `roko daemon` (via `daemon.rs` line 415), `roko do` (via `commands/do_cmd.rs` lines 33, 54, 95), and the top-level one-shot path in `main.rs` line 2836.

What is missing: the original spec described "shared locks" for read-only commands (`roko status`, `roko dashboard`, `roko learn all`). The current `acquire_workspace_lock` only supports exclusive locks — there is no `try_shared` variant. Additionally, some commands that write state have no lock at all (e.g., `roko prd draft` when writing PRD files, `roko research` when writing research artifacts).

## Current State

1. `crates/roko-cli/src/workspace_lock.rs` — `acquire_workspace_lock(roko_dir: &Path) -> Result<WorkspaceLockGuard>` uses `fs2::FileExt::try_lock_exclusive()`. The guard writes the caller's PID into the lock file and clears it on drop. Cross-process contention tests are present (lines 154-266) using subprocess helpers.

2. `crates/roko-cli/src/lib.rs` line 134 — `pub mod workspace_lock;` exports the module.

3. `commands/plan.rs` — `acquire_workspace_lock` called at lines 206-207, 328, 946-947, 1161-1162 for the plan run, graph run, and related paths.

4. `commands/server.rs` line 11 — `roko serve` acquires the workspace lock before binding the port.

5. `daemon.rs` line 415 — `roko daemon` acquires the workspace lock.

6. `commands/do_cmd.rs` lines 33, 54, 95 — `roko do` (three dispatch paths) each acquire the workspace lock.

7. `main.rs` line 2836 — one-shot prompt path acquires the workspace lock.

8. `commands/agent.rs` lines 51-57 — `roko agent` acquires the workspace lock conditionally (only for write-path subcommands).

9. `commands/prd.rs` lines 306-317 — `roko prd` acquires the workspace lock conditionally based on the subcommand.

10. `workspace_lock.rs` has no `try_shared` variant — the `fs2` crate supports shared locks via `try_lock_shared()`, but this is not implemented. The original spec item described shared locks for read-only commands; currently read-only commands either acquire no lock or acquire the exclusive lock.

## Implementation Plan

### Step 1: Audit missing lock sites

Run `grep -rn 'acquire_workspace_lock' crates/roko-cli/src/ --include='*.rs'` to enumerate current call sites. Then check the following entry points in `main.rs` / `commands/` to confirm which lack a lock:

- `roko status` — read-only, should acquire shared lock
- `roko learn all` — read-only, should acquire shared lock
- `roko dashboard` — read-only TUI, should acquire shared lock
- `roko knowledge query` — read-only, no lock needed (reads `.roko/learn/`)
- `roko research topic` — writes `.roko/research/`, should acquire exclusive lock

### Step 2: Add `try_shared` variant

In `crates/roko-cli/src/workspace_lock.rs`, add a second function `acquire_workspace_lock_shared(roko_dir: &Path) -> Result<WorkspaceLockGuard>` that calls `fs2::FileExt::try_lock_shared()` instead of `try_lock_exclusive()`. The `WorkspaceLockGuard` drop impl already calls `self.file.unlock()`, which works for both exclusive and shared locks.

```rust
pub fn acquire_workspace_lock_shared(roko_dir: &Path) -> Result<WorkspaceLockGuard> {
    let lock_dir = roko_dir.join("runtime");
    fs::create_dir_all(&lock_dir)?;
    let lock_path = lock_dir.join("roko.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    match file.try_lock_shared() {
        Ok(()) => Ok(WorkspaceLockGuard { file }),
        Err(_) => bail!(
            "Another roko process holds an exclusive lock on this workspace.\n  \
             hint: wait for it to finish or kill it."
        ),
    }
}
```

Note: shared locks do not write the PID into the file; only exclusive locks do.

### Step 3: Wire shared lock into read-only commands

In `commands/` handlers for `roko status`, `roko dashboard`, and `roko learn all`, acquire `acquire_workspace_lock_shared(&workdir.join(".roko"))?` before reading any `.roko/` files. Assign the result to `_lock` (RAII, held until command completes).

### Step 4: Wire exclusive lock into `roko research`

In the `research` command handler, add `acquire_workspace_lock(&workdir.join(".roko"))?` before writing research artifacts.

### Step 5: Verify `roko doctor` has no lock

`roko doctor` must always work, even if another process holds the lock. Confirm the doctor command handler does not call `acquire_workspace_lock`. If it does, remove it.

## Acceptance Criteria

1. `acquire_workspace_lock_shared` function exists in `workspace_lock.rs`.
2. `roko status` acquires a shared lock.
3. `roko dashboard` acquires a shared lock.
4. Running `roko status` while `roko plan run` holds the exclusive lock succeeds (shared and exclusive locks are compatible in `fs2`).
5. Running `roko plan run` in two terminals simultaneously: the second prints "Another roko process is running in this workspace (PID X)" and exits.
6. `roko doctor` completes without acquiring any lock, even if the lock file is held.
7. `cargo test -p roko-cli` passes with no regressions.

## Verification Checklist

- [ ] `grep -n 'acquire_workspace_lock' crates/roko-cli/src/workspace_lock.rs` shows both `acquire_workspace_lock` and `acquire_workspace_lock_shared`
- [ ] Start `roko plan run` in terminal A; run `roko status` in terminal B while A is running — terminal B succeeds
- [ ] Start `roko plan run` in terminal A; run `roko plan run` in terminal B — terminal B prints PID error and exits
- [ ] Kill terminal A; run `roko plan run` in terminal B — it acquires the lock and proceeds
- [ ] `roko doctor` works while `roko plan run` holds the lock
- [ ] `cargo test -p roko-cli 2>&1 | tail -5` shows all tests passed

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/workspace_lock.rs` | Add `acquire_workspace_lock_shared` function |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/mod.rs` | Wire shared lock into `roko status` handler |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/dashboard.rs` (or main.rs) | Wire shared lock into `roko dashboard` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs` | Wire shared lock into `roko learn all` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/research.rs` (or commands/research.rs) | Wire exclusive lock into `roko research topic` |
