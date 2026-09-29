+++
id = "spec-1ecf91"
kind = "spec"
title = "PB-004: Atom component library"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-backlog/00-INDEX.md#PB-004"
discovered_from = "audit:tmp/portal-backlog/00-INDEX.md#PB-004"
anchors = ["apps/portal/src/components/atoms/index.ts", "apps/portal/src/components/molecules/index.ts"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f apps/portal/src/components/atoms/index.ts && test -f apps/portal/src/components/molecules/index.ts"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Built: apps/portal/src/components/atoms/ holds Badge, Button, Pill, ProgressBar, Sparkline, Spinner, StatusLED, Toast and Tooltip (index.ts), and molecules/ (the merged PB-007 scope) holds AgentCard, GateResultRow, InboxItem, MetricBar, PlanCard, TaskRow, VitalCell and WaveformTrace. Landed in 9c6ec420c. Follow-on primitives are planned in plans/portal-programme/05-portal-foundation T09. There is no PB-004 spec file, so completeness was judged against the index entry only."
+++
Atom component library (index entry only; no spec file)

Imported without verification from:
- `tmp/portal-backlog/00-INDEX.md#PB-004`
- `tmp/portal-backlog/00-INDEX.md#PB-007`

How to verify: Check portal app for this page/feature; cross-check plans/portal-programme/* and existing web app dirs.

Merged 2 mined candidates: m1-250, m1-253.

Verified 2026-09-28: built (components/atoms, components/molecules). See [closed].
