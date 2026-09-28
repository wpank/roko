+++
id = "find-7aaf68"
kind = "finding"
title = "docs/v2 status banners are stale and contradict v3"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["docs/v2"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v2/25-DEPLOYMENT.md:5"
discovered_from = "audit:docs/v2/25-DEPLOYMENT.md:5"
anchors = ["docs/v2/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
docs/v2 chapters carry 2026-08 banners that conflict with v3/CLAUDE.md (e.g. systemd 'IMPLEMENTED' vs v3 'designed, not implemented'; demurrage 'not wired' vs E24; EFE routing gap vs E23). Mark v2 superseded or refresh banners.

Imported without verification from:
- `docs/v2/25-DEPLOYMENT.md:5`
- `docs/v2/00-INDEX.md:6`
- `docs/v2/28-ROADMAP.md:62`

How to verify: Check whether docs/v2 has a deprecation header like docs/v1.
