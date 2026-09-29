+++
id = "gap-09e478"
kind = "gap"
title = "Dogfood Session Evidence Bundle"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "tooling"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f8e08a8a5"
source = "tmp/backlog/228-dogfood-session-evidence-bundle.md#228 — Dogfood Session Evidence Bundle"
discovered_from = "audit:tmp/backlog/228-dogfood-session-evidence-bundle.md#228 — Dogfood Session Evidence Bundle"
anchors = ["scripts/run_evidence.py::DEFAULT_APPEND_LOGS", "scripts/run_evidence.py::validate_bundle", "dev.sh::cmd_fast", "crates/roko-cli/src/graph_execution/event_log.rs::run_recorded", "crates/roko-cli/src/runner/status_file.rs::write_status_debounced", "plans/portal-programme/_harness/fake-claude"]
links = { depends_on = [], blocks = [], related = ["bug-f7943a", "bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'state/graph' scripts/run_evidence.py && test -f scripts/test_run_evidence_graph.py && cargo build -p roko-cli && python3 scripts/test_run_evidence_graph.py"
+++

## Problem

The evidence-bundle harness (`./dev.sh run-evidence`, `./dev.sh fast`, `./dev.sh evidence-validate`, all backed by `scripts/run_evidence.py`) was built and proven against the Runner-v2 surfaces. Runner-v2 was deleted on 2026-09-06, and no bundle from a real Graph-engine plan run has been produced and validated. Concretely:

- The collector slices these append-only logs by offset and `run_id` (`DEFAULT_APPEND_LOGS`, `run_evidence.py:52-56`): `.roko/events.jsonl`, `.roko/state/run-ledger.jsonl`, `.roko/run-ledger.jsonl`. A Graph run writes its state to `.roko/state/graph/<plan>/` (`checkpoint.json`, `activities.jsonl`, `costs.json`), which the collector never reads. Only `crates/roko-cli/src/runner/persist.rs` refers to `run-ledger.jsonl`.
- The collector samples `--status-file`, which defaults to `.roko/state/status.json` (`run_evidence.py:2444-2448`). Nothing in production writes that file any more: `runner/status_file.rs::write_status_debounced` and `write_status_immediate` have no callers, and only the readers in `status.rs` and the TUI snapshot remain. Status samples from a Graph run are therefore empty.
- A bare `roko plan run` (without `--log-file`) writes no `.roko/events.jsonl` (`bug-230de6`), so a `run-evidence -- roko plan run ...` bundle has no events unless the caller passes `--log-file {bundle}/events.jsonl`, as `dev.sh fast` does.
- Of the original verification checklist, only the loopback endpoint/CLI smoke run passed on a real binary (2026-08-31, Runner-v2 era). Three Graph-engine runs are still missing:
  - a one-task success;
  - an agent that exits before its first event (expected: a `lost_effect`-style terminal record plus a diagnosis);
  - a gate timeout (expected: timing, timeout kind and ledger event agree).

Expected: `./dev.sh fast plans/<one-task-plan>`, or `./dev.sh run-evidence --require-events -- target/debug/roko plan run <plan> --no-tui --log-file {bundle}/events.jsonl`, produces a bundle that includes the Graph checkpoint/activity evidence and passes `./dev.sh evidence-validate <bundle>` for those three scenarios.

## Why it matters

- Goal `tooling`: this makes developing roko with roko debuggable. Self-hosting failures are expensive to reproduce. Without a validated Graph bundle, dogfood evidence is again hand-assembled from terminal logs, state files and worktrees, and an old defect cannot be told apart from a regression.
- It is the evidence path for the "fresh self-hosting proof" priority in `CLAUDE.md`, and FAST mode depends on it (`gap-4a6dcb`).
- Related:
  - `bug-230de6`: Graph default engine writes no `.roko/events.jsonl`; `crates/roko-cli/tests/default_engine.rs:11` is still `#[ignore]`;
  - `bug-f7943a`: `dev.sh fast` used the removed `--engine runner-v2`. Done in `725f21e05`;
  - `gap-4a6dcb`: the rest of FAST mode on Graph.

## Where

- `scripts/run_evidence.py` (3,406 lines, tracked):
  - `DEFAULT_APPEND_LOGS` (:52) lists the logs to slice;
  - `validate_events_jsonl` (:804) checks lifecycle and `run_id`;
  - `validate_bundle` (:2140) requires exactly one run start, one terminal and one `run_id` (:2320-2340);
  - `RUN_START_NAMES`/`RUN_TERMINAL_NAMES` (:78, :86) already include `run.started`/`run.completed`;
  - `--status-file` (:2444) and `--require-events` (:2558) are CLI options.
- `dev.sh`: `cmd_run_evidence` (:87), `cmd_evidence_validate` (:93), `cmd_fast` (:126). FAST runs `plan run <dir> --no-tui --max-retries N --log-file {bundle}/events.jsonl --max-tasks N` (:309-319) under `run_evidence.py` with a deadline.
- `crates/roko-cli/src/graph_execution/event_log.rs::run_recorded` (:129): the `--log-file` writer. Its format is one `run.started` line, `dashboard.<kind>` lines wrapping `DashboardEvent`s, `log.lagged`, and one `run.completed` line with `outcome` and `exit_code` (module doc :8-14).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan`: the Graph plan run (entry point `roko plan run`).
- `crates/roko-cli/src/runner/status_file.rs`: Runner-v2 status writer, now unused in production.
- `plans/portal-programme/_harness/fake-claude`: a deterministic Python stand-in for the `claude` CLI. Point `[providers.claude_cli] command` at it to run real plans in seconds at no cost. It supports `SLOW <n>`, `ARTIFACT <path>`, live streaming, and a slow knob in `.roko/fake-claude-slow`.
- Docs: `docs/v2/30-EVIDENCE-BUNDLES.md` (schema-v2 operator contract), `docs/v2/29-FAST-DEVELOPMENT.md`.

## Current state

- Harness code is complete for schema v2: bundle lifecycle, redaction, metrics, score, debrief and validation (`bba2f8858`, `25aaca597`; FAST path `a58bdbacb`).
- `725f21e05` fixed the FAST command line (no more `--engine runner-v2`), and `--log-file` now writes JSONL through `run_recorded` with one `run.started` and one `run.completed` per `run_id`, which the validator accepts.
- A provider-free synthetic proof of all seven checklist scenarios exists, but only locally: `scripts/test_run_evidence_runtime.py` plus `scripts/evidence_runtime_fixture.py`. Both are ignored by `.gitignore:95` (`scripts/*`), so they are not in git and will not exist in a fresh worktree. The fixture imitates Runner-v2 surfaces (`.roko/state/run-ledger.jsonl`, `.roko/state/status.json`, `failure_kind: "lost_effect"`), not the Graph engine.
- Unknown: what a Graph run records when the agent exits before its first event, and when a gate times out. Check the `dashboard.*` event kinds and `activities.jsonl` records for task failure and gate timeout before choosing assertions.

## Plan

1. Port the collector to Graph surfaces in `scripts/run_evidence.py`:
   - add the per-plan Graph activity log `.roko/state/graph/<plan>/activities.jsonl` to the sliced append logs. The plan directory name is only known after launch, so glob `.roko/state/graph/*/activities.jsonl` and record pre-launch offsets for every existing file;
   - at terminal state, copy the bounded `checkpoint.json` and `costs.json` for the plan(s) the run touched;
   - capture `roko diagnose <plan-id>` output if it is not already collected;
   - keep the Runner-v2 paths as optional, `skipped` when absent.
2. Status sampling. Options:
   - (a) point `--status-file` at something the Graph run writes;
   - (b) sample `roko status --json` or the serve `/api/statehub/snapshot` endpoint;
   - (c) revive `runner/status_file.rs::write_status_debounced` from the Graph `StateHub`.
   - Recommendation: (c) if cheap, because `roko status` and the TUI still read that file (`status.rs:236`, `tui/state/snapshot.rs:38`). Otherwise record status sampling as `skipped` for Graph runs, so the bundle is honest about it.
3. Write a tracked end-to-end test, e.g. `scripts/test_run_evidence_graph.py` (Python `unittest`, force-added with `git add -f` like `run_evidence.py`). It uses the prebuilt `target/debug/roko` and `fake-claude`, in a temp workspace with a `roko.toml` that points `[providers.claude_cli] command` at the fake. Scenarios:
   - a one-task plan succeeds. The bundle validates with `--require-events`: one `run.started`, one `run.completed`, one `run_id`, and the Graph checkpoint present;
   - a fake-claude mode that exits non-zero before printing any stream event (add a prompt knob such as `EXIT_EARLY`). The bundle shows a failed terminal state with a diagnosis, never "succeeded";
   - a task whose `verify` command sleeps past the gate timeout. The bundle's timing and the timeout kind agree with the activity log;
   - a pre-populated `.roko/events.jsonl` or activity log from an older run. The bundle holds only the new `run_id`.
   Reuse helpers from the local `scripts/test_run_evidence_runtime.py` where useful. Copy code rather than import it, since that file is untracked.
4. Update `docs/v2/30-EVIDENCE-BUNDLES.md` to list the Graph surfaces and drop the Runner-v2-only ones from "required".
5. Run `./dev.sh fast` once on a real one-task plan with the fake provider and keep the bundle path as closure evidence.

## Done when

- A Graph-engine bundle for a one-task success, an early agent exit and a gate timeout each passes `./dev.sh evidence-validate`. The failure cases are never labelled successful.
- Bundles include the Graph checkpoint/activity evidence sliced to the run's `run_id`.
- The end-to-end test is tracked in git and passes.
- Verify (new; the item has none): `grep -q 'state/graph' scripts/run_evidence.py && test -f scripts/test_run_evidence_graph.py && cargo build -p roko-cli && python3 scripts/test_run_evidence_graph.py`

## Notes

- Do not edit `.gitignore` (standing rule: ask first). Track new files under `scripts/` with `git add -f`, and mention it in the commit.
- `fake-claude` is modified in the working tree of the main checkout (`plans/portal-programme/_harness/fake-claude`). Base the work on the committed version or coordinate with the portal-programme work, and add the new knob in a backwards-compatible way.
- Never let the bundle persist secrets or the full environment. The redaction pass and the secret-leak validator check must stay on.
- `bug-230de6` (bare runs write no `.roko/events.jsonl`) is not a hard dependency, because the fixtures pass `--log-file {bundle}/events.jsonl`. If it lands first, add a bare-run scenario.
- Mostly Python and shell. Safe to run in parallel with Rust items, except that fixing status sampling (option c) touches `graph_execution/`.
- Size M.

## Original notes

[partial] SOURCE-IMPLEMENTED; STRICT LOOPBACK/CLI BUNDLE SMOKE VERIFIED (2026-08-31, `bba2f8858` + `25aaca597`)… — self-hosting failures are expensive to reproduce, and today's evidence is assembled manually across terminal logs, runner state, HTTP responses, screenshots, and Git worktrees. The…

Imported without verification from:
- `tmp/backlog/228-dogfood-session-evidence-bundle.md#228 — Dogfood Session Evidence Bundle`
- `tmp/backlog/archive/228-dogfood-session-evidence-bundle.md#(archived copy; status: SOURCE-IMPLEMENTED; STRICT LOOPBACK/CLI BUNDLE SMOKE…)`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.7 Proof Case 7: Evidence bundle validation`
- `tmp/archive/backlog-closure-2026-09-01.md#Items receiving verification updates (228)`
- `tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`

How to verify: Check: Run a one-task successful plan and validate the resulting bundle.; Run a mock agent that exits before its first event; verify the bundle records `lost_effect` or; Run a mock gate timeout; verify timing, timeout kind, and ledger event agree. [evidence: own status: Benchmark evidence collection is now scriptable via `scripts/run_benchmark_evidence.sh` (safe by default…; 00-STATUS-SUMMARY 2. Partial /…] / Check recent tmp/dogfood session reports for evidence-bundle usage.

Merged 2 mined candidates: m1-007, m4-012.

Verified 2026-09-28: still partial - scripts/run_evidence.py exists (its validator requires exactly one run start/terminal in the bundle events.jsonl, run_evidence.py:2320-2332), but the FAST bundle path is broken (dev.sh:307 passes the removed --engine runner-v2; bug-f7943a notes the Graph --log-file sink is never written) and bare Graph runs write no .roko/events.jsonl (bug-230de6), so no Graph-engine bundle has been validated.

Re-verified 2026-09-29: the FAST bundle path is fixed. 725f21e05 removed --engine runner-v2 from dev.sh::cmd_fast. --log-file now writes a JSONL log through graph_execution/event_log.rs::run_recorded, with one run.started and one run.completed per run_id, which matches the lifecycle names that run_evidence.py validates. Three things remain. First, a bare `roko plan run` without --log-file still writes no .roko/events.jsonl (bug-230de6; tests/default_engine.rs:11 is still ignored). Second, no Graph-engine bundle has been validated end to end; the missing runs are a one-task success, a mock agent exiting before its first event (lost_effect), and a mock gate timeout. Third, the rest of FAST mode is tracked as gap-4a6dcb.

Implemented 2026-09-29 on `work/gap-09e478` (wk-evidence). I re-checked the premise at `f8e08a8a5`. It still held: the collector did not read `.roko/state/graph`, and `runner/status_file.rs` has no production writer. The `fake-claude` in the main checkout matches BASE again.

Two facts about the Graph path changed the plan:

- A failed task writes no Activity record.
- Each Graph checkpoint keeps its own run ID (`graph-<plan>-<uuid>`), separate from the `--log-file` run ID.

So the collector slices `activities.jsonl` by the checkpoint's run ID, starting at the pre-launch offset only when a run resumes the same checkpoint. It takes the evidence for a failure from the event log, the learning ledgers and `roko diagnose`.

The Graph engine writes no `lost_effect` record. An agent that exits before its first event ends as a failed `dashboard.task_completed` with no gate verdict, which the bundle records as `failed_before_gate`.

Status sampling is recorded as `skipped`. Option (c), writing the status file, needs `plan_runner.rs`, which another worker owns.

Found during this work: for a verify step that overran its `timeout_ms`, `.roko/learn/gate-failures.jsonl` records `failure_kind = "permanent"` and `roko diagnose` reports `timed_out_attempts = 0`. The `dashboard.gate_result` event says `timed out after 1500 ms`. Bundles report the disagreement as a validation warning.
