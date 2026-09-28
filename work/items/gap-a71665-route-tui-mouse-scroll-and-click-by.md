+++
id = "gap-a71665"
kind = "gap"
title = "Route TUI Mouse Scroll and Click by Rendered Panel Coordinates"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/368-tui-mouse-hit-testing-scroll.md#368 — Route TUI Mouse Scroll and Click by Rendered Panel Coordinates"
discovered_from = "audit:tmp/backlog/archive/368-tui-mouse-hit-testing-scroll.md#368 — Route TUI Mouse Scroll and Click by Rendered Panel Coordinates"
anchors = ["tmp/tui-parity2/22-scroll-focus-selection.md", "TuiAction"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #237 and #365 —

Imported without verification from:
- `tmp/backlog/archive/368-tui-mouse-hit-testing-scroll.md#368 — Route TUI Mouse Scroll and Click by Rendered Panel Coordinates`

How to verify: Check: Scrolling a non-focused hovered panel scrolls/focuses that panel only.; Modal scroll/click cannot affect underlying content.; Detail panes do not share scroll offsets across tabs. [evidence: own status: Blocked on #237 and #365]
