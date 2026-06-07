# Parallel Agent Runner: Lessons, Patterns, and Operational Guide

> Written from the experience of running ~195 parallel code-generation batches
> via Codex (gpt-5.4-mini) against a 177K LOC Rust workspace (18 crates).
> Total time: ~6 hours to run 195 batches. Many learnings paid for in pain.

---

## 1. What the Runner Does

The runner takes a **DAG of code tasks** (defined in `batches.toml`), dispatches
them in parallel to AI agents (Codex) in isolated git worktrees, merges completed
work back into a central branch, and gates quality at wave boundaries.

Think of it as CI in reverse: instead of testing code humans wrote, it *generates*
code, validates it, and integrates it — all automatically.

### Core Flow

```
batches.toml  →  DAG scheduler  →  N parallel worktrees
                                      ↓
                              codex exec (per batch)
                                      ↓
                              anti-pattern checks
                                      ↓
                          merge to run branch (serialized)
                                      ↓
                          wave gate (cargo check + clippy)
                                      ↓
                          checkpoint merge-back to source
```

---

## 2. Architecture: How Parallel Code Generation Works

### 2.1 The Worktree Model

Each batch gets its own **git worktree** — a full checkout of the repo forked from
the runner's main branch at dispatch time. This means:

- **No lock contention** between batches reading/writing files
- **Full isolation** — batch A can't corrupt batch B's working tree
- **Deterministic base** — all batches in a wave start from the same commit
- **Preserved on failure** — worktrees survive for inspection/debugging

```
.roko/worktrees/
  mega-parity-run-20260429-030528-main/     ← runner's integration branch
  mega-parity-run-20260429-030528-R2_A01/   ← batch worktree
  mega-parity-run-20260429-030528-R2_A02/   ← another batch
  ...
```

Each worktree also gets a **named branch**: `codex/mega-parity-run-YYYYMMDD-HHMMSS-BATCH_ID`.
These branches are **never deleted** by the runner — they're always available for
manual merge, cherry-pick, or inspection.

### 2.2 The DAG Scheduler

Batches declare dependencies in `batches.toml`:

```toml
[[batch]]
id = "R5_B04"
title = "Regression test: one attempt + failed gate = one cost event"
group = "5B"
deps = ["R5_B03"]  # must complete before this can start
scope = ["crates/roko-cli/src/orchestrate.rs"]
also_read = ["crates/roko-cli/src/commands/learn.rs"]
```

The scheduler runs in **waves**: find all batches whose deps are satisfied,
dispatch up to N in parallel, wait for completion, merge results, repeat.

**Key insight**: The wave model means batches within the same wave can't see each
other's changes. Only after merge do subsequent waves see accumulated work. This
is fine for independent changes but causes problems when batches in the same wave
touch the same files (merge conflicts).

### 2.3 The Branch Model

```
main (or wp-arch2)                    ← your working branch
  └→ codex/mega-parity-run-...-main   ← runner's integration branch
       ├→ codex/..-R2_A01             ← batch branch (merged to main)
       ├→ codex/..-R2_A02             ← batch branch
       └→ codex/..-R3_G01-backup-...  ← backup on retry
```

Three tiers:
1. **Source branch** — where you're working (wp-arch2). Runner optionally merges back here.
2. **Runner branch** — where all batch merges accumulate. One per run.
3. **Batch branches** — one per batch execution, forked from runner branch.

When `--no-merge-back` is used, the runner branch accumulates all work and you
cherry-pick from it into your source branch manually. This is the safer mode.

### 2.4 Context Handoff Between Batches

Each batch prompt includes:

1. **Context pack** (`context-pack/*.md`) — shared rules, architecture, anti-patterns
2. **Cumulative section** — shows what files changed in prior batches (updated after each merge)
3. **File contents** — live snapshot of scope files from the worktree
4. **Previous failure context** — if retrying, includes the last error log
5. **The batch prompt** (`prompts/BATCH_ID.prompt.md`) — the actual task

The cumulative section is critical: it tells each batch "here's what other batches
changed in the files you'll be editing." Without it, batches blindly write code
that conflicts with work done 5 minutes ago by a sibling batch.

**Lesson**: Context is king. The more precisely you tell each agent what changed
around it, the fewer conflicts and regressions you get. But there's a token budget —
the cumulative section uses signature-only views for large files.

---

## 3. Speed: How to Make Things Go Fast

### 3.1 The #1 Speed Killer: Compilation

In a Rust workspace with 18 crates and 177K LOC, `cargo check --workspace` takes
3-8 minutes. `cargo clippy` adds another 2-5 minutes. `cargo test` is 5-15 minutes.

If every batch runs its own compile+lint+test cycle, a batch that takes 30 seconds
to write code will take 15-40 minutes total. With 195 batches, that's the difference
between finishing in 4 hours vs 50+ hours.

**The solution: defer compilation to wave boundaries.**

Three levels of build deferral, from conservative to aggressive:

| Level | What | Speed | Safety |
|---|---|---|---|
| Per-batch verify | Each batch runs `cargo check -p <crate>` | Slowest | Safest |
| Wave gates only | `cargo check --workspace` after each wave completes | Medium | Good |
| No gates at all | Only compile at the very end | Fastest | Riskiest |

The runner supports all three:
- **Per-batch**: Prompt includes "run cargo check" → agent does it
- **Wave gates**: `--no-gate` disables them (flag in `run-parallel.sh`)
- **No gates**: `--no-gate --no-test` skips everything

### 3.2 How We Actually Disabled Builds

We used a **context file** (`context-pack/05-NO-BUILD.md`) that tells the agent:

```markdown
# Build Policy
**Do NOT run any compilation or test commands.** This includes:
- `cargo check`, `cargo clippy`, `cargo build`, `cargo test`
- `rustup run stable cargo ...`
Focus exclusively on writing correct code. Do not attempt to verify it compiles.
If the batch prompt includes "Verification Commands", **ignore those instructions**.
```

This reduced batch times from **15-40 minutes to 1-5 minutes**.

**Why not a fake cargo binary?** We tried that too (`bin/cargo` → no-op script).
It works for plain `cargo check` but agents find ways around it:
- `rustup run stable cargo test` bypasses the fake `cargo`
- Agents use absolute paths: `/Users/.../.cargo/bin/cargo`
- Some agents detect the no-op and try alternative approaches

The context-file approach is cleaner: the agent cooperates instead of being tricked.

### 3.3 The Audit Phase Trade-off

The runner supports a two-pass model:
1. **Implementation pass** (fast model, e.g. gpt-5.4-mini at xhigh reasoning)
2. **Audit pass** (stronger model verifies and fixes)

With audit enabled, each batch takes 2x the time but catches more issues.
`AUDIT_ENABLED=0` disables it — we ran the entire mega-parity run without audit
and deferred verification to a separate pass later.

**Lesson**: For large batch runs, disable audit. Do a manual audit pass afterwards
on the merged result. The compounding time savings are enormous.

### 3.4 Per-Batch Target Directories

When 15+ batches compile simultaneously, they all fight over the cargo target
directory lock. Two solutions:

1. **Shared target dir** (default): One target dir in `/tmp/roko-par-RUNNER/RUN_ID/`.
   Saves disk but causes lock contention.
2. **Per-batch target** (`PER_BATCH_TARGET=1`): Each batch gets its own target dir.
   No contention but uses 5-15GB per batch.

If builds are disabled via context file, this is irrelevant — agents don't compile.

### 3.5 Parallelism Tuning

`PARALLEL=15` was our sweet spot for 20 codex workers on a MacBook Pro.

Too high → API rate limits, disk exhaustion, git lock contention
Too low  → waves take forever, downstream batches starve

Rule of thumb: set PARALLEL to the number of concurrent API slots you can sustain,
minus a few for merge overhead.

---

## 4. Things That Go Wrong (and How to Fix Them)

### 4.1 Merge Conflicts

**Cause**: Two batches in the same wave modify the same file.

**What happens**: First batch merges fine. Second batch's merge fails because its
base (the wave-start commit) doesn't include the first batch's changes.

**Result**: `merge_failed` status, branch preserved, worktree preserved.

**Fix options**:
- The runner retries with `MAX_RETRIES=2` — the retry starts from the updated main branch
- Cherry-pick manually: `git cherry-pick <batch_hash>`
- Use `--theirs` for auto-resolution: `git checkout --theirs <file> && git add <file>`

**Prevention**: Put batches touching the same files in different groups with
dependencies. If R2_A01 and R2_A02 both touch `orchestrate.rs`, make A02 depend on A01.

### 4.2 Anti-Pattern False Positives (AP-10)

**Cause**: AP-10 checks for hardcoded `localhost` strings. But some files
legitimately contain `localhost` (e.g., default configs, documentation).

**What happens**: Batch code is correct, but AP check fails. Both retry attempts
fail identically. Status: `spawn_failed` (misleading — should be `antipattern_failed`).

**Fix**: Mark the batch as `success` manually:
```bash
echo "success" > tmp/runners/RUNNER/logs/RUN_ID/BATCH.result
```

The code is preserved on a backup branch: `codex/RUNNER-RUN_ID-BATCH-backup-TIMESTAMP`.
Cherry-pick from there.

**Lesson**: Anti-pattern checks should have per-batch or per-file exemptions.
A blanket regex check across all files is too crude.

### 4.3 Worktree Ghosts (Spawn Failed in 0s)

**Cause**: A worktree from a prior run exists, but the branch it was on was
deleted or moved. Codex fails immediately with "No such file or directory."

**What happens**: `spawn_failed` status, 0-second duration, no meaningful output.

**Fix**:
```bash
git worktree remove --force .roko/worktrees/RUNNER-RUN_ID-BATCH
git branch -D codex/RUNNER-RUN_ID-BATCH
rm tmp/runners/RUNNER/logs/RUN_ID/BATCH.result
# Then restart the runner with --continue
```

**The runner now handles this** with aggressive pre-cleanup (prune + delete branch
before creating worktree), but edge cases remain when branches are manually
manipulated during a run.

### 4.4 Blocked Batches (Dependency Cascade)

**Cause**: Batch X depends on Batch Y, and Y failed (any terminal status).
X is marked `blocked` and never runs.

**What happens**: Entire dependency chains go dark. If R2_D01 fails, R2_D05
(depends on D01) and R6_A03 (depends on D01) and R3_D02 (depends on D05) all block.

**Fix**: If Y's code is actually fine (e.g., AP-10 false positive), mark Y as success:
```bash
echo "success" > tmp/runners/RUNNER/logs/RUN_ID/Y.result
```
Then restart with `--continue` — the scheduler will pick up the unblocked batches.

**Lesson**: The DAG scheduler checks results on each wave iteration. You can
manipulate `.result` files at any time and the runner will react. This is the
primary manual intervention mechanism.

### 4.5 Runner Process Dies

**Cause**: Usually `set -euo pipefail` combined with an unexpected error in a
cleanup/post-processing path. Common triggers:
- Git branch deletion fails (branch already deleted by previous cleanup)
- Worktree operations fail (filesystem permission, stale refs)
- Disk full during merge

**What happens**: The runner exits, preserving all worktrees. The EXIT trap runs,
printing which worktrees are preserved.

**Fix**:
1. Check the log (`/tmp/mega-parity-runN.log`) for the last error
2. Clean up any stale worktrees/branches that caused the error
3. Restart with `--continue` — it reads `.result` files and skips completed batches

**Lesson**: The `--continue` flag is your lifeline. It resumes from disk state,
not memory state. You can kill and restart the runner freely.

### 4.6 Disk Exhaustion

**Cause**: Cargo builds generate 5-15GB per target directory. With shared target,
incremental builds accumulate. With per-batch targets, it multiplies.

**What happens**: macOS runs out of space, processes fail with I/O errors, git
operations fail silently, and things get weird.

**How much space does this take?**
- Shared target dir: 3-33GB (grows with each wave)
- Each worktree: ~500MB (just source, no target)
- Incremental build cache: grows indefinitely

**Fix**: Periodic cleanup during runs:
```bash
# Clean incremental build cache (safe during pauses between waves)
find /tmp/roko-par-RUNNER/RUN_ID -name "incremental" -type d -exec rm -rf {} +
# Clean old .rlib files
find /tmp/roko-par-RUNNER/RUN_ID -name "*.rlib" -mmin +60 -delete
# Nuclear option: delete entire target dir (forces full rebuild)
rm -rf /tmp/roko-par-RUNNER/RUN_ID
```

The runner has a `check_disk_space` function that triggers cleanup before each wave,
but it's conservative (2GB threshold). Real-world Rust builds can consume 30GB+.

**Lesson**: Monitor `df -h /` every few minutes during runs. Set up a cron or
background script that cleans incremental caches. If builds are disabled (no-build
context), this is less of an issue, but agents that ignore the instruction will
still build.

### 4.7 Agents Ignore Instructions

**Cause**: LLMs are probabilistic. A context-file saying "don't run cargo" works
95% of the time. Some agents still run `rustup run stable cargo test` or try to
download dependencies.

**What happens**: Batch takes 30 minutes instead of 3. Disk fills up. Other batches
starve for API quota.

**Detection**: Watch for batches taking >10 minutes in no-build mode.
```bash
# Find stale in-progress batches
for f in logs/RUN_ID/*.result; do
  [[ "$(cat "$f")" == "in_progress" ]] || continue
  batch=$(basename "$f" .result)
  started=$(grep "Started:" logs/RUN_ID/${batch}.log | tail -1)
  echo "$batch: $started"
done
```

**Fix**: Kill the codex process manually:
```bash
pkill -f "mega-parity-run-RUNID-BATCHID"
echo "success" > logs/RUN_ID/BATCH.result  # if code is written
# or
rm logs/RUN_ID/BATCH.result  # to retry
```

---

## 5. The Cherry-Pick Workflow

When running with `--no-merge-back`, all work accumulates on the runner branch.
You need to get it into your source branch somehow.

### 5.1 Manual Cherry-Pick

```bash
# See what's on the runner branch but not your branch
git log --oneline --no-merges codex/RUNNER-RUNID --not source-branch

# Cherry-pick non-merge commits (oldest first)
git log --oneline --no-merges --reverse codex/RUNNER-RUNID --not source-branch \
  | awk '{print $1}' | while read sha; do
    git cherry-pick --no-commit "$sha" && \
    git commit --no-verify -m "$(git log --oneline -1 $sha | cut -d' ' -f2-)" || \
    { git checkout --theirs . && git add -A && \
      git commit --no-verify -m "$(git log --oneline -1 $sha | cut -d' ' -f2-)"; }
done
```

### 5.2 Auto-Pick Script

The runner includes `lib/auto-pick.sh` — a background monitor that:
1. Watches for batches transitioning to success
2. Cherry-picks them into the target branch
3. Auto-resolves conflicts (accepts `--theirs`)
4. Validates with `cargo check` after each cycle
5. Saves state to disk (survives restart)

```bash
# Run in a separate terminal
bash lib/auto-pick.sh --target-branch wp-arch2 --interval 90 --no-check
```

### 5.3 Conflict Resolution Strategy

When cherry-picking, conflicts happen because:
- Your source branch has changes the runner branch doesn't know about
- Multiple batches modified the same code differently

Resolution priority:
1. **Accept theirs** for batch implementation files — the batch's code is new work
2. **Accept ours** for configuration/infrastructure files — your branch has the latest
3. **Manual merge** when both sides have meaningful changes

**Lesson**: About 30% of cherry-picks will conflict in a large run. Automated
`--theirs` resolution works for most. The remaining 10-15% need manual inspection.

---

## 6. Verification and Auditing

### 6.1 When to Verify

There are three verification points:

1. **Per-batch** (disabled via no-build context): Agent runs `cargo check -p <crate>`
2. **Wave gates** (disabled via `--no-gate`): `cargo check --workspace + clippy` between waves
3. **End-of-run test gate** (disabled via `--no-test`): Full `cargo test --workspace`

For maximum speed: disable all three, batch all verification to the end.

### 6.2 Post-Run Verification

After all batches complete and are cherry-picked:

```bash
# On your branch with all cherry-picks applied
cargo check --workspace                                    # does it compile?
cargo clippy --workspace --no-deps -- -D warnings          # is it clean?
cargo test --workspace                                     # do tests pass?
```

**Reality**: After 195 batches with no-build mode, expect:
- 10-30 compile errors (conflicting types, missing imports, moved functions)
- 5-15 clippy warnings (unused variables, redundant clones)
- 3-10 test failures (changed APIs not reflected in tests)

This sounds bad but it's ~30 minutes of cleanup vs ~50 hours of per-batch compilation.

### 6.3 The Anti-Pattern Checks

Fast, no-cargo checks that catch common LLM mistakes:

| Check | What it catches |
|---|---|
| AP-1 | Stub gates that return pass (silent-pass) |
| AP-2 | `block_on` in async code |
| AP-3 | Duplicate trait definitions vs foundation.rs |
| AP-5 | Raw `Command::new("claude")` shell-outs |
| AP-6 | Inline prompt strings (`format!("You are a...")`) |
| AP-7 | std::sync::Mutex held across .await |
| AP-8 | Empty function bodies |
| AP-9 | unimplemented!/unreachable! left behind |
| AP-10 | Hardcoded localhost/port in non-test code |

These are grep-based, run in milliseconds, and catch the most common LLM code-gen
mistakes. They run per-batch by default and are the primary quality gate when
compilation is disabled.

**Lesson**: AP checks are worth keeping even when compilation is disabled.
They catch structural mistakes that compilation won't find.

---

## 7. Operational Procedures

### 7.1 Starting a Run

```bash
cd tmp/runners/mega-parity
# Full run with all gates
bash run.sh

# Fast run — no compilation at all
bash run.sh --no-gate --no-test --no-audit --no-merge-back --parallel 15

# Resume after crash
bash run.sh --continue --no-merge-back --parallel 15

# Only specific batches
bash run.sh --only R5_B03,R5_B04,R5_C01 --continue --no-merge-back

# Dry run — show wave plan without executing
bash run.sh --dry-run
```

### 7.2 Monitoring

```bash
# Live dashboard (in separate terminal)
bash run.sh --watch

# Tail all logs
bash run.sh --tail

# Specific batch log
tail -f logs/RUN_ID/R5_B03.log

# Quick status
cat logs/RUN_ID/status.json | python3 -m json.tool

# Manual status check
for f in logs/RUN_ID/*.result; do echo "$(basename $f .result): $(cat $f)"; done | sort
```

### 7.3 Manual Intervention Playbook

| Situation | Action |
|---|---|
| Batch stuck >10 min | `pkill -f "RUNID-BATCHID"` then decide: mark success or rm result |
| False-positive AP fail | `echo success > logs/RUN_ID/BATCH.result` |
| Blocked cascade | Fix upstream result file, restart with `--continue` |
| Runner died | Check log, fix cause, `--continue` |
| Disk full | Clean incremental: `find /tmp/roko-par-* -name incremental -exec rm -rf {} +` |
| Merge conflict in runner | Branch preserved — cherry-pick manually later |
| Agent building despite no-build | Kill process, mark result, add stronger no-build instruction |
| Need to skip a batch | `echo success_noop > logs/RUN_ID/BATCH.result` |

### 7.4 Post-Run Integration

```bash
# 1. Cherry-pick from runner branch
git log --oneline --no-merges --reverse codex/RUNNER-RUNID --not HEAD

# 2. Or run auto-pick
bash lib/auto-pick.sh --target-branch wp-arch2 --no-check

# 3. Compile check
cargo check --workspace 2>&1 | grep '^error' | head -20

# 4. Fix errors (often 10-30 after a large run)
# Most common: type mismatches, missing imports, moved functions

# 5. Clippy
cargo clippy --workspace --no-deps -- -D warnings

# 6. Tests
cargo test --workspace
```

---

## 8. Design Decisions and Trade-offs

### 8.1 Why Worktrees, Not Branches

Branches alone don't give you a working tree. Worktrees give each batch a full
checkout where the agent can read, write, and (optionally) compile independently.
The downside: each worktree is ~500MB for a large repo.

### 8.2 Why Serialized Merges

Merges to the runner branch are serialized with a mkdir-based lock. This prevents
concurrent `git merge` operations from corrupting the repo. The lock has stale
detection (checks if the holder PID is alive).

### 8.3 Why Wave Gates, Not Per-Batch

Per-batch compilation takes 5-15 minutes and often fails because the batch is
editing a file that another batch also edited — cargo check sees incomplete work
from the other batch via the shared codebase.

Wave gates wait until all batches in a wave are merged, then compile the integrated
result. This catches real errors (cross-batch type mismatches) without false positives
from mid-wave partial states.

### 8.4 Why `--no-merge-back` in Practice

Auto merge-back is elegant in theory: the runner automatically merges work into
your source branch at group boundaries. In practice:

- Your source branch has active development happening
- The merge creates conflicts that need human judgment
- Failed auto-merges leave the repo in a weird state
- Manual cherry-pick gives you control over what lands and when

We always run with `--no-merge-back` and cherry-pick manually or via auto-pick.

### 8.5 Why Context Packs, Not Fine-Tuning

Each batch gets the same set of context files (`00-RULES.md`, `01-ARCHITECTURE.md`,
etc.) prepended to its prompt. This is crude but effective:

- Rules are explicit and auditable
- Easy to update between runs
- No model-specific training needed
- Same context pack works for any LLM backend

The downside: ~4000 tokens of context overhead per batch. With 195 batches,
that's ~780K tokens just in shared context. But at $0.15/million input tokens
(gpt-5.4-mini), that's about $0.12 total — negligible.

### 8.6 Retries and Backup Branches

When a batch fails and is retried:
1. Uncommitted work is committed to a backup branch (`...-backup-TIMESTAMP`)
2. The worktree is reset to the current runner branch head
3. The retry prompt includes the failure context from the previous attempt
4. A fresh codex process runs with the updated context

Backup branches are **never deleted**. They're the safety net — if a retry
produces worse code, you can always cherry-pick from the backup.

---

## 9. What Roko Should Learn From This

### 9.1 The Agent Orchestration Loop

This runner IS the roko self-hosting loop, in bash form:

```
plan (batches.toml)  →  dispatch (codex exec)  →  gate (AP + cargo)
    ↑                                                    ↓
    └──── replan (retry with failure context) ←── persist (merge + result)
```

Roko's `orchestrate.rs` does the same thing: plan → dispatch → gate → persist → learn.
The runner validates that this loop works at scale.

### 9.2 Key Insights for Agent-Centric Development

1. **Isolation is non-negotiable.** Agents must work in separate worktrees/sandboxes.
   Shared mutable state between concurrent agents is a recipe for corruption.

2. **Context handoff is the hard problem.** Telling agent B what agent A changed
   (the cumulative section) is more important than telling agent B what to do.
   Bad context → merge conflicts → wasted work → cascade failures.

3. **Gates should be batched, not per-agent.** Compiling after every agent turn
   is too expensive. Compile after a batch of changes accumulates. The trade-off
   is delayed error detection, but the time savings are 10-100x.

4. **Result files are the coordination mechanism.** Not message passing, not shared
   memory — simple files on disk that say "success" or "failed." Any process can
   read them, any process can write them. This enables manual intervention at any
   point.

5. **Never delete branches or worktrees automatically.** They are your undo
   mechanism. Disk is cheap; lost work is expensive.

6. **The `--continue` pattern is essential.** Any long-running process will crash.
   The ability to resume from disk state (not memory state) is what makes the system
   reliable. Every agent system needs this.

7. **Manual intervention is a feature, not a bug.** The system should make it easy
   for a human to: read status, mark things as done, unblock dependencies, kill
   stalled processes, and restart. Fully autonomous is a goal; human-in-the-loop
   is the reality.

8. **The auto-pick pattern (background cherry-picker) is powerful.** Having a
   separate process that watches for completed work and integrates it into your
   branch means you can keep working while agents generate code. This is the
   "conveyor belt" pattern — agents produce, the picker integrates, you review.

### 9.3 Numbers to Remember

| Metric | Value | Notes |
|---|---|---|
| Batch time (with builds) | 15-40 min | Agent writes code + compiles + lints |
| Batch time (no builds) | 1-5 min | Agent writes code only |
| Wave gate (cargo check) | 3-8 min | Full workspace compile |
| Cherry-pick (manual) | ~30 sec each | With auto-resolve |
| Target dir size | 3-33 GB | Grows with incremental builds |
| Worktree size | ~500 MB each | Source only, no target |
| Merge conflict rate | ~30% of cherry-picks | In a large run with many shared files |
| AP false positive rate | ~2-3% of batches | Mostly AP-10 (hardcoded localhost) |
| Agent instruction compliance | ~95% | 5% ignore no-build instructions |
| Disk overhead per batch | 500 MB - 15 GB | Depends on whether builds run |

---

## 10. Configuration Reference

### 10.1 Environment Variables

| Variable | Default | What |
|---|---|---|
| `CONV_MODEL` | gpt-5.4-mini | Implementation model |
| `CONV_REASONING` | xhigh | Reasoning effort level |
| `AUDIT_MODEL` | gpt-5.4-mini | Audit model (if enabled) |
| `AUDIT_ENABLED` | 0 | Enable/disable audit pass |
| `PARALLEL` | 3 | Max concurrent batches |
| `MAX_RETRIES` | 2 | Retry attempts per batch |
| `CONV_TIMEOUT` | 5400 | Timeout per batch (seconds) |
| `PER_BATCH_TARGET` | 0 | Separate cargo target per batch |
| `SKIP_AP_CHECKS` | 0 | Skip anti-pattern checks |

### 10.2 CLI Flags

| Flag | What |
|---|---|
| `--no-gate` | Skip wave gates (cargo check between waves) |
| `--no-test` | Skip end-of-run test gate |
| `--no-audit` | Skip audit pass |
| `--no-merge-back` | Don't auto-merge to source branch |
| `--continue` | Resume from last run's state |
| `--only A,B,C` | Run only these batch IDs |
| `--parallel N` | Max concurrent batches |
| `--pause` | Pause between waves for inspection |
| `--dry-run` | Show wave plan without executing |
| `--watch` | Live dashboard |
| `--list` | List batches and exit |

### 10.3 File Layout

```
tmp/runners/mega-parity/
  run.sh                    ← entry point (sets model, parallelism)
  batches.toml              ← DAG definition
  prompts/                  ← per-batch prompt files
    R2_A01.prompt.md
    R2_A02.prompt.md
    ...
  context-pack/             ← shared context prepended to every prompt
    00-RULES.md
    01-ARCHITECTURE-TARGET.md
    02-CRATE-MAP.md
    03-REVIEW-VETOES.md
    04-PERF-CONTRACTS.md
    05-NO-BUILD.md           ← added to disable builds
  logs/
    run-YYYYMMDD-HHMMSS/    ← per-run directory
      R2_A01.log             ← batch log (codex output)
      R2_A01.result          ← batch status (success|failed|in_progress|...)
      R2_A01.result.hash     ← commit hash for cherry-pick
      events.jsonl           ← structured event log
      status.json            ← live status for dashboard
      manifest.env           ← run metadata
      gate-GROUP.log         ← wave gate output
  bin/                       ← (optional) fake binaries
```

---

## 11. Common Failure Modes Summary

| Failure | Symptom | Root Cause | Fix |
|---|---|---|---|
| `spawn_failed` (0s) | Codex exits immediately | Stale worktree/branch | Remove worktree + branch, clear result |
| `spawn_failed` (>0s) | Codex exits with error | API error, bad prompt, config issue | Check batch log |
| `antipattern_failed` | AP check blocks merge | False positive or real issue | Check AP log, exempt or fix |
| `merge_failed` | Branch can't merge | Parallel batches touched same file | Cherry-pick manually |
| `merge_conflict` | Same as above | Same | Cherry-pick from batch branch |
| `timeout` | Batch exceeds CONV_TIMEOUT | Agent running builds, stuck in loop | Kill process, adjust timeout |
| `blocked` | Dependency failed | Upstream batch failed | Fix upstream result file |
| `verify_failed` | Audit pass failed | Code doesn't pass audit checks | Inspect audit log |
| Runner died | Process exits unexpectedly | `set -e` + unexpected error | Check log, `--continue` |
| Disk full | I/O errors, weird failures | Cargo build cache | Clean incremental + .rlib files |

---

## 12. What We'd Do Differently Next Time

1. **Start with no-build context from the beginning.** We wasted hours on per-batch
   compilation before realizing it was unnecessary for the implementation phase.

2. **Use per-batch target directories if builds are needed.** Shared targets cause
   lock contention and cache invalidation across batches.

3. **Design batches for smaller file scope.** Batches that touch >3 files in the
   same crate have high merge conflict rates. Break them down.

4. **Run auto-pick from the start.** Having a background process that integrates
   completed work into your branch means you can start reviewing code while the
   runner is still going.

5. **Pre-validate the DAG.** Use `--dry-run` to see wave structure before committing
   to a multi-hour run. Catch dependency errors early.

6. **Set up disk monitoring as a cron job.** Relying on the runner's built-in check
   (2GB threshold) is insufficient for Rust workspaces.

7. **Exempt known false-positive AP checks per batch.** AP-10 blocked 3 batches
   in every run. A simple exemption in `batches.toml` would have saved hours.

8. **Log agent model calls separately from agent output.** When debugging "why did
   this batch take 30 minutes," you need to see individual model calls, not just
   the final output.

---

## 13. The Methodology: From Information to Mechanical Tasks

This section describes the general process for turning a vague goal ("make the UI
better" or "reach parity with the old system") into hundreds of tasks that fast,
cheap, context-free agents can execute reliably. This is the methodology the runner
validates — the same loop roko uses to develop itself.

### 13.1 Phase 0: Information Gathering (The Giant Folder)

Before writing a single line of implementation, **assimilate everything** into a
folder of structured documents. The more information the better. You're building
the context that every downstream agent will consume.

What to gather:

| Document Type | What Goes In | Example |
|---|---|---|
| **Current State Inventory** | Every file, every component, every hook, every API route. Line counts, import counts, what works, what's broken. | `01-CURRENT-STATE.md` |
| **Codebase Audit** | Dead code, memory leaks, bugs, duplicated utilities, CSS conflicts, a11y gaps. Per-file, with line numbers. | `08-AUDIT-FINDINGS.md` |
| **Architecture Target** | What the system SHOULD look like. Data flow diagrams, component hierarchy, state management. | `02-ARCHITECTURE.md` |
| **Design System / Tokens** | Colors, spacing, typography, animation, visual language. Concrete CSS values, not vibes. | `04-DESIGN-SYSTEM.md` |
| **Anti-Patterns** | Things that went wrong. Explicit prohibitions. "Do NOT do X because Y happened last time." | `00-RULES.md`, `10-UX-PHILOSOPHY.md` |
| **Bugs & Known Issues** | Every known bug with reproduction steps, root cause analysis, affected files. | `08-AUDIT-FINDINGS.md` section 2-3 |
| **Reference Implementations** | Code from the old system, working examples, prior art. With file paths and line numbers. | `solution-ACTUAL.md` |
| **Performance Contracts** | Hard limits: max response time, max memory, max bundle size, etc. | `04-PERF-CONTRACTS.md` |
| **Page/Feature Specs** | Per-page descriptions with wireframes, data requirements, interaction flows. | `05-PAGES.md` |
| **UX Philosophy** | Core principles, visual targets, methodology, "what NOT to build" lists. | `10-UX-PHILOSOPHY.md` |
| **Delta Checklist** | Gap analysis: what current state needs to match target state, organized by category. | `11-CURRENT-DELTA-CHECKLIST.md` |

**Key insight**: This phase is done by strong models (Opus, Sonnet) or humans.
It requires judgment, synthesis, and cross-referencing. Cheap models can't do it.
The output is 50-200KB of structured markdown that becomes the ground truth for
everything downstream.

**Example**: For the demo UI, we produced 11 documents totaling ~380KB:
- Complete inventory of 100+ source files with line-by-line audit
- 813 lines of identified dead code with exact file paths
- 2 confirmed memory leaks with root cause and fix
- 7 critical bugs with reproduction steps
- 6 duplicated utility functions with all copy locations
- 130 hardcoded colors cataloged
- 35+ composable primitive specs
- 75 implementation tasks

The information gathering phase took ~4 hours but saved ~40 hours of agent confusion,
re-discovery, and rework downstream.

### 13.2 Phase 1: Architecture & Design Documents

From the raw information, produce **prescriptive** documents that describe system
properties, design patterns, and features:

1. **Architecture document** — How subsystems compose. What depends on what.
   Data flow direction. State ownership. Naming conventions.

2. **Design primitives catalog** — Every reusable building block with props
   interface, CSS tokens, composition rules, and density guidelines.

3. **Migration plan** — How to get from current state to target state without
   breaking things. Which pieces can be swapped independently.

4. **Contract documents** — Performance budgets, anti-pattern rules, type
   constraints. These become the context-pack for agents.

These are still written by strong models or humans. They require architectural
judgment that cheap models lack.

### 13.3 Phase 2: First Pass — Implementation Plan

Break the architecture into **phases** and **tasks**:

```
Phase 0: Cleanup & Dead Code Removal    ← safe, non-breaking
Phase 1: Foundation (data layer, transport)  ← architectural bedrock
Phase 2: Core Components (design system)     ← depends on Phase 1
Phase 3: Feature Migration              ← depends on Phase 2
Phase 4: Integration & Polish           ← depends on Phase 3
Phase 5: Verification & Testing         ← depends on Phase 4
```

Each phase has **5-15 tasks**. Each task at this stage has:
- A title describing the outcome
- The files it touches (scope)
- Its dependencies (what must complete first)
- A 3-5 line description
- An acceptance criterion

This is the `batches.toml` + `prompts/` structure the runner consumes.

**Critical rule**: Tasks must have **isolated context**. An agent working on
task T3.4 should not need to understand tasks T1.1-T3.3. It should need only:
- The shared context pack (rules, architecture, design tokens)
- The cumulative section (what changed in files it will edit)
- Its own prompt
- The current file contents

If a task requires understanding 10 other tasks to implement, it's not broken
down enough.

### 13.4 Phase 3: Second Pass — Mechanical Decomposition

This is where tasks go from "implement the data layer" to step-by-step recipes.
The goal: **any model, no matter how dumb, can follow these instructions.**

Before (first pass):
```markdown
### T1.3: Create DataHub store
- Create Zustand store with slices for agents, workflows, events
- Include actions for SSE/WS subscriptions
- Export typed selectors
```

After (mechanical decomposition):
```markdown
### T1.3: Create `lib/dataHub.ts` — Zustand store with typed slices

**File**: `src/lib/dataHub.ts` (new file)

**Step 1**: Create the file with these exact imports:
```typescript
import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
```

**Step 2**: Define the state interface:
```typescript
interface DataHubState {
  agents: Map<string, AgentRecord>;
  workflows: Map<string, WorkflowRecord>;
  events: EventRecord[];
  transport: TransportState;
  // Actions
  upsertAgent: (id: string, data: Partial<AgentRecord>) => void;
  pushEvent: (event: EventRecord) => void;
  setTransportStatus: (status: 'connecting' | 'connected' | 'error') => void;
}
```

**Step 3**: Implement the store:
```typescript
export const useDataHub = create<DataHubState>()(
  subscribeWithSelector((set, get) => ({
    agents: new Map(),
    workflows: new Map(),
    events: [],
    transport: { status: 'disconnected', lastEvent: null },
    upsertAgent: (id, data) => set(state => {
      const agents = new Map(state.agents);
      agents.set(id, { ...agents.get(id), ...data } as AgentRecord);
      return { agents };
    }),
    pushEvent: (event) => set(state => ({
      events: [...state.events.slice(-499), event]
    })),
    setTransportStatus: (status) => set(state => ({
      transport: { ...state.transport, status }
    })),
  }))
);
```

**Step 4**: Export typed selectors:
```typescript
export const selectAgents = (s: DataHubState) => s.agents;
export const selectWorkflows = (s: DataHubState) => s.workflows;
export const selectEvents = (s: DataHubState) => s.events;
```

**Verify**:
- `npx tsc --noEmit` passes
- `import { useDataHub } from './lib/dataHub'` resolves
- `useDataHub.getState().agents` is `Map<string, AgentRecord>`
```

The mechanical version is 5x longer but requires zero judgment to execute.
A model just copies the code, adjusts imports, and verifies.

### 13.5 What Makes a Task "Mechanical Enough"

A task is ready for cheap/fast models when it has ALL of:

| Property | Bad Example | Good Example |
|---|---|---|
| **Exact file path** | "Create a data layer" | "Create `src/lib/dataHub.ts`" |
| **Exact imports** | "Import necessary deps" | "import { create } from 'zustand'" |
| **Exact types** | "Define the state shape" | Full TypeScript interface with every field |
| **Exact code pattern** | "Use Zustand" | Complete store definition with all slices |
| **Exact verification** | "It should work" | "`npx tsc --noEmit` passes, `grep -rn 'import.*dataHub' src/` returns 3+ results" |
| **No ambiguity** | "Handle errors appropriately" | "Wrap in try/catch, set `error` state to `e.message`" |
| **No architectural decisions** | "Choose the best approach" | "Use `subscribeWithSelector` middleware" |
| **Isolated scope** | "Integrate with the rest" | "Only modify `src/lib/dataHub.ts` and `src/App.tsx` line 12" |

### 13.6 Context Packs: Shared Knowledge for Dumb Agents

Every agent gets the same context pack prepended to its prompt. This is the
"things you need to know but aren't in your task description" document set.

Typical context pack (~4000-8000 tokens):

| File | Purpose | Tokens |
|---|---|---|
| `00-RULES.md` | Anti-patterns, do-not-repeat mistakes, contract rules | ~1500 |
| `01-ARCHITECTURE.md` | Target architecture summary, key patterns | ~800 |
| `02-CRATE-MAP.md` | Which crate does what, import paths | ~500 |
| `03-REVIEW-VETOES.md` | Hard vetoes from code review | ~400 |
| `04-PERF-CONTRACTS.md` | Performance rules all code must respect | ~300 |
| `05-NO-BUILD.md` | Build policy (skip compilation) | ~100 |

**Design rules for context packs**:
- Keep under 8000 tokens total (model context is precious)
- Anti-patterns > architecture > crate map (priority order)
- Include **existing** anti-patterns with file:line citations
- Include "do NOT" sections explicitly — models need negative examples
- Update between runs based on what went wrong

### 13.7 Verification, Acceptance Criteria, and Gates

Every task needs a machine-verifiable acceptance criterion. Not "it should work"
but a specific command that returns a specific result.

**Levels of verification (cheapest to most expensive)**:

| Level | Command | Time | What It Catches |
|---|---|---|---|
| 1. File exists | `test -f src/lib/dataHub.ts` | 0.001s | File not created |
| 2. Grep check | `grep -q 'export const useDataHub' src/lib/dataHub.ts` | 0.01s | Function not exported |
| 3. TypeScript check | `npx tsc --noEmit` | 5-30s | Type errors |
| 4. Lint | `npx eslint src/lib/dataHub.ts` | 2-10s | Style violations |
| 5. Unit test | `npx vitest run dataHub.test` | 5-30s | Logic errors |
| 6. Cargo check | `cargo check -p <crate>` | 30-300s | Compile errors (Rust) |
| 7. Cargo clippy | `cargo clippy -p <crate> -- -D warnings` | 30-300s | Lint (Rust) |
| 8. Integration test | `cargo test -p <crate>` | 60-600s | Regression |

**For fast runs**: Use levels 1-2 per task, level 6-7 at wave boundaries.
**For thorough runs**: Use levels 1-7 per task, level 8 at the end.

Anti-pattern checks (AP-1 through AP-10) are level-2 verifications that catch
common LLM mistakes without compilation. They run in milliseconds and are always
worth keeping.

### 13.8 The Model Selection Hierarchy

Different phases require different model capabilities:

| Phase | Model Class | Why |
|---|---|---|
| Information gathering | Opus / Sonnet / Human | Requires judgment, synthesis |
| Architecture design | Opus / Sonnet | Requires system-level thinking |
| First-pass task breakdown | Sonnet / Haiku | Can follow patterns, generate structure |
| Mechanical decomposition | Sonnet | Needs to read code and produce exact specs |
| Task execution | Mini / Haiku / any fast model | Just follows instructions |
| Verification | N/A (automated) | Shell commands, no model needed |
| Audit / review | Opus / Sonnet | Requires judgment about correctness |

**Cost optimization**: The expensive models do ~10% of the work (planning) and the
cheap models do ~90% (execution). A run of 195 batches at gpt-5.4-mini costs
roughly $3-5. The same at Opus would cost $150-300. Planning + execution at
mixed tiers costs $10-15 total.

### 13.9 Dealing with Dependencies Between Tasks

Three strategies for task ordering:

1. **Linear chains** — T1 → T2 → T3. Simple, safe, slow. Use when each task
   modifies the output of the previous one.

2. **Wide waves** — T1, T2, T3 all parallel, then T4, T5, T6 all parallel.
   Fast, but merge conflicts when tasks touch same files. Use when tasks touch
   different files.

3. **DAG** — T1 → T3, T2 → T3, T1 → T4, T2 → T5. Maximum parallelism with
   minimum conflicts. This is what the runner uses.

**How to design the DAG**:
- Group tasks by the files they touch (the "scope")
- Tasks with overlapping scope should be in a dependency chain
- Tasks with disjoint scope can be parallel
- When in doubt, add a dependency — wasted parallelism is better than merge conflicts

**Scope specification in batches.toml**:
```toml
[[batch]]
id = "R5_C01"
scope = ["crates/roko-cli/src/orchestrate.rs"]
also_read = ["crates/roko-learn/src/lib.rs"]
deps = ["R5_B02", "R5_A04"]
```

`scope` = files the batch will modify (used for merge conflict prediction)
`also_read` = files the batch needs to see but won't modify
`deps` = batches that must complete first (for correctness, not just conflict avoidance)

### 13.10 The Cumulative Context Problem

When batch B depends on batch A, B needs to know what A changed. The runner
handles this with a **cumulative section** — a snapshot of modified files taken
after each merge. This section is included in subsequent batch prompts.

For large files (>200 lines), only signatures are included:
```
### `orchestrate.rs` (modified by R5_A04, 18000 lines — signatures only)
```rust
pub fn emit_efficiency_event(...) { ... }
pub struct TaskTracker { ... }
```
```

For small files, full contents are included.

**Lesson**: The cumulative section is the most important part of the prompt for
dependent batches. Without it, agents write code that conflicts with work that
was just done 5 minutes ago by a sibling batch.

**Failure mode**: If the cumulative section is stale (not updated after a merge),
the next batch sees old code and writes incompatible changes. The merge then fails,
and the retry also fails because it still sees stale context.

### 13.11 The Retry Prompt Pattern

When a batch fails, the retry prompt includes the failure context:

```markdown
## Previous attempt failed

### Last 50 lines of log:
```
error[E0308]: mismatched types
 --> crates/roko-cli/src/orchestrate.rs:11219:42
   |
   expected `Option<String>`, found `String`
```

Fix the issues above. Do not repeat the same mistakes.
```

This gives the retrying agent specific information about what went wrong.
The agent starts from a fresh worktree (reset to current main branch head),
so it has the cumulative context from all successful merges plus the error
information from the failed attempt.

### 13.12 How Things Get Stuck and How to Unstick Them

| Stuck Pattern | Detection | Manual Fix |
|---|---|---|
| **False-positive AP check** | Batch fails AP-10 but code is correct | `echo success > BATCH.result` |
| **Dependency cascade** | 5+ batches show `blocked` | Fix upstream result file |
| **Agent building despite no-build** | Batch running >10 min | Kill codex process, mark result |
| **Stale worktree** | `spawn_failed` in 0s | Remove worktree + branch, clear result |
| **Runner process died** | No new log output | Check log, `--continue` |
| **Merge lock stuck** | All batches waiting to merge | Delete `merge_lock.d` directory |
| **Agent in infinite loop** | Log shows repeated attempts | Kill process, add failure context to retry |
| **Wrong base branch** | Merge conflicts on every batch | Rebuild main worktree from correct ref |
| **Context too large** | Agent hits token limit | Truncate cumulative section, reduce file includes |
| **Disk exhaustion** | I/O errors, git corruption | Clean target dirs, incremental caches |

**The universal fix**: Kill → clean → restart with `--continue`. The runner's
state is entirely on disk (result files, worktrees, branches). You can manipulate
any of it between runs.

---

## 14. Broader Patterns for Agent-Centric Development

### 14.1 The Document-First Workflow

```
Observe → Document → Plan → Break Down → Mechanicalize → Execute → Verify → Learn
```

Most agent frameworks skip straight to Execute. The document-first approach spends
60% of the time in Observe/Document/Plan and 40% in Execute. This seems slower
but produces 3-5x fewer failed tasks.

### 14.2 The "Giant Folder" Pattern

Create a folder (`tmp/solutions/thing/`) with 5-15 documents that collectively
describe everything about a subsystem:

```
tmp/solutions/demo-ui2/
  00-INDEX.md           ← table of contents
  01-CURRENT-STATE.md   ← what exists
  02-ARCHITECTURE.md    ← what should exist
  03-REALTIME-DATA.md   ← specific subsystem spec
  04-DESIGN-SYSTEM.md   ← design tokens, visual language
  05-PAGES.md           ← per-page specs
  06-AGENT-MODEL.md     ← agent behavior model
  07-IMPLEMENTATION.md  ← phased task list
  08-AUDIT-FINDINGS.md  ← bugs, dead code, issues
  09-DESIGN-PRIMITIVES.md ← component catalog
  10-UX-PHILOSOPHY.md   ← principles, anti-patterns
  11-CURRENT-DELTA.md   ← gap analysis (current vs target)
```

This folder is the **single source of truth**. All agents consume from it.
All humans review it. When something changes, one document is updated and all
downstream agents see the change via the context pack.

### 14.3 Features of Effective Implementation Plans

The best implementation plans we've seen share these features:

1. **Bugs listed first** — Known issues before new features. Agents are better at
   fixing specific bugs than implementing vague features.

2. **"Do NOT" sections** — Explicit prohibitions. "Do NOT create a new crate for
   this." "Do NOT use raw HTTP when an adapter exists." LLMs need negative examples
   more than positive ones.

3. **Citations to existing code** — "See `orchestrate.rs:11219` for the current
   implementation." Not "somewhere in the codebase." Line numbers give agents an
   anchor point.

4. **Acceptance criteria as shell commands** — Not "it should work" but
   `grep -q 'pub fn emit_efficiency' crates/roko-cli/src/orchestrate.rs`.

5. **Scope boundaries** — "Only modify these 2 files." Prevents agents from
   "helpfully" refactoring 15 other files.

6. **Predecessor context** — "After R5_B02, the `attempt_id` field exists on
   `AgentEfficiencyEvent`." Tells the agent what to assume without reading 10
   other task definitions.

### 14.4 Approaches That Help

1. **Refactoring before new features.** Break out shared utilities, kill dead code,
   fix type errors. Clean code is 3x easier for agents to modify correctly.

2. **Abstract-first implementation.** Define interfaces/traits first (Phase 1),
   then implement (Phase 2), then wire (Phase 3). Each phase has smaller scope.

3. **Test-driven tasks.** Write the test first (task A), then implement to pass it
   (task B). The test IS the acceptance criterion.

4. **Crate-isolated tasks.** One task per crate. Agents that modify one crate
   can't break another. Merge conflicts stay local.

5. **Context budget management.** Prompts over 100K tokens degrade quality.
   Split large files into "current content" + "signature only" views.

6. **Prompt compression.** For very large codebases, show agents only the
   relevant sections of files. `head -200`, `tail -200`, or `grep -n 'pub '`
   for large files.

7. **Parallel audit pass.** After all implementation batches complete, run a
   separate set of audit batches that read the output and check for issues.
   Cheaper than per-batch audit, catches cross-batch inconsistencies.

### 14.5 Approaches That Don't Work

1. **"Just tell it what to do" without context.** Agents without anti-pattern
   lists will repeat every mistake in the codebase.

2. **Monolithic tasks.** "Implement the entire data layer" → agent produces
   garbage or times out. Break it down.

3. **Implicit dependencies.** "Just read the codebase" → agent reads wrong
   files, writes incompatible code.

4. **Per-task builds for large workspaces.** Each `cargo check --workspace`
   costs 3-8 minutes. With 195 tasks, that's 10-26 wasted hours.

5. **Shared state between parallel agents.** Two agents editing the same file
   simultaneously → merge conflicts every time.

6. **Fine-tuning instead of context packs.** Context packs are readable,
   auditable, and updatable. Fine-tuned models are black boxes.

7. **Trusting process success as correctness.** Agent exits 0 ≠ code is right.
   Always verify with structural checks (AP-1 through AP-10) at minimum.

### 14.6 The Speed vs. Quality Spectrum

```
Fastest                                              Most Correct
   ↓                                                      ↓
   No verification ← Context-only → AP checks → Wave gates → Per-batch build → Full test suite
   1-2 min/batch    1-5 min/batch   +0.1s/batch  +5 min/wave  +10 min/batch    +15 min/batch
```

Pick your point on this spectrum based on:
- **How critical is correctness?** Production code → rightward. Prototype → leftward.
- **How many batches?** 10 → per-batch builds are fine. 200 → wave gates only.
- **How expensive are failures?** If retry is cheap, go fast. If retry is expensive, be thorough.
- **How long is the merge pipeline?** If you'll compile everything at the end anyway, skip per-batch.

Our mega-parity run: 195 batches, no-build context, AP checks only, no wave gates.
Result: ~30 compile errors to fix at the end vs. ~40 hours saved. Worth it.

### 14.7 Future Ideas

Things we haven't tried but should work:

1. **Agent-to-agent review.** After batch A completes, spawn a review agent that
   reads A's diff and flags issues before merge. Cheaper than full audit.

2. **Speculative execution.** Start downstream batches optimistically before
   upstream completes. If upstream changes the assumed interface, kill and restart
   downstream. Works when upstream changes are usually small.

3. **Incremental context packs.** Instead of one giant context pack, have per-group
   context packs that only include relevant anti-patterns and architecture.

4. **Conflict prediction.** Analyze scope overlap between batches in a wave before
   dispatching. Defer high-overlap batches to the next wave.

5. **Cost-aware scheduling.** Track cost-per-batch and route expensive batches to
   cheaper models. Route simple batches (delete dead code, add imports) to the
   cheapest model available.

6. **Live compilation server.** Instead of per-batch `cargo check`, run a persistent
   `cargo watch` on the runner branch. Batches query it for compile status.

7. **Knowledge transfer between runs.** After a run, extract what agents learned
   (common errors, useful patterns) and feed it into the context pack for the
   next run.

8. **Prompt experiments.** A/B test different prompt structures to find what
   produces the fastest, most correct results. The runner already tracks
   per-batch success/failure/duration, which is the experiment metric.
