# 212 — Stamp run_id into Snapshots and All Persisted Events

> **Status: SOURCE-DONE FOR THE AUTHORITATIVE RUNNER-V2 PATH AND OFFLINE REPAIR** (2026-08-31;
> PR #73, expanded by `5f689d66e` + `85c052fc9`). The implementation chose a stronger direct field
> contract: every `RunnerEvent`
> variant owns `run_id` and exposes `RunnerEvent::run_id()`, while `RunStateSnapshot` owns the same
> identity. New records are additionally projected into hashed per-run indexes. The original plan
> below to add a second JSON wrapper and mutate the legacy `ExecutorSnapshot` was superseded; it
> would duplicate identity and force an unnecessary file-format migration.

**Priority**: P2 — events from multiple runs are indistinguishable and snapshots cannot be correlated with events without a run_id
**Size**: XS (2-4 hours)
**Crates**: `roko-cli`
**Depends on**: None
**Sources**: tmp/backlog/_mori-diffs-gaps.md Groups E-2 and G-1; 23-HANDOFF-OPEN-ITEMS.md section 04 and 06; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-06

## Background

`run_id` is generated at runner startup and is already present in `RunStateSnapshot` (the runner-v2 snapshot). However, the legacy `ExecutorSnapshot` does not carry a `run_id`, and not every `RunnerEvent` variant is stamped with `run_id` before being persisted to `.roko/events.jsonl`. This makes it impossible to:

1. Correlate a legacy executor snapshot with the events that produced it
2. Query events by run from the snapshot alone
3. Filter `.roko/events.jsonl` to show only events from a specific run when multiple runs have occurred in the same workspace

This is a prerequisite for #215 (HTTP run-scoped event queries).

## Audited baseline (historical)

- `RunStateSnapshot` in `crates/roko-cli/src/runner/persist.rs` has a `run_id: String` field (line 100)
- `ExecutorSnapshot` in `crates/roko-cli/src/orchestrator/executor/snapshot.rs` does not have a `run_id` field
- `RunnerEvent` in `crates/roko-cli/src/runner/types.rs` has ~40 variants; some variants include `run_id` (e.g. `ResumeMarker`), but most do not
- Events are persisted in `crates/roko-cli/src/runner/persist.rs` and `crates/roko-cli/src/runner/structured_log.rs`
- The `run_id` is available in the runner context at persistence time but is not injected into every event

## Implementation Plan

1. **Add `run_id` to `ExecutorSnapshot`** in `crates/roko-cli/src/orchestrator/executor/snapshot.rs`:
   - Add `pub run_id: String` field with `#[serde(default)]` for backward compatibility
   - Populate it from the runtime context when writing the snapshot

2. **Add a top-level `run_id` wrapper** to RunnerEvent persistence:
   - In the structured log writer (`crates/roko-cli/src/runner/structured_log.rs`), wrap each persisted event in a `{ "run_id": "...", "event": { ... } }` envelope before writing to JSONL
   - This avoids modifying every RunnerEvent variant individually — the envelope is added at the persistence boundary

3. **Update event readers** to parse the envelope:
   - The structured log reader, TUI event loader, and any serve-side event parsers should extract `run_id` from the envelope
   - Backward compatibility: if the envelope is missing (pre-migration events), default `run_id` to `"unknown"`

4. **Add a migration note** — existing `.roko/events.jsonl` files will have events without the envelope; document that these are treated as `run_id: "unknown"`

## Acceptance Criteria

- [x] The authoritative runner snapshot (`RunStateSnapshot`) includes `run_id` populated from the
      runtime context. Adding it to the retired/legacy executor snapshot is superseded.
- [x] Every current `RunnerEvent` owns a direct top-level `run_id`; no redundant wrapper is needed.
- [x] Persistence and readers expose `RunnerEvent::run_id()` for filtering and per-run indexing.
- [x] Historical pre-index records are not relabeled as `unknown` or scanned on an HTTP request;
      bounded, dry-run-first `roko run-index repair` rebuilds them explicitly and atomically only
      after a complete scan.
- [ ] A final live run proves every emitted line and per-run index record has the same `run_id`;
      source construction is complete, but the coordinator's final fixture is pending.

## Verification Checklist

- [ ] `cargo build -p roko-cli` compiles on the final integrated tree — final batch pending.
- [ ] `cargo test -p roko-cli` passes, including existing snapshot and resume tests — final batch pending.
- [ ] Manual: run a plan, inspect `.roko/events.jsonl` and its per-run index — every line has the
      expected direct `run_id` field.
- [ ] Manual: run a second plan, inspect events — the two runs have different `run_id` values
- [ ] Manual: exercise offline repair dry-run/apply, truncation, malformed/cross-run records, and
      active-lock refusal against disposable historical fixtures.
- [x] No envelope migration is required because the implementation kept the direct event schema;
      pre-index historical repair is tracked separately.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/types.rs` | Direct `run_id` field on all `RunnerEvent` variants plus `RunnerEvent::run_id()` |
| `crates/roko-cli/src/runner/persist.rs` | `RunStateSnapshot.run_id` and per-run index projection |
| `crates/roko-cli/src/runner/structured_log.rs` | Persist the direct event schema without a redundant wrapper |
| `crates/roko-fs/src/run_index.rs` | Safe hashed per-run index paths for new records |
| `crates/roko-cli/src/commands/run_index.rs` | Bounded explicit repair of pre-index historical records |
