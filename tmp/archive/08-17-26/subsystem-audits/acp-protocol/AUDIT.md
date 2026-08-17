# ACP Protocol Subsystem Audit

JSON-RPC 2.0 over stdio, pure state machine pipeline, editor integration — the newest and cleanest runtime, with known gaps in learning and safety integration.

### Architecture Runner Status (2026-04-28)
**ACP integrated with WorkflowEngine.** Phases 3.1 + 4B completed:
- `AcpAdapter` (P3A, `roko-acp/src/acp_adapter.rs`) implements `EventConsumer` trait
- ACP sessions now route through `run_with_workflow_engine()` (P4B)
- Legacy `Command::new("claude")` removed from `bridge_events.rs`
- RuntimeEvents flow through EventBus → AcpAdapter → ACP JSON-RPC notifications
- **Remaining**: full 16 message type mapping verification, panel update integration

## The Problem

roko-acp (6,963 LOC, 11 files) is a complete ACP 0.12.2 implementation for Zed/JetBrains/Neovim/VS Code. It has the best architecture (pure state machine + effect driver), but runs as an isolated silo — no learning feedback, no safety contracts, no episode logging, no cascade router training.

---

## 1. File Inventory

| File | LOC | Purpose | Status |
|---|---|---|---|
| lib.rs | 18 | Public API facade | Active |
| types.rs | 750 | JSON-RPC 2.0 protocol types (2 tests) | Active |
| config.rs | 63 | ACP runtime configuration | Active |
| transport.rs | 307 | Stdio framing + message I/O (3 tests) | Active |
| handler.rs | 387 | Main dispatch loop + method routing (1 test) | Active |
| session.rs | 1,539 | Session lifecycle, state, persistence (15 tests) | Active |
| bridge_events.rs | 1,855 | Event bridge: provider streaming → ACP notifications | Active |
| pipeline.rs | 539 | Pure state machine (10 tests) | Active |
| runner.rs | 969 | Effect driver — executes pipeline actions | Active |
| workflow.rs | 143 | Workflow run metadata (2 tests) | Active |
| tests/protocol_conformance.rs | 393 | Integration tests (8 tests) | Active |
| **Total** | **6,963** | **41 tests** | |

---

## 2. Protocol Implementation

### Supported Methods

| Method | Purpose | Status |
|---|---|---|
| `initialize` | Protocol handshake, report capabilities | Wired |
| `session/new` | Create session, emit slash commands + config options | Wired |
| `session/list` | List live + persisted sessions | Wired |
| `session/load` | Restore persisted session | Wired |
| `session/prompt` | Dispatch prompt → agent → gates → commit | Wired |
| `session/config/update` | Update session settings (also accepts `session/set_config_option`) | Wired |
| `session/set_mode` | Set interaction mode | Wired |
| `session/cancel` | Cancel in-flight prompt (notification) | Wired |

### Session State

```rust
AcpSession {
    session_id: String,
    session_name: Option<String>,
    created_at: DateTime<Utc>,
    config_state: SessionConfigState,       // agent_mode, model, effort, temperament,
                                            // routing_mode, clippy_enabled, tests_enabled,
                                            // workflow, review_strictness, max_iterations
    client_capabilities: ClientCapabilities,
    cancel_token: CancelToken,
    busy: Arc<AtomicBool>,
    mcp_servers: Vec<McpServerConfig>,
    config_options: Vec<ConfigOption>,
    conversation_history: Vec<ConversationTurn>,
    active_run: Option<WorkflowRun>,
    shared_run: SharedWorkflowRun,          // Arc<Mutex<Option<WorkflowRun>>>
}
```

Persisted to `.roko/sessions/<id>.json`. GC at 7-day TTL.

---

## 3. Pipeline State Machine

### PipelinePhase (10 states)

```
Pending → Strategizing → Implementing → Gating → AutoFixing → Reviewing → Committing → Complete/Halted/Cancelled
```

### Workflow Templates

| Template | Phases | Auto-selected when |
|---|---|---|
| Express | Implement → Gate → Commit | Simple prompts (<15 words, "fix", "typo") |
| Standard | + Review | Medium prompts |
| Full | + Strategist | Complex prompts (>50 words, "refactor", "architecture") |

### State Machine Pattern

Pure `step(event) → action` — no I/O, easy to test and persist:
- **Events:** `Start`, `StrategyComplete`, `StrategySkipped`, `AgentCompleted`, `AgentFailed`, `GatesPassed`, `GateFailed`, `ReviewApproved`, `ReviewRevise`, `CommitDone`, `Timeout`, `BudgetExceeded`, `UserCancel`
- **Actions:** `SpawnStrategist`, `SpawnImplementer`, `SpawnAutoFixer`, `RunGates`, `SpawnReviewer`, `Commit`, `Done`, `Halt`
- **Retry loop:** Gate failure → auto-fix agent → retry gate (up to max_iterations)
- **Review loop:** Reviewer sends revision feedback → re-implement → re-gate

---

## 4. Effect Driver (runner.rs)

Executes pipeline actions:

| Action | What runner.rs does |
|---|---|
| SpawnStrategist | Spawn `claude --print` with strategist prompt |
| SpawnImplementer | Spawn `claude --print` with implementer prompt |
| SpawnAutoFixer | Spawn `claude --print` with fix prompt + gate errors |
| RunGates | CompileGate → TestGate → ClippyGate (adaptive skip) |
| SpawnReviewer | Single reviewer (quick/standard) or Architect+Auditor (thorough) |
| Commit | `git add -A` → `git commit -m "feat: ..."` |

**Agent spawn (pipeline phases):** Always via `claude --print --dangerously-skip-permissions <prompt>` subprocess (`run_claude_cli` in runner.rs). The single-agent path (`workflow == "none"`) also supports API providers (OpenAI-compat, Anthropic API, Gemini, Perplexity) via `run_openai_compat_cognitive_task` in bridge_events.rs.

**Gate adaptive thresholds:** Loaded/saved at `.roko/learn/gate-thresholds.json` (shared with other runtimes — potential conflict).

---

## 5. Event Bridge (bridge_events.rs)

Converts provider streaming output to ACP notifications:

```
Provider Output (SSE/API or Claude CLI stream-json)
    ↓ parse
CognitiveEvent (internal)
    ├─ TokenChunk
    ├─ ThinkingChunk
    ├─ ToolCallStart / ToolCallComplete
    ├─ PlanUpdate
    ├─ MaxTokens
    └─ Complete (stop_reason, usage)
    ↓ convert (map_event_to_update)
SessionUpdate (ACP type)
    ├─ AgentMessageChunk  ← TokenChunk
    ├─ AgentThoughtChunk  ← ThinkingChunk
    ├─ ToolCall           ← ToolCallStart
    ├─ ToolCallUpdate     ← ToolCallComplete
    └─ Plan               ← PlanUpdate
    ↓ send
Editor notification (session/update)
```

Note: `UsageUpdate` is defined in the ACP types but is not currently emitted by the event bridge — usage data arrives via `Complete` but is not forwarded as a `UsageUpdate` notification.

**Two execution modes:**
1. `workflow == "none"` (or unrecognized): Single-agent dispatch — resolves model from roko.toml, routes to `run_claude_cognitive_task` (ClaudeCli) or `run_openai_compat_cognitive_task` (API providers)
2. `workflow != "none"` (express/standard/full/auto): Delegates to `runner::run_workflow_pipeline` — multi-phase pipeline via state machine

---

## 6. ACP-Exclusive Features

Features in ACP that other runtimes don't have:

1. **Workflow pipeline** — Express/Standard/Full templates with automatic loop-back
2. **Real-time editor updates** — Plan entries, tool call cards, usage streamed live
3. **Interactive session config** — Workflow strictness, gates, MCP configurable per-session
4. **Multi-role review** — Architect + Auditor must both approve (thorough mode)
5. **Session persistence** — Independent per-session, resumable via `session/load`

---

## 7. What ACP Is Missing (vs Other Runtimes)

| Feature | Runner v2 | Orchestrate.rs | ACP |
|---|---|---|---|
| CascadeRouter learning | Partial | Full | None |
| Episode logging | Partial | Full | None |
| Safety contracts | None | Full | None |
| ToolDispatcher (10-stage) | Partial | Full | None — direct file I/O |
| Prompt experiments | None | Full | None |
| Playbook injection | None | Full | None |
| Knowledge routing | None | Full | None |
| Dream consolidation | None | Full | None |
| DAG execution | Yes | Yes | No — serial only |
| Resume across sessions | Yes | Yes | Partial — session config + conversation history persist via `session/load`; active pipeline run (`WorkflowRun`) is stored in session JSON but runner does not resume from it |
| Budget enforcement | None | Full | None |

---

## 8. Anti-Patterns & Issues

| Issue | Severity | Details |
|---|---|---|
| **No session concurrency locking** | Medium | `SessionManager::sessions` HashMap not protected by Mutex/RwLock. Safe only because handler loop is single-threaded. |
| **No agent timeout** | Medium | `run_claude_cli()` can hang indefinitely. Editor cancel works but no watchdog. |
| **Gate thresholds not synced** | Low | ACP and main orchestrator write to same file — can diverge. |
| **Dead cost/token metrics** | Low | `WorkflowRun::total_cost_usd` and `total_tokens` never updated (always 0). |
| **Brittle file estimation** | Low | Counts "Edit:" and "Create:" in CLI output to estimate files_changed. |
| **No MCP tool dispatch** | Design | MCP servers declared in session but tool calls don't route to them. |
| **Pipeline agents require Claude CLI** | Design | `run_claude_cli()` in runner.rs uses `claude --print --dangerously-skip-permissions`. No API fallback for pipeline phases (Strategist, Implementer, AutoFixer, Reviewer). The single-agent path (`workflow == "none"`) does support API providers. |

---

## 9. Integration Points

**Wired into roko ecosystem:**
- `roko_core::config` — workspace config loading
- `roko_core::agent::{ProviderKind, resolve_model}` — model resolution
- `roko_gate::{CompileGate, TestGate, ClippyGate, AdaptiveThresholds}` — gate execution with adaptive thresholds
- `roko_agent::streaming::parse_sse_line` — SSE parsing (OpenAI-compat stream)
- `roko_agent::StreamChunk` — stream event types

**Not integrated (roko_compose not in Cargo.toml):**
- `roko_compose::system_prompt_builder` — ACP uses simple per-mode static strings in `session.rs` (`CODE_MODE_SYSTEM_PROMPT`, `PLAN_MODE_SYSTEM_PROMPT`, `RESEARCH_MODE_SYSTEM_PROMPT`) instead of the 9-layer builder

**NOT integrated with:**
- `roko_orchestrator::ParallelExecutor` — ACP has its own pipeline
- `roko_learn` — no episodes, no cascade router, no efficiency events
- `roko_agent::Dispatcher` — uses CLI subprocess instead
- `roko_agent::safety` — no contracts, no SafetyLayer
- `roko_daimon` / `roko_neuro` — no affect or knowledge

---

## 10. Entry Point

```bash
# Editor spawns:
roko acp    # no "roko editor" alias exists

# Roko starts ACP server → editor communicates via JSON-RPC over stdin/stdout
```

CLI wiring: `roko-cli/src/main.rs:452` (`Command::Acp` enum variant) → `main.rs:1642-1671` (early exit path, runs server before CLI logging init) and `main.rs:2085-2097` (normal dispatch path).

---

## Sources

- `crates/roko-acp/src/lib.rs` — public API facade, re-exports
- `crates/roko-acp/src/types.rs` — JSON-RPC 2.0 protocol types, `SessionUpdate` variants, `ACP_SPEC_VERSION`
- `crates/roko-acp/src/config.rs` — `AcpConfig` struct, session persistence paths
- `crates/roko-acp/src/transport.rs` — `StdioTransport`, newline-delimited JSON framing
- `crates/roko-acp/src/handler.rs` — `run_acp_server`, method routing, `session/config/update` alias
- `crates/roko-acp/src/session.rs` — `AcpSession` struct fields, `SessionConfigState`, `SessionManager`, `build_slash_commands`, persistence to `.roko/sessions/`
- `crates/roko-acp/src/bridge_events.rs` — `CognitiveEvent` enum, `map_event_to_update`, provider dispatch, workflow mode check
- `crates/roko-acp/src/pipeline.rs` — `PipelinePhase` (10 states), `PipelineEvent`, `PipelineAction`, `WorkflowTemplate`, auto-select logic
- `crates/roko-acp/src/runner.rs` — `run_workflow_pipeline`, `run_claude_cli`, gate execution via `roko-gate`, adaptive thresholds at `.roko/learn/gate-thresholds.json`
- `crates/roko-acp/src/workflow.rs` — `WorkflowRun`, `GateResult`, `ReviewFinding`
- `crates/roko-acp/tests/protocol_conformance.rs` — 8 integration tests
- `crates/roko-acp/Cargo.toml` — dependencies (roko-core, roko-agent, roko-gate; no roko-compose)
- `crates/roko-cli/src/main.rs` — `Command::Acp` enum variant, CLI wiring
