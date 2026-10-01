+++
id = "gap-f46b85"
kind = "gap"
title = "Enforce Edge Validation at Every Graph Entry Point"
status = "done"
triage = "verified"
severity = "p0"
subsystem = ["roko-graph"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/271-graph-engine-edge-validation.md#271 — Enforce Edge Validation at Every Graph Entry Point"
discovered_from = "audit:tmp/backlog/archive/271-graph-engine-edge-validation.md#271 — Enforce Edge Validation at Every Graph Entry Point"
anchors = ["crates/roko-graph/src/engine.rs::validate_for_start", "crates/roko-graph/src/engine.rs::resume_from"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-graph --lib graph_validation"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Implemented (source status 2026-09-04) and present at 91b4745f8. GraphEngine::validate_for_start (crates/roko-graph/src/engine.rs:492) rejects test-stub nodes via registry descriptors and runs graph.validate_edges with descriptor introspection (:523). It is called by execute (:558), execute_at_tick (:575), execute_parallel_at_tick (:933) and start (:1663), and resume_from validates at :1440. Tests: graph_validation_* in engine.rs (:4681+). The uncommitted engine.rs diff does not touch validation."
+++
invalid typed edges can reach execution even though the validator already exists

Imported without verification from:
- `tmp/backlog/archive/271-graph-engine-edge-validation.md#271 — Enforce Edge Validation at Every Graph Entry Point`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#Phase A: Foundation (Wave 0-1, ~3-5 days #271`

Some cited files are gone: `tmp/engine-audit/02-graph-engine-gaps.md`.

How to verify: Check: Replace empty-TOML Cell construction during schema validation with side-effect-free registry schema introspection or validation against each node's actual config.; Call typed-edge validation from `execute`, `execute_parallel`… [evidence: own status: Implemented (2026-09-04); 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): -- | engine DAG | Done (Wave 0; 2026-09-04); (newer evidence overrides own status…]

Verified 2026-09-28: closed as done; every public entry point validates edges.
