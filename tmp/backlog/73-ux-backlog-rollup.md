# 73 — UX Backlog Rollup (4 Actionable Themes from Archive Audit)

**Priority**: P3 — UX polish; no data loss, no correctness impact
**Size**: M (5-8 days total; each theme is individually S)
**Crates**: `crates/roko-cli/src/tui/`, `crates/roko-agent/src/`
**Depends on**: None

---

## Background

An audit of the archived `tmp/archive/08-17-26/ux/ux-followup/` directory (112 entries; 72
done, 40 open as of 2026-08-17) produced this rollup. Of the 40 open items, 10 are already
covered by dedicated backlog specs (see cross-reference table at the bottom), 17 are
infrastructure/backend/roadmap work excluded from this UX spec, and 13 genuine UX items
fall into the four themes below.

This document owns those 13 items. Items 41 (TUI Push-Mode), 35 (CLI Output Redesign), 38
(Provider Error UX), and 10 (Daimon TUI View) remain their own separate specs and are not
re-specified here.

## Current State

1. **Theme 1 (TUI Incremental Reads):** `DashboardData::tick()` in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs` (marked
   `#[deprecated]`) re-reads entire JSONL files on every tick for episodes, events, and
   learning data. The efficiency and c-factor files already use `IncrementalTailer` (added
   via `efficiency_tailer` and `cfactor_tailer` fields at lines ~431-433 of `dashboard.rs`),
   proving the pattern works. The remaining consumers (gate verdicts, task outputs, episodes,
   events log, and the four `.roko/learn/*` JSON files) still do full re-reads on every
   tick.

2. **Theme 2 (Generation Counter Durability):** `DurableDashboardGenerationCounter` already
   exists at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard_gen.rs` and persists
   to `.roko/state/dashboard-gen.json`. The counter is loaded at startup via `load()` and
   incremented atomically via `next()`. This theme is already implemented; the original item
   (12-78) is DONE.

3. **Theme 3 (Documentation Sweep):** The `bardo-backup/` directory at
   `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/` contains ~140 `.md` files
   that are read-only reference material for a previous architecture. No stale-snapshot banner
   exists on those files yet. Live docs outside `bardo-backup/` may still contain legacy
   terminology (`grimoire`, `styx`, `clade`, `mortal`, `death`, `reincarnation`).

4. **Theme 4 (Code Hygiene):** Seven crate-level `lib.rs` files suppress
   `clippy::missing_errors_doc` and/or `clippy::missing_panics_doc` with
   `#[allow(...)]` directives rather than adding the missing doc sections:
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs`
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs`
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/lib.rs`
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs`
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/lib.rs`
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrator/safety/audit_chain.rs`

   Additionally, some tests in `crates/roko-agent/src/` use hardcoded sub-500ms timeouts
   (search `with_timeout_ms` in that directory) which can produce flaky results on slow CI.

## Implementation Plan

### Theme 1: TUI Incremental File Reading

The `IncrementalTailer<T>` type in
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/jsonl_tailer.rs` provides offset-
tracked JSONL reading. The `JsonlCursor` underneath it stores the byte position after the
last successful read. On each `tick()` call it seeks to that position and reads only new
bytes.

**Step 1.** In `dashboard.rs`, add `IncrementalTailer<Episode>` alongside the existing
efficiency and cfactor tailers (near line 431). Import
`roko_learn::episode_logger::Episode`.

**Step 2.** Remove or bypass the full `read_to_string` call in `tick()` for
`EPISODES_FILE` and delegate to the new tailer's `tick()` instead. Merge new items into
`self.episodes` (or the field that holds the episode list).

**Step 3.** For gate verdicts: the `verdicts.rs` module under `tui/` or the cursor in
`tui/cursors.rs` reads `gate-verdicts.jsonl` on each change event. Check whether a
`JsonlCursor`-based reader already exists in `tui/verdicts.rs`; if not, add one that stores
the byte offset and re-reads only new bytes.

**Step 4.** For task outputs: `tui/task_outputs.rs` (`TaskOutputCursors`) tracks
per-task output files. Verify it is already using cursors (the type is named
`TaskOutputCursors`). If any path does a full re-walk, switch it to mtime-gated partial reads.

**Step 5.** For the four `.roko/learn/*.json` (non-JSONL) files (experiments, gate
thresholds, cascade router, provider health): these are small enough that a full re-read on
stamp change is acceptable. No incremental read needed; confirm mtime is checked before
re-reading and document this decision with a comment.

### Theme 2: Generation Counter Durability

This theme is already done. `DurableDashboardGenerationCounter` in `dashboard_gen.rs`:
- Loads from `.roko/state/dashboard-gen.json` on startup
- Persists atomically on each `next()` call where the fingerprint changes
- Used in `dashboard.rs` via `DurableDashboardGenerationCounter::load(&root)`

No code changes required. Mark item 12-78 as closed in the archive index.

### Theme 3: Documentation Sweep

**Step 1.** Write a one-shot script (shell or Rust binary under `tmp/`) that prepends the
following banner to every `.md` file under `bardo-backup/tmp/roko-progress/`:

```
> **STALE SNAPSHOT (pre-roko):** This file describes the mori/bardo architecture from
> before the roko rewrite. Terminology, file paths, and subsystem names are outdated.
> See `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` for the current architecture.
```

**Step 2.** Run a ripgrep across all live docs (excluding `bardo-backup/` and `target/`):
```
rg -l 'grimoire|styx|clade|mortal|mortality|reincarnation' \
   --glob '*.md' \
   --glob '!bardo-backup/**' \
   --glob '!target/**' \
   /Users/will/dev/nunchi/roko/roko/
```
Replace each occurrence with its current term: `grimoire`→`neuro`, `styx`→`Korai`,
`clade`→`fleet`. Remove death/mortality references or replace with context-appropriate
language (e.g., "agent lifecycle" instead of "death").

**Step 3.** Update any stale status markers in `tmp/implementation-plans/00-INDEX.md` or
`plans/INDEX.md` that still show `pending` for items landed in PR #13 or later.

### Theme 4: Code Hygiene

**Step 1 (09-56): Remove suppressed clippy doc lints.** Pick one crate (suggest
`roko-learn/src/lib.rs` as smallest). Find all `pub fn` and `pub async fn` in that crate
that:
- Return `Result<_>` but have no `# Errors` section in their doc comment
- Can panic but have no `# Panics` section

Add the missing doc sections. Then remove the crate-level
`#[allow(clippy::missing_errors_doc)]` or `#[allow(clippy::missing_panics_doc)]` directive.
Verify `cargo clippy -p roko-learn --no-deps -- -D warnings` passes. Repeat for each
remaining crate incrementally.

**Step 2 (09-58): Fix flaky timeout tests.** Search for `with_timeout_ms` in
`crates/roko-agent/src/`:
```
rg 'with_timeout_ms' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/
```
For any test that uses a value under 500ms, replace it with either:
- `tokio::time::pause()` + `tokio::time::advance()` for deterministic async time control
- A CI-aware multiplier: `let ms = if std::env::var("CI").is_ok() { 2000 } else { 100 };`

## Acceptance Criteria

1. At least one additional JSONL consumer in `dashboard.rs` beyond the existing
   `efficiency_tailer` and `cfactor_tailer` uses offset-based incremental reading (e.g.,
   `IncrementalTailer<Episode>` for episodes).
2. Generation counter durability: `DurableDashboardGenerationCounter` is used in
   `dashboard.rs`; `.roko/state/dashboard-gen.json` exists after one `roko dashboard` run
   and survives a second restart with a non-zero generation. (Verify this is already done.)
3. Every `.md` file in `bardo-backup/tmp/roko-progress/` contains the stale-snapshot
   banner.
4. Zero occurrences of `grimoire`, `styx`, `clade` in live docs outside `bardo-backup/`
   (excluding `target/`).
5. At least one crate's `#[allow(clippy::missing_errors_doc)]` is removed after adding
   the missing `# Errors` doc sections to its public functions.
6. No test in `crates/roko-agent/src/` uses a hardcoded timeout under 500ms without a
   `tokio::time::pause()` guard or a CI-aware multiplier.
7. `cargo test --workspace` passes.
8. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] `grep -r 'IncrementalTailer' crates/roko-cli/src/tui/dashboard.rs` shows at least
  3 tailer uses (efficiency, cfactor, and at least one new one)
- [ ] Start `roko dashboard`, let it run, kill it, restart it; verify generation in
  `.roko/state/dashboard-gen.json` is non-zero and increments on the second run
- [ ] `ls bardo-backup/tmp/roko-progress/*.md | head -5 | xargs head -5` shows the
  stale-snapshot banner on each file
- [ ] `rg 'grimoire|styx|clade' --glob '*.md' --glob '!bardo-backup/**' crates/ docs/`
  returns no results
- [ ] `cargo clippy -p roko-learn --no-deps -- -D warnings` passes without suppression

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs` | Add `IncrementalTailer<Episode>` field and replace the full episode file re-read in `tick()` with incremental tailer |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/verdicts.rs` (if full re-read) | Switch gate verdict reader to `JsonlCursor`-based offset tracking |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs` (and others) | Add `# Errors` / `# Panics` doc sections to public fns; remove `#[allow(clippy::missing_errors_doc)]` |
| `bardo-backup/tmp/roko-progress/*.md` (~140 files) | Prepend stale-snapshot banner (batch script) |
| Live `.md` files containing legacy terminology | Replace `grimoire`→`neuro`, `styx`→`Korai`, `clade`→`fleet` |
| Tests in `crates/roko-agent/src/` using sub-500ms timeouts | Replace hardcoded timeouts with `tokio::time::pause()` or CI multiplier |

---

## Cross-Reference: Items Covered by Other Specs

| Archive ref | Existing spec | Theme |
|---|---|---|
| 12-70, 12-71, 12-72, 12-73, 12-74, 12-76, 14-87 | Backlog 41 (TUI Push-Mode Panel Data) | Live TUI push pipeline |
| Ad-hoc CLI output formatting | Backlog 35 (CLI Output Redesign) | CliReporter trait, spinners, colors |
| Raw provider errors | Backlog 38 (Provider Error UX) | Actionable error messages |
| DaimonState visibility | Backlog 10 (Daimon TUI View) | PAD gauges in TUI |

## Not in Scope

- TUI push-mode data pipeline (backlog 41)
- CLI output restructuring (backlog 35)
- Provider error messages (backlog 38)
- Daimon/affect TUI view (backlog 10)
- Phase-2 vision items (chain, dreams, full TUI editor, multi-tenant HTTP)
- Backend protocol parity (Codex/Cursor/Gemini conformance tests)
- Gate rung wiring correctness (tracked in GAPS.md)
