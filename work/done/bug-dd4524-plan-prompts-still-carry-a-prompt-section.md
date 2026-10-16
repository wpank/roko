+++
id = "bug-dd4524"
kind = "bug"
title = "Plan prompts still carry a 'Prompt section effectiveness' block read from section-effects.json, which nothing writes since the section bandit took over"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/dispatch", "roko-compose/context_provider"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "5cc79b9a0"
source = "status-quo-2026-10-15#finding-19"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::SectionEffectivenessSource", "crates/roko-cli/src/dispatch/prompt_cache.rs::load_effectiveness", "crates/roko-compose/src/context_provider.rs::section_effects_path", "crates/roko-learn/src/section_effect.rs::DEFAULT_SECTION_EFFECTS_PATH"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn prompt_has_no_section_effectiveness_block_from_a_stale_file' crates/roko-cli/ && cargo test -p roko-cli prompt_has_no_section_effectiveness_block_from_a_stale_file"

[[verify]]
command = "! grep -rq 'DEFAULT_SECTION_EFFECTS_PATH' crates/roko-cli/src/dispatch/ crates/roko-compose/src/"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T13:24:15Z"
commit = "5cc79b9a0"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T05:11:51Z"
forced = false
evidence = "Removed the stale section-effects.json reader: SectionEffectivenessSource + PromptAssembler.section_effectiveness (crates/roko-cli/src/dispatch/prompt_builder.rs), PromptCache.effectiveness/load_effectiveness (dispatch/prompt_cache.rs), PromptBuildOptions.section_effectiveness (prompting.rs), and context_provider.rs::section_effects_path (now cold-starts LearningContextBidder from an empty registry since that path has no production caller). Added prompt_has_no_section_effectiveness_block_from_a_stale_file, which fails before this change and passes after. At 61c0b4387 (pre-rebase, same diff): both verify commands passed in full -- 'grep ... && cargo test -p roko-cli prompt_has_no_section_effectiveness_block_from_a_stale_file' (ok, 1 passed, plus all ~80 other roko-cli test binaries and the roko bin linked and ran clean) and '! grep -rq DEFAULT_SECTION_EFFECTS_PATH crates/roko-cli/src/dispatch/ crates/roko-compose/src/' (no matches); cargo +nightly fmt --all applied; cargo clippy -p roko-cli -p roko-compose --no-deps -- -D warnings clean; roko-cli's prompt_builder (53/53), prompt_cache (1/1) and roko-compose's context_provider (29/29) lib tests passed. Rebased onto main (65a66027c -> 5cc79b9a0's parent f7dbaba72) with zero file overlap, confirmed by empty `git diff --stat` on the touched files between the two mainline tips. Re-verified on the rebased tree at 5cc79b9a0: fmt clean, clippy clean, and 'cargo test -p roko-cli --lib prompt_has_no_section_effectiveness_block_from_a_stale_file' passes; the plain grep verify command still passes. The full non-lib verify command (which links on the order of 80 separate roko-cli test binaries plus the roko bin) became flaky post-rebase under extreme host load (vm.loadavg ~150-165, other agents + the user's own concurrent work): repeated runs failed the generic 'linking with cc failed: exit status 1' on a different, unrelated target each time (plan_validate, golden_path_acceptance, connected_efficiency_render, engine_convergence_contract, the roko bin itself) with no content-specific diagnostic -- never on anything this change touches. roko-compose's canonical_system_prompts_match_snapshots still fails exactly as bug-a66fef already tracks (an unrelated Relevant Context section on quick-reviewer) -- unchanged by this work."
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
