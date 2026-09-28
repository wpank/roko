+++
id = "gap-296df0"
kind = "gap"
title = "[status-quo P3-D] Provider-internal visibility and adaptive immune memory (E34 residuals)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/safety"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/06-remaining-work.md#16. Provider-internal visibility"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/06-remaining-work.md#16. Provider-internal visibility"
anchors = ["immune Graph", "provider trace Signals", "crates/roko-core/src/immune.rs::ImmuneMemory", "crates/roko-core/src/immune.rs::ImmuneCalibration"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Provider-owned internal calls/results and provider trace Signals are not visible to the immune Graph; adaptive/semantic immune memory not implemented. Deferred per E34 acceptance scope.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/06-remaining-work.md#16. Provider-internal visibility`
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P3 items: 8/12 done (+1 P3-L, +1 P3-B)`
- `docs/v3/39-ROADMAP.md#7.7 Provider-Internal Security Visibility`
- `crates/roko-core/src/immune.rs:22`
- `docs/v3/39-ROADMAP.md#2.3 P2 -- Medium (Selected)`

How to verify: Check GAPS.md E34 residual list against safety code. / grep ImmuneMemory/ImmuneCalibration uses outside immune.rs.

Merged 2 mined candidates: m3-098, m5-090.
