+++
id = "spec-52eeb9"
kind = "spec"
title = "PB-006: Zustand dashboard store + event dispatch"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/portal-backlog/PB-006-zustand-store.md#PB-006"
discovered_from = "audit:tmp/portal-backlog/PB-006-zustand-store.md#PB-006"
anchors = ["apps/portal/src/stores/dashboard.ts::replaceFromSnapshot", "apps/portal/src/lib/runState.ts::fromSnapshot", "crates/roko-core/src/dashboard_snapshot.rs::DashboardSnapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqE \"recentGates|recentEpisodes|recentErrors|inboxItems\" apps/portal/src && grep -q \"fromSnapshot(s\" apps/portal/src/stores/dashboard.ts && (cd apps/portal && npx vitest run src/lib/runState.accept.test.ts src/lib/runState.test.ts)"

[closed]
at = 2026-09-29
commit = "39e72deef"
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "39e72deef (plan 05) rewrote apps/portal/src/stores/dashboard.ts to 56 lines: replaceFromSnapshot (:51) calls lib/runState.ts fromSnapshot (:1001), which reads the snake_case wire keys (plans, plan_set, tasks, gates, errors, p.tasks_total, ...) that match the unrenamed serde fields of crates/roko-core/src/dashboard_snapshot.rs:1194 DashboardSnapshot. The camelCase reads (recentGates/recentEpisodes/recentErrors/inboxItems) no longer occur anywhere in apps/portal/src, and stores/agents.ts and stores/intelligence.ts are deleted, so gap recovery no longer wipes slices."
+++
Zustand dashboard store + event dispatch. Implement the central Zustand store that holds all dashboard state. SSE events from the SSE client dispatch into this store. React components subscribe to slices via selectors.

Imported without verification from:
- `tmp/portal-backlog/PB-006-zustand-store.md#PB-006`

Some cited files are gone: `src/stores/compose.ts`.

How to verify: Check portal app for this feature (10 acceptance criteria, e.g. `src/stores/dashboard.ts` — main store with slices:; `applyEvent()` dispatches each `DashboardEvent` type to correct slice mutations); cross-check plans/portal-programme/* and existing web app dirs.

Verified 2026-09-28: the store is built but has a live defect. apps/portal/src/stores/dashboard.ts (815 lines) has applyEvent, replaceSnapshot and dismissInboxItem, with ring-buffer caps of 256/128/64/200 (:62-65). agents.ts and intelligence.ts exist; compose.ts (Phase 3) does not. The defect: replaceSnapshot (:247-266) reads camelCase keys (recentGates, recentEpisodes, recentErrors, inboxItems), but the server's DashboardSnapshot (crates/roko-core/src/dashboard_snapshot.rs:1181) has no serde rename, so gap recovery wipes those slices. Fixes are planned in plans/portal-programme/05-portal-foundation T04 (snapshot replacement) and T06 (dropped events), and T02 deletes agents.ts and intelligence.ts. Only that defect remains in scope. Severity lowered p1 to p2.
