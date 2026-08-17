# Current State — What's Wired, What's Dead, What's Floating

> **What is this?** Snapshot of the codebase categorized by runtime status: actively
> used, dead/deleted, or built-but-never-wired. Used to prioritize v2 refactoring work.
>
> **For first-time readers:** This document helps you understand which parts of roko's
> ~177K LOC are actually exercised at runtime vs. compiled but never called. If you are
> looking for where to contribute, the "Floating Code" section lists modules that need
> wiring into runtime paths. The "What's WIRED and WORKS" table shows what is
> production-ready.
>
> **Last updated: 2026-08-13**

## Active Codepaths (ACTUALLY USED)

### `roko run <prompt>` — Single-shot agent execution
- Entry: `main.rs` → `commands/util.rs:cmd_run`
- Path: WorkflowEngine v2 (roko-runtime)
- Default engine: `EngineVariant::V2`
- Legacy path: feature-gated, raises error if called

### `roko plan run <dir>` — Plan execution
- Entry: `main.rs` → `commands/plan.rs:cmd_plan`
- Path: Runner v2 (`runner/event_loop.rs`)
- Architecture: event-driven, streaming agent output, real-time state
- Uses: `ParallelExecutor` from roko-orchestrator
- State: flushed to `.roko/state/executor.json` per task

### `roko serve` — HTTP control plane
- ~85 REST routes + SSE + WebSocket on :6677
- Uses StateHub for real-time updates

### `roko dashboard` — Interactive TUI
- ratatui, F1-F7 tabs
- Reads from StateHub (zero-copy)

### `roko prd` — PRD lifecycle
- idea → draft → plan → publish
- `prd plan <slug>` generates tasks.toml via agent

### `roko research` — Research agent
- Perplexity integration, document enhancement

### `roko agent` — Agent management
- Per-agent HTTP sidecar (roko-agent-server, 13 routes)
- Interactive chat REPL

---

## Dead Code (DELETED or FEATURE-GATED)

### `orchestrate.rs` — DELETED (2026-07)
- **Status**: **Deleted** in E12-T07. Previously 23,331 lines behind `legacy-orchestrate` feature.
- **What happened**: All useful patterns (system prompt builder, episode logging, model
  routing) were reimplemented in Runner v2. The file was deleted entirely rather than
  kept behind a feature gate. (QW-4 complete.)

### `dispatch_direct.rs`
- **Status**: Behind `legacy-direct-dispatch` feature, NOT enabled
- **Contains**: Raw single-shot dispatch, no tools/system prompt/MCP
- **Comment in code**: "Deprecated; will be removed"

### Legacy `run_once()` in run.rs
- **Status**: Stub that raises error
- **Replaced by**: WorkflowEngine v2 path

---

## Floating Code (~15K LOC built but never wired)

### roko-runtime (8 modules, ~2K LOC)
| Module | Purpose | External uses |
|--------|---------|---------------|
| theta_consumer | State drift feedback | 0 |
| delta_consumer | Change detection | 0 |
| demurrage_consumer | Decay enforcement | 0 |
| energy | Cognitive energy tracking | 0 |
| heartbeat_attention | Attention modulation | 0 |
| heartbeat_probes | T0 health probes | 0 |
| run_ledger | Run-level cost tracking | 0 |
| task_scheduler | Cron/scheduled execution | 0 |

### roko-learn (14 modules, ~3K LOC)
| Module | Purpose | External uses |
|--------|---------|---------------|
| active_inference | Free energy minimization routing | 0 |
| baseline | Regression baselines | 0 |
| bayesian_confidence | Model confidence estimation | 0 |
| calibration_policy | Predict-publish-correct | 0 |
| error_enrichment | Gate error analysis | 0 |
| event_subscriber | Bus event consumer | 0 |
| jsonl_rotation | Log rotation | 0 |
| local_reward | Per-turn reward computation | 0 |
| oracles | Verification oracles | 0 |
| pareto | Multi-objective optimization | 0 |
| post_gate_reflection | Learning from gate failures | 0 |
| quality_judge | Output quality scoring | 0 |
| section_outcome | Prompt section effectiveness | 0 |
| verdict_scorer | Verdict → reward mapping | 0 |

### Language parsers (3 crates, ~5K LOC)
- roko-lang-rust, roko-lang-typescript, roko-lang-go
- Tree-sitter wrappers, symbol extraction
- Not called from agent context assembly

### MCP integrations (3 crates, ~2K LOC)
- roko-mcp-github, roko-mcp-slack, roko-mcp-scripts
- Built but not mounted in agent MCP dispatch

### Other
- ~~roko-calc~~: DELETED (QW-5 complete). Was an empty skeleton with no lib.rs.
- roko-acp: exists, listed as dependency, not called from runtime paths
- VCG auction: built in roko-compose but greedy path dominates at runtime

---

## What's WIRED and WORKS

| Component | Crate | Status |
|-----------|-------|--------|
| 7 LLM backends | roko-agent | WIRED |
| MultiAgentPool, TaskRunner, ToolLoop | roko-agent | WIRED |
| ParallelExecutor, DAG, plan discovery | roko-orchestrator | WIRED |
| 11 gates, 7 rungs, adaptive thresholds | roko-gate | WIRED |
| CascadeRouter, EpisodeLogger, experiments | roko-learn | WIRED |
| KnowledgeStore, tier progression | roko-neuro | WIRED |
| SystemPromptBuilder (9-layer) | roko-compose | WIRED |
| Circuit breaker, stuck detection | roko-conductor | WIRED |
| DaimonState affect modulation | roko-daimon | WIRED |
| ProcessSupervisor, EventBus, Heartbeat | roko-runtime | WIRED |
| 85 REST routes, SSE, WebSocket | roko-serve | WIRED |
| PulseBus (topic-filtered pub/sub) | roko-core | WIRED |
| Cell trait (identity + metadata) | roko-core | WIRED (but missing execute()) |
| Pulse + Topic + TopicFilter | roko-core | WIRED |

---

## V2 Primitives Already Partially Built

| V2 Concept | Current Code | Gap |
|------------|-------------|-----|
| Cell | `roko-core/src/cell.rs` — CellId, CellVersion, protocols(), estimated_cost() | Missing execute(), CellContext, schemas |
| Signal | `Signal` in `roko-core/src/engram.rs` (file retains old name); `pub type Engram = Signal` backward-compat alias. `balance` field added (QW-3). | DONE (2026-08-12). Struct is `Signal`; backward-compat alias retained. |
| Pulse | `roko-core/src/pulse.rs` — topic, kind, body, lineage_hint | Matches v2 closely |
| Bus | `PulseBus` in `roko-core/src/pulse_bus.rs` wrapping EventBus | Matches v2, missing And/Or/Not filters |
| Store | 6 methods in traits.rs, 4+ implementations | Matches v2 exactly |
| Score | Score trait + ScoreValue with 5 dimensions | Matches v2 |
| Verify | Verify trait → Verdict (hard/soft criteria, evidence) | Matches v2 |
| Route | Route trait → Selection | Matches v2 |
| Compose | Compose trait with budget + scorer | Matches v2 |
| React | React trait → PolicyOutputs | Matches v2 |
| Graph | NO equivalent — plans are flat TOML task lists | Build from scratch |
| Engine | NO equivalent — Runner v2 is plan-specific | Build from scratch |
| Flow | NO equivalent — implicit in event_loop.rs state | Build from scratch |
| Feed | NO equivalent | Build from scratch |
