+++
id = "bug-a66fef"
kind = "bug"
title = "roko-compose's system prompt snapshot test fails: prompts now include a Relevant Context section the snapshot lacks"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-compose/tests"]
created = 2026-10-11
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "6aebfcd75"
source = "CI-like local sweep 2026-10-11 (roko-90)"
anchors = ["crates/roko-compose/tests/system_prompt_snapshot.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-compose --test system_prompt_snapshot canonical_system_prompts_match_snapshots"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T11:48:58Z"
commit = "6aebfcd75"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T10:26:21Z"
forced = false
evidence = "Snapshot updated; the section is intended, so no code changed. 6aebfcd75 re-records system_prompt_snapshot__role__quick-reviewer.snap and system_prompt_snapshot__role__auto-fixer.snap: each gains the fixture's context_layer (\"## Relevant Context / ### Workspace Notes\", 5 lines) in the live prompt, and the same block under <!-- CONTEXT --> in the layered view; nothing else changes. Why it appears: 0c9a3c96b (gap-c8bfc8, merged in e37dd2c5e on 2026-10-04) split PromptBudget's runner_context from context. SystemPromptBuilder::section_budget_cap now caps context_layer and gate_feedback by runner_context. QuickReviewer and AutoFixer keep context = 0 (no cross-plan context: domain_context dropped, and skills = 0 drops the techniques) but got runner_context = 2,000, so that review and fix roles keep their task's verify commands and gate feedback. The other 27 canonical roles always had the section, and their snapshots still match. These were the only two never re-recorded. The section does not depend on features: before the update, the per-crate build failed on quick-reviewer with exactly the item's diff (new lines 46-50 and 123-128). After it, cargo test -p roko-compose --test system_prompt_snapshot canonical_system_prompts_match_snapshots passes in the session environment and in env -i with a scratch HOME, CI=true and no keys. The whole file passes 3/3, and so does cargo test --workspace --test system_prompt_snapshot (3/3); its test binary (system_prompt_snapshot-f8a719f705e58e6b, unlike the per-crate build's ed547c98cce411f7) is built with workspace feature unification, as cargo test --workspace builds it. cargo +nightly fmt --all --check is clean, and so is cargo clippy -p roko-compose --no-deps -- -D warnings."
+++

## Problem

`roko-compose`'s `system_prompt_snapshot::canonical_system_prompts_match_snapshots` fails in a workspace build,
locally as well as in a CI-like environment. The assembled `quick-reviewer` prompt (and others) now contains a
"## Relevant Context / ### Workspace Notes" section (lines 46-50 and 123-128 of the new output) that the stored insta
snapshot lacks.

## Why it matters

CI's `cargo test --workspace` runs this test. The batch gates ran roko-compose's lib tests only, so nothing flagged it.

## Where

`crates/roko-compose/tests/system_prompt_snapshot.rs` and its `snapshots/`, and the prompt builder's context section
(`crates/roko-compose/src/system_prompt_builder.rs`).

## Plan

1. Check whether the section is new, intended behaviour (then review and accept the snapshots with
   `cargo insta review`) or appears only when workspace feature unification turns a feature on (then pin the
   features the test builds with, or normalise the section).
2. Run the test both ways: `cargo test -p roko-compose --test system_prompt_snapshot`, and as part of
   `cargo test --workspace`.

## Done when

- [ ] The test passes both per-crate and in a workspace run.
