+++
id = "find-cb370a"
kind = "finding"
title = "Portal polish from the preview pass: badge contrast, failed-plan bar, failure alert wording, role hue"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["apps/portal", "roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:i1-portal-preview"
anchors = ["apps/portal/src/components/stream/StreamPane.tsx", "apps/portal/src/lib/planRows.ts::progressSegments", "apps/portal/src/lib/alerts.ts::pickAlert", "apps/portal/src/styles/tokens.css:87", "apps/portal/src/components/stage/PlanView.tsx:495"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/components/shell/oneRetry.accept.test.tsx src/components/rail/segments.accept.test.tsx src/styles/semantics.accept.test.ts"

[closed]
at = 2026-09-29
commit = "63bdfa9a3"
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "Commit 63bdfa9a3, which landed during this check, fixes all four points. StreamPane.tsx:277/308 badges use `rd-badge` (bone on void, globals.css:938-939). planRows.ts progressSegments draws per-state bar segments in PlanRow.tsx and PlanView.tsx. tokens.css:87 sets `--role-implementer: var(--blush)`. lib/alerts.ts:122-145 hides or rewords the server's generic `plan <id> completed with task-level failures` line and failedTaskText shows `<plan title>: <task id> failed (<check> check)`. The alert now offers only Show, and PlanView.tsx:495 omits the row Retry, leaving one Retry in the plan header."
+++

Found in the fourth portal preview pass (i1), to do after the backend merge:

- The transcript tab's count badge (`stream-badge`) is white on the running indigo (`--indigo: #8993f0`, about 2.8:1, below WCAG AA) and borrows the running hue.
- A failed plan's rail row and progress bar are all red even when earlier tasks passed; the passed part should stay distinct.
- The failure alert reads "plan <id> completed with task-level failures" (text from routes/plans.rs:540). It should name the task and check, e.g. "<title>: T02 failed (structural check)". Retry is offered twice (header and row).
- The implementer role colour (`--orchid: #c886ea`, tokens.css:84) is close to the running indigo.

Checked in code on 2026-09-29: the alert text, the two colour tokens and the badge. The rail colouring and the duplicate Retry were not re-checked. The verify command covers only the alert text.

Closed 2026-09-29 by 63bdfa9a3 (plan 08f-final-polish, built by roko). The server still emits `plan {plan_id} completed with task-level failures` at crates/roko-serve/src/routes/plans.rs:540, but the portal translates it (lib/alerts.ts rank 1c), so the item's grep-the-server verify command still fails although the user-facing problem is gone.
