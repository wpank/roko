+++
id = "gap-5b8767"
kind = "gap"
title = "4131's census fixture can't demonstrate L-know reinforcement: its workspace has no knowledge store"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "S"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-17b follow-up reports 2026-10-04 (gap-6c8965)"
discovered_from = "gap-6c8965 (open, work/backlog-batch-17b; facet 4 of 4, split out so the item closes on the other three)"
anchors = ["crates/roko-cli/tests/learning_wiring_census.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn l_know_census_credits_a_reinforced_knowledge_entry' crates/roko-cli/ && cargo test -p roko-cli --test learning_wiring_census l_know_census_credits_a_reinforced_knowledge_entry"
+++

## Problem

Task 4131's census fixture can't demonstrate a verified pass reinforcing included knowledge,
because its workspace has no knowledge store. `crates/roko-cli/tests/learning_wiring_census.rs:561`'s
own comment says so: "workspace has no knowledge store, so its rows are prompt sections and..."
— the fixture that's meant to prove every in-scope loop is live, observe-only or retired
structurally cannot exercise L-know's reinforcement path (a verified pass crediting the
knowledge entries it used), since there's no knowledge store for a pass to reinforce entries
in. This is facet 4 of `gap-6c8965`'s four related findings; facets 1-3 (the stale `loops.toml`
L-sec note, the census's L-know check now counting `exposures.jsonl` instead of the dead
`retrieval-outcomes.jsonl`, and documenting `[sections] pinned`) are done on
`work/backlog-batch-17b` (confirmed: `census.rs` and `loops.toml` and
`docs/v3/depth/21-config/01-schema-sections.md` are all in that branch's diff).
`learning_wiring_census.rs` is not, and the comment at line 561 is unchanged.

## Why it matters

Goal: learning, same goal as `gap-6c8965`. The 4131 fixture is the census's own acceptance test
("every in-scope loop is live, observe-only or retired"); without a seeded knowledge store, it
can never actually prove L-know's reinforcement half works, leaving the census's one piece of
hard evidence for this loop permanently unverifiable by this fixture, regardless of how
correct the real reinforcement code is.

## Where

- `crates/roko-cli/tests/learning_wiring_census.rs` (the 4131 fixture, around line 561).

## Current state

Unaddressed. Needs a real `roko` binary to check (the fixture likely exercises reinforcement
through an actual dispatch/verify cycle, not pure data assembly), so it couldn't be verified by
a static-only round and was split out rather than bundled with `gap-6c8965`'s other three,
code-only facets.

## Plan

1. Seed the 4131 fixture's workspace with a minimal knowledge store (a couple of entries that a
   verified pass's task could plausibly reference/reinforce).
2. Run a verified pass through the fixture and assert the census shows L-know crediting the
   reinforced entries, not `evaluated`/`live` by construction alone.
3. Confirm this requires the real binary (`target/debug/roko` or equivalent) to exercise, and
   run it that way rather than assuming a pure-data assembly suffices.

## Done when

- The 4131 fixture's workspace has a knowledge store, and its test shows a verified pass
  reinforcing an included entry, checked against the real binary.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-17b follow-up, gap-6c8965, work/backlog-batch-17b not yet merged): split out
  from `gap-6c8965`'s own facet 4, per the instruction that it needs the real binary to check
  and should let that item close on its other three facets. A matching note has been added to
  `gap-6c8965` itself.
