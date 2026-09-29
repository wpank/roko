+++
id = "spec-758405"
kind = "spec"
title = "PB-008: Auth + connection profile management"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-backlog/00-INDEX.md#PB-008"
discovered_from = "audit:tmp/portal-backlog/00-INDEX.md#PB-008"
anchors = ["apps/portal/src/app/settings/connection/page.tsx", "apps/portal/src/api/client.ts::updateConnection", "apps/portal/src/lib/env.ts"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'Bearer' apps/portal/src/api/client.ts && test -f apps/portal/src/app/settings/connection/page.tsx"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Built: settings/connection/page.tsx manages named connection profiles (name, url, apiKey), probes /api/health (:48) and activates a profile through api.updateConnection (:429). api/client.ts sends the saved key as `Authorization: Bearer` (:43-54, :167), and lib/env.ts saves the chosen URL in localStorage. Landed in 9c6ec420c. Note: plans/portal-programme/05-portal-foundation T02 will delete the settings route directory, and the SSE client sends no credentials (tracked under spec-ddc287)."
+++
Auth + connection profile management (index entry only; no spec file)

Imported without verification from:
- `tmp/portal-backlog/00-INDEX.md#PB-008`

How to verify: Check portal app for this page/feature; cross-check plans/portal-programme/* and existing web app dirs.

Verified 2026-09-28: built (settings/connection, api/client.ts). See [closed].
