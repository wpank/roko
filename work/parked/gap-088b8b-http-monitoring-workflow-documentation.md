+++
id = "gap-088b8b"
kind = "gap"
title = "HTTP Monitoring Workflow Documentation"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/routes"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/154-http-monitoring-workflow-documentation.md#154 — HTTP Monitoring Workflow Documentation"
discovered_from = "audit:tmp/backlog/archive/154-http-monitoring-workflow-documentation.md#154 — HTTP Monitoring Workflow Documentation"
anchors = ["crates/roko-serve/src/routes/", "docs/v2-depth/monitoring-workflows.md", "docs/v2-depth/endpoint-reference.md", "CLAUDE.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
documentation-only task; all endpoints already serve real data. Roko's HTTP control plane (`roko serve` on :6677) exposes ~317 routes covering plan execution, agent health, telemetry, learning state, and more. These endpoints serve real data and are used by the TUI's HTTP-backed panels and could…

Imported without verification from:
- `tmp/backlog/archive/154-http-monitoring-workflow-documentation.md#154 — HTTP Monitoring Workflow Documentation`

Some cited files are gone: `docs/v2-depth/endpoint-reference.md`, `docs/v2-depth/monitoring-workflows.md`.

How to verify: Check: Monitoring workflows document exists with actionable curl examples; Endpoint reference covers at least the 20 most important monitoring endpoints; Examples use real response shapes (verified against running `roko serve`) [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): XS | 7 |]
