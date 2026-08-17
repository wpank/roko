# 12 — Retirement: Delete the Old Runtimes

> Phase 6 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Pure deletion plan — only safe to run after 11 is complete and soaking has shown no regressions.

---

## Status (2026-05-01)

**NOT STARTED.** Massive amount of dead-or-dying code still in tree.

**What's still present:**

| Code | LOC | Status | Reason for retention |
|---|---|---|---|
| `crates/roko-cli/src/orchestrate.rs` | 22,756 | feature-gated `legacy-orchestrate` | Some unique features not yet ported (knowledge-routing-boost, anophily, custody) |
| `crates/roko-cli/src/runner/event_loop.rs` | ~3,035 | active (default plan runner) | Plan 11 / 06 migration in progress |
| `crates/roko-cli/src/dispatch_direct.rs` | ~405 | feature-gated `legacy-orchestrate` | Used by orchestrate; not by default |
| `crates/roko-cli/src/chat.rs::extract_clean_text` | ~246 | live | Used by chat_inline + agent_serve |
| `crates/roko-orchestrator/src/coordination.rs` (pheromones) | ~68,000 | live but unused at runtime | Audit doc 14 |
| `crates/roko-daimon/src/lib.rs` (PAD model) | ~40,000 | live but largely unused | Audit doc 14 |
| `crates/roko-compose/src/auction.rs` (VCG) | ~500 | live, exported | Plan 02 step 9 |
| `crates/roko-orchestrator/src/dag.rs::UnifiedTaskDag` | ~92,000 | live (used by orchestrate + runner DAG) | Replaced by `TaskScheduler` per plan 06 |
| `crates/roko-orchestrator/src/executor/mod.rs::ParallelExecutor` | ~ | live (used by orchestrate) | Replaced by WorkflowEngine per plan 11 |
| `crates/roko-orchestrator/src/merge_queue.rs` | ~32,000 | "fully built but never called" per audit 15 § 7 | Folded into `MergeService` per plan 07 |
| 12 noisy feedback hooks (HDC, somatic markers, calibration, etc.) | ~ | live | Plan 03 step 6 |
| `roko-cli runtime_feedback/` parallel sink trait | ~ | live | Plan 03 step 8 |
| `DashboardEvent` enum | ~ | live | Plan 10 step 2 |
| `RunStateSnapshot`, `ExecutorSnapshot`, `OrchestratorSnapshot` (3 schemas) | ~ | live | Plan 04 step 4 |
| `dispatch_helpers::build_system_prompt_with_context_validated` | ~ | live | Plan 02 step 9 |
| `TaskPromptComposer` (was `PromptAssembler` in CLI) | ~ | live | Plan 02 step 4 |

---

## Goal

After this plan, the binary contains **one** way to do each thing. The `legacy-orchestrate` feature flag is removed. The 12 hooks are gone. The pheromones, daimon PAD, VCG, HDC code is deleted. `roko-cli/src/runner/event_loop.rs` is deleted. `orchestrate.rs` is deleted. `dispatch_direct.rs` is deleted.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#10 God file** — three 20K+ LOC monoliths
- **#7 Copy-Paste** — duplicated dispatch / event / scheduler / sink code
- **#3 Build Another Runtime** — every legacy code path is a parallel runtime

---

## Pre-Conditions (Verify Before Deleting)

Each deletion is gated on these checks. Skip ahead to the per-file checklist if you want to start now and discover gaps.

### General preconditions

- [ ] Plan 11 (Entry Point Convergence) complete: `roko run`, `roko plan run`, ACP, HTTP all on `WorkflowEngine`
- [ ] Two-week soak in production / staging with no regressions reported
- [ ] All proof tests in plans 01–11 + 13–17 pass
- [ ] Default `cargo build --bin roko --no-default-features` succeeds
- [ ] Default `cargo test --bin roko` passes without `legacy-orchestrate` enabled
- [ ] HTTP API contract tests cover all routes that previously delegated to legacy code

### Per-feature preconditions

For each retirement, read the linked feature-extraction list. If anything is missing, file a follow-up plan entry and fix before deleting.

---

## Implementation Steps (Per Component)

Order matters. Do them in this order; each later deletion assumes earlier ones are complete.

### Step 1 — Delete the 12 noisy feedback hooks

**Sources:** `roko-learn/src/{hdc,anomaly_detector,strategy_metadata,force_backend_override,somatic_markers,calibration,enriched_run_recorder,context_attribution}.rs` and similar.

Per plan 03 § Step 6. The deletion is per-file; verify no live caller before each.

```bash
for f in hdc anomaly_detector strategy_metadata force_backend_override somatic_markers calibration enriched_run_recorder context_attribution; do
    rg "use.*${f}::" crates/ --type rust
done
# expected: only test files or nothing
```

### Step 2 — Delete CLI `runtime_feedback/` module

Per plan 03 § Step 8. After all CLI sinks have been moved to `roko-learn/src/sinks/` and `MultiSink` is the canonical fanout, delete `crates/roko-cli/src/runtime_feedback/`.

```bash
ls crates/roko-cli/src/runtime_feedback/
# expected after: directory does not exist
```

### Step 3 — Delete `DashboardEvent`

Per plan 10 § Step 2. After all consumers (TUI, SSE, WS) are on `RuntimeEvent`, delete `DashboardEvent`.

```bash
rg 'DashboardEvent' crates/ --type rust | grep -v '#\[deprecated\]'
# expected: 0
```

### Step 4 — Delete `extract_clean_text`

Per plan 01 § Step 7. After typed `ModelCallResponse` is the only response type, delete the 246-line monster from `crates/roko-cli/src/chat.rs`.

```bash
rg 'extract_clean_text' crates/ --type rust
# expected: 0
```

### Step 5 — Delete `runner/event_loop.rs`

After plan 11 step 1 + a soak period:

1. Verify `roko plan run` defaults to `WorkflowEngine` (no `--use-event-loop`)
2. Remove `--use-event-loop` flag (mark as removed in CHANGELOG)
3. Delete `crates/roko-cli/src/runner/event_loop.rs`
4. Delete `crates/roko-cli/src/runner/task_dag.rs` (replaced by `TaskScheduler`)
5. Delete `crates/roko-cli/src/runner/persist.rs::RunStateSnapshot` and `runner/resume.rs` (replaced by `PersistenceService`)
6. Delete `crates/roko-cli/src/runner/agent_stream.rs` if no remaining caller (otherwise reduce it to a thin re-export of `roko-agent::provider::claude_cli::stream::parse_stream_line`)

```bash
ls crates/roko-cli/src/runner/
# expected: only mod.rs (a thin re-export) or nothing
```

### Step 6 — Delete `dispatch_direct.rs`

After plan 01 § Step 8 (already feature-gated). Before deleting:

- [ ] Confirm no `#[cfg(feature = "legacy-orchestrate")]` consumers outside `orchestrate.rs`
- [ ] If any reasonable test or docs example depends on `dispatch_claude_cli`, fix it to use `ModelCallService`

Then:

```bash
rm crates/roko-cli/src/dispatch_direct.rs
sed -i '/pub mod dispatch_direct/d' crates/roko-cli/src/lib.rs
```

### Step 7 — Extract remaining unique features from `orchestrate.rs`

Audit doc 15 § 8 lists features only in `orchestrate.rs`. For each, decide: extract or drop.

| Feature | Decision Guide |
|---|---|
| Knowledge routing (`build_knowledge_routing_advice`) | EXTRACT into `roko-learn/src/knowledge_routing.rs`; consumed by `CascadeRouter` (plan 08) as a routing hint |
| Anophily detection + remediation | DROP unless live use case — audit § 8 says no live caller |
| Custody audit chain | DROP — audit § 1 says CLI inspection only |
| Skill extraction (`SkillLibrary::extract`) | EXTRACT to `roko-learn/src/skill_library.rs`; consumed by `PlaybookSink` (plan 03 § 3) |
| C-factor computation | DROP — fold into router multi-objective signal (plan 08) |
| 30+ enrichment steps | EXTRACT only the steps that have observable signal: knowledge query, prior outputs, gate feedback. Drop the rest. |
| `gate_failure_next_action` (replan classifier) | EXTRACT to `roko-runtime/src/failure_classifier.rs` (plan 05 § Step 4) |
| `cascade_routing_context` (17-feature) | DROP — replaced by 6-feature `RoutingContext` (plan 08 § Step 1) |

### Step 8 — Delete `orchestrate.rs`

After Step 7:

1. Run `cargo build --features legacy-orchestrate` — should still build
2. Run all tests with the feature enabled
3. Move `crates/roko-cli/src/orchestrate.rs` → `tmp/legacy/orchestrate.rs.bak` (one release for safety)
4. Remove `pub mod orchestrate` from `crates/roko-cli/src/lib.rs`
5. Remove the `legacy-orchestrate` feature from `Cargo.toml`
6. Remove all `#[cfg(feature = "legacy-orchestrate")]` annotations from the codebase

```bash
rg 'cfg.*legacy-orchestrate' crates/ --type rust
# expected: 0
```

After one release with the file in `tmp/legacy/`, delete the backup.

### Step 9 — Delete `roko-orchestrator` legacy components

These were the orchestrate.rs back-ends:

| File | Action |
|---|---|
| `crates/roko-orchestrator/src/dag.rs::UnifiedTaskDag` | Delete after plan 06 (TaskScheduler) covers all DAG features |
| `crates/roko-orchestrator/src/executor/mod.rs::ParallelExecutor` | Delete after plan 11 (engine entry-point convergence) |
| `crates/roko-orchestrator/src/merge_queue.rs` | Move content to `roko-runtime/src/merge_service.rs` (plan 07 § 5); delete original |
| `crates/roko-orchestrator/src/coordination.rs` (pheromones) | Delete (per plan 15) |
| `crates/roko-orchestrator/src/replan.rs` | Already covered by `FailureClassifier` (plan 05 § 4); delete |
| `crates/roko-orchestrator/src/repair.rs` | Audit; if unique, extract; else delete |
| `crates/roko-orchestrator/src/post_merge.rs` | Move into `MergeService`; delete |
| `crates/roko-orchestrator/src/safety/` | Verify same as `roko-agent/src/safety/`; consolidate |

After: `roko-orchestrator` is mostly gone or shrunk to: plan discovery + worktree management + service factory.

### Step 10 — Delete `roko-daimon` PAD model

Per plan 15. Replace with `FailureTracker { consecutive_failures, last_failure_kind }` (per `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md` § 6.2.2).

The `AffectPolicy` trait in `roko-core::foundation` stays (so future affect modulation can be added behind it). The default `NoOpAffectPolicy` is the production implementation post-deletion.

```bash
rm -rf crates/roko-daimon
sed -i '/roko-daimon/d' Cargo.toml workspace.toml
```

### Step 11 — Delete pheromones

Per plan 15. Layer 8 of the prompt becomes a `Vec<String>` of warnings (already migrated in plan 02 § Step 3 §1).

```bash
rm crates/roko-orchestrator/src/coordination.rs
rm crates/roko-orchestrator/src/coordination/   # if directory
```

Replace `pheromone_chunks` references with `warnings` calls.

### Step 12 — Delete VCG auction

Per plan 02 § Step 9.

```bash
rm crates/roko-compose/src/auction.rs
sed -i '/pub use auction::/d' crates/roko-compose/src/lib.rs
```

### Step 13 — Verify final state

```bash
# 1. No god files
wc -l crates/roko-cli/src/*.rs crates/roko-acp/src/*.rs | sort -nr | head -5
# expected: every file < 2000 LOC

# 2. No feature-gated dead code
rg '#\[cfg\(feature = "legacy' crates/ --type rust
# expected: 0

# 3. Single canonical names
rg 'pub trait FeedbackSink|pub trait PromptAssembler|pub trait ModelCaller|pub enum FeedbackEvent|pub enum DashboardEvent|pub enum RuntimeEvent' crates/ --type rust
# Each name should appear exactly once (in roko-core)

# 4. Total LOC sanity check
find crates/ -name '*.rs' | xargs wc -l | tail -1
# expected: significantly less than baseline (~150K-200K LOC removed)

# 5. Default build clean
cargo build --bin roko --no-default-features
cargo build --workspace
cargo test --workspace
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #3 Build another runtime | Forking a copy of `event_loop.rs` instead of deleting | Just delete |
| #4 Wrong layer | Moving `coordination.rs` to a new crate "in case we need pheromones later" | Delete; revert if needed |
| #10 God file | Splitting `orchestrate.rs` into 5 files instead of deleting after extraction | Extract specific features; delete the rest |

---

## Things NOT To Do

1. **Don't delete a file before verifying no live caller.** Use `rg` exhaustively. The `legacy-orchestrate` feature gate is a *temporary* protection; once removed, code that referenced it stops compiling.
2. **Don't delete in one PR.** Each step is its own PR (or batch of related PRs). Reverting a 100K-LOC deletion is painful.
3. **Don't delete tests for legacy paths until the path is gone.** Tests verify migration didn't lose behavior. Delete test + path together.
4. **Don't delete persisted file formats.** `.roko/learn/cascade-router.json` schema must remain readable; the legacy 17-feature `LinUcbState` stays in the on-disk format until enough new-format observations accumulate (per plan 08 § 1).
5. **Don't delete `roko-daimon` types from the public Rust API.** External crates may import them. Mark `#[deprecated]` for one release, then delete in the next major version.
6. **Don't delete worktree code unless `MergeStrategy::Worktree` is also being removed.** Plan 07 keeps the strategy; the implementation moves into `MergeService`.
7. **Don't compress the schedule.** This is a multi-week burndown across many small PRs. Each one is small individually; sequencing matters.
8. **Don't skip the "tmp/legacy/" backup step** for `orchestrate.rs`. 22K LOC is hard to recover from `git log` if only one branch had it.

---

## Tests / Proof Criteria

After each deletion, the following must hold:

- [ ] `cargo build --bin roko --no-default-features` succeeds
- [ ] `cargo test --workspace` passes
- [ ] All proof tests from plans 01–11 still pass
- [ ] Manual smoke test of `roko`, `roko run`, `roko plan run`, `roko acp`, HTTP routes
- [ ] No new `TODO(retirement)` comments without an associated follow-up

After the entire plan:

- [ ] Total `crates/` LOC has dropped by ≥ 100K (mostly from `orchestrate.rs`, `coordination.rs`, `roko-daimon`)
- [ ] Episode write latency p99 < 5ms (was ~15ms before plan 03 § 6)
- [ ] Cold start of `roko serve` < 1s (was ~2-3s with all the dead modules loading)
- [ ] `git log --shortstat` for this plan's PRs shows net negative LOC

---

## Dependencies

This plan **requires** plans 01-11 + 13-17 to be complete. Each deletion has its own immediate dependency from the table above.

This plan **blocks** plan 18 (Proof Runs) because the proofs are best validated against the post-deletion binary.

---

## Estimated Effort

**XL.** ~2-3 weeks of pure deletion and verification work, sequential by step.

| Step | Effort |
|---|---|
| 1 — 12 hooks | M (3 days) |
| 2 — runtime_feedback/ | S (1 day) |
| 3 — DashboardEvent | M (2 days) |
| 4 — extract_clean_text | S (1 day) |
| 5 — event_loop.rs + runner/ | M (3 days; many test fix-ups) |
| 6 — dispatch_direct.rs | S (1 day) |
| 7 — extract orchestrate features | L (5 days; biggest unknown) |
| 8 — delete orchestrate.rs | M (2 days; verification heavy) |
| 9 — orchestrator legacy | M (3 days) |
| 10 — daimon | S (1 day, mostly mechanical) |
| 11 — pheromones | S (1 day) |
| 12 — VCG | S (1 day) |
| 13 — final verify | M (2 days) |
