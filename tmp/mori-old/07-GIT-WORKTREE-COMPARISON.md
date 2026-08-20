# Git, Worktree, and Merge Comparison: Mori vs Roko

## 1. Branch Naming Convention

### Mori

Mori uses the `codex/` namespace with a two-tier hierarchy:

```
codex/batch/<batch_id>        -- Aggregation branch for a run session
codex/plan/<plan_base>        -- Per-plan branch (e.g. codex/plan/01-workspace-scaffold)
codex/task/<plan>/<task>       -- (referenced in TUI graph but task branches are uncommon)
```

The batch branch collects all plan merges before being merged into `main`. Plans
branch from the batch, not from main directly. This creates a three-level merge
chain: plan -> batch -> main.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/mod.rs` lines 682-759.

### Roko

Roko uses the `roko/` namespace with three tiers:

```
roko/plan/<plan_id>            -- Per-plan branch
roko/task/<plan_id>/<task_id>  -- Per-task branch (legacy)
roko/attempt/<blake3_hash>     -- Per-attempt branch (collision-resistant hash)
```

Each task attempt gets its own branch via `format_attempt_branch_name()`, which
hashes `plan_id + task_id + attempt_number` through BLAKE3. There is no
intermediate batch branch -- plans merge directly. The attempt-level isolation is
a significant architectural difference: Mori shares a plan worktree across all
tasks, while Roko can give each attempt its own worktree.

Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrator/worktree.rs` lines 389-417.

---

## 2. Worktree Directory Layout

### Mori

Worktrees live at `.worktrees/` under the repo root:

```
<repo_root>/.worktrees/plan-<plan_base>/     -- Plan worktree
<repo_root>/.mori/tmp/merge-<target>-<ns>/   -- Temporary merge worktrees
<repo_root>/.mori/cache/cargo-target/        -- Shared cargo target
<repo_root>/.mori/cache/cargo-target-scoped/ -- Per-worktree scoped targets
```

Created by `WorktreeManager::new(repo_root)` which sets
`worktree_base = repo_root.join(".worktrees")`.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` lines 1372-1379.

### Roko

Worktrees use a configurable root directory:

```
<worktrees_root>/<id>/                       -- Plan or attempt worktree
.roko/state/worktree-snapshot.json           -- Persistent registry
```

The `WorktreeConfig` struct takes `worktrees_root` as an explicit parameter,
defaulting in the runner to `<workdir>/.roko/worktrees/`. Each worktree has a
stable `id` (the plan_id for plan worktrees, or a BLAKE3 hash for attempts).

Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrator/worktree.rs` lines 244-263.

---

## 3. Worktree Creation

### Mori

`create_plan_worktree(plan_base, base_branch)` runs the standard git plumbing:

1. Checks if the branch exists; if not, creates it from `base_branch`
2. `git worktree add <path> <branch>`
3. Copies untracked root files (Cargo.toml, Cargo.lock, rust-toolchain.toml, etc.)
   into the worktree -- but only files NOT tracked by git (to avoid clobbering
   branch-owned content)
4. Writes `.cursor/cli.json`, `.cursor/mcp.json`, `.codex/config.toml`,
   `.mori/mcp-config.local.json` for agent IDE integration
5. Writes `.cargo/config.toml` to redirect builds to a shared target directory
6. Regenerates workspace-map.md for context
7. Creates `context/in/` and `context/out/` directories

The creation is debounced via a `Mutex<HashMap<PathBuf, Instant>>` with a 5-second
window so rapid calls don't redundantly copy files.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` lines 809-867.

### Roko

`create(id, branch)` is a hardened async operation:

1. Validates the id (no path separators, NUL bytes, leading dots)
2. Acquires an `AsyncMutex` operation lock (single-writer concurrency)
3. Acquires a filesystem-level repository mutation lock (flock on the Git common
   directory)
4. Publishes a durable creation marker (journaled to disk via fd-relative I/O
   with inode identity verification on macOS/Linux)
5. Creates the branch via `git update-ref` with a compare-and-swap old OID
6. Registers the linked worktree by directly writing Git's administrative file
   layout (bypassing `git worktree add` because the no-descendant kernel resource
   profile blocks spawning child git processes)
7. Runs `git reset --hard` to populate the checkout
8. Removes the lock file and transitions the creation marker
9. On failure, rolls back: deletes the admin dir, removes the branch, removes the
   marker

The security model is notably more sophisticated: atomic mkdir, flock, and
fd-relative I/O protect against concurrent processes. The creation marker schema
(version 2) provides crash-recovery: if the process dies mid-creation, a
subsequent startup can detect the incomplete creation and roll it back. Budget
enforcement (`max_live`) happens before touching git.

Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrator/worktree.rs` lines 534-737.

---

## 4. Worktree Preparation (IDE/Agent Config)

### Mori

Prepares each worktree for three agent backends:

| Config File | Purpose |
|---|---|
| `.cursor/cli.json` | Cursor ACP agent file/run permissions (no "version" key) |
| `.cursor/mcp.json` | Cursor MCP server config pointing to `mori-mcp` binary |
| `.codex/config.toml` | Codex MCP server config with startup/tool timeouts |
| `.mori/mcp-config.local.json` | Mori runtime MCP config |
| `.cargo/config.toml` | Shared cargo target dir + sccache wrapper |

Also writes configs into the main repo root via `ensure_repo_mcp_configs()` and
refreshes all existing worktree configs with `refresh_existing_worktree_mcp_configs()`.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` lines 10-362.

### Roko

Roko does not write IDE/agent configs into worktrees at the worktree-manager
level. Agent integration (MCP, tool policy) is handled at the dispatch/runner
layer. The worktree manager is a pure git isolation primitive. The runner's event
loop handles creating per-attempt worktrees and setting up the execution
environment through the dispatch layer.

---

## 5. Merge Workflow

### Mori

Mori uses a three-stage merge pipeline with temporary merge worktrees:

**Plan -> Batch**: `merge_plan_to_batch(plan_base, batch_branch)`
- Uses `merge_ref_into_branch_via_temp_worktree()` which:
  1. Creates a temporary detached-HEAD worktree at `.mori/tmp/merge-<slug>-<ns>/`
  2. Runs `git merge --no-ff <source_ref> -m <message>` inside the temp worktree
  3. Uses `git update-ref` to advance the target branch to the merge commit
  4. Syncs the repo-root checkout if it happens to be on the target branch
  5. Cleans up the temp worktree (always, even on failure)
  6. On merge failure, aborts the merge in the temp worktree first
- After merging, deletes the plan branch with `git branch -d` (if not checked out
  elsewhere)

**Batch -> Staging**: `merge_batch_to_staging()` -- same temp-worktree mechanism

**Batch -> Main**: `merge_batch_to_main(batch_branch)` -- adds safety checks:
- Runs `ensure_merge_to_main_safe()` which checks for detached HEAD, unmerged
  paths, batch branch existence, main checked out elsewhere, and local/origin
  main divergence
- Refuses to merge if local main does not match origin/main

**Merge feasibility**: `check_merge_feasibility()` uses `git merge-tree --write-tree`
(git 2.39+) for zero-side-effect dry-run merge simulation, returning one of:
`Identical`, `FastForward`, `CleanMerge`, or `Conflicted(Vec<String>)`.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/mod.rs` lines 602-915,
`/Users/will/dev/uniswap/bardo/apps/mori/src/git/ops.rs` lines 71-141.

### Roko

Roko has a two-component merge system:

**MergeQueue** (`orchestrator/merge_queue.rs`): A file-conflict-aware queue that
serializes merges. Key features:
- Tracks which files each plan modifies
- Only allows a merge to proceed if its file set does not overlap with any
  in-progress merge
- Priority ordering with retry-count demotion
- Reservation system (`MergeReservation`) for atomic claim of the next mergeable
- `ready_batch()` computes the maximal set of non-conflicting merges
- Failed merges are tracked with a configurable retry budget (default 3)

**PlanMerger** (`runner/merge.rs`): Drives the actual merge through the queue:
- Uses a pluggable `MergeBackend` trait (default: `GitMergeBackend`)
- `GitMergeBackend::merge()` runs `git merge --no-ff --no-edit <branch>` in the
  working directory
- After merge, runs a pluggable `RegressionGate` (default: `cargo check`)
- Emits `GateCompletion` events so the runner event loop can react

**Branch Cleanup** (`runner/branch_cleanup.rs`): Post-run cleanup that:
- Enumerates `roko/plan/*`, `roko/task/*`, `roko/attempt/*` branches
- Checks each branch for a merged PR via `gh pr view --json state`
- Deletes both local and remote refs for merged branches

**GitHub Workflow** (`runner/github_workflow.rs`): Plan PRs and CI integration:
- Creates plan branches, pushes them, opens draft PRs
- Posts terminal comments/issues on completion
- Publishes exact accepted-commit references
- Coordinates local-regression + CI merge ordering

Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrator/merge_queue.rs`,
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/merge.rs`,
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/branch_cleanup.rs`.

---

## 6. Worktree Recovery and Health

### Mori

Sophisticated recovery system with five decisions:

| Decision | Meaning | Action |
|---|---|---|
| `Healthy` | Aligned with batch | Reuse as-is |
| `NeedsResync` | Stale, no useful commits | Discard and recreate |
| `NeedsRebase` | Behind batch but has commits | Cherry-pick agent commits onto fresh base |
| `ParseRepair` | Corrupt state | Quarantine + recreate |
| `QuarantineAndRecreate` | Dirty with uncommitted changes | Archive to `.mori/runs/recovery/` |
| `ManualAttention` | Unresolvable conflict | Quarantine + stop |

Recovery snapshots are persisted to `.mori/runs/recovery/<plan_base>/<timestamp>/`
with `report.toml` files. Recovery refs are stored under `refs/mori/recovery/`.
The `choose_recovery_snapshot_for_restore()` function selects the snapshot with
preserved commits (ahead > 0) for best-effort restore.

`diagnose_plan_worktree()` computes ahead/behind counts, checks for uncommitted
changes (filtering out ignorable MCP/fastembed artifacts), and detects
in-progress git operations (MERGE_HEAD, rebase-merge, CHERRY_PICK_HEAD).

`RefreshResult` tracks cherry-pick preservation: how many commits were preserved,
skipped (conflicted), and total.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` lines 1238-1340, 1837-2036.

### Roko

Health checking via `WorktreeHealth` enum:

| Health | Meaning |
|---|---|
| `Ok` | Path exists, expected branch checked out |
| `Missing` | Worktree directory absent |
| `StaleLock` | `.git/index.lock` older than 60 seconds |
| `Detached` | HEAD not on expected branch |

The `isolation_statuses()` method reports idle time, reclaimability, and path
existence for each tracked handle. `reclaim_idle()` removes worktrees exceeding
the configured `idle_ttl`. Stale locks are cleared by
`clear_stale_locks()`.

The `AcceptedWorktree` pattern provides a different recovery model: instead of
diagnosing and repairing existing worktrees, each new task attempt gets a fresh
worktree branching from the last accepted immutable tip. If an attempt fails, its
worktree is discarded; on success, `accept_attempt()` advances the plan's
accepted tip. This makes recovery a non-issue by design.

The `DirtyWorktree` error preserves owned or unknown changes rather than silently
discarding them.

Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrator/worktree.rs` lines 282-387.

---

## 7. Disk Management

### Mori

Aggressive multi-tier disk reclamation:

- **Low disk (< 30 GiB)**: Prune stale worktree `target/` dirs, scoped targets
- **Critical disk (< 15 GiB)**: Additionally prune root `target/debug`,
  `target/release/deps`, `target/release/build`
- **Recovery cleanup**: Prune old recovery snapshots, trim large patch files
  (> 64 MiB, or > 16 MiB under critical pressure)
- **Fastembed caches**: Remove `.fastembed_cache` from each worktree
- **Completed plan targets**: `prune_completed_plan_scoped_targets()` removes
  isolated build artifacts for plans that finished
- Shared cargo target at `.mori/cache/cargo-target/` with per-worktree scoping at
  `.mori/cache/cargo-target-scoped/<scope>/`

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` lines 1381-1574.

### Roko

Budget-based admission control:

- `max_live` config caps simultaneously active worktrees (returns
  `BudgetExhausted` error)
- `idle_ttl` marks worktrees for reclamation after inactivity
- `reclaim_idle()` removes worktrees that have exceeded their idle TTL
- `prune()` runs `git worktree prune` to clean stale metadata
- The runner event loop runs `cleanup_orphan_worktrees()` at session end to
  remove worktrees for plans in terminal states

The `roko doctor disk` CLI command reports free space, stale targets, oversized
JSONL logs, and worktree state. Disk-aware admission is checked before creating
new worktrees.

---

## 8. Git Safety

### Mori

`GitSafetyReport` performs pre-merge safety checks:

- Detached HEAD detection
- Dirty repo root detection
- Unmerged paths listing
- Branch checked-out-elsewhere detection (prevents merging into a branch that
  another worktree has checked out)
- Local/origin main divergence detection
- Stale/locked worktree detection

`AutoStashSession` ties auto-stash restoration to the lifetime of a run. When
checkout would overwrite local changes, Mori stashes automatically and restores
on session drop (LIFO order). Failed stash pops preserve the stash and emit a
`MANUAL:` error event.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/git/mod.rs` lines 12-200, 522-600.

### Roko

The worktree manager uses a stricter security model:

- **Repository mutation lock**: flock-based lock on a file in the Git common
  directory prevents concurrent mutations across processes
- **Creation markers**: Journaled, inode-verified markers detect incomplete
  creations from crashed processes
- **No-descendant Git execution**: Git processes run with a kernel resource
  profile that prevents them from spawning child processes (linked-worktree
  registration bypasses `git worktree add` to work within this constraint)
- **Compare-and-swap branch creation**: `git update-ref --no-deref` with expected
  old OID prevents race conditions
- **Dirty worktree preservation**: `remove()` fails with `DirtyWorktree` error
  instead of silently discarding changes

---

## 9. TUI Git View (F4)

### Mori

Full-featured git visualization at F4:

**Layout**: Left 35% (branch tree 60% + worktree list 40%) | Right 65% (commit
graph 60% + branch info 40%)

**Branch tree** (`widgets/branch_tree.rs`):
- Hierarchical: main -> batch -> plan with tree connectors
- Status-based coloring from `RunPlanStatus`
- Worktree path annotations
- Scrollbar with selection highlighting
- Groups `codex/batch/*` and `codex/plan/*` branches

**Worktree list** (`views/git_view.rs`):
- Branch, commit hash, flags (detached/locked/stale), path
- Plan-filtered view: shows only batch + main + selected plan's worktree
- Color-coded flags: ember for stale, rose for locked/detached, sage for ok

**Commit graph**: `git log --oneline --graph --decorate --all`
- Per-lane coloring for graph characters (`|`, `/`, `\`, `*`)
- Auto-commit collapsing: 3+ consecutive auto-commits from the same plan are
  collapsed into a summary line showing count and agent names
- Status-aware commit coloring (merged references highlighted)

**Branch info panel**: Current branch, merge feasibility, ahead/behind counts,
divergence warnings.

Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/views/git_view.rs`,
`/Users/will/dev/uniswap/bardo/apps/mori/src/tui/widgets/branch_tree.rs`.

### Roko

Similar F4 git tab but independently implemented:

**Layout**: Left 35% (branch tree 50% + worktree list 25% + status 25%) |
Right 65% (commit log table + branch info)

**Branch tree** (`tui/widgets/branch_tree.rs`):
- Three branch types: Local, Remote, Tag
- Recursive tree with `flatten_tree()` -- connectors use Unicode box-drawing
- Selection highlighting and cursor tracking
- Simpler than Mori: no status-from-plan, no worktree path annotations

**Worktree list**: Branch, status string, path

**Git watcher** (`tui/git_watch.rs`):
- `notify::RecommendedWatcher` on git admin paths with 500ms debounce
- Resolves linked-worktree `.git` indirection files to watch the correct
  admin directory
- Falls back to polling if notify fails
- Event-driven refresh instead of periodic polling

**Data population** (`views/git_view.rs`):
- `GitViewData` populated by background refresh thread (zero I/O in render path)
- `GitBranchNode` includes ahead/behind counts
- Graceful "not a git repository" handling

Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/git_view.rs`,
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/branch_tree.rs`,
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/git_watch.rs`.

---

## 10. Summary: Architectural Differences

| Aspect | Mori | Roko |
|---|---|---|
| **Branch hierarchy** | 3-level: plan -> batch -> main | 3-level: attempt -> plan -> main (no batch) |
| **Worktree granularity** | Per-plan (tasks share one worktree) | Per-attempt (each attempt gets its own) |
| **Branch naming** | `codex/plan/<name>`, `codex/batch/<id>` | `roko/plan/<id>`, `roko/attempt/<blake3>` |
| **Worktree location** | `.worktrees/plan-<base>` | Configurable `worktrees_root/<id>` |
| **Creation mechanism** | `git worktree add` shell command | Direct Git admin file layout registration |
| **Concurrency control** | Single-threaded sequential + plan-level parallelism | Async mutex + flock + creation markers |
| **Merge strategy** | Temporary detached-HEAD merge worktrees | File-conflict-aware merge queue with reservations |
| **Recovery model** | Diagnose + rebase/quarantine/restore | Immutable accepted tips; discard failed attempts |
| **IDE integration** | Written into each worktree (Cursor, Codex, MCP) | Handled at runner/dispatch layer |
| **Merge feasibility** | `git merge-tree --write-tree` dry run | File-set overlap check in merge queue |
| **Safety checks** | GitSafetyReport + AutoStashSession | Repository mutation lock + creation markers |
| **Disk management** | Multi-tier: 30 GiB/15 GiB thresholds | Budget-based max_live + idle_ttl |
| **Cleanup** | Branch deletion after merge + worktree dir removal | `gh pr view` merge check + local/remote ref deletion |
| **Git TUI** | Plan-status coloring, auto-commit collapsing, lane colors | Node types, ahead/behind, fs-watcher refresh |
| **Crash recovery** | Recovery snapshots with refs under `refs/mori/recovery/` | Journaled creation markers with inode verification |
| **Git event system** | `GitEvent` enum via `mpsc::UnboundedSender` | Events handled by runner event loop directly |

### Key evolution from Mori to Roko

1. **Attempt-level isolation**: Mori's per-plan worktrees meant concurrent tasks
   within a plan could conflict. Roko's per-attempt worktrees eliminate this by
   giving each task attempt its own checkout.

2. **Fail-discard over repair**: Mori invested heavily in recovery (cherry-pick
   preservation, quarantine, rebase). Roko takes the opposite approach -- failed
   attempts are cheaply discarded and recreated from the last accepted tip.

3. **Security hardening**: Roko's creation markers with inode verification, flock
   locks, and no-descendant execution profiles are substantially more paranoid
   than Mori's straightforward `git worktree add`.

4. **Merge queue**: Mori's temp-worktree merge approach works but doesn't prevent
   concurrent plan merges from conflicting. Roko's file-level conflict tracking
   with reservation semantics prevents overlapping merges entirely.

5. **GitHub integration**: Roko adds PR lifecycle management (draft PRs, terminal
   comments, exact commit publication, CI merge coordination) that Mori handled
   externally.
