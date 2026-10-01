+++
id = "gap-2bc986"
kind = "gap"
title = "Paged Agent Output History and Search"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["workspace"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/backlog/archive/367-tui-agent-output-history-search.md#367 — Paged Agent Output History and Search"
discovered_from = "audit:tmp/backlog/archive/367-tui-agent-output-history-search.md#367 — Paged Agent Output History and Search"
anchors = ["crates/roko-cli/src/tui/state/mod.rs::AgentOutputHistory", "crates/roko-cli/src/tui/state/mod.rs::bounded_output_lines", "crates/roko-cli/src/tui/state/snapshot.rs:806", "crates/roko-cli/src/tui/input.rs:35"]
links = { depends_on = [], blocks = [], related = ["spec-5c8b9c", "gap-aabeff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q \"bounded_output_lines\" crates/roko-cli/src/tui/state/snapshot.rs && grep -rqE \"\\.before\\(\" crates/roko-cli/src/tui --include='*.rs' --exclude=tests.rs"
+++
[blocked] Blocked on #208, #248, and #366 —

Imported without verification from:
- `tmp/backlog/archive/367-tui-agent-output-history-search.md#367 — Paged Agent Output History and Search`

How to verify: Check: More than 50 lines remain scrollable and older-than-memory content pages in.; Search finds both in-memory and older canonical records.; Live and settled copies of one event render once. [evidence: own status: Blocked on #208, #248, and #366]

Verified 2026-09-28: partly done. AgentOutputHistory (crates/roko-cli/src/tui/state/mod.rs:500) keeps 2,000 records per agent (MAX_AGENT_OUTPUT_RECORDS, :427), with seq-based before() pagination and a regex search() (:637), and the input layer has an AgentOutputSearch mode (tui/input.rs:34, :526). Still missing: nothing loads older records from the canonical runtime event history. The type's doc (:497) leaves that to 'the caller', and no caller does it. Search covers only the in-memory window, and the legacy 50-line bounded_output_lines helper (:424, :829) is still there. Blocker #208 is done (see spec-5c8b9c). tui/state/mod.rs has a small uncommitted edit in the working tree (+4 lines). Severity lowered p1 to p2 because this is TUI usability.

Re-checked 2026-09-29: unchanged in substance; the line numbers moved. AgentOutputHistory is now tui/state/mod.rs:515, MAX_AGENT_OUTPUT_RECORDS :442, before() :632, search() :658, and bounded_output_lines :926 (still called from tui/state/snapshot.rs:806 and :1127). Still missing: a caller that loads older records from the canonical runtime event history, and search beyond the in-memory window.

## Notes

- 2026-10-01 (wk-tuiv): blocked; no code changed. Re-checked at BASE. The in-memory part works:
  `AgentOutputHistory` keeps 2,000 records per agent, the Agents tab renders and scrolls all of them
  (`agents_view.rs` reads `records_for`), search covers them, and live and settled copies dedupe (`push_dedup`,
  `settle_screened_transcript`). What remains:
  (1) Paging records older than memory from the canonical log. `AgentOutput` events are persisted with their text
  in `.roko/events.jsonl` and `.roko/events-by-run/<sha256(run id)>.jsonl` (`WorkspaceEventLog`), but a record
  carries a TUI-local `seq` and a push-time timestamp, so nothing maps an evicted record to its canonical event.
  Turning `AgentOutput` events into records happens in `tui/app/channels.rs` (`push_agent_output_record`), the
  path gap-aabeff (wk-streams, p1, L) is reworking.
  (2) Search beyond memory, which needs the same reader.
  `bounded_output_lines` in `tui/state/snapshot.rs` is a deliberate defensive cap (test
  `connected_snapshot_output_is_bounded_to_source_ring_limit`): remove it once the paged history replaces the
  50-line rings, not before.
  Next step, after gap-aabeff: give persisted `AgentOutput` events a per-agent sequence (or keep each event's
  byte offset in the run index), read an agent's page of events from the run index through the shared
  event-to-record conversion, page it in when the output pane scrolls past its oldest record (through
  `AgentOutputHistory::before`), and run the search over the same pages.
