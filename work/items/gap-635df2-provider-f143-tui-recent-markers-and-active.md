+++
id = "gap-635df2"
kind = "gap"
title = "[provider F143] TUI recent_markers and active_biases always empty Vecs"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F143"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F143"
anchors = ["crates/roko-cli/src/runner/tui_bridge.rs", "recent_markers", "active_biases"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`affect_updated()` on `TuiBridge` passes `Vec::new()` for both `recent_markers` and `active_biases`. The TUI cannot display which somatic markers or affect biases are currently active.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F143`
- `tmp/archive/provider-audit/16-daimon-affect.md`

How to verify: Confirm in crates/roko-cli/src/runner/tui_bridge.rs whether still true: TUI `recent_markers` and `active_biases` always empty Vecs
