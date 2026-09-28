+++
id = "gap-5fbcd3"
kind = "gap"
title = "TP1-MX.7 / TP2-34 #123: ROSEDUST contrast failures and no 256-color fallback"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui/theme"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)"
discovered_from = "audit:tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)"
anchors = ["crates/roko-cli/src/tui/theme.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
v1 audit: ROSEDUST fails WCAG contrast and the TUI is 24-bit RGB only; palette port/fallback proof never completed and text captures cannot verify colors.

Imported without verification from:
- `tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)`
- `tmp/tui-parity/00-INDEX.md#3. New items from the UX audit not in this tracker`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`

How to verify: Check theme for a 256-color/ANSI fallback and contrast ratios.
