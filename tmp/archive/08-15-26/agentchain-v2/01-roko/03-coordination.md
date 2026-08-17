# 03 — Coordination

> How Roko interacts with the world and other agents. The inference gateway, feeds and recipes, groups and emergent communication, the four external protocols, extensions and triggers, tools, surfaces, and the design principles for collective intelligence.

---

## 1. The Exoskeleton

Four external protocols form a fixed exoskeleton. Roko defines what flows through them but does not replace them.

| Protocol | Purpose | Roko's role |
|---|---|---|
| **MCP** (Model Context Protocol) | Tool discovery and invocation between LLM agents and external tools | Roko agents consume MCP servers and expose Roko tools as MCP servers |
| **A2A** (Agent-to-Agent) | Agent capability cards, discovery, and inter-agent dispatch | Roko agents publish A2A cards and consume cards from other agents |
| **ERC-8004** | On-chain agent identity, reputation, and validation registries | Each Roko agent has an ERC-8004 passport; reputation flows from gate verdicts |
| **x402** | HTTP-native micropayments | Roko agents pay for tool calls, knowledge queries, feed subscriptions |

These protocols are open standards owned by external bodies. Roko's value is in the orchestration and learning that flows *through* them, not in replacing them. MCP is a Linux Foundation standard with on the order of 97M monthly SDK downloads. ERC-8004 is the Ethereum standard for on-chain agent identity (EIP draft August 2025; mainnet January 29, 2026; by late 2025 approximately 106,996 agents indexed across Base, BSC, and Ethereum). x402 (Coinbase, May 2025) is the emerging standard for agent-to-agent and agent-to-service micropayments with sub-second finality on Base L2; the x402 Foundation was co-founded with Cloudflare. Galaxy Research has reported 75M+ x402 transactions across the ecosystem.

---

## 2. The Inference Gateway

Every LLM call is mediated through a single Pipeline. Concentrating all provider interaction in one place delivers four properties no per-agent implementation can match:

1. **No API keys in agent space.** Agents hold an `InferenceHandle` — a channel sender to the gateway. The agent never sees credentials, never selects a provider directly, never bypasses caching. Compromising an agent does not leak provider secrets.
2. **Universal cost tracking.** Every call is observable. Per-agent budgets and per-model spend are enforced at the gateway.
3. **Cross-cutting optimization.** Caching, batch dispatch, and circuit-breaking apply to every call without per-agent code.
4. **Single attack surface.** Prompt-injection guards and output safety checks are concentrated where they matter.

The gateway is itself a Pipeline Graph. Adding a stage means writing a new Cell and wiring it into the Graph TOML.

### The 9 stages

| # | Stage | What it does |
|---|---|---|
| 1 | Loop Detection | Reject calls that repeat recent inputs verbatim, indicating a stuck agent |
| 2 | Prompt Cache Lookup | Return cached response when prompt + tools + temperature match |
| 3 | Tool Pruning | Drop tools the agent demonstrably will not use given the prompt |
| 4 | Output Budgeting | Set `max_tokens` based on task complexity and remaining budget |
| 5 | Thinking-Cap Negotiation | Pass reasoning effort hints to backends that support them |
| 6 | Convergence Check | Verify the call is making progress toward a goal (not flailing) |
| 7 | Provider Call | The actual LLM invocation |
| 8 | Cache Store | Persist successful responses for future cache hits |
| 9 | Cost Tracking | Charge the agent's vitality budget; emit `cost.charged` Pulse |

Stages 1, 2, 6, and 7 can short-circuit via early-exit edges. Stages 3, 4, 5 are pure transformations. Stage 7 is the only Activity Cell; the rest are Workflow Cells (deterministic, replayable).

### Loop detection

Loop detection uses HDC fingerprinting of the assembled prompt. The gateway maintains a per-agent ring buffer of recent prompt fingerprints. If the new prompt's HDC similarity to any of the last N fingerprints exceeds a threshold (default 0.95), the call is rejected with `LoopDetected`, an `agent.{id}.loop_detected` Pulse is published, and the agent's escalation watcher is notified. The threshold is conservative — false positives would block legitimate retries.

### Prompt cache

Content-addressed prompt caching delivers the largest single cost reduction in production: typically **5×**.

Cache key: `BLAKE3(prompt + sorted_tools + temperature + model_key)`. The cache stores `(InferenceResponse, hit_count, last_hit_at)`. A hit returns immediately without contacting any provider; the cache emits a `cache.hit` Pulse for telemetry.

Cache invalidation is **demurrage-driven**, not TTL-based. Each entry has a balance starting at 1.0; every hit reinforces; demurrage applies the same per-kind decay as the knowledge store; entries below the cold threshold are evicted. Hot prompts (system messages, tool documentation, frequently-recurring task templates) stay cached indefinitely. Rare prompts age out.

For providers that expose prompt caching (Anthropic's `cache_control`), the gateway sets cache-control markers on the prefix-cacheable layers (1+2+5 from the 9-layer prompt builder) so that even when the gateway cache misses, the provider cache may hit. This compounds.

### Tool pruning, output budgeting, thinking cap

Tool pruning uses a per-task tool histogram (which tools have been called by similar tasks) and prompt heuristics (regex/keyword matching for tool-relevant verbs). It removes tools whose joint probability of being called on this task is below a threshold. Tools always available are never pruned.

Output budgeting sets `max_tokens` aggressively because output tokens are typically 5–10× more expensive than input. `max_tokens = clamp(estimated_response_size * 1.5, min=256, max=remaining_budget/output_price)`. If the model truncates, the agent retries with a higher budget.

Thinking-cap negotiation propagates reasoning-effort hints to backends that support them. Conductor and watcher roles default to Low; Implementer to Medium; Architect, Critic, ErrorDiagnoser to High or Max. For providers that ignore the hint, the gateway adds a corresponding system-prompt prefix.

### Convergence check

A subtle failure mode: an agent makes a sequence of calls that looks productive but produces no actual progress (gate verdict hasn't moved in N consecutive turns). Convergence checking examines the trajectory of recent gate verdicts, the trajectory of prediction errors, and the fraction of tool calls that produced new information vs repeated information. If all three stagnate, the gateway returns `NoProgress` and the agent's Conductor watcher escalates.

### Provider call and cascade fallback

The provider call resolves the model hint to a concrete provider, looks up the transport, constructs the provider-specific request, issues with timeout and cancellation, stream-decodes the response, emits per-token Pulses for live UI, and returns the `ProviderResponse`. If the provider returns a transient failure, the gateway retries with exponential backoff. If retries exhaust, the gateway consults the cascade router for a fallback model.

### Cache store and cost tracking

Successful responses (hard-pass Verdict) are stored async (the cache write does not block the response). The gateway emits a `cost.charged` Pulse with input/output token counts, cost, cache_hit flag. This Pulse graduates to a Signal — `cost.charged` is in the always-graduate list. The aggregated cost stream feeds the budget UI, the cascade router, and the audit trail.

### Batch API and circuit breakers

For workloads tolerating batch latency (overnight dream consolidation, bulk research enrichment), the gateway exposes a batch API that submits multiple prompts together to providers that support batching at reduced rates.

The gateway maintains per-provider circuit breakers. After 5 consecutive failures within 60 seconds (defaults), the breaker opens; new calls receive `ProviderUnavailable` and the cascade router skips the provider until the breaker half-opens (a single probe every 30 seconds; success closes; failure resets the open-timer). This protects against amplified failure during provider outages.

### Cost-reduction math

| Lever | Typical reduction | Source |
|---|---|---|
| Prompt cache | 5× | Stage 2 |
| Cascade router (cheaper model selection) | 3× | Stage 7 + cascade router |
| T0 short-circuit (~80% of agent ticks skip the gateway entirely) | 2× | Agent loop |
| Section effectiveness | 1.3–1.5× | Compose protocol |
| Skill / playbook reuse | ~1.4× | Learning |

Stacked, this is **10–30× cost reduction** in production. Every gateway call produces an audit Signal recording per-stage outcomes, prompt and response hashes, tokens, cost, latency, and cache_hit. Audit Signals feed the regulatory audit trail and the section-effectiveness loop.

---

## 3. Connectivity

### The relay wire

The **relay** is Roko's internal cross-process Bus. When agents run in separate processes (multi-instance, sandboxed isolation), the relay carries Pulses between them with low latency.

A `RelayEnvelope` carries version, kind (Pulse, PulseAck, SignalNotification, Heartbeat, JoinRoom, LeaveRoom, BackpressureSignal), origin node ID, correlation ID, and payload. Subscriptions are organized into **rooms** — namespaces that map to topic prefixes. Heartbeats every 5 seconds; missed heartbeat for 30 seconds marks the connection Disconnected. Reconnection is automatic with exponential backoff (1s, 2s, 4s, 8s, capped at 30s); on reconnect the client requests `replay_since(last_seq)` to catch up within the relay's ring window (default 4,096 Pulses per room).

Within a single room, Pulses are delivered in `seq` order. Across rooms, ordering is not guaranteed; for cross-room ordering, the consumer joins both rooms and orders by `emitted_at_ms`.

### The Bus trait across topologies

A Cell publishing a Pulse does not know whether subscribers are in-process, on a relay, or across a chain. The Bus abstraction hides topology entirely.

| Backend | Scope | Latency | Status |
|---|---|---|---|
| `BroadcastBus` (Tokio + ring) | In-process | sub-microsecond | Ships now |
| `MemoryBus` | Testing | sub-microsecond | Ships now |
| `RelayBus` | Cross-process within an org | ~5ms hop | Ships now |
| `NatsBus` / `KafkaBus` | Multi-instance, distributed | ~10–50ms | Phase 2 |
| `ChainBus` | On-chain events | ~12s (block time) | Phase 2+ |
| `MultiBus` | Aggregates multiple backends | n/a | Ships now |

`MultiBus` aggregates multiple backends into a single Bus surface. A Cell can subscribe to topics that may originate from in-process events, a relay, or on-chain events without caring which is the source.

### Backpressure

Backpressure must propagate across the relay or fast producers will overwhelm slow consumers. Four strategies, selectable per topic: Coalesce (buffer; emit latest per interval, used for heartbeats), Drop-oldest (ring buffer; slow consumers miss old events, used for streaming output), Lossless (queue with flow control; producer slows, used for gate verdicts), Sample (every Nth Pulse, used for high-throughput feeds). Lossless mode requires Pulse acknowledgements — the publisher waits for ack before considering the publish complete.

### WebSocket subscriptions

The control plane exposes WebSocket endpoints for real-time UI subscribers (the dashboard, the TUI). Each subscriber receives StateHub projections (typed delta updates) for the topic of interest. WebSocket connections are stateful — disconnection causes the server to drop the subscription; reconnects re-subscribe and replay missed deltas if the disconnect was brief.

### MCP — consume and expose

The `McpConnectCell` connects to MCP servers, lists their tools, and dispatches calls. Three transports: STDIO (subprocess via stdin/stdout, most common for local servers — see safety notes below), HTTP Stream (server-sent events), WebSocket (bidirectional streaming).

Each MCP server's manifest is **hash-pinned** at installation. If the manifest changes between invocations, the agent refuses to use the tool until the new manifest is explicitly approved. The agent also computes an HDC fingerprint of each tool's observed behaviour — a drift alarm triggers when the fingerprint diverges significantly from baseline.

Roko also exposes a built-in MCP server. External LLM clients (Claude Desktop, Cursor, OpenAI Codex CLI) can connect and call Roko's tools as native MCP tools: `query_knowledge`, `submit_task`, `get_episode`, `propose_heuristic`, `list_agents`, `freeze_knowledge`. Roko can be used **as a tool** by other agent systems, not just as a runtime.

### A2A — agent capability cards

A2A defines agent capability cards and discovery. Each Roko agent publishes an A2A card describing identity (ERC-8004 reference), capabilities (canonical bitmask + human-readable list), endpoints (MCP server URL, WebSocket URL, P2P address), pricing (free, x402, MPP session), and service-level commitments. The card is signed by the agent's wallet.

A2A cards are themselves Signals — content-addressed, graduated to Store, looked up by agent ID or capability filter.

### x402 — payments

A service that requires payment returns HTTP `402 Payment Required` with a payment descriptor. The agent constructs a USDC transfer to the specified address with the nonce, signs it, and replays the request with `X-Payment-Receipt`. The service verifies the on-chain transfer (typically via a Base L2 indexer) and serves the response. Two payment patterns: per-request (low-volume, high-value) and session-based (MPP for high-volume, low-value streaming).

### Multi-chain

Roko's chain integration is multi-chain. A `ChainClient` trait exposes `read_state`, `submit_transaction`, `query_knowledge`, `subscribe_events`. Built-in implementations target Ethereum, Base, Arbitrum, and the Daeji testnet. The `MultiChainClient` aggregates them, dispatching reads to the appropriate chain by chain ID.

### Iroh (optional P2P)

For agent-to-agent traffic that should not pass through a central relay (peer-to-peer knowledge sharing, swarm coordination), Roko optionally uses Iroh — a peer-to-peer networking library based on QUIC. Iroh provides NAT traversal, end-to-end encryption, resumable transfers. Opt-in; the default deployment uses the relay.

### The Conductor's 10 watchers

The Conductor (a Reactive-mode meta-orchestrator) watches all other agents. Ten built-in watchers monitor cross-cutting concerns: Health Monitor (process and connection liveness), Stuck Detector (agents not making progress), Circuit Breaker (cascading failures), Anomaly Detector (outlier metrics), Budget Watcher (spend trajectory), Provider Health (LLM error rates), Relay Health (connection quality), Chain Health (RPC freshness and finality), Feed Health (subscribed feed staleness), Memory Health (knowledge store growth and demurrage). Each emits Pulses when thresholds cross; the Conductor responds with remediation Pulses, escalation, or circuit breakers.

---

## 4. Extensions

An **Extension** is a Cell that intercepts another Cell's pipeline at a defined point. Categorically, an Extension is a Functor pattern — a cross-cut that enriches Signals before or after a Cell without changing the Graph's topology. Formally, an endofunctor `F : Signal → Signal`.

Extensions implement orthogonal concerns that apply across many Cells without requiring each Cell to know about them. They have three properties: cross-cuts compose independently; they do not change loop topology; they can be tested independently.

### The 8 layers and 22 hooks

| Layer | Name | Medium | Purpose |
|---|---|---|---|
| L1 | Perception | Pulse | Filter raw input before observation |
| L2 | Memory | Signal | Enrich queries with retrieval; intercept writes |
| L3 | Cognition | Signal | Modify Score, Route, Compose decisions |
| L4 | Action | Signal | Wrap tool calls and LLM dispatches |
| L5 | Verification | Signal | Add custom Verify criteria |
| L6 | Meta | Signal | React to lifecycle events; tune learning |
| L7 | Coordination | Pulse | Inject pheromones and Group signals |
| L8 | Surface | Mixed | Modify outbound responses to clients |

Layers are processed in order. Each layer has named hooks: L1 has `on_observe`, `filter_input`; L2 has `on_retrieve`, `on_store`; L3 has `on_score`, `on_route`, `on_compose`, `on_gate`, `pre_inference`, `post_inference`; L4 has `pre_action`, `post_action`, `on_tool_call`; L5 has `verify_pre`, `verify_post`, `verify_stream`; L6 has `on_reflect`, `on_cost_update`, `on_phase_change`; L7 has `pheromone_emit`, `pheromone_observe`; L8 has `before_response`. Each hook receives a typed payload and may return a modified payload, a veto, or a sentinel indicating no change.

Multiple Extensions can hook the same point. Resolution order is configurable per hook (declaration order is the default). Each can Allow (pass through unchanged), Modify (rewrite the payload), or Reject (short-circuit the chain). The first rejection short-circuits. All decisions are emitted as audit Signals.

Extension declarations include name, version, tier (Tier 1 config, Tier 2 declarative tools, Tier 3 WASM, Tier 4 native Rust), hooks, and required capabilities. Tier 3 and 4 require explicit operator approval. Extensions execute synchronously within the Cell pipeline — hook handlers run on the same task as the Cell they intercept. For async behaviour, Extensions can spawn a Tokio task that publishes a Pulse on completion; the main pipeline continues without waiting.

### CaMeL — capability-tagged information flow

Extensions are subject to **CaMeL** (Debenedetti et al. 2024) information flow control. Every data flow through an Extension is tagged with both its capability provenance and its taint level. When a Cell receives tagged data from an Extension, the Cell's effective capabilities must be a superset of the tag's capabilities. If a Cell has only `Net` and receives data tagged `{FsRead, Secrets}`, the data is rejected.

Extensions cannot strip tags — the runtime computes output tags as the union of input tags and the Extension's own tags. Sensitive data cannot flow to network-accessible Cells without explicit user-approved declassification. Every declassification is logged.

This solves prompt injection structurally: provably solves 67% of the AgentDojo benchmark without any model fine-tuning, by making it structurally impossible for injected prompts to reach tool execution. The query-LLM (Q-LLM) processes user requests with zero direct tool access; tool invocations pass through a capability-controlled pipeline.

---

## 5. Triggers

A **Trigger** is a Cell that conforms to the Trigger protocol. Triggers listen for events and fire Graphs in response. They are the entry point for the system — the way work enters Roko.

### Seven trigger kinds

| Kind | Push mechanism | Example |
|---|---|---|
| **Cron** | Tokio timer | Nightly consolidation at 3 AM |
| **Webhook** | HTTP handler on the control port | GitHub PR opened |
| **FileWatch** | OS filesystem event | Plan file changed |
| **Bus** | Internal event subscription | Gate failure detected |
| **ChainEvent** | Chain indexer WebSocket | On-chain identity updated |
| **Manual** | Explicit API/CLI/TUI invocation | Deploy to staging |
| **SignalPattern** | Store graduation subscription | Cluster of 3+ high-severity findings in 5 minutes |

The system is push-based and event-driven end to end. There is no polling.

### Bindings

Trigger bindings are persistent TOML configurations re-armed on startup. Each specifies a kind and source, a Graph to fire, optional input mapping (JSONPath from event payload to Graph input Signals), a concurrency policy (Queue, Skip, CancelRunning, Parallel), optional event filters, and optional debounce.

Concurrency policy depends on semantics: nightly consolidation uses Skip (no overlapping consolidations); PR review uses Parallel (each PR independent); auto-fix uses Queue (failures should be processed in order). Debounce collapses multiple events within a time window into a single firing — useful for filesystem events that emit multiple times per save.

Each Trigger firing produces an audit Signal (trigger name, kind, event, fired Graph, run ID, fired-at timestamp). Triggered work is fully traceable: from event → trigger → Graph execution → completion → outcome.

---

## 6. Tools

A "tool" in the LLM-agent world (Read, Write, Bash, Grep) is a Roko Connect-protocol Cell exposed to the LLM via tool-call schema. All tools are Cells; not all Cells are tools.

### Built-in tool handlers

The runtime ships 19 built-in tool handlers organized by capability:

| Tool | Capability | What it does |
|---|---|---|
| `Read` | FsRead | Read file contents |
| `Write` | FsWrite | Write file contents (overwrite) |
| `Edit` | FsRead + FsWrite | Replace exact text in file |
| `MultiEdit` | FsRead + FsWrite | Multiple edits in one call |
| `Bash` | Execute(Sandboxed) | Run a shell command |
| `Glob` | FsRead | Find files by pattern |
| `Grep` | FsRead | Search file contents with regex |
| `LS` | FsRead | List directory contents |
| `WebFetch` | Network | Fetch URL content as markdown |
| `WebSearch` | Network | Web search via configured provider |
| `Task` | SpawnAgent | Dispatch a sub-agent for autonomous work |
| `TodoWrite` | (none) | Update structured task list |
| `NotebookEdit` | FsRead + FsWrite | Modify Jupyter notebook cells |
| `Git_Status` / `Git_Diff` / `Git_Log` | FsRead | Git read operations |
| `Git_Add` / `Git_Commit` / `Git_Branch` | FsWrite | Git write operations |

Each handler is a `Cell` implementing the Connect protocol with declared capabilities and cost estimates. Beyond the 19, the runtime ships additional Cells across the nine-protocol surface: Score Cells (SumScorer, RelevanceScorer, NoveltyScorer, plus the recipe library — add, subtract, mean, ema, volatility, correlation), Verify Cells (compile-gate, test-gate, clippy-gate, diff-gate, llm-judge-gate, consensus-gate plus orchestrator gates), Route Cells (EFE, Cascade, RegimeAware), Compose Cells (SystemPromptBuilder, PromptComposer, VCGComposer), React Cells, Observe Lenses, Connect Cells, Trigger Cells.

### The 6-stage safety funnel

Every tool call passes through a 6-stage funnel:

1. **Validate**: arguments checked against the tool's JSON schema.
2. **Resolve**: the canonical `ToolDef` is looked up. Tool names are namespaced (`builtin:Read`, `mcp:filesystem.read_file`).
3. **Authorize**: required permissions compared against role-granted permissions. A Reviewer with `write=false` cannot call `Write`. An Implementer with `network=false` cannot call `WebFetch`. Fail-closed.
4. **Safety checks**: secret scrubbing detects API keys, bearer tokens, private keys, connection strings, and redacts in-place. Provenance logging records a custody chain. Contract enforcement checks the agent's contract YAML for additional restrictions.
5. **Hook chain**: optional sequence of `SafetyHook` implementations. Allow / Modify / Reject. First rejection short-circuits.
6. **Execute**: handler runs with timeout, cancellation token, and result-size cap (default 16KB after UTF-8-aware truncation).

`ToolDispatcher::dispatch_batch` groups calls by concurrency mode: Parallel (read-only operations) run concurrently; Serial (shell commands, file writes) run sequentially to avoid races.

Secret scrubbing is applied **before** tool execution (preventing secrets from being passed to tools) and **after** (preventing tool outputs from leaking secrets into the next prompt). Replacements are deterministic: the same secret is always replaced with the same opaque token, allowing the agent to reason about presence without seeing values.

Every tool invocation is recorded in a custody log: tool name, post-scrub arguments, result summary, timestamp, agent role, trace ID, and the 6 stages' outcomes. This provides a complete audit trail of what every agent did, when, and why. Custody Signals are append-only and graduated to Store.

### Agent contracts

`AgentContract` defines per-role constraints in YAML. Contracts can restrict which tools a role may use, set cost ceilings, require specific output formats, and enforce domain-specific invariants:

```yaml
role: Auditor
allowed_tools: [Read, Glob, Grep, LS, Git_Diff, Git_Log]
denied_tools: [Write, Edit, Bash, Git_Commit]
cost_ceiling_per_turn: 0.50
required_output_format: markdown_review
invariants:
  - never_modify_production_data
  - all_findings_must_cite_file_and_line
```

When a contract YAML is missing, the system falls back to a permissive default.

### MCP integration and supply chain

MCP servers are integrated via the `McpConnector` Cell. Each server's manifest is cryptographically pinned at installation. The agent computes an HDC fingerprint of each tool's observed behaviour (input-output patterns) — a drift alarm triggers when the fingerprint diverges from baseline.

The MCP ecosystem has had multiple disclosed vulnerabilities, illustrating the systemic nature of the problem:

- **CVE-2025-6514** (`mcp-remote`, CVSS 9.6): RCE via crafted tool descriptions, 437,000 downloads at disclosure.
- **CVE-2025-54136** (MCPoison in Cursor): tool-poisoning via injected instructions in tool descriptions.
- **Postmark-mcp backdoor** (September 2025): backdoored MCP server distributed to ~300 organizations before detection.
- **OX Security STDIO flaw** (April 2026): fundamental architectural vulnerability in STDIO transport, affecting 150M+ downloads across 7,000+ servers.
- **CVE-2025-68143/68144/68145** (Anthropic `mcp-server-git`): three chained vulnerabilities allowing RCE via crafted Git repository content.

Roko's hardening: every tool description is treated as `ExternalFetch`-tainted data flow under CaMeL; the Q-LLM has zero direct tool access; hash-pinning, behavioural drift alarming; HTTP-stream / WebSocket transports for production deployments rather than STDIO.

### The Q-LLM / T-LLM split

A critical structural defense (CaMeL framework): the agent is split into two LLM contexts. The Q-LLM (query) processes user requests, generates planning, formulates intent. The T-LLM (tool) receives validated tool invocations and produces tool-specific output. The Q-LLM has **zero direct tool access**. All tool invocations pass through a capability-controlled pipeline. This makes prompt injection structurally unable to reach tool execution: an injected prompt seen by the Q-LLM cannot directly cause tool calls; it can only generate a request that goes through the safety funnel, where the injection is caught by capability checks, taint propagation, and contract enforcement.

### Tool output verification

Six built-in Verify Cells gate tool outputs: compile-gate, test-gate, clippy-gate, diff-gate (validates against constraints — max lines, no secrets, no binaries, restricted paths), llm-judge-gate, consensus-gate. Every Verify Cell implements both `verify_pre()` and `verify_post()`.

Every tool call produces an output Signal that graduates to Store, becoming part of the lineage DAG. Future episodes can trace which tool calls contributed to which outcomes.

---

## 7. Feeds and Recipes

A **Feed** is a Cell that combines three protocols: Connect (external data source I/O), Trigger (event-driven activation), and Store (optional persistence). Feeds are the "always-on" complement to one-shot queries — continuous data streams that other Cells consume.

Treating Feeds as a Cell specialization means they inherit every property of Cells: typed I/O schemas, capability declarations, cost estimates, predict-publish-correct calibration, composable into Graphs, auditable via the same Bus topics. There is no "Feed system" — only Cells that conform to the Connect + Trigger + Store protocol triple.

### Four feed types

| Type | Source | Compute | Example |
|---|---|---|---|
| **Raw** | Direct external ingestion | None or normalization | Aave borrow-rate WebSocket; GitHub PR webhook |
| **Derived** | One or more raw feeds | Agent-computed | "ETH borrow rate volatility (1h EMA)" |
| **Composite** | Multiple derived feeds, often cross-domain | Aggregation logic | "Cross-protocol funding-rate divergence flag" |
| **Meta** | Other feeds | Health monitoring, accuracy tracking | "Borrow-rate feed staleness alert" |

The chain is intentionally compositional: a Derived feed consumes Raw feeds, a Composite consumes Derived, and so on. Each link pays for its inputs and charges for its output.

### Feed lifecycle and registry

A Feed connects to its source via Connect, subscribes to upstream events via Trigger (or polls on a Cron schedule), transforms incoming events into typed Signals, optionally persists via Store, and always publishes a Pulse on `feed:{name}.event`.

The FeedRegistry maintains a catalog of available Feeds, their schemas, prices, and health metrics. Feeds can be **dynamically registered** at runtime: an agent can publish a new Feed that didn't exist before, and other agents discover it through the registry.

Feeds intended for cross-organization consumption can advertise themselves on-chain. The on-chain advertisement is a Signal containing schema, price, payment endpoint, and current operator's ERC-8004 identity. Agents from other organizations discover the Feed via on-chain query and subscribe via x402 payments.

### Feed economics

Each agent in the chain pays for its inputs and charges for its output. Pricing models: Public (free, often a loss-leader to attract attention), Per-request (x402 — each subscriber pays a micro-payment per event consumed), Session-based (MPP — subscribers establish a session with a streaming budget; the Feed bills against the session as events are delivered).

A high-volume Feed can overwhelm a slow subscriber. Each Feed declares a backpressure policy: Coalesce, Drop-oldest, Lossless, Sample. Subscribers can override per-subscription if they have credentials.

### Recipes — pure data Graphs

A **Recipe** is a Graph of Score Cells with no LLM calls and no agent involvement. Recipes are composable data transformations distinct from Plans (task DAGs) and Compose (prompt assembly).

```toml
[recipe]
name = "rate-divergence-detector"
version = "1.0.0"

[[recipe.nodes]]
id = "fetch-aave"
cell = "roko.feeds.aave_borrow_rate"

[[recipe.nodes]]
id = "fetch-compound"
cell = "roko.feeds.compound_borrow_rate"

[[recipe.nodes]]
id = "compute-divergence"
cell = "roko.recipes.subtract_and_abs"

[[recipe.nodes]]
id = "threshold-flag"
cell = "roko.recipes.threshold_flag"
[recipe.nodes.params]
threshold_bps = 150
```

Recipes are deterministic — the same input always produces the same output. They are useful for data normalization, cross-protocol comparison, threshold flagging, aggregation. Because Recipes have no LLM cost, they can run continuously at gamma cadence with negligible compute.

| Concept | What it does | LLM involvement | Output |
|---|---|---|---|
| **Plan** | Task DAG to execute | Yes (per task) | Code, documents, decisions |
| **Compose** | Assemble a prompt from sections | No (assembly only) | A composed prompt Signal |
| **Recipe** | Data transformation pipeline | No | A computed value Signal |

The built-in stateless Score Cells include arithmetic (add, subtract, multiply, divide, abs, negate, clamp), statistical aggregation (mean, median, percentile, stddev), moving averages (ema, sma, wma), volatility, correlation (Pearson r over a rolling window), threshold_flag, linear_combine (weighted sum), winsorize (trim outliers to percentile bounds), and volume_weighted_median (TVL-weighted median).

A canonical Recipe use case: the runtime side of an Internet Secured Funding Rate (ISFR) component computation. The Recipe consumes Raw feeds for borrow rates from each constituent protocol, applies winsorization, computes a TVL-weighted median per validator, computes a median across validators, and emits the final rate as a typed Signal. The Recipe is deterministic and reproducible — any third party can run the same Recipe against the same inputs and verify the result. The on-chain ISFR oracle and chain HDC precompile are owned by the chain.

### Cross-organization feeds and meta-feeds

The most valuable Feeds cross organizational boundaries. A Feed produced by Operator A can be subscribed to by an agent run by Operator B. Three properties make this work: discovery via on-chain registry (no central directory), trustless payment via x402 (no invoice negotiation), verifiable provenance via ERC-8004 (Operator B can verify Feed identity and reputation).

Feed Health is itself a Feed. A meta-Feed `feed:{name}.health` emits Signals describing the parent Feed's state. Subscribers detect degradation: if events-per-minute drops below a threshold, switch to a fallback Feed or escalate.

For Feeds carrying critical data, the recommended pattern is to subscribe to multiple redundant Feeds from different operators and apply a Recipe that computes a median or quorum. This is **intersubjective validation** (Yuan et al., arXiv:2504.13443) at the data-pipeline layer.

---

## 8. Groups and Coordination

A **Group** is a Space — a Graph that owns a Bus partition and a Store partition, with members sharing these resources under access control — augmented with **membership** and a **coordination mode**. Every property of Spaces (isolation, capability scoping, shared resources) extends naturally to Groups.

A Group is created either explicitly (operator request) or emergently (when stigmergic activity around a topic exceeds a threshold and the conductor proposes formation).

### Four coordination modes

Each Group declares one of four coordination modes:

- **Stigmergic** (default for ad-hoc collectives): members coordinate exclusively through the shared Store and pheromone field. No direct messages. Best for cross-organization research, distributed monitoring, knowledge accumulation. Scales O(1) per agent — independent of group size.
- **Pipeline** (sequential refinement): members work in sequence, each refining the previous member's output, with the option to fork into parallel sub-teams. The "Sequential Hybrid" protocol. Best for editorial workflows, multi-stage validation, code review chains. Per Dochkina (arXiv:2603.28990, March 2026), Sequential Hybrid beats the Coordinator protocol by +14% and Shared Autonomy by +44% (Cohen's d=1.86).
- **Broadcast** (every member sees everything): the classical "all-to-all" topology. Best for small expert panels, voting, consensus formation. Cost scales as O(N²); degrades quickly past ~20 members.
- **Leader-Follower** (coordinator dispatches): a designated coordinator receives all task inputs, decomposes them, dispatches subtasks, aggregates results. Best for hierarchical organizations with strong sequential dependencies.

### Hard cap at 64 agents

Empirically, scaling beyond 64 agents yields no statistically significant quality improvement. Dochkina (arXiv:2603.28990) studied 8 LLM models, agent counts 4–256, 8 coordination protocols, 4 difficulty levels, and over 25,000 tasks. The Kruskal-Wallis H-test (H=1.84, p=0.61) cannot reject the null that 64 and 256 agents produce identical quality.

The runtime enforces this as a default `max_members: 64`. For problems requiring more agents, **shard** into independent sub-Groups. Inter-Group communication is through the parent Space's pheromone field, not direct member-to-member messaging.

Three additional findings from Dochkina:

1. **Protocol matters more than headcount.** Sequential Hybrid beats Coordinator by 14% and Shared Autonomy by 44%.
2. **Agents self-regulate at scale.** At N=256, 45% of agents voluntarily abstain — they evaluate the work and conclude they have nothing useful to add.
3. **The hard cap is a default, not a hard limit.** Groups can override with explicit operator approval.

### Communication density threshold

Three independent research groups converge on a sparse-communication optimum:

- **Li et al.** (Google, EMNLP 2024, arXiv:2406.11776): ring topology at density ~0.22 matches fully-connected debate while saving 40–60% of tokens.
- **MacNet** (Qian et al., ICLR 2025, arXiv:2406.07155): small-world networks (Watts-Strogatz, rewiring probability 0.1–0.3) win, with logistic saturation at N≈32.
- **Kim et al.** (DeepMind, arXiv:2512.08296, December 2025): optimal communication density **c\* = 0.39** messages per agent per turn. Independent multi-agent systems amplify errors **17.2× vs a single agent**; centralized systems amplify 4.4×. Sparse communication with local validation minimizes amplification.

Roko's default coordination topology is **small-world**: a ring of members with a small fraction of random shortcut links. Communication density target is 0.2–0.4. Fully-connected broadcast is opt-in.

### Diversity as engineered property

Yang et al. (ICML 2026, arXiv:2602.03794) prove information-theoretically that **2 diverse agents match or exceed 16 homogeneous agents** — an 8× compute reduction. The complementarity rate determines whether each additional agent contributes new information or merely echoes existing ones.

The **K\* metric** (entropy effective rank of the Gram matrix of agent output embeddings) measures actual diversity at runtime. K\* is **label-free** — no ground-truth answers needed — making it directly usable as a runtime quality signal. Roko computes K\* per Group and: logs it as telemetry, uses it as a gating condition for L4 group expansion, triggers restructuring when K\* drops over time.

Engineered diversity comes from different model families, different prompting strategies (specialist vs generalist; deferent vs assertive), different tool access, different role assignments.

### Pheromone field

The pheromone field is the stigmergic medium for the Group. It is a Bus topic partition (`pheromone:{group_id}.{location_hash}`) where members deposit short-lived intensity Pulses about specific locations (file paths, code regions, knowledge entries, market positions). Members both **deposit** (intensity += contribution) and **read** (intensity at location).

Common uses: code coordination (members deposit at files they're editing so others avoid concurrent modification), research focus (members deposit at topics they're investigating so others can join or fork), risk awareness (members deposit at code paths that recently caused failures), market signals (trading agents deposit at strategies that recently produced gain or loss).

### Membership lifecycle

`Invited → Joined → Active → { Suspended | Departed }`. Membership transitions emit `group.{id}.member.{state}` Pulses on Bus. Groups can span organizational boundaries — an operator at organization A can invite an agent run by organization B. Invitations carry the inviting agent's signed credential; the joining agent verifies via ERC-8004 identity check.

### Shared knowledge

The Group's `shared_knowledge` is a Memory Cell scoped to the Group. Knowledge written to it is visible to all current and future members, decays via demurrage, can be promoted to organization-wide or chain-wide visibility through operator approval, and can be exported as a knowledge bundle for sale in the marketplace. When a member leaves, their contributions remain with their authorship intact — leaving doesn't erase legacy.

---

## 9. The 14 Failure Modes (MAST)

Multi-agent systems fail in well-catalogued ways. Cemri et al. (NeurIPS 2025, arXiv:2503.13657) cataloged 14 distinct failure modes through annotation with inter-annotator agreement κ=0.88.

| Failure mode | Detection mechanism |
|---|---|
| Mutual contradiction | Sheaf cohomology obstruction (H¹ ≠ 0) |
| Redundant work | PID redundancy spike |
| Withheld critical information | Information-flow gaps in lineage |
| Misinterpreted coordinator instructions | Reply-pattern mismatch |
| Incompatible output formats | Schema validation failure |
| Infinite delegation loops | Graph cycle detection |
| Over-specialization | K* drops; coverage gaps |
| Failure to aggregate partial results | Coverage analysis on coordinator output |
| Premature termination | Deadline check + completion-rate trend |
| Conflict failure to resolve | Open-issue counter; oscillating votes |
| Resource exhaustion before completion | Budget tracker tripped |
| Knowledge poisoning | AntiKnowledge similarity hits |
| Coordinator overload | Dispatch latency increase |
| Sycophancy spiral | Pleasure overshooting; dominance under-shooting |

Each detected failure triggers proportional consequences — quality deductions, reputation penalties, or economic slashing.

---

## 10. PID Diagnostics — Synergy as Order Parameter

Partial Information Decomposition (PID; Williams-Beer-Lizier-Mediano lattice) decomposes the information that a group of variables provides about a target into three components: redundancy (information multiple agents provide identically — high = waste), unique information (information only one agent provides — high = specialization), synergy (information only available from the combination of agents — high = genuine coordination).

Riedl (Northeastern, arXiv:2510.05174, October 2025) applies PID to multi-agent systems by computing time-delayed mutual information over agent state trajectories. Key finding: **synergy serves as an order parameter for coordination phase transitions.** When synergy crosses a threshold, the system transitions from a collection of independent agents to a genuinely coordinated collective.

Roko computes PID metrics over rolling windows of agent activity inside Groups. When synergy drops below a threshold and redundancy rises, the Group is degenerating into redundant parallel work. The conductor recommends reducing agent count, increasing diversity, or restructuring the topology.

### Sheaf consensus

Cellular sheaf theory provides a complementary framework. A cellular sheaf assigns a data space to each agent and a "restriction map" to each communication link. Agreement is not "identical outputs" but "outputs consistent when translated through restriction maps."

Hanks and Riess et al. (arXiv:2504.02049 + arXiv:2510.00270, 2025) prove ADMM convergence for cellular-sheaf-based distributed optimization with bounded asynchronous delays and Lyapunov-stable tracking. The diagnostic: the **first cohomology group H¹** of the sheaf. When H¹ = 0, all local consistencies compose globally. When H¹ ≠ 0, agents appear pairwise consistent but the Group as a whole is contradictory — the signature of subtle coordination failures invisible to pairwise checking.

An open question: does H¹(sheaf) = 0 correspond to PID synergy > 0? If so, sheaf cohomology and PID synergy measure the same phenomenon from different angles.

### Group-level Verify

Each Group has a Verify Cell scoped to its outputs. Group-level Verify validates quorum consistency across member outputs, sheaf cohomology (H¹ = 0), PID synergy above threshold, no catastrophic redundancy, and coverage of declared sub-tasks. It produces a Group-Verdict that determines whether the output is accepted or returned for revision.

### Cost allocation

Costs from Group activity are allocated based on coordination mode: Stigmergic (each member pays for their own contributions), Pipeline (each member pays for their stage; final consumer pays for aggregation), Broadcast (pro-rata share by member), Leader-Follower (coordinator pays for dispatch; member pays for their own execution).

---

## 11. Emergent Communication

When agents interact repeatedly, they can develop compressed protocols more efficient than natural language.

### Agora — self-organizing protocol negotiation

Marro et al. (Oxford, arXiv:2410.11905, October 2024) built Agora: 100 LLM agents negotiate communication protocols. Instead of pre-defined message formats, agents propose, test, and refine JSON-schema-based routines for specific interaction types. Result: **5× cost reduction** versus unconstrained natural language. Emergent protocols strip away politeness markers, redundant context-setting, verbose explanations, retaining only information-theoretically necessary content.

A natural extension: replace SHA1 hashing of protocols with HDC fingerprinting. Similar protocols produce similar fingerprints, enabling near-match discovery across version drift. SHA1 cannot do this — a single-character change produces an entirely different hash.

### Modular composite representations

Angioli, Kymn, Loutfi, Kleyko (arXiv:2511.09708, November 2025) provide the computational substrate. Modular Composite Representations achieve **3.08× faster execution and 2.68× lower energy** versus Binary Spatter Codes (the HDC binding/bundling baseline). Combined with Agora-style protocol negotiation, agents can both invent compressed protocols and execute them at near-zero marginal cost.

### The population-scale trap

Not everything about emergent communication is positive. Chaabouni et al. (ICLR 2022) demonstrated that **scaling population size alone does NOT induce compositional communication protocols**. Compositionality requires specific conditions: heterogeneity (agents must differ so communication is genuinely necessary), learning-speed asymmetry (when some learn faster, slower agents pressure faster ones to communicate clearly), length cost (penalizing message length is necessary to drive compression).

Lossy Iterated Learning (arXiv:2511.18220, November 2025) extends Fano's inequality to show even a fraction of a bit of channel capacity difference can flip a population from accumulating compositional knowledge to plateauing. Channel capacity is a phase-transition parameter, not a continuous dial.

Practical implication: building a large fleet of identical agents and hoping they develop efficient communication is a recipe for wasted compute. Engineer heterogeneity (different models, tools, roles) and bottlenecks (limited bandwidth, sparse topology) deliberately.

---

## 12. Stigmergy as Immune Tissue

When agents communicate through a shared persistent environment, that environment accumulates traces of all agent activity. The environment becomes an **immune system**, detecting pathological agent behaviour the way biological immune systems detect pathogens.

Three categories of signals (after Matzinger 1994):

- **PAMPs** (known-bad signatures): traces matching previously catalogued attack patterns — prompt injections with known structure, outputs matching known hallucination patterns, activity matching known Sybil coordination.
- **Safe signals**: traces from healthy activity — normal outputs, valid provenance, expected parameter ranges.
- **Danger signals**: traces that are not known-bad but indicate tissue damage — anomalous patterns, unexpected mutations, provenance gaps, output distributions diverging from baseline.

Detection mechanisms:

- **Dendritic-cell-style temporal correlation**: track the ratio of danger to safe signals over a sliding window. A single anomalous trace is not actionable. Sustained elevation triggers investigation.
- **Auto-quarantine of necrotic regions**: isolate sections where danger signals dominate, walling them off from agent writes pending review while preserving read access for context.
- **Bond-slash on emission**: agents posting traces have a stake; pathological traces forfeit a portion. Direct incentive for self-filtering.

Why stigmergic substrates uniquely enable this: in direct-messaging architectures, messages exist only between sender and receiver with no shared inspection surface. In stigmergic architectures, every agent action leaves a visible trace. The environment is simultaneously communication medium and diagnostic surface.

The threat is real. **Prompt Infection** (ICLR 2025, arXiv:2410.07283) demonstrates self-propagating viral injection across multi-agent systems — a single poisoned agent contaminates others through shared context, and contamination spreads geometrically. Multi-agent prompt injection is OWASP's #1 risk for LLM applications (2025 edition).

### Sheaf-consistent reputation for deception detection

Multi-agent systems with economic incentives face manipulation. **MAD-Spear** (arXiv:2507.13038, July 2025) demonstrates Sybil-coordinated manipulation where colluding agents use conformity pressure to herd honest agents. **AdapAM** (arXiv:2511.15292, November 2025) shows strict black-box adaptive attacks against multi-agent reputation systems are now feasible.

Detection requires comparing what agents say with what they believe. Four mechanisms form defense in depth:

1. **Thought Communication latent extraction** (arXiv:2510.20733): infer an agent's internal reasoning state from its outputs. Divergence between stated conclusions and latent processing flags potential deception.
2. **Cellular sheaf consensus**: H¹ ≠ 0 reveals coordinated deception (pairwise consistency, global contradiction).
3. **PID synergy/redundancy**: under manipulation, redundancy spikes (colluding agents echo each other) while synergy drops. Monitor the synergy/redundancy ratio.
4. **On-chain reputation**: scores on an append-only ledger reduce proportionally when sheaf cohomology detects obstruction. Agents cannot erase deception flags.

---

## 13. Eight Design Principles for Roko Groups

The research above translates to concrete principles enforced by the Group runtime:

1. **Hard-cap clusters at 64 agents.** Above that, shard.
2. **Use ring or small-world topology, not fully connected.** Target communication density 0.2–0.4.
3. **Prioritize diversity over scale.** Monitor K\*; engineer heterogeneity across model families, tool access, prompting strategies, roles.
4. **Engineer communication bottlenecks to drive protocol emergence.** Limit bandwidth; introduce heterogeneity; let agents negotiate.
5. **Implement active decay on shared state.** Govcraft Theorem 3: decay is mathematically necessary.
6. **Monitor synergy and redundancy as health metrics.** Compute PID over rolling windows. Drop in synergy + rise in redundancy → restructure.
7. **Map failure modes to economic consequences.** Use the MAST 14-failure catalog as a checklist with graduated responses.
8. **Build deception detection into the coordination layer.** Latent extraction + sheaf cohomology + PID + economic consequences. Defense in depth.

---

## 14. Surfaces

Roko exposes itself through five primary surfaces:

| Surface | Audience |
|---|---|
| **CLI** (`roko`) | Operators, developers, scripts |
| **HTTP API** | Programmatic integrations, dashboards (~85 REST routes plus SSE and WebSocket on port 6677) |
| **TUI** | Operators on terminal-only environments (built on `ratatui`) |
| **Web Dashboard** | Operators, reviewers, observers (browser application via WebSocket + REST) |
| **MCP Server** | External LLM clients (Claude Desktop, Cursor) |

All surfaces consume StateHub projections and submit through the same control plane. The kernel knows nothing about UI; the surfaces are pure consumers.

### CLI subcommands

```
roko init                            # initialize a workspace
roko serve                           # start the daemon
roko dashboard                       # open the TUI
roko status / doctor                 # health and diagnostics

roko prd idea / draft / plan / list  # capture and plan work
roko plan run [--resume]             # execute or resume a plan
roko plan list / show

roko agent list / dispatch / stop
roko knowledge query / freeze / challenge
roko learn cascade-show / experiments / export-episodes
roko research enhance-prd

roko config show / edit / validate / migrate / reload
roko config secrets set <name>

roko auth issue-key / invite / login
```

### HTTP API

Routes are grouped by resource: Plans, Tasks, Agents, PRDs, Knowledge, Learning, Configuration, Health. Streaming endpoints include SSE streams for agent and plan events and WebSocket endpoints for StateHub projections, per-agent connections, and Feed subscriptions.

### The TUI

Launched via `roko dashboard`. Tabs (F1 Overview, F2 Agents, F3 Plans, F4 Knowledge, F5 Learning, F6 Costs, F7 Logs) each subscribe to the appropriate StateHub projection and re-render on delta updates. The file watcher detects changes to plan files, briefs, and config and reflects them in the relevant tab.

### The web dashboard

Pages mirror the TUI tabs but with richer visualizations: time-series charts, DAG visualizations, knowledge graph browsers, learning curve plots. The dashboard subscribes to projections via WebSocket. The data contract is identical to the TUI's, so a single change to a projection is reflected in both surfaces simultaneously.

### Built-in MCP server

Roko exposes a built-in MCP server. Exposed tools: `query_knowledge`, `submit_task`, `get_episode`, `propose_heuristic`, `list_agents`, `freeze_knowledge`. Roko can be used **as a tool** by other agent systems.

### Visual composition with 12 primitives

The web dashboard hosts a **Generative Canvas** — a visual editor for Graphs of Cells. It uses the Digital Audio Workstation pattern: nodes are plugins, connections are patch cables, parameters are knobs. Twelve primitive node types map onto the protocol surface: Source (Trigger), Sink (Store), Transform (Score), Filter (Score with predicate), Splitter (FanOut), Merger (FanIn), Loop (Graph with feedback edge), Verify, Route, Compose, Lens (Observe), Subgraph (embedded Graph for fractal composition).

The canvas only writes Tier 1–3 Cells (config, declarative tools, WASM). It cannot write native Rust, so visually-built Graphs are safe by construction. Type compatibility, capability intersection, and protocol adjacency are enforced at edit time — the editor refuses to wire incompatible nodes.

### The Workbench, Inbox, and Autonomy Slider

The **Workbench** is a structured task delegation surface (modelled on Linear/Notion patterns). It lets a human operator define tasks with acceptance criteria, decompose tasks into subtasks, assign tasks to specific agent roles, review agent outputs with diff-aware comparison, and approve, reject, or revise. The Workbench is itself a Graph: a Trigger fires on task creation; a Plan dispatches the agent; a Verify Cell runs the gate pipeline; a custom React Cell waits for human approval before persisting.

The **Agent Inbox** aggregates agent-emitted notifications: approval requests, escalations from the Conductor, new insights from dream consolidation, knowledge challenges, marketplace events. The inbox supports filtering, batching, and forwarding to external channels.

The **Autonomy Slider** is a per-capability progressive trust control:

```
Level 1: Suggest only        — agent describes what it would do; human acts
Level 2: Propose with diff   — agent prepares the change; human approves
Level 3: Apply with review   — agent applies; human reviews after
Level 4: Apply with audit    — agent applies; only audit log records the action
Level 5: Full autonomy       — agent acts without notification
```

The slider is per-capability, not per-agent. An agent can have full autonomy for `Read`, propose-with-diff for `Write`, suggest-only for `Bash`. This lets operators scale trust gradually as agents prove reliable.

### Streaming output

Every agent's LLM output streams in real time to the TUI and web dashboard. The agent emits per-token Pulses on `agent:{id}.output`; the dashboard subscriber renders them as they arrive. Long-running agent work feels responsive: the operator can see exactly what the agent is generating moment by moment, intervene if going off-track, or cancel mid-generation.

### Per-agent HTTP sidecar and ACP

Each agent has an HTTP sidecar on a dedicated port. The sidecar exposes routes for agent info, state, message dispatch, WebSocket streaming, predictions, research artifacts, and current tasks. This lets external systems interact with individual agents over HTTP, useful for integrations that don't fit the dashboard or CLI patterns.

Roko's Cursor backend integrates via Cursor's Agent Client Protocol (ACP) over JSON-RPC. ACP defines the interaction contract between IDEs and agent backends.

The next document, [Trust and Economy](./04-trust-and-economy.md), describes security, authentication, payments, the marketplace, arenas, and how DeFi adjacency works at the runtime side.
