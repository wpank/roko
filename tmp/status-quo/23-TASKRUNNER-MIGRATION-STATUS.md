# 23 — Runner Lineage & Runner v2 Migration Status

> **Scope**: How roko arrived at its shipped in-process plan executor ("Runner v2"),
> which historical "runner" efforts were adopted vs abandoned, and what design intent
> the shipped code still lacks. Verified against code at git HEAD `5852c93c05`.
>
> **One-sentence answer**: The shipped plan executor is `crates/roko-cli/src/runner/`
> (Runner v2, 20 files, 17,090 LOC), built to the spec in
> `tmp/unified-migration-runner/RUNNER-V2-IMPLEMENTATION.md` (tasks R001–R045). Every
> other `tmp/*runner*` folder is a **bash meta-orchestrator** — a shell harness that
> spawned Claude/Codex agents in git worktrees to *build* roko — not a competing design
> for the executor itself. The term "runner" is badly overloaded in this repo.

---

## 0. TL;DR for navigators

| Question | Answer | Evidence |
|---|---|---|
| What actually runs `roko plan run`? | Depends on `--engine`. Default `graph` is a **stub no-op**; `runner-v2` is the real executor. | `commands/plan.rs:258-267`, `main.rs:1361` |
| What is Runner v2? | In-process, event-driven, streaming plan executor. | `runner/mod.rs`, `runner/event_loop.rs` (6,681 LOC) |
| Which design produced it? | `tmp/unified-migration-runner/RUNNER-V2-IMPLEMENTATION.md` (R001–R045). | file:1-9, module layout matches 1:1 |
| Are `tmp/taskrunner`, `tmp/runners`, `tmp/acp-runner` executor designs? | **No.** They are shell harnesses that spawn build-agents. Abandoned as tooling; their *outputs* landed. | `taskrunner/README.md:1-8`, `acp-runner/README.md:1-3` |
| Biggest trap? | The CLI default engine (`graph`) silently emits `task-output:dry-run:…` and never dispatches an LLM. | `roko-graph/src/cells/task_executor.rs:70-92` |

---

## 1. The "runner" name is overloaded — three distinct things

The word "runner" refers to at least three unrelated artifacts. Conflating them is the
single biggest source of confusion in the `tmp/` archaeology.

| Kind | Example | What it is | Runtime relevance |
|---|---|---|---|
| **A. Shipped plan executor** | `crates/roko-cli/src/runner/` | The in-process Rust module that dispatches agents, runs gates, persists state for `roko plan run`. | **This is Runner v2. Live.** |
| **B. Bash meta-orchestrators** | `tmp/taskrunner/`, `tmp/unified-migration-runner/`, `tmp/acp-runner/`, `tmp/runners/` | Shell harnesses (`run.sh`, `scripts/spawn.sh`) that spawn *external* Claude/Codex agents in git worktrees to write roko's own code, wave by wave, with gates between waves. | **Build-time only.** Not shipped, not called at runtime. |
| **C. The Graph Engine** | `crates/roko-graph/` + `commands/plan.rs cmd_plan_run_engine` | An alternative execution path (`--engine graph`) that converts a plan to a cell graph and executes it. Intended eventual successor to Runner v2, but its task cell is a stub. | **Default flag, but a no-op for real plan execution.** |

Everything in `FOCUS` (`tmp/taskrunner`, `tmp/runners`, `tmp/unified-migration-runner`,
`tmp/unified-migration`, `tmp/acp-runner`) is category **B or its design docs** — not
competing executor implementations.

---

## 2. Timeline / lineage

All folders are dated ~May 1–6 2026 (the `tmp/` mtimes are a bulk copy of Jul 7; the
authored dates live inside STATE/STATUS files).

```
Apr 26  unified-migration-runner run-20260426-080451: 4-agent, crate-partitioned
        parallel build harness executes the unified migration (STATE.md).
        └── One of its task lists = RUNNER-V2-IMPLEMENTATION.md (R001–R045)
            → produces crates/roko-cli/src/runner/  ← ★ Runner v2 born here

May 1   acp-runner: overnight Codex batch harness (ACP01–ACP18)
        → produces crates/roko-acp/ (15.8K LOC, shipped). ✔ succeeded
        runners/: earlier convergence/parity harness generation (3,935 files of logs).
        unified-migration/: the 4 phase design docs the runner above consumed.

May 6   taskrunner: newest harness. 100 wiring/cleanup tasks in waves 0–6
        (worktree-per-agent). Tasks include "Runner v2 Gate Failure Path" (Task 033)
        → proves Runner v2 already existed and was being *hardened*, not built, by May 6.
```

**Conclusion**: Runner v2 is the single adopted executor design. It was authored by the
`unified-migration-runner` harness, then hardened by the `taskrunner` harness. The
`acp-runner` produced a sibling crate (`roko-acp`), not a competing executor. `runners/`
and `unified-migration/` are upstream design/log material.

---

## 3. Runner v2 — what shipped (verified)

`crates/roko-cli/src/runner/` — 20 files, 17,090 LOC (spec estimated ~2,500; it grew 7×).

| Spec task | Design intent | Shipped? | Evidence |
|---|---|---|---|
| R001–R007 | Module scaffold, plan_loader (no discovery magic), atomic persist, TUI bridge | ✅ | `runner/mod.rs`, `plan_loader.rs` (703), `persist.rs` (728), `tui_bridge.rs` (210) |
| R008–R014 | Stream-json parser, agent events, gate dispatch, process groups | ✅ | `agent_stream.rs` (352), `agent_events.rs` (187), `gate_dispatch.rs` (539) |
| R015–R022 | Core event loop over `ParallelExecutor::tick()`, action dispatch, retry, RunReport | ✅ | `event_loop.rs:198,268-273,1749` uses `ParallelExecutor`; `PlanReport`/`RunReport` re-exported |
| R023 | Retire `orchestrate.rs` behind a feature gate | ✅ | `Cargo.toml:16,108` `legacy-orchestrate`; `lib.rs:94` `#[cfg(feature="legacy-orchestrate")]` |
| R024–R027 | Episode + efficiency + routing recording, resume from snapshot | ✅ | `resume.rs` (503) `prepare_resume`/`prepare_resume_with_force`; RunLedger in `persist.rs` |
| R018 | Wire runner as the default `plan run` path | ⚠️ **Regressed** | Default is now `--engine graph` (the stub), not Runner v2 — see §5 |
| R045 | Optional per-plan git worktree isolation (`use_worktrees`) | ❌ **Gap** | `worktree.rs` exists in `roko-orchestrator` but `event_loop.rs` never creates one (only a stale comment at :1290) |

**Design intent that shipped *beyond* the spec:**

- **Real merge queue.** The spec's R016 naively translated `MergeBranch → MergeSucceeded`.
  Shipped `runner/merge.rs` (777 LOC) routes through `roko_orchestrator::MergeQueue` with a
  real post-merge regression gate — an improvement noted in the file header (`merge.rs:1-18`).
- **Strict resume validation.** `resume.rs` refuses to resume when task fingerprints diverge,
  with an explicit `force_resume` override — stronger than R026's "load snapshot and skip."
- **Extra modules not in the spec**: `sse_stream.rs`, `snapshot_writer.rs`, `projection.rs`,
  `output_sink.rs` (1,399), `extension_loader.rs`, `inline_output.rs`, `task_dag.rs` (613).

**Parallelism**: fully realized. `event_loop.rs:268-273` builds an executor config with
`max_concurrent_tasks` and `DEFAULT_RUNNER_MAX_CONCURRENT_PLANS`; scheduling is tick-driven
(`executor.tick()` at `:1749`). Concurrency derives from `roko.toml` `runner.max_concurrent_tasks`
(`commands/plan.rs:342-348`).

---

## 4. `roko-orchestrator` vs `runner/` — the boundary

Runner v2 does **not** reimplement the DAG/scheduler; it *drives* `roko-orchestrator`.

| Concern | Owner | Note |
|---|---|---|
| DAG topo-sort, ready-set, `tick()` scheduling | `roko-orchestrator::ParallelExecutor` | Consumed by `runner/event_loop.rs` |
| Merge queue + locks | `roko-orchestrator::MergeQueue` | Wrapped by `runner/merge.rs` |
| Snapshot type | `roko-orchestrator::OrchestratorSnapshot` | Persisted/loaded by `runner/persist.rs` + `resume.rs` |
| Worktree isolation | `roko-orchestrator::worktree` | **Built but not called by the runner** (§5, gap) |
| Agent spawn, stream parse, gates, TUI, persist | `runner/` | The event-loop glue the spec set out to build |

This is the intended relationship from `RUNNER-V2-IMPLEMENTATION.md:38,55` ("Uses existing
crate APIs (ParallelExecutor, run_rung, …)"). No duplication of the scheduler was found.

---

## 5. Open issues — P0/P1

### P0 — Default engine (`graph`) is a silent no-op for plan execution
`roko plan run <dir>` defaults to `--engine graph` (`main.rs:1361 default_value="graph"`).
That path (`commands/plan.rs:266 cmd_plan_run_engine`) converts each task to a
`task-executor` cell, and `default_registry` registers `TaskExecutorCell::default()`
which is **`dry_run: true`** (`roko-graph/src/cells/task_executor.rs:30-34`). Its
`execute()` emits `task-output:dry-run:{label}` and never dispatches an LLM
(`task_executor.rs:70-92`). Nothing anywhere constructs it with `dry_run:false` (grep:
only `orchestrate.rs`, `main.rs run` prompt path, and enrichment do). **Net effect: the
out-of-box `roko plan run` reports SUCCESS while doing nothing.** The real executor
requires `--engine runner-v2`.
- Fix options: (a) flip the CLI default to `runner-v2`; or (b) implement live dispatch in
  `TaskExecutorCell` (delegate to the runner agent path, as its own doc-comment promises).
- Note the type-level `#[default]` on `PlanEngine` *is* `RunnerV2` (`main.rs:1301`); only the
  clap `default_value` string disagrees — a one-line drift with large consequences.

### P1 — Worktree isolation (R045) never wired into the runner
`roko-orchestrator/src/worktree.rs` exists; the runner event loop never creates a per-plan
worktree (`event_loop.rs` has only a stale comment at :1290). Parallel plans touching
overlapping files rely solely on the merge-queue regression gate, not on isolation.

### P1 — Resume unsupported on the default engine
`--resume-plan` is silently ignored under `--engine graph` (`commands/plan.rs:260-263`
prints a note and drops the snapshot). Resume only works on the `runner-v2` path. If the
default flips to graph permanently, crash-recovery — a core motivation of Runner v2 (spec
:27) — is unreachable by default.

### P2 — `orchestrate.rs` still on disk (legacy)
21K-LOC god object retained behind `legacy-orchestrate` feature (`lib.rs:94`). Not compiled
by default. Keep as reference or delete; it currently anchors stale `grep` hits.

---

## 6. Adopted vs abandoned — folder verdicts

| Folder | Verdict | Basis |
|---|---|---|
| `tmp/unified-migration-runner/RUNNER-V2-IMPLEMENTATION.md` | **Adopted** — the canonical Runner v2 spec. Preserve. | Module layout matches `runner/` 1:1 |
| `tmp/unified-migration-runner/` (harness, `run.sh`, prompts) | **Abandoned tooling; outputs landed.** Migration completed (all target crates exist). "Blocked 28 / Pending 109" in MASTER-CHECKLIST is a **mid-run snapshot**, not final state. | `STATE.md` run-20260426; crates present |
| `tmp/unified-migration/` (4 phase docs) | **Consumed source material.** Design input to the runner above. | Referenced by RUNNER-V2 spec |
| `tmp/taskrunner/` | **Abandoned tooling; outputs landed.** Hardened Runner v2 (Tasks 033, 025) + wiring. `wired=0` in STATUS.toml is a **broken aggregate counter**, not proof of non-integration — every task row reads `status="implemented"` and the meta counters (`pending=0, claimed=0` too) were never maintained by `complete.sh`. Spot-checks (PostGateReflection, RunLedger, feature gate) confirm the code landed. | STATUS.toml:8-14; grep of runner/ |
| `tmp/acp-runner/` | **Succeeded.** Produced `crates/roko-acp/` (15.8K LOC, shipped). | `acp-runner/README.md:1-3`; crate exists |
| `tmp/runners/` (3,935 files) | **Archaeology.** Earlier convergence/parity/perf harness runs + logs. Log noise; mine for task lists only. | Dir listing (converge, mega-parity, perf, …) |

**Correction to prior version of this doc**: it treated `taskrunner wired=0` and
`unified-migration-runner Blocked=28` as evidence the migration did not complete. Both
numbers are unmaintained counters / mid-run snapshots. The migration **did** complete: the
crates, the runner module, `roko-acp`, and the feature gate all exist in the tree today.
The residual risk is not "did it land" but the **P0 default-engine trap** above.

---

## 7. Runner v2 gap analysis vs original design intent

| Design goal (spec) | Status | Gap |
|---|---|---|
| Stream agent output (TUI live) | ✅ | — |
| Per-task persistence flush (crash-safe) | ✅ | `persist.rs`, resume validated |
| No discovery magic (tasks.toml = truth) | ✅ | `plan_loader.rs` |
| Parallel plans + tasks | ✅ | via `ParallelExecutor` |
| Real merge queue | ✅ (exceeds spec) | — |
| Resume from snapshot | ✅ | but not under `--engine graph` |
| Default `plan run` = Runner v2 | ❌ | Default is the graph stub (P0) |
| Worktree isolation per plan (R045) | ❌ | Built in orchestrator, unwired (P1) |
| Graph Engine as eventual successor | ❌ | `TaskExecutorCell` is dry-run-only (P0) |

---

## 8. Checklist (reverify before copying into `.roko/GAPS.md` or [24](24-OPEN-ISSUE-LEDGER.md))

- [ ] **P0**: Decide graph-vs-runner-v2 default. Either flip `main.rs:1361` to `runner-v2`,
      or implement live dispatch in `roko-graph/src/cells/task_executor.rs`. Until then,
      document loudly that `roko plan run` needs `--engine runner-v2`.
- [ ] **P1**: Wire `roko-orchestrator::worktree` into `runner/event_loop.rs` behind
      `runner.use_worktrees` (R045).
- [ ] **P1**: Support `--resume-plan` on the graph path, or gate it so the graph default
      doesn't silently discard snapshots.
- [ ] **P2**: Resolve `orchestrate.rs` — keep as documented reference or delete.
- [ ] Preserve `RUNNER-V2-IMPLEMENTATION.md` as the authoritative executor spec; archive the
      harness `run.sh`/`prompts/` trees and their unmaintained STATUS counters.
- [ ] Do **not** re-open taskrunner/unified-migration items on the strength of `wired=0` /
      `Blocked=28`; those are stale counters. Reverify each candidate against code +
      [25-PROOF-GATES.md](25-PROOF-GATES.md) first.

---

## 9. Roadmap

1. **Converge on one executor** (P0). The repo ships two half-defaults: a working
   `runner-v2` and a stub `graph`. Pick graph-as-successor (finish `TaskExecutorCell`
   live dispatch) *or* runner-v2-as-default (flip the flag) — not both silently.
2. **Close the crash-recovery gap** (P1): worktrees + resume must both work on whatever
   becomes the default.
3. **Retire the harnesses**: move `tmp/taskrunner`, `tmp/unified-migration-runner`,
   `tmp/runners`, `tmp/acp-runner` under `tmp/archive/done-runners/` (where earlier runs
   already live) once their still-open task lists are triaged into the ledger.
4. **Tag every future runner-generated doc** with a status: `implemented-only`, `wired`,
   `tested`, `verified`, or `superseded` — so a broken counter never again reads as truth.
