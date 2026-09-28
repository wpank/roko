+++
id = "gap-611631"
kind = "gap"
title = "Retire Verified Deprecated Shims and Orphaned Helpers"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/342-deprecated-shim-and-orphaned-helper-retirement.md#342 — Retire Verified Deprecated Shims and Orphaned Helpers"
discovered_from = "audit:tmp/backlog/archive/342-deprecated-shim-and-orphaned-helper-retirement.md#342 — Retire Verified Deprecated Shims and Orphaned Helpers"
anchors = ["crates/roko-cli/src/lib.rs", "crates/roko-core/src/lib.rs", "crates/roko-learn/src/routing_extras.rs", "lib.rs", "routing_extras.rs", "AgentState::research", "AgentState::llm_backend", "FileSubstrate::name"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #43, #276, and #283 — zero-caller shims/helpers and broad dead-code allowances obscure the live API. The audit found deprecated one-to-one wrappers plus non-deprecated helpers, constants, aliases, and scaffolding with no production callers. Broad allowances make it hard for a…

Imported without verification from:
- `tmp/backlog/archive/342-deprecated-shim-and-orphaned-helper-retirement.md#342 — Retire Verified Deprecated Shims and Orphaned Helpers`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#1.5 Remove 8 zero-caller `#[deprecated]` items`

How to verify: Check: Every deletion row is absent; `knowledge_store` remains and only its stale attribute is gone.; No module-level `#![allow(dead_code)]` remains in `roko-learn/src/routing_extras.rs`; blanket active-CLI cleanup remains exclusively owned by… [evidence: own status: Blocked on #43, #276, and #283]
