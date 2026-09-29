+++
id = "gap-872a5a"
kind = "gap"
title = "DOCS-02: Cross-cutting documentation gaps no doc version covers"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["docs"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/02-DOCS-VS-CODE.md#Cross-Cutting Documentation Gaps"
discovered_from = "audit:tmp/docs-audit/02-DOCS-VS-CODE.md#Cross-Cutting Documentation Gaps"
anchors = ["docs/v3/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Missing guides: step-by-step Graph engine execution walkthrough, per-provider setup (12 kinds), error propagation patterns, test strategy/subsets, deployment cookbook, plugin development guide, ACP v0.13 integration guide.

Imported without verification from:
- `tmp/docs-audit/02-DOCS-VS-CODE.md#Cross-Cutting Documentation Gaps`

How to verify: grep docs/v3 for these guide topics.
