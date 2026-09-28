+++
id = "gap-4f1f10"
kind = "gap"
title = "TUI PNG Snapshot Rendering"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/151-tui-png-snapshot-rendering.md#151 — TUI PNG Snapshot Rendering"
discovered_from = "audit:tmp/backlog/archive/151-tui-png-snapshot-rendering.md#151 — TUI PNG Snapshot Rendering"
anchors = ["roko-cli/Cargo.toml", "tui/png_renderer.rs", "roko.toml", "crates/roko-cli/Cargo.toml", "crates/roko-cli/src/tui/png_renderer.rs", "crates/roko-cli/src/tui/mod.rs", "crates/roko-cli/src/screenshot.rs", "CrosstermBackend", "tui-png feature", "png_renderer.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
enables automated visual assessment of TUI quality; text snapshots exist but pixel-level visual inspection requires PNG output. Roko's TUI (built with ratatui in `crates/roko-cli/src/tui/`) currently supports text-based terminal snapshots via the screenshot command. However, for automated visual…

Imported without verification from:
- `tmp/backlog/archive/151-tui-png-snapshot-rendering.md#151 — TUI PNG Snapshot Rendering`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Summary`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Remaining Open Work`
- `tmp/tui-parity2/36-OPEN-GAPS.md#P2 — rendering and evidence`
- `tmp/tui-parity2/32-SCREENSHOT-HARNESS-STATUS.md#Evidence boundary`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Missing`

Some cited files are gone: `crates/roko-cli/src/screenshot.rs`.

How to verify: Check: `roko screenshot --format png` produces a valid PNG file; PNG output visually matches the terminal rendering (correct colors, characters, attributes); Font atlas is initialized once and reused across captures [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 4 |] / cargo tree -p roko-cli --features tui-png | grep image.

Merged 2 mined candidates: m1-026, m4-131.
