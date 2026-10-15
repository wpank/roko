+++
id = "bug-dd4524"
kind = "bug"
title = "Plan prompts still carry a 'Prompt section effectiveness' block read from section-effects.json, which nothing writes since the section bandit took over"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/dispatch", "roko-compose/context_provider"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-19"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::SectionEffectivenessSource", "crates/roko-cli/src/dispatch/prompt_cache.rs::load_effectiveness", "crates/roko-compose/src/context_provider.rs::section_effects_path", "crates/roko-learn/src/section_effect.rs::DEFAULT_SECTION_EFFECTS_PATH"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn prompt_has_no_section_effectiveness_block_from_a_stale_file' crates/roko-cli/ && cargo test -p roko-cli prompt_has_no_section_effectiveness_block_from_a_stale_file"

[[verify]]
command = "! grep -rq 'DEFAULT_SECTION_EFFECTS_PATH' crates/roko-cli/src/dispatch/ crates/roko-compose/src/"
+++

## Problem

Plan prompts still get a `# Prompt section effectiveness` block read from `.roko/learn/section-effects.json`, a file
nothing at HEAD writes. `SectionEffectivenessSource` (`crates/roko-cli/src/dispatch/prompt_builder.rs:1532`, its
`PromptSectionSource` impl at :2338-2364) and the prompt cache (`dispatch/prompt_cache.rs:176`, `load_effectiveness`)
load `roko_learn::section_effect::DEFAULT_SECTION_EFFECTS_PATH`; roko-compose's context provider reads the same file
(`crates/roko-compose/src/context_provider.rs:1498`, `section_effects_path`). `SectionEffectivenessRegistry` is only
loaded or built in tests. The section bandit (4122-4124) saves its posteriors to a different file,
`section-bandit.json`.

So a workspace with an old `section-effects.json` (this repo's, from 2026-09-29) gets stale numbers in every plan
prompt, and a new workspace gets nothing (A05 §4 S4).

## Why it matters

Goal `learning` (S02 L9: the bandit was meant to take this file over). NEW-FINDINGS finding 19. Stale guidance in the
prompt; it also muddies the section bandit's own exclusion draws, since both act on the same sections. Stage 1 of the
epic. The 42xx summary left this (X14 D-L24) to slice 41/42, and it was never closed.

## Where

- `crates/roko-cli/src/dispatch/prompt_builder.rs::SectionEffectivenessSource` and the `section_effectiveness` field of
  the builder.
- `crates/roko-cli/src/dispatch/prompt_cache.rs::load_effectiveness` and the cache's `effectiveness` field.
- `crates/roko-cli/src/prompting.rs` (`section_effectiveness` option).
- `crates/roko-compose/src/context_provider.rs::section_effects_path` and its bidder.
- `crates/roko-learn/src/section_effect.rs::DEFAULT_SECTION_EFFECTS_PATH`.

## Current state

The bandit path is live (E2b LM6: posteriors saved in all 7 real workspaces, 28 exclusions). Only the old reader
remains.

## Plan

Recommended: delete the old reader. Remove `SectionEffectivenessSource`, the cache's `effectiveness`, the prompting
option and the compose bidder's read of `section-effects.json`. Keep the registry type only if a test or the bandit
needs it. The alternative, rendering the block from `section-bandit.json`, would put bandit posteriors into prompts,
which S02 L9 does not ask for.

Test `prompt_has_no_section_effectiveness_block_from_a_stale_file`: a workspace holding an old `section-effects.json`
assembles a plan prompt with no `Prompt section effectiveness` heading.

## Done when

Both `[[verify]]` commands pass.

## Notes

Leave `.roko/learn/section-effects.json` files on disk alone; nothing reads them once this lands.
