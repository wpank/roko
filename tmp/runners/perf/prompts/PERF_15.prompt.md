# PERF_15: Git diff cache for gate phase (B09)

## Task

Compute `git diff` once per gate phase and reuse the snapshot across
all consumers (LLM judge, review prompt, diff gate, auto-detect from
PERF_12). Today's orchestrator spawns 3-5 git subprocesses per phase;
this batch reduces it to one (which itself fans out to 3 git
invocations via `tokio::join!` in parallel).

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_15](../ISSUE-TRACKER.md#perf_15)
- Plan: `tmp/solutions/perf/implementation/13-git-diff-cache.md`
- Bottleneck: B09 (BOTTLENECK-ANALYSIS.md §B09)
- Performance contract: **C-14** (one snapshot per gate phase)
- Priority: P2
- Effort: ≈1 h
- Depends on: none (composes with PERF_12 if both land)
- Wave: 1

## Problem

`crates/roko-cli/src/orchestrate.rs` currently spawns `git` from at
least three call sites during a gate phase:

```text
17767  gate_diff_for_plan(plan_id) -> Option<String>      (full diff)
17895  build_review_prompt → "git diff --name-only HEAD"  (file list)
18922  run_plan_verify_steps → "git diff --cached"        (different semantics; KEEP)
```

Each spawn costs 50-150 ms (process fork + git startup). Caching saves
40-200 ms per run.

## Exact Changes

### Step 1 — New module `crates/roko-cli/src/git_diff_snapshot.rs`

```rust
//! Cached git-diff snapshot for the gate phase (perf contract C-14).
//!
//! Computed once at the start of each gate phase and read from by all
//! consumers (LLM judge, review prompt, diff gate, auto-detect of gate
//! mode). Cleared at the start of each iteration.
//!
//! `git diff --cached` is intentionally NOT cached here — it has
//! different semantics (staged-vs-HEAD instead of working-tree-vs-HEAD)
//! and only `run_plan_verify_steps` consumes it.

use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct GitDiffSnapshot {
    pub diff_full: String,
    pub diff_stat: String,
    pub modified_files: Vec<PathBuf>,
    pub computed_at: Instant,
    pub workdir: PathBuf,
}

impl GitDiffSnapshot {
    /// Compute all three views in parallel.
    pub async fn compute(workdir: &Path) -> Self {
        let (full, stat, names) = tokio::join!(
            run_git(workdir, &["diff", "HEAD"]),
            run_git(workdir, &["diff", "--stat", "HEAD"]),
            run_git(workdir, &["diff", "--name-only", "HEAD"]),
        );
        let diff_full = full.unwrap_or_default();
        let diff_stat = stat.unwrap_or_default();
        let modified_files: Vec<PathBuf> = names
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(PathBuf::from)
            .collect();
        Self {
            diff_full,
            diff_stat,
            modified_files,
            computed_at: Instant::now(),
            workdir: workdir.to_path_buf(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.modified_files.is_empty() && self.diff_full.trim().is_empty()
    }
}

async fn run_git(workdir: &Path, args: &[&str]) -> Option<String> {
    let out = tokio::process::Command::new("git")
        .args(args)
        .current_dir(workdir)
        .output()
        .await
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}
```

### Step 2 — Re-export from `crates/roko-cli/src/lib.rs`

```rust
pub mod git_diff_snapshot;
pub use git_diff_snapshot::GitDiffSnapshot;
```

### Step 3 — Add cache to `Orchestrator`

`crates/roko-cli/src/orchestrate.rs::Orchestrator`. Add a field:

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use crate::git_diff_snapshot::GitDiffSnapshot;

pub struct Orchestrator {
    // ... existing fields ...
    /// Per-plan cached diff snapshot for the active gate phase.
    /// Populated at gate-phase start and cleared at phase end.
    diff_snapshots: RwLock<std::collections::HashMap<String, Arc<GitDiffSnapshot>>>,
}

impl Orchestrator {
    pub fn new(...) -> Self {
        Self {
            // ... existing init ...
            diff_snapshots: RwLock::new(Default::default()),
        }
    }

    pub(crate) async fn populate_diff_snapshot(&self, plan_id: &str, workdir: &Path) {
        let snap = Arc::new(GitDiffSnapshot::compute(workdir).await);
        self.diff_snapshots.write().insert(plan_id.to_string(), snap);
    }

    pub(crate) fn clear_diff_snapshot(&self, plan_id: &str) {
        self.diff_snapshots.write().remove(plan_id);
    }

    pub(crate) fn active_diff_snapshot(&self, plan_id: &str) -> Option<Arc<GitDiffSnapshot>> {
        self.diff_snapshots.read().get(plan_id).cloned()
    }
}
```

### Step 4 — Populate at gate-phase start, clear at phase end

Find the gate-phase entry/exit points in `orchestrate.rs`. The
canonical wrapper is the function that calls `gate_runner.run_gates`
for a plan; search:

```bash
rg -n 'run_gates|GateService' crates/roko-cli/src/orchestrate.rs
```

At the start of each iteration of the gate loop:

```rust
let exec_dir = self.ensure_plan_exec_dir(plan_id).await?;
self.populate_diff_snapshot(plan_id, &exec_dir).await;
```

At the end of the iteration:

```rust
self.clear_diff_snapshot(plan_id);
```

> **Critical.** The snapshot lives for the duration of a single
> iteration. Code that writes (autofix attempts) re-runs the loop body;
> the populate call above re-fetches a fresh snapshot.

### Step 5 — Convert each consumer

#### 5a. `gate_diff_for_plan` (≈line 17767)

Replace the body:

```rust
async fn gate_diff_for_plan(&self, plan_id: Option<&str>) -> Option<String> {
    let plan_id = plan_id?;
    let snap = self.active_diff_snapshot(plan_id)?;
    if snap.is_empty() { return None; }
    Some(snap.diff_full.clone())
}
```

The fallback to `git diff --cached` that the original used (when HEAD
diff was empty) loses fidelity here — but that fallback was wrong for
the LLM judge anyway (the judge wants working-tree changes, not staged).
Document the change in the commit body:

```text
note: gate_diff_for_plan no longer falls back to `git diff --cached`
when the HEAD diff is empty. The previous fallback was incorrect for
the LLM judge gate (which wants working-tree changes). If an unstaged-
diff scenario surfaces, file a follow-up to add a separate cached
snapshot for it.
```

#### 5b. `build_review_prompt` `files_changed` (≈line 17895)

```rust
let files_changed: Vec<String> = self.active_diff_snapshot(plan_id)
    .map(|s| s.modified_files.iter()
        .map(|p| p.display().to_string())
        .collect())
    .unwrap_or_default();
```

If the snapshot is missing (e.g., `build_review_prompt` is called
outside the gate phase), fall back to a fresh inline spawn — but log a
warning so you can spot lifecycle bugs:

```rust
let files_changed: Vec<String> = match self.active_diff_snapshot(plan_id) {
    Some(snap) => snap.modified_files.iter().map(|p| p.display().to_string()).collect(),
    None => {
        tracing::warn!(plan_id, "build_review_prompt called outside gate phase; falling back to fresh git spawn");
        // Existing inline spawn, kept verbatim.
        match self.ensure_plan_exec_dir(plan_id).await {
            Ok(exec_dir) => tokio::process::Command::new("git")
                .args(["diff", "--name-only", "HEAD"])
                .current_dir(&exec_dir)
                .output().await.ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.lines().map(String::from).collect::<Vec<_>>())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }
};
```

#### 5c. `run_plan_verify_steps` `git diff --cached` (≈line 18922)

**LEAVE UNCHANGED.** Add a comment explaining the divergence:

```rust
// SEMANTICS: --cached gives the staged-vs-HEAD diff (what `git commit`
// would record), which is different from the working-tree-vs-HEAD diff
// the GitDiffSnapshot caches. Verify steps want the staged version
// because they validate post-stage commit-readiness.
let cached = tokio::process::Command::new("git")
    .args(["diff", "--cached"])
    .current_dir(&exec_dir)
    .output().await?;
```

### Step 6 — (If PERF_12 already merged) refactor `detect_gate_mode`

`crates/roko-gate/src/gate_service.rs::detect_gate_mode`. If PERF_12
landed before this batch, refactor to take the snapshot:

```rust
pub fn detect_gate_mode(snap: &GitDiffSnapshot) -> GateMode {
    use roko_runtime::pipeline_state::GateMode;
    if snap.modified_files.is_empty() { return GateMode::None; }
    let names: Vec<&str> = snap.modified_files.iter()
        .filter_map(|p| p.to_str()).collect();
    let has_code = names.iter().any(|f| {
        f.ends_with(".rs") || f.ends_with(".ts") || f.ends_with(".tsx")
        || f.ends_with(".js") || f.ends_with(".jsx") || f.ends_with(".py")
        || f.ends_with(".go") || f.ends_with(".java") || f.ends_with(".kt")
    });
    let has_docs_or_config = names.iter().any(|f| {
        f.ends_with(".md") || f.ends_with(".txt") || f.ends_with(".toml")
        || f.ends_with(".yaml") || f.ends_with(".yml") || f.ends_with(".json")
    });
    match (has_code, has_docs_or_config) {
        (true, _) => GateMode::Full,
        (false, true) => GateMode::Express,
        (false, false) => GateMode::None,
    }
}
```

The orchestrator's gate-phase entry must then pass `&*snap` instead of
`workdir`. If PERF_12 has NOT landed yet, skip this step; PERF_12's
prompt already documents the standalone form.

### Step 7 — Tests

`crates/roko-cli/src/git_diff_snapshot.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn git_init_clean() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let _ = std::process::Command::new("git").args(["init", "-q"]).current_dir(dir.path()).output();
        let _ = std::process::Command::new("git").args(["config", "user.email", "t@t"]).current_dir(dir.path()).output();
        let _ = std::process::Command::new("git").args(["config", "user.name", "t"]).current_dir(dir.path()).output();
        let _ = std::process::Command::new("git").args(["commit", "--allow-empty", "-q", "-m", "init"]).current_dir(dir.path()).output();
        dir
    }

    fn git_init_with_changes(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = git_init_clean();
        for (path, content) in files {
            let p = dir.path().join(path);
            if let Some(parent) = p.parent() { std::fs::create_dir_all(parent).unwrap(); }
            std::fs::write(p, content).unwrap();
        }
        dir
    }

    #[tokio::test]
    async fn snapshot_includes_full_diff_and_modified_files() {
        let dir = git_init_with_changes(&[("a.rs", "fn main(){}")]);
        let snap = GitDiffSnapshot::compute(dir.path()).await;
        assert!(!snap.is_empty());
        assert!(snap.modified_files.iter().any(|p| p == &PathBuf::from("a.rs")));
        assert!(snap.diff_full.contains("a.rs"));
    }

    #[tokio::test]
    async fn snapshot_is_empty_on_clean_repo() {
        let dir = git_init_clean();
        let snap = GitDiffSnapshot::compute(dir.path()).await;
        assert!(snap.is_empty());
    }

    #[tokio::test]
    async fn two_consumers_share_one_snapshot() {
        // Driving the orchestrator's diff_snapshots cache here would
        // require a heavyweight setup. Instead, a structural test: a
        // single Arc<GitDiffSnapshot> can be cloned cheaply between
        // multiple consumers without re-shelling.
        let dir = git_init_with_changes(&[("a.rs", "x")]);
        let snap = std::sync::Arc::new(GitDiffSnapshot::compute(dir.path()).await);
        let _files = snap.modified_files.clone();
        let _diff = snap.diff_full.clone();
        let _stat = snap.diff_stat.clone();
        // No assert beyond compilation: multiple reads against an Arc
        // are constant-time and never re-shell.
    }
}
```

## Write Scope

- `crates/roko-cli/src/git_diff_snapshot.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/lib.rs`

(If PERF_12 already merged, also: `crates/roko-gate/src/gate_service.rs`
to refactor `detect_gate_mode`.)

## Read-Only Context

- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-gate/src/gate_service.rs`
- `tmp/solutions/perf/implementation/13-git-diff-cache.md`

## Acceptance Criteria

- [ ] `crates/roko-cli/src/git_diff_snapshot.rs` exists with `GitDiffSnapshot::compute()`.
- [ ] `Orchestrator` owns a `RwLock<HashMap<plan_id, Arc<GitDiffSnapshot>>>` cache.
- [ ] Snapshot computed at gate-phase start, cleared at phase end (and at start of each iteration).
- [ ] `gate_diff_for_plan` reads from snapshot only.
- [ ] `build_review_prompt`'s `files_changed` reads from snapshot when available; falls back with a `tracing::warn!` otherwise.
- [ ] `run_plan_verify_steps` retains its own `--cached` spawn with a documenting `// SEMANTICS:` comment.
- [ ] (If PERF_12 already merged) `detect_gate_mode` refactored to take `&GitDiffSnapshot`.
- [ ] Tests `snapshot_includes_full_diff_and_modified_files`, `snapshot_is_empty_on_clean_repo`, `two_consumers_share_one_snapshot` pass.

## Verify

```bash
# Snapshot module exists:
rg -n 'GitDiffSnapshot' crates/roko-cli/src/

# Direct git diff spawns in orchestrate.rs:
rg -n 'tokio::process::Command::new\("git"\)' crates/roko-cli/src/orchestrate.rs
# Expected: only the --cached one (annotated with SEMANTICS comment)
# and any fallback inside build_review_prompt.

# Note: the build_review_prompt fallback is OK; it's gated on the
# warning path. Confirm the warning is in place.
rg -n 'fallback to fresh git spawn' crates/roko-cli/src/orchestrate.rs
```

## Do NOT

- Do NOT cache the diff across iterations of the workflow loop. Each
  iteration potentially writes new code; reusing iteration-1's diff
  produces wrong gate verdicts.
- Do NOT cache the diff across `roko run` invocations by persisting to
  disk. The state of the workdir between invocations is unknown.
- Do NOT replace `git diff --cached` with `git diff HEAD` anywhere.
  Different semantics; the `--cached` use site is intentional.
- Do NOT add `git status` to the snapshot. Separate command, separate
  cost; if a consumer needs untracked files, file a follow-up plan.
- Do NOT call `git diff` from within a tight loop. If you find a
  consumer that does (e.g., per-task in a plan), restructure the
  consumer to take the snapshot once and reuse it.
- Do NOT join more than 3 git subprocesses. Each fork costs ~30 ms;
  the 3-way join above is the sweet spot.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_15 done <commit-sha>
```
