+++
id = "gap-93b605"
kind = "gap"
title = "TaskSpeedPriority's own doc comments still describe the routing bias 3109 removed"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-core/task"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "7789cfee6"
source = "wave-6 follow-up reports 2026-10-03 (PK14 gap-997366)"
discovered_from = "gap-997366"
anchors = ["crates/roko-core/src/task.rs::TaskSpeedPriority", "crates/roko-cli/src/plan_validate.rs::speed_priority_diagnostics"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Optimize for turnaround (pick faster models)' crates/roko-core/src/task.rs"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T11:30:38Z"
commit = "7789cfee6"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T09:32:41Z"
forced = false
evidence = "Gate 17a (merged 7789cfee6): its verify passes, with workspace check, clippy, lib tests and the learning wiring census green. Part of the p3 cleanup branch (work/gap-9d32d0): L-dream-bias retired from loops.toml and the census, TaskSpeedPriority docs, the dead vcg_warmup_observations field, error_pattern_store's module doc, and demo_seed feeding the attempt ledger instead of task-metrics.jsonl."
+++

## Problem

`roko_core::task::TaskSpeedPriority`'s own doc comments (`crates/roko-core/src/task.rs:336-342`) still
describe a routing effect that backlog task 3109 (decision 3108) removed:

```rust
pub enum TaskSpeedPriority {
    /// Optimize for turnaround (pick faster models).
    Latency,
    /// Default blend.
    Balanced,
    /// Optimize for correctness (pick deeper models).
    Accuracy,
}
```

`crates/roko-cli/src/plan_validate.rs:307-309`'s own comment says the opposite is now true: "backlog 3109
(decision 3108): `speed_priority = \"latency\"` asked the router for cheaper models through a routing bias
that no longer exists, so a task that sets it gets a PLAN_047 warning that it routes nothing." `plan_validate`
actively warns authors away from relying on the very behavior the enum's doc comment still promises.

The field's only remaining live consumer is `roko-daimon`'s affect engine (`crates/roko-daimon/src/lib.rs:815-818`),
which reads `speed_priority` to bias `deadline_proximity` (`Latency` → 0.85, `Accuracy` → 0.35, `Balanced` →
a complexity-derived value) — an affect/pacing signal, not a model-selection one.

## Why it matters

A plan author reading `TaskSpeedPriority`'s own doc comments (the first thing they'd see from an IDE tooltip
or generated docs) would reasonably set `speed_priority = "latency"` expecting faster/cheaper model routing —
exactly what `PLAN_047` now exists to warn them away from. The type's documentation and the linter built to
correct a misunderstanding of it disagree with each other.

## Where

- `crates/roko-core/src/task.rs:332-343` (`TaskSpeedPriority`'s doc comments).
- `crates/roko-cli/src/plan_validate.rs:307-324` (`speed_priority_diagnostics`, `PLAN_047`) — the correct,
  current description of the field's effect.
- `crates/roko-daimon/src/lib.rs:815-818` — the field's one real remaining consumer.

## Current state

Unfixed. The doc comments were not updated when 3109 removed the routing bias.

## Plan

Rewrite the three variant doc comments (and the enum's own "Latency vs correctness tradeoff dial" summary
line, `task.rs:332`) to describe the current effect: a pacing/affect signal for `roko-daimon`'s
`deadline_proximity`, not a model-routing hint. Point to `PLAN_047` for the historical note, so a reader who
remembers the old behavior understands why it changed.

## Done when

- `TaskSpeedPriority`'s doc comments describe its current (affect-only) effect, not the removed routing bias.
- The `[[verify]]` command passes.

## Notes

- Docs-only (Rust doc comments) — no behavior change.
- Same source finding (PK14/gap-997366) as `gap-9d32d0` (L-dream-bias retirement) and `bug-19ae56`
  (plan_brief's skip_enrichment) from this same wave-6 batch; filed separately since each touches different
  code.
