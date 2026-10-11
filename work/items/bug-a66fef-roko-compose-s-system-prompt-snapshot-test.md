+++
id = "bug-a66fef"
kind = "bug"
title = "roko-compose's system prompt snapshot test fails: prompts now include a Relevant Context section the snapshot lacks"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-compose/tests"]
created = 2026-10-11
updated = 2026-10-11
last_verified = 2026-10-11
source = "CI-like local sweep 2026-10-11 (roko-90)"
anchors = ["crates/roko-compose/tests/system_prompt_snapshot.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-compose --test system_prompt_snapshot canonical_system_prompts_match_snapshots"
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
