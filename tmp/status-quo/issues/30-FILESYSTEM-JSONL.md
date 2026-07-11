# Filesystem and JSONL Store Issues

## Critical

### `events.jsonl` — 43MB write-only firehose
- 157,549 lines. 97% is `feed_tick` (152,965) + `chain_block` (3,291) events.
- `state_hub.rs:137-141`: Appends every `DashboardEvent` without filter.
- `FeedTick` and `ChainBlock` have zero effect on materialized state — pure noise.
- No rotation, truncation, or cap. Grows ~2-5MB/day under load.
- TUI cursor (`cursors.rs:139-145`) tries to parse entire file as single JSON value → ALWAYS fails. View is always empty.

### Two independent writers with no coordination
- `roko serve` opens `EventLogWriter` at `state.rs:842`.
- `roko run` opens SECOND writer for same path at `run.rs:1236`.
- Runner also appends different schema (`RunnerEvent`) via `append_jsonl` at `persist.rs:334`.
- No file lock. Concurrent processes can produce torn lines.

### Episodes triplicated — three files, three writers
- `.roko/episodes.jsonl` (82KB) — written by EpisodeLogger, FeedbackService
- `.roko/learn/episodes.jsonl` (166KB) — written by service_factory
- `.roko/memory/episodes.jsonl` (82KB) — written by TUI state

TUI fallback chain checks `memory/` then root, never `learn/` (the largest).

## High

### `state/run-ledger.jsonl` — 5.8MB write-only, no reader
- 41,969 lines. Opens file fresh on EVERY call (no buffered writer).
- 8+ call sites in `event_loop.rs`. No truncation at run start.
- Not consumed by TUI, HTTP API, or orchestrator. Not in GcEngine scan scope.

### `GcEngine` never invoked at runtime
- `roko-fs/gc.rs:97-298`: Fully functional GC. Never instantiated outside manual `roko knowledge gc`.
- No awareness of `events.jsonl`, `run-ledger.jsonl`, `section-outcomes.jsonl`.
- `size_threshold_mb` default is 500MB — .roko/ would need 500MB before even recommending cleanup.

## Medium

### `learn/section-outcomes.jsonl` — 703KB growing, never read at runtime
- Written after every gate passage. `SectionOutcomeStore.read_all()` exists but never called from runtime.

### `knowledge-confirmations.jsonl` — 649KB, potentially unread
- Written by ingestion pipeline. No confirmed runtime consumer.

### `append_ledger_entry` — open/write/close per call
- `event_loop.rs:6418-6428`: OS file descriptor open, write, `sync_data()`, close per record. Hundreds of syscalls during heavy runs.
