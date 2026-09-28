+++
id = "gap-fc9958"
kind = "gap"
title = "[cli-audit #209] Provider proof matrix (hermetic matrix + prebuilt-binary live script)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/providers"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 4 — parity, cutover, and lifecycle UX"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 4 — parity, cutover, and lifecycle UX"
anchors = ["backlog #209-provider-proof-matrix.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
All prerequisites (#192/#208/#243/#248/#262/#333) done, but #209 provider proof matrix is not run: hermetic matrix in B2/B7 batches and live prebuilt-binary script at the final release gate.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 4 — parity, cutover, and lifecycle UX`
- `tmp/archive/cli-audit-2026-09-21/FINDINGS-COVERAGE-MATRIX.md#Verification-only / archive queue`

How to verify: Search for a provider proof-matrix test/script; check whether it runs in CI.
