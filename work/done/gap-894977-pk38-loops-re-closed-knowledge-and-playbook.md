+++
id = "gap-894977"
kind = "gap"
title = "PK38 Loops re-closed: Knowledge and playbook withhold arms: a withheld source stays out of the prompt and is… (+5 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 38
size = "L"
subsystem = ["roko-cli/learning"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "c1eb6c2c5"
source = "tmp/backlog/2026-10-02-complete-and-wire PK38"
anchors = ["crates/roko-cli/src/dispatch/mod.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/graph_task_dispatch/wiring.rs", "crates/roko-cli/tests/learning_wiring_census.rs", "crates/roko-compose/src/role_prompts.rs"]
lane = "rust-hot"
parent = "spec-446a41"
links = { depends_on = ["gap-cc5051", "gap-f61823", "gap-b5caf3", "gap-aea13a", "gap-943046"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn withhold_arm_omits_sections_and_logs_propensity' crates/roko-cli/src/ && cargo test -p roko-cli withhold_arm_omits_sections_and_logs_propensity"

[[verify]]
command = "grep -rqw 'fn pinned_sections_never_excluded' crates/roko-cli/src/ && cargo test -p roko-cli pinned_sections_never_excluded"

[[verify]]
command = "grep -rqw 'fn labelled_attempt_updates_section_posteriors' crates/roko-cli/src/ && cargo test -p roko-cli labelled_attempt_updates_section_posteriors"

[[verify]]
command = "! grep -rq 'retrieval_outcomes_path\\|retrieval_ctx' crates/roko-cli/src/"

[[verify]]
command = "grep -q 'const EXPECTED_MISSING: &\\[&str\\] = &\\[\\];' crates/roko-cli/tests/learning_wiring_census.rs && cargo test -p roko-cli --test learning_wiring_census"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T07:36:24Z"
commit = "c1eb6c2c5"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T04:13:02Z"
forced = false
evidence = "Gate 7b (work/backlog-batch-7b, merged into main as c1eb6c2c5): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 12,124 tests over roko-agent, -cli, -compose, -core, -fs, -gate, -graph, -learn and -serve (one OpenAPI coverage failure fixed in 484e172fe), roko-cli bin 430 passed, the golden-path canaries pass incl. golden_path_acceptance's fixture plan (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn, roko-graph and roko-agent integration tests pass, each parked feature builds (fault-injection lib 1,321), PK79's tree and chain checks pass; every [[verify]] passes. PK38 5/6 done, 4121 dropped by decision 4115 (no L-err arm); withhold arms, the section bandit and its persistence, retrieval-outcomes retired, the census's loop states. Gate fixes 08e85d2e4 (a test's DispatchContext) and 71ff4a6d0 (lints); the graph_task_dispatch.rs conflict with PK76 resolved in the merge."
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK38, slice 41xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 4120 | M | p2 | Knowledge and playbook withhold arms: a withheld source stays out of the prompt and is logged | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4120-knowledge-and-playbook-withhold-arms.md` |
| 2 | 4121 | S | p3 | Error-pattern withhold arm (L-err), if decision 4115 approves a third factor | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4121-error-pattern-withhold-arm.md` |
| 3 | 4123 | M | p2 | Prompt assembly drops droppable sections by the bandit's draw, never pinned ones | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4123-prompt-assembly-drops-sections-by-bandit-draw.md` |
| 4 | 4124 | S | p2 | Settle section posteriors from labelled attempts and save them when the run ends | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4124-settle-section-posteriors-and-save-at-run-end.md` |
| 5 | 4129 | S | p3 | Stop writing `retrieval-outcomes.jsonl` once exposures record what reached the prompt | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4129-stop-writing-retrieval-outcomes-jsonl.md` |
| 6 | 4131 | S | p2 | Census green: every Graph learning loop is live, observe-only or retired | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4131-census-green-every-loop-live-or-retired.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4100-loops-reclosed-on-verified-outcomes.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/dispatch/mod.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/feedback.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-cli/src/graph_task_dispatch/wiring.rs`, `crates/roko-cli/tests/learning_wiring_census.rs`, `crates/roko-compose/src/role_prompts.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK09 (gap-cc5051), PK10 (gap-f61823), PK32 (gap-b5caf3), PK33 (gap-aea13a), PK35 (gap-943046).
- Suggested model: opus.

## Progress

- 4120: implemented at d1fea5585 (cargo verification deferred to the batch gate). L-know and L-play withhold arms keep a withheld source's section out of the prompt; its items are logged `withheld_arm` and its ids are not credited.
- 4121: blocked: decision 4115 approves no L-err withhold arm (L-err stays observe-only), so `withheld_error_patterns_leave_the_prompt` is not written and this item's second `[[verify]]` cannot pass; drop that entry or close 4121 won't-fix.
- 4123: implemented at 796af57f1 (cargo verification deferred to the batch gate). PINNED_SECTIONS / DROPPABLE_SECTIONS with a const disjointness check, `[sections] pinned`, per-section bandit draws on L-sec's learned arm, `bandit_excluded` items and section_decisions, sections decision rows with p = 1 - p_ex. Graph tests that need learned content in the prompt now run in maximize mode (arm sets draw on the UTC day).
- 4124: implemented at afbab062f (cargo verification deferred to the batch gate). SectionOutcomes on the feedback context (None when frozen), settled in emit_feedback, folded into `.roko/learn/section-bandit.json` under its lock at run end; `sink.section_effect` wired, EXPECTED_MISSING empty.
- 4129: implemented at 8cd3972da (cargo verification deferred to the batch gate). RAG-10 writes, `retrieval_ctx` and `retrieval_outcomes_path` removed; the TUI and serve readers keep the historical file.
- 4131: implemented at 80abfe778 and a9a495a15 (cargo verification deferred to the batch gate). Census adds store.arm_set, reader.withhold_arms, store.placebo, sink.router_source_credit and a live/observe_only/retired state per registry loop; the fixture shows the section and playbook joins. Not shown: "a verified pass reinforces the included knowledge" (the fixture has no knowledge store).
- gap-29fe0a: implemented at a0da86af0 (cargo verification deferred to the batch gate). Maximize mode zeroes the spec gate's holdout_frac (plan-load gate and plan run's pre-check); test `no_holdout_flag_draws_no_holdout_attempts`.
- 2026-10-03 (coordinator, gate 7b): task 4121 (an L-err withhold arm) is not done and its verify left this item: decision 4115 (DECISIONS.md, 2026-10-02) confirmed "no L-err arm", so the loop stays observe-only.
