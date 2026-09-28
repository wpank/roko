+++
id = "gap-0eefb7"
kind = "gap"
title = "Compile auto-fix (cargo fix before agent retry) not wired to dispatch"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/gates"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs", "crates/roko-gate/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Compile Auto-Fix scaffolding exists but final wiring to agent dispatch is missing (backlog #04). Listed under roadmap §3.3 half-implemented features.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C6. Compile Auto-Fix path`

How to verify: Locate the scaffolding named in the roadmap row and confirm no runtime caller. / Find auto-fix types and their callers.

Merged 2 mined candidates: m5-071, m4-045.
