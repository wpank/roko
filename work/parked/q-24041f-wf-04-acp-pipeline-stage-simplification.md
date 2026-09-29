+++
id = "q-24041f"
kind = "question"
title = "WF-04: ACP pipeline stage simplification"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-acp"]
created = 2026-09-23
updated = 2026-09-28
source = "tmp/workflow-audit/04-ACP-STAGE-AUDIT.md#Recommendation"
discovered_from = "audit:tmp/workflow-audit/04-ACP-STAGE-AUDIT.md#Recommendation"
anchors = ["crates/roko-acp/src/pipeline.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Deferred: keep backend stages but show only generating/executing/done in UI (Option C); later drop Strategizing, fold AutoFixing into Implementing retries, make Reviewing and Committing opt-in.

Imported without verification from:
- `tmp/workflow-audit/04-ACP-STAGE-AUDIT.md#Recommendation`
- `tmp/workflow-audit/04-ACP-STAGE-AUDIT.md#Future Work`

How to verify: Owner decision; check portal/ACP UI stage exposure.
