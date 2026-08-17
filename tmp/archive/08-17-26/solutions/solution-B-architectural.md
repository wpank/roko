# Solution B — Architectural Redesign (Do It Right)

**Philosophy**: Fix the root causes, not the symptoms. Stand up the proper abstractions once,
and most individual bugs resolve as side effects. The key insight from all four audit directories:
**one change — a unified InferenceGateway — unblocks ~60% of everything else.**

**Total estimate**: ~55-65 hours
**Risk**: Medium — requires touching core dispatch path, which everything depends on.
Mitigated by doing it behind a feature flag and cutting over.
**Upside**: Fixes S1, S2, S3, S6, S7, S8 in a single coherent pass. Unblocks all 12 GAP
clusters from mori-diffs. Makes the converge-runner subsystem audits tractable.

---

## The Central Insight

Every audit directory arrives at the same conclusion from a different angle:

| Directory | Finding | Root |
|---|---|---|
| **binary-issues** | Chat is a thin pipe; no tools, no history, no streaming | S1 + S6 |
| **mori-diffs** | `create_agent_for_model` called outside dispatch; two runtimes | GAP-01, GAP-06 |
| **subsystem-audits** | CascadeRouter, knowledge, adaptive thresholds built but unwired | MASTER-IMPL Phase 4-6 |
| **converge-runner** | 55 open issues, most blocked on "no single dispatch path" | Critical batch |

The single highest-leverage change: **stand up `InferenceGateway` as the one path through
which all model calls flow** — chat, run, plan execution, agent sidecar, everything.

---

## Phase 0: Security Hardening (3h) — same as Solution A Batch 1

Do this first regardless of approach. Identical to Solution A Batch 1. Non-negotiable before
any deployment.

---

## Phase 1: InferenceGateway (12-15h)

**What it is**: A session-scoped service that owns the HTTP client, model config, conversation
history, system prompt, tool registry, cost tracking, and streaming state. All model calls
go through it.

**What it replaces**:
- `dispatch_direct.rs` (500 LOC — deleted entirely)
- `reqwest::Client::new()` per-call (S2)
- Bare text-pipe dispatch (S1)
- Hardcoded model/URL/version (S7)
- Missing conversation history (S1.4)
- Two separate dispatch paths for chat vs orchestrator (S10 partial)

### Implementation

```
crates/roko-agent/src/inference_gateway.rs  (new, ~400 LOC)

pub struct InferenceGateway {
    client: reqwest::Client,          // created once, shared
    config: ProviderConfig,           // loaded once from roko.toml
    system_prompt: String,            // built from SystemPromptBuilder
    tools: Vec<ToolDefinition>,       // registered at startup
    history: ConversationHistory,     // accumulated per-session
    cost_tracker: CostTracker,        // per-session cost
    feedback_sink: FeedbackSink,      // learning events
    streaming_tx: Option<StreamTx>,   // optional streaming channel
}

impl InferenceGateway {
    pub fn new(config, workspace_root) -> Self { ... }
    pub async fn send(&mut self, msg: &str) -> Result<Response> { ... }
    pub async fn send_streaming(&mut self, msg: &str, tx: StreamTx) -> Result<()> { ... }
    pub fn set_system_prompt(&mut self, prompt: &str) { ... }
    pub fn set_effort(&mut self, effort: Effort) { ... }
    pub fn conversation_history(&self) -> &[Message] { ... }
}
```

### What this solves automatically

| Issue | How | S# |
|---|---|---|
| No system prompt | Built at startup from SystemPromptBuilder | S1.1 |
| No conversation history | Accumulated in gateway.history | S1.4 |
| No workspace context | Injected into system prompt at startup | S1.5 |
| Fresh client per request | One client per gateway instance | S2.1-3 |
| No streaming | `send_streaming()` feeds StreamingState | S6.1,4 |
| Hardcoded model/URL | Read from ProviderConfig | S7.1-5 |
| `/system` is no-op | `gateway.set_system_prompt()` | S3.1 |
| `/effort` is no-op | `gateway.set_effort()` | S3.2 |
| Config loaded per-call | Loaded once into gateway | S2.5 |
| Cost tracking at agent level | CostTracker computes from tokens × rates | CT1-3 (doc 20) |
| No timeout on dispatch | Client built with timeouts from config | S2.6-7 |

**Count**: 1 abstraction resolves ~25 individual issues.

### Migration path

1. Build `InferenceGateway` in `roko-agent`
2. Wire `chat_inline.rs` to use it (replaces `dispatch_direct`)
3. Wire `run.rs` to use it (replaces the direct dispatch TODO at line 1271)
4. Wire `ModelCallService` to delegate to it (replaces per-message agent creation)
5. Delete `dispatch_direct.rs`

**Test**: `roko` → multi-turn conversation with tools, streaming output, system prompt visible.

---

## Phase 2: Streaming End-to-End (8-10h)

**Depends on**: Phase 1 (gateway provides `send_streaming`)

| Change | File | LOC |
|---|---|---|
| Connect StreamingState to gateway's streaming channel | `chat_inline.rs:1457-1491` | ~40 |
| Claude CLI: forward stream-json line-by-line | `claude_cli_agent.rs` | ~60 |
| Anthropic API: `"stream": true` + SSE parser | `inference_gateway.rs` | ~100 |
| OpenAI-compat: `"stream": true` + SSE parser | `inference_gateway.rs` | ~80 |
| Plan execution: print agent progress to terminal | `runner/event_loop.rs:338-345` | ~30 |
| `roko run`: show real-time output | `run.rs:583-678` | ~30 |

**Test**: `roko` → send a long prompt → tokens appear incrementally, not all-at-once.

---

## Phase 3: Tool Registry (4-5h)

**Depends on**: Phase 1 (gateway has `tools` field)

| Change | File | LOC |
|---|---|---|
| Register builtin tools in gateway at startup | `inference_gateway.rs` | ~30 |
| Include tool definitions in Anthropic API requests | `inference_gateway.rs` | ~20 |
| Include tool definitions in OpenAI-compat requests | `inference_gateway.rs` | ~20 |
| Handle tool_use responses (dispatch to tool, feed result back) | `inference_gateway.rs` | ~80 |
| MCP tool passthrough from config | `inference_gateway.rs` | ~30 |
| Knowledge store query as a tool | New tool definition | ~40 |

**What this resolves**: S1.3 (no tools), S1.7 (knowledge not in chat), S1.8 (no MCP),
plus mori-diffs GAP-03 (tool system parity).

**Test**: `roko` → "read the contents of Cargo.toml" → model uses file_read tool → works.

---

## Phase 4: Cleanup & Learning Wiring (8-10h)

Now that the core architecture is right, wire the subsystems that were built but disconnected:

| Change | What it fixes | Hours |
|---|---|---|
| Auto-trigger episode compaction | S8.1 | 0.5h |
| Persist LinUCB bandit weights | S8.3, LR1 | 1h |
| Dream trigger → inline at plan completion | S8.2, KS2 | 1.5h |
| Wire MaxCostPerTurn to gateway cost tracker | S8.5, SAF3 | 1h |
| CancellationToken in chat dispatch | S9.4-5 | 1h |
| Auth detection timeout | S9.1 | 0.5h |
| MCP stderr → log file | S9.2 | 0.5h |
| parking_lot::Mutex swap | S11.1-2 | 0.5h |
| Top-10 .ok() audit in orchestrate.rs | S4.1 partial | 1.5h |
| Remove crate-level lint suppression | S11.5 | 0.5h |

---

## Phase 5: Code Consolidation (10-12h) — Optional

This phase addresses S10 (duplicate code). It's valuable but high-risk and can be deferred.

| Change | What | Risk |
|---|---|---|
| Deprecate legacy PlanRunner in orchestrate.rs | S10.1 | High — 21K LOC, many features only exist here |
| Merge two chat event loops | S10.2 | Medium |
| Delete chat.rs (legacy) | S10.5 | Low |
| Unify init paths | S10.6 | Low |
| Route all dispatch through InferenceGateway | S10.3 | Low (Phase 1 does most of this) |

**Recommendation**: Do S10.3 (already done by Phase 1), S10.5, and S10.6. Defer S10.1 (the
21K-line merge) to a dedicated sprint with full test coverage.

---

## Cross-Directory Resolution

### binary-issues (MASTER-INDEX)

| Systemic # | Status after Phase 4 | Notes |
|---|---|---|
| S1 | **Resolved** (Phase 1+3) | InferenceGateway is the session agent |
| S2 | **Resolved** (Phase 1) | Shared client in gateway |
| S3 | **Mostly resolved** (Phase 1) | /system, /effort wired; /run inline deferred |
| S4 | **Partially resolved** (Phase 4) | Top-10 .ok() fixed, full sweep deferred |
| S5 | **Resolved** (Phase 0) | Security hardened |
| S6 | **Resolved** (Phase 2) | End-to-end streaming |
| S7 | **Resolved** (Phase 1) | Config-driven, not hardcoded |
| S8 | **Mostly resolved** (Phase 4) | Phantom features wired |
| S9 | **Mostly resolved** (Phase 4) | Timeouts, cancellation, stderr |
| S10 | **Partially resolved** (Phase 1+5) | dispatch_direct deleted; PlanRunner deferred |
| S11 | **Resolved** (Phase 4) | parking_lot + pattern fixes |

### mori-diffs (12 GAP clusters)

| GAP | Status | Notes |
|---|---|---|
| GAP-01 (dispatch parity) | **Resolved** by Phase 1 | InferenceGateway = mori's InferenceGateway |
| GAP-02 (streaming) | **Resolved** by Phase 2 | End-to-end streaming |
| GAP-03 (tool system) | **Resolved** by Phase 3 | Tool registry wired |
| GAP-04 (session state) | **Resolved** by Phase 1 | Gateway holds conversation |
| GAP-05 (cost tracking) | **Resolved** by Phase 1 | CostTracker in gateway |
| GAP-06 (two runtimes) | **Partially** by Phase 5 | PlanRunner merge deferred |
| GAP-07 (permissions) | **Resolved** by Phase 0 | Security hardened |
| GAP-08 (knowledge) | **Resolved** by Phase 3 | Knowledge as tool |
| GAP-09 (learning) | **Resolved** by Phase 4 | Bandit weights, compaction |
| GAP-10 (config) | **Resolved** by Phase 1 | Config loaded once, used |
| GAP-11 (security) | **Resolved** by Phase 0 | Auth, CORS, terminal |
| GAP-12 (observability) | **Partially** | StateHub unification deferred |

### subsystem-audits

| Phase | Status | Notes |
|---|---|---|
| Phase 0-3 (already done) | N/A | |
| Phase 4 (dispatch unification) | **Resolved** by Phase 1 | InferenceGateway |
| Phase 5 (learning wiring) | **Resolved** by Phase 4 | |
| Phase 6 (consolidation) | **Partially** by Phase 5 | |

### converge-runner (55 open issues)

After Phases 0-4, estimate ~35 of 55 converge-runner issues resolve as side effects
(they're symptoms of the same root causes). Remaining ~20 are subsystem-specific wiring
that can be addressed incrementally.

---

## Summary

| Phase | Hours | What you get |
|---|---|---|
| 0: Security | 3h | Safe to deploy |
| 1: InferenceGateway | 12-15h | Chat with tools, history, context, cost tracking |
| 2: Streaming | 8-10h | Real-time token output |
| 3: Tool Registry | 4-5h | Model can read files, search, run commands |
| 4: Cleanup | 8-10h | Learning works, errors surfaced, subprocesses managed |
| 5: Consolidation (optional) | 10-12h | One engine, one chat loop, one init |
| **Total (without Phase 5)** | **~35-43h** | |
| **Total (with Phase 5)** | **~45-55h** | |

**vs Solution A**: Same hours for Phases 0+4 (~11-13h), but Phase 1 replaces ~6 surgical
batches AND unblocks streaming and tools. Net: fewer total hours for dramatically more
capability.
