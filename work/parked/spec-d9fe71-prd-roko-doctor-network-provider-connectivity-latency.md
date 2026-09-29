+++
id = "spec-d9fe71"
kind = "spec"
title = "PRD: `roko doctor network` provider connectivity/latency probe (two drafts)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/doctor"]
created = 2026-08-17
updated = 2026-09-28
source = ".roko/prd/drafts/doctor-network-probe.md"
discovered_from = "audit:.roko/prd/drafts/doctor-network-probe.md"
anchors = ["roko doctor network"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Two drafts (doctor-network-probe, doctor-network-v2 skeleton) specify probing configured LLM provider endpoints for reachability and latency. CLAUDE.md lists `roko doctor network`.

Imported without verification from:
- `.roko/prd/drafts/doctor-network-probe.md`
- `.roko/prd/drafts/doctor-network-v2.md`

How to verify: Likely implemented; run `roko doctor network` and compare with PRD REQs, then dedupe/archive drafts.
