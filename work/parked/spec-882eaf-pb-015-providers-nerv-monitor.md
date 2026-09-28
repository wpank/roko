+++
id = "spec-882eaf"
kind = "spec"
title = "PB-015: Providers: NERV monitor"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
source = "tmp/portal-backlog/PB-015-providers-monitor.md#PB-015"
discovered_from = "audit:tmp/portal-backlog/PB-015-providers-monitor.md#PB-015"
anchors = ["apps/portal", "src/app/providers/page.tsx", "tmp/portal/spec/09-PROVIDERS.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Providers: NERV monitor. Build the NERV-aesthetic provider health monitor — the most visually distinctive page in the portal. Evangelion-inspired institutional monitoring with katakana headers, dense tabular data, and waveform traces.

Imported without verification from:
- `tmp/portal-backlog/PB-015-providers-monitor.md#PB-015`

How to verify: Check portal app for this feature (10 acceptance criteria, e.g. Route: `src/app/providers/page.tsx`; NERV Header: "PROVIDER MONITOR" in fullwidth Unicode katakana, double border in `--rose-dim`); cross-check plans/portal-programme/* and existing web app dirs.
