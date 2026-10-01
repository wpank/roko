+++
id = "bug-4aa696"
kind = "bug"
title = "The composer's foraging pre-pass keeps at most three optional sections, so domain_context (knowledge, episodes, playbooks) is silently dropped from real prompts"
status = "done"
triage = "verified"
severity = "p1"
goal = "cybernetic"
size = "S"
subsystem = ["roko-compose"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "479bec688"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-childenv's report on bug-86117a, branch work/bug-86117a at 2082c0f3a)"
anchors = ["crates/roko-compose/src/prompt.rs::foraging_prepass"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-86117a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn foraging_keeps_every_section_that_fits' crates/roko-compose/src/ && cargo test -p roko-compose --lib foraging_keeps_every_section_that_fits"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T12:49:56Z"
by = "coordinator (session 7622b882)"
size = "S"
claimed_at = "2026-10-01T08:26:58Z"
forced = false
evidence = "Batch 20d gate on fae7133cd, re-checked with the clippy fix on 9f3c184c5 (MAIN 479bec688 has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/compose/core/execution/graph/neuro/runtime/serve; lib tests roko-cli 3282, roko-agent 2282, roko-core 1962, roko-serve 990, roko-compose 561, roko-graph 476, roko-runtime 288, roko-execution 245, roko-neuro 239 all pass; extras: codex/cursor/openai parity 4+4+4 (streaming tests no longer ignored), default_engine 1, C1 1, C7 2, bin 429, graph_task_dispatch loop 10/10, including foraging_keeps_every_section_that_fits (roko-compose foraging tests 10/10 on the re-check). Merged 479bec688."
+++

## Problem

Since dc418ae0a (2026-09-06), `foraging_prepass` in `crates/roko-compose/src/prompt.rs` keeps at most three optional sections. It sorts the candidates by density and compares each one to the running mean, which already includes that candidate, so the ratio is never above 1 and the stop rule fires at count 3 (wk-childenv). No test covers it.

An implementer prompt has four optional sections: conventions, tool_instructions, domain_context and anti_patterns. Durable knowledge, episodes and playbooks all live in domain_context, which by childenv's density numbers is dropped whenever it is longer than conventions (about 1,230 bytes). A long task description alone can do that. The dropped section is also missing from `dropped_sections`, because it is removed before the manifest is built.

## Why it matters

Cybernetic core (epic spec-6ac537): the learning loops feed prompts through domain_context. With this bug, bug-86117a's fix (knowledge reaching the cache) still leaves many real plan-run prompts without the knowledge, and the prompt diagnostics don't show it was dropped.

## Plan

1. Compare each candidate to the mean of the sections already kept (or use the budget alone), so every optional section that fits the budget is kept.
2. Record any section the pre-pass drops in `dropped_sections`.
3. Add `foraging_keeps_every_section_that_fits`: four optional sections that fit the budget are all kept, including a domain_context longer than conventions; one that doesn't fit is dropped and listed.

## Done when

- [ ] Every optional section that fits the budget reaches the prompt, and drops are recorded.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-childenv): Implemented on `work/bug-86117a` at `7531a3b39`, together with bug-86117a; cargo
  verification is deferred to the batch check.
  - The pre-pass now sets aside only the candidates that cannot fit the remaining budget on their own, so every
    section that fits reaches the auction. The manifest lists the set-aside ones as excluded
    (`dropped_by_foraging_budget`), so they reach `dropped_sections`, and the `dropped_section_action_ids` tag
    includes them.
  - Plan step 1's other option, comparing each candidate with the mean of the sections already kept, would not
    help: in density order a candidate is never above the mean of the denser ones either, so the rule still fires
    at the third. Any density-ratio stop drops long sections first, and `domain_context` is usually the longest.
  - The forager's gain curves stay unused (`MultiPatchForager`'s profiles and `environment_rate`). HDC dedup
    drops (`with_hdc_dedup`, which no production composer enables) are still not listed in the manifest.
  - Tests: `foraging_keeps_every_section_that_fits` (roko-compose) and, on the bug-86117a side,
    `cached_knowledge_survives_a_long_domain_context` (roko-cli), where `domain_context` is longer than
    `conventions`.
