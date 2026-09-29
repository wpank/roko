# Worktree Isolation

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 10.
> Preserves and updates content from v1 `01-orchestration/07-worktree-isolation.md`.

---

## Overview

Git worktrees provide per-plan filesystem isolation. Each active plan gets
its own worktree -- a separate working directory on its own branch, sharing
the same `.git` repository. This allows multiple agents to work on
different plans simultaneously without conflicting on the filesystem.

The `WorktreeManager` handles the full lifecycle: creation, branch naming,
health monitoring, idle reclamation, stale lock cleanup, and budget
enforcement.

**Source:** `crates/roko-cli/src/runner/worktree.rs`

---

## Why Worktrees

Without isolation, concurrent agents conflict on files, builds, and test
results:

1. **File conflicts**: two agents editing the same file overwrite each
   other's changes.
2. **Build conflicts**: agent A's half-finished edit causes agent B's
   compilation to fail.
3. **Test contamination**: test results reflect a mix of changes from
   different plans.
4. **Merge hell**: combining simultaneous changes requires complex merge
   resolution.

Git worktrees solve this at the filesystem level. Each worktree:

- Has its own working directory (separate files)
- Has its own branch (separate commit history)
- Shares the same `.git` repository (efficient -- no full clone needed)
- Can run `cargo build`, `cargo test` independently

Agents working in different worktrees cannot conflict on files. They
operate on isolated branches and only interact at merge time, where
conflicts are handled explicitly by the `MergeQueue`.

---

## Branch Naming Convention

```
roko/plan/<plan_id>
```

For example:

```
roko/plan/01-workspace-scaffold
roko/plan/02-core-traits
roko/plan/08a-chain-layer
```

This convention:

1. **Namespaces** branches under `roko/plan/` to avoid conflicts with
   user-created branches.
2. **Includes the plan ID** for traceability -- you can identify which
   plan produced which branch by inspection.
3. **Is deterministic** -- the same plan always gets the same branch name,
   enabling idempotent `ensure_for_plan()` on resume.

---

## WorktreeConfig

```rust
pub struct WorktreeConfig {
    pub repo_root: PathBuf,      // main repository root
    pub base_branch: String,     // branch to create from (e.g. "main")
    pub worktrees_root: PathBuf, // where worktrees live
    pub max_live: usize,         // maximum concurrent worktrees
    pub idle_ttl: Duration,      // idle time before reclamation
}
```

### Defaults

| Parameter | Default | Source |
|---|---|---|
| `repo_root` | Working directory | CLI `--workdir` |
| `base_branch` | `"main"` | Config |
| `worktrees_root` | `.roko/worktrees/` | Convention |
| `max_live` | 8 | `config.conductor.max_agents` |
| `idle_ttl` | 30 minutes | `DEFAULT_WORKTREE_IDLE_TTL_SECS` |

---

## WorktreeHandle

Each active worktree is tracked by a handle:

```rust
pub struct WorktreeHandle {
    pub id: String,          // worktree identifier
    pub path: PathBuf,       // filesystem path
    pub branch: String,      // git branch name
    pub created_at_ms: u64,  // creation timestamp
    pub last_active_ms: u64, // last activity timestamp
}
```

The `last_active_ms` field is updated whenever an agent operates in the
worktree. It drives the idle reclamation system.

---

## Lifecycle Operations

### create(id)

Creates a new worktree with a fresh branch:

1. Check if `max_live` would be exceeded; if so, try `reclaim_idle()`.
2. If still over budget, return `WorktreeError::BudgetExceeded`.
3. Create branch: `git branch roko/plan/<id> <base_branch>`.
4. Create worktree: `git worktree add <worktrees_root>/<id> roko/plan/<id>`.
5. Record the `WorktreeHandle` in the internal HashMap.

### ensure_for_plan(id)

Creates a worktree if absent, returns existing if present. This is the
preferred method for the runtime -- it is idempotent and handles resume
scenarios where a worktree may already exist from a previous run.

### remove(id)

Removes a worktree and optionally deletes its branch:

1. Run `git worktree remove <path>` (with `--force` if needed).
2. Optionally run `git branch -D roko/plan/<id>`.
3. Remove the handle from the tracking map.

Note: per user preference, worktrees and branches are preserved for
inspection and history rather than automatically deleted.

### check_health(id)

Returns the health status of a worktree:

| Health | Meaning | Action |
|---|---|---|
| `Ok` | Worktree exists and is functional | None needed |
| `Missing` | Directory does not exist | Recreate or remove handle |
| `StaleLock` | A `*.lock` file exists (leftover from crashed git) | `clear_stale_locks()` |
| `Detached` | HEAD is detached (not on the expected branch) | Investigate manually |

### reclaim_idle()

Removes worktrees that have been idle longer than `idle_ttl`. Returns the
IDs of reclaimed worktrees. The default 30-minute TTL gives agents time
to finish before reclamation.

### clear_stale_locks()

Removes leftover `*.lock` files from worktrees. Lock files are created by
git during merge and rebase operations. If interrupted (crash, kill), the
lock file persists and blocks future git operations. This method detects
and removes stale locks, unblocking the worktree.

### prune()

Runs `git worktree prune` to clean up stale worktree metadata from
`.git/worktrees/` that points to non-existent directories.

---

## Budget Enforcement

The `max_live` parameter enforces a hard limit on concurrent worktrees:

1. `create()` checks current worktree count against `max_live`.
2. If over budget, calls `reclaim_idle()` to free idle worktrees.
3. If still over budget, returns `WorktreeError::BudgetExceeded`.

This prevents disk space exhaustion (each worktree is a full copy of the
working directory) and keeps the system within configured resource bounds.

---

## Thread Safety

`WorktreeManager` uses `Arc<WorktreeConfig>` for shared configuration and
a mutex-protected `HashMap<String, WorktreeHandle>` for mutable state.
Multiple async tasks can safely call `create()`, `remove()`, and
`check_health()` concurrently.

Git operations themselves are serialized by the filesystem -- git uses
lock files to prevent concurrent modifications to the same repository.

---

## Integration with Execution

When a plan is dispatched:

1. `ensure_for_plan(plan_id)` creates or reuses a worktree.
2. The worktree path becomes the `exec_dir` for all agent processes.
3. Gates run in the worktree directory (compile, test, clippy).
4. On merge, the worktree branch is merged into the target branch.

The worktree path is injected into `CellResources.worktree_path` so
cells can access it during execution. The workspace path
(`CellResources.workspace_path`) always points to the main repository,
while the worktree path points to the plan-specific working directory.

---

## Disk Awareness

The `roko doctor disk` command reports worktree status including:

- Number of live worktrees
- Total disk usage across all worktrees
- Stale worktrees (idle beyond TTL)
- Orphaned worktrees (not tracked by any plan)

The `roko cache prune` command can reclaim disk space from completed
plan worktrees. The disk admission check runs before worktree creation
to prevent exhausting available space.

---

## Error Types

```rust
pub enum WorktreeError {
    GitError(String),
    AlreadyExists(String),
    NotFound(String),
    BudgetExceeded { max_live: usize, current: usize },
}
```

All errors are actionable:
- `GitError`: check git state, run `git worktree prune`
- `AlreadyExists`: use `ensure_for_plan()` instead of `create()`
- `NotFound`: the worktree was removed externally
- `BudgetExceeded`: increase `max_live` or wait for idle reclamation

---

## Verification Commands

```bash
# Check worktree status
cargo run -p roko-cli -- doctor disk

# List git worktrees
git worktree list

# Prune stale worktree metadata
git worktree prune

# Clean up worktree caches
cargo run -p roko-cli -- cache prune
```
