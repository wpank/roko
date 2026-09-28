+++
id = "spec-f20136"
kind = "spec"
title = "PRD: Cursor Composer 2 ACP backend"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/cursor-acp"]
created = 2026-05-06
updated = 2026-09-28
source = ".roko/prd/drafts/cursor-composer-backend.md"
discovered_from = "audit:.roko/prd/drafts/cursor-composer-backend.md"
anchors = ["ProviderKind::CursorAcp"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Draft PRD (roko-agent): spawn `agent --force --workspace <dir> --output-format json acp`, JSON-RPC over stdio (session/new, session/prompt), port Mori CursorAcpConnection. A PRD-local tasks.toml exists. CursorAcp provider kind now exists.

Imported without verification from:
- `.roko/prd/drafts/cursor-composer-backend.md`
- `.roko/prd/plans/cursor-composer-backend/tasks.toml`

How to verify: Likely implemented; confirm CursorAcp covers REQs, then mark PRD done/archived.
