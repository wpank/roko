+++
id = "gap-7d2055"
kind = "gap"
title = "TUI Keyboard Model Fixes"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/237-tui-keyboard-model-fixes.md#237 — TUI Keyboard Model Fixes"
discovered_from = "audit:tmp/backlog/archive/237-tui-keyboard-model-fixes.md#237 — TUI Keyboard Model Fixes"
anchors = ["crates/roko-cli/src/tui/input.rs", "crates/roko-cli/src/tui/app.rs", "input.rs:60-67", "input.rs", "app.rs", "crates/roko-cli/src/tui/state.rs", "FocusZone::next()", "Tab::Dashboard"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Global number key shadowing prevents per-tab number handlers from firing; `v` is mapped to effects cycle instead of verify; 7 tabs lack focus zones.. Three keyboard model issues reduce TUI usability:

Imported without verification from:
- `tmp/backlog/archive/237-tui-keyboard-model-fixes.md#237 — TUI Keyboard Model Fixes`

Some cited files are gone: `crates/roko-cli/src/tui/app.rs`, `crates/roko-cli/src/tui/state.rs`.

How to verify: Check: On F2 Plans, pressing `1` selects the first plan in the list (not a sub-tab).; On F1 Dashboard, pressing `1-6` still switches right-panel sub-tabs.; Pressing `v` navigates to the verify/gate view. [evidence: own status: Done (implemented 2026-09-02, landed with #365); 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 4 |; (newer evidence overrides own status "closed")]
