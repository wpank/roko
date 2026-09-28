+++
id = "gap-a3c57d"
kind = "gap"
title = "ExtensionChain: formalize 8 layers and wire into execution"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-runtime"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
anchors = ["ExtensionChain"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
ExtensionChain 8-layer formalization was to be wired into orchestrate.rs (since removed). Later docs claim per-workspace chains are shared by serve and plan execution.

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring`

How to verify: grep ExtensionChain construction in plan execution (graph_execution) and roko-serve; check the 8 layers exist.
