# 05 — Architecture: Spec vs Reality

**What the specs describe vs what actually runs.**

> Current correction, 2026-07-07: this inherited overview still contains older `orchestrate.rs`-centric framing. Today the key execution split is Runner v2 as the live executor, Graph as the default-but-dry-run `plan run` path, WorkflowEngine for one-shot workflows, and `orchestrate.rs` as legacy/source material. Use `13-CURRENT-STATE-MATRIX.md`, `24-OPEN-ISSUE-LEDGER.md`, and `36-ORCHESTRATION-RUNNERS.md` for current priority and runtime ownership.

---

## Spec Architecture (V1 + V2)

### V1: The Implemented Foundation

```
┌─────────────────────────────────────────────┐
│                 Application                  │ Layer 5: CLI, TUI, HTTP
├─────────────────────────────────────────────┤
│                  Compose                     │ Layer 4: Prompt assembly, enrichment
├─────────────────────────────────────────────┤
│                   Agent                      │ Layer 3: LLM dispatch, tool loop, safety
├─────────────────────────────────────────────┤
│                  Runtime                     │ Layer 2: Process supervisor, event bus
├─────────────────────────────────────────────┤
│                   Core                       │ Layer 1: Signal + 6 traits, types, config
└─────────────────────────────────────────────┘
```

**Universal loop** (spec): `query → score → route → compose → act → verify → write → react`

### V2: The Target Architecture

```
┌──────────────────────────────────────────────┐
│               Graph of Cells                  │
│  ┌────────┐  ┌────────┐  ┌────────┐         │
│  │ Cell A │→│ Cell B │→│ Cell C │  ...     │
│  └────────┘  └────────┘  └────────┘         │
│       ↕           ↕           ↕              │
│  ┌──────────────────────────────────────┐    │
│  │         Pulse / Bus Kernel            │    │
│  │  (tick → predict → publish → correct) │    │
│  └──────────────────────────────────────┘    │
└──────────────────────────────────────────────┘
```

---

## Actual Runtime Architecture

What actually happens when you run `roko plan run plans/`:

```
┌─────────────────────────────────────────────────────┐
│  roko-cli/src/orchestrate.rs (23K+ LOC)             │
│                                                      │
│  PlanRunner {                                        │
│    1. Load tasks.toml → build DAG                    │
│    2. For each ready task:                           │
│       a. Build system prompt (RoleSystemPromptSpec)  │
│       b. Enrich with context (neuro, playbooks)      │
│       c. Select model (CascadeRouter)                │
│       d. Dispatch agent (ClaudeCliBackend)            │
│       e. Run gate pipeline (compile→test→clippy)     │
│       f. Record episode + efficiency                 │
│       g. If gate fails → replan                      │
│    3. Persist state → .roko/state/executor.json      │
│    4. Repeat until DAG complete                      │
│  }                                                   │
└─────────────────────────────────────────────────────┘
         │           │            │           │
    ┌────┘     ┌─────┘      ┌─────┘     ┌────┘
    ▼          ▼             ▼           ▼
 roko-     roko-          roko-       roko-
 compose   agent          gate        learn
```

### Key Observations

1. **orchestrate.rs IS the runtime**: Not `roko-runtime`, not `roko-orchestrator`, not `roko-graph`. The actual execution logic lives in this single 23K+ LOC file.

2. **roko-orchestrator provides types, not execution**: The `roko-orchestrator` crate provides `TaskGraph`, `TaskState`, merge queue types — but `orchestrate.rs` implements its own `PlanRunner` that uses these types.

3. **roko-runtime is underused**: Despite having `ProcessSupervisor`, `EventBus`, and `WorkflowEngine`, only `ProcessSupervisor` is actually used (for agent lifecycle tracking).

4. **roko-graph is reachable but not production-live**: The full Cell/Graph/DAG executor exists and the Clap default for `plan run` is Graph, but plan task execution still dry-runs through `TaskExecutorCell` unless parity work lands.

---

## Component-by-Component Reality Check

### Core Types

| Spec Component | Implementation | Wired? |
|---------------|----------------|--------|
| Signal (Engram) | `roko-core::Engram` with alias | ✅ Yes — used everywhere |
| 6 verb traits | All defined in `roko-core` | ✅ Yes — concrete impls exist |
| `loop_tick()` | Defined in roko-core | ❌ No — never called in production |
| Config (roko.toml) | `roko-core::config` | ✅ Yes — loaded at startup |
| Error types | `roko-core::errors` | ✅ Yes |

### Agent Layer

| Spec Component | Implementation | Wired? |
|---------------|----------------|--------|
| ClaudeCliBackend | `roko-agent/src/dispatcher/claude_cli.rs` | ✅ Yes — primary backend |
| ClaudeApiBackend | `roko-agent/src/dispatcher/claude_api.rs` | ✅ Yes |
| Other backends (8+) | Individual dispatcher files | ✅ Yes (Codex, Cursor, Ollama, Gemini, etc.) |
| Tool loop | `roko-agent/src/tool_loop.rs` | ✅ Yes |
| MCP passthrough | Via `--mcp-config` flag | ✅ Yes |
| Safety (pre/post checks) | `roko-agent/src/safety/` | ✅ Yes; bundled/restricted fallback is fail-closed, with operator carve-outs still needing proof |
| Agent pools | `roko-agent/src/pool.rs` | ⚠️ Built, minimally used |
| AttentionBidder | `roko-agent/src/attention.rs` | ✅ Yes — context bidding at dispatch |

### Gate Layer

| Spec Component | Implementation | Wired? |
|---------------|----------------|--------|
| 11 gate types | `roko-gate/src/gates/` | ✅ Yes |
| 7-rung pipeline | `roko-gate/src/pipeline.rs` | ✅ Yes |
| Adaptive thresholds | `roko-gate/src/adaptive.rs` | ✅ Yes — EMA per rung |
| Rung oracles (4-6) | orchestrate.rs `enrich_rung_config` | ✅ Yes |
| Gate failure replan | orchestrate.rs | ✅ Yes |

### Compose Layer

| Spec Component | Implementation | Wired? |
|---------------|----------------|--------|
| SystemPromptBuilder (9 layers) | `roko-compose/src/system_prompt_builder.rs` | ✅ Yes |
| 9 templates | `roko-compose/src/templates/` | ✅ Yes |
| VCG auction | `roko-compose/src/vcg.rs` | ⚠️ Built but greedy dominates |
| Context enrichment | orchestrate.rs `dispatch_agent_with` | ✅ Yes |

### Learning Layer

| Spec Component | Implementation | Wired? |
|---------------|----------------|--------|
| EpisodeLogger | `roko-learn/src/episodes.rs` | ✅ Yes |
| CascadeRouter | `roko-learn/src/cascade_router.rs` | ✅ Yes — persists to JSON |
| Prompt experiments | `roko-learn/src/experiments.rs` | ✅ Yes |
| Efficiency tracking | `roko-learn/src/efficiency.rs` | ✅ Yes |
| Bandit routing | `roko-learn/src/bandits.rs` | ✅ Yes |
| Playbook store | `roko-learn/src/playbooks.rs` | ✅ Yes — queried at dispatch |

### Knowledge Layer

| Spec Component | Implementation | Wired? |
|---------------|----------------|--------|
| Neuro store | `roko-neuro/` | ✅ Yes |
| Distillation | `roko-neuro/src/distill.rs` | ✅ Yes |
| Tier progression | `roko-neuro/src/tiers.rs` | ✅ Yes |
| Dream consolidation | `roko-dreams/` | ⚠️ Built; runtime triggers exist, but v2 cron/delta/BusPulse scheduling and routing consumption remain incomplete |
| Affect engine (Daimon) | `roko-daimon/` | ✅ Yes — loaded per-task |

### Infrastructure

| Spec Component | Implementation | Wired? |
|---------------|----------------|--------|
| HTTP control plane | `roko-serve/` (~278 routes) | ✅ Yes |
| Per-agent sidecar | `roko-agent-server/` (13 routes) | ✅ Yes |
| TUI dashboard | `roko-cli/src/tui/` (F1-F7) | ✅ Yes |
| Process supervisor | `roko-runtime/src/supervisor.rs` | ✅ Yes |
| Event bus | `roko-runtime/src/event_bus.rs` | ⚠️ Built, lightly used |

---

## Architecture Gaps

### 1. The Monolith Problem

`orchestrate.rs` at 23K+ LOC contains:
- Plan loading and DAG construction
- Task scheduling and parallel execution
- System prompt building and enrichment
- Model selection and cascade routing
- Agent dispatch and result handling
- Gate pipeline execution
- Episode and efficiency recording
- Gate failure replanning
- C-factor metrics computation
- State persistence and resume

This should be decomposed into at least 5-6 focused modules.

### 2. The Two-Engine Problem

| Engine | Location | LOC | Used? |
|--------|----------|-----|-------|
| PlanRunner | orchestrate.rs | ~23K | ⚠️ Legacy/source runtime; Runner v2 is the live path and Graph is the current default-but-dry-run plan target |
| WorkflowEngine | roko-runtime/src/workflow.rs | ~2K | ❌ No — dead code |

Either remove `WorkflowEngine` or merge the two.

### 3. The Layer Violation Problem

`roko-runtime` (layer 1-2) depends on:
- `roko-learn` (layer 3) — learning state
- `roko-compose` (layer 3) — prompt assembly
- `roko-gate` (layer 2-3) — gate pipeline

This inverts the layer hierarchy. Lower layers should not depend on higher layers.

### 4. The V2 Integration Problem

`roko-graph` implements the V2 Cell/Graph model but:
- Nothing calls `Graph::execute()` or `Pulse::tick()`
- PlanRunner builds its own DAG from `tasks.toml`, not from Graph
- No bridge between V1 TaskGraph and V2 Cell Graph

### 5. The Persistence Split

| What | Persisted Via |
|------|--------------|
| Signals | FileSubstrate (JSONL) |
| Episodes | JSONL in `.roko/episodes.jsonl` |
| Executor state | JSON in `.roko/state/executor.json` |
| Learning state | JSON files in `.roko/learn/` |
| Knowledge | Neuro store (custom format) |
| Plans | TOML files on disk |

No unified persistence layer — each subsystem serializes differently.
