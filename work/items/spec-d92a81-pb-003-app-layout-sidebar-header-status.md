+++
id = "spec-d92a81"
kind = "spec"
title = "PB-003: App layout (sidebar, header, status bar)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-backlog/PB-003-layout-shell.md#PB-003"
discovered_from = "audit:tmp/portal-backlog/PB-003-layout-shell.md#PB-003"
anchors = ["apps/portal/src/components/layout/Sidebar.tsx", "apps/portal/src/components/layout/AppShell.tsx"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Superseded by plans/portal-programme/05-portal-foundation T02/T11 and 06-portal-shell T02. The PB-003 shell was built in 9c6ec420c: a 7-section Sidebar with a 56px collapsed mode and a connection indicator, Header, StatusBar, DrawerOverlay (480px, Escape to dismiss) and SubNav. Only the mobile (<768px) bottom tab bar was never built. Plan 05 T02 deletes Sidebar, SubNav, Header, AppShell and all section routes, and 06 T02 replaces them with a three-pane workspace shell, so that remaining piece no longer applies."
+++
App layout (sidebar, header, status bar). Build the persistent layout: sidebar navigation (7 sections), header bar, status bar footer, and drawer overlay system. Every page renders inside this shell.

Imported without verification from:
- `tmp/portal-backlog/PB-003-layout-shell.md#PB-003`

How to verify: Check portal app for this feature (8 acceptance criteria, e.g. `src/app/layout.tsx` — root layout with ROSEDUST background, font loading, providers; `src/components/layout/Sidebar.tsx`:); cross-check plans/portal-programme/* and existing web app dirs.

Verified 2026-09-28: superseded by the portal-programme shell redesign. See [closed].
