# Synthesis 06: Security, Observability, and Deployment

## How Roko Secures Autonomous Agents, Observes Their Behavior, and Deploys Across Environments

---

## 1. Security Model -- Agent Sandboxing, Tool Authorization, and Contract Enforcement

Roko's security model is built on one principle: **the system fails closed**. No agent component (called a "Cell") executes unless every layer of the security stack explicitly permits it. There is no override mechanism. Five mechanisms enforce this, from innermost to outermost.

### 1.1 Three-Layer Capability Intersection

Every Cell declares what system resources it requires -- file reads, file writes, network access, shell commands, LLM calls, blockchain interactions, secret access, knowledge store reads/writes, process management, or custom extension-defined capabilities. Eleven capability types cover the full resource surface.

The effective capability of any Cell is the **strict intersection** of three independent layers:

1. **Cell Declaration.** The Cell's TOML manifest lists required capabilities with constraints (e.g., `FsRead` restricted to `src/**` and `docs/**`). This is a ceiling -- the Cell can never exceed what it declares, regardless of what other layers permit.

2. **Graph Allow-List.** The Graph (a composition of Cells wired into a DAG) may further restrict its constituent Cells. A general-purpose Cell that declares `Shell` capability becomes safe for a read-only analysis Graph simply by the Graph not including `Shell` in its allow-list.

3. **Space Grant.** The Space (workspace) is the user's authority. The user grants capabilities in `workspace.toml`. A capability not granted by the Space is denied regardless of what Cells and Graphs declare.

The intersection is computed at Graph-load time. At runtime, every resource access is checked against the effective capability set. Violations emit a `CapabilityDenied` error Signal and are logged to the audit trail. The narrowest constraint at any layer wins. For example, if a Cell declares `Net { domains: ["api.openai.com"] }`, the Graph allows `Net { domains: ["api.openai.com", "api.anthropic.com"] }`, and the Space grants `Net { domains: ["*"] }`, the effective capability is `Net { domains: ["api.openai.com"] }` -- the Cell's own declaration is the tightest.

### 1.2 Taint Lattice Information Flow Control

Every piece of data flowing through the system carries a taint level, ordered in a monotonic lattice: `Clean < UserInput < LlmGenerated < ExternalFetch < Propagated`. Taint can only increase through derivation, never decrease. A Signal tainted at ingestion stays traceably tainted through all its descendants.

This prevents a critical attack: an adversary cannot launder a poisoned Signal by deriving a clean-looking descendant from it. The only way to "clean" tainted data is through human review recorded in the custody chain.

CaMeL IFC (Capability-tagged information flow control, after Fang et al. 2024) extends this to Extensions: every data flow through an Extension is tagged with both its capability provenance and its taint level. When a Cell receives tagged data from an Extension, the Cell's effective capabilities must be a superset of the tag's capabilities. If a Cell has only `Net` capability and receives data tagged `{FsRead, Secrets}`, the data is rejected. Extensions cannot strip tags -- the runtime computes output tags as the union of input tags and the Extension's own capability tags.

Sensitive data (from Secrets, sensitive file paths) cannot flow to network-accessible Cells without explicit user-approved declassification. Every declassification event is logged as a `SecurityEvent::Declassification` Signal with full provenance.

### 1.3 Five-Head Lexicographic Corrigibility

Every agent decision passes through a 5-head lexicographic ordering (after Nayebi 2024). The heads are evaluated in strict priority order, and a higher-priority head always trumps a lower-priority head regardless of magnitude:

| Priority | Head | Meaning |
|----------|------|---------|
| 1 (highest) | **Deference** | Obey the human's stated preferences and constraints |
| 2 | **Switch** | Preserve the human's ability to change the agent's behavior |
| 3 | **Truth** | Represent information accurately; do not deceive |
| 4 | **Impact** | Minimize unintended side effects; prefer reversibility |
| 5 (lowest) | **Task** | Accomplish the assigned task effectively |

Each head is a separate Verify Cell. They run in sequence during the pre-action verification phase. The chain short-circuits on first rejection: if the Deference head rejects an action, the Switch, Truth, Impact, and Task heads are never consulted. This is deliberately lexicographic rather than a weighted sum -- weighted-sum safety is Goodhart-vulnerable (an agent could find a task action worth 100 points with a safety cost of 9.5 points and take it). Lexicographic ordering eliminates this failure mode entirely.

### 1.4 Verify Outside the Modifiable Surface

The agent can choose which Cells to run, which models to use, how to allocate budget, and which strategies to apply. The Verify pipeline is architecturally **outside** this modifiable surface. The agent cannot add, remove, or reorder Verify heads. It cannot modify Verify Cell implementations. It cannot bypass pre-action verification -- the execution engine calls it, not the agent. Structural changes to the verification pipeline require explicit human approval.

### 1.5 Cognitive Immune System

A 5-layer pipeline Graph processes every Signal crossing a trust boundary:

1. **Taint Propagation** -- tracks untrusted lineage through Signals and checks against a recognition library of known attack patterns (HDC fingerprint matching).
2. **Anomaly Detection** -- detects contradiction clusters, score spikes without supporting evidence, taint fan-out bursts, sandbox violation clusters, tenant boundary mismatches, and lineage gaps. Uses z-score thresholds.
3. **Quarantine Gate** -- isolates suspect Signals from default retrieval pending investigation.
4. **Incident Response** -- links findings to custody records, enables replay, and generates postmortems.
5. **Immune Memory** -- stores attack patterns and defensive responses, feeding back into Layer 1's recognition library for future detection.

Threat classes include prompt injection, memory poisoning, taint cascade, adversarial retrieval, sandbox violation, cross-tenant leakage, and lineage mismatch. Containment actions range from monitoring to quarantine, re-verification, escalation, or plugin disablement.

---

## 2. Authentication and Authorization

Authentication is expressed as a **pipeline of Verify Cells** with two stages.

**Stage 1 -- Authentication** uses four Verify Cells, each handling a different credential type. The pipeline short-circuits on first acceptance:

| Path | Surface | Credential |
|------|---------|------------|
| Privy/JWT | Web dashboard | JWT signed by Privy JWKS |
| API Key | CLI, external integrations | `sk_roko_...` header with 4 scopes (Read, AgentWrite, PlanWrite, Admin) |
| Agent Bearer Token | Agents (relay, sidecar, inference) | `roko_agent_...` bearer |
| Relay Read | Feed subscribers | No credential (read-public, read operations only) |

If all four Verify Cells skip (no matching credential), the pipeline returns `401 Unauthorized`.

**Stage 2 -- Authorization** checks workspace membership. The `AuthorizeCell` receives the authenticated identity, checks it against `.roko/users/`, resolves the user's role (`Owner`, `Admin`, `Member`, or `Viewer`), and verifies the role has grants for the requested route. Route grants are fine-grained: methods (GET/POST/PUT/DELETE or wildcard) and path patterns (e.g., `/api/agents/*`).

JWT validation uses Privy's public JWKS endpoint with a caching strategy: 1-hour TTL with stale-while-revalidate on endpoint unavailability. Key rotation is handled by re-fetching JWKS on signature mismatch (single retry). No Privy app secret is needed on user deployments -- only the public app ID.

For headless machines, a device flow is available: `roko login` displays a code, the user approves in a browser, and the token is stored in the OS keychain.

---

## 3. Telemetry and Observability

Roko's telemetry system has two layers: **Lenses** (raw observation machinery) and **StateHub** (typed projections consumed by display surfaces).

### 3.1 The Lens System

A Lens is a Cell implementing the Observe protocol. It receives read-only lifecycle events and emits structured observation Signals. Four principles govern the design:

- Observation is passive. Removing all Lenses changes nothing about system behavior -- only visibility.
- Observation is compositional. Lenses stack (multiple Lenses on one target), chain (a Lens watches another Lens's output), and scope (Cell, Graph, Agent, Space, or Global granularity).
- Observation uses the same primitives as everything else. Lens output is a Signal. Lens composition is a Graph. Lens configuration is TOML.
- Projections are the data contracts between telemetry and display surfaces.

The system ships 11 built-in Lenses:

| Lens | What It Measures |
|------|-----------------|
| **CostLens** | USD and token expenditure with model breakdown and budget remaining |
| **LatencyLens** | Execution duration with p50/p95/p99 percentile tracking |
| **QualityLens** | Pass/fail rates from Verify Cells with continuous reward tracking and per-rung breakdown |
| **EfficiencyLens** | Tokens-per-task, cost-per-quality ratios, prediction error, vitality phase |
| **ErrorLens** | Error classification and aggregation (timeout, capability denied, external, logic, input, cancelled) |
| **DriftLens** | Knowledge quality degradation via demurrage-driven balance changes, tier distribution, promotion/demotion rates |
| **AnomalyLens** | Threat detection indicators (part of the immune system) |
| Additional Lenses | Memory lifecycle, trigger lifecycle, extension hooks, calibration |

### 3.2 Observable Events

The system emits structured events across seven lifecycle categories: Signal lifecycle (created, scored, routed, verified, composed, pruned), Cell lifecycle (started, completed, failed, retried, cancelled, predictions published), Graph lifecycle (started, node completed, completed, failed, paused, resumed), Agent lifecycle (tick, regime change, budget update, mode change, phase change), Memory lifecycle (retrieved, stored, consolidated, demurrage applied), Verify lifecycle (pre and post results), and Trigger lifecycle (fired, armed, disarmed).

### 3.3 StateHub Projections

StateHub consumes Lens output and produces typed, versioned projections. Display surfaces (TUI, web dashboard, Slack, audit logs) subscribe to projections -- they never read raw Lens output. This decoupling means new surfaces can be added without modifying the observation layer.

---

## 4. Deployment Modes

All deployment tiers use the same binary. The difference is configuration: environment variables, execution mode, relay involvement, and Bus topology.

### 4.1 Three Scaling Tiers

| Tier | Users | Bus Topology | Description |
|------|-------|-------------|-------------|
| **Solo Developer** | 1 | In-process (tokio::broadcast, sub-microsecond delivery) | `roko serve` on localhost with 1-10 agents. No relay needed. |
| **Small Team** | 2-10 | Relay-backed (local + relay bridge for cross-instance, ~5ms hop) | Single Railway or Fly.io instance with 10-50 agents. Optional relay. |
| **Production** | 10+ | Relay-backed (required, with topic partitioning) | Multi-instance with 50+ agents including isolated execution. |

A Cell publishing a Pulse does not know whether subscribers are in-process or across a relay. The Bus abstraction hides topology differences entirely.

### 4.2 Local Development

```bash
roko init                              # Initialize workspace
roko config secrets set llm.anthropic  # Set API key
roko serve --insecure                  # Start on localhost:6677 (no auth for local dev)
roko dashboard                         # Interactive TUI
```

The control plane provides approximately 85 HTTP routes, SSE, and WebSocket. The TUI connects to the same port and displays real-time Agent status, plan progress, and learning metrics via StateHub projections.

### 4.3 Daemon Lifecycle

The daemon wraps `roko serve` as a managed background process. Commands include `start` (writes PID file), `stop` (SIGTERM with 10s graceful shutdown window, then SIGKILL), `status` (PID alive check plus `/api/health`), `logs` (tail daemon output), and `install` (generates systemd unit on Linux or launchd plist on macOS for auto-start).

The daemon lifecycle is itself expressed as a Graph of Cells with typed inputs and outputs, not ad-hoc shell scripting.

### 4.4 Self-Healing Supervisor

Production crash recovery is a Graph with a circuit-breaker edge. The supervisor runs outside the main roko process so it survives the crash it is recovering from. The pipeline: crash detection (extracts panic signature from stderr) -> error deduplication (tracks signatures in Store, skips already-seen errors) -> diagnosis (LLM-based root-cause analysis, only when enabled) -> fix application (limited to config changes only; code changes require human approval) -> restart.

A circuit breaker prevents crash loops: after 3 consecutive restarts within 5 minutes, the breaker opens, the Graph halts, and a `supervisor.circuit-open` Pulse is emitted requiring human intervention. Auto-fix is disabled by default.

### 4.5 Cloud Deployment

Railway and Fly.io are first-class deployment targets. The same binary runs with different environment variables selecting the scale. Isolated agents run as separate Fly Machines connecting their local Bus to a relay via WebSocket bridge.

### 4.6 WASM Packaging

The core compiles to both native and WASM targets. WASM is used for sandboxed Cell execution and portability where full native binary deployment is impractical. Progressive enhancement: start with native, deploy WASM components where sandboxing or portability matters.

---

## 5. TEE Integration -- Trusted Execution Environments

Roko integrates with Trusted Execution Environments (specifically AWS Nitro Enclaves) for a cryptographic clearing engine that provides collusion-proof agent-to-agent coordination.

### 5.1 The Problem

In agent-to-agent markets, visible orders create three attack vectors: front-running (an agent sees a large order and places its own ahead), sandwich attacks (placing orders on both sides of a victim's order), and information leakage (order flow reveals strategy over time). These problems are especially acute in agent economies where participants operate at machine speed and detect patterns algorithmically.

### 5.2 Commit-Reveal-Clear Protocol

The TEE clearing engine eliminates all three attack vectors through a three-phase protocol:

1. **Commit.** Each agent submits a sealed commitment: a keccak256 hash of their order parameters concatenated with a random nonce. The hash reveals nothing about the order.
2. **Reveal.** After the commit deadline, agents reveal actual parameters. The contract verifies each reveal matches the previously submitted hash. Early reveals are penalized (1% of stake).
3. **Clear.** Revealed parameters are forwarded to the Nitro Enclave via VSOCK. Inside the enclave, orders are decrypted, the optimization runs, and a clearing result with a KKT optimality certificate is produced. No data leaves the enclave except the final result and the proof.

At no point is any agent's order visible to any other agent, to the relay operator, or to the enclave operator before the batch is sealed.

### 5.3 Cooperative Batch Clearing

Clearing happens in cooperative batches rather than continuous order-by-order matching. Every 10 seconds, a batch of orders is sealed, a solver inside the enclave finds the single uniform clearing price that maximizes total economic surplus, and a mathematical certificate (KKT optimality conditions) proves the result is optimal. The certificate can be verified on-chain in O(n) time without re-running the optimization.

The system uses TEE rather than ZK proofs for a practical reason: the clearing optimization is O(N log N) for sorting plus O(80N) for bisection, and generating a ZK proof at 10-second batch cadence is not feasible with current proving systems. The design notes this as a Phase 1 decision with a migration path to ZK as proving systems mature.

---

## 6. Cross-Cutting Concerns

### 6.1 Endofunctor Architecture

Cross-cutting concerns (Memory, Daimon/affect, Dreams/consolidation, Safety) are modeled as **endofunctors F: Signal -> Signal** that transform the cognitive loop from the side. They do not occupy positions in the 7-step agent loop (SENSE, ASSESS, COMPOSE, ACT, VERIFY, PERSIST, REACT) -- they modify it. Each cross-cut wraps Cells with pre/post enrichment hooks.

This has three consequences: cross-cuts compose independently (enable Memory without Daimon), they do not change the loop's topology (the Graph TOML stays the same 7 nodes), and they can be tested independently.

### 6.2 Error Handling

Errors are classified into six categories: Timeout, CapabilityDenied, External, LogicError, InvalidInput, and Cancelled. The ErrorLens aggregates errors by category and by Cell, tracking retry counts and retry success rates. Errors produce Signals that flow through the same Bus as all other data, enabling the same composition and observation patterns.

### 6.3 Configuration

Configuration uses a multi-tier system. Cell declarations are TOML manifests. Graph compositions are TOML files defining nodes and edges. Space grants are workspace-level TOML. Trigger bindings persist as TOML files in `.roko/triggers/` and are re-armed on startup. All configuration is declarative and version-controlled.

---

## 7. Tool System -- How Agents Interact with External Systems

Roko ships over 70 built-in Cells organized by nine protocol types. For external interaction, the relevant protocols are:

### 7.1 Connect Cells (External I/O)

Five built-in Connect Cells handle external system integration: `chain-rpc` (blockchain RPC), `mcp` (Model Context Protocol for LLM tool use), `database`, `webhook`, and `api`. Each declares typed I/O, required capabilities, and cost estimates.

### 7.2 Verify Cells (Safety Boundaries for Tool Output)

Six built-in Verify Cells gate tool outputs: `compile-gate` (checks code compiles), `test-gate` (runs test suites, continuous reward = pass rate), `clippy-gate` (static analysis), `diff-gate` (validates diffs against constraints -- max lines, no secrets, no binaries, restricted paths), `llm-judge-gate` (LLM-based quality evaluation), and `consensus-gate` (multi-verifier agreement).

Every Verify Cell implements both `verify_pre()` (can veto execution before it happens) and `verify_post()` (evaluates results after execution). Pre-action provides safety boundary; post-action provides reward signal for learning. Hard criteria are conjunctive (all must pass). Soft criteria use multi-objective Pareto ranking rather than weighted sums to resist Goodhart effects.

### 7.3 React Cells (Policy Enforcement)

Four React Cells enforce runtime policies: `safety-reactor` (responds to safety violations), `budget-reactor` (enforces cost limits), `escalation-reactor` (notifies humans when thresholds are crossed), and `calibration-policy` (tunes learning parameters based on feedback).

---

## 8. Triggers and Event Handling

The trigger system is push-based and event-driven end to end. There is no polling. Seven built-in trigger kinds handle different event sources:

| Kind | Push Mechanism | Example |
|------|---------------|---------|
| **Cron** | tokio timer | Nightly consolidation at 3:00 AM |
| **Webhook** | HTTP handler on `:6677` | GitHub PR opened |
| **FileWatch** | OS filesystem event via `notify` | Plan file changed |
| **Bus** | Internal event bus subscription | Gate failure detected |
| **ChainEvent** | Chain indexer WebSocket | On-chain identity updated |
| **Manual** | Explicit API/CLI/TUI invocation | Deploy to staging |
| **SignalPattern** | Store graduation subscription | Cluster of 3+ high-severity findings in 5 minutes |

Trigger bindings are persistent TOML configurations stored in `.roko/triggers/` and re-armed on startup. Each binding specifies a Graph to fire, optional input mapping (JSONPath from event payload to Graph input Signals), a concurrency policy (Queue, Skip, CancelRunning, or Parallel), and optional event filters.

Triggers compose through Bus: the output of one Graph publishes Pulses that trigger another Graph, creating event-driven pipelines without explicit wiring between Graphs. Space scoping ensures triggers can only observe topics within their Space's Bus partition and fire Graphs visible within that Space.

---

## 9. Audit Trail and Compliance

### 9.1 Signal-Based Audit

Every grant, usage, denial, and security event is logged as a Signal. Since Signals carry content-addressable hashes, taint levels, and parent lineage, the audit trail is a DAG that can be traversed from any point to reconstruct the full provenance chain. This is structural -- audit is not a separate subsystem bolted on.

### 9.2 Custody Chain

The custody system tracks who created, modified, reviewed, and approved every piece of data. Declassification events (when sensitive data is permitted to cross trust boundaries) are explicitly logged with the approver's identity and the full provenance of the declassified data. The `chain-store` Cell can anchor content hashes on-chain for tamper-evident audit that survives the destruction of the local Store.

### 9.3 Episode Logging

Agent execution is recorded as episodes -- sequences of turns, tool calls, gate results, and outcomes. Each episode carries an HDC fingerprint for similarity matching, enabling the system to recognize when it encounters a situation similar to a previous episode and retrieve the outcome.

### 9.4 Immune Memory as Audit

Layer 5 of the immune system (Immune Memory) stores attack patterns and defensive responses. This creates an institutional memory of security incidents that feeds back into detection, making the system progressively harder to attack with the same techniques.

---

## 10. What Makes This Novel for Agent Systems

### 10.1 Capability Intersection, Not Capability Escalation

Most agent frameworks use additive permission models: agents request capabilities, administrators grant them. Roko inverts this. The effective capability is always the intersection of three independent layers, and capabilities can be narrowed but never widened when delegated. There is no escalation path that bypasses the intersection.

### 10.2 Safety as Architecture, Not Policy

The five-head corrigibility ordering and the Verify-outside-modifiable-surface principle make safety a structural property rather than a policy configuration. The agent cannot modify its own verification pipeline. This is not enforced by a policy check -- it is enforced by the execution engine's architecture. The agent never has access to the code path that invokes verification.

### 10.3 Information Flow Control on Agent Data

Monotonic taint propagation with CaMeL IFC tags on Extension data flows is uncommon in agent frameworks. Most systems treat agent-produced data as uniformly trusted or uniformly untrusted. Roko tracks taint at the data level, through derivation chains, preventing information laundering.

### 10.4 Observability Through the Same Primitives

Lenses produce Signals. Lenses compose into Graphs. Lens output is consumed by the same Bus that carries all other data. This means the observation infrastructure benefits from the same capability controls, taint tracking, and audit logging as the observed computation. There is no separate monitoring stack with its own security model.

### 10.5 TEE for Agent Coordination

Using Trusted Execution Environments to prevent collusion between autonomous agents is a direct response to the unique threat model of agent economies. Traditional security models assume a human adversary. Agent-to-agent markets face adversaries operating at machine speed with algorithmic pattern detection. The commit-reveal-clear protocol inside a Nitro Enclave addresses this by making order flow invisible to all participants until the batch is sealed and the optimal result is cryptographically certified.

### 10.6 Self-Healing as a Graph

The supervisor's crash-recovery pipeline is itself a Graph of Cells with typed inputs, outputs, and a circuit breaker -- not a shell script wrapping a restart command. This means crash recovery is observable (Lenses can watch it), auditable (Signals are logged), and composable (new recovery strategies are new Cells wired into the Graph).
