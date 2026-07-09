# E01 — Execution Engine  *(M0 — bootstrap epic, must precede all others)*

> **Milestone M0.** Roko cannot self-execute its own backlog until the default
> `plan run` path does real work. Every other epic (E02+) is dispatched *through*
> this engine, so E01 is the gate on self-hosting. Nothing else in the backlog can
> be trusted to run until E01-T01 lands.

## Goal

Make `roko plan run <dir>` — **with no flags** — spawn real agents, run gates,
persist episodes/snapshots, and resume correctly. Then close the four load-bearing
holdouts on the live Runner v2 path (no real DAG, prompt-only replan, shallow gate
enrichment, unwired worktree isolation) so parallel self-hosting is safe.

## Why

`roko plan run <dir>` currently defaults to the **Graph Engine**, which maps every
task to a dry-run stub: it prints `SUCCESS` in ~2 s, spawns 0 agents, spends $0,
changes no code, and writes no episodes/snapshot. Empirical proof (doc 95): the same
8-node plan reported "SUCCESS, 0 agents, $0.00" on the default path vs 8 real agents /
$2.75 / 621 s under `--engine runner-v2`. A self-developing agent whose default
execution command fabricates success cannot bootstrap. This epic makes the default
honest and closes the migration debt left behind when live capability was built for
`orchestrate.rs` and only ~80% re-hosted into Runner v2.

## Source docs

`95-ENGINE-DRIFT.md` (navigation ref) · `96-TRACE-RUNNER-V2-EXECUTION.md` (deep trace) ·
`36-ORCHESTRATION-RUNNERS.md` · `37-RUNNER-V2-AND-GRAPH.md` (Minimum Graph Parity) ·
`31-GRAPH-CELLS-ENGINE.md` (built-not-registered cells) · `92-RUNNER-V2-MODULE-FAMILY.md` ·
supporting: `101-TRACE-GATE-PIPELINE.md`, `60-STATE-PERSISTENCE-LEDGER.md`.

## Findings covered

| # | Finding | Source doc | Evidence (re-verified @ HEAD `5852c93c05`) | Prio | State |
|---|---|---|---|---|---|
| a | Default `plan run` = Graph dry-run stub (`TaskExecutorCell.dry_run:true`, live branch "not yet implemented" falls back to dry-run + warn) | 95 §"why hollow", 31 | `roko-graph .../task_executor.rs:31-34,70-92`; `engine.rs:356-358` binds `task-executor`→stub | **P0** | open |
| b | Default engine mismatch: clap `default_value="graph"` overrides enum `#[default] RunnerV2` | 95, 96 §1.2 | `main.rs:1295-1303` enum default = RunnerV2 (**already correct**); `main.rs:1361` clap `default_value = "graph"` **still wins** — the real bug | **P0** | open |
| c | `roko resume` hardcodes `PlanEngine::Graph` → snapshot discarded, run dry-runs | 95 §"resume bug" | `main.rs:2697-2708` (`engine: PlanEngine::Graph`) | **P0** | open |
| d | Runner v2 has no real DAG: flat `task_index: HashMap` + per-plan FSM; concurrency per-**plan** (`max_concurrent_plans=4`, one agent/plan); `max_concurrent_tasks` only sizes gate semaphore; `runner/task_dag.rs::TaskDag` dead | 96 §4-5, 92 | `event_loop.rs:62` imports only `task_status_is_terminal` (TaskDag struct never instantiated); `defaults.rs:313` const=4; `event_loop.rs:473` semaphore | **P1** | open |
| e | Gate-failure "replan" is prompt-append only — no plan/DAG mutation, no task re-gen (vs legacy `build_gate_failure_plan_revision`) | 96 §9, 95 | `event_loop.rs:1549-1590` `set_replan_context` → appended at `:4417-4418` | **P1** | open |
| f | Worktree isolation built in `roko-orchestrator` but unwired in runner; all plans mutate one shared tree, "merge" = cargo-check | 96 §10 | `roko-orchestrator/src/worktree.rs` exists; runner only 1 mention in `merge.rs`; `merge.rs:146-205` in-place mode | **P1** | open |
| g | Gate path shallow: `run_gate_once` uses `RungExecutionInputs::default()`, never calls `enrich_rung_config`; rungs 3-6 stub-pass | 95 consequence tbl, 101 | `enrich_rung_config` has **0** matches in `runner/`; `gate_dispatch.rs:104,140` | **P1** | open |

## Reconciliation with existing plans P11 / P12 / P15

| Plan | Covers | **Gap this epic must still close** |
|---|---|---|
| **P11-runner-v2-default** (5 tasks) | (b) partial — T1 adds `legacy-runner-v2` to default cargo features; T2 moves `#[default]` to `RunnerV2` (**enum already shows this at `main.rs:1301`**); T3 strips `cfg` gates; T5 adds TOML validation on `plan generate` | **P11 never touches the clap `default_value = "graph"` at `main.rs:1361`** — the literal that actually overrides the enum default. A bare `roko plan run plans/` still routes to Graph even after all of P11. **This is the single missed line that keeps the default hollow → E01-T01.** |
| **P12-runner-parallelism** (5 tasks) | (d) *symptom* only — reads `max_parallel`, converts `active_agent_tasks`/`agent_handles` to multi-task-per-plan maps, relaxes the one-agent-per-plan guard, emits multiple `SpawnAgent` per plan | Relaxes the FSM guard but **keeps the flat `task_index`**; `runner/task_dag.rs::TaskDag` stays dead (no topological scheduler, no real dependency-ordered readiness across the parallel set). Also does not decouple agent concurrency from the fixed `max_concurrent_plans=4`. → E01-T04, E01-T05 build the actual DAG scheduler P12 stops short of. |
| **P15-error-recovery-wiring** (5 tasks) | *adjacent, not (e)* — wires `classify_agent_crash` into `handle_agent_failure` / `do_cmd.rs`, adds `crash_class` to ledger, warns on silent `roko.toml` parse fallback | Handles **agent crash** classification, **not** gate-failure replan. Finding (e) (prompt-only replan → true task/plan revision) is untouched. → E01-T06. |

**Net:** P11/P12/P15 are complementary but leave the P0 default-honesty line (T01),
the resume fix (T02), the real DAG (T04), gate-failure task revision (T06), worktree
isolation (T08), and gate enrichment (T09) uncovered.

## Task list

Each task: id · title · tier · files · depends_on · acceptance · verify.

### E01-T01 — Flip `plan run` default engine to runner-v2  *(THE self-hosting unblock)*
- **tier** mechanical · **files** `crates/roko-cli/src/main.rs` · **depends_on** [] (soft: P11-T1 for feature)
- Change clap `default_value = "graph"` → `"runner-v2"` at `main.rs:1361`. Enum `#[default]` is already `RunnerV2`; this line is the override that makes Graph win.
- **Acceptance:**
  1. `main.rs:1361` reads `default_value = "runner-v2"`.
  2. `roko plan run plans/<any>` with no `--engine` flag routes to the Runner v2 block (`commands/plan.rs:269`), not `cmd_plan_run_engine`.
  3. A bare-default run spawns ≥1 real agent and appends to `.roko/episodes.jsonl`.
  4. `--engine graph` still selectable explicitly.
- **verify:** `structural` `grep -q 'default_value = "runner-v2"' crates/roko-cli/src/main.rs` · `compile` `cargo check -p roko-cli` · `integration` run a 1-task plan with no flag, assert `test -s .roko/episodes.jsonl`.

### E01-T02 — Fix `roko resume` to route to RunnerV2
- **tier** mechanical · **files** `crates/roko-cli/src/main.rs` · **depends_on** ["E01-T01"]
- Change `engine: PlanEngine::Graph` → `PlanEngine::RunnerV2` at `main.rs:2699` so the located snapshot is honored (Graph prints "snapshots ignored").
- **Acceptance:**
  1. `main.rs:2699` uses `PlanEngine::RunnerV2`.
  2. `roko resume` on an interrupted run resumes from `.roko/state/executor.json` and skips already-completed tasks.
  3. No "snapshots will be ignored" warning on resume.
- **verify:** `structural` `grep -A6 'PlanCmd::Run {' crates/roko-cli/src/main.rs | grep -q 'PlanEngine::RunnerV2'` · `compile` `cargo check -p roko-cli`.

### E01-T03 — Make the Graph path honest (defense in depth)
- **tier** focused · **files** `crates/roko-cli/src/commands/plan.rs`, `crates/roko-graph/src/cells/task_executor.rs` · **depends_on** ["E01-T01"]
- When `task-executor` resolves to a `dry_run:true` cell, `--engine graph` must print a loud warning ("Graph engine is a dry-run stub — no agents will run; use --engine runner-v2") instead of a bare `SUCCESS`. Do not silently fabricate success.
- **Acceptance:**
  1. `roko plan run <dir> --engine graph` emits a visible dry-run/no-op warning to stderr.
  2. Warning names `--engine runner-v2` as the real path.
  3. Exit code and report clearly mark the run as dry-run, not SUCCESS.
- **verify:** `integration` `roko plan run <dir> --engine graph 2>&1 | grep -qi 'dry-run'` · `compile` `cargo check -p roko-cli -p roko-graph`.

### E01-T04 — Wire a real intra-plan DAG scheduler (or delete dead `task_dag.rs`)
- **tier** architectural · **files** `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/task_dag.rs` · **depends_on** ["E01-T01"] (reconciles P12)
- Replace the flat `task_index` readiness walk (`event_loop.rs:3920-3950`) with `runner/task_dag.rs::TaskDag` (`ready_tasks`, `next_ready_task`, `mark_running`, topo helpers) so intra-plan tasks with satisfied `depends_on` dispatch in parallel and topological order. If P12's flat-map approach is retained instead, delete `TaskDag` as confirmed dead code.
- **Acceptance:**
  1. `TaskDag` is instantiated and consumed in the live loop, **or** `task_dag.rs` is deleted and `event_loop.rs:62` import removed.
  2. Two independent ready tasks in one plan dispatch concurrently (up to the concurrency cap).
  3. A task with an unsatisfied `depends_on` never dispatches before its dep completes.
  4. No dead-code warnings for `task_dag.rs`.
- **verify:** `compile` `cargo check -p roko-cli` · `test` `cargo test -p roko-cli runner::task_dag` (add a topo-readiness unit test) · `structural` no `#[allow(dead_code)]` remaining on `TaskDag`.

### E01-T05 — Make agent concurrency configurable, decoupled from `max_concurrent_plans=4`
- **tier** focused · **files** `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-core/src/defaults.rs` · **depends_on** ["E01-T04"]
- `max_concurrent_tasks` / `--max-tasks` must actually throttle dispatched agents (today it only sizes the gate semaphore at `event_loop.rs:473`). Decouple from the hardcoded `DEFAULT_RUNNER_MAX_CONCURRENT_PLANS=4` (`defaults.rs:313`).
- **Acceptance:**
  1. `--max-tasks N` caps the number of concurrently running agents at N across the run.
  2. `max_concurrent_plans` is configurable (not a bare const) or documented as an independent knob.
  3. Gate-semaphore sizing is separated from agent-throttle sizing.
- **verify:** `compile` `cargo check -p roko-cli` · `test` `cargo test -p roko-cli` (concurrency-cap unit/integration test).

### E01-T06 — Upgrade gate-failure replan from prompt-append to task/plan revision
- **tier** integrative · **files** `crates/roko-cli/src/runner/event_loop.rs` · **depends_on** ["E01-T01"]
- Port legacy `orchestrate.rs::build_gate_failure_plan_revision`: on a terminal gate failure, mutate the plan/task set (regenerate or split the failing task) rather than only appending `set_replan_context` text (`event_loop.rs:1549-1590`).
- **Acceptance:**
  1. A gate-failing task triggers a plan/task revision (new or edited `TaskDef`), not just prompt text.
  2. The revision is persisted to the snapshot and visible in the run ledger.
  3. Existing prompt-context feedback still flows.
- **verify:** `compile` `cargo check -p roko-cli` · `test` `cargo test -p roko-cli` replan revision test.

### E01-T07 — Wire per-plan worktree isolation into the runner
- **tier** integrative · **files** `crates/roko-cli/src/runner/merge.rs`, `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-orchestrator/src/worktree.rs` · **depends_on** ["E01-T04"]
- Use the built `roko-orchestrator/src/worktree.rs` to give each plan (or parallel task) a `git worktree`, so concurrent plans don't corrupt one shared dirty tree; make `MergeBranch` merge a real branch instead of validating the in-place tree (`merge.rs:146-205`).
- **Acceptance:**
  1. Each concurrently executing plan runs in its own worktree/branch.
  2. `MergeBranch` performs a real branch merge with the post-merge `cargo check` verdict.
  3. Two parallel plans no longer see each other's uncommitted changes.
- **verify:** `compile` `cargo check -p roko-cli -p roko-orchestrator` · `test` `cargo test -p roko-cli runner::merge`.

### E01-T08 — Wire `enrich_rung_config` into the live gate dispatch
- **tier** integrative · **files** `crates/roko-cli/src/runner/gate_dispatch.rs`, `crates/roko-cli/src/runner/rung_dispatch.rs` · **depends_on** ["E01-T01"]
- `run_gate_once` must call `enrich_rung_config` (oracles 4-6, real rung inputs) instead of `RungExecutionInputs::default()`, so rungs 3-6 run real checks rather than stub-passing (`gate_dispatch.rs:104,140`).
- **Acceptance:**
  1. `enrich_rung_config` is called on the live gate path (grep > 0 in `runner/`).
  2. Rungs 3-6 execute real oracle logic, not `Verdict::pass` stubs.
  3. Gate-threshold EMA updates for rungs beyond rung 2.
- **verify:** `structural` `grep -rq enrich_rung_config crates/roko-cli/src/runner/` · `compile` `cargo check -p roko-cli` · `test` `cargo test -p roko-cli`.

### E01-T09 — Regression test: bare default `plan run` does real work
- **tier** focused · **files** `crates/roko-cli/tests/` (new) · **depends_on** ["E01-T01","E01-T02"]
- Add an integration test that runs a minimal 1-task plan with **no `--engine` flag** and asserts real execution artifacts (episode written, snapshot written, non-dry-run report). Guards against future default regressions.
- **Acceptance:**
  1. Test runs `plan run` with no engine flag.
  2. Asserts `.roko/episodes.jsonl` non-empty and `.roko/state/executor.json` written.
  3. Asserts report is not the Graph dry-run shape.
- **verify:** `test` `cargo test -p roko-cli default_engine_does_real_work`.

### E01-T10 — Reconcile CLAUDE.md + GAPS + docs to the real engine
- **tier** mechanical · **files** `CLAUDE.md`, `.roko/GAPS.md` · **depends_on** ["E01-T01","E01-T02"]
- Update orchestration rows, self-host workflow steps (5/6) and the `legacy-runner-v2` framing to name `runner/` + `roko-graph` as the live path (not `orchestrate.rs`), and correct the resume flag docs.
- **Acceptance:**
  1. CLAUDE.md no longer claims `orchestrate.rs` is the wired execution hub.
  2. Self-host workflow uses the corrected default/resume commands.
  3. GAPS.md Task 102 `legacy-runner-v2` note corrected.
- **verify:** `structural` `! grep -q 'orchestrate.rs.*Wired' CLAUDE.md` (manual review acceptable).

## Critical path (within epic)

```
E01-T01 ──▶ E01-T02 ──▶ E01-T09  (make default honest → fix resume → lock it with a test)
   │
   ├─▶ E01-T03  (graph-path defense in depth)
   ├─▶ E01-T04 ─▶ E01-T05        (real DAG → configurable concurrency)
   │        └────▶ E01-T07       (worktree isolation depends on parallel DAG)
   ├─▶ E01-T06                   (gate-failure task revision)
   ├─▶ E01-T08                   (gate enrichment)
   └─▶ E01-T10                   (docs reconcile)
```

**M0 gate:** E01-T01 alone flips the default to real execution. Everything else in
the entire backlog is dispatched through it, so **E01-T01 is the single task that
unblocks self-hosting.** T02 + T09 make it durable; T04-T08 make parallel self-hosting
safe and correct.

---

```toml
[meta]
plan = "E01-execution-engine"
total = 10
done = 0
status = "ready"
max_parallel = 1

# ─────────────────────────────────────────────────────────────────────────────
# E01-T01: Flip `plan run` default engine to runner-v2  (THE self-hosting unblock)
# ─────────────────────────────────────────────────────────────────────────────
# Enum `#[default]` is already RunnerV2 (main.rs:1301) but clap's
# `default_value = "graph"` at main.rs:1361 overrides it, so a bare
# `roko plan run plans/` routes to the Graph dry-run stub. Change that one
# literal to "runner-v2". This is the line P11 missed.
[[task]]
id = "E01-T01"
title = "Flip plan run default engine to runner-v2"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 5
files = ["crates/roko-cli/src/main.rs"]
role = "implementer"
depends_on = []
domain = "execution-engine"

[task.context]
read_files = [
    { path = "crates/roko-cli/src/main.rs", lines = "1294-1304", why = "PlanEngine enum — #[default] is already RunnerV2; confirms the enum is not the bug" },
    { path = "crates/roko-cli/src/main.rs", lines = "1356-1365", why = "clap `default_value = \"graph\"` at :1361 is the override to change" },
]
symbols = [
    "PlanEngine — enum { Graph, RunnerV2 }; RunnerV2 has #[default]",
]
anti_patterns = [
    "Do NOT delete the Graph variant or its --engine graph selectability.",
    "Do NOT change the enum #[default] — it is already RunnerV2. Only the clap default_value literal changes.",
    "Do NOT touch the resume PlanCmd::Run at main.rs:2699 (that is E01-T02).",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'default_value = \"runner-v2\"' crates/roko-cli/src/main.rs"
fail_msg = "clap default_value must be runner-v2"

[[task.verify]]
phase = "structural"
command = "! grep -q 'default_value = \"graph\"' crates/roko-cli/src/main.rs"
fail_msg = "no clap arg may still default to graph"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile"

[[task.verify]]
phase = "integration"
command = "cargo run -p roko-cli -- plan run plans/e2e-smoke 2>&1 | tee /tmp/e01t01.log; test -s .roko/episodes.jsonl"
fail_msg = "bare-default plan run must spawn a real agent and write episodes.jsonl"

acceptance = [
    "main.rs:1361 reads default_value = \"runner-v2\"",
    "bare `roko plan run <dir>` routes to the Runner v2 block, not cmd_plan_run_engine",
    "a no-flag run appends to .roko/episodes.jsonl",
    "--engine graph is still explicitly selectable",
]

# ─────────────────────────────────────────────────────────────────────────────
# E01-T02: Fix `roko resume` to route to RunnerV2
# ─────────────────────────────────────────────────────────────────────────────
# `roko resume` builds PlanCmd::Run { engine: PlanEngine::Graph, .. } at
# main.rs:2697-2708. The Graph path prints "snapshots will be ignored" and
# dry-runs, so resume silently discards the snapshot. Route to RunnerV2, which
# auto-resumes from .roko/state/executor.json.
[[task]]
id = "E01-T02"
title = "Route roko resume to PlanEngine::RunnerV2"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 10
files = ["crates/roko-cli/src/main.rs"]
role = "implementer"
depends_on = ["E01-T01"]
domain = "execution-engine"

[task.context]
read_files = [
    { path = "crates/roko-cli/src/main.rs", lines = "2695-2710", why = "roko resume builds PlanCmd::Run with hardcoded PlanEngine::Graph at :2699" },
]
symbols = [
    "Command::Resume handler — constructs PlanCmd::Run { engine, resume_plan, .. }",
]
anti_patterns = [
    "Do NOT change any other field of the PlanCmd::Run struct literal.",
    "Do NOT alter snapshot-location logic above line 2697.",
]

[[task.verify]]
phase = "structural"
command = "sed -n '2696,2709p' crates/roko-cli/src/main.rs | grep -q 'PlanEngine::RunnerV2'"
fail_msg = "resume must construct engine: PlanEngine::RunnerV2"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile"

acceptance = [
    "main.rs:2699 uses PlanEngine::RunnerV2",
    "roko resume honors .roko/state/executor.json and skips completed tasks",
    "no 'snapshots will be ignored' warning on resume",
]

# ─────────────────────────────────────────────────────────────────────────────
# E01-T03: Make the Graph path honest (defense in depth)
# ─────────────────────────────────────────────────────────────────────────────
# When --engine graph resolves task-executor to a dry_run:true cell, warn
# loudly instead of printing a bare SUCCESS. Prevents fabricated success if
# anyone selects graph explicitly.
[[task]]
id = "E01-T03"
title = "Warn on Graph dry-run stub instead of fabricating SUCCESS"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-5"
max_loc = 40
files = [
    "crates/roko-cli/src/commands/plan.rs",
    "crates/roko-graph/src/cells/task_executor.rs",
]
role = "implementer"
depends_on = ["E01-T01"]
domain = "execution-engine"

[task.context]
read_files = [
    { path = "crates/roko-cli/src/commands/plan.rs", lines = "1567-1715", why = "cmd_plan_run_engine — Graph path that prints SUCCESS" },
    { path = "crates/roko-graph/src/cells/task_executor.rs", lines = "28-95", why = "dry_run:true stub; live branch falls back to dry-run + warn" },
]
symbols = [
    "TaskExecutorCell — dry_run field default true",
    "cmd_plan_run_engine — Graph execution entry",
]
anti_patterns = [
    "Do NOT implement live dispatch in the Graph cell here — only warn and mark the run as dry-run.",
    "Do NOT change the default engine (that is E01-T01).",
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli -p roko-graph 2>&1"
fail_msg = "roko-cli and roko-graph must compile"

[[task.verify]]
phase = "integration"
command = "cargo run -p roko-cli -- plan run plans/e2e-smoke --engine graph 2>&1 | grep -qi 'dry-run'"
fail_msg = "graph engine must warn it is a dry-run stub"

acceptance = [
    "roko plan run <dir> --engine graph emits a visible dry-run/no-op warning to stderr",
    "the warning names --engine runner-v2 as the real path",
    "the run report marks itself dry-run, not SUCCESS",
]
```
