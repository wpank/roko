# Implementation Status (2026-04-27)

## What's Done

### ACP Core (working in Zed today)

| Feature | Status | Where |
|---------|--------|-------|
| ACP server (stdio, JSON-RPC 2.0) | ✓ Done | `crates/roko-acp/` |
| 6 LLM providers, 22 models | ✓ Done | `bridge_events.rs` |
| **9 config options** | ✓ Done | `session.rs` `build_config_options()` |
| **49 slash commands** | ✓ Done | `session.rs` + `bridge_events.rs` |
| 3 modes (code, plan, research) | ✓ Done | `session.rs` mode-specific system prompts |
| Multi-turn conversation history | ✓ Done | `session.rs` ConversationTurn, FIFO trim 40/64K |
| File context injection (include_context) | ✓ Done | `bridge_events.rs` extract_resource_uris + read_file_context |
| Session persistence (.roko/sessions/) | ✓ Done | `session.rs` persist/load/gc, `handler.rs` auto-persist |
| Plan updates (ACP plan session update) | ✓ Done | `types.rs` + `CognitiveEvent::PlanUpdate` |
| Tool call cards (ACP tool_call update) | ✓ Done | `types.rs` |
| **Pipeline state machine** | ✓ Done | `pipeline.rs` — PipelinePhase, PipelineState, step() |
| **WorkflowRun tracking** | ✓ Done | `workflow.rs` — run state, timing, cost, GateResult, ReviewFinding |
| **Pipeline runner (executor)** | ✓ Done | `runner.rs` — PipelineConfig, spawns agents, runs gates, commits |
| **Pipeline routing in dispatch** | ✓ Done | `bridge_events.rs` — routes to pipeline when workflow != "none" |
| **Auto workflow selection** | ✓ Done | `pipeline.rs` auto_select() — picks express/standard/full from prompt |
| **Review strictness** | ✓ Done | `runner.rs` — none/quick/standard/thorough prompts |
| **Proper plan events** | ✓ Done | `CognitiveEvent::PlanUpdate` → `SessionUpdate::Plan` |
| **active_run on session** | ✓ Done | `session.rs` — `Option<WorkflowRun>` field, persisted |
| **roko-gate integration** | ✓ Done | `runner.rs` — CompileGate, TestGate, ClippyGate via Verify trait |
| **Adaptive thresholds** | ✓ Done | `runner.rs` — loads/saves `.roko/learn/gate-thresholds.json`, EMA per rung |
| **Shared workflow state** | ✓ Done | `session.rs` SharedWorkflowRun, `/workflow status` reads live state |
| **Structured review parsing** | ✓ Done | `runner.rs` — `parse_structured_review_verdict()`, JSON schema prompt |

### Config Options in Zed Status Bar (9 total)

1. **Model** — all models from roko.toml
2. **Effort** — Low / Medium / High / Max
3. **Temperament** — Conservative / Balanced / Aggressive / Exploratory
4. **Routing** — Auto / Manual
5. **Clippy** — On / Off
6. **Tests** — On / Off
7. **Workflow** — None / Express / Standard / Full / Auto
8. **Review** — None / Quick / Standard / Thorough
9. **Retries** — 1 / 2 / 3

### Slash Commands (49 total)

| Category | Commands |
|----------|----------|
| Status & Diagnostics | `/status`, `/doctor`, `/config`, `/learn` |
| Research | `/research`, `/search`, `/enhance-prd`, `/analyze` |
| Specification (PRD) | `/prd-idea`, `/prd-draft`, `/prd-list`, `/prd-status`, `/prd-plan`, `/prd-consolidate` |
| Planning | `/plan-list`, `/plan-show`, `/plan-generate`, `/plan-validate`, `/plan-run`, `/plan-resume` |
| Implementation | `/run`, `/agents`, `/agent-chat`, `/agent-start`, `/agent-stop` |
| Verification | `/build`, `/test`, `/clippy`, `/fmt`, `/gate`, `/review` |
| Knowledge | `/knowledge`, `/knowledge-stats`, `/knowledge-gc`, `/knowledge-backup`, `/dream` |
| Code Intel | `/index`, `/explain`, `/replay` |
| Learning | `/learn-router`, `/learn-episodes`, `/learn-tune` |
| Workflow | `/workflow`, `/express`, `/full`, `/review-this`, `/pipeline` |
| System | `/audit`, `/help` |

## Verification

- 45 total tests (37 unit + 8 integration), all passing
- clippy clean (`-D warnings`)
- roko-acp compiles clean

## Phase Status (from 08-IMPLEMENTATION-PLAN)

### Phase 1: Workflow Runner Core — DONE

- [x] `WorkflowRun` struct — `workflow.rs`
- [x] `PipelineStateMachine` (9 states, step function) — `pipeline.rs`
- [x] `run_workflow_pipeline()` executor — `runner.rs`
- [x] Pipeline routing — `bridge_events.rs`
- [x] `/express`, `/full` slash commands wired
- [x] `PipelineConfig` struct for clean parameter passing
- [x] Auto workflow mode (complexity-based selection)
- [x] 10 pipeline tests + 2 workflow tests

### Phase 2: Gate Integration — DONE

- [x] Gates run automatically: compile → test → clippy (configurable)
- [x] Emit tool_call updates per gate
- [x] On failure: autofix → retry → reimpl fallback
- [x] Wire `roko-gate` crate: CompileGate, TestGate, ClippyGate via Verify trait
- [x] GatePayload signal with working_dir, Verdict with duration_ms + test_count
- [x] Adaptive thresholds: load/observe/save `.roko/learn/gate-thresholds.json`
- [x] Rung skip decisions based on consecutive pass streaks (20+ passes → skip)
- [x] Shared WorkflowRun state via `Arc<Mutex<>>` for `/workflow status`

### Phase 3: Review Integration — DONE

- [x] Reviewer agent (claude CLI)
- [x] APPROVED/REVISE verdict parsing
- [x] Review feedback → retry prompt
- [x] review_strictness: none/quick/standard/thorough
- [x] Structured JSON review output via `parse_structured_review_verdict()`
- [x] JSON schema hint appended to all review prompts
- [ ] Multi-role reviews (architect + auditor) for "thorough"

### Phase 4-6: Multi-task, Custom Workflows, Triggers — FUTURE

## Architecture

```
User prompt in Zed
  │
  ▼
bridge_events.rs → handle_session_prompt_inner()
  │
  ├─ workflow == "none" → single agent dispatch (existing)
  │
  ├─ workflow == "auto" → auto_select(prompt) → express|standard|full
  │
  └─ workflow == "express"|"standard"|"full"
      │
      ▼
    runner::run_workflow_pipeline(PipelineConfig)
      │
      ├─ Creates WorkflowRun
      ├─ Drives PipelineState (pure state machine)
      │    step(event) → action (no side effects)
      │
      ├─ PipelineAction::SpawnStrategist → claude --print (full only)
      ├─ PipelineAction::SpawnImplementer → claude --print
      ├─ PipelineAction::RunGates → cargo build/test/clippy
      ├─ PipelineAction::SpawnAutoFixer → claude --print (on gate fail)
      ├─ PipelineAction::SpawnReviewer → claude --print (standard/full)
      ├─ PipelineAction::Commit → git add + git commit
      └─ PipelineAction::Done/Halt → final summary

    ACP events emitted:
      CognitiveEvent::PlanUpdate → SessionUpdate::Plan (progress cards)
      CognitiveEvent::ToolCallStart/Complete → tool_call cards per phase
      CognitiveEvent::TokenChunk → streaming text updates
```

## Files

| File | What | Tests |
|------|------|-------|
| `crates/roko-acp/src/pipeline.rs` | PipelinePhase (9 states), PipelineState, WorkflowTemplate, step(), auto_select() | 10 |
| `crates/roko-acp/src/workflow.rs` | WorkflowRun, GateResult, ReviewFinding | 2 |
| `crates/roko-acp/src/runner.rs` | PipelineConfig, run_workflow_pipeline(), roko-gate Verify gates, adaptive thresholds, shared run state, structured review parsing | — |
| `crates/roko-acp/src/session.rs` | +active_run field, +workflow/review/retries config, +5 workflow slash commands | 14 |
| `crates/roko-acp/src/bridge_events.rs` | +PlanUpdate event, +pipeline routing, +auto dispatch, +5 slash command arms | 5 |

## Next Steps

1. **Production test** — Set workflow to "express" in Zed, send prompt, observe pipeline
2. **Structured review** — JSON verdict schema, multi-role for "thorough"
3. **Multi-task plans** — Wire `roko-orchestrator` for plan DAG execution
4. **Custom workflow templates** — Load from `.roko/workflows/*.toml`
5. **Triggers** — File watch, github event triggers
