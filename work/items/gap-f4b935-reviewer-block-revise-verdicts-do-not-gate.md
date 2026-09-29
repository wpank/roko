+++
id = "gap-f4b935"
kind = "gap"
title = "Reviewer BLOCK/REVISE verdicts do not gate Graph tasks, so nothing ever produces forced_accept"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-dispatch", "roko-gate/review"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e1-verdict"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::settle_task_verification", "crates/roko-gate/src/review_verdict.rs::parse_structured_review_verdict", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
links = { depends_on = [], blocks = [], related = ["bug-6dc672", "bug-82d47b", "gap-85f102"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "parse_structured_review_verdict" crates/roko-cli/src'
+++

Since bug-82d47b, failing authored verify steps can no longer be force-accepted, and every Graph task output carries a `roko.gate.verdict` tag. The Graph dispatcher still never evaluates a review or judge verdict:

- reviewer-role tasks that answer `BLOCK` or `REVISE` complete as successes;
- `gates.max_review_cycles` has no effect on `roko plan run`;
- `TaskGateVerdict::ForcedAccept` is consumed (resume, TUI bridge, snapshot) but produced only in tests, so the amber `accepted_with_failures` rendering from bug-6dc672 is exercised only by tests;
- plan-level success (`execution_succeeded` in plan_runner.rs, and `GraphCheckpointStatus`) ignores verdicts and will need to show forced accepts once something produces them.

Fix: parse structured review verdicts (`roko_gate::review_verdict::parse_structured_review_verdict`) on the Graph path, loop REVISE up to the cap, emit `ForcedAccept` when the cap is hit, and surface forced accepts in the plan outcome.
