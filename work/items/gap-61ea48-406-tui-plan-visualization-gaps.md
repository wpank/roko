+++
id = "gap-61ea48"
kind = "gap"
title = "#406 — TUI Plan Visualization Gaps"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/406-tui-plan-visualization-gaps.md##406 — TUI Plan Visualization Gaps"
discovered_from = "audit:tmp/backlog/archive/406-tui-plan-visualization-gaps.md##406 — TUI Plan Visualization Gaps"
anchors = ["tmp/plan-audit/14-tui-visualization.md", "crates/roko-cli/src/tui/modals/batch_review.rs", "crates/roko-cli/src/tui/state/mod.rs", "modals/mod.rs", "crates/roko-cli/src/tui/widgets/plan_tree.rs", "plans_view.rs", "crates/roko-cli/src/tui/views/plans_view.rs", "crates/roko-cli/src/tui/app/actions.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The F2 Plans TUI view has good parity for wave tree rendering, task dependency trees, and the plan detail panel, but is missing five visualization features that Mori had. The highest-impact gap is the batch review modal, which is currently a 46-line stub that renders a single summary string. The…

Imported without verification from:
- `tmp/backlog/archive/406-tui-plan-visualization-gaps.md##406 — TUI Plan Visualization Gaps`

Some cited files are gone: `tmp/plan-audit/14-tui-visualization.md`.

How to verify: Check whether the gap described in tmp/backlog/archive/406-tui-plan-visualization-gaps.md still exists at the anchored paths. [evidence: no status line; no index/roll-up evidence]
