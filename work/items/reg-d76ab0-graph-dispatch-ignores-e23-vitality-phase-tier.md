+++
id = "reg-d76ab0"
kind = "regression"
title = "Graph dispatch ignores E23 vitality, phase tier caps and energy charges"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-cli/graph-dispatch", "roko-daimon"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "gaps-md#recently-closed-epic-status/e23"
discovered_from = "review:gaps-md-migration-2026-09-28"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-daimon/src/goals.rs", "crates/roko-learn/src/active_inference.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE 'vitality|LifecyclePhase' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib graph_dispatch_applies_low_vitality_cost_pressure"
+++

E23 was accepted on the claim that "Runner-v2 applies numeric low-vitality cost pressure, hard phase tier caps, restart-durable Terminal skips, and post-spawn energy charges". Runner-v2 has been deleted. In non-test code, no file in `roko-cli`, `roko-graph`, `roko-agent` or `roko-execution` now refers to vitality, lifecycle phases or energy charging. Vitality appears only in telemetry lenses, StateHub projections and serve routes. GoalTree and EFE routing exist only in `crates/roko-daimon/src/goals.rs` and `crates/roko-learn/src/active_inference.rs`, and nothing in dispatch uses them.

Fix: decide which E23 constraints apply to Graph plan dispatch and wire them into routing and admission (for example in `graph_task_dispatch.rs`), with tests for cost pressure and phase caps. Alternatively, record their removal as a decision.
