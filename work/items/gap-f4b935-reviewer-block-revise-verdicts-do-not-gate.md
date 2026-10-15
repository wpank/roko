+++
id = "gap-f4b935"
kind = "gap"
title = "Reviewer BLOCK/REVISE verdicts do not gate Graph tasks, so nothing ever produces forced_accept"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph-dispatch", "roko-gate/review"]
created = 2026-09-29
updated = 2026-10-15
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e1-verdict"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-gate/src/review_verdict.rs::parse_structured_review_verdict", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
links = { depends_on = [], blocks = [], related = ["bug-6dc672", "bug-82d47b", "gap-85f102"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'parse_structured_review_verdict' crates/roko-cli/src/graph_task_dispatch && grep -rqw 'fn reviewer_verdict_is_recorded_and_listed' crates/roko-cli/ && cargo test -p roko-cli --lib reviewer_verdict_is_recorded_and_listed"
+++

Since bug-82d47b, failing authored verify steps can no longer be force-accepted, and every Graph task output carries a `roko.gate.verdict` tag. The Graph dispatcher still never evaluates a review or judge verdict:

- reviewer-role tasks that answer `BLOCK` or `REVISE` complete as successes;
- `gates.max_review_cycles` has no effect on `roko plan run`;
- `TaskGateVerdict::ForcedAccept` is consumed (resume, TUI bridge, snapshot) but produced only in tests, so the amber `accepted_with_failures` rendering from bug-6dc672 is exercised only by tests;
- plan-level success (`execution_succeeded` in plan_runner.rs, and `GraphCheckpointStatus`) ignores verdicts and will need to show forced accepts once something produces them.

Fix: parse structured review verdicts (`roko_gate::review_verdict::parse_structured_review_verdict`) on the Graph path, loop REVISE up to the cap, emit `ForcedAccept` when the cap is hit, and surface forced accepts in the plan outcome.

## Notes

- 2026-10-01 (wk-honestbench): blocked on design decisions; no code changed. The premise holds at BASE
  `ebdc0f5d5`: nothing under `crates/roko-cli/src` calls `parse_structured_review_verdict`, `ForcedAccept` is
  produced only in tests, and `settle_task_verification` says so (`graph_task_dispatch/verification.rs:756-760`).
  Open questions:
  - What is reviewed: the output of `reviewer`-role tasks, or a review step after an implementer task?
  - What REVISE does in a DAG. A reviewer node that fails only retries itself, and the Graph engine cannot re-run
    the upstream node it reviewed. Runner-v2's loop instead re-dispatched the implementer with the review, up to
    `max_review_cycles`, inside one task.
  - What an unstructured review means. `parse_structured_review_verdict` fails closed to `needs_human`, so gating
    on it would fail every existing reviewer task that answers in prose.
  - How `execution_succeeded` (`graph_execution/plan_runner.rs:2909`) and `GraphCheckpointStatus` count forced
    accepts.

  Next step: choose a review-cycle model and an unstructured-output policy. A safe first step is then to parse and
  record reviewer verdicts in the attempt record, advisory only, and measure how often BLOCK/REVISE would fire
  before gating on them.

2026-10-15 decision (Will delegated it to the coordinator's recommendation, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md, decision 9; he can veto it): Advisory first. What is reviewed: the output of reviewer-role tasks only, with no new review step. REVISE in a DAG: not now. The Graph path parses each reviewer task's answer with parse_structured_review_verdict and records the verdict on the attempt record (prose answers are recorded as needs_human), and the run summary lists reviewer tasks that answered BLOCK or REVISE. The verdict gates nothing, no ForcedAccept is produced, and execution_succeeded is unchanged. Re-scope the item to that step and replace its verify with `grep -rq 'parse_structured_review_verdict' crates/roko-cli/src/graph_task_dispatch && cargo test -p roko-cli --lib reviewer_verdict_is_recorded_and_listed`. Gating and review cycles are decided once the records show how often BLOCK and REVISE fire. Why: Nothing under crates/roko-cli/src calls parse_structured_review_verdict today (grep), so a reviewer that answers BLOCK completes green. That is a silent false green, and listing it fixes the silence. Gating now would fail every prose reviewer, because the parser fails closed to needs_human. REVISE cannot re-run the reviewed upstream node in the Graph engine, which runs each task's node independently. execution_succeeded is computed at crates/roko-cli/src/graph_execution/plan_runner.rs:3773. Recording and listing is the cheapest reversible step and gives the data for the gating decision. If vetoed: With gating now, a structured BLOCK fails the reviewer task, REVISE retries it up to gates.max_review_cycles and then emits ForcedAccept (the item's original verify), and prose reviews fail as needs_human.
