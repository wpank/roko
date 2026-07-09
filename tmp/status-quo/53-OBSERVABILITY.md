# Observability — tracing, metrics, traces, logs, telemetry

> Status-quo audit · verified 2026-07-08 (re-verified against HEAD 5852c93c05) · supersedes the earlier concise draft of this file · sources: `roko-cli/src/main.rs` tracing bootstrap, `roko-core/src/obs/`, `roko-fs` sinks, `roko-serve` metrics routes + OTLP stub, `roko-runtime/src/state_hub.rs`, runner v2 event loop, `roko-serve/src/feed_agents/`, `roko-acp`, `apps/`, **live `.roko/` listing (all claims re-checked)**, `docs/v2/15-TELEMETRY.md`, `docs/v2-depth/09-telemetry/01-observability-as-lens-pipeline.md`

> **2026-07-08 re-verification note**: every file:line and dead-pipe claim below re-confirmed against current source. New/sharpened findings: (1) `grep '\bLens\b' crates/` → **0** (v2 Lens pipeline still 0% built); (2) `grep 'metrics: Some' crates/` → **0** (runner metric pipe still dead); (3) **`.roko/events.jsonl` decomposed** — 157,264 lines / 44 MB, of which **152,965 (97.3%) are `feed_tick`** + 3,291 `chain_block`; genuine run/gate/task events are <0.5% of the file. This is a **feed-agent + chain-watcher firehose masquerading as the run event log** — see new §"events.jsonl firehose". Live `.roko/metrics/` and `.roko/traces/` **still empty**; `.roko/roko.log` 12 MB, `chain-watcher.log` **23 MB**, no rotation anywhere.

Status vocab: ✅ wired | 🔌 built-not-wired | 🟡 partial | ❌ missing | 🕰️ legacy-only (`legacy-orchestrate`)

## Summary

Observability is **file-first JSONL plus one in-process `MetricRegistry` per surface**, with a sophisticated tracing bootstrap and a real Prometheus scrape endpoint — but several deliberate-looking pipes are dead ends:

- **Tracing** is centralized and good: one registry in `roko-cli/src/main.rs:2074-2145` writes `.roko/roko.log` (or `.roko/serve-tui.log` in TUI mode, :2100-2106), adds a stderr layer only with `--verbose`/`RUST_LOG` (:2125-2139), applies secret redaction (`RedactingFormat`, :2131-2134), and per-mode `EnvFilter`s (:2076-2083). Satellite binaries roll their own (`roko-acp` → `.roko/acp.log`, chain-watcher/agent-relay → stderr).
- **Metrics**: `roko_core::obs::MetricRegistry` (metrics.rs:263) is wired end-to-end **only in `roko serve`**: AppState builds it (roko-serve/src/state.rs:947-950), threads it into `ModelCallService` (`with_metrics`, roko-agent/src/model_call_service.rs:100-102, 282), and exposes `GET /metrics` (root Prometheus scrape, routes/metrics.rs:1-40+) plus `GET /api/metrics{,/prometheus,/summary,…}` (routes/status/metrics.rs:31-141, mounted status/mod.rs:32). **Runner v2's metric hook is dead**: `commands/plan.rs:379-380` creates + registers a registry, then builds `RunConfig` with `metrics: None` (:569); `serve_runtime.rs:628` also passes `None`; nobody constructs `metrics: Some(…)` — so the gate-verdict counter at `runner/event_loop.rs:1024-1037` never records.
- **Tool traces/metrics**: `roko-fs` has production-quality sinks (`JsonlTraceSink` → `.roko/traces/YYYY-MM-DD/{trace_id}.jsonl`, trace_sink.rs; `JsonlMetricsSink` → `.roko/metrics/tool_metrics.jsonl`, tool_metrics_sink.rs; wrapper `FsObservabilitySinks`, observability.rs). The CLI bootstraps the directories on every run (`bootstrap_observability_dirs`, main.rs:3098-3110) but **only legacy orchestrate.rs attaches the sinks** (orchestrate.rs:85, 2734, 4558) — live `.roko/metrics/` and `.roko/traces/` are **both empty**, and no reader exists.
- **OTLP is a stub**: gated behind non-default feature `otlp` (roko-serve/Cargo.toml:14-22); even when enabled, `init_otlp_tracing` only logs "layer installation deferred" because the CLI already installed the global subscriber (roko-serve/src/lib.rs:313-317, 2879-2909).
- **StateHub is the real projection layer**: watch-channel snapshot + replay ring + optional append to `.roko/events.jsonl` (roko-runtime/src/state_hub.rs:59, 75, 107-125), replay/ingest APIs, instantiated per-workdir by serve (state.rs:839-846). TUI reads at render, SSE/WS subscribe, REST clones snapshots. **But its on-disk log is polluted**: 97.3% of `.roko/events.jsonl` is `feed_tick` UI-liveness noise from serve's 15 feed agents (feed_agents/mod.rs:90) — the resume-critical run/gate/task events are <0.5% of a 44 MB file. See §events.jsonl firehose.
- **v2 telemetry design is 0% implemented as designed**: no `Lens` type exists anywhere in the workspace (grep `struct|trait|enum *Lens` → 0). But several lens *outcomes* exist under other names — c-factor is computed and served (`/api/metrics/c_factor`), cost/efficiency aggregates come from learn JSONLs, StateHub projections exist.

Net: decide ownership (the old draft's question stands), then close three dead pipes — RunConfig.metrics, FsObservabilitySinks attachment, OTLP — or delete them.

## Telemetry inventory table

| Signal source | Writer | Reader | Status |
|---|---|---|---|
| tracing spans/logs (whole workspace) | `roko-cli/src/main.rs:2074-2145` registry: file layer → `.roko/roko.log` (append, no ANSI, :2100-2120); stderr layer only w/ `--verbose`/`RUST_LOG`/raw-logs (:2125-2139); redaction (:2131-2134); TUI-mode filter suppressions (:2076-2083) | humans; `roko-serve` workspace detail tails last 50 lines (routes/workspaces.rs:371-376); `commands/plan.rs:741` points users at it | ✅ |
| serve-TUI tracing | same init, file switches to `.roko/serve-tui.log` when TUI mode (main.rs:2101-2104) | humans only | ✅ write-only |
| dashboard-TUI tracing | separate `tracing::Dispatch` → `.roko/tui.log` (tui/app.rs:473-495) | humans only | ✅ write-only |
| ACP tracing | `roko-acp/src/handler.rs:620-646`: `tracing_appender::rolling::never` + non_blocking → `--log-file` (default `.roko/acp.log`, main.rs:696; editor profile `.roko/editor-acp.log`, main.rs:3575); `set_global_default` best-effort (`let _ =`, :637) | humans | ✅ write-only |
| daemon stdout/stderr | `.roko/logs/daemon.log` (daemon.rs:755-756; launchd plist launchd.rs:50) | `roko daemon logs` (`daemon_logs` daemon.rs:709, `print_recent_daemon_logs` :871, follow mode) | ✅ only log with a CLI reader |
| chain-watcher subprocess | roko-serve spawns it with output redirected → `.roko/chain-watcher.log` (roko-serve/src/lib.rs:440-444); the binary itself logs stderr (`apps/roko-chain-watcher/src/main.rs:42-49`, `ROKO_LOG` env) | none | ✅ write-only |
| approval-TUI stderr | `.roko/runner-stderr.log` via `layout.runner_stderr_log()` (commands/plan.rs:582; path decl roko-fs/src/layout.rs:257-260, roko-core/src/workspace.rs:255-258) | none | ✅ write-only |
| `MetricRegistry` counters/histograms | roko-core/src/obs/{metrics,histograms,schema,health,scrub}.rs; `register_standard_metrics`; populated by serve's `ModelCallService` LLM-call metrics (model_call_service.rs:100-102, `with_metrics` :282, via `ServiceConfig.metrics` state.rs:949-950) | `GET /metrics` root Prometheus scrape (routes/metrics.rs:23-40+, combines registry render + uptime/agents/plans); `GET /api/metrics` snapshot JSON + `/api/metrics/prometheus` (routes/status/metrics.rs:31-34, 140-141, 253; mounted status/mod.rs:32) | 🟡 serve-only |
| Runner-v2 gate-verdict metrics | hook exists: `runner/event_loop.rs:1024-1037` increments `ROKO_GATE_VERDICTS_TOTAL` **iff `config.metrics`** — but `RunConfig.metrics` (runner/types.rs:1338) is `None` at every construction site (plan.rs:569 — after building a registry at :379-380 that is then dropped; serve_runtime.rs:628; defaults types.rs:1458,1499); grep `metrics: Some` → 0 | — | 🔌 dead pipe |
| Tool-call traces | `JsonlTraceSink` → `.roko/traces/<date>/<trace_id>.jsonl` (roko-fs/src/trace_sink.rs; trait `TraceSink` roko-core/src/tool/trace.rs:749; noop default in roko-std/src/trace_sink.rs) | none found (no reader of `.roko/traces` in serve/tui/commands) | 🔌 dirs bootstrapped every CLI run (main.rs:3098-3110) but sink attached only in 🕰️ orchestrate.rs:4558; **live dir empty** |
| Tool metrics | `JsonlMetricsSink` → `.roko/metrics/tool_metrics.jsonl` (roko-fs/src/tool_metrics_sink.rs; trait roko-core/src/tool/metrics.rs:275) | none | 🔌 same as above; **live dir empty** |
| Legacy Prometheus dump | orchestrate.rs:5937-5944 writes `.roko/metrics/prometheus.txt` post-run | post-mortem humans | 🕰️ |
| Episodes | `.roko/episodes.jsonl` via `roko-learn/src/episode_logger.rs`; runner v2 writes through `FeedbackFacade` episode sink (commands/plan.rs:471-489; `runtime_feedback.rs`) | serve metrics summary/model_efficiency (routes/status/metrics.rs:22 imports `Episode`), `roko learn episodes`, dreams | ✅ |
| Efficiency events | `.roko/learn/efficiency.jsonl` (`AgentEfficiencyEvent`, roko-learn/src/efficiency.rs; runner v2 learning subscriber; legacy subscriber orchestrate.rs:8543-8552) | serve metrics routes (routes/status/metrics.rs:19-20 incl. `compute_fleet_cfactor`), legacy cost-overrun watcher (orchestrate.rs:6328-6342) | ✅ |
| C-factor | `refresh_cfactor_snapshot` post-run (orchestrate.rs:8594 legacy; learn crate) + fleet compute in serve | `GET /api/metrics/c_factor` (routes/status/metrics.rs:66) | 🟡 |
| StateHub events/snapshot | publishers: runner v2 `TuiBridge`/projection, serve; `publish` broadcasts + ring + snapshot + optional `.roko/events.jsonl` append (state_hub.rs:59, 75, 85, 107-125 `with_event_log`); serve instantiates with event log (state.rs:839-846); **also written directly by runner persist family (`persist.rs:282 append_jsonl(paths.events_jsonl,…)`)** | TUI watch-channel @render; SSE/WS subscribers; REST snapshot; `replay_from_log`/`ingest_log`; TUI replay on startup (tui/app.rs:536,550,2515) | 🟡 closest thing to v2 projection layer, **but the on-disk file is 97% feed-agent noise — see §events.jsonl firehose** |
| `feed_tick` firehose | `roko-serve/src/feed_agents/mod.rs:90` — 15 background feed agents `publish(ServerEvent::FeedTick{…})` on the event bus (events.rs:654, lib.rs:1602); mapped to `DashboardEvent::FeedTick` (dashboard_snapshot.rs:201) → appended to `.roko/events.jsonl` via StateHub | relay forward (lib.rs:2741-2749), TUI feed pane | 🟡 **undocumented in v2 persistence; dominates events.jsonl (152,965/157,264 lines)** |
| RuntimeEvent JSONL | `roko_runtime::JsonlLogger::from_roko_dir` (serve state.rs:958-959) + runner persist family (`events.jsonl`, `run-ledger.jsonl` — see 36-ORCHESTRATION:46) | resume/recovery, projections | ✅ |
| Heartbeat snapshots | `.roko/learn/` via `crates/roko-cli/src/heartbeat.rs:1-6` | dashboards/post-mortem (per doc comment) | 🕰️ only caller is gated orchestrate.rs:6996-7075 |
| OTLP export | `init_otlp_tracing` (roko-serve/src/lib.rs:2879-2909) — **logs intent and returns**; called from `start_background` when `[serve.tracing].otlp_endpoint` set (lib.rs:313-317); feature `otlp` not in `default = []` (Cargo.toml:14-22) | — | ❌ stub (config parsed, no export) |

## Log sprawl audit

Live `.roko/` (this repo, verified by listing) contains **six root-level log files** — `acp.log`, `chain-watcher.log`, `roko.log`, `runner-stderr.log`, `serve-tui.log`, `tui.log` — plus the `logs/` dir for `daemon.log`. One writer each (table above); ownership is clear but conventions are not:

- **Three different tracing sinks for three UIs**: main CLI (`roko.log`), serve-TUI (`serve-tui.log`), dashboard-TUI (`tui.log` via a *second* subscriber dispatch, tui/app.rs:477-495). A crash spanning modes scatters evidence across files.
- **No rotation anywhere**: `roko.log` is plain append (main.rs:2100-2120); ACP uses `rolling::never` (handler.rs:628); daemon recreates on start (`StdFile::create`, daemon.rs:755). Long-lived workspaces grow unbounded.
- **One reader total**: only `daemon.log` has a CLI reader (`roko daemon logs`); serve tails `roko.log` for the workspace-detail route (workspaces.rs:371-376). Nothing reads acp/tui/runner-stderr/chain-watcher logs programmatically.
- **Best-effort global installs**: roko-acp's `set_global_default` failure is swallowed (`let _ =`, handler.rs:637) — embedded in a process that already has a subscriber, ACP logs vanish silently.
- Satellite binaries (`apps/roko-chain-watcher/src/main.rs:42-49`, `apps/agent-relay/src/main.rs:40-43`) log to stderr with their own env filters (`ROKO_LOG` vs default env) — inconsistent knobs.
- Legacy flush plumbing (`sync_file_if_present(".roko/logs/daemon.log")`, orchestrate.rs:6036; daemon.rs:1445) suggests logs were once part of snapshot integrity — undocumented.
- **Tracing dependency is near-universal, not opt-in**: 21 of the workspace crates declare `tracing` in `Cargo.toml`, so the wave-1 "some crates carry a false 'no tracing dep' comment" concern is a documentation artifact — the sole `tracing`-adjacent "no …" comments in code (e.g. orchestrate.rs:6868 "without tracing" = a stderr fallback) are ordinary usage, not stale dep annotations. The real inconsistency is **filter knobs**, not deps: CLI/serve use `--verbose`/`RUST_LOG` + per-mode `EnvFilter`; satellites use `ROKO_LOG` (chain-watcher main.rs:42-49) vs their own defaults (agent-relay main.rs:40-43). One binary's `RUST_LOG` does not affect another's. Unify (P3).

## events.jsonl firehose (undocumented, 44 MB)

`.roko/events.jsonl` is presented everywhere (persist.rs:51-52 doc, state_hub.rs:75, TUI replay) as the **runner event log** — the append-only history of a plan run, consumed by TUI/server for resume and replay. In practice it is a **mixed firehose** with no partitioning. Verified line-count breakdown of the live 157,264-line / 44 MB file:

| type | count | % | writer |
|---|---|---|---|
| `feed_tick` | 152,965 | 97.3% | roko-serve feed agents (feed_agents/mod.rs:90) |
| `chain_block` | 3,291 | 2.1% | chain-watcher → serve event bus |
| `feed_agent_online` | 232 | — | feed agents |
| `gate.dispatch.started` / `gate.completed` | 133 / 130 | — | runner v2 (the events resume/replay actually needs) |
| `task.attempt.{started,completed}` | 119 / 112 | — | runner v2 |
| `agent.token_usage` / `agent.tool_call` | 55 / 42 | — | runner v2 dispatch |
| `run.{started,completed}`, `plan.started`, `resume.marker`, `prompt.assembled`, `agent.dispatch.*` | ≤22 each | — | runner v2 |
| `chain_tx`, `chain_contract_event` | 14 / 9 | — | chain-watcher |

**Implications / drift:**
- The **signal-to-noise ratio for run replay is ~1:200**. Anyone replaying `events.jsonl` for resume/recovery scans 150K feed ticks to find ~600 real run events.
- Two independent writers hit the same file: StateHub's `EventLogWriter` (broadcast fan-out incl. feed ticks) **and** the runner persist family (`persist.rs:282`, direct append of the run-critical events). No coordination, no schema partition, no rotation/GC (state_hub.rs append is best-effort).
- **Not a Signal log.** Entries are `DashboardEvent`/`ServerEvent` JSON (`type`/`timestamp`/`run_id`/`marker`…), not `Signal` — so it is invisible to the Signal/lineage story the v2 docs assume. v2 persistence (docs/v2) never mentions this file.
- Feed ticks are **ephemeral UI liveness pulses** that should never have been persisted. This is the single largest observability-data-quality defect: it inflates the file 200×, defeats any human `tail`/`grep`, and will keep growing unbounded while serve runs.

**Fix direction** (see checklist P1): stop persisting `FeedTick` (filter at the StateHub `EventLogWriter` sink, or route feed ticks to a broadcast-only channel that never touches disk), and/or split the on-disk log into `run-events.jsonl` (resume-critical) vs an ephemeral feed channel. Chain events belong with `chain-watcher.log`, not here.

## vs v2 telemetry design

`docs/v2/15-TELEMETRY.md` specifies an **Observe protocol** (read-only Cells: `observe()`/`observes()`/`scope()`, LensScope), **11 built-in Lenses** (CostLens, LatencyLens, QualityLens, EfficiencyLens, ErrorLens, DriftLens, BudgetLens, TrendLens, AnomalyLens, UsageLens, CollectiveIntelligenceLens §4.1-4.11) with c-factor as five sub-lenses (§5), and StateHub projections as typed data contracts. `docs/v2-depth/09-telemetry/01-observability-as-lens-pipeline.md` adds the 3-stage **Collector → Transform → Export** lens pipeline, logs as Bus pulses on `telemetry.log.*`, MetricLens numeric projections, and TraceLens OTLP export with Signal lineage.

| v2 concept | Reality |
|---|---|
| Observe protocol / Lens Cells | ❌ zero `Lens` types workspace-wide (grep) |
| Collector→Transform→Export pipeline | ❌; nearest analog is tracing-subscriber layers (fmt+file+filter) |
| Logs as Bus pulses (`telemetry.log.*`) | ❌; logs are tracing-only, never enter the Signal/event bus |
| MetricLens → numeric projections | 🟡 `MetricRegistry` + `/metrics` covers the export half; projections are hand-rolled per-route aggregations over learn JSONLs (routes/status/metrics.rs) |
| TraceLens → OTLP, Signal lineage | ❌ OTLP stub; `JsonlTraceSink` records tool-call traces, not Signal lineage |
| CostLens / EfficiencyLens / BudgetLens | 🟡 same data exists as `costs`/`efficiency.jsonl` aggregation + budget guardrails, not composable Cells |
| AnomalyLens | 🟡 `AnomalyDetector` exists in roko-learn (used by legacy learning subscriber, orchestrate.rs:8541) — not a Lens, not in v2 runner |
| CollectiveIntelligenceLens (c-factor) | 🟡 computed for real (`compute_fleet_cfactor`, `/api/metrics/c_factor`) — the *metric* landed without the Lens machinery |
| StateHub named projections as contracts | 🟡 StateHub is live w/ ring+replay+`events.jsonl` (state_hub.rs) and serve has a `projection_contract` module (routes/status/metrics.rs:15 imports `RuntimeProjectionSet`), but the 15-TELEMETRY named-view catalog (plan_health, cost_meter, …) is not the implemented shape |

Conclusion: v2 telemetry should be read as a **refactor target for existing flows** (JSONL aggregations → Lenses, registry → MetricLens export) rather than greenfield — most of the *data* already exists.

## Migration checklist

- [ ] **[P0]** Un-dead-end runner metrics: pass the registry built at `commands/plan.rs:379-380` into `RunConfig.metrics` (plan.rs:569) and thread serve's `state.metrics` through `serve_runtime.rs:628`, so gate-verdict counters (event_loop.rs:1024-1037) and future run metrics reach `/metrics` — verify: run a plan, then `curl -s localhost:6677/metrics | grep roko_gate_verdicts_total` shows non-zero
- [ ] **[P1]** Stop persisting `feed_tick` to `.roko/events.jsonl`: filter `DashboardEvent::FeedTick` (and `chain_block`/`chain_tx`) at the StateHub `EventLogWriter` sink (state_hub.rs `append`, ~:59) so only resume-critical run/gate/task events hit disk; alternatively split into `run-events.jsonl` vs a disk-less feed channel. Today the file is 97.3% noise (152,965/157,264 lines) and unbounded — verify: after a serve session with feeds running, `grep -c feed_tick .roko/events.jsonl` is 0 and file size tracks run-event volume, not uptime
- [ ] **[P1]** Attach `FsObservabilitySinks` in the live tool-loop path (runner v2 / `roko run`) or delete the sinks + dir bootstrap (main.rs:3098-3110) — empty `.roko/traces/` + `.roko/metrics/` on every workspace is cargo-cult — verify: after `roko run "…"`, `ls .roko/traces/$(date +%F)/` is non-empty (or dirs no longer created)
- [ ] **[P1]** Finish or remove OTLP: either accept an OTLP layer at CLI tracing bootstrap (per the plan written in lib.rs:2888-2903) or drop `[serve.tracing].otlp_endpoint` + the `otlp` feature — verify: spans arrive at a local collector, or config field gone from schema + docs
- [ ] **[P1]** Log lifecycle policy: adopt rotation (size- or day-based) for `.roko/*.log`, document each file in `roko doctor`/README, and consider a `roko logs [--source]` umbrella reader — verify: `roko doctor` lists every log file with its writer; logs rotate past threshold
- [ ] **[P2]** Decide telemetry ownership (carried from prior draft): distributed writers with documented schemas vs a shared telemetry crate; record in `tmp/status-quo/26-CANONICAL-DECISIONS.md` — verify: decision entry exists and names owners for metrics/traces/events
- [ ] **[P2]** Dashboard parity + bridge coverage (carried): test that StateHub projections match persisted run/gate/episode files, and that event-bridge conversions don't drop variants — verify: `cargo test -p roko-serve projection` covers every `DashboardEvent`/`RuntimeEvent` variant
- [ ] **[P2]** Expose feedback backlog/drops as metrics (carried): the learning/feedback channels drop silently under backpressure — verify: `/metrics` exposes a `roko_feedback_dropped_total` counter
- [ ] **[P3]** Implement the Lens layer where it pays first: wrap existing JSONL aggregations (cost, efficiency, gate rate, c-factor) as Observe-protocol Cells feeding StateHub named projections per 15-TELEMETRY §6 — verify: TUI + `GET /api/metrics/summary` read the same projection type
- [ ] **[P3]** Unify satellite-binary logging knobs (`ROKO_LOG` vs `RUST_LOG` vs hardcoded filters) and surface ACP subscriber-install failure instead of `let _ =` (roko-acp/src/handler.rs:637) — verify: one documented env var controls all roko binaries

## Deep pass 2 — obs-signal map + Lens-gap per signal (verified HEAD `5852c93c05`, 2026-07-08)

Every observability signal roko can emit, classified by **kind** (metric / trace / log / event), **what is emitted**, **where written**, **who reads it**, and the **v2 Lens-pipeline gap** (`docs/v2/15-TELEMETRY.md` names the Lens each *would* map to). Re-confirmed: `grep '\bLens\b' crates/` → **0** (no Lens type exists), `grep 'metrics: Some' crates/` → **0** (runner metric pipe still dead), `.roko/metrics/` + `.roko/traces/` empty on disk, `.roko/events.jsonl` = 44 MB / 97% feed noise, `.roko/chain-watcher.log` = **23 MB**, `.roko/roko.log` = **12 MB**, none rotated.

| Kind | Signal | Emitted (writer file:line) | Written to | Read by | Lens-gap |
|---|---|---|---|---|---|
| metric | LLM-call counters/histograms | serve `ModelCallService.with_metrics` model_call_service.rs:100-102,282 → `MetricRegistry` obs/metrics.rs:263 | in-process registry | `GET /metrics` (routes/metrics.rs:23-40), `GET /api/metrics{,/prometheus,/summary}` (status/metrics.rs:31-141) | 🟡 = MetricLens *export* half; no Collector→Transform Cell |
| metric | gate-verdict counter `ROKO_GATE_VERDICTS_TOTAL` | hook event_loop.rs:1024-1037 `iff config.metrics` | — | **nobody** | 🔌 **dead pipe**: `RunConfig.metrics` is `None` at every ctor (plan.rs:569 after building+dropping a registry at :379-380; serve_runtime.rs:628; defaults types.rs:1458,1499) |
| metric | c-factor (fleet + per-run) | `refresh_cfactor_snapshot` orchestrate.rs:8594 (legacy); `compute_fleet_cfactor` (serve) | learn JSONL + in-mem | `GET /api/metrics/c_factor` status/metrics.rs:66 | 🟡 CollectiveIntelligenceLens metric landed, Lens machinery didn't |
| metric | legacy Prometheus dump | orchestrate.rs:5937-5944 | `.roko/metrics/prometheus.txt` | post-mortem humans | 🕰️ legacy-only |
| trace | tool-call traces | `JsonlTraceSink` trace_sink.rs (trait tool/trace.rs:749) | `.roko/traces/<date>/<trace_id>.jsonl` | **nobody** (no reader in serve/tui/commands) | 🔌 dirs bootstrapped every run (main.rs:3098-3110) but sink attached only in 🕰️ orchestrate.rs:4558 → **dir empty**; would be TraceLens/OTLP |
| metric | tool metrics | `JsonlMetricsSink` tool_metrics_sink.rs (trait tool/metrics.rs:275) | `.roko/metrics/tool_metrics.jsonl` | **nobody** | 🔌 same attach gap → **dir empty** |
| log | main-CLI tracing | main.rs:2074-2145 (file layer, redaction :2131-2134, per-mode EnvFilter) | `.roko/roko.log` (append, **no rotation**, 12 MB) | humans; serve tails last 50 (workspaces.rs:371-376) | ❌ v2 wants logs as Bus pulses `telemetry.log.*`; these never enter the Signal/event bus |
| log | serve-TUI tracing | same init, path switch main.rs:2101-2104 | `.roko/serve-tui.log` | humans | ❌ write-only |
| log | dashboard-TUI tracing | **second** `tracing::Dispatch` tui/app.rs:473-495 | `.roko/tui.log` | humans | ❌ write-only; crash spanning modes scatters evidence |
| log | ACP tracing | handler.rs:620-646 `rolling::never` + non_blocking; `set_global_default` best-effort (`let _=` :637) | `.roko/acp.log` (or `editor-acp.log`) | humans | ❌ install failure swallowed silently |
| log | daemon stdout/stderr | daemon.rs:755-756 `StdFile::create` (recreates on start) | `.roko/logs/daemon.log` | `roko daemon logs` (daemon.rs:709/871, follow) | ✅ **only log with a CLI reader** |
| log | chain-watcher | serve redirect lib.rs:440-444; binary stderr apps/roko-chain-watcher/main.rs:42-49 (`ROKO_LOG`) | `.roko/chain-watcher.log` (**23 MB, no rotation**) | nobody | ❌ write-only, largest log |
| log | approval-TUI stderr | plan.rs:582 → `layout.runner_stderr_log()` (roko-fs/layout.rs:257) | `.roko/runner-stderr.log` | nobody | ❌ write-only |
| event | StateHub events/snapshot | runner `TuiBridge`/projection + serve `publish` (state_hub.rs:59,75,107-125 `with_event_log`); **also runner persist.rs:282 direct append** | `.roko/events.jsonl` (append, best-effort, **no GC**) | TUI @render, SSE/WS subs, REST snapshot, `replay_from_log`, TUI startup replay (tui/app.rs:536,550,2515) | 🟡 closest to v2 projection layer; **file 97% feed noise** (§firehose) |
| event | `feed_tick` firehose | 15 serve feed agents feed_agents/mod.rs:90 `publish(ServerEvent::FeedTick)` → dashboard_snapshot.rs:201 → StateHub append | `.roko/events.jsonl` | relay forward lib.rs:2741-2749, TUI feed pane | 🟡 **152,965 / 157,264 lines (97.3%)** — ephemeral UI pulse that should never touch disk |
| event | episodes | episode_logger.rs; runner via FeedbackFacade sink plan.rs:471-489 | `.roko/episodes.jsonl` | serve metrics, `roko learn episodes`, dreams | ✅ (QualityLens data exists un-Lensed) |
| event | efficiency | efficiency.rs `AgentEfficiencyEvent`; runner learning subscriber | `.roko/learn/efficiency.jsonl` | serve status/metrics.rs:19-20 (`compute_fleet_cfactor`), legacy cost watcher orchestrate.rs:6328 | ✅ EfficiencyLens/CostLens data exists un-Lensed |
| event | RuntimeEvent JSONL | `JsonlLogger::from_roko_dir` serve state.rs:958-959 + runner persist family | `events.jsonl`, `run-ledger.jsonl` | resume/recovery, projections | ✅ |
| event | heartbeat snapshots | heartbeat.rs:1-6 | `.roko/learn/` | dashboards/post-mortem | 🕰️ only caller is gated orchestrate.rs:6996-7075 |
| trace | OTLP export | `init_otlp_tracing` lib.rs:2879-2909 (**logs intent, returns**) | — | — | ❌ stub; feature `otlp` not in `default=[]` (Cargo.toml:14-22); even enabled it defers because CLI owns the global subscriber |

**Signal-kind coverage:** metrics 🟡 (registry export works serve-only, run pipe dead), traces ❌ (sinks built, never attached in live path, OTLP stub), logs ✅-write-but-❌-read (6 root files, 1 reader), events ✅-but-polluted (real projection layer, 97% noise). **Not one signal reaches a `Lens`** — the entire v2 Observe/Collector→Transform→Export protocol is 0% built; the data for CostLens/EfficiencyLens/QualityLens/CollectiveIntelligenceLens already exists as raw JSONL aggregations, making v2 telemetry a **refactor target, not greenfield**.

### Log-rotation gap (the P1 operational risk)

| File | Size (live) | Rotation | Writer strategy |
|---|---|---|---|
| `.roko/chain-watcher.log` | **23 MB** | ❌ none | serve subprocess redirect (lib.rs:440-444) |
| `.roko/roko.log` | **12 MB** | ❌ none | plain append (main.rs:2100-2120) |
| `.roko/events.jsonl` | **44 MB** | ❌ none (best-effort append, no GC) | StateHub + persist.rs:282 |
| `.roko/acp.log` | 72 KB | ❌ `rolling::never` (handler.rs:628) | non_blocking appender |
| `.roko/serve-tui.log`, `tui.log`, `runner-stderr.log`, `logs/daemon.log` | small | daemon *recreates* on start; others plain append | — |

**No rotation anywhere.** ACP explicitly uses `rolling::never`; the CLI/serve file layers are plain `append`; daemon is the only one that truncates (recreates on start). Long-lived `roko serve` workspaces grow `chain-watcher.log` and `events.jsonl` **unbounded**. Only `daemon.log` has a CLI reader. **Env-knob inconsistency confirmed:** CLI/serve gate stderr on `--verbose` / `RUST_LOG` (main.rs:2124) and choose format via `ROKO_LOG_FORMAT` (main.rs:2942), the directive builder reads *both* `RUST_LOG` and `ROKO_LOG` (main.rs:2293), while satellite `roko-chain-watcher` reads only `ROKO_LOG` (main.rs:44) and `agent-relay` uses its own default — **one binary's `RUST_LOG` does not affect another's**, and there is no umbrella `roko logs` reader.

### Deep-pass checklist (additions to the P0-P3 list above)

- [ ] **[P1]** Add size/day rotation to `.roko/*.log` (start with the 23 MB `chain-watcher.log` and 12 MB `roko.log`) via `tracing_appender::rolling::daily` — verify: `.roko/roko.log.<date>` files appear and old ones age out
- [ ] **[P1]** GC/cap `.roko/events.jsonl` (44 MB) *and* stop persisting `feed_tick`/`chain_block` (97% of it) — see §firehose — verify: `grep -c feed_tick .roko/events.jsonl` → 0, file size tracks run volume not uptime
- [ ] **[P2]** Unify the log env knob: make `ROKO_LOG` (or `RUST_LOG`) authoritative across CLI + `roko-chain-watcher` + `agent-relay`; document in `roko doctor` — verify: one env var toggles verbosity in all three binaries
- [ ] **[P2]** Add a `roko logs [--source]` umbrella reader so the 5 currently-reader-less logs are inspectable without `cat` — verify: `roko logs --source chain-watcher` tails the file

## Open questions

1. **Is root `GET /metrics` meant to be publicly scrapeable?** It sits alongside authed API routes — confirm auth posture for Prometheus scrapes vs `/api/metrics/*`.
2. **Which registry when serve launches runs?** Serve has `state.metrics`; runner v2 wants `RunConfig.metrics`. Same instance (one process-wide registry) or per-run registries aggregated at scrape time?
3. **What is the canonical replay source** — StateHub `events.jsonl`, runner `run-ledger.jsonl`, episodes, or (unwritten) tool traces? Four partially-overlapping histories exist; v2 lineage design assumes one.
4. **`events.jsonl` growth**: StateHub appends best-effort with no GC/rotation — but the real problem is content, not just size: 97.3% is `feed_tick` (see §firehose). Should feed ticks be persisted **at all**, or is this a serve-only bug where an ephemeral UI channel accidentally shares the runner's resume log? Bound it *and* filter it.
5. **Do tool-call traces have a future consumer** (replay UI? episode enrichment?) or should `TraceSink` be folded into the episode/lineage story before anyone wires it?
6. **Heartbeat snapshots** (`.roko/learn/`, heartbeat.rs) — port to runner v2 as the delta/theta cadence for dreams + stuck detection (see `88-CONDUCTOR.md`), or superseded by serve-side schedulers?
