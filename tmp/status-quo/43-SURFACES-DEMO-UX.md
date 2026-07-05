# Surfaces — TUI, Demo, Chat, Web Apps

> Status-quo audit · **re-verified 2026-07-08 against git HEAD `5852c93c0`** (was 2026-07-07) · sources this pass: `crates/roko-cli/src/tui/` (80 files, 42K LOC), **`crates/roko-cli/src/runner/` (20 files, 17,090 LOC — NEW, the default `plan run` path)**, **`crates/roko-cli/src/projection/` (3 files — NEW rendering-unification seam)**, **`crates/roko-demo/` (21 rs files — NEW compiled chain-scenario orchestrator + its own ratatui TUI)**, `crates/roko-core/src/dashboard_snapshot.rs` (2,964 LOC, materially expanded), `demo/` (5 subdirs, demo-app = 515 src files, dist rebuilt Jul 7), `apps/` (3 workspace crates), `docs/v1/12-interfaces/` (24 docs), `docs/v2-depth/16-surfaces/` (8 docs), `tmp/tmp-feedback/2/{09,25}` (TUI-panels-broken / agent-data-gaps), `tmp/status-quo/{20,66,68}`.
>
> Status vocab: ✅ wired/working · 🔌 built-not-wired · 🟡 partial · ❌ not implemented · 🕰️ legacy/superseded.

## What changed since the 07-07 pass (read this first)

The navigation layer moved under my feet. Three whole subsystems that the prior draft never named now dominate this surface:

1. **Runner v2 is the default `plan run` path.** `crates/roko-cli/src/commands/plan.rs:654` dispatches to `roko_cli::runner::event_loop::run(...)` — **not** the old `orchestrate.rs` `PlanRunner`. The `runner/` module (20 files, 17,090 LOC) owns: agent-event handling (`agent_events.rs`), the `TuiBridge` push seam (`tui_bridge.rs`), a `Projection` broadcast facade (`projection.rs`), gate dispatch, merge, resume, and an async `snapshot_writer.rs`. **My prior census, docs/v2-depth, and CLAUDE.md all still describe orchestrate.rs's PlanRunner as the runtime — that is now stale.**
2. **A new `crates/roko-cli/src/projection/` module** (`mod.rs`, `cli_progress.rs`, `dashboard.rs`) is the explicit "single seam that turns runner events into" TUI / HTTP-SSE / non-TUI-CLI shapes (`projection/mod.rs:1-18`). Its doc-comment names the exact bug docs 09/25 describe: *"TUI showed one set of fields, HTTP showed another, non-TUI CLI rendered a third."* This is the fix-in-progress for the panel drift.
3. **`crates/roko-demo/` is now a compiled workspace crate** (`Cargo.toml:71`), superseding the `bin/roko-demo` shell script + `demo/demo-old`. It has its own `clap` CLI (`up|deploy|seed|verify|benchmark|tournament|autonomous|tui|register-agent|list`), its own **ratatui TUI** (`src/tui.rs`, driven by `DemoEvent` over an mpsc channel, panels: agents/knowledge/economics/log/round), and its own **WebSocket broadcast server on :9090** (`src/ws_server.rs`). This is a **fourth live TUI-class surface** the prior draft missed entirely.

**Storage split-brain is now a triple-brain** (see dedicated section). Gate verdicts feeding the dashboard are read from `.roko/engrams.jsonl` (`dashboard_snapshot.rs:1276`, `read_signal_gates(&engrams_path)`), while `.roko/signals.jsonl` (80 KB) and the new 44 MB `.roko/events.jsonl` are separate logs. On disk right now: `engrams.jsonl` is 10 KB and stale (May 6), `signals.jsonl` 80 KB (May 8), `events.jsonl` 44 MB (May 9) — the dashboard reads the smallest/stalest of the three.

## Summary

**Lead (07-08 delta):** the runtime driving these surfaces changed. `plan run` now flows through **Runner v2** (`crates/roko-cli/src/runner/`, 20 files / 17K LOC) via `commands/plan.rs:654`, not orchestrate.rs's PlanRunner. Runner v2 ships a `TuiBridge` push seam, a `Projection` broadcast facade, an async unified `state-snapshot.json` writer, and a new 44 MB `.roko/events.jsonl` event log — plus a new top-level `projection/` module that exists specifically to stop the "TUI shows one field set, HTTP another, CLI a third" drift the panel-broken feedback docs describe. The struct-level gaps those docs flag (agent tokens/task/cost, output tail, diagnoses) are now closed in `roko-core`'s `DashboardSnapshot` and published by `TuiBridge`; what's unverified is the end-to-end runtime fill and the **storage split-brain** (dashboard pull-mode still reads gate verdicts from the stale `engrams.jsonl` while verdicts flow through the runner path — the mechanical cause of empty panels). Separately, **`crates/roko-demo` is now a compiled chain-scenario orchestrator** with its own ratatui TUI + WS:9090 — a fourth TUI-class surface the prior draft denied existed. The rest of the census below (10 F-tabs, dual dashboards, demo-app drift, Spectre/ROSEDUST gaps, chat entries, apps/) stands.

The TUI is real, large, and interactive: **10 F-key tabs (F1–F10) with 29 sub-views**, launched by `roko dashboard` (TTY) or `roko serve --tui` (StateHub-connected). CLAUDE.md's "F1–F7 tabs" is stale. Data flows from a notify-based `.roko/` watcher, a git watcher, JSONL tailers, a sidecar WebSocket client, an HTTP topology fetcher, and (in connected mode) a live StateHub — a deliberate mixed push/file model. Interactivity goes beyond navigation: approval modals wired to the orchestrator over IPC, signal injection, config editing, pause, filters, mouse hit-testing. The count "29" coincidentally matches the designed "29 screens across 6 regions" (v1 + v2-depth docs), but the *content* delta is significant: roughly half the designed screens (Daimon state, predictions, tool trace, chat-in-TUI, Gantt timeline, pheromone landscape, Spectre gallery, autonomy controls…) don't exist, while four whole implemented tabs (Git, Marketplace, Atelier, Learning) were never in the design. ROSEDUST exists as a palette but diverges from documented tokens; **Spectre is absent from the TUI** and lives instead as a React `SpectreAvatar` in demo-app. A second, legacy 6.4K-LOC text dashboard (`dashboard.rs` `DashboardScaffold`, 16 pages incl. a Dreams page) coexists with the tabbed TUI; the `widgets/dream_view.rs` widget is declared but never called. Demo is a five-headed surface: the live React demo-app (real routes on :6677, several broken endpoints), static demo-web (🕰️), CLI commands (`roko demo setup|warm`, `roko bench demo`, `roko dev`), shell scenario packs (`demo/demo-resources`), and a pile of tmp pitch/redesign material. Chat has four entry paths, all real. `apps/` holds three compiling workspace crates including mirage-rs with its own static web console.

## Runner v2 + the push/pull data model (the panel-drift story, verified)

`tmp/tmp-feedback/2/09-TUI-PANELS-BROKEN.md` and `25-TUI-AGENT-DATA-GAPS.md` document a live `roko plan run` where Output = "no agent output yet", Efficiency = all zeros, Diagnosis empty, Agent task column = "-", progress = "0k/200k". Re-verified against HEAD `5852c93c0` — the **struct-level gaps those docs describe are now closed, but the closure lives in Runner v2, so the docs' file:line references are stale**:

- **`AgentState` now carries the missing fields** (`dashboard_snapshot.rs:271-286`): `input_tokens`, `output_tokens`, `cost_usd`, `current_task`, `current_plan` — all `#[serde(default)]`. Doc 25's "Fix 1/Fix 2" (add these) is effectively done.
- **`current_task`/`current_plan` are set** when a `TaskStarted` event lands, by matching the agent-id prefix `"{plan_id}:"` (`dashboard_snapshot.rs:952-959`). Answers doc 09 §4 ("Agent Task Column shows -").
- **Per-agent token/cost accrual** happens in the `EfficiencyEvent` handler by metric name `input_tokens|output_tokens|cost_usd` (`dashboard_snapshot.rs:1092-1129`), also bumping `stats.cost_usd_total`. Answers doc 09 §2 / §5.
- **`task_outputs: HashMap<task_id, VecDeque<String>>`** field + `TaskOutputAppended` event exist (`dashboard_snapshot.rs:799, 1157-1165`); **`AgentOutput { agent_id, content }`** event increments `output_bytes` (`:1046-1050`). Answers doc 09 §1.
- **`Diagnosis { summary: DiagnosisSummary }`** event + a 50-deep `diagnoses` ring exist (`:80-84, 768-770, 1130-1132`). Answers doc 09 §3.

**Who publishes them (push mode).** `runner/tui_bridge.rs` (210 LOC) is the push seam; it emits **every** relevant variant: `agent_output` (`:83`), `efficiency_event` (`:117`), `agent_spawned` (`:74`), `task_started` (`:39`), `gate_result`, `diagnosis`-adjacent event-log entries, `cfactor_trend_updated`, `efficiency_trend_updated`, plus `model_selected`/`extension_hook`. `TuiBridge::new(state_hub.sender())` is constructed once per run at `runner/event_loop.rs:562` and threaded through ~15 call sites. Verified live call sites: `event_loop.rs:4557-4558` fires `agent_spawned` + `task_started` at dispatch; `agent_events.rs:71` fires `agent_output` on each `AgentEvent::MessageDelta`. **So push-mode now carries output + tokens + task assignment** — the core complaint of docs 09/25.

**Two `DashboardSnapshot` types now coexist** (naming collision, real code):
- `roko_core::dashboard_snapshot::DashboardSnapshot` — the rich materialized state (plans/tasks/agents/gates/diagnoses/episodes/trends/marketplace/atelier/knowledge), fed by `apply(&DashboardEvent)` in push mode and `load_from_workdir()` in pull mode.
- `runner::projection::DashboardSnapshot` — a **bounded ring of the last 200 `ProjectionEvent`s** (`runner/projection.rs:123-127`), tool output truncated to 4 KB, exposed via a `broadcast` channel + drop/coerce counters. This is the runner-local telemetry buffer, not the TUI's state.

**Pull mode** (standalone `roko dashboard`) still reads disk: `TuiState::from_dashboard_data()` (`tui/state.rs:1514`), and `DashboardSnapshot::load_from_workdir()` reads `state/executor.json`, `state/task-trackers.json`, **`engrams.jsonl` for gate verdicts**, `state/events.json`, `learn/experiments.json`, `learn/c-factor.jsonl`, then bootstraps episodes + efficiency from JSONL (`dashboard_snapshot.rs:1252-1303`). Runner v2 additionally persists an async unified `state/state-snapshot.json` (`runner/snapshot_writer.rs:15`) and appends every runner event to `state.events_jsonl` = **`.roko/events.jsonl`** (`runner/persist.rs:52,80,282`).

**Residual risk (why panels may still read empty at runtime).** The struct + publisher exist, but three things remain unverified end-to-end and are the live P0/P1s: (a) the **pull-mode gate source is `engrams.jsonl`**, which is stale on this workspace while verdicts flow through the runner's `events.jsonl`/signals path — a dashboard opened standalone will show old/empty gates (**split-brain**); (b) `EfficiencyEvent` per-agent attribution relies on `find_agent_key_for_task` which falls back to "only if exactly one active agent" (`:1244-1248`) — with parallel waves, token/cost can misattribute or drop; (c) the projection/ unification seam is new and its coverage across TUI vs HTTP vs CLI is not yet contract-tested. None of these are reflected in docs/v2-depth or CLAUDE.md.

## TUI screen census (vs 29 designed)

### Implemented: 10 tabs × 29 sub-views (interactive TUI)

Tab enum `crates/roko-cli/src/tui/tabs.rs:10-31` (F1–F10); sub-views `crates/roko-cli/src/tui/views/mod.rs:51-176`; render dispatch `views/mod.rs:273-295`. Number keys 1–9 select sub-views within a tab (`views/mod.rs:7-12`).

| Tab (F-key) | Sub-views (SubView enum) | Renderer |
|---|---|---|
| F1 Dashboard | Health, Mesh, Cost | `views/dashboard_view.rs` (2,294 LOC) |
| F2 Plans | DAG, Task, Waves | `views/plans_view.rs` (1,377) |
| F3 Agents | Output, Gates, Tokens | `views/agents_view.rs` (1,251) |
| F4 Git | Branches, Commits, Worktrees | `views/git_view.rs` (789) |
| F5 Logs | Log, Signals | `views/logs_view.rs` |
| F6 Config | Config, Providers, Models | `views/config_view.rs` (721) |
| F7 Inspect | Overview, Engrams, Episodes, Knowledge | `views/context_view.rs` (1,110) |
| F8 Marketplace | Jobs, Detail, New Job | `views/marketplace_view.rs` (612; reads `.roko/jobs/*.json`, header line 5) |
| F9 Atelier | PRDs, Plans | `views/atelier_view.rs` |
| F10 Learning | Route, History, Efficiency | `views/learning_view.rs` |

Total: 3+3+3+3+2+3+4+3+2+3 = **29 sub-views** — numerically equal to the designed 29, structurally different.

### Designed: 29 screens / 6 regions — two variants

- **v1**: `docs/v1/12-interfaces/09-tui-29-screens.md:22-53` (self-labeled "Implementation: Scaffold", line 6). Regions: Navigation(6): Agent List, Plan List, Mesh Status, Knowledge Browser, Episode Timeline, Settings · Agent Detail(6): Output Stream, Gate Results, **Daimon State**, **Prediction Dashboard**, **Tool Trace**, Cost Breakdown · Plan Detail(5): DAG View, Task Detail, **Merge Queue**, **Timeline**, Worktree Status · Knowledge(4): **Neuro Explorer**, **Tier Progression**, **Cross-Domain Map**, **Knowledge Graph** · Collective(4): **C-Factor Dashboard**, **Agent Comparison**, **Pheromone Landscape**, **Stigmergy Map** · System(4): Provider Health, **Resource Monitor**, Event Log, **Spectre Gallery**. Navigation model: number keys 1–6 for regions + Tab/Shift-Tab within (09:16).
- **v2-depth**: `docs/v2-depth/16-surfaces/03-tui-screen-architecture.md:61-123` revises the list (adds **Chat** 2.6, **Artifacts** 3.3, **Dream History** 4.4, **Resonance Graph** 4.2, **Lineage Walker** 4.3, **Autonomy Controls** 6.4, **Coordination Map** 5.2) and prescribes StateHub projections per screen plus an Elm-architecture `Model`/`Message` pattern (03:126-150) and F1–F7 region keys (03:11).

### Delta

- **Designed → implemented (some form)**: Agent List/Output/Gates/Cost (F3), Plan List/DAG/Task (F2), Mesh Status (F1.2), Knowledge Browser (F7.4), Episode Timeline (F7.3), Settings/Config (F6.1), Provider Health (F6.2), Worktrees (F4.3), Event Log (F5). C-Factor breakdown exists only in the **text** dashboard Health page (`commands/dashboard.rs:188-253`), not as an interactive screen.
- **Designed, ❌ not implemented**: Daimon State, Prediction Dashboard, Tool Trace, Chat-in-TUI, Merge Queue, plan Timeline/Gantt, Artifacts, Neuro tier progression, Cross-Domain Map, Knowledge/Resonance Graph, Lineage Walker, interactive Dream History, C-Factor Dashboard (interactive), Agent Comparison, Pheromone Landscape, Stigmergy/Coordination Map, Spectre Gallery, Resource Monitor (partial — `widgets/sys_metrics.rs` + bg collector `app.rs:760,3322` feed the header, no dedicated screen), Autonomy Controls.
- **Implemented, never designed in the 29**: entire Git tab, Marketplace tab, Atelier tab, Learning tab, Signal stream sub-view, Model Comparison.
- **Navigation mismatch**: docs say 6 regions/F1–F7/Tab-cycling + TEA; code uses 10 tabs/F1–F10 + number-key sub-views + `TuiState` (no TEA `Model`/`Message` naming). v2-depth 04 cites `tui/color.rs` (`04-rosedust-and-spectre.md:70`) — file doesn't exist (actuals: `theme.rs`, `widgets/rosedust.rs`).

### Entry points, data sources, interactivity

- **Entry**: `roko dashboard` → `commands/dashboard.rs:7-46`; TTY → interactive `App::new_with_page` / `App::new_connected_with_page`, else text fallback (`render_dashboard_text`, scaffold pages). `roko serve --tui` starts the HTTP server in background and a StateHub-connected TUI (`main.rs:2508-2560`). No feature flags.
- **Data sources (mixed push/file)**: `.roko/` recursive notify watcher, 200 ms debounce + 1 s poll fallback (`tui/fs_watch.rs:19-23,89,117-138`); git watcher (`tui/git_watch.rs`, wired `app.rs:783,2951`); JSONL tailers/cursors (`jsonl_tailer.rs`, `jsonl_cursor.rs`, `task_outputs.rs` tails `.roko/task-outputs/`); WebSocket client to the **per-agent sidecar** `/stream`+`/ws` (`tui/ws_client.rs:269-276,388`, sidecar frames `ws_client.rs:304`); HTTP agent-topology fetch thread (`app.rs:3041-3058`); background sysinfo thread (`app.rs:760`, `collect_sys_metrics_bg` `app.rs:3322`); StateHub snapshot receiver in connected mode (`app.rs:131,570`); orchestrator approval IPC (`tui/approval_ipc.rs:1-5`); durable dashboard generation counter `.roko/state/dashboard-gen.json` (`tui/dashboard_gen.rs:1-4`); store-backed gate verdict aggregation (`tui/verdicts.rs:1`).
- **Interactivity** (✅, beyond navigation): approval modal — `y`/Enter approve, `A`/Ctrl-A approve-all (`tui/input.rs:470-475`, answered back over IPC oneshot); signal injection mode (`input.rs:241-243,546-547`, `modals/inject.rs`); config edit mode (`input.rs:305-306,414`); pause toggle (`input.rs:320`); log filter (`input.rs:327`); mouse via `hit_test.rs`; modal suite (13 files in `tui/modals/`: task/plan detail, batch review, wave/queue overview, agent pool, help, quit…).
- **Effects**: atmosphere animations + PostFX pipeline with Off/Minimal/Full presets, default **Off**, runtime-toggleable (`tui/effects_config.rs:7-15`, `app.rs:833,927-950,1223`).
- **Tests**: 236 `#[test]` occurrences under `crates/roko-cli/src/tui/` (grep count); e.g. `tabs.rs:181-212`, StateHub-connected update test `app.rs:3651`.

### Built-not-wired & legacy inside the TUI

- 🔌 **`widgets/dream_view.rs`** — declared at `widgets/mod.rs:8`, **zero `dream_view::` call sites** anywhere in the crate (verified grep). Confirms the dreams-audit claim.
- 🕰️ **Legacy text scaffold `tui/dashboard.rs` (6,373 LOC)** — a second dashboard: 16 `PageId`s (`tui/pages/mod.rs:13-46`) incl. `Dreams`, rendered as *plain text* via `roko dashboard --page <slug>` fallback (`commands/dashboard.rs:43-88`). Its Dreams page reads `.roko/dreams/journal.jsonl` + `archive.jsonl` (`dashboard.rs:4619-4622`); a `dream_phase` widget lives in `pages/operations.rs:140`. So dreams are visible in the text dashboard but **not** in the interactive tabbed TUI (no dreams SubView). Two parallel dashboard code paths persist.
- 🟡 **ROSEDUST**: implemented as canonical `Theme` (`tui/theme.rs:66` "ROSEDUST palette — warm rose/indigo aesthetic from Mori's design system") with no-color and high-contrast variants (`theme.rs:106-119`), plus OKLab-free gradient helpers (`gradient_fire`, `gradient_ocean`, `brighten`) re-exported through the 9-line compat shim `widgets/rosedust.rs` (also aliases `MoriTheme`). But token values diverge from the spec: `ROSE = rgb(185,120,148)` (#B97894) vs doc `rose #c77d8f`; `VOID/BG = rgb(0,0,0)` while the doc mandates "Never use pure #000000" (`docs/v2-depth/16-surfaces/04-rosedust-and-spectre.md:25-29,35-38`); no jade/amber/crimson/sapphire token names (SAGE/EMBER/WARNING/DREAM instead, `theme.rs:48-51`); no OKLab interpolation or APCA contrast machinery (04:66-68).
- ❌ **Spectre in TUI**: designed across `docs/v1/12-interfaces/10-spectre-creature-visualization.md`, `11-spectre-rendering-per-interface.md`, `12-spectre-as-collective-display.md` and v2-depth 04 (BLAKE3 shape seed, 8 archetypes, PAD-driven animation, frame budget reserves 1.5 ms/frame for "Spectre rasterization", `03-tui-screen-architecture.md:51`) — zero spectre code in `crates/`. It *does* exist in React: `demo/demo-app/src/components/Spectre/SpectreAvatar.tsx` + `AgentIdentity.ts` (archetypes, `ROLE_PALETTES`), used by Terminal panes (`components/Terminal/TerminalPane.tsx:3-5,62`).

## Storage split-brain (triple-brain) — why dashboards read empty

Four overlapping `.roko/` logs now record overlapping facts; dashboards and the runner disagree on which is canonical:

| Log | Written by | Read by | On-disk now |
|---|---|---|---|
| `.roko/engrams.jsonl` | legacy signal/engram writer (`workspace.rs:269`) | **dashboard pull-mode gate verdicts** (`dashboard_snapshot.rs:1276`, `:2891`) | 10 KB, **stale May 6** |
| `.roko/signals.jsonl` | signal substrate (`workspace.rs:167`) | signal DAG / `roko replay` / status | 80 KB, May 8 |
| `.roko/events.jsonl` | **Runner v2** append-only event log (`runner/persist.rs:52,282`) | TUI/server event consumers | **44 MB, May 9** |
| `.roko/state/state-snapshot.json` | Runner v2 async `snapshot_writer.rs` | unified pull-mode snapshot | present |

The **consequence**: a standalone `roko dashboard` (pull-mode) sources gate verdicts from `engrams.jsonl`, but Runner v2 writes verdicts through gate-dispatch into the runner event/signal path, not `engrams.jsonl`. So the Gates/Health panels can show old or empty data even though a run just completed. This is the mechanical root of "panels empty" beyond the push-mode gaps. **No doc reconciles these four; there is no single `engrams.jsonl`-vs-`events.jsonl` owner decision recorded.**

## Demo reality

### `crates/roko-demo` — the compiled scenario orchestrator (✅ NEW, undocumented)

A workspace-member Rust crate (`Cargo.toml:71`) with a `roko-demo` binary. **This did not exist / was explicitly denied ("there is no `crates/roko-demo` crate") in the 07-07 draft — it is new.** It supersedes the `bin/roko-demo` shell script and the `demo/demo-old` scenario paradigm by compiling them into typed Rust. Structure (21 rs files):

- **CLI** (`src/main.rs`): subcommands `up` (deploy+seed+agents), `deploy`, `seed`, `verify`, `benchmark c-factor`, `tournament`, `autonomous`, **`tui`**, `register-agent`, `list`. Flags for `--llm-backend` (default `stub`), `--events none|ws|both`, `--ws-port 9090`, `--persist-reputation`.
- **Own ratatui TUI** (`src/tui.rs`): a `DemoEvent`-driven live view over an mpsc channel — panels for agents (model/reputation/earned/status), knowledge (insight count + latest), economics (distributed/treasury), a scrollable log, and current round/winner. Independent of the roko-cli TUI (`tui/`); this is a **separate ratatui app** for chain-demo runs.
- **Own WebSocket broadcast server** (`src/ws_server.rs`): binds `127.0.0.1:9090`, `tokio-tungstenite`, broadcasts serialized `DemoEvent`s to connected clients. Drives external dashboards during a demo.
- **Chain integration** (`src/chain_ctx.rs`, `src/bindings.rs`, `src/deploy.rs`): alloy `MockERC20`/`AgentRegistry`/`WorkerRegistry` bindings, deploys against a mirage-rs fork, writes `demo/.runtime/deployments.json`.
- **Scenarios** (`src/scenarios/`): `yield_routing`, `defi_routing`, `job_board`, `consortium`, `flywheel`, `llm` — the chain-era scenario set, now compiled + parameterized by a `manifest.toml`.
- **Benchmark/tournament/autonomous** (`benchmark.rs`, `tournament.rs`, `autonomous.rs`): C-factor cold-vs-warm measurement, multi-round tournaments, and an autonomous poster/agent loop.

**Open questions this raises**: (1) is `roko-demo` (chain scenarios, mirage fork, ERC-8004 registries) still v2 product scope or a chain-era resurrection given the chain pivot? (2) it duplicates a ratatui TUI and a WS server that roko-cli/roko-serve already provide — intentional isolation or drift? (3) it depends on `roko-chain` with `alloy-backend` + `alloy = "1"`, tying the demo to the 1.91+ rustc blocker.

### CLI demo commands (✅)

- `roko demo setup|warm` — top-level command (`main.rs:2458-2474`): `setup` = binary build + workspace check, `warm` = LLM cache pre-population (`crates/roko-cli/src/demo_cmd.rs:19,110`). There is **no `crates/roko-demo` crate** (CLAUDE.md doesn't list one; `ls crates/` confirms).
- `roko bench demo [--real]` — simulated-vs-real benchmark demo (`main.rs:1128-1140`).
- `roko init --demo` (`main.rs:3510`); `roko dev [--no-frontend]` — serve + `npm run dev` frontend with PID management (`commands/dev.rs:1-66`).

### demo/demo-app — the live React surface (🟡 real but drifting)

React+Vite+zustand, 515 src files, 15 Playwright e2e specs (`demo/demo-app/e2e/`). Routes (`src/main.tsx:96-121`): `/` Landing; `/dashboard` → cost (index), fleet, knowledge, integrity, entries, routing (CascadeRouter), dreams, feeds, relay; `/isfr`; `/demo`; `/terminal`; `/builder`; `/explorer`; `/settings`; `/bench` (+`run/:id`, `compare`); `/share(/:token)`. Backend: roko-serve at `http://localhost:6677` (`src/lib/serve-url.ts:21-27`), Mirage WS `ws://localhost:8545` (serve-url.ts:69-71).

- ✅ `DataHub.ts` central store exists (`src/app/DataHub.ts` + `bootstrap.ts`); many pages call real routes (cost, knowledge, bench, terminal, builder, relay).
- 🕰️ Deprecated providers **still wrap the app**: `WorkspaceProvider`, `RokoConfigProvider`, `EventStreamProvider` (`src/main.tsx:6-9,91-93`) — DataHub migration incomplete (see `tmp/status-quo/66-FRONTEND-API-PARITY.md:7-14`).
- 🕰️ `/demo` special case: `<Route path="demo" element={null} />` (`main.tsx:111`); the Demo page (scenario slots + xterm panes, `src/pages/Demo/`) is mounted keep-alive inside `src/components/AppShell.tsx`. Scenario progress partly derived from regex over terminal output (`pages/Demo/ScenarioSlot.tsx:875` `match(/WSDIR:(\S+)/)`).
- ❌ **Broken/unregistered endpoints** (verified: no matching route in `crates/roko-serve/src`): `/api/isfr/stream` (`src/lib/isfr-api.ts:113`), `/ws/agents` (`src/pages/isfr/IsfrTabDrawer.tsx:101`); plus from 66-FRONTEND-API-PARITY: `/api/share/{token}` vs server's `/api/shared/{token}`, `/api/bench/matrix` unmounted.
- 🟡 **Dreams page event mismatch** (verified): `DreamsView.tsx:72` subscribes to `dream_started|dream_completed|dream_phase_changed|knowledge_ingested|knowledge_consumed`; backend emits only operation kind `dream_run:{mode}` (`crates/roko-serve/src/routes/dream.rs:55`) — no emitter for those names anywhere in `crates/`. The data fetches work though: `/api/dream/journal` is a real route (`routes/dream.rs:20-21`).

### demo/demo-web (🕰️ static) & demo/demo-old (🕰️ pre-pivot)

- `demo-web/`: 8 hand-built HTML pages (index/terminal/builder/explorer/bench/bench-live/demo) with hardcoded values, plus `ISSUES.md` whose top item documents **3–15 s CLI startup latency** (full config+state bootstrap incl. reading entire `episodes.jsonl` on every invocation) — a real cross-surface perf bug.
- `demo-old/`: chain/marketplace-era scenario demo (manifest.toml, wallets.toml, scenarios: consortium, defi-routing, flywheel, job-board, yield-routing; agent prompt cards). Legacy paradigm.

### demo/demo-resources (✅ v2-path scripts) & demo/demo-research

11 scenario dirs (prd-workflow, research-workflow, agent-workflows, agent-setup, agent-matchmaking, chain-coordination, provider-routing, benchmark-flow, coding-agent-benchmarks, full-self-hosting, dashboard-quickstart) of shell scripts that call the **real CLI** (`roko prd`, `roko plan run`, `roko research`, serve APIs), orchestrated by `bin/roko-demo` (subcommands: list/doctor/build/serve/seed-agents/dashboard-smoke/verify-local/bench/run — `bin/roko-demo:10-25`) with `roko-up.sh`/`roko-down.sh`/`roko-smoke.sh`. These exercise current v2 paths, not legacy. `demo-research/`: 9 docs (benchmarks, frameworks, cost, eval harnesses, realtime viz, recipes, methodology, reuse map) — research input, not a surface.

### tmp demo material (🕰️ unless noted)

`tmp/demo-req/` (pitch strategy, DEMO-CONCEPTS, STATUS-AUDIT, FINAL-GAP-ANALYSIS, IMPLEMENTATION-PLAN, xterm guide — requirements-vs-reality source); `tmp/demo-uis/` (17 pitch HTML iterations v1–v17 + concept docs); `tmp/demo-new/` (00-INDEX/ARCHITECTURE/VISUAL-SYSTEM/DEMO-SCENARIOS redesign set); `tmp/demo-redesign/AUDIT.md`; `tmp/demo-current/` (built static site with `dist/`); `tmp/demo-app-backup/`; `tmp/demo-resources/` (tmp copy). `tmp/solutions/demo-running/` (May 5, waves A–E): STATUS.md verdict — "56 batches attempted, only 18 fully done… Demo app UX unchanged (still 14 old scenarios)… scenario redesign was never implemented… CLI still 35+ subcommands" — marked SUPERSEDED by `20-TMP-NEWEST.md:22`.

### UX problem backlog relevant to surfaces

`tmp/solutions/self-developing/` (May 6): 23 numbered problems, index status 14 Open / 4 Fixed / 5 Partial (`00-INDEX.md:7-31`). Owned by this doc per `68-SELF-DEVELOPING-CROSSWALK.md:18,30-31`: **#10** terminal output corruption (Open), **#22** plan-run TUI broken — silent exit, stale snapshot, hidden stderr (Partial; also 37-RUNNER-V2-AND-GRAPH), **#23** TUI plan-list scroll bug (Open).

## Chat & apps inventory

### Chat surfaces (all real)

| Entry | Path | Backend | Notes |
|---|---|---|---|
| bare `roko` (TTY, no subcommand) | `unified.rs` (336 LOC) → `cmd_unified_chat` (`main.rs:2054,2318`) | auto-detect (Claude CLI → API key), may spawn background serve | default UX |
| inline chat UI | `chat_inline.rs` (5,699 LOC) | `dispatch_v2` in-process | "Claude Code-like": ratatui viewport, streaming, multi-line, `/` commands, Ctrl-C, cost meter (`chat_inline.rs:1-7`); non-TTY falls back to line REPL |
| `roko agent chat --agent X` | `agent_serve.rs:131,718` → `chat.rs` (956 LOC) `run_chat_repl` | **sidecar-direct** (bind from `.roko/runtime/agents.json`, `chat.rs:76-93`) or **via roko-serve :6677**, with unreachable fallback (`chat.rs:54-61`); run-id polling via serve (`chat.rs:222-228`) | session summaries persisted via `chat_history.rs` (92 LOC) |
| session layer | `chat_session.rs` (2,988 LOC) | — | `ChatAgentSession` (cited by 20-TMP-NEWEST as the partial fix for solutions backlog) |

### apps/ (inventory only — all three are workspace members, root `Cargo.toml:33-37`, compile with the workspace)

- **`apps/mirage-rs`** — standalone EVM fork simulator with roko bridge (chain/HDC substrates, simulation gate, subscriptions), HTTP API (`src/http_api/`: agents, tasks, predictions, pheromones, knowledge, topology, **isfr.rs**, ws.rs) and a **static web console** (`static/index.html`, `demo-agents.html`, `js/` incl. agent_registry, pheromones, topology, charts; ERC-8004 registry fixtures). This is the "Mirage static console" surface.
- **`apps/agent-relay`** — relay bus/protocol/state + chain watcher (`src/{bus,protocol,state,chain_watcher}.rs`), the `/relay/*` backend consumed by demo-app RelayDashboard (proxied through roko-serve; canonical-owner question open in 66-FRONTEND-API-PARITY.md:25,44-45).
- **`apps/roko-chain-watcher`** — long-running agent observing a mirage-rs chain, posting insights (README; `src/{watcher,block_observer,reactions}.rs`).

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| **Runner v2 default `plan run`** | — (undocumented) | `runner/event_loop.rs:run`, 20 files/17K LOC | ✅ **NEW** | `commands/plan.rs:654` dispatches here |
| **Projection unification seam** | docs 09/25 (implicit) | `projection/{mod,cli_progress,dashboard}.rs` + `runner/projection.rs` | 🟡 new | `projection/mod.rs:1-18` names the drift it fixes |
| **TuiBridge push seam** | docs 09/25 | `runner/tui_bridge.rs` (210) | ✅ | emits output/efficiency/task-assignment `:74-183` |
| **Push-mode agent fields** (tokens/task/cost) | docs 09/25 "broken" | `dashboard_snapshot.rs:271-286, 952-959, 1092-1129` | ✅ struct done | end-to-end runtime unverified (parallel attribution risk) |
| **`crates/roko-demo` chain orchestrator** | — (undocumented) | 21 rs files, own ratatui TUI + WS:9090 | ✅ builds **NEW** | `Cargo.toml:71`; `src/{tui,ws_server,main}.rs` |
| **Storage split-brain (4 logs)** | — | `engrams.jsonl`/`signals.jsonl`/`events.jsonl`/`state-snapshot.json` | 🟡 drift | dashboard reads stale `engrams.jsonl` (`:1276`) |
| TUI shell (tabs, 60fps loop, modals) | v1 08/09, v2-depth 16/03 | `tui/app.rs` (4,491), `tabs.rs`, `views/`, `modals/` | ✅ | `tabs.rs:10-31`; `views/mod.rs:273-295` |
| 29 designed screens | 09-tui-29-screens.md:22-53; v2-depth 03:61-123 | 29 sub-views, different set | 🟡 ~half | delta section above |
| `roko dashboard` entry + text fallback | 12-interfaces/00-01 | `commands/dashboard.rs:7-46` | ✅ | TTY→App, else scaffold text |
| `roko serve --tui` (StateHub-connected) | 22-statehub-projection.md | `main.rs:2508-2560`; `app.rs:131,570` | ✅ | connected snapshot rx test `app.rs:3651` |
| fs/git watchers | CLAUDE.md ("notify watcher") | `tui/fs_watch.rs:19-23,89`; `git_watch.rs` | ✅ | 200ms debounce + poll fallback |
| Sidecar WS stream + topology fetch | 06-websocket-streaming.md | `tui/ws_client.rs:269-276,388`; `app.rs:3041` | ✅ | parses sidecar frames |
| Approval/inject/config-edit interactivity | 21-user-ux (REF23 verbs) | `tui/input.rs:236-547`; `approval_ipc.rs` | ✅ | verb coverage still partial vs REF23 |
| ROSEDUST palette | 07-rosedust; v2-depth 04:17-70 | `tui/theme.rs:34-119`; `widgets/rosedust.rs` (shim) | 🟡 | hexes ≠ doc tokens; pure-black BG violates spec; no OKLab/APCA |
| Spectre (TUI creature) | v1 10/11/12-spectre; v2-depth 04:74-120 | none in crates/ | ❌ | grep: zero rust hits |
| SpectreAvatar (React) | 11-spectre-rendering-per-interface | `demo-app/src/components/Spectre/` | ✅ | used in Terminal panes |
| PostFX/atmosphere effects | — (not designed) | `effects_config.rs`, `postfx*.rs`, `atmosphere.rs` | ✅ (default Off) | `app.rs:927-950,1223` |
| Dream view widget | v2-depth 03 (4.4 Dream History) | `tui/widgets/dream_view.rs` | 🔌 | declared `widgets/mod.rs:8`; zero call sites |
| Legacy text dashboard (16 pages incl Dreams) | — | `tui/dashboard.rs` (6,373); `pages/mod.rs:13-46` | 🕰️ ✅(text) | Dreams reads `.roko/dreams/*.jsonl` (`dashboard.rs:4619-22`) |
| TEA architecture | v2-depth 03:126-150 | `TuiState`/`TuiAction` instead | ❌ (as specced) | naming/structure differ |
| `roko demo setup/warm` | — | `demo_cmd.rs:19,110`; `main.rs:2458-74` | ✅ | |
| `roko bench demo [--real]` | demo-research | `main.rs:1128-40`; `commands/bench.rs` | ✅ | |
| `roko dev` (serve+vite) | — | `commands/dev.rs` | ✅ | PID files, SIGINT/SIGTERM |
| demo-app routes/pages | tmp/demo-req, tmp/demo-new | `src/main.tsx:96-121` | ✅ | 16+ routes live |
| DataHub migration | 66-FRONTEND-API-PARITY | `src/app/DataHub.ts` | 🟡 | deprecated providers wrap `main.tsx:91-93` |
| ISFR stream/WS endpoints | daiji isfr-implementation-guide | `lib/isfr-api.ts:113`; `IsfrTabDrawer.tsx:101` | ❌ broken | no serve route registered |
| Dreams page live events | — | `DreamsView.tsx:72` vs `routes/dream.rs:55` | 🟡 | fetch works; event names never emitted |
| demo-web static pages | — | `demo/demo-web/*.html` | 🕰️ | ISSUES.md documents 3-15s CLI latency |
| demo-old scenarios | chain-era | `demo/demo-old/` | 🕰️ | pre-pivot marketplace demo |
| demo-resources scripts | — | `demo/demo-resources/` (11 dirs + bin/roko-demo) | ✅ | drives real v2 CLI |
| Chat (4 entries) | 12-interfaces/21 | `unified.rs`, `chat_inline.rs`, `chat.rs`, `chat_session.rs` | ✅ | table above |
| mirage-rs + static console | 08-chain docs | `apps/mirage-rs/` | ✅ builds | workspace member :33 |
| agent-relay | daiji pr24-review | `apps/agent-relay/` | ✅ builds | workspace member :34 |
| roko-chain-watcher | — | `apps/roko-chain-watcher/` | ✅ builds | workspace member :37 |
| Web portal / A2UI / sonification | v1 13/15/16 | none | ❌ | aspirational docs only |

## V2-aligned

- Tabbed TUI with sub-view regions, StateHub-connected mode, immediate-mode 60fps loop — matches v2-depth 03's Lens-Cell/Observe framing in substance (read-only projections, keyboard routing) if not in naming.
- Marketplace tab reading `.roko/jobs/` and pointing users at `POST :6677/api/jobs` (`marketplace_view.rs:5,580`) — file-substrate + serve dual access is the intended pattern.
- demo-resources scenario packs exercise the real self-hosting loop (`prd → plan → run → dashboard`), aligned with M0-C "truthful demo surface" direction (`tmp/solutions/REVISED-BEST-SOLUTION-AFTER-DEMO.md` via 20-TMP-NEWEST:43).
- Chat consolidation onto `dispatch_v2`/sidecar/serve rather than bespoke loops.

## Old paradigm & tech debt

- **Two dashboards**: legacy `DashboardScaffold` text pages (6.4K LOC) vs tabbed views — duplicate render paths for plans/logs/config/providers/models; Dreams exists only on the legacy side.
- **Mori residue**: `rosedust.rs` shim aliasing `MoriTheme` "for one commit cycle"; tabs.rs headed "Mori-style TUI".
- **Mixed data plumbing**: fs-watch + tailers + WS + HTTP threads + StateHub — works, but per-tab source is inconsistent and some config reads happen at render time (draft claim; consistent with `config_meta.rs` 831 LOC).
- **demo-app**: deprecated providers, `/demo` keep-alive special case, regex-derived scenario progress, endpoint string drift (no route contract), page-level raw WebSockets (66-FRONTEND-API-PARITY.md:14).
- **CLI startup latency 3–15 s** hits every surface that shells out to `roko` (demo-web ISSUES.md) — includes demo scenarios and `roko dev` flows.
- **Stale docs**: CLAUDE.md "F1–F7"; v2-depth cites nonexistent `tui/color.rs`; v1 09 cites `views/agents.rs`/`widgets/agent_grid.rs` (nonexistent names).
- 🕰️ pile: demo-web, demo-old, tmp/demo-uis (17 pitch HTMLs), tmp/demo-current, tmp/demo-app-backup, tmp/solutions/demo-running (superseded).

## Not implemented (rosedust gaps, spectre, missing screens)

- **Spectre**: no TUI creature, no gallery, no PAD-driven animation, no collective display (v1 docs 10/11/12 fully unimplemented in Rust; only the React avatar approximates archetype identity).
- **ROSEDUST full spec**: OKLab gradient interpolation, APCA contrast verification, documented token values (void-black/twilight/dusk, jade/amber/crimson/violet/sapphire), light variant.
- **Screens**: Daimon State, Predictions, Tool Trace, Chat-in-TUI, Merge Queue, Gantt Timeline, Artifacts, Neuro Explorer/Tier Progression, Knowledge/Resonance Graph, Cross-Domain Map, Lineage Walker, interactive Dream History, interactive C-Factor Dashboard, Agent Comparison, Pheromone Landscape, Stigmergy/Coordination Map, Autonomy Controls, Resource Monitor screen.
- **REF23 unified verbs in TUI** (`08-tui-main-layout.md:24-40`): `ask`, `replay`, `learn` curation, `tune` forms, `connect` — not exposed as TUI actions.
- **Other designed surfaces**: web portal (13), generative interfaces/A2UI (15), sonification (16), agent onboarding flow (14).

## Migration checklist

- [ ] **[P0]** Reconcile the storage split-brain: pick one canonical gate-verdict source and point pull-mode dashboard at it. Today `load_from_workdir` reads `engrams.jsonl` (`dashboard_snapshot.rs:1276`) while Runner v2 writes to `events.jsonl` + the signal path — standalone `roko dashboard` shows stale/empty gates — verify: `rg engrams_path crates/roko-core` and confirm the same file a completed `plan run` wrote verdicts to; a fresh run then shows current gates in standalone dashboard
- [ ] **[P0]** Verify Runner v2 push-mode end-to-end: `cargo run -p roko-cli -- plan run plans/` with `roko dashboard` open must fill Output / Efficiency / Agent-task / progress panels (the struct + `TuiBridge` publishers exist — `runner/tui_bridge.rs:74-183` — but no live confirmation). Watch parallel-wave token attribution (`find_agent_key_for_task` single-agent fallback, `dashboard_snapshot.rs:1244-1248`)
- [ ] **[P1]** Decide `crates/roko-demo` scope: is the compiled chain-scenario orchestrator (mirage fork, ERC-8004, own :9090 WS + ratatui TUI) v2 product or chain-era resurrection? De-dupe its TUI/WS against roko-cli/roko-serve or document it as an isolated harness — verify: decision in docs INDEX + GAPS.md; `roko-demo list` runs
- [ ] **[P0]** Fix broken demo-app endpoints: register or repoint `/api/isfr/stream`, `/ws/agents`, `/api/share/{token}`→`/api/shared/{token}`, `/api/bench/matrix` — verify: `rg "isfr/stream|ws/agents" crates/roko-serve/src` finds routes, then load `/isfr` + `/share` pages with `roko serve` running (no 404/WS close)
- [ ] **[P0]** Close self-developing #22 (plan-run TUI silent exit: stale snapshot, hidden stderr, Graph default) — verify: `cargo run -p roko-cli -- plan run plans/` with `roko dashboard` open shows live progress or a visible error
- [ ] **[P1]** Add route contract test (extract serve routes vs frontend `fetch`/`EventSource`/`WebSocket` strings; fail on unowned paths) — verify: script exists per `66-FRONTEND-API-PARITY.md:49-57` and runs in CI
- [ ] **[P1]** Emit dream lifecycle events (`dream_started`/`dream_completed`/`dream_phase_changed`) from serve dream_run or change `DreamsView.tsx:72` to consume operation events — verify: `rg dream_started crates/roko-serve` non-empty + Dreams page updates during `POST /api/dream/run`
- [ ] **[P1]** Wire `widgets/dream_view.rs` into F7 Inspect (or a Dreams sub-view) or delete it; fold legacy `PageId::Dreams` data path into the tabbed TUI — verify: `rg "dream_view::" crates/roko-cli/src` non-empty and a Dreams screen reachable via keys in `roko dashboard`
- [ ] **[P1]** Finish DataHub migration; remove `WorkspaceProvider`/`RokoConfigProvider`/`EventStreamProvider` — verify: `rg "WorkspaceProvider|RokoConfigProvider|EventStreamProvider" demo/demo-app/src/main.tsx` empty; e2e suite green
- [ ] **[P1]** Fix TUI plan-list scroll (#23) and terminal output corruption (#10) — verify: repro steps in `tmp/solutions/self-developing/{23,10}-*.md` no longer reproduce
- [ ] **[P2]** Reconcile screen design vs implementation: either implement priority missing screens (Daimon State, Predictions, Chat-in-TUI, interactive C-Factor/Dreams) or update `docs/v2-depth/16-surfaces/03` + v1 09 to the shipped 10-tab/29-sub-view model — verify: doc screen table matches `SubView::for_tab` (`views/mod.rs:134-176`)
- [ ] **[P2]** ROSEDUST token parity: align `theme.rs` values with doc tokens (or update docs), remove `MoriTheme` shim after its "one commit cycle" — verify: `rg MoriTheme crates/` empty; palette test `dashboard.rs:5315` asserts doc hexes
- [ ] **[P2]** Decide Spectre scope: implement TUI Spectre gallery per v2-depth 04, or descope and mark docs aspirational (React avatar as the only renderer) — verify: decision recorded in docs INDEX + GAPS.md
- [ ] **[P2]** De-duplicate the two dashboards: make legacy `DashboardScaffold` pages thin wrappers over view renderers or CLI-only reports — verify: no duplicated plans/logs/config rendering logic between `dashboard.rs` and `views/`
- [ ] **[P2]** Attack CLI startup latency (lazy episode load, cached config) — verify: `time roko status` < 1 s warm (demo-web ISSUES.md baseline 3–15 s)
- [ ] **[P3]** Archive/label 🕰️ material: `demo/demo-web`, `demo/demo-old`, `tmp/demo-uis`, `tmp/demo-current`, `tmp/demo-app-backup`, `tmp/solutions/demo-running` — verify: README/ARCHIVED markers present
- [ ] **[P3]** Update CLAUDE.md TUI row (F1–F10 tabs, `roko serve --tui`, demo commands) — verify: CLAUDE.md matches `tabs.rs` and `main.rs:2508`
- [ ] **[P3]** Expose REF23 unified verbs (`ask`, `replay`, `tune`, `connect`) as TUI keyboard actions — verify: help modal lists them; actions dispatch

## Open questions

1. **Which dashboard survives?** The tabbed TUI and the 16-page text scaffold overlap heavily; is the text scaffold a deliberate non-TTY/API surface (it also feeds StateHub bootstrap) or leftover to fold in?
2. **Marketplace/Atelier tabs**: Marketplace (`.roko/jobs/`) predates the chain pivot decisions — is a jobs marketplace still v2 product scope, or should F8 become one of the missing designed screens?
3. **Canonical chat entry**: bare `roko` unified chat vs `roko agent chat` vs ACP — three UX stacks (`chat_inline`, `chat.rs` REPL, ACP) with different capabilities (see self-developing #19/#20); which is the convergence target?
4. **Relay route ownership**: serve proxy vs direct `agent-relay` for `/relay/*` in demo-app (66-FRONTEND-API-PARITY.md:25,44-45) — pick one before contract tests.
5. **Where do Spectre/ROSEDUST specs live going forward** — docs/v3 convergence never ran (20-TMP-NEWEST:10); v1 and v2-depth screen lists disagree with each other *and* the code; which is authoritative for the census?
6. **Is `tmp/demo-req` still the demo requirements source** for M0-C "truthful demo surface", or does `tmp/demo-new/03-DEMO-SCENARIOS.md` supersede it? (demo-running says the scenario redesign never shipped.)
7. **Runner v1 → v2**: is `orchestrate.rs`'s `PlanRunner` now dead/legacy given `plan run` routes to `runner::event_loop::run` (`plan.rs:654`), or does it still back `roko run`/`do`/serve? CLAUDE.md's entire "Wired" table cites orchestrate.rs as the runtime — needs a runner-v1-vs-v2 ownership decision recorded.
8. **Canonical event log**: `engrams.jsonl` vs `signals.jsonl` vs `events.jsonl` vs `state-snapshot.json` — four overlapping stores, dashboard reads the stalest. Which is authoritative for gate verdicts, and should `load_from_workdir` repoint off `engrams.jsonl`?
9. **`roko-demo` vs roko-cli/roko-serve**: two ratatui TUIs and two WS servers (:9090 vs sidecar/serve) now exist for demos — converge or keep the chain harness isolated?
