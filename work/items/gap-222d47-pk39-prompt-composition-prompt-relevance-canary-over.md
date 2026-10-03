+++
id = "gap-222d47"
kind = "gap"
title = "PK39 Prompt composition: Prompt-relevance canary: over two scripted runs, each prompt holds only its own task's… (+3 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
rank = 39
size = "L"
subsystem = ["roko-cli/tests"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK39"
anchors = ["crates/roko-cli/src/dispatch/factory.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-compose/src/auction.rs", "crates/roko-compose/src/cost_attribution.rs", "crates/roko-compose/src/lib.rs", "crates/roko-compose/src/prompt.rs", "crates/roko-compose/src/strategy.rs", "crates/roko-core/src/config/schema.rs", "roko.toml"]
lane = "rust-cold"
parent = "spec-446a41"
links = { depends_on = ["gap-aea13a", "gap-943046", "gap-894977"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f crates/roko-cli/tests/prompt_relevance_canary.rs && grep -qw 'fn unrelated_tasks_get_no_learned_sections' crates/roko-cli/tests/prompt_relevance_canary.rs && cargo test -p roko-cli --test prompt_relevance_canary"

[[verify]]
command = "! grep -rq 'fn load_attention_bidders' crates/roko-cli/src/ && ! grep -rq 'fn set_learning_bidders' crates/roko-cli/src/ && ! grep -rq 'attention-bidders.json' crates/roko-cli/src/ && cargo test -p roko-cli --lib dispatch::prompt_builder::"

[[verify]]
command = "! grep -q 'pub fn vcg_allocate' crates/roko-compose/src/auction.rs && ! grep -q 'pub struct LearningBidder' crates/roko-compose/src/auction.rs && grep -rqw 'fn composer_never_selects_vcg' crates/roko-compose/src/ && cargo test -p roko-compose composer_never_selects_vcg"

[[verify]]
command = "! grep -q 'vcg_warmup_observations' roko.toml && grep -rqw 'fn legacy_vcg_prompt_keys_load_as_density_greedy' crates/roko-core/src/ && cargo test -p roko-core legacy_vcg_prompt_keys_load_as_density_greedy"
+++

## Problem

This package delivers 4 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK39, slice 42xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 4220 | M | p2 | Prompt-relevance canary: over two scripted runs, each prompt holds only its own task's learned context | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4220-prompt-relevance-canary-over-two-scripted-runs.md` |
| 2 | 4217 | S | p3 | Delete the attention-bidder learner that plan runs never load, save or update | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4217-delete-the-cli-attention-bidder-learner.md` |
| 3 | 4218 | M | p3 | Retire VCG section allocation and learning bidders from roko-compose's composer | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4218-retire-vcg-allocation-from-roko-compose.md` |
| 4 | 4219 | S | p3 | Deprecate the VCG prompt config: composition_strategy auto and vcg, and vcg_warmup_observations | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4219-deprecate-vcg-prompt-config-keys.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4200-prompt-composition-without-noise.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/dispatch/factory.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/tests/prompt_relevance_canary.rs`, `crates/roko-compose/src/auction.rs`, `crates/roko-compose/src/cost_attribution.rs`, `crates/roko-compose/src/lib.rs`, `crates/roko-compose/src/prompt.rs`, `crates/roko-compose/src/strategy.rs`, `crates/roko-core/src/config/schema.rs`, `roko.toml`.

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

- Waits on: PK33 (gap-aea13a), PK35 (gap-943046), PK38 (gap-894977).
- Suggested model: opus.

## Progress

- 4220: implemented at dc3249b88 (cargo verification deferred to the batch gate). New tests/prompt_relevance_canary.rs: two scripted runs over R2's calc fixture, in maximize mode, with helper calls on a provider of their own; `unrelated_tasks_get_no_learned_sections` and `a_later_task_sees_its_topics_lessons` (pattern with `Fix:`, lesson, calc playbook).
- 4217: implemented at da3219956 (cargo verification deferred to the batch gate). The attention-bidder learner, its persistence and the factory hook are deleted; the loop registry retires L-bid (by 4217), since its findings pointed at the deleted symbols, and the registry and census tests list it as retired.
- 4218: implemented at 5ff47ffde (cargo verification deferred to the batch gate). VCG allocation, the learning bidders and their diagnostics are gone from roko-compose; every strategy resolves to density-greedy; `composer_never_selects_vcg`.
- 4219: implemented at 185e0763f (cargo verification deferred to the batch gate). `composition_strategy` defaults to density_greedy; `auto` and `vcg` load as density_greedy with a one-time warning; `vcg_warmup_observations` loads and is ignored; roko.toml drops its [prompt] table; `legacy_vcg_prompt_keys_load_as_density_greedy`.
- gap-29fe0a: implemented at 427f81e4b (cargo verification deferred to the batch gate). `roko run --no-holdout` reaches every run path (one task, --plan, plan directory, --serve/--share); `cli_parses_run_no_holdout`, `no_holdout_reaches_the_run`.
