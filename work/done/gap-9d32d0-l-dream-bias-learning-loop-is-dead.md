+++
id = "gap-9d32d0"
kind = "gap"
title = "L-dream-bias learning loop is dead (3109 removed its routing-bias path) but still registered as live"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "7789cfee6"
source = "wave-6 follow-up reports 2026-10-03 (PK14 gap-997366)"
discovered_from = "gap-997366 (backlog task 3109)"
anchors = ["crates/roko-learn/src/loop_audit/loops.toml", "crates/roko-learn/src/loop_audit/spec.rs", "crates/roko-learn/src/loop_audit/assign.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'L-dream-bias' crates/roko-learn/src/loop_audit/loops.toml"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T11:30:38Z"
commit = "7789cfee6"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T09:32:40Z"
forced = false
evidence = "Gate 17a (merged 7789cfee6): its verify passes, with workspace check, clippy, lib tests and the learning wiring census green. Part of the p3 cleanup branch (work/gap-9d32d0): L-dream-bias retired from loops.toml and the census, TaskSpeedPriority docs, the dead vcg_warmup_observations field, error_pattern_store's module doc, and demo_seed feeding the attempt ledger instead of task-metrics.jsonl."
+++

## Problem

The L-dream-bias learning loop is dead but still registered as live in the loop census. Its own registry entry
already documents why: `crates/roko-learn/src/loop_audit/loops.toml:44-57` (`id = "L-dream-bias"`):

```
claim = "With provider health attached, cascade_pick calls route_with_health_scored, which takes no bias, so the
dream bias never reaches the pick."
```

Backlog task 3109 ("routing bias applies with provider health", implemented at `e9d37c7c4`, part of gap-997366)
removed the routing-bias path this loop depended on. But `loops.toml` still lists `L-dream-bias` as a loop to
audit, and `crates/roko-learn/src/loop_audit/spec.rs` and `crates/roko-learn/src/loop_audit/assign.rs` still carry
tests that treat its assignment layer (`route.dream_bias`) as meaningful (`spec.rs:1021,1030,1083,1090-1091,1136`;
`assign.rs:564-571,699-700`).

## Why it matters

A retired mechanism that still appears "live" in the loop census misrepresents what the system's learning loops
actually do, and the registry's own `claim` field says as much — it's a documented dead loop that nobody finished
retiring.

## Where

- `crates/roko-learn/src/loop_audit/loops.toml:44-57` — the `L-dream-bias` entry.
- `crates/roko-learn/src/loop_audit/spec.rs` — the loop's spec/registry metadata and its tests.
- `crates/roko-learn/src/loop_audit/assign.rs` — layer-assignment tests exercising `route.dream_bias`.
- `crates/roko-learn/src/loop_audit/census.rs:459,563` — the census case for `L-dream-bias`.

## Current state

Dead code path (per the loop's own `claim`), still fully wired into the registry, spec and tests as if live.

## Plan

1. Decide the retirement shape: remove the `L-dream-bias` entry from `loops.toml` entirely (matching how other
   retired loops in this same wave were removed, e.g. L-holdout per earlier batches' gap-b5caf3 work), or mark it
   `retired` if the registry has that status (check `loops.toml`'s other entries/schema for a status field).
2. Remove or update the `spec.rs`/`assign.rs` tests that assert on `route.dream_bias`'s assignment layer, since
   the loop it was for no longer draws anything through that layer.
3. Update `census.rs`'s `L-dream-bias` case accordingly.

## Done when

- `L-dream-bias` is retired (or removed) from the loop registry, consistent with its own `claim` field.
- No test asserts on `route.dream_bias` as if the loop were live.
- The `[[verify]]` command passes.

## Notes

- Discovered while reviewing gap-997366 (PK14), which implemented task 3109 (the routing-bias removal this loop's
  own `claim` describes) but did not retire the loop itself.
