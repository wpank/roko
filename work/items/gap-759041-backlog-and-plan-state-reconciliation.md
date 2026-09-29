+++
id = "gap-759041"
kind = "gap"
title = "Backlog and Plan State Reconciliation"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "tooling"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/229-backlog-plan-state-reconciliation.md#229 — Backlog and Plan State Reconciliation"
discovered_from = "audit:tmp/backlog/archive/229-backlog-plan-state-reconciliation.md#229 — Backlog and Plan State Reconciliation"
anchors = ["crates/roko-cli/src/commands/backlog.rs::build_audit_report", "crates/roko-cli/src/commands/backlog.rs::read_toml_plan_statuses", "crates/roko-cli/src/commands/backlog.rs::read_executor_plan_phases", "crates/roko-cli/src/commands/backlog.rs::read_run_state_task_terminals", "crates/roko-cli/src/graph_checkpoint.rs::canonical_checkpoint_status", "crates/roko-cli/src/orchestrator/plan_discovery.rs::find_plan_dirs", "crates/roko-cli/src/main.rs::BacklogCmd::Audit"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn audit_reports_graph_completed_plan_left_ready' crates/roko-cli/ && cargo test -p roko-cli --bin roko audit_reports_graph_completed_plan_left_ready"
+++

## Problem

`roko backlog audit` is supposed to catch plans and tasks whose recorded status disagrees with what actually ran. It reads only the state files of the deleted Runner-v2 engine, so on the Graph engine (the only engine) it reports no drift at all:

- `read_executor_plan_phases` reads `.roko/state/state-snapshot.json`, falling back to `executor.json`.
- `read_run_state_task_terminals` reads `run-state.json`, embedded in or beside that snapshot.

A Graph run writes neither. It writes `.roko/state/graph/<plan>/checkpoint.json` plus `activities.jsonl` and `costs.json`.

Also:

- `read_toml_plan_statuses` (`backlog.rs:278-308`) scans only `plans/*/tasks.toml`, one level deep, so nested plan sets such as `plans/portal-programme/01-backend-plan-service/` are invisible.
- A `tasks.toml` that fails to parse, or lacks `[meta] plan`, is skipped silently with `continue`.
- The Graph path never writes task or plan status back to `tasks.toml`.

So a plan that completed under Graph stays `status = "ready"` forever, and the audit says nothing. Dogfooding found 11 such `tasks.toml` files. On this repo the audit reported "13 plans on disk, 0 in the executor, 0 drift".

Expected: `roko backlog audit [--json]` reads Graph checkpoints, walks nested plans, and reports every mismatch with a stable code, the plan/task/path, the `run_id` and the evidence. In particular, a plan whose Graph run succeeded but whose `tasks.toml` still says `ready` is always reported.

## Why it matters

- Goal `tooling`: roko must track its own work. Stale `ready` plans can be picked and re-dispatched after their work has merged, and real unfinished work can be mistaken for done. This is the "roko tracks its own work" priority in `CLAUDE.md`.
- `bug-9fc3fc` ("backlog audit reads the removed engine's state and ignores Graph checkpoints") was closed as a duplicate of this item on 2026-09-29. Its two extra criteria (recurse into nested plans; report plans that fail to parse or lack `[meta] plan`) belong here.
- Related:
  - `gap-20ab07`: `roko plan status` reads only `succeeded` from Graph checkpoints;
  - `bug-4cac0e`: the F2 Plans view scans one level;
  - `bug-5e7de4`: `plans/INDEX.md` is inaccurate.
  All of them need the same read-only "Graph run summary" reader.

## Where

- `crates/roko-cli/src/commands/backlog.rs` (bin crate; unchanged since `244f564e1`):
  - `cmd_backlog_audit` (:651) is the entry point for `roko backlog audit [--json] [--fix-safe]`;
  - `build_audit_report` (:441) compares TOML statuses with runner phases and terminals and builds `PlanDrift`/`TaskDrift` rows, printed by `print_audit_report` (:572);
  - `read_toml_plan_statuses` (:278): a one-level scan, silent skip on parse failure;
  - `read_executor_plan_phases` (:314) and `read_run_state_task_terminals` (:366): the Runner-v2 readers;
  - `apply_safe_fixes` (:698), `fix_broken_index_references` (:726) and `fix_duplicate_index_ids` (:767): the `--fix-safe` scope (index repairs only);
  - `cmd_backlog_mark_done` (:810): explicit semantic closure with evidence.
- `crates/roko-cli/src/main.rs` (around :1756): the `BacklogCmd::Audit` doc comment still describes "runner completion records"/executor state.
- `crates/roko-cli/src/graph_checkpoint.rs` (lib module `roko_cli::graph_checkpoint`):
  - `canonical_checkpoint_status(workdir, plan_id)` (:1265) is the existing read-only plan-status reader, also used by `graph_execution/plan_set.rs:181`;
  - `GraphCheckpointManifest` (:445) holds `plan_id`, `run_id`, `status: GraphCheckpointStatus` (running/succeeded/failed/cancelled/interrupted), `activity_log` and `extensions`;
  - `GateVerdictSummary` (:205) is stored under `GATE_VERDICT_EXTENSION = "roko.gate.verdict@1"` and holds per-task-node `TaskGateVerdict`s, refreshed on every terminal checkpoint write;
  - `recorded_gate_verdicts` (:287) reads them from `activities.jsonl`.
- `crates/roko-graph/src/replay.rs::RecordEntry`: one line in `activities.jsonl` (`graph_id`, `run_id`, `node_id`, `tick`, `signals`). Only successful Activity executions are appended.
- `crates/roko-cli/src/orchestrator/plan_discovery.rs::find_plan_dirs` (:218): the existing recursive plan walker. It skips `archive/`, `_meta/` and dot-directories, and reports duplicate plan IDs.

## Current state

- `roko backlog audit` with `--json`/`--fix-safe`, and `backlog mark-done`, landed in `72e0a76b8`. That covers the spec's command surface, safe index fixes and explicit semantic closure.
- Still missing:
  - no Graph checkpoint reader in the audit;
  - no recursion;
  - silent skips;
  - no stable finding codes;
  - no CI read-only check.
- The Graph engine does not persist task or plan status into `tasks.toml`. Writing statuses back is a separate decision ("plan status sync"). This item only needs the audit to detect the drift.
- Unknown: whether Graph task-node IDs equal `tasks.toml` task IDs (for example `T01`). Check the plan-to-graph conversion in `crates/roko-cli/src/graph_execution/` before keying findings by node id.
- A local plan outside git (`tmp/work-management/plans/work-graph/02-plan-closes`, tasks T01 and T04) proposed this design and declares `closes = ["bug-9fc3fc", "gap-759041"]`. It is not in the repo; the relevant parts are copied into the Plan below.

## Plan

1. Add a read-only reader to `crates/roko-cli/src/graph_checkpoint.rs`:
   - `pub fn read_graph_run_summary(workdir, plan_id) -> Result<Option<GraphRunSummary>>` returns `GraphRunSummary { plan_id, run_id, status, updated_at_ms, recorded_nodes: BTreeSet<String>, gate_verdicts: BTreeMap<String, TaskGateVerdict> }`;
   - `pub fn list_graph_run_summaries(workdir) -> Vec<GraphRunSummary>` covers every directory under `.roko/state/graph/`.
   - Resolve paths the way the writer does (`safe_plan_component`), not by formatting strings by hand.
   - Migrate v2 manifests in memory as the writer does.
   - `recorded_nodes` means only the lines whose `run_id` equals the manifest's `run_id`.
   - Tolerate a truncated last line. Take no locks and write nothing.
   - Tests prefixed `run_summary_`.
2. In `backlog.rs`, replace `read_toml_plan_statuses` with a walk over `find_plan_dirs(workdir/plans)`. Record each plan that fails to parse or lacks `[meta] plan` as a finding instead of skipping it.
3. Rebuild `build_audit_report` on the summaries. Keep the Runner-v2 readers only as a fallback for plans with no Graph checkpoint, or delete them. Emit findings with stable codes, each carrying plan, task, path, `run_id` and evidence:
   - `AUDIT_RUN_SUCCEEDED_TOML_READY`: the checkpoint status is `succeeded` (or the task's gate verdict passed), but `tasks.toml` meta or the task still says `ready`/`pending`;
   - `AUDIT_TASK_DONE_NOT_RECORDED`: `tasks.toml` says done, but no succeeded run or passing verdict records the task;
   - `AUDIT_PLAN_SKIPPED`: the plan was unparseable or had no `[meta] plan`;
   - `AUDIT_ORPHAN_CHECKPOINT`: a checkpoint exists with no plan on disk;
   - keep the existing informational "failed run but TOML ready" as its own code. An interrupted or cancelled run must not count as done.
4. Output: text plus `--json` with a `findings` array. Exit non-zero when any error-class finding exists, so CI can use it read-only. `--fix-safe` keeps its current scope and never changes statuses.
5. Update the `BacklogCmd::Audit` doc comment in `main.rs`.
6. Tests in `backlog.rs`, named `audit_*`, with a tempdir fixture per code. They must include `audit_reports_graph_completed_plan_left_ready`: a nested `plans/set/01-x/tasks.toml` with `status = "ready"`, plus a `.roko/state/graph/01-x/checkpoint.json` with `status: "succeeded"`, gives an `AUDIT_RUN_SUCCEEDED_TOML_READY` finding. Also cover an interrupted run (no finding for "done"), a parse failure (`AUDIT_PLAN_SKIPPED`), and an orphan checkpoint.

## Done when

- `roko backlog audit --json` on this repo lists nested plans such as `portal-programme/*`, reads their Graph checkpoints, and reports Graph-completed plans still marked `ready`.
- Unparseable plans appear as findings.
- Findings carry stable codes and evidence. `--fix-safe` never changes a status.
- Verify: `grep -rqw 'fn audit_reports_graph_completed_plan_left_ready' crates/roko-cli/ && cargo test -p roko-cli --bin roko audit_reports_graph_completed_plan_left_ready`

## Notes

- The audit is read-only by default. Never infer completion from file existence or an agent's claim. A Git commit message is only advisory evidence.
- Do not change task or plan statuses from `audit`. Writing statuses back into `tasks.toml` after a Graph run is a separate change. File it as its own item if you want it.
- Do not reintroduce Runner-v2 files as the primary source.
- The reader in step 1 is shared with `gap-20ab07`, `bug-4cac0e` and `bug-5e7de4`. Land it in `graph_checkpoint.rs` with a stable signature, and do not start those items in parallel on the same file.
- Touches `commands/backlog.rs`, `graph_checkpoint.rs` (reader only) and the `main.rs` doc comment. Low conflict risk otherwise.
- Size M.

## Original notes

stale indexes and plan status can re-dispatch completed work or hide unfinished work. Dogfooding found 11 `tasks.toml` files still marked ready after their implementation had merged. The 2026-08-31 backlog index also continued to list several PR #64–#72 items as open even though the current code…

Imported without verification from:
- `tmp/backlog/archive/229-backlog-plan-state-reconciliation.md#229 — Backlog and Plan State Reconciliation`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.6 Proof Case 6: Backlog/plan state reconciliation`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `scripts/check_backlog_integrity.*`.

How to verify: Check: `roko backlog audit --json` reports every mismatch with stable codes, paths, IDs, evidence, and; A fully completed plan cannot remain silently `status = "ready"` without an audit failure.; An archived spec with a stale root link is reported… [evidence: own status: Overall: NOT STARTED. No implementation work has landed for `roko backlog audit` or reconciliation logic. No…; 00-STATUS-SUMMARY 3. Open / P1 -- High…]

Verified 2026-09-28: `roko backlog audit` exists (commands/backlog.rs::cmd_backlog_audit, --json/--fix-safe, landed 72e0a76b8), but runner-side evidence comes only from Runner-v2 files (state-snapshot.json, executor snapshot, run-state.json). Graph checkpoints under .roko/state/graph/ are never read, and the Graph path does not write task status back to tasks.toml, so a Graph-completed plan left `ready` is not detected.

Re-verified 2026-09-29: still open, and commands/backlog.rs has not changed since 244f564e1. bug-9fc3fc (filed 2026-09-28) describes the same defect and adds two fix criteria, which belong here. First, read_toml_plan_statuses (backlog.rs:278-308) scans only plans/*/tasks.toml, so nested plan sets are invisible; it should recurse the way plan validate does. Second, plans that fail to parse or lack [meta] plan are skipped silently and should be reported. Plan tmp/work-management/plans/work-graph/02-plan-closes T04 declares that it closes both items.
