+++
id = "spec-932ced"
kind = "spec"
title = "Automated Visual Assessment Loop"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/153-automated-visual-assessment-loop.md#153 — Automated Visual Assessment Loop"
discovered_from = "audit:tmp/backlog/archive/153-automated-visual-assessment-loop.md#153 — Automated Visual Assessment Loop"
anchors = ["tui/visual_assessment.rs", "gate-thresholds.json", "crates/roko-cli/src/tui/visual_assessment.rs", "crates/roko-cli/src/runner/gate_dispatch.rs", "crates/roko-cli/src/screenshot.rs", "crates/roko-core/src/config/mod.rs", "capture()", "roko screenshot reference import <dir>"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
capstone integration that ties screenshot capture, comparison, and plan generation into a self-improving visual quality loop. Roko's self-development loop (PRD → plan → execute → gate → learn) currently validates correctness through compile, test, clippy, and diff gates. It has no mechanism to…

Imported without verification from:
- `tmp/backlog/archive/153-automated-visual-assessment-loop.md#153 — Automated Visual Assessment Loop`

Some cited files are gone: `crates/roko-cli/src/screenshot.rs`, `crates/roko-cli/src/tui/visual_assessment.rs`, `tui/visual_assessment.rs`.

How to verify: Check: Visual assessment runs automatically after TUI-modifying tasks; Regressions detected with > configurable threshold produce a fix plan; Fix plans are executable through the normal runner-v2 loop [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): L | 7 |]
