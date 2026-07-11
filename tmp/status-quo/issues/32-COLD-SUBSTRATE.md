# Cold Substrate and Archival Issues

## Critical

### Copy-not-move in ALL archival paths
- `roko-serve/lib.rs:2185`: `run_cold_archival_tick` — no hot-store deletion after `archive_batch`.
- `orchestrate.rs:3904`: `post_plan_cold_archival` — same.
- `commands/knowledge.rs:237-243`: `cmd_archive` — comment acknowledges skip: "hot-side cleanup happens via normal prune path on next dream cycle." That dream cycle is never triggered.
- `FileSubstrate.prune()` and `compact()` exist but are never called after archival.

### No deduplication in `archive_batch`
- `cold_substrate.rs:218-242`: No `contains` check before appending. Re-runs append duplicates.
- Index entry updated to new offset — old bytes orphaned, unreachable, never freed.

### Unbounded hourly re-append
- Default `interval_secs = 21600` (6 hours). Each tick:
  1. Queries same aged engrams (never deleted from hot).
  2. Appends each again to cold archive.
- After N ticks: N × batch_size duplicate lines. ~2000 new lines/day indefinitely.

## High

### `purge_before` is index-only — JSONL bytes never reclaimed
- `cold_substrate.rs:269-294`: Removes hash entries from index, rewrites `index.json`.
- Does NOT touch underlying `.roko/cold/YYYY-MM.jsonl` files. Purged bytes accumulate forever.
- No compaction path exists for cold JSONL files.

## Medium

### `engrams.jsonl` retention rotates independently from cold archival
- `retention.rs:124-130`: `CompactionStrategy::Rotate` renames to `.old` and truncates.
- Uncoordinated with cold archival. If rotate fires first, aged engrams are lost without being archived.

### `read_from_archive` uses O(n) line-scanning seek
- `cold_substrate.rs:145-158`: Stored offset is byte offset but read scans by lines. Degrades as archives grow.
