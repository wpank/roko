# Implementation roadmap

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Merges the "Implementation path", "Bardo source references", and "Migration from v1" sections.

---

## Implementation path

### Phase 1: auth + secrets (mostly done)

Already shipped in commit `5af205d3`:
- Secrets HTTP API (GET/POST/DELETE/test)
- Multi-key auth middleware (X-Api-Key + Bearer)
- API key scopes + SHA-256 + expiry
- `roko login` CLI with credential store
- Agent CRUD (create/start/stop/restart/list)
- ProcessSupervisor integration

Remaining:
- Privy JWT validation (real JWKS, not structural stub)
- Scope enforcement at route level
- Device flow for headless CLI login

### Phase 2: agent runtime

Port the 9-step heartbeat pipeline into the existing `AgentRuntime`:

- Define `AgentRuntime` struct with cortical state, extensions, clock
- Implement `TickPipeline` with T0/T1/T2 gating
- Add `AgentMode` enum (ephemeral/persistent/reactive) to agent lifecycle
- Wire `AdaptiveClock` (gamma/theta/delta timescales)
- Define `Extension` trait with 22 hooks across 8 layers
- Build domain profile system (string-based, user-extensible, with built-in defaults for coding/research/chain)
- Support user-authored profiles in ~/.roko/profiles/*.toml
- Wire prediction error tracking and sleepwalk fallback

Depends on: Phase 1 (agent CRUD exists).

See [Agent Runtime](02-agent-runtime.md) and [Extensions](03-extensions.md).

### Phase 3: relay + dashboard integration

Convert all data flows to subscription-only:

- Define WebSocket message envelope (seq, ts, room, type, payload)
- Implement room-based subscription in the relay
- Add reconnection with sequence-based replay
- Add backpressure and coalescing strategies
- Dashboard: replace all polling with WebSocket subscriptions
- Dashboard: graceful degradation when roko-serve is down
- Dashboard: subscribe per-page, unsubscribe on unmount
- Test: dashboard works with relay only (no roko-serve)

Depends on: Phase 2 (agents publish heartbeats to relay).

See [Connectivity and Relay](04-connectivity.md) and [Dashboard Architecture](15-dashboard.md).

### Phase 4: inference gateway

Centralize all LLM API key management:

- Build `InferenceGateway` struct with request queue and provider backends
- Build `InferenceHandle` (channel-based, no secrets)
- Wire `CascadeRouter` as the model selection layer
- Add L1 (exact) and L2 (semantic) response caching
- Add per-request, per-agent, per-model cost tracking
- Add `/api/inference/proxy` endpoint for remote agents
- Publish cost_update events to relay
- Remove API key passing from agent environment

Depends on: Phase 2 (agents use InferenceHandle).

See [Inference Gateway](07-gateway.md).

### Phase 5: agent feeds, dynamic endpoints, and paid subscriptions

Enable agents to produce, consume, and monetize real-time data streams:

- Define `FeedRegistration` and `FeedSubscription` types
- Add feed registry to the relay
- Implement `FeedPublisherExt` extension for chain agents
- Add feed discovery API (`GET /api/feeds`)
- Dashboard: Feeds page with live data visualization
- Agent-to-agent feed subscription
- Paid feed support with budget integration
- Dynamic endpoint registration at runtime

Depends on: Phase 3 (relay handles subscriptions), Phase 4 (cost tracking for paid feeds).

See [Feeds and Data Streams](05-feeds.md) and [Paid Feeds and MPP](06-paid-feeds.md).

### Phase 6: clusters + coordination

Enable multi-agent pipelines:

- Define `Cluster` type with pipeline DAG
- Cluster creation API (POST /api/clusters)
- Pipeline stage execution with dependency ordering
- Shared context distribution to cluster agents
- Dashboard: cluster pipeline visualization
- Cluster lifecycle management (stop all, destroy)
- Event publishing to `cluster:{id}` room

Depends on: Phase 2 (agent lifecycle), Phase 3 (relay subscriptions).

See [Deployment](17-deployment.md) (clusters section).

### Phase 7: isolated execution (Fly Machines)

Full agent isolation for untrusted workloads:

- `FlyMachineManager` implementation
- `roko agent run --relay ... --inference-proxy ...` child mode
- Inference proxying through parent gateway (uses Phase 4 endpoint)
- Volume management for persistent state
- Auto-suspend for reactive agents (Fly Machine stop/start)
- Network policy: outbound-only from Fly Machine

Depends on: Phase 4 (inference proxy), Phase 3 (relay connectivity).

See [Deployment](17-deployment.md) (scaling section).

### Phase 8: multi-tenant

Organization and team support:

- Organization model with members and roles
- Invitation-based onboarding
- Per-org resource isolation (separate agent namespaces)
- Per-org billing (aggregate cost tracking)
- Dashboard: team management UI

Depends on: Phase 1 (auth), Phase 4 (cost tracking).

---

## Bardo source references

Everything in this redesign has prior art in the bardo codebase (`/Users/will/dev/uniswap/bardo/`). This section maps each component to its bardo implementation.

### Inference gateway -- bardo-gateway (22.8K LOC)

The gateway already exists. `apps/bardo-gateway/` is a production LLM inference proxy with:

- **3-layer cache**: hash (exact match), semantic (embedding similarity), prefix (prompt prefix)
- **5 provider backends**: Anthropic, OpenAI, OpenRouter, Venice, Bankr
- **Tool pruning**: strips unused tool definitions to reduce token count
- **Batch API**: Anthropic batch endpoint for async, cheaper inference
- **Cost tracking**: per-request, per-model, per-session with SQLite persistence
- **WebSocket stats**: `/v1/ws/stats` streams snapshots + events to dashboard

Port the cache, provider abstraction, and cost tracking. Skip the batch API and USDC micropayments for now.

| File | LOC | What |
|------|-----|------|
| `apps/bardo-gateway/src/` | 22,856 | Full gateway server |
| `crates/bardo-inference/src/` | 413 | Protocol types |
| `crates/golem-inference/src/client.rs` | 723 | Gateway HTTP client |

### Agent runtime -- mori (108K LOC)

Mori is the production orchestrator that roko-cli/orchestrate.rs replaces. Key patterns:

- **Process group isolation**: `libc::setpgid(0, 0)` per agent, SIGTERM then SIGKILL with 200ms grace
- **MultiAgentPool + warm spawning**: pre-spawn warm agents during gate overlap
- **26 agent roles**: with priority scheduling
- **3 LLM backends**: Claude CLI, Codex, Cursor ACP
- **Rate limiter**: 8-agent default concurrency, priority-sorted queue
- **Conductor**: 10 watchers with 3-tier interventions (Nudge/Restart/Abort)

Port the warm spawning, conductor watchers, and process isolation. The agent model itself is redesigned (heartbeat pipeline replaces mori's event loop).

| File | LOC | What |
|------|-----|------|
| `apps/mori/src/agent/connection.rs` | 3,358 | Agent spawn/kill lifecycle |
| `apps/mori/src/agent/mod.rs` | 400+ | MultiAgentPool + warm spawning |
| `apps/mori/src/conductor/mod.rs` | 600+ | Conductor + 10 watchers |

### Heartbeat -- golem-heartbeat (10.2K LOC)

Full 9-step CoALA pipeline. Built but never integrated into mori's runtime. This is the core of the redesigned agent runtime.

- **9-step tick**: Observe, Retrieve, Analyze, Gate, Simulate, Validate, Execute, Verify, Reflect
- **AdaptiveClock**: gamma/theta/delta timescales
- **T0/T1/T2 gating**: prediction error threshold
- **VCG attention auction**: 6 cognitive bidder kinds

Port the full pipeline. Wire it as the `TickPipeline` inside `AgentRuntime`.

| File | LOC | What |
|------|-----|------|
| `crates/golem-heartbeat/src/engine.rs` | 1,307 | HeartbeatEngine |
| `crates/golem-heartbeat/src/pipeline.rs` | 3,019 | 9-step TickPipeline |
| `crates/golem-heartbeat/src/gating.rs` | 481 | PredictionError, AdaptiveGate |
| `crates/golem-heartbeat/src/auction.rs` | 1,112 | VCG AttentionAuction |
| `crates/golem-heartbeat/src/clock.rs` | 470 | AdaptiveClock |

### DeFi tools -- golem-tools (7.2K LOC)

29+ tool categories with capability-gated execution. Port the ToolExecutor framework and the vault/identity tools.

### Chain runtime -- golem-chain (5.3K LOC)

12 networks, ProviderPool, SubgraphClient, RevmSimulator, Warden time-delay safety.

### Dashboard -- apps/dashboard (Next.js, ~27K LOC)

20 React components, real-time WebSocket, canvas-based charts. Port the monitoring components. Add agent management, settings UI, and feeds page (new).

---

## Migration from v1

This section documents what changed between the v1 redesign and this revision.

| v1 design | v2 design | Why |
|-----------|-----------|-----|
| Per-agent HTTP sidecar (13 routes) | No sidecar. Agents are tokio tasks or outbound-WS-connected processes. | Sidecars break behind NAT. Channels are faster for in-process. Relay handles remote. |
| Single `roko` deployment is the backbone | Relay + Mirage is the backbone. `roko` is optional. | Dashboard should work without the control plane. Relay is the universal bus. |
| Dashboard polls REST endpoints | Subscription-only via WebSocket | Polling wastes bandwidth, creates jitter, scales poorly. |
| Agent discovery from roko-serve only | Three sources: relay, chain, deployment list | Each source has different strengths. Merge client-side. |
| API keys in agent env vars | Centralized InferenceGateway, agents get channel handle | Eliminates key sprawl, enables cost tracking, audit, rotation. |
| No agent modes | Ephemeral / Persistent / Reactive | Different workloads need different lifecycles. |
| Agent specialization via code | Extension chain composition | Extensions are composable and configurable. No code forks. |
| No chain data feeds | Agents expose and subscribe to real-time feeds | Creates a data marketplace. Dashboard subscribes to same feeds. |
| No dynamic endpoints | Agents register endpoints at runtime | Enables feed discovery and subscription. |

---

## Bardo → Roko naming map

> Folded from `tmp/bardo-integration-plan.md`. Essential reference for porting work.

| Bardo Crate | Roko Crate | Status | Notes |
|-------------|-----------|--------|-------|
| golem-core | roko-core | Migrated | |
| golem-runtime | roko-runtime | Migrated | |
| golem-grimoire | roko-neuro | Partial | Renamed grimoire → neuro |
| golem-daimon | roko-daimon | Migrated | |
| golem-dreams | roko-dreams | Migrated | |
| golem-chain | roko-chain | Partial | |
| golem-tools | roko-std | Partial | 19 builtin tools, no DeFi |
| golem-heartbeat | roko-conductor + roko-runtime | Partial | Split into two crates |
| golem-safety | roko-agent/safety | Migrated | |
| golem-eval | roko-gate | Migrated | |
| golem-inference | roko-gateway (new) | Not ported | See [07-gateway.md](07-gateway.md) |
| golem-triage | roko-orchestrator (merge) | Not ported | |
| golem-context | roko-compose | Partial | VCG exists |
| golem-identity | roko-chain (merge) | Not ported | |
| bardo-gateway | roko-gateway (new) | Not ported | Key missing piece |
| dashboard | apps/dashboard (new) | Not ported | See [15-dashboard.md](15-dashboard.md) |
| mori | roko-cli/src/orchestrate.rs | Reference only | 108K LOC reference |
| mpp | roko-mpp (new) | Not ported | Payments, optional |

### Bardo source reference (LOC counts)

| Component | Bardo Path | LOC | Roko equivalent |
|-----------|-----------|-----|-----------------|
| Inference gateway | `bardo/apps/bardo-gateway/` | 22,800 | `crates/roko-gateway/` (new) |
| Agent runtime (mori) | `bardo/apps/mori/` | 108,000 | `crates/roko-cli/src/orchestrate.rs` |
| Heartbeat | `bardo/crates/golem-heartbeat/` | 10,200 | `crates/roko-conductor/` |
| DeFi tools | `bardo/crates/golem-tools/` | 7,200 | `crates/roko-std/` |
| Chain runtime | `bardo/crates/golem-chain/` | 5,300 | `crates/roko-chain/` |
| Dashboard | `bardo/apps/dashboard/` | 27,000 | `apps/dashboard/` (new) |
| Terminal | `bardo/apps/bardo-terminal/` | ~4,000 | `crates/roko-cli/src/tui/` |
| MPP | `bardo/crates/mpp/` | 988 | `crates/roko-mpp/` (new) |

---

## Implementation task summary

> Folded from `tmp/bardo-integration-plan.md`. 48 tasks across 12 phases.

| Phase | Tasks | Priority | Parallelizable | Spec doc |
|-------|-------|----------|----------------|----------|
| 1. Inference Gateway | 12 | P0 | No (sequential foundation) | [07-gateway.md](07-gateway.md) |
| 2. Orchestrator Gaps | 7 | P0 | Yes (with Phase 1) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) |
| 3. Learning Loop Gaps | 5 | P1 | Yes (with Phases 1-2) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) |
| 4. Heartbeat Pipeline | 2 | P1 | Yes (with Phases 1-3) | [02-agent-runtime.md](02-agent-runtime.md) |
| 5. Agent Modes | 3 | P1 | After Phase 2 | [02-agent-runtime.md](02-agent-runtime.md) |
| 6. Dashboard | 4 | P1 | After Phase 1.11 | [15-dashboard.md](15-dashboard.md) |
| 7. DeFi Tools + Chain | 4 | P2 | Yes (standalone) | [12-defi.md](12-defi.md), `defi/gap/` |
| 8. TUI Enhancements | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-operations.md) |
| 9. Operational Infra | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-operations.md) |
| 10. Fly Machines | 2 | P2 | After Phase 5 | [17-deployment.md](17-deployment.md) |
| 11. Clusters | 2 | P3 | After Phases 4-5 | [17-deployment.md](17-deployment.md) |
| 12. Payments | 1 | P3 | After Phase 1 | [06-paid-feeds.md](06-paid-feeds.md) |
| **Total** | **48** | | | |

### Dependency graph

```
Phase 1 (Gateway) ──→ Phase 6 (Dashboard)
     │                Phase 10 (Fly Machines)
     └─→ Phase 12 (Payments)

Phase 2 (Orchestrator) ──→ Phase 5 (Agent Modes) ──→ Phase 10 (Fly)
Phase 3 (Learning) ──→ (standalone)
Phase 4 (Heartbeat) ──→ Phase 11 (Clusters)
Phase 5 (Agent Modes) ──→ Phase 11 (Clusters)

Phase 7 (DeFi) ──→ (standalone, parallel with everything)
Phase 8 (TUI) ──→ (standalone)
Phase 9 (Ops) ──→ (standalone)
```

### Critical path

**Phase 1 (Gateway) → Phase 6 (Dashboard) → Phase 10 (Fly)**

### Parallel tracks

Phases 2+3+4 (Orchestrator + Learning + Heartbeat) can run alongside Phase 1. Phases 7+8+9 (DeFi + TUI + Ops) are fully independent.

---

## Phase 1 task breakdown: Inference Gateway

> Detailed per-task specification with source references. See [07-gateway.md](07-gateway.md) for the architectural spec.

| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 1.1 | Port inference protocol types | `bardo/crates/bardo-inference/src/lib.rs` (413), `golem-inference/src/client.rs` (723) | `crates/roko-gateway/src/types.rs` | S |
| 1.2 | Port hash cache (Layer 1) | `bardo/apps/bardo-gateway/src/cache.rs` | `crates/roko-gateway/src/cache/hash_cache.rs` | M |
| 1.3 | Port semantic cache (Layer 2) | `bardo/apps/bardo-gateway/src/semantic_cache.rs` | `crates/roko-gateway/src/cache/semantic_cache.rs` | M |
| 1.4 | Port provider abstraction + key rotation | `bardo/apps/bardo-gateway/src/providers/` | `crates/roko-gateway/src/providers/` | L |
| 1.5 | Port cost computation + tracking | `bardo/apps/bardo-gateway/src/pricing.rs`, `handler.rs`, `cost_db.rs` | Wire into `roko-learn/src/costs_db.rs` + new `pricing.rs` | M |
| 1.6 | Port loop detection | `bardo/apps/bardo-gateway/src/loop_guard.rs` | `crates/roko-gateway/src/loop_guard.rs` | M |
| 1.7 | Port output budgeting | `bardo/apps/bardo-gateway/src/output_budget.rs` | `crates/roko-gateway/src/output_budget.rs` | S |
| 1.8 | Port tool pruning | `bardo/apps/bardo-gateway/src/tools.rs` | `crates/roko-gateway/src/tool_pruning.rs` | S |
| 1.9 | Port convergence detection | `bardo/apps/bardo-gateway/src/convergence.rs` | `crates/roko-gateway/src/convergence.rs` | S |
| 1.10 | Port thinking cap | `bardo/apps/bardo-gateway/src/thinking_cap.rs` | `crates/roko-gateway/src/thinking_cap.rs` | S |
| 1.11 | Wire gateway into roko-serve | — | `crates/roko-serve/src/routes/gateway.rs` | L |
| 1.12 | Port batch API | `bardo/apps/bardo-gateway/src/batch.rs` | `crates/roko-gateway/src/batch.rs` | M |

**Sequence**: 1.1 → 1.2 → 1.3 → 1.4 → 1.5 → (1.6, 1.7, 1.8, 1.9, 1.10 in parallel) → 1.11 → 1.12

---

## Phase 4 task breakdown: Heartbeat Pipeline

> See [02-agent-runtime.md](02-agent-runtime.md) for the full 9-step pipeline spec.

| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 4.1 | Port 9-step TickPipeline | `golem-heartbeat/src/pipeline.rs` (3,019), `engine.rs` (1,307) | `crates/roko-conductor/src/tick_pipeline.rs` | L |
| 4.2 | Wire T0/T1/T2 at dispatch time | `golem-heartbeat/src/gating.rs` (481) | Modify `dispatch_agent_with()` in orchestrate.rs | M |

---

## Phase 5 task breakdown: Agent Modes + Profiles

> See [02-agent-runtime.md](02-agent-runtime.md) for mode and profile specs.

| Task | Description | Target | Size |
|------|------------|--------|------|
| 5.1 | Add AgentMode + AgentProfile enums | `crates/roko-core/src/config/schema.rs` | S |
| 5.2 | Wire ephemeral auto-stop | `roko-serve/src/routes/agents.rs` + `roko-runtime/src/process.rs` | S |
| 5.3 | Wire reactive mode (webhook/cron) | New `crates/roko-runtime/src/reactive.rs` | L |

---

## Phase 6 task breakdown: Dashboard

> See [15-dashboard.md](15-dashboard.md) for the dashboard architecture spec.

| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 6.1 | Set up Next.js app in monorepo | `bardo/apps/dashboard/`, `bardo/packages/ui/` | `apps/dashboard/`, `packages/ui/` | S |
| 6.2 | Add Privy auth | — | `apps/dashboard/src/app/login/`, AuthProvider | M |
| 6.3 | Add agent management pages | — | `apps/dashboard/src/app/agents/` | L |
| 6.4 | Add settings page | — | `apps/dashboard/src/app/settings/` | M |

---

## Phase 10-12 task breakdowns

> See [17-deployment.md](17-deployment.md) for Fly Machines and clusters specs.

| Task | Phase | Description | Target | Size |
|------|-------|------------|--------|------|
| 10.1 | Fly | Fly Machines REST API client | `crates/roko-runtime/src/fly.rs` | M |
| 10.2 | Fly | Extend ProcessSupervisor for Fly | `crates/roko-runtime/src/process.rs` | L |
| 11.1 | Clusters | Wire FleetConductor (L4) | `crates/roko-conductor/src/federation.rs` | M |
| 11.2 | Clusters | Cluster API routes | `crates/roko-serve/src/routes/clusters.rs` | L |
| 12.1 | Payments | Port MPP (ERC-3009 USDC) | `crates/roko-mpp/` (new) | M |
