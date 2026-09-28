+++
id = "gap-0addc3"
kind = "gap"
title = "Expose and Verify Learning Graduation Status"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-gate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/334-learning-graduation-command.md#334 — Expose and Verify Learning Graduation Status"
discovered_from = "audit:tmp/backlog/archive/334-learning-graduation-command.md#334 — Expose and Verify Learning Graduation Status"
anchors = ["GraduationStatus", "GraduationConfig", "GraduationConfig::should_graduate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #257 and #267 —

Imported without verification from:
- `tmp/backlog/archive/334-learning-graduation-command.md#334 — Expose and Verify Learning Graduation Status`

How to verify: Check: `roko learn graduation` reports active/inactive state, resolved policies, the observation window, and typed totals without mutating workspace state.; An inactive or not-yet-wired cell is reported explicitly and is never rendered as a healthy… [evidence: own status: Blocked on #257 and #267]
