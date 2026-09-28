+++
id = "gap-aabeff"
kind = "gap"
title = "[tool T003] TUI paths still consume string/tail projections that lose semantics (TUI adoption unproven)"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/tui"]
created = 2026-09-14
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/transcript/convert.rs", "crates/roko-cli/src/transcript/block.rs", "crates/roko-core/src/transcript_store.rs", "crates/roko-cli/src/tui/views/dashboard_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'TranscriptBlock|blocks_from_records' crates/roko-cli/src/tui"
+++
Register marks On main via bounded TranscriptStore in roko-core, but Packet F release gate is open: TUI terminal-size snapshots (80x24/120x40/200x60) never generated and live-vs-replay equivalence unverified. Related T002/T018 (start/result pairing, shared TranscriptBlock renderer).

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Check whether TUI Dashboard/Agents output panes render from TranscriptStore/TranscriptBlock (blocks_from_records) rather than output_lines/last_output_line string tails or '[tool_call] ...' strings.

Verified 2026-09-28: No file under crates/roko-cli/src/tui references TranscriptStore, TranscriptBlock or blocks_from_records, while output_lines/last_output_line string tails appear 91 times under tui/.
