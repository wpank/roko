+++
id = "gap-c42c8e"
kind = "gap"
title = "run_gate_once can only be deleted once ProductionGateService ports its seven pinned capabilities"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/runner", "roko-gate/production-service"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK59 gap-147c4d)"
discovered_from = "gap-147c4d; run_gate_once's own doc comment at gate_dispatch.rs:911-920"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::run_gate_once", "crates/roko-gate/src/production_service.rs::ProductionGateService"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn production_gate_service_publishes_verdicts' crates/roko-gate/ && cargo test -p roko-gate production_gate_service_publishes_verdicts"
+++

## Problem

`run_gate_once` (`crates/roko-cli/src/runner/gate_dispatch.rs:922`) carries its own doc comment admitting it is
dead in production and naming exactly what blocks its removal:

```
/// Run a gate rung to completion and return its summary.
///
/// No production path calls this since 7129 deleted `spawn_gate`: tasks gate
/// through the Graph dispatcher, and the rich-topology path through
/// [`RunnerProductionGateAdapter`] over `ProductionGateService`. It stays
/// because its tests pin what that service does not do yet: publishing
/// verdicts, gate telemetry, streamed output lines, impact analysis,
/// filtering pre-existing failures, the focused baseline verify and the
/// targeted compile rung. Port those to the service, with their tests, and
/// delete it.
```

`crates/roko-gate/src/production_service.rs` (934 lines) has none of the seven: grepping it for
`verdict_publisher`, `impact_analysis`, `pre.existing`, `focused_baseline`, `targeted_compile`, `gate_telemetry`
and `streamed` returns zero matches. `run_gate_once` itself does use all seven (`VerdictPublisher` at line 30,
`impact_analysis::analyze` at line 969, `with_targeted_compile_rung` at line 1157, `run_focused_baseline_verify`
at line 1998, a `pre-existing-filtered:` gate-name convention at line 2802, and the test
`live_gate_verdicts_publish_signal` at line 3025 pinning the verdict-publishing behavior), so this is a real,
specific parity gap, not a vague TODO.

## Why it matters

This is the only thing keeping a known-dead function (per its own comment) in the codebase, and it's the sort of
gap the work graph exists to stop going stale (CLAUDE.md rule: "if nothing closes it, it goes stale" — exactly
what happened here: the comment has presumably sat unactioned since 7129 deleted `spawn_gate`, with no tracked
item pointing at it). Each of the seven capabilities also represents a real behavior gap in `ProductionGateService`
today for anyone relying on it directly (e.g. verdicts aren't published, pre-existing failures aren't filtered).

## Where

- `crates/roko-cli/src/runner/gate_dispatch.rs::run_gate_once` (the function to delete once parity is reached)
  and its seven capabilities: `VerdictPublisher` (~line 30, 1250, 1394, 1560), `impact_analysis::analyze` (~969),
  `with_targeted_compile_rung` (~1157), `run_focused_baseline_verify` (~1998), the `pre-existing-filtered:` gate
  convention (~2802), gate telemetry and streamed output lines (search the function body for its telemetry/stdout
  plumbing).
- `crates/roko-gate/src/production_service.rs::ProductionGateService` (the port target — `run_canonical_pipeline`,
  `run_verify_steps`, `ProductionGateRunner::run`).
- `RunnerProductionGateAdapter` (the live caller of `ProductionGateService` — find it to see how/where a ported
  capability would actually get exercised in production).

## Current state

`run_gate_once` is unreachable from production (confirmed by its own comment and by `run_gate_once(` only
appearing inside `gate_dispatch.rs` itself — no external caller found). Its test suite is the only thing still
exercising the seven capabilities.

## Plan

1. Port each of the seven capabilities into `ProductionGateService`, one at a time, each with its own test ported
   from `run_gate_once`'s existing suite (don't just delete the old tests — move their assertions).
2. Once all seven are ported and `RunnerProductionGateAdapter`'s callers can reach every one of them, delete
   `run_gate_once` and its now-redundant tests.
3. Do this incrementally — seven independent ports, not one large change — so each can be verified separately.

## Done when

- `ProductionGateService` publishes verdicts, emits gate telemetry and streamed output lines, runs impact
  analysis, filters pre-existing failures, runs the focused baseline verify, and applies the targeted compile
  rung — each covered by a test that used to live on `run_gate_once`.
- `run_gate_once` and `crates/roko-cli/src/runner/gate_dispatch.rs`'s now-dead code around it are deleted.
- The `[[verify]]` command passes.

## Notes

- This is explicitly a migration/cleanup item, not a functional defect in anything currently reachable in
  production — `run_gate_once` being dead code is itself harmless; the risk is only that it silently diverges
  further from `ProductionGateService` the longer both exist.
- Do this in small, separately-verifiable steps (see Plan) — the `[[verify]]` command below checks only the first
  capability (verdict publishing) as a representative start; add one per step as this item progresses, or split
  into follow-up items per capability if a worker picks this up and finds seven-in-one too large for one pass.
