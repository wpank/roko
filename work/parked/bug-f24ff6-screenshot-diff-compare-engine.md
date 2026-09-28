+++
id = "bug-f24ff6"
kind = "bug"
title = "Screenshot Diff/Compare Engine"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/152-screenshot-diff-compare-engine.md#152 — Screenshot Diff/Compare Engine"
discovered_from = "audit:tmp/backlog/archive/152-screenshot-diff-compare-engine.md#152 — Screenshot Diff/Compare Engine"
anchors = ["tmp/mori-old/", "tui/screenshot_diff.rs", ".roko/screenshots/baselines/", "roko.toml", "crates/roko-cli/src/tui/screenshot_diff.rs", "crates/roko-cli/src/tui/mod.rs", "crates/roko-cli/src/screenshot.rs", "crates/roko-cli/src/config.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
enables automated visual regression testing of TUI changes against baselines and Mori reference screenshots. With PNG snapshot rendering (#151), Roko can capture pixel-level TUI images. The next step is comparing them: detecting visual differences between a current capture and a reference image…

Imported without verification from:
- `tmp/backlog/archive/152-screenshot-diff-compare-engine.md#152 — Screenshot Diff/Compare Engine`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P2-TUI-8 (Subsystem: TUI)`

Some cited files are gone: `.roko/screenshots/baselines/`, `crates/roko-cli/src/screenshot.rs`, `tmp/mori-old/`.

How to verify: Check: `roko screenshot diff --reference ref.png` produces a similarity score and diff image; Text diff mode works for terminal text snapshots; PNG diff correctly identifies changed pixels with configurable tolerance NOTE: Same scope as CONSOLIDATED P2-TUI-8 / TUI-G5 (baselines, tolerance masks, CI gating). [evidence: CONSOLIDATED P2-TUI-8: open; 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 4 |]
