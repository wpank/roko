+++
id = "gap-aea13a"
kind = "gap"
title = "PK33 Prompt composition: Delete roko-execution's duplicate prompt cache, which nothing reads (+13 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
rank = 33
size = "L"
subsystem = ["roko-cli/prompt"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "5bb643122"
source = "tmp/backlog/2026-10-02-complete-and-wire PK33"
anchors = ["crates/roko-cli/src/dispatch/factory.rs", "crates/roko-cli/src/dispatch/mod.rs", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/dispatch/prompt_cache.rs", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/runtime_feedback/error_patterns.rs", "crates/roko-cli/src/runtime_feedback/verified_knowledge.rs", "crates/roko-cli/tests/dispatch_feedback_projection_e2e.rs", "crates/roko-compose/src/context_provider.rs", "crates/roko-execution/src/builder.rs", "crates/roko-execution/src/lib.rs", "crates/roko-execution/src/prompt/cache.rs", "crates/roko-execution/src/prompt/mod.rs", "crates/roko-learn/src/error_pattern_store.rs", "crates/roko-neuro/src/lifecycle.rs"]
lane = "rust-hot"
parent = "spec-446a41"
links = { depends_on = ["gap-f61823"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test ! -e crates/roko-execution/src/prompt/cache.rs && ! grep -rqw 'PromptCacheHandle' crates/ && grep -rqw 'fn builder_for_test_constructs_all_profiles' crates/roko-execution/src/ && cargo test -p roko-execution --lib builder_for_test_constructs_all_profiles"

[[verify]]
command = "! grep -q 'discovered-patterns' crates/roko-compose/src/context_provider.rs && grep -rqw 'fn failure_patterns_bidder_reads_error_patterns_json' crates/roko-compose/src/ && grep -rqw 'fn legacy_discovered_patterns_file_is_set_aside' crates/roko-learn/src/ && cargo test -p roko-compose failure_patterns_bidder_reads_error_patterns_json && cargo test -p roko-learn legacy_discovered_patterns_file_is_set_aside"

[[verify]]
command = "! grep -q 'fn load_pheromone_jsonl_context' crates/roko-cli/src/dispatch/prompt_builder.rs && ! grep -rq 'pheromones.jsonl' crates/roko-cli/src/ && cargo test -p roko-cli --lib dispatch::prompt_builder::"

[[verify]]
command = "! grep -rq 'fn generate_cfactor_context' crates/roko-cli/src/ && ! grep -rq 'cached_cfactor_context' crates/roko-cli/ && grep -rqw 'fn plan_prompt_has_no_collective_calibration_block' crates/roko-cli/src/ && cargo test -p roko-cli --lib plan_prompt_has_no_collective_calibration_block"

[[verify]]
command = "! grep -rqE 'fn dream_routing_bias|routing_bias:' crates/roko-cli/src/dispatch crates/roko-cli/src/graph_task_dispatch crates/roko-cli/src/graph_task_dispatch.rs"

[[verify]]
command = "grep -rqw 'fn turn_cap_failure_records_no_error_pattern' crates/roko-cli/src/ && grep -rqw 'fn load_drops_turn_cap_and_timeout_patterns' crates/roko-learn/src/ && cargo test -p roko-cli --lib turn_cap_failure_records_no_error_pattern && cargo test -p roko-learn load_drops_turn_cap_and_timeout_patterns"

[[verify]]
command = "grep -rqw 'fn keyed_summary_skips_patterns_of_other_tasks_and_commands' crates/roko-learn/src/ && cargo test -p roko-learn keyed_summary_skips_patterns_of_other_tasks_and_commands"

[[verify]]
command = "grep -rqw 'fn error_pattern_from_other_crate_not_in_prompt' crates/roko-cli/src/ && cargo test -p roko-cli --lib error_pattern_from_other_crate_not_in_prompt && ! grep -q 'format_error_patterns_for_prompt(5)' crates/roko-cli/src/graph_task_dispatch.rs crates/roko-cli/src/graph_task_dispatch/streaming.rs"

[[verify]]
command = "grep -rqw 'fn knowledge_section_ignores_id_and_path_word_matches' crates/roko-cli/src/ && grep -rqw 'fn knowledge_section_skips_success_notes' crates/roko-cli/src/ && cargo test -p roko-cli --lib knowledge_section_ignores_id_and_path_word_matches && cargo test -p roko-cli --lib knowledge_section_skips_success_notes"

[[verify]]
command = "grep -rqw 'fn playbooks_without_overlap_are_not_injected' crates/roko-cli/src/ && cargo test -p roko-cli --lib playbooks_without_overlap_are_not_injected"

[[verify]]
command = "grep -rqw 'fn episode_section_skips_episodes_with_nothing_to_say' crates/roko-cli/src/ && grep -rqw 'fn episode_section_ignores_role_and_model_matches' crates/roko-cli/src/ && cargo test -p roko-cli --lib episode_section_skips_episodes_with_nothing_to_say && cargo test -p roko-cli --lib episode_section_ignores_role_and_model_matches"

[[verify]]
command = "! grep -q 'fn update_prompt_cache' crates/roko-cli/src/dispatch/factory.rs && grep -rqw 'fn prompt_cache_is_one_snapshot_per_run' crates/roko-cli/src/ && cargo test -p roko-cli --lib prompt_cache_is_one_snapshot_per_run"

[[verify]]
command = "grep -rqw 'fn verified_task_prompt_asks_for_a_lesson_line' crates/roko-cli/src/ && cargo test -p roko-cli --lib verified_task_prompt_asks_for_a_lesson_line"

[[verify]]
command = "grep -rqw 'fn verified_pass_stores_stated_lesson' crates/roko-cli/src/ && grep -rqw 'fn verified_pass_without_lesson_admits_no_entry' crates/roko-neuro/src/ && cargo test -p roko-cli --lib verified_pass_stores_stated_lesson && cargo test -p roko-neuro verified_pass_without_lesson_admits_no_entry"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T00:42:00Z"
commit = "5bb643122"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T22:19:31Z"
forced = false
evidence = "Gate 5a (merged into main as 5bb643122, tree identical to work/backlog-batch-5a apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 12,477 passed over 12 crates, roko-cli bin + golden-path canaries + plan_revise/plan_validate/plan_spec_gate integration tests (only bug-2a31bc's two known alias tests fail), ViabilityBench suite 548 passed, portal vitest + tsc; every [[verify]] passes."
+++

## Problem

This package delivers 14 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK33, slice 42xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 4203 | S | p3 | Delete roko-execution's duplicate prompt cache, which nothing reads | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4203-delete-roko-execution-duplicate-prompt-cache.md` |
| 2 | 4204 | S | p3 | Point roko-compose at error-patterns.json and set Runner-v2's discovered-patterns.json aside | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4204-roko-compose-reads-error-patterns-json.md` |
| 3 | 4205 | S | p3 | Delete the pheromones.jsonl prompt reader, which no code writes | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4205-delete-pheromones-jsonl-prompt-reader.md` |
| 4 | 4206 | S | p3 | Stop injecting the c-factor block into plan prompts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4206-stop-injecting-c-factor-block-into-plan-prompts.md` |
| 5 | 4207 | S | p3 | Read dream routing advice only while plan-run dreams are on, and pin its staleness rule | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4207-no-dream-routing-advice-while-dreams-are-held.md` |
| 6 | 4208 | S | p2 | Learn error patterns only from verify failures, not turn caps or timeouts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4208-learn-error-patterns-only-from-verify-failures.md` |
| 7 | 4209 | S | p2 | Error-pattern store: select a pattern only for the same task or the same failing verify command | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4209-error-pattern-store-selects-by-task-and-command.md` |
| 8 | 4210 | M | p2 | Inject only the error patterns keyed to a task, and honour knowledge_error_patterns | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4210-inject-only-keyed-error-patterns.md` |
| 9 | 4211 | S | p2 | Rank knowledge by topic words, not task ids, roles and path words, and keep success notes out | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4211-rank-knowledge-by-topic-words-not-ids-and-paths.md` |
| 10 | 4212 | S | p2 | Inject a playbook only when it shares topic words with the task | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4212-inject-playbooks-only-when-relevant.md` |
| 11 | 4213 | S | p3 | Episode section: match on topic words and skip episodes with nothing to say | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4213-episode-section-matches-topic-and-skips-empty.md` |
| 12 | 4214 | S | p3 | Make the plan run's prompt cache one explicit snapshot and delete the refresh path nothing calls | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4214-one-prompt-cache-snapshot-per-run.md` |
| 13 | 4215 | S | p2 | Ask the agent on a verified task for a one-line lesson | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4215-ask-verified-tasks-for-a-one-line-lesson.md` |
| 14 | 4216 | S | p2 | Store the agent's lesson, not a success note, when a verified pass writes knowledge | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4216-store-the-lesson-not-a-success-note.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4200-prompt-composition-without-noise.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/dispatch/factory.rs`, `crates/roko-cli/src/dispatch/mod.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/dispatch/prompt_cache.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/routing_context.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/runtime_feedback/error_patterns.rs`, `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs`, `crates/roko-cli/tests/dispatch_feedback_projection_e2e.rs`, `crates/roko-compose/src/context_provider.rs`, `crates/roko-execution/src/builder.rs`, `crates/roko-execution/src/lib.rs`, `crates/roko-execution/src/prompt/cache.rs`, `crates/roko-execution/src/prompt/mod.rs`, `crates/roko-learn/src/error_pattern_store.rs`, `crates/roko-neuro/src/lifecycle.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK10 (gap-f61823).
- Suggested model: opus.

## Progress

Implemented on `work/gap-aea13a`; cargo verification deferred to the batch gate. The static part of every verify
passes. `eaa19b074` hand-formats lines from several tasks to rustfmt defaults.

- 4203: implemented at 33f3dc263
- 4204: implemented at 9d2eb8e64
- 4205: implemented at 294ccbc48
- 4206: implemented at 4833134ee
- 4207: implemented at 1c08cd953
- 4208: implemented at fd79f2c10
- 4209: implemented at dc11ae0da
- 4210: implemented at 6ed6e5f01
- 4211: implemented at e00a5b582
- 4212: implemented at 499334f05
- 4213: implemented at 8c631e328
- 4214: implemented at badb7911a
- 4215: implemented at f864016d7
- 4216: implemented at d79ee8a1a (test follow-up fe88daaf5)
- 2026-10-03 (coordinator, after gate 6b): task 4207's verify named two tests of the dream routing bias that PK14's 3109 deleted along with RoutingBias itself (decision 3108, gap-997366, merged in 059450273). The verify now checks that the bias is gone from the plan path.
