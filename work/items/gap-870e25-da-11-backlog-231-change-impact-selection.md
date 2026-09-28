+++
id = "gap-870e25"
kind = "gap"
title = "DA-11 / backlog #231: Change-impact selection lacks symbol-level, macro-aware and non-Rust consumer analysis"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/gate-dispatch"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/dev-audit/11-implementation-status.md#Explicit residuals"
discovered_from = "audit:tmp/dev-audit/11-implementation-status.md#Explicit residuals"
anchors = ["runner/gate_dispatch.rs", "crates/roko-cli/src/graph_execution/", "backlog #231"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Impact-selected gates use syntax/Cargo-graph analysis only; dogfood recorded a bool->Option<bool> public type change that missed consumers in three crates. The 2026-09-14 sweep found no ImpactAnalysis type; residual explicitly open. Dogfood audit 09-03 claims a roko-index symbol oracle landed; co...

Imported without verification from:
- `tmp/dev-audit/11-implementation-status.md#Explicit residuals`
- `tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)`
- `tmp/dev-audit/README.md#Verification note (2026-09-14)`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Summary`
- `tmp/archive/dogfood-2026-08-25/DOGFOOD-DEBRIEF.md#Issue 5: Cross-crate type changes exceed agent scope (HIGH)`

How to verify: Run a public-field-type-change fixture through the Graph gate path; check reverse dependents are compiled.
