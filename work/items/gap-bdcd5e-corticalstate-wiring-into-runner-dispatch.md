+++
id = "gap-bdcd5e"
kind = "gap"
title = "CorticalState Wiring into Runner Dispatch"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-primitives"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/190-corticalstate-wiring.md#190 — CorticalState Wiring into Runner Dispatch"
discovered_from = "audit:tmp/backlog/archive/190-corticalstate-wiring.md#190 — CorticalState Wiring into Runner Dispatch"
anchors = ["crates/roko-primitives/src/pad.rs", "crates/roko-runtime/src/heartbeat.rs", "crates/roko-cli/src/runner/event_loop.rs", "crates/roko-learn/src/active_inference.rs", "crates/roko-runtime/src/theta_consumer.rs", "delta_consumer.rs", "event_loop.rs", ".roko/state/cortical.json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
CorticalState is fully built (2,717 LOC, E23 10/10 accepted) but not instantiated in the runner. CorticalState implements agent cognitive autonomy: lifecycle type-state, behavioral vitality, energy fields, energy accounting, adaptive timescales, energy/affect coupling, EFE routing, GoalTree, and…

Imported without verification from:
- `tmp/backlog/archive/190-corticalstate-wiring.md#190 — CorticalState Wiring into Runner Dispatch`

Some cited files are gone: `.roko/state/cortical.json`, `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: `CorticalState` is instantiated once per plan run in `event_loop.rs`; Heartbeat tick fires on a periodic timer and updates energy fields; `AgentEfficiencyEvent` feeds into CorticalState energy accounting after each turn [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): M | 6 |]
