# Solution 1 — The Service Triad

**Philosophy**: Three services are the missing spine. Every subsystem audit (gateway, dispatch,
prompts, UX, ACP) describes a different view of the same gap: there's no shared infrastructure
between entry points. Build the three services that all callers route through. Design them
to work embedded (in-process) OR remote (HTTP) from day one.

**Total estimate**: ~70-90 hours
**What it addresses**: All 6 subsystem audits, all 11 systemic problems (S1-S11), all 12
mori-diffs GAP clusters, ~45 of 55 converge-runner issues.

---

## Why Three Services, Not One

My earlier "InferenceGateway" proposal was a single struct holding everything. That's wrong.
The subsystem audits show three distinct concerns that should be independently deployable:

| Service | What it owns | Can be remote? | Subsystem audits addressed |
|---|---|---|---|
| **InferenceGateway** | HTTP client, provider routing, caching, cost, streaming, safety | Yes — HTTP proxy | gateway, inference-dispatch |
| **PromptAssemblyService** | 9-layer builder, VCG auction, context bidding, section effectiveness | No — lives with caller | prompt-assembly |
| **SessionService** | Conversation history, tool registry, workspace context, feedback recording | No — lives with caller | ux, acp-protocol |

The InferenceGateway is already designed as a library-or-standalone crate in the gateway
audit. The PromptAssemblyService already exists (`roko-compose/src/prompt_assembly_service.rs`)
but is only wired into `roko plan run`. The SessionService is what's completely missing.

---

## Service 1: InferenceGateway (`roko-gateway`)

**Source**: `tmp/subsystem-audits/gateway/` (9 docs, complete spec)

### What it is

A transparent HTTP proxy that accepts Anthropic/OpenAI wire format, routes to upstream
providers, and injects intelligence (routing, caching, optimization, safety, cost tracking)
on every request. Designed as a **library crate** embeddable in `roko-serve` OR a standalone
binary.

### What it replaces

| Current code | LOC | Replaced by |
|---|---|---|
| `dispatch_direct.rs` (bare HTTP calls) | 500 | Gateway provider forward |
| `reqwest::Client::new()` per-call (S2) | scattered | Shared client per provider |
| 4 copies of stream-json parser | ~600 | One `ClaudeStreamParser` |
| Hardcoded model/URL/version (S7) | ~20 sites | Config-driven provider registry |
| CascadeRouter (built, never called) | ~1500 | Gateway routing layer |
| CostTable (8 models, $0 for unknown) | ~120 | Config-driven pricing |
| `naive_opus_cost()` hardcoded | ~30 | Real cost headers per request |

### Key design properties

1. **Format-agnostic**: Accepts Anthropic Messages API or OpenAI Chat Completions. Auto-detects
   wire format, normalizes to internal `ProxyRequest`, responds in original format.

2. **Pipeline architecture**: 9-stage linear pipeline, each stage can veto, transform, or redirect:
   ```
   LoopDetect → CacheLookup → ToolPrune → OutputBudget → ThinkingCap
     → ConvergenceDetect → ProviderCall → CacheStore → CostTrack
   ```

3. **Remote-capable from day one**: When embedded in `roko-serve`, agents make in-process calls.
   When deployed standalone, agents make HTTP calls to `ROKO_GATEWAY_URL`. Same code path.
   This means:
   - Local dev: gateway runs in-process (zero latency overhead)
   - Team: shared gateway with multi-key routing (one Anthropic API key, many agents)
   - Production: gateway on Railway/Fly with caching, billing, analytics

4. **Learning integration**: Every request produces passive learning signal (cost, latency, cache
   hit). With feedback endpoint (`POST /v1/feedback`), gate results feed back into routing.
   CascadeRouter finally gets called on every model invocation, not just dead code.

5. **Streaming as first-class**: SSE passthrough with async tapping. Stream bytes to client
   immediately, tap asynchronously for token counting and cost. No buffering.

6. **3-layer caching**: L1 hash (BLAKE3, exact match), L2 semantic (SimHash, near-match),
   L3 prefix (Anthropic cache_control injection). Config-driven, each layer optional.

### What this solves (by systemic problem)

| S# | How gateway solves it |
|---|---|
| S1 (thin pipe) | All dispatch goes through gateway — tools, system prompt, history sent by caller via standard API |
| S2 (throwaway clients) | One client per provider, connection pooled |
| S5 (security) | Gateway can enforce auth, rate limits, PII scanning |
| S6 (no streaming) | SSE passthrough built into pipeline |
| S7 (hardcoded values) | Config-driven providers, models, pricing |
| S8 (phantom features) | CascadeRouter wired into every request |

### What this does NOT solve

- Prompt assembly (caller's responsibility — they send the system prompt)
- Session state (caller's responsibility — they send conversation history)
- Tool dispatch (caller's responsibility — they handle tool_use responses)
- UX/surfaces (separate concern)

### Effort: ~20-25h for Phases 1-3 of gateway spec

---

## Service 2: PromptAssemblyService (wire existing `roko-compose`)

**Source**: `tmp/subsystem-audits/prompt-assembly/` (4 docs)

### What it is

The 9-layer `SystemPromptBuilder` and `PromptAssemblyService` already exist and work
correctly — but only `roko plan run` and `roko run` use them. The service needs to be wired
into every entry point, not rebuilt.

### Current state

| Entry point | Uses prompt assembly? | What it gets |
|---|---|---|
| `roko plan run` | **Yes** — full 9-layer + knowledge + playbooks | Rich context, role-specific |
| `roko run` | **Yes** — `build_role_system_prompt_validated()` | Good but less enriched |
| `roko chat` | **No** — bare text pipe | Nothing |
| `roko "prompt"` | **No** — `dispatch_direct` | Nothing |
| ACP | **No** — hardcoded format strings | Inline role descriptions |
| Agent sidecar | **No** | Nothing |

### What needs to happen

Not building new code — **wiring existing code to all callers**:

1. `SessionService` (below) calls `PromptAssemblyService.assemble()` on session start
   and re-assembles when context changes (new tools, mode switch, knowledge update)

2. ACP runner replaces hardcoded "Architect Reviewer" / "Security Auditor" strings with
   `ReviewerTemplate` / `AuditorTemplate` calls

3. All callers send the assembled system prompt to the gateway as standard API `system` field

### What this solves

| Issue | How |
|---|---|
| S1.1 (no system prompt) | Every call gets 9-layer prompt |
| S1.5 (no workspace context) | Layer 3 (domain context) includes workspace |
| S1.7 (knowledge not in chat) | Layer 7 (anti-patterns) from neuro store |
| S3.1 (`/system` no-op) | `/system` mutates session's prompt assembly |
| S3.2 (`/effort` no-op) | `/effort` adjusts composition budget |
| Prompt-assembly bypass (ACP) | ACP uses templates |

### Effort: ~5-8h (wiring, not building)

---

## Service 3: SessionService (the missing piece)

**Source**: Inferred from gaps in all audits — no single audit describes this because it
doesn't exist. Every audit complains about its absence from a different angle.

### What it is

A session-scoped service that owns the state that makes an agent conversation work:
conversation history, tool registry, workspace context, mode, feedback recording. This is
what sits between the user-facing entry point and the gateway + prompt assembly.

### What it owns

```
SessionService
├── conversation_history: Vec<Message>      // accumulated, context-windowed
├── tool_registry: ToolRegistry             // builtin + MCP + workspace tools
├── workspace: WorkspaceContext             // path, git branch, file listing
├── prompt_assembly: PromptAssemblyService  // 9-layer builder (Service 2)
├── gateway_handle: InferenceHandle         // channel to gateway (Service 1)
├── feedback_sink: FeedbackSink             // episode, efficiency, routing events
├── mode: SessionMode                       // code / plan / research
├── config: SessionConfig                   // model, effort, gates, workflow
└── cost_tracker: SessionCostTracker        // per-session cumulative cost
```

### How entry points use it

| Entry point | Creates SessionService? | What changes |
|---|---|---|
| `roko` (chat) | Yes — long-lived | Full interactive session |
| `roko "prompt"` (oneshot) | Yes — ephemeral | Single turn, then exit |
| `roko run` | Yes — ephemeral | Assembles rich prompt, single dispatch |
| `roko plan run` | Yes — per-task | Task-scoped session with role switching |
| ACP | Yes — per editor session | FSM pipeline uses session for each phase |
| Agent sidecar | Yes — per-agent | Sidecar wraps session |

### The conversation loop

```
User message
  → SessionService.send(message)
    → Append to history
    → Re-assemble system prompt (if stale)
    → Format request (system prompt + history + tools)
    → Send to InferenceGateway (local or remote)
    → Receive streaming response
    → If tool_use: dispatch tool, feed result back, re-send
    → Append assistant response to history
    → Record episode + feedback
    → Return response (or stream)
```

### What this solves

| Issue | How |
|---|---|
| S1.3 (no tools) | Tool registry populated at session start |
| S1.4 (no history) | History accumulated per-session |
| S1.6 (no codebase context) | WorkspaceContext injected |
| S1.8 (no MCP) | MCP tools registered in tool registry |
| S1.9 (run bypasses ModelCallService) | Run goes through session → gateway |
| S3.1-4 (slash commands lie) | Commands mutate session state, take effect next turn |
| S4.5 (episode logger failure) | Feedback recording mandatory in session |
| S4.9 (FeedbackSink optional) | Always present in session |
| S8.6 (human approval stub) | Session handles approval flow |
| ACP missing learning | ACP session records episodes |
| ACP missing safety | Session enforces AgentContract |
| ACP missing SystemPromptBuilder | Session uses PromptAssemblyService |

### Effort: ~15-20h

---

## How Everything Connects

```
┌─────────────────────────────────────────────────┐
│  Entry Points                                    │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌─────┐ ┌───────┐  │
│  │ chat │ │ run  │ │ plan │ │ ACP │ │sidecar│  │
│  └──┬───┘ └──┬───┘ └──┬───┘ └──┬──┘ └───┬───┘  │
│     └────────┴────────┴────────┴─────────┘      │
│                      │                           │
│              ┌───────▼────────┐                  │
│              │ SessionService │ ◄── /system, /effort, /model, /mode  │
│              │  history       │                  │
│              │  tools         │                  │
│              │  workspace     │                  │
│              │  feedback      │                  │
│              └───────┬────────┘                  │
│                      │                           │
│     ┌────────────────┼────────────────┐         │
│     │                │                │         │
│     ▼                ▼                ▼         │
│ ┌────────┐   ┌──────────────┐  ┌──────────┐   │
│ │ Prompt │   │  Inference   │  │ Feedback │   │
│ │Assembly│   │  Gateway     │  │  Sink    │   │
│ │Service │   │ (local/remote)│  │(episodes)│   │
│ └────────┘   └──────┬───────┘  └──────────┘   │
│                      │                          │
│              ┌───────▼────────┐                 │
│              │   Providers    │                 │
│              │ Claude│OpenAI  │                 │
│              │ Gemini│Ollama  │                 │
│              └────────────────┘                 │
└─────────────────────────────────────────────────┘
```

---

## Addressing Each Subsystem Audit

### gateway audit → InferenceGateway (Service 1)
- Transparent proxy: ✅ (HTTP proxy with format auto-detection)
- Routing: ✅ (CascadeRouter integrated)
- Caching: ✅ (3-layer)
- Cost: ✅ (per-request headers)
- Streaming: ✅ (SSE passthrough)
- Remote: ✅ (library or standalone)
- Learning: ✅ (feedback endpoint)

### inference-dispatch audit → InferenceGateway + SessionService
- 13 call sites unified: ✅ (all go through SessionService → Gateway)
- 4 duplicate parsers: ✅ (one ClaudeStreamParser in gateway)
- CascadeRouter called: ✅ (gateway routing layer)
- Episode logging: ✅ (SessionService feedback sink)
- Credential centralization: ✅ (gateway owns API keys)

### prompt-assembly audit → PromptAssemblyService (Service 2)
- All entry points use 9-layer builder: ✅ (via SessionService)
- ACP uses templates: ✅ (not hardcoded strings)
- Section effectiveness learning: ✅ (feedback sink records)
- VCG auction available: ✅ (existing code, better wiring)

### ux audit → SessionService + Gateway
- Task execution API: ✅ (SessionService per-task)
- Streaming to surfaces: ✅ (Gateway SSE → SessionService → TUI/web/ACP)
- Unified ViewModel: partially (SessionService provides canonical state)
- Board/Epic/Task hierarchy: separate work (not blocked by triad)

### acp-protocol audit → SessionService
- Learning integration: ✅ (session records episodes)
- Safety integration: ✅ (session enforces contracts)
- SystemPromptBuilder: ✅ (session uses PromptAssemblyService)
- Multi-backend dispatch: ✅ (session → gateway, any provider)
- Pipeline agents API fallback: ✅ (gateway handles provider selection)

---

## Implementation Order

### Phase 0: Security (3h)
Same as before. Non-negotiable.

### Phase 1: InferenceGateway core (15-18h)
- `roko-gateway/src/lib.rs` — proxy pipeline (9 stages)
- `roko-gateway/src/provider/` — 6 provider adapters (reuse existing `roko-agent` backends)
- `roko-gateway/src/routing/` — CascadeRouter integration
- `roko-gateway/src/cache/` — L1 hash cache (L2/L3 later)
- `roko-gateway/src/cost/` — per-request cost tracking
- `roko-gateway/src/stream/` — SSE passthrough with async tapping
- Embed in `roko-serve` as middleware
- Config: `[gateway]` section in roko.toml

### Phase 2: SessionService (15-20h)
- `roko-agent/src/session.rs` — SessionService struct
- Conversation history with context windowing
- Tool registry (builtin + MCP)
- Workspace context (path, git, files)
- Integration with PromptAssemblyService
- Feedback recording (episodes, efficiency, routing)
- Wire into `chat_inline.rs` (replacing dispatch_direct)
- Wire into `run.rs`
- Wire into ACP runner

### Phase 3: Wire PromptAssemblyService to all callers (5-8h)
- Chat: session builds 9-layer prompt at startup, refreshes on context change
- ACP: replace hardcoded role strings with template calls
- Sidecar: session includes prompt assembly
- `/system`, `/effort`, `/mode` mutate assembly and take effect

### Phase 4: Streaming end-to-end (6-8h)
- Gateway provides streaming by default
- SessionService forwards stream to UI layer
- Connect StreamingState in chat_inline.rs
- Claude CLI stream-json forwarded incrementally
- Plan execution shows per-turn progress

### Phase 5: Tool loop (8-10h)
- SessionService handles tool_use responses from gateway
- Dispatch to tool, feed result back, re-send to gateway
- MCP tools invoked via existing MCP client
- Safety contracts enforced per tool call
- Knowledge store queryable as a tool

### Phase 6: Reliability & cleanup (8-10h)
- Persist LinUCB bandit weights (gateway routing layer)
- Auto-trigger episode compaction
- Dream trigger inline at plan completion
- parking_lot::Mutex swap
- Top-10 .ok() audit
- Subprocess timeouts and cancellation
- Delete `dispatch_direct.rs`, legacy `chat.rs`

---

## Remote Gateway Deployment

The gateway spec explicitly supports this:

```toml
# Local (default): gateway embedded in roko-serve
[gateway]
mode = "embedded"

# Remote: roko connects to shared gateway
[gateway]
mode = "remote"
url = "https://gateway.myteam.dev"
api_key_env = "ROKO_GATEWAY_KEY"
```

Remote mode means:
- **One API key, many developers**: Team shares an Anthropic key via the gateway
- **Shared caching**: If developer A sends a prompt, developer B gets an L1 cache hit
- **Centralized learning**: CascadeRouter learns from all team members' usage
- **Cost visibility**: Admin dashboard shows per-developer spend
- **Rate limiting**: Gateway enforces org-wide budget

This is the LiteLLM/OpenRouter model but with roko's learning loops built in.

---

## What This Architecture Gets Right (That My Previous Solutions Didn't)

1. **Gateway is a crate, not a struct** — deployable standalone for remote inference
2. **Prompt assembly is a separate service** — not jammed into the gateway
3. **Session state is a separate service** — not jammed into the gateway
4. **All entry points share the same three services** — no special cases
5. **Learning happens at the gateway level** — routing intelligence shared across all callers
6. **Feedback happens at the session level** — episodes recorded regardless of entry point
7. **ACP gets full infrastructure** — not a second-class citizen with hardcoded strings

## Trade-offs vs Other Approaches

| vs Solution 2 (Cell/Graph) | Trade-off |
|---|---|
| Less future-proof | Services are conventional — not the unified Cell/Graph model |
| More practical | Working system in ~70h vs ~120h+ for Cell/Graph engine |
| Easier to refactor later | Services can be wrapped in Cell interfaces when ready |

| vs Solution 3 (Mori port) | Trade-off |
|---|---|
| More upfront design | Mori port could be faster for specific features |
| Better architecture | Mori's dispatch was also messy; this is cleaner |
| Handles remote inference | Mori was local-only |
