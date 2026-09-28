+++
id = "gap-e204ec"
kind = "gap"
title = "[tool T019] Tool result folding/search/selection/follow-tail lacks TUI snapshot evidence"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/transcript"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/transcript/fold.rs", "crates/roko-cli/src/transcript/projection.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
fold.rs/projection.rs exist (On main) but Packet F gap: TUI snapshots at three target sizes and under resize/follow-tail actions were never generated.

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Check for TUI snapshot tests (insta/ratatui TestBackend) at 80x24/120x40/200x60 covering fold and follow-tail; check FoldState is used by a live TUI view.
