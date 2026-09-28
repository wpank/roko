+++
id = "gap-2bc986"
kind = "gap"
title = "Paged Agent Output History and Search"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["workspace"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/367-tui-agent-output-history-search.md#367 — Paged Agent Output History and Search"
discovered_from = "audit:tmp/backlog/archive/367-tui-agent-output-history-search.md#367 — Paged Agent Output History and Search"
anchors = ["crates/roko-cli/src/tui/state/mod.rs::AgentOutputHistory", "crates/roko-cli/src/tui/state/mod.rs::bounded_output_lines", "crates/roko-cli/src/tui/input.rs:34"]
links = { depends_on = [], blocks = [], related = ["spec-5c8b9c"], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #208, #248, and #366 —

Imported without verification from:
- `tmp/backlog/archive/367-tui-agent-output-history-search.md#367 — Paged Agent Output History and Search`

How to verify: Check: More than 50 lines remain scrollable and older-than-memory content pages in.; Search finds both in-memory and older canonical records.; Live and settled copies of one event render once. [evidence: own status: Blocked on #208, #248, and #366]

Verified 2026-09-28: partly done. AgentOutputHistory (crates/roko-cli/src/tui/state/mod.rs:500) keeps 2,000 records per agent (MAX_AGENT_OUTPUT_RECORDS, :427), with seq-based before() pagination and a regex search() (:637), and the input layer has an AgentOutputSearch mode (tui/input.rs:34, :526). Still missing: nothing loads older records from the canonical runtime event history. The type's doc (:497) leaves that to 'the caller', and no caller does it. Search covers only the in-memory window, and the legacy 50-line bounded_output_lines helper (:424, :829) is still there. Blocker #208 is done (see spec-5c8b9c). tui/state/mod.rs has a small uncommitted edit in the working tree (+4 lines). Severity lowered p1 to p2 because this is TUI usability.
