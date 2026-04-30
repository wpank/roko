# Architecture & Components — How Everything Works

## The Universal Loop

Every agent operation — from a single prompt to a 200-task plan — runs through the same 8-step state machine. Each step is observable, replayable, and individually gated. The loop is the contract between the agent author and the substrate.

```
PROMPT → PLAN → ROUTE → GATE → CALL → OBSERVE → DEPOSIT → ATTEST
```

1. **PROMPT (Intent):** User input plus prior session context. Tokenized, redacted, normalized
2. **PLAN (Decompose):** Generate ordered subtasks. Each is independently routable, gateable, retryable
3. **ROUTE (Cascade):** Cheapest model that can answer it, with a confidence bid. Tier promotion on miss
4. **GATE (Verify):** Eleven gates. PII, cost, policy, schema, jurisdiction. Pre-side-effect
5. **CALL (Execute):** Tool call or model invocation. Idempotent, signed, time-bounded
6. **OBSERVE (Receipt):** Token counts, latency, cost, confidence. Recorded as structured event
7. **DEPOSIT (Compound):** HDC fingerprint of the result deposited into NeuroStore for next session
8. **ATTEST (Settle):** Ed25519 signature over the call envelope, anchored on chain

> *Every step is a verb. Every verb is replayable. The loop is the audit log.*

### Alternate Formulation (Internal Architecture)

The internal Roko architecture uses a 1-noun-6-verb pattern:

**1 Noun:** Signal — content-addressed, durable data with demurrage decay

**6 Verbs (Traits):**
- **Substrate** — storage and retrieval
- **Scorer** — evaluation and ranking
- **Gate** — verification and enforcement
- **Router** — model and task routing
- **Composer** — prompt assembly and enrichment
- **Policy** — authorization and constraints

**Universal internal loop:**
```
query → score → route → compose → act → verify → write → react
```

### Three Primitives

1. **Signal** — Durable, content-addressed data. Has demurrage-based decay (loses value without renewal). The noun that flows through the system
2. **Pulse / Bus** — Ephemeral events with predict-publish-correct mechanism. Every Cell predicts outcomes, Bus records predictions and errors, enabling continuous learning
3. **Cell** — Atomic computation unit implementing 9 protocols: Store, Score, Verify, Gate, Route, Compose, React, Observe, Connect, Trigger. Ten specializations: Flow, Rack, Trigger, Lens, Loop, Memory, Space, Extension, Agent, Connector

---

## Roko Runtime — Component Deep Dive

### CascadeRouter (Cost-Aware Model Routing)

Routes each query to the cheapest model that can answer it, with measured confidence intervals from prior calls.

**How it works:**
- Each tier (model) carries a measured confidence interval from accumulated call data
- The router bids the lowest tier whose confidence interval covers the current task
- On miss (confidence below threshold), the next tier runs and the verdict updates the prior
- The router gets sharper with traffic — workload-model fit, learned per tenant

**Cascade tiers (example):**

| Tier | Model | Cost/call | Confidence |
|------|-------|-----------|------------|
| 1 | haiku-3.5 | $0.001 | 82% |
| 2 | gpt-4o-mini | $0.004 | 94% |
| 3 | claude-sonnet | $0.015 | 99% |
| 4 | opus-4.7 | $0.075 | escalate |

**Result:** 4–10x cost reduction vs single-model deployment, with measured quality maintenance.

**Persistence:** Router state persists to `.roko/learn/cascade-router.json`. Configurable models. Learns across sessions.

**Integration with LLM backends:** Roko dispatches to 8+ backends — Claude CLI, Claude API, OpenAI-compatible, Ollama, Gemini, Perplexity, Codex, Cursor. The cascade router sits in front of all of them.

### Gate Pipeline (11 Gates, 7 Rungs)

Eleven verifiers sit between every model call and every side effect. "The model decided" is not an acceptable answer to a regulator. Each gate signs its verdict. Each verdict is anchored. Each violation is a structured event, not an exception.

**The 11 Gates:**

| # | Gate | What It Verifies |
|---|------|-----------------|
| 1 | **Schema** | Output conforms to JSON/protobuf contract |
| 2 | **PII** | Personally identifiable data redacted before egress |
| 3 | **Cost** | Per-call and per-session cost ceilings enforced |
| 4 | **Latency** | Hard SLA timeouts with graceful tier promotion |
| 5 | **Policy** | SOC 2, FINRA, EU AI Act envelopes per tenant |
| 6 | **Jurisdiction** | Routing constrained to data-residency boundary |
| 7 | **Idempotent** | Same input, same output, same attestation |
| 8 | **Provenance** | Tool inputs traced back to attested sources |
| 9 | **Budget** | Workload-level credit envelope verified live |
| 10 | **Consent** | Tenant scope checked against agent identity |
| 11 | **Audit** | Every verdict written, none silently bypassed |

**7-Rung Pipeline:** Gates are organized in rungs with early-exit — if test passes at rung 3, rungs 4–7 don't execute. Language-agnostic (supports cargo/npm/pytest/jest/clippy/eslint/ruff/golangci-lint).

**6 Anti-Goodhart Safeguards:** Prevent agents from gaming gate metrics.

**Adaptive thresholds:** Gate thresholds adjust via EMA per rung, persisted to `.roko/learn/gate-thresholds.json`.

> *A bypassed gate is a structured event. A passed gate is a signed proof.*

### NeuroStore (Hyperdimensional Memory)

A durable knowledge store indexed by 10,000-dimensional hyperdimensional computing (HDC) binary vectors. Sub-millisecond nearest-neighbor recall.

**How it works:**
- Every call's input, output, plan, and verdict gets a holographic HDC fingerprint (10,240-bit binary vector, 1,280 bytes per entry)
- New calls retrieve nearest neighbors in <1μs
- Per-tenant by default, sharded. Cross-tenant via marketplace at 5% fee
- Ebbinghaus-style decay with tier progression (Transient → Persistent)
- Dreams consolidation cycle for offline knowledge compression

**Why HDC:**
- 10,240-bit binary vectors: ~1μs similarity search
- 100K entries ≈ 128 MB memory footprint
- MAP-binary is 3–4x faster than HRR (holographic reduced representations)
- PathHD shows 40–60% latency reduction
- Resonator Networks achieve 10x faster factoring
- NorthPole projection: 100M searches/sec theoretical

**Storage and decay:**
- 7-day half-life for decay (demurrage principle — data without renewal loses value)
- Tier progression: entries that prove useful are promoted from Transient to Persistent
- Dreams cycle: offline consolidation for cross-referencing and compression

> *The thousandth agent learns from the first.*

### EpisodeLogger (Deterministic Replay)

Every agent turn — tool calls, model calls, decisions, results — recorded as structured episodes in `.roko/episodes.jsonl`. Each episode includes:
- Agent turn data with structured metadata
- Gate results per task
- HDC fingerprint of the episode
- Timestamps, costs, model used, confidence scores

Episodes enable:
- Deterministic replay across non-deterministic LLMs
- Audit trail for compliance
- Training data for router improvement
- Playbook extraction for future agents

### SystemPromptBuilder (9-Layer Prompt Assembly)

Assembles agent system prompts from 9 layers:

1. **Role definition** — agent's purpose and constraints
2. **Domain context** — relevant domain knowledge
3. **Task context** — current task description and requirements
4. **Tool descriptions** — available tools and their schemas
5. **Knowledge hints** — relevant NeuroStore entries (via HDC similarity search)
6. **Playbook injection** — proven strategies from prior episodes
7. **Policy constraints** — safety rules, budget limits, jurisdiction
8. **Session history** — relevant prior turns
9. **Enrichment** — additional context from bidding system

Uses role templates from `crates/roko-compose/src/templates/`.

### Context Bidding (Attention Economy)

Three context bidders compete for limited prompt space:

- **NeuroBidder** — bids based on NeuroStore knowledge relevance
- **TaskBidder** — bids based on task requirements
- **ResearchBidder** — bids based on research artifact relevance

VCG auction mechanism (`vcg_allocate`) is built and exported but currently greedy path dominates at runtime. Each bidder submits an `AttentionBid` with relevance score; highest-value context wins prompt space.

### Predict-Publish-Correct (Learning Loop)

Every Cell (computation unit) follows this pattern:
1. **Predict** — register a falsifiable prediction before acting (cost estimate, time estimate, outcome probability)
2. **Publish** — execute the action, record actual outcomes
3. **Correct** — compute residual (predicted vs actual), update priors

This mechanism drives continuous improvement: cost forecasts get sharper, risk estimates get more accurate, routing decisions get cheaper. The residual is the learning signal.

### ProcessSupervisor (Lifecycle Management)

`PlanRunner` in `roko-runtime` manages agent lifecycle:
- Tracks running agents
- Handles graceful shutdown
- Manages cancellation via event bus
- Provides circuit-breaking via `roko-conductor` (10 watchers, diagnosis)

### Orchestration Engine

The main orchestration loop in `orchestrate.rs`:
1. Plan discovery — find and parse task DAGs from TOML
2. DAG execution — parallel execution with dependency resolution
3. Agent dispatch with enrichment — knowledge hints, playbook queries, context bidding, 9-layer system prompts
4. Gate validation — per-task gate pipeline execution
5. State persistence — snapshot + resume via `.roko/state/executor.json`
6. Learning updates — efficiency events, cascade router updates, prompt experiments
7. Gate failure replan — `build_gate_failure_plan_revision` when gates fail
8. C-Factor computation — `CFactorSummary` for full metrics

---

## Korai Substrate — Component Deep Dive

### ERC-8004 Agent Identities

Open standard for cryptographic agent identity:

- **Transferable** across organizations
- **SPIFFE SVID-based** — namespace + key ID + standard version
- **7-domain EMA reputation** — time-decayed across: reliability, accuracy, cost-efficiency, safety, speed, compliance, knowledge
- **22,900 registrations** in first 3 days on Ethereum mainnet (March 17, 2026)

```
spiffe://acme/research     — namespace
0x4c3a...e2f1              — key ID
erc-8004 v1                — standard
```

An agent's identity carries its provenance: which tenant chartered it, what gates it inherits, what budget envelope it obeys. Identity persists when containers die.

### HDC Precompile (0xA01)

Native hyperdimensional computing at the consensus layer:

- Fixed gas cost (~400 gas for top-K similarity)
- Available to any smart contract in a single opcode
- ~170μs for 100K vector queries
- Same fingerprint format as Roko's NeuroStore — seamless bridge between off-chain memory and on-chain knowledge

### ZK-HDC Proofs

Prove behavioral fingerprints without revealing underlying data:

- **Implementation:** Circom + Groth16
- **Proving time:** <1 second
- **Verification gas:** ~250K
- **Use case:** Cross-organization reputation verification without exposing proprietary data
- **Research basis:** Bionetta/UltraGroth 373x faster than Halo2

### Simplex Consensus

BFT-family consensus optimized for agent coordination:

- **Block time:** ~50ms
- **Finality:** ~3 seconds
- **Throughput:** ~3,200 tx/s
- **Initial topology:** Co-located Tokyo validators (Phase 1)
- **Gateway median:** ~50ms

### Cooperative Clearing Engine

Multi-tenant agent work settlement without a trusted clearinghouse:

**How it works:**
1. Orders submitted with hash-locked predictions
2. Clearing triggered by: 5 orders, 10s elapsed, imbalance threshold, or 10bp price move
3. 800ms solver competition — permissionless, maximize surplus
4. KKT (Karush-Kuhn-Tucker) verification — provably optimal, O(n) linear program
5. 1.2s settlement across three blocks
6. CRPS scoring — strictly proper, truth-telling is the optimal strategy
7. ClearingInsight emission to knowledge store

**Why KKT works for yield perpetuals:** Convexity maintained (partially fillable, continuous sizes, linear payoffs), convex feasible set, concave objective — KKT conditions are necessary and sufficient (Boyd & Vandenberghe).

**Solver economics:** 5% of surplus, capped. 50 NUNCHI bond. 10-block challenge window. 10% bond slash on loss. Permissionless verification.

**Graceful degradation:** Normal → cooperative KKT → orders roll → CLOB fallback → circuit breaker.

### ISFR (Internet Secured Funding Rate)

A composite benchmark index representing the cost of secured funding across DeFi. Computed by validators every 10 seconds via oracle precompile.

**Why it matters:** The gap between $668 trillion in TradFi OTC interest rate derivatives and <$100M in DeFi rate derivatives isn't a market inefficiency — it's a missing primitive. ISFR is that primitive.

**Two-level aggregation (mirrors SOFR methodology):**

**Level 1 — Intra-class:** TVL-weighted median per source class

| Class | Weight | V1 Sources | What It Measures |
|-------|--------|------------|-----------------|
| LENDING | 0.60 | Aave V3, Compound V3 | Collateralized lending yield |
| STRUCTURED | 0.25 | Ethena sUSDe | Delta-neutral strategy yield |
| FUNDING | 0.10 | Hyperliquid ETH perp | Perpetual futures funding rate |
| STAKING | 0.05 | ETH staking rate | PoS validator yield |

**Level 2 — Inter-class:** Weighted sum
```
ISFR = 0.60 × LENDING + 0.25 × STRUCTURED + 0.10 × FUNDING + 0.05 × STAKING
```

**Manipulation resistance:** 200% funding spike contributes only 20bp to composite. Need 49% TVL control of a single class to move it. Two layers make manipulation exponentially more expensive.

**Hybrid oracle + market:**
```
ISFR = ISFR_oracle + EMA(ISFR_market - ISFR_oracle)
```
At launch: pure oracle. At maturity: market-led with oracle anchor. No binary cutover.

**Self-improving (V2):**
- Self-calibrating source confidence via leave-one-out MSPE
- Adaptive class weights via Bates-Granger optimal combination
- Kalman filter smoothing (distinguishes measurement vs process noise)
- Cost-stratified trim fractions per class
- Nelson-Siegel 4-parameter yield curve published on-chain

**Prediction layer:**
- Agents predict ISFR every 10 seconds (8,640 predictions/day)
- Scored via CRPS (strictly proper — truthful reporting is the unique optimal strategy)
- Epistemic reputation tiers: Oracle (top 10%, 2x quota, 0.5x gamma), Calibrated, Standard, Uncalibrated

---

## The Gateway — The 50ms Path

Three cache layers sit in front of every model call:

```
CLIENT → L1 EXACT → L2 SEMANTIC → L3 PARTIAL → CASCADE → MODEL
```

| Layer | Method | Latency | Hit Rate |
|-------|--------|---------|----------|
| L1 Exact | BLAKE3 hash match | ~5ms | 18% |
| L2 Semantic | HDC fingerprint similarity | ~50ms | 31% |
| L3 Partial | SimHash plan reuse | ~120ms | 9% |
| Cascade | Fall-through to model | ~800ms | 42% |

**58% of calls finish at the gateway, never reaching a model.** The remaining 42% are routed by Cascade to the cheapest tier that can answer.

---

## The Cost Stack — Why 10–30x

Practical cost reduction is multiplicative, not additive:

| Lever | Multiplier | Mechanism |
|-------|-----------|-----------|
| Cache hit (58% never reach model) | ×0.42 | Three-layer gateway cache |
| Cascade routing (avg 4x cheaper) | ×0.30 | Confidence-bid tier selection |
| Context trim (fingerprint reuse) | ×0.65 | HDC-based redundancy elimination |
| Batching (amortized overhead) | ×0.85 | Header + auth amortization |
| **Composed** | **×0.069** | **$0.069 per $1.00 of intent** |

From HAL benchmark: $42.11 naive → $1.42 Nunchi optimized = **~30x reduction**.

Each lever is modest. Composed, they compress an order of magnitude.

> *Cheaper inference is competitive. Composed coordination is structural.*

---

## The Learning Stack

### Continuous Learning Systems

| System | What It Does | Storage |
|--------|-------------|---------|
| **EpisodeLogger** | Records agent turns + gate results | `.roko/episodes.jsonl` |
| **CascadeRouter** | Learns workload-model fit per tenant | `.roko/learn/cascade-router.json` |
| **Prompt Experiments** | A/B testing of prompt variants | `.roko/learn/experiments.json` |
| **Adaptive Gate Thresholds** | EMA per rung | `.roko/learn/gate-thresholds.json` |
| **Efficiency Events** | Per-turn cost/latency/quality metrics | `.roko/learn/efficiency.jsonl` |
| **Playbooks** | Proven strategies extracted from episodes | Queried at dispatch time |
| **C-Factor** | Composite performance metric | Computed per task |

### Predict-Publish-Correct Everywhere

The predict-publish-correct mechanism applies at every level:
- **Cost prediction:** Router predicts cost before routing; actual cost updates the prior
- **Time prediction:** Estimated completion time vs actual drives scheduling
- **Quality prediction:** Expected gate pass rate vs actual drives model selection
- **Outcome prediction:** ISFR predictions scored via CRPS for reputation

### Dream Consolidation

Offline knowledge compression cycle:
- **Hypnagogia:** Initial pattern recognition across recent episodes
- **Imagination:** Cross-referencing and hypothesis generation
- **Cycle:** Consolidation, compression, tier promotion
- Built in `crates/roko-dreams/`, triggered from orchestrate.rs

---

## Protocol Integrations

### Exoskeleton Protocols

Nunchi integrates with (not replaces) the emerging agent protocol stack:

| Protocol | Downloads/Adoption | What It Owns | Nunchi's Relationship |
|----------|-------------------|-------------|----------------------|
| **MCP** | 97M monthly SDK downloads | Tool & context access | Native passthrough in `roko.toml` |
| **A2A** | 150+ organizations | Agent interoperability | Coordination layer above |
| **ERC-8004** | 22.9K registrations | Identity & reputation | Native at Korai genesis |
| **x402** | $50M volume, 165M txns | Machine payments | Settlement rail integration |

> *Each protocol owns a rail. Coordination across them is empty. That's Nunchi.*

---

## Self-Hosting — The Complete Workflow

Roko develops itself using its own tooling:

```bash
# 1. Capture a work item
roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"

# 2. Draft a PRD
roko prd draft new "system-prompt-wiring"

# 3. Research for context
roko research enhance-prd system-prompt-wiring

# 4. Generate implementation plan + tasks
roko prd plan system-prompt-wiring

# 5. Execute the plan (agents run tasks, gates validate, state persists)
roko plan run plans/

# 6. Resume if interrupted
roko plan run plans/ --resume .roko/state/executor.json

# 7. Watch progress
roko dashboard

# 8. Check status
roko status
```

The system that builds features is the same system that builds *itself*. 177K lines of Rust, 18 crates, the majority generated and validated through this loop.

---

## Infrastructure Surface

### HTTP Control Plane (~85 Routes)

`roko serve` exposes a REST API on :6677 for dashboards, external callers, and programmatic control. Includes SSE and WebSocket support.

### Interactive TUI (ratatui)

`roko dashboard` provides a terminal UI with F1–F7 tabs for monitoring plan execution, agent status, gate results, learning metrics, and knowledge state.

### Per-Agent Sidecar (13 Routes)

`roko-agent-server` provides per-agent HTTP endpoints: `/message` (real LLM dispatch), `/stream` (WebSocket), `/predictions`, `/research`, `/tasks`.

### CLI Reference

60+ subcommands covering: core workflow, planning & PRDs, agent management, research, knowledge (neuro + dreams + custody + archive), learning & feedback, jobs, configuration, server & deployment, utilities.
