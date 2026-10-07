+++
id = "gap-9fd9ff"
kind = "gap"
title = "PromptBuildHandle.vcg_warmup_observations survives the VCG auction's retirement with no consumer"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-execution/prompt"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "7789cfee6"
source = "wave-8 follow-up reports 2026-10-03 (PK39 gap-222d47)"
discovered_from = "gap-222d47"
anchors = ["crates/roko-execution/src/prompt/builder.rs::PromptBuildHandle", "crates/roko-core/src/config/schema.rs::PromptConfig"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'vcg_warmup_observations' crates/roko-execution/src/prompt/builder.rs"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T11:30:39Z"
commit = "7789cfee6"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T09:32:42Z"
forced = false
evidence = "Gate 17a (merged 7789cfee6): its verify passes, with workspace check, clippy, lib tests and the learning wiring census green. Part of the p3 cleanup branch (work/gap-9d32d0): L-dream-bias retired from loops.toml and the census, TaskSpeedPriority docs, the dead vcg_warmup_observations field, error_pattern_store's module doc, and demo_seed feeding the attempt ledger instead of task-metrics.jsonl."
+++

## Problem

`crates/roko-execution/src/prompt/builder.rs::PromptBuildHandle.vcg_warmup_observations` (line 18, default `20`,
line 25) is a second, separate copy of the VCG-warmup concept that survives the VCG auction's retirement
(backlog 4218; `crates/roko-compose/src/strategy.rs:1945`'s comment: "the VCG auction is retired"). Unlike its
sibling on the `roko-core` config side — `crates/roko-core/src/config/schema.rs::PromptConfig.vcg_warmup_observations`
(line 286), which is correctly documented as deprecated (`schema.rs:240-241`: "Default of the deprecated
`vcg_warmup_observations`"; `schema.rs:3121`: "`vcg_warmup_observations` is ignored") — this `roko-execution`
copy carries no such acknowledgment. Grepping every non-test use confirms it has no real consumer: the only
other references are `builder.rs:688-689` and `:813-814`, both test assertions comparing the field against its
own default, never against anything read from config or passed into a real prompt build.

## Why it matters

Goal: tooling (dead-config hygiene). A field that looks live (no deprecation marker, a plausible default,
exercised by passing tests) but has no real consumer is exactly the kind of thing CLAUDE.md's "search before
writing" rule exists to catch before someone wires new code to it, assuming it does something.

## Where

- `crates/roko-execution/src/prompt/builder.rs::PromptBuildHandle` (the dead field).
- `crates/roko-core/src/config/schema.rs::PromptConfig::vcg_warmup_observations` (the already-correctly-marked
  sibling, for comparison/reference).

## Current state

Confirmed dead (no non-test, non-definition reference anywhere in `crates/`).

## Plan

1. Remove `PromptBuildHandle.vcg_warmup_observations` and its two test assertions, or mark it deprecated the same
   way `schema.rs` already does for its sibling, whichever the maintainer prefers — removal is cleaner since
   nothing reads it.

## Done when

- `PromptBuildHandle` has no dead `vcg_warmup_observations` field (removed, or explicitly marked deprecated to
  match its `roko-core` sibling).
- The `[[verify]]` command passes.

## Notes

- Discovered during PK39's work (gap-222d47, done). The related docs/v2 staleness (the removed `vcg` fields
  still documented in `docs/v2/19-CONFIG.md`) is tracked as a Notes addition on `gap-e8d97e` instead of its own
  item, per the report (docs/v2 is historical).
