# Full System Audit: CLI, TUI, Event Pipeline, Subsystems

Comprehensive audit of everything disconnected, stubbed, or broken across the
roko CLI, TUI dashboard, event pipeline, and orchestration subsystems.

## Executive Summary

| Area | Verdict |
|------|---------|
| CLI Commands (45) | **100% working** — no stubs, no dead code |
| TUI Panels (~30) | **~75% working** — 5 panels broken in push mode, 4 stubs |
| Event Pipeline (52 call sites) | **~80% wired** — 5 critical event types never published |
| DashboardSnapshot (22 fields) | **12 gaps** — data in pull mode that drops in push mode |
| orchestrate.rs (19 subsystems) | **17/19 wired** — VCG unused, safety permissive fallback |

**Root cause of most issues**: Push mode (during `roko plan run`) uses `DashboardSnapshot`
which doesn't carry efficiency events, agent output, git diff, or conductor diagnoses.
Pull mode (standalone `roko dashboard`) reads these from disk and works fine.

---

## 1. CLI Commands: All Working

All 45 CLI commands are fully implemented. No `todo!()`, `unimplemented!()`, or stub functions.

| Category | Commands | Status |
|----------|----------|--------|
| Core workflow | init, do, develop, run, status, show, doctor, setup, layer-check | WORKING |
| Planning | plan (list/show/create/validate/run/generate/regenerate), prd (idea/list/status/draft/plan/consolidate) | WORKING |
| Agents | agent (create/start/stop/list/status/serve/chat) | WORKING |
| Research | research (topic/search/enhance-prd/plan/tasks), think, note | WORKING |
| Knowledge | knowledge (query/stats/gc/backup/restore/sync/dream/custody/archive) | WORKING |
| Learning | learn (all/router/experiments/efficiency/episodes), tune | WORKING |
| Jobs | job (list/create/match/show/execute/cancel) | WORKING |
| Config | config (15+ subcommands including providers/models/subscriptions/plugins/secrets/mcp) | WORKING |
| Server | dev, up, serve, acp, daemon (8 subcommands), deploy (railway/fly/docker), worker | WORKING |
| Interactive | dashboard, vision-loop | WORKING |
| Auth | login, logout, whoami | WORKING |
| Utilities | resume, replay, history, inject, completions, new (9 scaffold types), explain | WORKING |
| Code intel | index (build/rebuild/search/stats) | WORKING |
| Graph | graph (run/validate/show) | WORKING |
| Other | bench (demo/swe), demo (setup/cleanup/seed/status), feed (list/status), isfr (start/status/sources) | WORKING |

**Key finding**: The "built but not wired" pattern from CLAUDE.md does NOT apply to the CLI layer.
All commands call real backend implementations with proper error handling.

---

## 2. TUI Dashboard: Tab-by-Tab Audit

### F1: Dashboard (Main View)

| Panel | Push Mode | Pull Mode | Issue |
|-------|-----------|-----------|-------|
| Plans (left) | WORKING | WORKING | — |
| Phase pipeline | WORKING | WORKING | — |
| Task progress | WORKING | WORKING | — |
| **Agents** | **PARTIAL** | WORKING | Task column "-", progress "0k/200k" (BUG #08) |
| **Output** | **BROKEN** | WORKING | "no agent output yet" — `agent_output_tail` not in snapshot |
| **Diff** | **BROKEN** | WORKING | `git_diff` not in snapshot |
| Verify (gates) | WORKING | WORKING | — |
| **Git** | **STUB** | **STUB** | Data struct never populated; falls back to live git commands |
| **MCP** | **STUB** | **STUB** | No data source at all |
| Learning | WORKING | WORKING | — |
| **Procs** | **STUB** | **STUB** | No ProcessSupervisor integration |
| Wave progress (bottom) | WORKING | WORKING | — |
| Token sparkline | WORKING | WORKING | — |
| System metrics | WORKING | WORKING | Independent background thread |

### F2: Plans

| Panel | Push Mode | Pull Mode | Issue |
|-------|-----------|-----------|-------|
| Plan tree | WORKING | WORKING | — |
| **Plan summary** | **BROKEN** | WORKING | `current_plan_execution` not in snapshot |
| Wave tree | WORKING | WORKING | — |
| Task detail table | WORKING | WORKING | — |
| Gate results | WORKING | WORKING | — |

### F3: Agents

| Panel | Push Mode | Pull Mode | Issue |
|-------|-----------|-----------|-------|
| Agent roster | WORKING | WORKING | — |
| Summary line | WORKING | WORKING | — |
| Token sparkline | WORKING | WORKING | — |
| Agent output | WORKING | WORKING | From episodes + task outputs |
| Token burn gauge | WORKING | WORKING | — |
| **Agent topology** | WORKING | **BROKEN** | Only populated from snapshot, no disk load |

### F4: Git — **ALL STUB**

| Panel | Status | Issue |
|-------|--------|-------|
| Branch tree | STUB | Data struct never populated |
| Worktree list | STUB | Same |
| Commit graph | STUB | Same |
| Branch info | STUB | View runs live git commands as workaround |

### F5: Logs — WORKING

| Panel | Push Mode | Pull Mode |
|-------|-----------|-----------|
| Filtered log | WORKING | WORKING |
| Signal stream | WORKING | WORKING |

### F6: Config

| Panel | Push Mode | Pull Mode | Issue |
|-------|-----------|-----------|-------|
| **Config editor** | **STUB** | **STUB** | `config_pending` never initialized |
| **Provider health** | **BROKEN** | **BROKEN** | No `provider_health` field in TuiState |
| Model comparison | WORKING | WORKING | Uses cascade router + efficiency data |

### F7: Inspect / Knowledge

| Panel | Push Mode | Pull Mode | Issue |
|-------|-----------|-----------|-------|
| Context overview | WORKING | WORKING | — |
| Token burn by role | WORKING | WORKING | — |
| Cost by model | WORKING | WORKING | — |
| Cascade router | WORKING | WORKING | — |
| Alerts & health | WORKING | WORKING | — |
| **Engram DAG** | **PARTIAL** | WORKING | Signals not explicitly synced in push |
| Episode replay | WORKING | WORKING | — |
| **Knowledge browse** | **BROKEN** | WORKING | `knowledge_entries` not updated in push mode |

### F8: Marketplace — WORKING

| Panel | Push Mode | Pull Mode |
|-------|-----------|-----------|
| Job list | WORKING | WORKING |
| Job detail | WORKING | WORKING |
| Create job form | PARTIAL (no submit) | PARTIAL |

### F9: Atelier — WORKING

All panels working in both modes.

### F10: Learning — WORKING

All three sub-tabs (Route, History, Efficiency) working in both modes.

### Summary

| Classification | Count | Panels |
|---------------|-------|--------|
| **WORKING** (both modes) | ~20 | Plans, tasks, gates, logs, marketplace, atelier, learning |
| **PULL-ONLY** | 4 | Diff, plan summary, knowledge browse, engram DAG |
| **PUSH-ONLY** | 1 | Agent topology |
| **STUB** | 6 | Git (4 panels), MCP, Procs |
| **BROKEN** | 3 | Config editor, provider health, output panel |

---

## 3. Event Pipeline: What Flows vs What Drops

### Events That Flow End-to-End (Runner → TUI)

| Event | Source | Status |
|-------|--------|--------|
| PlanStarted | event_loop.rs:3801 | WORKING |
| PlanCompleted | event_loop.rs (×5) | WORKING |
| TaskStarted | event_loop.rs:4436 | WORKING |
| TaskCompleted | event_loop.rs (×4) | WORKING |
| AgentSpawned | event_loop.rs:4435 | WORKING (partial — no task_id) |
| AgentOutput | agent_events.rs:71 | WORKING |
| AgentCompleted | agent_events.rs:141 | WORKING |
| GateResult | event_loop.rs:1014 | WORKING (missing duration_ms, output) |
| PhaseTransition | event_loop.rs (×7) | WORKING |
| Error | event_loop.rs (×11) | WORKING |
| CascadeRouterUpdated | event_loop.rs:5227 | WORKING |
| GateThresholdsUpdated | event_loop.rs:4910 | WORKING |
| EventLogEntry | via runner_event() | WORKING |

### Events That NEVER Flow

| Event | Issue | Impact |
|-------|-------|--------|
| **EfficiencyEvent** | TuiBridge method exists but **never called** by runner | Efficiency panel shows zeros |
| **TaskPhaseChanged** | TuiBridge method exists but **never called** | No intermediate task phase visibility |
| **TaskOutputAppended** | Event variant defined but **no TuiBridge method**, **not in apply()** | Live output text never flows |
| **Diagnosis** | Published by orchestrate.rs only, **no TuiBridge method** | Runner can't diagnose |
| **EpisodeRecorded** | Published by orchestrate.rs only, **no TuiBridge method** | Episode detail bypasses runner |

### Events Missing Data

| Event | Missing Field | Impact |
|-------|---------------|--------|
| AgentSpawned | `task_id` | Agent task column shows "-" |
| AgentSpawned/Update | `input_tokens`, `output_tokens` | Agent progress shows "0k/200k" |
| GateResult | `duration_ms`, `output`, `failure_kind` | Can't show gate timing or failure detail |

---

## 4. DashboardSnapshot vs DashboardData: Field Parity Gaps

Data that exists in pull mode (DashboardData reads from disk) but **drops** in push mode
(DashboardSnapshot doesn't carry it):

| DashboardData Field | Source File | Snapshot Equivalent | Gap |
|---------------------|------------|---------------------|-----|
| `efficiency` (aggregate) | `.roko/learn/efficiency.jsonl` | None | Cost/turn summary lost |
| `efficiency_events` (raw) | `.roko/learn/efficiency.jsonl` | None | Per-task token data lost |
| `cascade_router` (typed) | `.roko/learn/cascade-router.json` | `cascade_router_json` (string) | Typed struct → opaque JSON |
| `experiment_store` (full) | `.roko/learn/experiments.json` | `experiment_winners` (summary) | A/B detail lost |
| `adaptive_thresholds` (typed) | `.roko/learn/gate-thresholds.json` | `gate_thresholds_json` (string) | Typed struct → opaque JSON |
| `gate_results_page` (rich) | Signal analysis | None | Signal summaries + thresholds lost |
| `current_plan_execution` | Episodes + task outputs | None | Plan execution context lost |
| `git_diff` | `git diff HEAD` | None | Git state never pushed |
| `git_diff_is_staged` | Git state | None | Staged flag lost |
| `conductor_alerts` | Signals with `conductor:alert:` prefix | None | Alert summaries lost |
| `recent_signals` | `.roko/engrams.jsonl` | None | Signal summaries not in snapshot |
| `cfactor` (latest) | `.roko/learn/c-factor.jsonl` | `cfactor_trend` (buckets only) | Latest CFactor value lost |

### Snapshot Fields That Are Set But Never Read

| Field | Set By | Issue |
|-------|--------|-------|
| `errors` (ring buffer) | `Error` event | Only `stats.errors_total` consumed; 64-entry ring buffer ignored |
| `episodes` (ring buffer) | `EpisodeRecorded` event | TUI uses DashboardData.episodes instead; ring buffer unused |
| `agent_topology` | **Never set** — no event handler | Checked by TUI but always empty |

---

## 5. orchestrate.rs Subsystem Wiring Audit

### Fully Wired & Functioning (15/19)

| Subsystem | Evidence | Notes |
|-----------|----------|-------|
| SystemPromptBuilder | Lines 16000-16200, `build_role_system_prompt()` | 9-layer assembly with AttentionBidder |
| EpisodeLogger | Lines 12212, 19239, 10667 | `.roko/episodes.jsonl` with HDC fingerprints |
| ProcessSupervisor | Lines 4600, 4820, 5033 | Active agent monitoring via `supervisor.count()` |
| MCP passthrough | Lines 4106-4283 | Real server spawn + tool discovery |
| Efficiency events | Lines 18724-18835, 11602-11661 | Per-task token/cost tracking, flushed to disk |
| CascadeRouter | Lines 15303-15570 | LinUCB bandits + health filtering |
| Adaptive thresholds | Lines 4643-4647, 17499-17501 | EMA per rung, loaded/saved |
| Gate rung oracles | Lines 3209-3300, 18172-18196 | Perplexity fact-check + LLM judge |
| C-factor metrics | Lines 7268-7275, 15497-15503 | Fleet-wide calibration |
| Enrichment dispatch | Lines 9282-9401 | Non-fatal, 4096 token cap |
| Gate failure replan | Lines 5479-5667 | Config-gated, per-plan cap |
| HDC fingerprint | Lines 3176-3179, 10667 | Per-episode encoding |
| Playbook queries | Lines 15642-15662, 11189-11522 | Match + record + save |
| Context bidders | Lines 16034-16186 | Neuro/Task/Research/Oracles |
| Daimon affect | Lines 3087-3141, 8221-8237, 16019-16021 | Strategy coordinates, appraise outcomes |

### Conditionally Wired (3/19)

| Subsystem | Condition | Default |
|-----------|-----------|---------|
| Prompt experiments | Returns None if no active experiment | Falls through to cascade routing |
| Safety contracts | Falls back to permissive if YAML missing | Violations logged but don't block |
| Auto-dream | `config.dreams.auto_dream` must be true | Disabled by default |

### Not Wired (1/19)

| Subsystem | Issue |
|-----------|-------|
| VCG auction | Zero references in orchestrate.rs. Built in roko-core but greedy path dominates. |

---

## 6. The Fix Priority Matrix

### Tier 1: Fix push-mode data flow (biggest bang for buck)

These fixes make the TUI actually useful during `roko plan run`:

| Fix | Files | Effort | Impact |
|-----|-------|--------|--------|
| Publish EfficiencyEvent from runner | `event_loop.rs`, `tui_bridge.rs` | 30 min | Efficiency panel shows real data |
| Add task_id to AgentSpawned event | `tui_bridge.rs`, `dashboard_snapshot.rs` | 15 min | Agent task column works |
| Forward token counts per agent turn | `agent_events.rs`, `tui_bridge.rs` | 30 min | Agent progress gauge works |
| Forward agent output text to snapshot | `event_loop.rs`, `dashboard_snapshot.rs` | 1 hr | Output panel shows streaming text |
| Add git_diff to snapshot | `dashboard_snapshot.rs`, `state.rs` | 30 min | Diff panel works in push mode |

### Tier 2: Fill TUI stub panels

| Fix | Files | Effort | Impact |
|-----|-------|--------|--------|
| Wire Git view to populate TuiState | `tui/views/git_view.rs`, `state.rs` | 2 hr | F4 Git tab works |
| Wire MCP panel data | `state.rs`, `dashboard.rs` | 1 hr | MCP status visible |
| Wire config editor to load config | `tui/views/config_view.rs` | 1 hr | Config editing works |
| Wire provider health check | `state.rs`, config providers | 1 hr | Provider status visible |

### Tier 3: Event pipeline gaps

| Fix | Files | Effort | Impact |
|-----|-------|--------|--------|
| Call TaskPhaseChanged from runner | `event_loop.rs` | 15 min | Intermediate phases visible |
| Add TuiBridge method for Diagnosis | `tui_bridge.rs`, `event_loop.rs` | 30 min | Diagnosis panel works |
| Enrich GateResult with timing data | `event_loop.rs`, `tui_bridge.rs` | 30 min | Gate details visible |
| Handle TaskOutputAppended in apply() | `dashboard_snapshot.rs` | 15 min | Live output accumulation |

### Tier 4: Snapshot field cleanup

| Fix | Files | Effort | Impact |
|-----|-------|--------|--------|
| Expose errors ring buffer to TUI | `state.rs` | 15 min | Error history visible |
| Use snapshot.episodes in push mode | `state.rs` | 15 min | Episode detail in push mode |
| Add AgentTopologyUpdated event | `dashboard_snapshot.rs` | 30 min | Agent mesh rendering |
| Deserialize JSON strings in TUI | `state.rs` | 30 min | Typed cascade router + thresholds |

---

## 7. What's Actually Broken vs Just Missing

### Actually Broken (wrong behavior)

1. **BUG #06**: Preflight verify skips agents when stubs compile (doc 08)
2. **BUG #07**: Gate crate name extraction from nested paths (doc 08)
3. **BUG #08**: TUI shows 0 agents while agents run (doc 08)
4. **Config test assertion**: Model persist test may have inverted assertion (doc 07)
5. **MCP crash detection**: Crashing MCP binary invisible to session (doc 07)
6. **ACP stdin EOF**: Process doesn't exit on stdin close (doc 07)

### Missing Data Flow (feature gap, not bug)

7. Efficiency panel zeros during plan run
8. Output panel empty during plan run
9. Diff panel empty during plan run
10. Agent task/progress columns empty during plan run
11. Diagnosis panel empty always
12. Plan summary panel empty during plan run

### Stub Panels (never implemented)

13. Git view (F4) — data struct exists, never populated
14. MCP panel — no data source
15. Procs panel — no ProcessSupervisor integration
16. Config editor — never loads config
17. Provider health — no field in TuiState

### Conditional Subsystems (work but off by default)

18. Auto-dream — `config.dreams.auto_dream` defaults false
19. Advanced gate rungs — `gates.enable_advanced_rungs` not set
20. Gate failure replan — `learning_config.replan_on_gate_failure` must be true
21. Safety contracts — permissive fallback when YAML missing

### Dead Code

22. VCG auction — built in roko-core, zero references in orchestrate.rs
23. `errors` ring buffer on DashboardSnapshot — populated, never read
24. `agent_topology` on DashboardSnapshot — read by TUI, never populated
