# State And Persistence Ledger
> DEEPER SECOND PASS · re-verified 2026-07-08 @ HEAD 5852c93c05 against live `du -sh`/`wc -l`/event-kind histograms and traced write-side call-sites (not just path helpers).

This file maps durable state paths and the migration decisions needed to make them coherent. The complete writer→reader matrix is now the lead section; the canonical-candidate map and migration plan follow.

**Verification note (2026-07-08):** on-disk truths reconfirmed. Writes go to `state/state-snapshot.json` (never `executor.json`); root `.roko/episodes.jsonl` (27 rows) is canonical with `learn/episodes.jsonl` (16 rows) and `memory/episodes.jsonl` (8 rows) as duplicates — note `layout.rs:323-326` still labels `memory/episodes.jsonl` "the main episodes log"; gate verdicts append to `signals.jsonl` (467 flat GateVerdict rows) while `engrams.jsonl` (10 Engram rows) is the real log; `learn/gate-thresholds.json` never materializes because Runner v2 folds thresholds into `state-snapshot.json` and only the retired `orchestrate.rs:5953` writes the standalone file.

---

## §A. Complete writer → reader matrix

Every load-bearing `.roko/` path, with **exact write-side call-site** (a path helper alone is *not* a writer — traced to the actual `write`/`append`/`atomic_write`), reader call-sites, on-disk schema, and split/orphan/never-written verdict. Live stats from `du`/`wc -l` on 2026-07-08.

| Path | Live size / rows | Writer(s) — file:line (actual write call) | Reader(s) — file:line | Format / schema | Verdict |
|---|---|---|---|---|---|
| `engrams.jsonl` | 12K · 10 | `roko-fs/file_substrate.rs:48` (create), `:72/:91` (append + `engrams.jsonl.tmp` compaction); serve `state.rs:1537-1568` (`SignalPersist`); `run.rs:1205`; `orchestrate.rs:7154,8515` | `status/episodes.rs:35`, `status/metrics.rs:119`, `status/gates.rs:92`, `research.rs:509`, dreams `cycle.rs:52` (`ENGRAMS_LOG_FILE`), `dashboard_snapshot.rs:1269,2891`, `commands/show.rs:442` | JSONL of full `Engram` records | ✅ canonical but **underused** — Runner v2 bypasses it |
| `signals.jsonl` | 80K · 467 | **only** `runner/event_loop.rs:1147-1168` (append via `layout.signals_path()`, `layout.rs:214/220`) | `agent_serve.rs:1388-1393` (archive copy), acp `session.rs:1474` (`.exists()` workspace-detect), `chat_inline.rs:3201` (count) | flat `{"kind":"GateVerdict","plan_id","task_id","rung","passed",…}` — **not** Engram | 🔴 **split-brain**: legacy filename revived with a non-Engram schema; `roko init` (`commands/util.rs:138`) renames it into `engrams.jsonl` → schema-mix risk |
| `episodes.jsonl` (root) | 68K · 27 | `EpisodeLogger` via `orchestrate.rs`; acp `bridge_events.rs:420,453,3069,5493`; `feedback_service.rs:141`; serve `routes/prds.rs:77` | `dashboard_snapshot.rs:2515-2520` (fallback root→learn→memory), `runtime_feedback.rs:3427-3454`, serve `workspaces.rs:337-343`, dreams `runner.rs:810` | JSONL `Episode` | 🟡 canonical; **triplicated** |
| `learn/episodes.jsonl` | · 16 | `roko-core/workspace.rs:264` helper (`learn_episodes_path`); learn-side loggers | `dashboard_snapshot.rs:2518` (2nd in fallback) | JSONL `Episode` | 🟡 duplicate #2 |
| `memory/episodes.jsonl` | · 8 | **none since May 2** (frozen) | `dashboard_snapshot.rs:2520` (3rd fallback), serve projection registry `lib.rs:1022,1099` (declares it **canonical** — wrong), research prompt `research.rs:385` | JSONL `Episode` | 🔴 duplicate #3; `layout.rs:326` mislabels it "main"; serve points here |
| `events.jsonl` | 43M · 157 264 | `persist.rs:281 append_runner_event` (via `event_loop.rs:2719`), `run.rs:1231`, StateHub `roko-core/state_hub.rs:67` + `roko-runtime/state_hub.rs:75`, serve `state.rs:841` | serve `workflows.rs:1391`, `status/gates.rs:86`, `projection_contract.rs:1601`, `commands/util.rs:966`, resume `runner/resume.rs` | JSONL tagged `{"type":…}`; **97.3 % `feed_tick`** (152 965), `chain_block` 3 291 | 🟡 alive; ❌ **no retention** — 2nd biggest file, mostly feed-agent heartbeats |
| `runtime-events.jsonl` | **absent** | writer class `roko-runtime/jsonl_logger.rs:33` (`from_roko_dir`), `workflow_engine.rs:1315` — **not instantiated in serve/CLI runtime** | serve `routes/runs.rs:20`, `shared_runs.rs:326`, `projection_contract` | JSONL `RuntimeEventEnvelope` | 🔴 **reader-without-file**: 2 serve routes read a file nothing writes |
| `state/state-snapshot.json` (+18 `.bak.*`) | 6M dir | `runner/snapshot_writer.rs` thread ← `save_snapshot` (`event_loop.rs:3341`, called :919,:995) → `paths.state_snapshot_json` (`persist.rs:73`) | resume `event_loop.rs:340`, `commands/plan.rs:284` | unified `StateSnapshot{executor_json, orchestrator_json, run_state_json, gate_thresholds_json}` | ✅ canonical snapshot; 🟡 18 `.bak.*` uncapped |
| `state/executor.json` | **absent** | `persist.rs:286 save_executor_snapshot` — **ZERO runtime caller** (only fn def + tests; `state_hub.rs:477/490` writes are tests) | serve `workspaces.rs:323` (→ `"executor.json: <error>"`), StateHub bootstrap `dashboard_snapshot.rs:1274,2790`, `projection_contract.rs:1599`, resume default `main.rs:2659,2662`, `do_cmd.rs:793`, `util.rs:199` | JSON `ExecutorSnapshot` | 🔴 **never-written-but-read** by ≥4 live consumers; `layout.rs:382` still exposes `executor_snapshot()` |
| `state/run-ledger.jsonl` | 6M · 41 894 | `runner/event_loop.rs:5781 append_ledger_entry` (:1094/:1365/:1626/:1804/:2139) + `persist_run_ledger` (:1926,:1938) | `roko replay`/status readers | flat `{"kind":"task_started"/…,"data":{…},"ts"}` | 🟡 alive; ❌ no retention; ⚠ **not** the `roko_runtime::RunLedger` (that type is "not wired", `run_ledger.rs:1-5`) |
| `state/run-state.json` | absent | `persist.rs:309 save_run_state` (test-only callers `resume.rs:337…492`) | resume validation | JSON `RunStateSnapshot` | 🔴 legacy; superseded by embedded `run_state_json` in snapshot |
| `state/daimon.json` | 16K | orchestrator `DaimonPolicy` (`service_factory.rs:236`) read+write | same | JSON PAD | 🟡 **daimon split** (orchestrator store) |
| `daimon/affect.json` | 12.7K | `roko-daimon/lib.rs:2365 persist()`; path via `config_helpers.rs:73`, `orchestrate.rs:304`, neuro `context.rs:244` | serve `state.rs:48`, `runtime_feedback.rs:80`, `commands/knowledge.rs:694` | JSON PAD | 🟡 **daimon split** (CLI/serve/neuro store) |
| `learn/gate-thresholds.json` | **absent** | Runner v2: folded into `state-snapshot.json` (`save_snapshot` → `StateSnapshot.gate_thresholds_json`). Standalone: **only** legacy `orchestrate.rs:5953 adaptive_thresholds.save` (`adaptive_threshold.rs:274`) | serve `learning/mod.rs:120,747`, `dashboard_snapshot.rs:1294`, tui `dashboard.rs:52`, acp `runner.rs:1873`, retention `retention.rs:160` (manual) | JSON per-rung EMA | 🔴 **never-written** on default path; 4+ readers see nothing |
| `learn/cascade-router.json` (+11 `.tmp.*`) | · | `orchestrate.rs`/router `atomic_write` | dispatch enrichment, serve learning | JSON | ✅ alive; ❌ 11 leaked `.tmp.<pid>.<n>` |
| `learn/efficiency.jsonl` | · 37 | `orchestrate.rs` efficiency events | `roko learn efficiency`, serve | JSONL | ✅ alive; ✅ retention-covered (`retention.rs:137`) |
| `neuro/knowledge.jsonl` | 156K · 89 | `roko-neuro/knowledge_store.rs:363-379` | `commands/learn.rs:487`, dispatch, dreams | JSONL `KnowledgeEntry` | ✅ alive, hot |
| `neuro/knowledge-confirmations.jsonl` | 636K · 1 734 | knowledge_store confirmations | knowledge query | JSONL | ✅ alive, hot |
| `cold/2026-05.jsonl` | · 8 | `ArchiveColdSubstrate` hourly timer `serve/lib.rs:2096-2147` | `roko knowledge archive/query` | JSONL monthly | ✅ alive (CLAUDE.md item 14 stale) |

*(For the full 42-entry inventory incl. dormant/orphaned layout dirs — `plans/`, `runs/`, `traces/`, `metrics/`, `config/`, `cache/`, `templates/` — see [55-DATA-DIR.md](55-DATA-DIR.md) "Full inventory table".)*

## §B. Split-brain / orphan / never-written subtable

Nine paths where writer and reader disagree. **3 never-written-but-read**, **2 split-brain (two writers, two schemas)**, **1 reader-without-file**, **3 duplicate stores**.

| # | Path | Class | Writer truth | Reader truth | Convergence action |
|---|---|---|---|---|---|
| 1 | `signals.jsonl` vs `engrams.jsonl` | **split-brain** | v2 runner writes GateVerdicts to `signals.jsonl` (`event_loop.rs:1159`); everything else writes `engrams.jsonl` | 3 signals readers + ~10 engrams readers | Make runner append verdicts as Engrams to `engrams_path()` (or a typed `gate-verdicts.jsonl`); stop writing `signals.jsonl` |
| 2 | `state/executor.json` | **never-written / read×4** | no runtime writer (`save_executor_snapshot` uncalled) | serve `workspaces.rs:323`, dashboard bootstrap, resume default | Repoint readers at `state-snapshot.json` (extract `.executor_json`); drop `executor_snapshot()` from `layout.rs:382` |
| 3 | `learn/gate-thresholds.json` | **never-written / read×4** | standalone writer only on retired `orchestrate.rs:5953`; v2 folds into snapshot | serve/tui/acp readers | Have Runner v2 also emit standalone file, OR repoint readers at `state-snapshot.json.gate_thresholds_json` |
| 4 | `runtime-events.jsonl` | **reader-without-file** | `JsonlLogger` never instantiated in runtime | serve `runs.rs:20`, `shared_runs.rs:326` | Wire `JsonlLogger::from_roko_dir` at serve startup, or drop the 2 routes |
| 5 | `state/run-state.json` | **never-written** (runtime) | `save_run_state` test-only | resume validation path | Delete legacy path; rely on embedded `run_state_json` |
| 6 | episodes ×3 | **duplicate store** | root writers active; learn secondary; memory frozen | fallback union, but serve declares `memory/` canonical (`lib.rs:1022`) | Pick root; repoint serve projection + `layout.rs:326` label |
| 7 | daimon (`daimon/affect.json` vs `state/daimon.json`) | **duplicate store** | CLI/neuro write affect.json; orchestrator writes state/daimon.json | disjoint reader sets | Pick one; alias the other |
| 8 | `memory/*` vs `learn/*` | **duplicate store** | learn/ hot; memory/ frozen May 2 | fallback-only | Migrate memory episodes → root; delete rest |
| 9 | `state/events.json` (`persist.rs:49`) vs `events.jsonl` | **duplicate concept** | `events_json` path defined, rarely written; `events.jsonl` hot | StateHub reads `events.jsonl` | Drop `events.json` snapshot path |

## §C. Log-rotation gap table

`retention::apply_retention` **runs** (scheduled hourly, serve `lib.rs:2149-2155`) but `default_retention_policies()` (`retention.rs:115-174`) only enumerates 8 artifacts. The four largest disk consumers are uncovered.

| Path | Live size | Retention policy? | Growth driver | Fix |
|---|---|---|---|---|
| `worktrees/` | **161 M** | ❌ none | stale `mega-parity-run-…` checkout w/ committed `node_modules` | Prune worktrees on run-end (`WorktreeManager`); gitignore node_modules |
| `events.jsonl` | 43 M | ❌ none | 97.3 % `feed_tick` heartbeats | Add `Rotate`/`TailKeep` policy; or route feed_tick to a separate sink |
| `chain-watcher.log` | 23 M | ❌ none | chain-watcher subprocess stdout redirect (`serve/lib.rs:440`) | Add size-capped rotation |
| `roko.log` | 12 M | ❌ none | always-on tracing file layer (`main.rs:2100`) | `tracing-appender` rolling file |
| `state/run-ledger.jsonl` | 6 M | ❌ none | per-task ledger appends | Add `TailKeep` policy |
| `state/*.bak.*` | ·18 files | ❌ none | snapshot writer backups | Cap to N most recent |
| `learn/*.tmp.*` | ·11 files | ❌ none | failed atomic-rename cleanup | Fix `atomic_write` temp sweep |
| `workspaces/` | ·64 dirs | ❌ none | demo PTY sessions (`terminal.rs:457`) | GC on session close |
| `acp.log` | 73 K | ❌ none | ACP bridge log | Add rotation |
| — covered — | | ✅ | | |
| `episodes.jsonl` | | ✅ TailKeep 10k (`retention.rs:118`) | | |
| `engrams.jsonl` | | ✅ Rotate (`:125`) | | |
| `learn/efficiency.jsonl` | | ✅ TailKeep 5k (`:137`) | | |
| `learn/c-factor.jsonl` | | ✅ TailKeep 1k (`:145`) | | |
| `task-outputs/*` | | ✅ Archive (`:169`) | | |

## §D. Ordered convergence checklist (which files to unify first)

Ordered by **blast radius × read-fanout**. Each item cites the write-side line to change and a CLI/curl verification.

- [ ] **[C0 · highest impact] Unify the signal store.** Runner v2 gate-verdict append at `runner/event_loop.rs:1147-1168` is the single point creating the split-brain. Write verdicts as `Engram` records to `engrams_path()` (or a typed `.roko/gate-verdicts.jsonl`) and stop touching `signals.jsonl`. Also guard `commands/util.rs:138` init-migration to only rename rows that parse as Engrams. — verify: `roko plan run plans/ && wc -l .roko/signals.jsonl` (must not grow); `roko init` on a GateVerdict-only fixture keeps schemas separate.
- [ ] **[C1] Fix `executor.json` reader drift** (read by serve workspace route + dashboard bootstrap + resume, written by nothing). Repoint `routes/workspaces.rs:323`, `dashboard_snapshot.rs:1274,2790`, `projection_contract.rs:1599`, and resume defaults (`main.rs:2659`) at `state-snapshot.json` (extract `.executor_json`); remove `layout.rs:379-382 executor_snapshot()`. — verify: `curl :6677/api/.../workspace/<id>` no longer lists `executor.json` in `errors[]`.
- [ ] **[C2] Materialize or repoint `gate-thresholds.json`.** Either have `save_snapshot` (`event_loop.rs:3341`) *also* write the standalone file, or repoint the 4 readers (`learning/mod.rs:120`, `dashboard.rs:52`, acp `runner.rs:1873`) at `state-snapshot.json.gate_thresholds_json`. — verify: `roko plan run plans/ && cat .roko/learn/gate-thresholds.json` OR `curl :6677/api/learning/gate-thresholds` returns non-empty.
- [ ] **[C3] Add retention for the 4 big uncovered files** (`events.jsonl`, `chain-watcher.log`, `roko.log`, `state/run-ledger.jsonl`) in `retention.rs:115-174`; add `worktrees/` prune + `state/*.bak.*` cap. — verify: `du -sh .roko/{events.jsonl,roko.log,chain-watcher.log,worktrees}` stays bounded after a serve soak.
- [ ] **[C4] Collapse episodes to root.** Repoint serve projection registry (`lib.rs:1022,1099`) and `layout.rs:323-326` label off `memory/`; migrate `memory/episodes.jsonl` (8 rows) into root; converge writers (`feedback_service.rs:141` + orchestrate) on one file. — verify: after a run, only `.roko/episodes.jsonl` grows.
- [ ] **[C5] Resolve `runtime-events.jsonl` reader-without-file.** Instantiate `JsonlLogger::from_roko_dir` (`jsonl_logger.rs:33`) at serve startup, or delete `routes/runs.rs:20` + `shared_runs.rs:326`. — verify: `curl :6677/api/runs` returns data or route is gone.
- [ ] **[C6] Consolidate daimon.** Pick `daimon/affect.json` (more readers); alias orchestrator's `state/daimon.json` (`service_factory.rs:236`). — verify: one file mutates per run.
- [ ] **[C7] Retire dead second-implementations.** `state/run-state.json` (test-only writer), `state/events.json` (`persist.rs:49`), and `roko_runtime::RunLedger` (`run_ledger.rs:1-5` "not wired") — delete or wire. — verify: `rg 'run-state.json|events_json|RunLedger' crates/` returns only live paths.
- [ ] **[C8] Sweep cruft & introduce `LayoutVersion::V2`.** Remove 11 `learn/*.tmp.*`, 18 `state/*.bak.*` (keep N), 64 `workspaces/` demo dirs, stale `worktrees/` checkout; land the real `.roko/VERSION` migration once C0-C4 stabilize. — verify: `roko doctor` reports no dead dirs; `cat .roko/VERSION` → `2`.

---

## Canonical Candidate Map

| Concern | Canonical target | Current duplicates / aliases | Migration action |
|---|---|---|---|
| Workspace layout | Project root plus `.roko/`; public API through `Workspace`; low-level catalog through `roko-fs::RokoLayout` | Hard-coded `.roko` joins across CLI/serve; top-level dir sets differ between `Workspace`, `RokoLayout`, Docker/deploy scripts | Add one `WorkspacePaths` facade over `RokoLayout`; align `ensure_dirs()` with hot runtime dirs: `learn`, `neuro`, `jobs`, `prd`, `research`, `templates`, `runtime`, `state`. |
| Signal/engram log | `.roko/engrams.jsonl` | `.roko/signals.jsonl`, `Workspace::signals_path()`, runner gate verdict append still using a signals path | Convert remaining writers to engrams; keep `signals.jsonl` as read/rename migration input only. |
| Episodes | `.roko/episodes.jsonl` | `.roko/learn/episodes.jsonl`, `.roko/memory/episodes.jsonl`; `RokoLayout` still labels memory as the main episode area | Root is canonical because runner, feedback, truth-map, and readers prefer it; update `RokoLayout` docs/helpers and keep fallback order root -> learn -> memory. |
| Learning state | `.roko/learn/*` | CLI helper functions hard-code many individual paths | Move path construction behind `Workspace`/`RokoLayout` or `LearningPaths`; no new `.roko/memory` writes. |
| Runtime events | `.roko/events.jsonl` for StateHub event stream; `.roko/runtime-events.jsonl` for runtime envelope log | `.roko/state/events.json`, several bus/event vocabularies | Keep both logs only if their roles are distinct; document StateHub projection versus runtime envelope semantics. |
| Runner/dashboard snapshot | `.roko/state/state-snapshot.json` (the only file the writer emits) and Runner-compatible checkpoint schema | `.roko/state/run-state.json`, `executor.json` (**live reader:** serve `routes/workspaces.rs:322-323` + `layout.rs:382`; no writer), `orchestrator.json` | Define one snapshot contract; repoint the `executor.json` reader at `state-snapshot.json`; keep legacy snapshots read-only until migration. |
| Runtime agents | `.roko/runtime/agents.json` | `.roko/runtime/agent-pids.json` from runner persistence | Merge PID cleanup fields into structured `agents.json`; derive old PID list during migration. |
| Gate thresholds | `.roko/learn/gate-thresholds.json` plus gate-owned threshold API | Gate-specific state in more than one layer | Make threshold persistence go through one gate/learn API. |
| Knowledge | Durable store `.roko/neuro/knowledge.jsonl` plus `knowledge-confirmations.jsonl` | Candidate spool documented/constant-named as `.roko/learn/knowledge-candidates.jsonl` | Put admission-owned candidates under `.roko/neuro/knowledge-candidates.jsonl` or add an explicit learn->neuro drain. |
| Daimon/PAD | Prefer `.roko/daimon/affect.json` unless a migration chooses otherwise | `pad-state.json`, `.roko/state/daimon.json` references in docs/designs | Pick one reader/writer; add alias import for the other path. |
| Dreams | `.roko/dreams/*` for dream outputs; `.roko/learn/playbooks/*` for playbooks | Event names and trigger docs are uneven | Keep ownership split; wire generated advice to learning or label it diagnostic. |
| Jobs | `.roko/jobs/*.json`, `.roko/jobs/plan-runs/*` | Chain marketplace/contracts also claim job authority | Local JSON is canonical only for local mode; chain-backed mode needs explicit conversion. |
| PRDs/plans | Top-level `plans/` for source plans; `.roko/plans` and `.roko/prd` for generated/runtime artifacts | Mixed route docs and generated artifacts | Mark source, generated, and runtime artifacts in route/API output. |
| Secrets/auth | Workspace `.roko/secrets.toml`; user login `~/.roko/credentials.json`; env secrets `ROKO_SECRET_*` | Provider keys and server auth tokens | Preserve separation: workspace secrets are project-local; credentials are user-global; provider env stays process-local. |
| Mirage/deploy | Concrete `--state-dir` passed to Mirage/deploy binaries | `ROKO_STATE_ROOT`, `ROKO_WORKDIR`, `RAILWAY_VOLUME_MOUNT_PATH`, `MIRAGE_STATE_DIR` each re-derive roots | Centralize root resolution before spawning child binaries. |
| Terminal | In-memory session manager plus env from config | Route-level and frontend expectations around sessions | Explicit ephemeral state; no durable secret persistence. |

## Current Drift

- `roko-fs::RokoLayout` documents `.roko/memory` as episodes/playbook/skills, while newer learning code uses `.roko/learn`.
- `roko-serve` seed registry describes episodes as `.roko/memory/episodes.jsonl`, while multiple live paths write root `.roko/episodes.jsonl`.
- TUI and status routes read old and new event paths.
- `StateHub` appends `.roko/events.jsonl`, while `JsonlLogger::from_roko_dir` writes `.roko/runtime-events.jsonl`.
- `roko resume` / serve workspace-detail (`routes/workspaces.rs:322-323`) look for `.roko/state/executor.json`, but no writer emits it — Runner v2 writes `state-snapshot.json` only, so the serve route returns an `executor.json` read error for every real workspace, and Graph-path resume remains unsupported.
- `roko_core::config::loader` is the intended config authority, but CLI validation still returns a legacy `ConfigLayer` result after loading core config.
- Atomic write helpers differ: shared helpers use consistent temp naming, while runner persistence still has local `foo.tmp` style writes.

## Migration Plan

1. Add `roko state doctor` or extend `roko doctor` to list every discovered `.roko` state file and whether it is canonical, legacy, generated, or unknown.
2. Add a dry-run migration that prints file moves/copies without mutating.
3. Pick canonical writers before moving old files.
4. Keep legacy readers for at least one release cycle.
5. Add tests with old layouts: root episodes only, learn episodes only, memory episodes only, signals only, engrams only, old executor snapshot.
6. Update serve feed registry and TUI labels to match canonical paths.
7. Make route/status endpoints report source path and freshness.
8. Replace remaining direct `.roko` joins with path helpers or an explicit exception comment.
9. Replace ad hoc persistence temp files with `roko_fs::atomic_write_*` helpers.
10. Update retention/archive docs only after code writes have moved.

## Checklist

- [ ] All direct `.roko` path joins replaced or justified with `RokoLayout`.
- [ ] One canonical episode writer.
- [ ] One canonical runtime event writer.
- [ ] One canonical signal/engram writer.
- [ ] One canonical Daimon affect path.
- [ ] StateHub can hydrate from canonical runtime event log.
- [ ] Resume uses a snapshot schema shared by Runner v2 and Graph.
- [ ] Old paths are visible in doctor output until migrated.
- [ ] Gate verdicts append to `.roko/engrams.jsonl`, not `.roko/signals.jsonl`.
- [ ] Runtime agent registry is one structured file, not separate `agents.json` and `agent-pids.json` writers.
- [ ] Knowledge candidate staging has one owner: learn-to-neuro bridge or neuro admission.
