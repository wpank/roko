# E02 — Storage Convergence

> Epic backlog · authored 2026-07-09 @ HEAD 5852c93c05
> Source docs: [55-DATA-DIR](../../55-DATA-DIR.md), [60-STATE-PERSISTENCE-LEDGER](../../60-STATE-PERSISTENCE-LEDGER.md), [97-TRACE-SERVE-LIFECYCLE](../../97-TRACE-SERVE-LIFECYCLE.md), [32-EVENTS-BUS-STATEHUB](../../32-EVENTS-BUS-STATEHUB.md)
> Native task schema: `crates/roko-cli/src/task_parser.rs::TaskDef` · exemplar: `plans/P24-workspace-paths/tasks.toml`

## Goal

`.roko/` is written by ≥6 subsystems with **three competing path authorities**
(`roko_fs::RokoLayout`, `roko_core::Workspace`, `roko-cli/workspace_paths.rs`). The result is
split-brain stores, never-written-but-read files, and 86 MB of unrotated logs. The flagship
symptom: **gate verdicts land in one file while every dashboard reads another → empty panels.**

This epic makes each durable concern have exactly one canonical writer that the readers actually
read. It is ordered by blast-radius × read-fanout: **unify the signal store first** — it is the
single highest-impact fix because it makes the dashboards non-empty.

## Findings → tasks map

| # | Finding | Evidence (verified this pass) | Task |
|---|---|---|---|
| a | **Split-brain signal store.** Runner v2 appends flat `{"kind":"GateVerdict",…}` rows to `signals.jsonl`, but 6 serve readers + dashboards read `engrams.jsonl` (full `Engram` records). Panels read the file the verdicts never reach. | write: `runner/event_loop.rs:1147-1168` (`config.layout.signals_path()`); paths: `layout.rs:204 engrams_path` vs `:219 signals_path` (both point at real files, opposite schemas); readers: `status/{episodes,metrics,gates}.rs`, `research.rs:509`, `dashboard_snapshot.rs`, `commands/show.rs:442` | **E02-T01** |
| a' | **Init migration schema-mix risk.** `roko init` renames `signals.jsonl`→`engrams.jsonl` when the latter is absent — mixing GateVerdict rows into the Engram store. | `commands/util.rs:135-150` | **E02-T02** |
| c | **`executor.json` never-written-but-read×4.** `save_executor_snapshot` has zero runtime caller; Runner v2 writes only `state-snapshot.json`. Serve workspace route emits `"executor.json: <error>"` for every real workspace. | reader `routes/workspaces.rs:322-331`; also `dashboard_snapshot.rs:1274,2790`, `projection_contract.rs:1599`, resume `main.rs:2659`; helper `layout.rs:382`; writer `state-snapshot.json` = `runner/snapshot_writer.rs`←`event_loop.rs:3341` | **E02-T03** |
| f | **`gate-thresholds.json` never materializes.** Runner v2 folds thresholds into `state-snapshot.json.gate_thresholds_json`; only the retired `orchestrate.rs:5953` writes the standalone file → 4+ readers see nothing. | readers: serve `learning/mod.rs:120,747`, `dashboard_snapshot.rs:1294`, tui `dashboard.rs:52`, acp `runner.rs:1873` | **E02-T04** |
| d | **Episodes triplicated** (root 27 / `learn/` 16 / `memory/` 8 frozen). Serve declares `memory/` canonical (wrong); `layout.rs:323-326` mislabels memory as "main". | serve `lib.rs:1022,1099`; `layout.rs:323-326`; writers `feedback_service.rs:141` + orchestrate | **E02-T05** |
| e | **Daimon split.** `daimon/affect.json` (CLI/neuro) vs `state/daimon.json` (orchestrator) — disjoint reader sets. | `roko-daimon/lib.rs:2365`, neuro `context.rs:244` vs `service_factory.rs:236` | **E02-T06** |
| g | **~86 MB unrotated logs.** `retention.rs:115-174` covers 8 artifacts; the 4 biggest (`events.jsonl`, `roko.log`, `chain-watcher.log`, `state/run-ledger.jsonl`) + `state/*.bak.*` are uncovered. | `retention.rs:115 default_retention_policies`; scheduled hourly `serve/lib.rs:2149` | **E02-T07** |
| b | **`events.jsonl` 44 MB, 97.3 % `feed_tick`, write-only.** `FeedTick` is a no-op in `DashboardSnapshot::apply`; 29 feed agents heartbeat into the orchestration event log. | 152 965/157 264 rows `feed_tick`; producers `feed_agents/*` publish `ServerEvent::FeedTick` (`mod.rs:108`←`lib.rs:415`) | **E02-T08** |
| — | **`runtime-events.jsonl` reader-without-file.** 2 serve routes read a file `JsonlLogger` never instantiates. | readers `routes/runs.rs:20`, `shared_runs.rs:326`; writer class `roko-runtime/jsonl_logger.rs:33` (uninstantiated) | **E02-T09** |
| — | **Dead second-implementations.** `state/run-state.json` (test-only writer), `state/events.json`, `roko_runtime::RunLedger` ("not wired"). | `persist.rs:309,49`; `run_ledger.rs:1-5` | **E02-T10** |
| — | **No layout version migration.** `VERSION`=1, only `LayoutVersion::V1` exists; nothing consumes it. Land `V2` + `roko doctor` state listing once T01-T05 stabilize. | `layout.rs:30-57,476-505` | **E02-T11** |
| h | **Cold-substrate archival copies, never moves → unbounded growth.** The hourly serve timer archives aged engrams to `.roko/cold/` but never prunes them from the hot store, so every tick re-queries the same aged rows and re-appends them to the cold archive. Runtime-live (not legacy-only). | trigger `serve/lib.rs:344,800` → `run_cold_archival_tick` (`serve/lib.rs:2166-2187`) calls `cold.archive_batch(candidates)` but never deletes the candidates from the hot `FileSubstrate`; `archive_batch` (`roko-fs/cold_substrate.rs:218-242`) appends via `append_to_archive` with no dedup (index overwrites per-hash, file grows unbounded) | **E02-T12** |

## Reconciliation with P24-workspace-paths

P24 is **partial and adjacent** — it fixes the *plans-directory* path split and adds two doctor
cruft-warnings, but touches none of the storage split-brains above.

| P24 task | Scope | Overlap with E02 |
|---|---|---|
| P24-T1/T2 | Align `resolve_plans_dir`/doc strings to prefer top-level `plans/` | **None** — plans dir, not `.roko/` state stores. Leave in P24. |
| P24-T3 | `roko doctor` warns on orphaned `learn/*.tmp.*` | Complements E02 cruft cleanup; keep in P24. **E02-T11** extends `roko doctor` with a full canonical/legacy state listing — build on P24-T3's pattern (`doctor.rs` `DoctorCheck`). |
| P24-T4 | `roko doctor` warns when both `plans/` and `.roko/plans/` exist | **None** — keep in P24. |

**Decision:** E02 does not re-implement P24. E02-T11's `roko doctor` state audit should reuse the
`DoctorCheck`/`DoctorStatus` scaffolding P24-T3/T4 add to `crates/roko-cli/src/doctor.rs`.
Sequence P24 (plans-dir) and E02 (state stores) independently; only E02-T11 has a soft dependency
on P24-T3 landing first (shared doctor-check pattern).

## Cross-epic dependencies

- **E03 · Type Consolidation (GateVerdict shape) — REQUIRED for E02-T01.** The convergence writes
  gate verdicts as `Engram` records. Today the verdict is an ad-hoc inline `serde_json::json!`
  blob (`event_loop.rs:1150`) plus a separate `GateVerdictSummary` struct (`event_loop.rs:66`).
  E03 must fix the canonical verdict type / `Engram` provenance mapping before T01 can encode
  verdicts as Engrams without inventing a schema. **Sequence: E03 GateVerdict decision → E02-T01.**
  If E03 slips, T01 can ship the safer interim (dedicated typed `.roko/gate-verdicts.jsonl` +
  repoint readers) without blocking on the Engram-shape decision.
- **E01 · (workspace/path facade, if present)** — E02-T03/T05 repoint readers via `RokoLayout`
  helpers; if E01 introduces a `WorkspacePaths` facade, T03/T05 should route through it rather than
  add new `layout.rs` helpers. Soft dependency; not blocking.

## Task breakdown

Ordered by impact. Tiers: `mechanical` < `focused` < `integrative` < `architectural`.

| Task | Title | Tier | Files | Depends on | Acceptance |
|---|---|---|---|---|---|
| **E02-T01** | Unify signal store: Runner v2 writes gate verdicts to the canonical log, stops writing `signals.jsonl` | integrative | `runner/event_loop.rs`, `roko-fs/src/layout.rs` | E03 GateVerdict shape (soft) | Verdict append + dashboard readers resolve to the SAME path; `signals.jsonl` no longer grows on a plan run |
| **E02-T02** | Guard init migration against schema-mixing (only rename rows that parse as `Engram`) | focused | `commands/util.rs` | — | `roko init` on a GateVerdict-only fixture does not fold verdicts into `engrams.jsonl` |
| **E02-T03** | Repoint `executor.json` readers at `state-snapshot.json` (`.executor_json`); drop `executor_snapshot()` helper | integrative | `routes/workspaces.rs`, `dashboard_snapshot.rs`, `projection_contract.rs`, `main.rs`, `roko-fs/src/layout.rs` | — | Serve workspace route no longer reports `executor.json` in `errors[]`; no reader references `executor.json` |
| **E02-T04** | Materialize or repoint `gate-thresholds.json` (emit standalone in `save_snapshot`, or read `state-snapshot.json.gate_thresholds_json`) | focused | `runner/event_loop.rs` or `serve/learning/mod.rs` + 3 readers | E02-T03 (snapshot-read helper) | Thresholds readers return non-empty after a plan run |
| **E02-T05** | Collapse episodes to root; repoint serve projection + `layout.rs` label off `memory/`; migrate `memory/episodes.jsonl` | focused | `roko-serve/src/lib.rs`, `roko-fs/src/layout.rs`, `feedback_service.rs` | — | Serve projection + writers all name `.roko/episodes.jsonl`; only root file grows after a run |
| **E02-T06** | Consolidate daimon to `daimon/affect.json`; alias orchestrator `state/daimon.json` | focused | `roko-orchestrator/src/service_factory.rs` | — | One daimon file mutates per run |
| **E02-T07** | Add retention for `events.jsonl`, `roko.log`, `chain-watcher.log`, `run-ledger.jsonl`; cap `state/*.bak.*` | focused | `roko-serve/src/retention.rs` | — | `default_retention_policies()` enumerates all 4 + bak cap; soak stays bounded |
| **E02-T08** | Stop `feed_tick` heartbeats polluting `events.jsonl` (separate sink or drop; it's a no-op in `apply`) | integrative | `roko-serve/src/feed_agents/mod.rs`, `state.rs` | — | `events.jsonl` `feed_tick` share drops to ~0; orchestration events preserved |
| **E02-T09** | Resolve `runtime-events.jsonl` reader-without-file (wire `JsonlLogger` at serve startup, or drop 2 routes) | focused | `roko-serve/src/lib.rs`, `routes/runs.rs`, `shared_runs.rs` | — | `/api/runs` returns data OR the routes are removed |
| **E02-T10** | Retire dead second-impls: `state/run-state.json`, `state/events.json`, `roko_runtime::RunLedger` | mechanical | `runner/persist.rs`, `roko-runtime/src/run_ledger.rs` | E02-T03 | `rg 'run-state.json\|events_json\|RunLedger'` returns only live paths |
| **E02-T11** | Introduce `LayoutVersion::V2` + real `.roko/VERSION` migration + `roko doctor` state audit (canonical/legacy/orphan listing) | integrative | `roko-fs/src/layout.rs`, `roko-cli/src/doctor.rs` | E02-T01,T03,T05; P24-T3 (doctor pattern) | `cat .roko/VERSION` → `2`; `roko doctor` lists every `.roko` state file with a verdict |
| **E02-T12** | Make cold-substrate archival move-not-copy: prune archived engrams from the hot store after `archive_batch`, and dedup the cold append so re-runs don't grow the archive | focused | `roko-serve/src/lib.rs`, `roko-fs/src/cold_substrate.rs`, `roko-fs/src/file_substrate.rs` | — | After a tick the archived engrams are gone from the hot store; a second identical tick archives 0 and the cold file byte-size is unchanged |

**Task count: 12.** First wave (T01–T03) below as executable TOML; T04–T12 follow the same schema (T12 authored in full below).

## First wave — executable tasks

```toml
[meta]
plan = "E02-STORAGE-CONVERGENCE"
total = 3
done = 0
status = "ready"
max_parallel = 1

# ────────────────────────────────────────────────────────────────────
# E02-T01: Unify the signal store (HIGHEST IMPACT — fixes empty dashboards)
# ────────────────────────────────────────────────────────────────────
# Runner v2 appends flat {"kind":"GateVerdict",...} rows to signals.jsonl
# (event_loop.rs:1147-1168 via layout.signals_path()), while every serve
# reader + dashboard reads engrams.jsonl. The verdicts never reach the
# panels. Make the runner write verdicts to the SAME store the dashboards
# read, and stop touching signals.jsonl.
#
# CROSS-EPIC: depends (soft) on E03's GateVerdict-shape decision. If E03
# has landed the Engram provenance mapping, encode verdicts as Engram
# records to layout.engrams_path(). If E03 has NOT landed, ship the safe
# interim: a dedicated typed .roko/gate-verdicts.jsonl written via a new
# layout.gate_verdicts_path() helper AND repoint the readers at it — still
# one writer, one reader path, no schema-mix.

[[task]]
id = "E02-T01"
title = "Runner v2 writes gate verdicts to the canonical log; stop writing signals.jsonl"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-5"
max_loc = 90
files = ["crates/roko-cli/src/runner/event_loop.rs", "crates/roko-fs/src/layout.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-cli/src/runner/event_loop.rs", lines = "1140-1175", why = "The gate-verdict append: writes flat JSON to layout.signals_path() — the split-brain source" },
    { path = "crates/roko-cli/src/runner/event_loop.rs", lines = "5040-5060", why = "GateVerdictSummary — the typed verdict already flowing here; source for the Engram/typed row" },
    { path = "crates/roko-fs/src/layout.rs", lines = "200-221", why = "engrams_path (canonical), engrams_path_legacy + signals_path (both -> signals.jsonl). Add gate_verdicts_path() here if using interim path" },
    { path = "crates/roko-fs/src/file_substrate.rs", lines = "40-95", why = "How Engram records are appended/compacted to engrams.jsonl — mirror for verdict-as-Engram" },
    { path = "crates/roko-serve/src/routes/status/gates.rs", lines = "80-95", why = "A dashboard reader — must resolve to the SAME path the writer now uses" },
]
symbols = [
    "signals_path — fn signals_path(&self) -> PathBuf (roko-fs/src/layout.rs:219, returns signals.jsonl)",
    "engrams_path — fn engrams_path(&self) -> PathBuf (roko-fs/src/layout.rs:204)",
    "GateVerdictSummary — struct in runner/event_loop.rs (the typed verdict payload)",
]
anti_patterns = [
    "Do NOT keep writing signals.jsonl 'as well' — the whole point is one store. Remove the layout.signals_path() append.",
    "Do NOT invent an Engram schema inline. If E03's GateVerdict->Engram mapping is unavailable, use the typed .roko/gate-verdicts.jsonl interim and repoint readers — do not fold flat rows into engrams.jsonl.",
    "Do NOT touch commands/util.rs init-migration here — that is E02-T02.",
    "Do NOT delete signals_path()/engrams_path_legacy() from layout.rs yet — the init migration (E02-T02) still needs them as read/rename inputs.",
]

# verify: the verdict WRITE and the dashboard READ must hit the same file,
# and signals.jsonl must no longer be the verdict sink.
[[task.verify]]
phase = "structural"
command = "! grep -n 'signals_path()' crates/roko-cli/src/runner/event_loop.rs"
fail_msg = "Runner v2 must no longer append gate verdicts via layout.signals_path()"

[[task.verify]]
phase = "structural"
command = "grep -nE 'engrams_path\\(\\)|gate_verdicts_path\\(\\)' crates/roko-cli/src/runner/event_loop.rs"
fail_msg = "Gate verdicts must be written to engrams_path() (or the typed gate_verdicts_path()) — the store dashboards read"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli -p roko-fs 2>&1"
fail_msg = "Must compile after repointing the verdict writer"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli runner:: 2>&1"
fail_msg = "Runner event-loop tests must pass with the new verdict sink"

# ────────────────────────────────────────────────────────────────────
# E02-T02: Guard init migration against schema-mixing
# ────────────────────────────────────────────────────────────────────
# roko init renames signals.jsonl -> engrams.jsonl wholesale when engrams
# is absent (util.rs:135-150). If signals.jsonl holds GateVerdict rows,
# that folds a non-Engram schema into the Engram store. Only migrate rows
# that parse as Engram; leave (or quarantine) the rest.

[[task]]
id = "E02-T02"
title = "Guard signals->engrams init migration to only move rows that parse as Engram"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-5"
max_loc = 55
files = ["crates/roko-cli/src/commands/util.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-cli/src/commands/util.rs", lines = "130-155", why = "The signals.jsonl -> engrams.jsonl rename — must become a parse-filtered copy" },
    { path = "crates/roko-core/src/engram.rs", lines = "55-75", why = "Engram struct — deserialize target used to test each row" },
    { path = "crates/roko-fs/src/layout.rs", lines = "200-221", why = "engrams_path / signals_path source and target of the migration" },
]
symbols = [
    "Engram — struct (roko-core/src/engram.rs:63) with serde derive — parse test per line",
]
anti_patterns = [
    "Do NOT delete signals.jsonl rows that fail to parse — leave the file in place (or move to signals.jsonl.legacy) so no data is lost.",
    "Do NOT rename wholesale anymore — read line-by-line, only append rows that deserialize as Engram to engrams.jsonl.",
    "Do NOT change the runner writer here — that is E02-T01.",
]

[[task.verify]]
phase = "structural"
command = "grep -nE 'serde_json::from_str::<Engram>|from_str.*Engram' crates/roko-cli/src/commands/util.rs"
fail_msg = "Migration must parse-test each row as Engram before moving it"

[[task.verify]]
phase = "structural"
command = "! grep -nE 'fs::rename\\([^)]*signals' crates/roko-cli/src/commands/util.rs"
fail_msg = "Must not wholesale-rename signals.jsonl into engrams.jsonl anymore"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "Must compile after guarding the migration"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli init 2>&1"
fail_msg = "Init/migration tests must pass; add a GateVerdict-only fixture case"

# ────────────────────────────────────────────────────────────────────
# E02-T03: Fix executor.json reader drift
# ────────────────────────────────────────────────────────────────────
# save_executor_snapshot has ZERO runtime caller; Runner v2 writes only
# state/state-snapshot.json (StateSnapshot.executor_json). But serve's
# workspace route reads state/executor.json (workspaces.rs:323) and emits
# "executor.json: <error>" for every real workspace. Repoint the readers
# at state-snapshot.json (extract .executor_json) and remove the dead
# executor_snapshot() helper.

[[task]]
id = "E02-T03"
title = "Repoint executor.json readers at state-snapshot.json; drop executor_snapshot() helper"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-5"
max_loc = 80
files = [
    "crates/roko-serve/src/routes/workspaces.rs",
    "crates/roko-serve/src/dashboard_snapshot.rs",
    "crates/roko-fs/src/layout.rs",
]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/workspaces.rs", lines = "318-335", why = "Reads state/executor.json and pushes 'executor.json: <error>' — repoint at state-snapshot.json.executor_json" },
    { path = "crates/roko-cli/src/runner/persist.rs", lines = "40-90", why = "state_snapshot_json path + StateSnapshot with executor_json field — the real source" },
    { path = "crates/roko-serve/src/dashboard_snapshot.rs", lines = "1270-1300", why = "StateHub bootstrap also reads executor snapshot — same repoint" },
    { path = "crates/roko-fs/src/layout.rs", lines = "375-385", why = "executor_snapshot() helper (:382) — remove once no reader references executor.json" },
]
symbols = [
    "StateSnapshot — struct with executor_json/orchestrator_json/run_state_json/gate_thresholds_json fields (runner/persist.rs)",
    "executor_snapshot — fn (roko-fs/src/layout.rs:~382) returning state/executor.json — to be removed",
]
anti_patterns = [
    "Do NOT make the snapshot writer ALSO emit executor.json — repoint readers instead (one file, not two).",
    "Do NOT break the resume default path (main.rs:2659) — repoint it at state-snapshot.json too, do not leave it pointing at executor.json.",
    "Do NOT remove executor_snapshot() until grep confirms no remaining executor.json reader.",
]

[[task.verify]]
phase = "structural"
command = "! grep -rn 'executor.json' crates/roko-serve/src crates/roko-fs/src"
fail_msg = "No serve/fs code should reference executor.json after repointing"

[[task.verify]]
phase = "structural"
command = "grep -nE 'state-snapshot|state_snapshot|executor_json' crates/roko-serve/src/routes/workspaces.rs"
fail_msg = "Workspace route must read state-snapshot.json.executor_json"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve -p roko-fs 2>&1"
fail_msg = "Must compile after repointing readers and removing executor_snapshot()"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-serve workspaces 2>&1"
fail_msg = "Workspace-detail tests must pass; no executor.json error for a real workspace"
```

## Authored task — E02-T12 (cold-substrate move-not-copy)

```toml
[meta]
plan = "E02-STORAGE-CONVERGENCE-T12"
total = 1
done = 0
status = "ready"
max_parallel = 1

# ────────────────────────────────────────────────────────────────────
# E02-T12: Make cold-substrate archival MOVE, not COPY
# ────────────────────────────────────────────────────────────────────
# The hourly cold-archival timer (serve/lib.rs:344,800 -> start_cold_archival_timer
# -> run_cold_archival_tick, serve/lib.rs:2166-2187) queries aged engrams from the
# hot FileSubstrate and calls cold.archive_batch(candidates) — but it NEVER removes
# them from the hot store. So the hot store keeps every aged engram, and each hourly
# tick re-queries the SAME rows and re-appends them to .roko/cold/. archive_batch
# (roko-fs/cold_substrate.rs:218-242) appends via append_to_archive with no dedup
# (the in-memory index overwrites per-hash, but the archive JSONL file only grows).
# Net: runtime-live, hourly, unbounded cold-file growth. This is a MOVE that was
# implemented as a COPY.
#
# Fix: after a successful archive_batch, prune the archived ids out of the hot
# FileSubstrate (add a delete-by-ids / retain path — FileSubstrate has compact()
# (file_substrate.rs:88) + prune(threshold) (file_substrate.rs:317) but no
# delete-by-id) and compact() so the hot log shrinks. Guard archive_batch against
# re-archiving an id already in the cold index (dedup the append) as a belt-and-braces
# so a crash between archive and prune cannot grow the cold file on the next tick.

[[task]]
id = "E02-T12"
title = "Cold-substrate archival moves engrams out of the hot store (prune-after-archive + dedup append)"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-5"
max_loc = 90
files = [
    "crates/roko-serve/src/lib.rs",
    "crates/roko-fs/src/cold_substrate.rs",
    "crates/roko-fs/src/file_substrate.rs",
]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-serve/src/lib.rs", lines = "2162-2187", why = "run_cold_archival_tick: archives candidates but never deletes them from the hot substrate — the copy-not-move bug" },
    { path = "crates/roko-serve/src/lib.rs", lines = "2084-2160", why = "start_cold_archival_timer: the hourly loop (called at :344 and :800) that keeps re-archiving the same rows" },
    { path = "crates/roko-fs/src/cold_substrate.rs", lines = "218-242", why = "archive_batch: append_to_archive with no dedup against the cold index — the unbounded-append source" },
    { path = "crates/roko-fs/src/file_substrate.rs", lines = "88-119", why = "compact(): rewrites the hot log from the in-memory index — use after removing archived ids to reclaim disk" },
    { path = "crates/roko-fs/src/file_substrate.rs", lines = "317-325", why = "prune(threshold): removes from the in-memory index only — model a delete-by-ids path on this" },
]
symbols = [
    "run_cold_archival_tick — fn (roko-serve/src/lib.rs:2166): must prune archived ids from the hot store after archive_batch",
    "ArchiveColdSubstrate::archive_batch — fn (roko-fs/src/cold_substrate.rs:218): dedup against self.index before append_to_archive",
    "FileSubstrate::compact — fn (roko-fs/src/file_substrate.rs:88): shrink the hot log after removal",
]
anti_patterns = [
    "Do NOT leave the archived engrams in the hot store 'for safety' — that is the copy-not-move bug; the whole point is to MOVE them.",
    "Do NOT delete from the hot store BEFORE the archive_batch succeeds — archive first, then prune, so a failure cannot lose data.",
    "Do NOT skip the dedup guard in archive_batch — without it a crash between archive and prune re-grows the cold file on the next tick.",
    "Do NOT change the query cutoff / max_age / batch_size semantics here — only make the archived rows actually leave the hot store.",
]

# verify: archived rows must LEAVE the hot store, and a repeat tick must be a no-op
# on the cold file (no unbounded re-append).
[[task.verify]]
phase = "structural"
command = "grep -nE 'compact\\(\\)|delete|prune|remove' crates/roko-serve/src/lib.rs | grep -i cold || grep -nE 'hot\\.(compact|delete|prune|remove)' crates/roko-serve/src/lib.rs"
fail_msg = "run_cold_archival_tick must remove archived engrams from the hot substrate after archiving"

[[task.verify]]
phase = "structural"
command = "grep -nE 'contains|index.*get|already.*archived|dedup' crates/roko-fs/src/cold_substrate.rs"
fail_msg = "archive_batch must dedup against the cold index so re-runs do not re-append"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve -p roko-fs 2>&1"
fail_msg = "Must compile after the prune-after-archive + dedup changes"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-fs cold 2>&1"
fail_msg = "Cold-substrate tests must pass; add a case asserting a second archive of the same batch archives 0 and the cold file size is unchanged, and that the hot store no longer contains the archived ids"
```

## Definition of done (epic)

- Gate verdicts and every dashboard/status reader resolve to **one** store path (T01).
- No never-written-but-read files remain (`executor.json` T03, `gate-thresholds.json` T04, `runtime-events.jsonl` T09).
- Episodes and daimon each have one canonical writer (T05, T06).
- `retention.rs` bounds the 4 biggest logs + bak files; `events.jsonl` stops accumulating feed noise (T07, T08).
- Dead second-implementations removed (T10); `.roko/VERSION` → `2` with a real migration and `roko doctor` state audit (T11).
- Cold-substrate archival **moves** engrams (prunes the hot store + dedups the cold append) so the hourly timer stops re-appending the same rows (T12).
- Verify per task via the grep/`cargo`/`curl` commands above; the load-bearing check is that **writer path == reader path** for verdicts, executor state, thresholds, and episodes.
