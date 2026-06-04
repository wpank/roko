# 13 — Git Diff Cache for the Gate Phase (B09)

> Bottleneck: the orchestrator spawns `git diff` (twice — `--stat` and
> full) for the LLM judge gate, the review prompt, the diff gate, and
> auto-detect of gate mode (Plan 10). Each spawn is 50–150 ms. With 3–5
> consumers per run, the cost adds up.
>
> Target savings: 50–200 ms per run.
> Effort: ≈1 h. Risk: low.

---

## Goal & success criteria

After this change:

1. A `GitDiffSnapshot` struct is computed **once per gate phase** of a
   workflow run.
2. All consumers (LLM judge, review prompt, diff gate, auto-detect)
   read from the cached snapshot.
3. The snapshot is invalidated when the gate phase begins a new
   iteration (new code may have been written).

Done when:

- `rg "git.*diff" crates/roko-cli/src/orchestrate.rs` returns at most
  **one** location: the snapshot constructor.
- A unit test confirms two consumers in the same gate phase share one
  `git diff` subprocess invocation.
- Macro-benchmark on standard workflow shows ≥40 ms improvement.

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B09,
  `OPTIMIZATION-PLAYBOOK.md` §10.
- Current call sites (verified):

  ```text
  crates/roko-cli/src/orchestrate.rs
   17767  gate_diff_for_plan(plan_id) -> Option<String>
            spawns "git diff HEAD" + fallback "git diff --cached"
   17895  build_review_prompt
            spawns "git diff --name-only HEAD"
   18922  run_plan_verify_steps
            spawns "git diff --cached"
  ```

- Each call site is async (`tokio::process::Command`), but the cost
  per call is ≈80–150 ms wall-clock dominated by process fork +
  git's own startup. Caching is the right primitive.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-cli/src/orchestrate.rs` | All current call sites. |
| `crates/roko-runtime/src/effect_driver.rs` | Where `gate_runner.run_gates` is invoked from the workflow engine. |
| `crates/roko-gate/src/gate_service.rs` | The receiver of `GateConfig`; carry the snapshot here. |

---

## Code-level plan

### Step 1 — Define `GitDiffSnapshot`

New file: `crates/roko-cli/src/git_diff_snapshot.rs`.

```rust
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
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
    pub async fn compute(workdir: &Path) -> Self {
        let (full, stat, names) = tokio::join!(
            run_git(workdir, &["diff", "HEAD"]),
            run_git(workdir, &["diff", "--stat", "HEAD"]),
            run_git(workdir, &["diff", "--name-only", "HEAD"]),
        );
        let diff_full = full.unwrap_or_default();
        let diff_stat = stat.unwrap_or_default();
        let modified_files = names
            .unwrap_or_default()
            .lines()
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
        self.modified_files.is_empty()
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

### Step 2 — Wire the snapshot through the gate phase

The `Orchestrator` (or whichever struct enters the gate phase) computes
the snapshot once at phase start:

```rust
// orchestrate.rs, beginning of the gate phase per task
let diff_snapshot = Arc::new(GitDiffSnapshot::compute(&exec_dir).await);
self.set_active_diff_snapshot(plan_id, Arc::clone(&diff_snapshot));

// ... gate runs ...

// After last consumer, drop the snapshot or let it die with the phase.
self.clear_active_diff_snapshot(plan_id);
```

A simple owner-side cache:

```rust
struct Orchestrator {
    diff_snapshots: parking_lot::RwLock<HashMap<String, Arc<GitDiffSnapshot>>>,
}

impl Orchestrator {
    fn set_active_diff_snapshot(&self, plan_id: &str, snap: Arc<GitDiffSnapshot>) {
        self.diff_snapshots.write().insert(plan_id.into(), snap);
    }
    fn clear_active_diff_snapshot(&self, plan_id: &str) {
        self.diff_snapshots.write().remove(plan_id);
    }
    fn active_diff_snapshot(&self, plan_id: &str) -> Option<Arc<GitDiffSnapshot>> {
        self.diff_snapshots.read().get(plan_id).cloned()
    }
}
```

### Step 3 — Convert each call site

For each of the three current call sites, replace the inline
`tokio::process::Command::new("git")…` with a snapshot lookup:

```rust
async fn gate_diff_for_plan(&self, plan_id: Option<&str>) -> Option<String> {
    let plan_id = plan_id?;
    let snap = self.active_diff_snapshot(plan_id)?;
    if !snap.is_empty() { Some(snap.diff_full.clone()) } else { None }
}
```

```rust
// build_review_prompt — files_changed
let files_changed = self.active_diff_snapshot(plan_id)
    .map(|s| s.modified_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>())
    .unwrap_or_default();
```

For `run_plan_verify_steps` (which uses `--cached`), keep the original
spawn — `--cached` is a different diff than the snapshot's
working-tree diff. Document the divergence in a comment so the next
agent doesn't accidentally "unify" them.

### Step 4 — Auto-detect (Plan 10) reuses the snapshot

If both Plan 10 and Plan 13 are merged, refactor `detect_gate_mode`
(Plan 10 §Step 4) to take `&GitDiffSnapshot` instead of spawning its
own diff:

```rust
fn detect_gate_mode(snap: &GitDiffSnapshot) -> GateMode {
    if snap.modified_files.is_empty() { return GateMode::None; }
    let names: Vec<&str> = snap.modified_files.iter()
        .filter_map(|p| p.to_str()).collect();
    let has_code = names.iter().any(|f|
        f.ends_with(".rs") || f.ends_with(".ts") || /* ... */);
    /* ... rest unchanged ... */
}
```

---

## Step-by-step execution

1. `git checkout -b perf/13-git-diff-cache`.
2. Add `git_diff_snapshot.rs` (Step 1). Tests: empty repo, dirty repo,
   no-diff repo.
3. Add the cache to `Orchestrator` (Step 2).
4. Convert each call site (Step 3). Document the `--cached` divergence.
5. (If Plan 10 already merged) Refactor `detect_gate_mode` (Step 4).
6. Macro-benchmark on standard workflow.
7. PR `perf(orchestrator): cache git diff snapshot per gate phase
   (B09)`.

---

## Anti-patterns / things NOT to do

- **Do NOT cache the diff across iterations of the workflow loop.**
  Each iteration potentially writes new code; reusing the iteration-1
  diff in iteration-2 produces wrong gate verdicts. Drop and recompute
  at the start of each iteration.
- **Do NOT cache the diff across `roko run` invocations** by
  persisting to disk. The state of the workdir between invocations is
  unknown; trust the live `git diff`.
- **Do NOT replace `git diff --cached` with `git diff HEAD`** anywhere.
  `--cached` is what is staged for commit; `HEAD` is what differs from
  the previous commit. Different surfaces; different semantics. The
  `run_plan_verify_steps` use of `--cached` is intentional.
- **Do NOT add a `git status` to the snapshot.** That's a separate
  command with separate cost; if a consumer needs untracked files,
  they should call out to `git status` and we can cache *that* in a
  follow-up plan.
- **Do NOT call `git diff` from within a tight loop.** If you find a
  consumer that does (e.g., per-task in a plan), restructure the
  consumer to take the snapshot once and reuse it.
- **Do NOT join more than 3 git subprocesses.** Each fork costs ≈30 ms
  on its own. The 3-way join above is the sweet spot; adding more (e.g.,
  `git log`) is more cost than savings.

---

## Test plan

```rust
#[tokio::test]
async fn snapshot_includes_full_diff_and_modified_files() {
    let dir = git_init_with_changes(&[("a.rs", "fn main(){}")]);
    let snap = GitDiffSnapshot::compute(dir.path()).await;
    assert!(!snap.diff_full.is_empty());
    assert!(snap.modified_files.iter().any(|p| p == &PathBuf::from("a.rs")));
}

#[tokio::test]
async fn snapshot_is_empty_on_clean_repo() {
    let dir = git_init_clean();
    let snap = GitDiffSnapshot::compute(dir.path()).await;
    assert!(snap.is_empty());
}

#[tokio::test]
async fn two_consumers_share_one_snapshot() {
    let dir = git_init_with_changes(&[("a.rs", "x")]);
    let counter = install_git_call_counter();
    let snap = Arc::new(GitDiffSnapshot::compute(dir.path()).await);
    let calls_after_compute = counter.load(Ordering::Relaxed);   // 3 (full+stat+names)
    let _files = snap.modified_files.clone();
    let _diff = snap.diff_full.clone();
    let calls_now = counter.load(Ordering::Relaxed);
    assert_eq!(calls_now, calls_after_compute, "consumers should not re-shell");
}
```

Macro-benchmark: standard workflow with judge gate enabled. Expect
≥40 ms improvement (1 spawn instead of 3).

---

## Rollback plan

- The snapshot lives entirely inside the orchestrator; reverting the
  wiring restores per-call git invocations.
- `git_diff_snapshot.rs` becomes dead code; harmless.

---

## Status check (acceptance)

- [ ] `GitDiffSnapshot` exists with tests.
- [ ] Orchestrator computes one snapshot per gate phase per plan.
- [ ] All previous direct `tokio::process::Command::new("git")
      .args(["diff", ...])` call sites in the gate phase route through
      the snapshot.
- [ ] `--cached` use site retains its own spawn with a documenting
      comment.
- [ ] Macro-benchmark improvement ≥40 ms recorded.
