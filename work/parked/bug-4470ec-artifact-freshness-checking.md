+++
id = "bug-4470ec"
kind = "bug"
title = "Artifact Freshness Checking"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/184-artifact-freshness-checking.md#184 — Artifact Freshness Checking"
discovered_from = "audit:tmp/backlog/archive/184-artifact-freshness-checking.md#184 — Artifact Freshness Checking"
anchors = ["crates/roko-cli/src/plan_validate.rs", "crates/roko-cli/src/runner/plan_loader.rs", "crates/roko-cli/src/runner/event_loop.rs", ".roko/prd/<slug>/", "prd.md", "draft.md", "plan.md", ".roko/research/<slug>/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
correctness safeguard; plans can silently run against stale PRD or research artifacts with no warning. The plan execution pipeline assumes that enrichment artifacts (PRDs in `.roko/prd/`, research in `.roko/research/`) are up to date relative to the `tasks.toml` they inform. In practice, an…

Imported without verification from:
- `tmp/backlog/archive/184-artifact-freshness-checking.md#184 — Artifact Freshness Checking`

Some cited files are gone: `.roko/prd/<slug>/`, `.roko/research/<slug>/`, `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: `roko plan validate <dir>` warns when PRD or research artifacts are newer than `tasks.toml`; Runner preflight logs freshness warnings before dispatching the first task; Warnings include the stale file path and both timestamps [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): XS | 7 |]
