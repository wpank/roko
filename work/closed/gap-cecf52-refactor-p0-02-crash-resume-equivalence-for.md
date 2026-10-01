+++
id = "gap-cecf52"
kind = "gap"
title = "[refactor P0-02] Crash/resume equivalence for Graph engine never proven (harness designed only)"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-graph/engine"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness"
anchors = ["crates/roko-graph/tests/crash_resume_equivalence.rs", "crates/roko-graph/src/replay.rs::ActivityReplayer"]
links = { depends_on = [], blocks = [], related = ["bug-975f77", "gap-dcae09"], supersedes = [], duplicate_of = "gap-6ca8fb" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-6ca8fb: the same missing kill-point crash/resume harness. Checked 2026-09-28: the only harness is the in-process error-injection test crates/roko-graph/tests/crash_resume_equivalence.rs (P0-GE-1; 3-node linear DAG, 4 tests). The designed kill-at-random-point harness with 7 checkpoint sites (refactor P0-02) and crates/roko-cli/tests/graph_crash_resume.rs do not exist. Severity p0 overstated a missing proof."
+++
Kill-at-random-point harness with chaos injection at 7 checkpoint sites designed across resume paths (one-shot snapshot, hot durable; Runner-v2 legacy path now obsolete) but not implemented. Engine-audit #138 claimed coverage via checkpoint/Activity replay.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness`
- `tmp/archive/refactoring-audit-2026-09-21/P0-02-CRASH-RESUME-DESIGN.md`

How to verify: Search for a crash-injection/kill-point test that resumes and compares final state to an uninterrupted run.

Verified 2026-09-28: still true (no kill-point harness) but duplicate of gap-6ca8fb.
