+++
id = "bug-4aa696"
kind = "bug"
title = "The composer's foraging pre-pass keeps at most three optional sections, so domain_context (knowledge, episodes, playbooks) is silently dropped from real prompts"
status = "open"
triage = "unverified"
severity = "p1"
goal = "cybernetic"
size = "S"
subsystem = ["roko-compose"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-childenv's report on bug-86117a, branch work/bug-86117a at 2082c0f3a)"
anchors = ["crates/roko-compose/src/prompt.rs::foraging_prepass"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-86117a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn foraging_keeps_every_section_that_fits' crates/roko-compose/src/ && cargo test -p roko-compose --lib foraging_keeps_every_section_that_fits"
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
