+++
id = "find-cb370a"
kind = "finding"
title = "Portal polish from the preview pass: badge contrast, failed-plan bar, failure alert wording, role hue"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["apps/portal", "roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:i1-portal-preview"
anchors = ["apps/portal/src/components/stream/StreamPane.tsx", "apps/portal/src/components/rail/PlanRow.tsx", "apps/portal/src/styles/tokens.css", "crates/roko-serve/src/routes/plans.rs:540"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "completed with task-level failures" crates/roko-serve/src/routes/plans.rs'
+++

Found in the fourth portal preview pass (i1), to do after the backend merge:

- The transcript tab's count badge (`stream-badge`) is white on the running indigo (`--indigo: #8993f0`, about 2.8:1, below WCAG AA) and borrows the running hue.
- A failed plan's rail row and progress bar are all red even when earlier tasks passed; the passed part should stay distinct.
- The failure alert reads "plan <id> completed with task-level failures" (text from routes/plans.rs:540). It should name the task and check, e.g. "<title>: T02 failed (structural check)". Retry is offered twice (header and row).
- The implementer role colour (`--orchid: #c886ea`, tokens.css:84) is close to the running indigo.

Checked in code on 2026-09-29: the alert text, the two colour tokens and the badge. The rail colouring and the duplicate Retry were not re-checked. The verify command covers only the alert text.
