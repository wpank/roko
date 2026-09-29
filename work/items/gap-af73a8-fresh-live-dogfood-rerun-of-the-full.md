+++
id = "gap-af73a8"
kind = "gap"
title = "Fresh live dogfood rerun of the full self-hosting workflow (blocked by SnapshotRebased arm gap)"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["self-hosting"]
created = 2026-09-15
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "docs/v3/39-ROADMAP.md#7.5 Fresh Dogfood Proof"
discovered_from = "audit:docs/v3/39-ROADMAP.md#7.5 Fresh Dogfood Proof"
anchors = ["CLAUDE.md", "crates/roko-cli/src/prd.rs", "crates/roko-cli/src/research.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "set -- work/history/dogfood-rerun-*.md; test -f \"$1\" && grep -q \"prd idea\" \"$1\" && grep -q \"prd draft\" \"$1\" && grep -q \"research enhance-prd\" \"$1\" && grep -q \"prd plan\" \"$1\" && grep -q \"plan run\" \"$1\" && grep -Eq \"graph-(run-)?[0-9a-f]\" \"$1\""
+++

## Problem

Nobody has recorded a live, end-to-end run of the self-hosting workflow in `CLAUDE.md` since the first dogfood
run on 2026-08-13. That workflow is: `roko prd idea` → `roko prd draft new` → `roko research enhance-prd` →
`roko prd plan` → `roko plan run` → (resume) → `roko status`. Each piece has been exercised on its own:
- a pre-execution proof on 2026-09-15 ran `doctor`, `prd idea`, `plan list`, `plan validate`, `status` and
  `learn all`, with no agent dispatch;
- the portal programme (plans `01`-`08g`, September 2026) ran many live `roko plan run` executions, but on
  hand-written plans.
No run has started from `roko prd idea` and carried one work item through PRD, research, plan generation,
execution and status with real agents.

The title's blocker is obsolete. The 2026-09-15 build failure (`DashboardEvent::SnapshotRebased` not
matched in `format_dashboard_event`) is fixed: `crates/roko-cli/src/runner/output_sink.rs:1493` now has the
arm. The remaining work is to run the workflow and record the result.

## Why it matters

- Goal `core` (plan runs work reliably). `CLAUDE.md` lists "Fresh self-hosting proof" as long-term priority 1.
  Roko's premise is that it develops itself. Until one full pass is recorded, that claim rests on component
  tests and hand-written plans.
- The run will surface defects in the PRD and plan-generation half of the loop, which the portal runs never
  touched. File each one as a work item.
- Related: `gap-09e478` (dogfood evidence bundle; use it to record this run), `bug-230de6` (a bare
  `plan run` writes no `.roko/events.jsonl`; pass `--log-file`), `bug-a5cd6b` (`roko prd plan` also
  regenerates every old-format plan under `plans/` with LLM calls), `bug-165b22` (`roko diagnose` does not
  work on Graph runs). The parked `gap-5a4afd` and `gap-b41d36` ask for the same rerun; mark them
  `duplicate_of = "gap-af73a8"`.

## Where

Commands and the code behind each step (entry points are the CLI subcommands):
- `roko prd idea|draft|plan`: `crates/roko-cli/src/commands/prd.rs` (dispatch), `crates/roko-cli/src/prd.rs`
  (PRD storage in `.roko/prd/`, plan generation; `regenerate_old_format_plans` is reached from the CLI
  `prd plan` path, see `bug-a5cd6b`).
- `roko research enhance-prd`: `crates/roko-cli/src/research.rs` (Perplexity-backed; artifacts in
  `.roko/research/`).
- `roko plan run <dir>`: `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan` (line 701).
  The Graph engine is the only executor. Run ids look like `graph-run-<millis>`. State lands in
  `.roko/state/graph/<plan>/` (`checkpoint.json`, `activities.jsonl`, `costs.json`).
- Evidence: `--log-file <path>` writes a JSONL run log (`graph_execution/event_log.rs::run_recorded`);
  `scripts/run_evidence.py` validates bundles (`gap-09e478`).
- Provider keys: `load_startup_env_files` (`crates/roko-cli/src/main.rs:4498`) auto-loads `~/.roko/.env` and
  `<workdir>/.roko/.env`.
- `crates/roko-cli/tests/e2e_self_host.rs::self_host_smoke_full_pipeline`: a synthetic in-process test of
  DAG, projection and resume. It runs no agents and does not count as this proof.

## Current state

- Blocker fixed (`output_sink.rs:1493`). Runner-v2 was deleted on 2026-09-06, so the rerun exercises the
  Graph engine. `roko prd` and `roko do` no longer call the removed runner (`bug-96aff4`, done), and
  `./dev.sh fast` no longer passes `--engine runner-v2` (`bug-f7943a`, done).
- Evidence that exists: `tmp/dogfood/2026-09-20-final-session.md` (local only) says a fresh run had 4 of 5 demo
  plans working end to end; the 2026-09-25 portal-programme run log records live `plan run` executions. Neither
  covers `prd idea` → `prd plan`.
- Known hazards: `bug-a5cd6b` (extra LLM calls and rewritten plan files during `prd plan`), `bug-230de6`
  (no events file without `--log-file`), `bug-165b22` (use `.roko/learn/gate-failures.jsonl` and
  `.roko/roko.log.<date>` to diagnose failed tasks instead of `roko diagnose`).

## Plan

1. Prepare: a fresh git worktree of the roko repo on its own branch, with a clean tree (`roko doctor` warns about
   dirty trees, and `plan run` creates task worktrees). Build `target/debug/roko` once. Confirm providers with
   `roko config providers health`. The Claude CLI may hit session limits; API-key providers from `~/.roko/.env`
   are the fallback.
2. Pick a real, small open item as the subject, so the run produces a useful change: for example `bug-1cb461`
   (S, one file, has a named test). Avoid items touching auth, persistence or migrations.
3. Run the workflow, saving each command's output:
   - `roko prd idea "<one-line description>"`
   - `roko prd draft new "<slug>"`
   - `roko research enhance-prd <slug>`
   - `roko prd plan <slug>`; then `git status` to see whether `bug-a5cd6b` rewrote other plans. Revert
     those files if it did.
   - `roko plan validate plans/<generated-dir>`
   - `roko plan run plans/<generated-dir> --log-file <evidence-dir>/run.jsonl`. Run only the generated plan
     directory. `plans/` itself holds every plan in the repo, including the portal programme.
   - If interrupted, `roko plan run plans/<generated-dir> --resume-plan`.
   - `roko status` and `roko show costs`.
4. Record the run in `work/history/dogfood-rerun-<YYYY-MM-DD>.md`: the commit it ran at, each command with its
   outcome, the Graph run id(s), task outcomes, cost, time, the evidence-bundle path (validated with
   `scripts/run_evidence.py` if `gap-09e478` allows), and a list of defects found with the IDs of the items filed
   for them.
5. File each blocker or defect found as a work item (per `work/README.md`), and link it from the record.
6. Mark `gap-5a4afd` and `gap-b41d36` as duplicates of this item.

## Done when

- One complete pass from `prd idea` to `status` has run with live agents at a recorded commit. If a step
  fails, the record says where and which item tracks it. The item closes when the pass completes, possibly
  after fixes and a second attempt.
- The record file exists and names the commands and the Graph run id.
- Verify (the item has no `[[verify]]` yet); suggested:
  `set -- work/history/dogfood-rerun-*.md; test -f "$1" && grep -q "prd idea" "$1" && grep -q "prd draft" "$1" && grep -q "research enhance-prd" "$1" && grep -q "prd plan" "$1" && grep -q "plan run" "$1" && grep -Eq "graph-(run-)?[0-9a-f]" "$1"`

## Notes

- Costs real money and provider quota (PRD drafting, research, plan generation, agent tasks). Confirm the budget
  with the owner before starting.
- Git: every commit, push, merge or branch deletion needs the owner's explicit approval each time. Never push to
  `main`. Do not delete worktrees or plan branches afterwards; they are kept for inspection.
- Landing `bug-a5cd6b` first is recommended but not required. Without it, check and revert stray plan rewrites
  after `prd plan`.
- The title still says "blocked by SnapshotRebased arm gap"; that blocker is gone and the title can drop it.
- Not parallel-safe with other live plan runs in the same workspace; use a separate worktree.

## Original notes

Full live agent-dispatch rerun of prd→plan→run has not been performed since the 2026-08-13 fixes; roadmap says it was blocked by a dirty-tree build failure from the SnapshotRebased arm gap in output_sink.rs.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#7.5 Fresh Dogfood Proof`
- `tmp/refactoring-audit/P1-06-DOGFOOD-PROOF.md`

Warning: every file this item cites is gone (`crates/roko-cli/src/**/output_sink.rs`) — likely obsolete or moved.

How to verify: cargo check -p roko-cli; grep SnapshotRebased match arms; then run the CLAUDE.md self-hosting workflow.

Verified 2026-09-28: the blocker is gone, but the full rerun is still unrecorded. output_sink.rs:1493 now handles DashboardEvent::SnapshotRebased, so the build failure cited in the title no longer applies. Some live evidence exists: tmp/dogfood/2026-09-20-final-session.md:170 records a fresh run in which 4 of 5 demo plans worked end to end, and the 2026-09-25 portal-programme run log records live `plan run` executions. None of these records the full prd idea -> draft -> plan -> run -> status workflow. What remains is to run that complete pass and record it.

Re-checked 2026-09-29: the portal programme (plans 01-08g) ran many live `plan run` executions built by roko, but none started from `roko prd idea` / `prd draft` / `prd plan`, so the full self-hosting pass is still unrecorded. The SnapshotRebased blocker in the title no longer applies (output_sink.rs:1493), so the title should drop it. What remains is one live run of the CLAUDE.md self-hosting workflow, recorded with its run ids (see gap-09e478 for the evidence bundle). The parked, unverified gap-5a4afd and gap-b41d36 (both also created 2026-09-15) ask for the same rerun and can be marked duplicate_of this item.
