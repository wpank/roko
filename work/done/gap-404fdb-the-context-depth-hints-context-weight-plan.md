+++
id = "gap-404fdb"
kind = "gap"
title = "The context-depth hints context_weight, plan_section, skills and research_before_edit are parsed but unused"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/task_parser", "roko-cli/dispatch/prompt_builder"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-taskdef's report, checked on work/gap-0f3980 at b27c02717)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskDef", "crates/roko-cli/src/dispatch/prompt_builder.rs"]
lane = "rust-cold"
links = { depends_on = ["gap-0f3980"], blocks = [], related = ["gap-0f3980"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn context_depth_hints_shape_the_prompt' crates/roko-cli/src/ && cargo test -p roko-cli --lib context_depth_hints_shape_the_prompt"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:36Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:31Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

gap-0f3980's branch parses the TaskDef context-depth hints `context_weight`, `plan_section`, `skills` and `research_before_edit`, but prompt assembly doesn't use them. At b27c02717, `plan_section` and `research_before_edit` have no reader outside the parser at all.

## Why it matters

The planner's advice on how much context a task needs, and which section of the plan and which skills it concerns, is dropped. p3.

## Where

The fields on `TaskDef`, and the prompt builder.

## Plan

1. Wire each hint where it belongs: `plan_section` selects the plan excerpt, `skills` selects skill cards, `context_weight` scales the context budget, and `research_before_edit` adds a research step. Or remove the ones that won't be wired.
2. Add `context_depth_hints_shape_the_prompt`.

## Done when

- [ ] Each hint changes the prompt, or it's gone.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-0f3980's branch.
- 2026-10-01 (wk-specq): implemented on work/gap-404fdb; cargo verification deferred to the batch check. All four
  hints now shape the prompt (`dispatch/prompt_builder.rs`):
  - `plan_section` narrows the PRD excerpt to the section it names;
  - `skills` adds a `## Skills` section from `.roko/learn/skills.json`;
  - `research_before_edit` adds a `## Before You Edit` note;
  - `context_weight` scales the plan context: `slim` loads none, `deep` twice as much.
  `unused_hints` (PLAN_039) no longer lists them. The test is `context_depth_hints_shape_the_prompt`.
