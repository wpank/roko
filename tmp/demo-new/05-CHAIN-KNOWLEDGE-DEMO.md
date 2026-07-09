# Demo: Knowledge Sharing Between Agents

Two demos that show the same core idea at different complexity levels:
knowledge produced by one agent gets shared, verified, and reused by another.

| Demo | Domain | Chain? | Complexity | Time | What it proves |
|------|--------|--------|-----------|------|----------------|
| **A: Code Agents** | Two REST APIs | No | Lower | ~3 min | Knowledge compounds across related tasks |
| **B: DeFi Agents** | Yield + Hedging on forked mainnet | Yes | Higher | ~5 min | Chain-anchored knowledge graph with real protocol data |

Both run live (real LLM calls, real output). Demo A works today with minimal wiring.
Demo B requires the chain infrastructure.

---

# DEMO A: Code Knowledge (No Chain)

## The Pitch

> "Two agents are building REST APIs in separate projects — one for user management,
> one for inventory. Watch the first agent figure out error handling patterns, input
> validation, and test structure. Now watch the second agent skip all that discovery
> and start with proven patterns from the first. Same problem domain, different project,
> shared knowledge. The second API gets built faster and cheaper."

## Why This Demo First

- Uses only `roko run` — no chain, no mirage, no DeFi ABIs
- Exercises the existing episode → distillation → knowledge store → playbook pipeline
- Every piece already exists in the codebase (just needs wiring)
- Demonstrates the value prop without any blockchain complexity
- Fast to build, reliable to run, easy to reason about

## Setup

```
┌────────────────────────────────────────────────────────┐
│  roko serve (:6677)                                    │
│  - Episodes logged to .roko/episodes.jsonl             │
│  - Knowledge distillation after each run               │
│  - Playbook store queried at dispatch time              │
│  - Workflow projection via SSE + WS                    │
└───────────────┬────────────────────────┬───────────────┘
                │                        │
       ┌────────▼────────┐      ┌───────▼─────────┐
       │  Agent Alpha    │      │  Agent Beta     │
       │  "User API"     │      │  "Inventory API"│
       │                 │      │                 │
       │  Build a user   │      │  Build an       │
       │  management     │      │  inventory      │
       │  REST API with  │      │  REST API with  │
       │  CRUD, auth,    │      │  CRUD, search,  │
       │  validation     │      │  validation     │
       └────────┬────────┘      └───────┬─────────┘
                │                        │
                ▼                        ▼
       ┌────────────────────────────────────────┐
       │  Shared Knowledge Store               │
       │  .roko/neuro/knowledge.jsonl          │
       │  .roko/learn/playbooks.jsonl          │
       │  - Heuristics, strategies, warnings   │
       │  - Queried at dispatch → system prompt │
       └────────────────────────────────────────┘
```

### Agent Roles

**Agent Alpha — "User API"**
- Prompt: "Build a REST API in Rust using actix-web for user management. Include CRUD endpoints, input validation, structured error responses, and integration tests."
- This is the **first** agent. No prior knowledge. Cold start.
- Expected discoveries (the insights it will produce):
  - Actix-web handler patterns (extractors, responders, error types)
  - Validation approach (serde + custom validators or validator crate)
  - Error response structure (JSON error body with code + message + details)
  - Test patterns (reqwest client, test server setup, assertion style)
  - Cargo.toml dependency choices

**Agent Beta — "Inventory API"**
- Prompt: "Build a REST API in Rust using actix-web for inventory management. Include CRUD endpoints for products, search/filter, input validation, structured error responses, and integration tests."
- This is the **second** agent. It runs after Alpha finishes.
- At dispatch time, roko queries the playbook store and knowledge store. Alpha's insights get injected into Beta's system prompt.
- Expected behavior:
  - Skips the "which framework?" decision — knowledge says actix-web
  - Skips the "how to structure errors?" exploration — uses Alpha's pattern
  - Starts with a proven test harness — copies Alpha's reqwest setup
  - Produces code faster, with fewer iterations, at lower cost

### The Knowledge Transfer Moment

1. Alpha finishes building the User API. Episodes are logged.
2. Roko's distiller (Claude Haiku) processes Alpha's episodes → extracts KnowledgeEntry records:
   - Heuristic: "Use actix_web::web::Json<T> for request bodies with #[derive(Deserialize, Validate)]"
   - Strategy: "Error responses should return JSON with {error: string, code: number, details: object}"
   - Heuristic: "Integration tests: spawn actix test server, use reqwest::Client, assert status codes"
   - Warning: "actix-web default error handler returns plain text — override with custom ErrorResponse"
3. Beta starts. At dispatch, `PlaybookStore::query("inventory REST API actix-web")` returns these entries.
4. The entries get injected into Beta's system prompt under a "## Prior Knowledge" section.
5. Beta skips the exploration phase and starts building with proven patterns.

**What the audience sees:** Beta's terminal output is visibly shorter. The side panel shows
"Knowledge entries applied: 4" and "Estimated savings: ~2 turns, ~$0.15". The efficiency
metrics bar shows the cumulative cost comparison.

## UI Layout

### Demo Tab: "Knowledge Transfer"

Uses existing 2-pane layout (same as the Race scenario). No new components needed
beyond the knowledge flow panel.

```
┌──────────────────────────────────────────────────────────────────┐
│ [1 PRD Pipeline] ... [9 Knowledge Transfer] [10 Chain Intel]     │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────┐  ┌─────────────────────────────┐   │
│  │  AGENT ALPHA             │  │  AGENT BETA                 │   │
│  │  "User API" (cold start) │  │  "Inventory API" (w/ knowledge) │
│  │                          │  │                             │   │
│  │  xterm terminal           │  │  xterm terminal             │   │
│  │  ░░░░░░░░░░░░░░░░░░░░░░  │  │  (waiting for Alpha...)    │   │
│  └─────────────────────────┘  └─────────────────────────────┘   │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  KNOWLEDGE PANEL                                           │  │
│  │                                                            │  │
│  │  ┌─ Alpha ─┐    distill    ┌─ Store ──┐   inject   ┌─ Beta ─┐│
│  │  │ episodes│ ────────────▶ │ insights │ ─────────▶ │ prompt ││
│  │  └─────────┘               └──────────┘            └────────┘│
│  │                                                            │  │
│  │  Insights extracted: 0          Applied to Beta: 0         │  │
│  │                                                            │  │
│  │  ┌─────────────────────────────────────────────────────┐   │  │
│  │  │ (insight cards appear here as Alpha produces them)  │   │  │
│  │  └─────────────────────────────────────────────────────┘   │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Alpha cost: $0.42  |  Beta cost: $0.28  |  Saved: 33%    │  │
│  │  Alpha turns: 8     |  Beta turns: 5     |  Saved: 3      │  │
│  │  Alpha time: 2:14   |  Beta time: 1:31   |  Saved: 43s    │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### UI Sections

**1. Agent Terminals (top, side-by-side)**
- Existing 2-pane Race scenario layout. No new components.
- Left pane has rose border, right has sage border.
- Right pane shows "Waiting for Alpha to finish..." overlay until Alpha completes.

**2. Knowledge Panel (middle)**
- Simple flow diagram: Alpha episodes → Store → Beta prompt
- Animated: particles flow left→center when Alpha produces episodes
- Animated: particles flow center→right when Beta starts and knowledge is injected
- Below the diagram: scrolling list of insight cards
- Each card shows: kind badge (HEURISTIC/STRATEGY/WARNING), content preview, confidence

**3. Comparison Bar (bottom)**
- Three-column comparison: Alpha metrics | Beta metrics | Savings
- Cost, turns, wall-clock time
- Savings column highlights in gold (`--color-bone`)

### Implementation: Scenario Definition

```typescript
// New entry in scenarios.ts
const knowledgeTransfer: Scenario = {
  id: 'knowledge-transfer',
  title: 'Knowledge Transfer',
  subtitle: 'Two agents build similar APIs. The second one learns from the first.',
  panes: 2,
  labels: ['Agent Alpha (cold start)', 'Agent Beta (with knowledge)'],
  panel: true,
  promptBar: false,
  steps: [
    { label: 'Setup workspaces', sublabel: 'roko init × 2' },
    { label: 'Alpha builds User API', sublabel: 'roko run (cold)' },
    { label: 'Distill knowledge', sublabel: 'episodes → insights' },
    { label: 'Beta builds Inventory API', sublabel: 'roko run (warm)' },
    { label: 'Compare results', sublabel: 'efficiency metrics' },
  ],
  async run(ctx) {
    const [alpha, beta] = ctx.entries;
    const ROKO = getRoko();
    ctx.timeline.init(this.steps);

    // ── Phase 1: Setup ──────────────────────────────────────────
    ctx.timeline.setActive(0);
    ctx.playback.setProgress(0, 5, 'Setting up workspaces');

    // Both terminals init in parallel
    const dirA = await setupWorkspace(alpha, 'roko-user-api');
    const dirB = await setupWorkspace(beta, 'roko-inventory-api');

    // Beta shows waiting state
    beta.execCmd('echo "⏳ Waiting for Agent Alpha to finish..."', 5000);

    // ── Phase 2: Alpha builds (cold start) ──────────────────────
    await ctx.playback.waitForStep();
    ctx.timeline.setActive(1);
    ctx.playback.setProgress(1, 5, 'Alpha building User API');

    const alphaResult = await showCmd(alpha,
      `${ROKO} run "Build a REST API in Rust using actix-web for user management. ` +
      `Include CRUD endpoints for users, input validation with the validator crate, ` +
      `structured JSON error responses, and integration tests with reqwest."`,
      {
        timeout: 300000, // 5 minutes
        onLog: ctx.logCommand,
        onGate: ctx.setGate,
      }
    );

    ctx.setMetric('cost', alphaResult.cost ?? '$?.??');
    ctx.setMetric('time', `${alphaResult.elapsed.toFixed(0)}s`);

    // ── Phase 3: Distill knowledge ──────────────────────────────
    await ctx.playback.waitForStep();
    ctx.timeline.setActive(2);
    ctx.playback.setProgress(2, 5, 'Distilling knowledge from Alpha');

    // Show the knowledge extraction happening
    await showCmd(alpha, `${ROKO} learn all`, {
      timeout: 60000,
      onLog: ctx.logCommand,
    });
    await showCmd(alpha, `${ROKO} knowledge stats`, {
      timeout: 30000,
      onLog: ctx.logCommand,
    });

    // TODO: Update knowledge panel with extracted insights
    // This requires reading .roko/neuro/knowledge.jsonl from the serve API
    // and populating the insight cards in the UI

    // ── Phase 4: Beta builds (with knowledge) ───────────────────
    await ctx.playback.waitForStep();
    ctx.timeline.setActive(3);
    ctx.playback.setProgress(3, 5, 'Beta building Inventory API (with knowledge)');

    // Clear Beta's waiting message
    beta.clearTerminal();

    // Copy knowledge store from Alpha's workspace to Beta's
    // (In production, they'd share a knowledge store. For demo, we copy.)
    await beta.execCmd(
      `cp -r ${dirA}/.roko/neuro ${dirB}/.roko/neuro 2>/dev/null; ` +
      `cp -r ${dirA}/.roko/learn ${dirB}/.roko/learn 2>/dev/null; ` +
      `echo "Knowledge store synced from Alpha"`,
      10000
    );

    const betaResult = await showCmd(beta,
      `${ROKO} run "Build a REST API in Rust using actix-web for inventory management. ` +
      `Include CRUD endpoints for products, search and filter, input validation, ` +
      `structured JSON error responses, and integration tests with reqwest."`,
      {
        timeout: 300000,
        onLog: ctx.logCommand,
        onGate: ctx.setGate,
      }
    );

    // ── Phase 5: Compare ────────────────────────────────────────
    await ctx.playback.waitForStep();
    ctx.timeline.setActive(4);
    ctx.playback.setProgress(4, 5, 'Comparing results');

    await showCmd(beta, `${ROKO} learn efficiency`, {
      timeout: 30000,
      onLog: ctx.logCommand,
    });

    ctx.timeline.setActive(5);
  },
};
```

## What Needs to Exist in Roko

### Already built — use directly

| Component | Where | Notes |
|-----------|-------|-------|
| `roko run "<prompt>"` | `crates/roko-cli/src/` | Single-shot agent dispatch with gates |
| Episode logging | `crates/roko-learn/src/episode_logger.rs` | Appends to `.roko/episodes.jsonl` after each task |
| Knowledge distiller | `crates/roko-neuro/src/distiller.rs` | Episodes → Claude Haiku → `KnowledgeEntry` records |
| Knowledge store | `crates/roko-neuro/src/knowledge_store.rs` | JSONL append, HDC similarity, decay, tiers |
| Playbook store | `crates/roko-learn/src/playbook.rs` | Goal → steps, queried at dispatch |
| Playbook injection | `crates/roko-cli/src/orchestrate.rs` | `PlaybookStore::query()` at dispatch → system prompt |
| `roko learn all` | CLI subcommand | Shows episodes, router, experiments, efficiency |
| `roko knowledge stats` | CLI subcommand | Shows knowledge store metrics |
| 2-pane terminal layout | `demo/demo-app/` Race scenario | Existing component, just reuse |
| showCmd / detectFromOutput | `demo/demo-app/src/hooks/useTerminalSession.ts` | Types command, waits for output, detects gates/cost |

### Needs wiring

| What | Gap | How to fix | Effort |
|------|-----|-----------|--------|
| **Distillation trigger after `roko run`** | `roko run` logs episodes but doesn't trigger distillation automatically. Need to call distiller at end of run. | Add `distiller.distill(episodes)` call after agent loop in `orchestrate.rs` `run_single` path, or add a `roko learn distill` subcommand. | Small |
| **Knowledge injection at dispatch** | `PlaybookStore::query()` is called at dispatch but `KnowledgeStore::search()` may not be. Need to check that knowledge entries (not just playbooks) get injected into the system prompt. | Check `orchestrate.rs` `dispatch_agent_with` — verify `enrichment_sources` includes neuro store. If not, add `knowledge_store.search(task_context)` alongside playbook query. | Small |
| **Shared knowledge between workspaces** | Two separate `roko run` invocations in different directories have separate `.roko/` dirs. Knowledge doesn't automatically flow between them. | Options: (a) Copy `.roko/neuro/` between dirs (hacky but works for demo), (b) Use `roko serve` as central knowledge broker (proper but more work), (c) Run both in same workspace but different subdirs. | Small for (a), Medium for (b) |
| **Knowledge panel React component** | No existing component shows knowledge entries being extracted and applied. | New React component: fetch from `/api/knowledge/entries` or read from SSE workflow events. Show insight cards with kind/content/confidence. | Medium |
| **Comparison metrics bar** | No existing component compares two agent runs side-by-side with savings calculation. | Simple React component with 3-column layout. Data from `showCmd` return values (cost, elapsed, turns). | Small |

### Implementation detail: Knowledge injection path

The critical code path that makes this demo work:

```
orchestrate.rs::dispatch_agent_with()
  │
  ├── PlaybookStore::query(task_title, role, task_id)
  │     → Returns Vec<Playbook> ranked by relevance + recency
  │
  ├── KnowledgeStore::search(task_context)         ← VERIFY THIS EXISTS
  │     → Returns Vec<KnowledgeEntry> by HDC similarity
  │
  └── SystemPromptBuilder::with_playbooks(playbooks)
      ::with_knowledge(entries)                     ← VERIFY THIS EXISTS
        → Renders into system prompt:
          "## Prior Knowledge
           The following insights were extracted from previous agent runs:
           - [HEURISTIC] Use actix_web::web::Json<T> for request bodies...
           - [STRATEGY] Error responses should return JSON with...
           - [WARNING] actix-web default error handler returns plain text..."
```

File to check: `crates/roko-cli/src/orchestrate.rs` — search for `playbook` and `knowledge`
or `neuro` to verify the injection path exists.

File to check: `crates/roko-compose/src/system_prompt_builder.rs` — verify it has a
`with_knowledge()` or similar method for injecting knowledge entries.

---

# DEMO B: DeFi Chain Intelligence

## The Pitch

> "Two autonomous agents working DeFi strategies against a fork of Ethereum mainnet.
> Real Aave rates, real Uniswap pools, real on-chain state. As they work, they produce
> insights — posted to an on-chain knowledge graph. The other agent picks them up,
> confirms them, builds on them. Watch cost decrease and quality increase in real time.
> Every insight is chain-attested — tamper-proof, auditable, and composable."

## Setup

### Infrastructure Stack

```
┌───────────────────────────────────────────────────────────────┐
│  mirage-rs (forked ETH mainnet at latest block)               │
│                                                               │
│  JSON-RPC (:8545)          HTTP REST (:8545)                  │
│  ├── eth_* methods          ├── GET  /api/health              │
│  ├── chain_postInsight      ├── GET  /api/knowledge/entries   │
│  ├── chain_searchInsights   ├── POST /api/knowledge/{id}/confirm │
│  ├── chain_confirmInsight   ├── GET  /api/pheromones          │
│  └── chain_stats            ├── GET  /api/pheromones/heatmap  │
│                             ├── GET  /api/agents              │
│  WebSocket (:8545/api/ws)   ├── GET  /api/stats               │
│  ├── channel: insight       └── GET  /api/deployment          │
│  ├── channel: pheromone                                       │
│  ├── channel: agent         Block interval: 2000ms            │
│  └── channel: prediction    Snapshot: every 30s               │
│                                                               │
│  Forked state includes:                                       │
│  ├── Aave V3 LendingPool (0x87870B...2725Dd)                 │
│  ├── Uniswap V3 Router   (0xE59242...61564)                  │
│  ├── USDC                 (0xA0b869...E1c7A)                  │
│  ├── WETH                 (0xC02aaA...756Cc2)                 │
│  └── All pool contracts at fork block                         │
│                                                               │
│  Pre-funded wallets:                                          │
│  ├── Alpha: 0x...  (10 ETH + 500K USDC via deal cheatcode)   │
│  └── Beta:  0x...  (110 ETH via deal cheatcode)               │
└───────────────┬────────────────────────┬──────────────────────┘
                │                        │
       ┌────────▼────────┐      ┌───────▼─────────┐
       │  Agent Alpha    │      │  Agent Beta     │
       │  "Yield Scout"  │      │  "Risk Hedger"  │
       └────────┬────────┘      └───────┬─────────┘
                │                        │
                ▼                        ▼
       ┌────────────────────────────────────────┐
       │  On-Chain Knowledge Graph              │
       │  (mirage chain_* RPC methods)          │
       │                                        │
       │  Insight lifecycle:                    │
       │  Posted → Active → Confirmed/Challenged│
       │  HDC vectors for similarity search     │
       │  Automatic time-decay (half-life)      │
       │  Cross-agent confirmation records      │
       └────────────────────────────────────────┘
```

### Startup Sequence

```bash
# 1. Start mirage-rs with mainnet fork + chain extensions
cargo run -p mirage-rs --features chain -- \
  --rpc-url https://eth.llamarpc.com \
  --block-interval-ms 2000 \
  --chain-id 1 \
  --enable-knowledge true \
  --enable-stigmergy true \
  --enable-hdc true

# 2. Fund agent wallets (using Anvil cheatcodes on the fork)
# Alpha gets USDC (deal cheatcode sets balance directly)
cast rpc anvil_setBalance 0xAlpha 10000000000000000000  # 10 ETH
cast send 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  "transfer(address,uint256)" 0xAlpha 500000000000 \
  --from 0x... --unlocked  # USDC whale

# Beta gets ETH
cast rpc anvil_setBalance 0xBeta 110000000000000000000  # 110 ETH

# 3. Start roko serve (control plane for demo UI)
cargo run -p roko-cli -- serve

# 4. Open demo UI
cd demo/demo-app && npm run dev
```

### Agent Roles

**Agent Alpha — "Yield Scout"**
- System prompt: See [Agent Prompts](#agent-system-prompts) below
- Task: "Analyze yield opportunities for 500K USDC across Aave V3 and Uniswap V3 on this Ethereum fork. Research rates, compare options, execute the optimal strategy, and post your findings to the knowledge graph."
- Available tools: `chain.balance`, `chain.get_pool_info`, `chain.swap`, `chain.add_liquidity`, `chain.simulate_tx`, `chain.gas_estimate`, plus a bridge tool for `chain_postInsight` / `chain_searchInsights`
- Wallet: Pre-funded with 10 ETH + 500K USDC

**Agent Beta — "Risk Hedger"**
- Task: "Hedge a 100 ETH long position using Aave V3 borrows and Uniswap V3 LP on this Ethereum fork. Before doing your own research, check the knowledge graph for existing insights. Execute your strategy and post findings."
- Available tools: Same as Alpha
- Wallet: Pre-funded with 110 ETH

### The Knowledge Transfer Moments

There are three key transfer events in the DeFi demo. Each should trigger a
visible UI animation.

**Transfer 1: Rate Data (Alpha → Beta)**
- Alpha queries Aave V3 supply rates for USDC → posts insight with rates + utilization curve
- Beta needs Aave rates for borrow cost calculation → finds Alpha's insight via `chain_searchInsights`
- Beta confirms the insight → cross-agent confirmation record created
- **Savings:** Beta skips 2-3 `chain.balance` calls to Aave rate oracle contracts

**Transfer 2: Pool Selection (Alpha → Beta)**
- Alpha evaluates USDC/WETH pools across fee tiers (0.05%, 0.3%, 1%) → posts pool comparison
- Beta needs a pool for its LP hedge → finds Alpha's pool analysis
- **Savings:** Beta skips the pool comparison research, goes straight to best pool

**Transfer 3: Combined Strategy (Beta → Alpha)**
- Beta discovers a carry trade: borrow USDC at 3.8%, LP at 11.8% = 7.96% net carry
- Beta posts this as a STRATEGY insight
- Alpha picks it up during its final optimization pass → realizes the carry trade
  beats pure Aave supply by ~3.7%
- **Compounding moment:** Agent B's insight improves Agent A's strategy

## UI Layout

### Demo Tab: "Chain Intelligence"

```
┌─────────────────────────────────────────────────────────────────────────┐
│  [tabs...]  [9 Knowledge Transfer]  [10 Chain Intelligence]            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌───────────────────────────┐  ┌───────────────────────────┐          │
│  │  YIELD SCOUT (Alpha)      │  │  RISK HEDGER (Beta)       │          │
│  │  0x7a3b...f1e2             │  │  0x9c4d...a3b5            │          │
│  │                            │  │                           │          │
│  │  ░░ xterm.js ░░░░░░░░░░░  │  │  ░░ xterm.js ░░░░░░░░░░  │          │
│  │  ░░░░░░░░░░░░░░░░░░░░░░░  │  │  ░░░░░░░░░░░░░░░░░░░░░░  │          │
│  │  ░░░░░░░░░░░░░░░░░░░░░░░  │  │  ░░░░░░░░░░░░░░░░░░░░░░  │          │
│  └───────────────────────────┘  └───────────────────────────┘          │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                     KNOWLEDGE FLOW                                │  │
│  │                                                                   │  │
│  │  ┌──────────┐         ┌────────────────┐         ┌──────────┐    │  │
│  │  │  Alpha   │──post──▶│   On-Chain     │◀──query──│  Beta    │    │  │
│  │  │  ○ ○ ○   │◀─query──│   Knowledge    │──confirm─▶│  ○ ○    │    │  │
│  │  │  3 posts │         │   Graph        │          │  2 posts │    │  │
│  │  └──────────┘         │  ┌──────────┐  │         └──────────┘    │  │
│  │                       │  │ 5 active  │  │                        │  │
│  │                       │  │ 3 confirm │  │                        │  │
│  │                       │  └──────────┘  │                        │  │
│  │                       └────────────────┘                        │  │
│  │                                                                   │  │
│  │  ┌─ Insight Feed ─────────────────────────────────────────────┐  │  │
│  │  │ HEURISTIC  Alpha  "Aave V3 USDC: 4.21% APR at 78%..."    │  │  │
│  │  │ ✓ CONFIRM  Beta   confirmed Alpha's rate insight   0xc1d..│  │  │
│  │  │ STRATEGY   Alpha  "USDC/WETH 0.05% pool: best risk-adj.."│  │  │
│  │  │ CAUSAL     Beta   "3.84% borrow + 11.8% LP = 7.96%..."   │  │  │
│  │  │ ✓ CONFIRM  Alpha  confirmed Beta's carry trade insight    │  │  │
│  │  └───────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  ┌─────────────────────────┐  ┌─────────────────────────────────────┐  │
│  │  CHAIN ACTIVITY          │  │  LIVE POSITIONS                     │  │
│  │                          │  │                                     │  │
│  │  Block 19,847,223 (2s)   │  │  ALPHA                BETA         │  │
│  │  ┊                       │  │  ┌─────────────┐     ┌───────────┐ │  │
│  │  ├ 0xa3f.. pool query    │  │  │ USDC  282K  │     │ ETH   10  │ │  │
│  │  ├ 0xb7e.. insight post  │  │  │ WETH  55.2  │     │ USDC  90K │ │  │
│  │  ┊                       │  │  │ aUSDC 300K  │     │ aETH  100 │ │  │
│  │  Block 19,847,222        │  │  │             │     │ debt  90K │ │  │
│  │  ┊                       │  │  │ APR: 7.2%   │     │ HF: 2.31  │ │  │
│  │  ├ 0xc1d.. Aave deposit  │  │  └─────────────┘     └───────────┘ │  │
│  │  ├ 0xd2e.. insight conf  │  │                                     │  │
│  │  ┊                       │  │  ┌─────────────────────────────┐   │  │
│  │  Block 19,847,221        │  │  │ Shared LP: USDC/WETH 0.05% │   │  │
│  │  ┊                       │  │  │ Alpha: tokenId 847291      │   │  │
│  │  ├ 0xe4f.. swap          │  │  │ Beta:  tokenId 847294      │   │  │
│  │  └                       │  │  │ Range: [1700, 1900]        │   │  │
│  │                          │  │  └─────────────────────────────┘   │  │
│  └─────────────────────────┘  └─────────────────────────────────────┘  │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │  INSIGHTS: 5   CONFIRMS: 3   REUSE: 57%   SAVED: ~$0.47 / 8 calls │ │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### UI Components (New)

**1. Knowledge Flow Panel** — `KnowledgeFlowPanel.tsx`

The centerpiece visualization. Shows insights flowing between agents through
the on-chain knowledge graph.

**Data source:** WebSocket to `ws://mirage:8545/api/ws?insights=true&pheromones=true`

**WebSocket event format (from mirage-rs):**
```json
// Insight posted
{"channel": "insight", "data": {"type": "posted", "id": "insight:a1b2...", "kind": "heuristic", "content": "Aave V3 USDC...", "author": "agent-alpha", "createdAt": 1700000000}}

// Insight confirmed
{"channel": "insight", "data": {"type": "confirmed", "id": "insight:a1b2...", "by": "agent-beta", "at": 1700000010}}

// Insight state transition
{"channel": "insight", "data": {"type": "stateTransition", "id": "insight:a1b2...", "from": "Active", "to": "Confirmed", "at": 1700000010}}
```

**Rendering approach:** Canvas 2D (matches existing BarChart/CostChart pattern).
- Two agent nodes (circles) on left and right
- Central knowledge graph node (hexagon or rounded rect)
- Bezier curve edges connecting them
- On `posted` event: spawn a particle (small circle) that travels from agent → center, 1.2s animation
- On `confirmed` event: spawn a particle center → confirming agent + pulse the edge gold
- Counter badges on each node (posts count, confirms count)
- Central node shows total active insights + confirmation rate

**The "aha" animation:**
When a `confirmed` event arrives where the confirmer is different from the author:
1. Both edges light up simultaneously (gold pulse, 0.5s)
2. A brief label appears on the edge: "Knowledge reused"
3. The efficiency counter in the bottom bar increments
4. A subtle screen flash (the whole Knowledge Flow panel border pulses bone-gold for 0.3s)

**Insight Feed:** Below the graph, a scrolling list. Each row:
```
[KIND badge] [Agent dot + name] [Content preview, max 1 line] [tx hash, mono, 50% opacity]
```
Kind badges: HEURISTIC (blue), STRATEGY (green), CAUSAL (amber), WARNING (red)
Agent dots: Alpha = rose, Beta = sage
New entries slide in from the bottom with 0.3s ease-out

**2. Chain Activity Panel** — `ChainActivityPanel.tsx`

Mini block explorer showing live chain activity.

**Data source:** Poll mirage RPC `eth_blockNumber` + `eth_getBlockByNumber` every 2s
(matches block interval). Or subscribe to new block headers via WebSocket.

**Rendering:** Simple scrolling list, newest block at top.
Each block row:
```
Block 19,847,223  (2s ago)
├── 0xa3f..  pool query       [blue dot]
├── 0xb7e..  insight posted   [green dot]
└── 0xc1d..  Aave deposit     [rose dot]
```

Color coding:
- Blue: read-only calls (eth_call)
- Green: insight operations (chain_postInsight, chain_confirmInsight)
- Rose: state-changing DeFi operations (swap, deposit, borrow, LP)
- Bone: other transactions

**3. Live Positions Panel** — `LivePositionsPanel.tsx`

Two portfolio cards showing real-time balances and positions.

**Data source:** Poll every block (2s):
- `eth_getBalance(address)` → ETH balance
- `eth_call(USDC.balanceOf(address))` → USDC balance
- `eth_call(aToken.balanceOf(address))` → Aave deposits
- `eth_call(debtToken.balanceOf(address))` → Aave borrows

**Rendering:** Two `<Pane>` components side-by-side, each containing:
- Header: Agent name + wallet address (truncated)
- Token balances using `<PhosphorNumber>` (flash on change)
- Position summary: APR or Health Factor, highlighted
- Optional: shared LP positions shown in a connecting section between the two cards

**4. Efficiency Metrics Bar** — `EfficiencyBar.tsx`

Bottom bar. Single row of stats.

**Data source:** Computed from WebSocket insight events:
- `insights`: count of `posted` events
- `confirms`: count of `confirmed` events where `by !== author`
- `reuse_rate`: confirms / insights
- `calls_saved`: estimated from confirmed insights (each confirmation = N skipped RPC calls)
- `cost_saved`: calls_saved × estimated_cost_per_call

**Rendering:** Use existing `<Mosaic>` with `<MosaicCell>` components. 5 cells in a row.
Values use `<PhosphorNumber>` for flash-on-change effect.

## Agent System Prompts

### Alpha: Yield Scout

```toml
# roko agent config (passed via --agent-config or roko.toml)
[agent.alpha]
name = "yield-scout"
role = "yield-optimization"
model = "claude-sonnet-4-6"

[agent.alpha.system_prompt]
template = """
You are a DeFi yield optimization agent operating on a fork of Ethereum mainnet.

## Your Mission
Analyze yield opportunities for 500,000 USDC across Aave V3 and Uniswap V3.
Research rates, compare options, execute the optimal strategy.

## Available Protocols
- Aave V3 LendingPool: 0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2
- Uniswap V3 Router: 0xE592427A0AEce92De3Edee1F18E0157C05861564
- USDC: 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
- WETH: 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2

## Knowledge Graph Protocol
You have access to an on-chain knowledge graph. Use it:

1. BEFORE researching a topic, call chain_searchInsights with relevant tags
   to check if another agent has already researched it.
2. AFTER discovering a significant finding, call chain_postInsight to share it.
3. When you find an existing insight that matches your observations, call
   chain_confirmInsight to strengthen it.

Insight kinds: HEURISTIC (patterns), STRATEGY (action plans), CAUSAL (if-then),
WARNING (pitfalls). Always include relevant tags for discoverability.

## Workflow
1. Check knowledge graph for existing yield research
2. Query Aave V3 supply rates for USDC, DAI, USDT
3. Query Uniswap V3 pools: USDC/WETH at 0.05%, 0.3%, 1% fee tiers
4. Compare risk-adjusted returns
5. Post findings to knowledge graph
6. Execute optimal strategy
7. Post final strategy as STRATEGY insight
"""
```

### Beta: Risk Hedger

```toml
[agent.beta]
name = "risk-hedger"
role = "risk-management"
model = "claude-sonnet-4-6"

[agent.beta.system_prompt]
template = """
You are a DeFi risk management agent operating on a fork of Ethereum mainnet.

## Your Mission
Hedge a 100 ETH long position using Aave V3 borrows and Uniswap V3 LP positions.
Minimize directional exposure while maximizing carry yield.

## Available Protocols
(same as Alpha)

## Knowledge Graph Protocol
IMPORTANT: You MUST check the knowledge graph BEFORE doing your own research.
Another agent may have already researched rates, pools, or strategies that are
relevant to your task. This saves time and cost.

1. FIRST: call chain_searchInsights with tags like "aave", "rates", "usdc", "yield"
2. If you find relevant insights, USE THEM. Call chain_confirmInsight to validate.
3. Only do your own primary research for topics not covered by existing insights.
4. Post YOUR discoveries to the knowledge graph for other agents to use.

## Workflow
1. Query knowledge graph for Aave rates and Uniswap pool data
2. If found: confirm and use. If not: research independently.
3. Design hedge: deposit ETH as collateral, borrow stablecoins, LP for carry
4. Calculate health factor — NEVER go below 2.0
5. Execute strategy
6. Post hedge strategy + risk analysis as insights
"""
```

### Bridge Tool: `chain.post_insight`

The chain tools in `roko-chain/src/tools.rs` interact with the fork via JSON-RPC. But
`chain_postInsight` is a mirage-specific RPC method, not a standard eth_* call. The agent
needs a tool that wraps this RPC call.

**Option A: Add a tool to roko-chain**

```rust
// In crates/roko-chain/src/tools.rs, add:
ToolDef {
    name: "chain.post_insight".to_string(),
    description: "Post an insight to the on-chain knowledge graph".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "kind": { "type": "string", "enum": ["heuristic", "causalLink", "warning", "strategyFragment"] },
            "content": { "type": "string", "description": "The insight text" },
            "confidence": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
            "tags": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["kind", "content"]
    }),
}

// In crates/roko-cli/src/chain_handler.rs, add handler:
async fn handle_post_insight(&self, args: &serde_json::Value) -> ToolResult {
    let kind = args["kind"].as_str().unwrap_or("insight");
    let content = args["content"].as_str().ok_or("missing content")?;
    let confidence = args["confidence"].as_f64().unwrap_or(0.8);
    let tags: Vec<String> = args["tags"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    // JSON-RPC call to mirage
    let response = self.rpc_client.request("chain_postInsight", json!([{
        "author": self.agent_id,
        "kind": kind,
        "content": content,
        "confidence": confidence,
        "tags": tags,
    }])).await?;

    ToolResult::structured(serde_json::to_string(&response)?)
}
```

**Option B: Let agent use raw JSON-RPC via existing bash tool**

```bash
# Agent can call this via the bash tool:
curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_postInsight","params":[{
    "author":"yield-scout",
    "kind":"heuristic",
    "content":"Aave V3 USDC supply: 4.21% APR at 78% utilization",
    "tags":["aave","usdc","yield"]
  }],"id":1}'
```

Option A is cleaner but requires Rust changes. Option B works today but is ugly
in the terminal output. **Recommend Option A.**

Similarly, add `chain.search_insights` and `chain.confirm_insight` tools.

## Scenario Flow (Detailed Timeline)

### Phase 1: Mirage Boot + Wallet Setup (0:00 - 0:30)

The demo scenario's `run()` function starts by booting mirage and funding wallets.
This can be done in setup before the demo starts, or shown live.

**Terminal Alpha:**
```
$ mirage-rs --rpc-url https://eth.llamarpc.com --block-interval-ms 2000 --features chain
[mirage] Forking eth-mainnet at block 19,847,200
[mirage] Chain features: knowledge=true stigmergy=true hdc=true
[mirage] JSON-RPC + HTTP + WebSocket on 127.0.0.1:8545
[mirage] Auto-mining every 2000ms
[mirage] Ready.
```

**UI updates:**
- Chain Activity panel starts ticking: empty blocks every 2s
- Block counter starts incrementing
- Knowledge Flow panel shows empty graph (0 insights)
- Positions panel shows initial balances

### Phase 2: Alpha Yield Research (0:30 - 2:00)

Alpha's terminal shows the agent working:

```
[yield-scout] Starting yield analysis for 500K USDC...

[yield-scout] Checking knowledge graph for existing research...
→ chain.search_insights(tags=["aave", "yield", "usdc"])
  No existing insights found. Starting fresh research.

[yield-scout] Querying Aave V3 supply rates...
→ chain.balance(address="0xBcca60bB61934080951369a648Fb03DF4F96263C", token="USDC")
  aUSDC balance: 2,147,832,419.42 USDC (protocol deposits)

→ chain.get_pool_info(protocol="aave_v3", asset="USDC")
  Supply APR: 4.21% | Borrow APR: 5.83% | Utilization: 78.3%
  Optimal utilization: 80% | Rate slope1: 4% | Rate slope2: 75%

💡 Posting to knowledge graph...
→ chain.post_insight(
    kind="heuristic",
    content="Aave V3 USDC: 4.21% supply APR at 78.3% utilization.
    Rate model: slope1=4% under 80% util, slope2=75% above. Current deposits
    $2.15B. Rate will jump to ~6.1% if utilization exceeds 85%.",
    confidence=0.93,
    tags=["aave", "usdc", "supply-rate", "yield", "utilization"]
  )
  ✓ Posted insight ins_a7f3 (block 19,847,205, tx 0xc1d...)

[yield-scout] Querying Uniswap V3 pools...
→ chain.get_pool_info(protocol="uniswap_v3", pair="USDC/WETH", fee=500)
  Pool 0x88e6...  TVL: $183.2M | sqrtPriceX96: 1.41e27 | tick: -201820
  Estimated fee APR (24h volume): ~11.8%

→ chain.get_pool_info(protocol="uniswap_v3", pair="USDC/WETH", fee=3000)
  Pool 0x8ad5...  TVL: $412.7M | sqrtPriceX96: 1.41e27 | tick: -201820
  Estimated fee APR (24h volume): ~7.2%

→ chain.get_pool_info(protocol="uniswap_v3", pair="USDC/WETH", fee=10000)
  Pool 0x11b8...  TVL: $45.1M | Fee APR: ~14.3% (but high IL risk)

💡 Posting pool comparison...
→ chain.post_insight(
    kind="strategy",
    content="USDC/WETH Uni V3 pool comparison:
    - 0.05% fee: $183M TVL, ~11.8% APR, moderate IL. Best for active LP.
    - 0.3% fee:  $413M TVL, ~7.2% APR, lower IL. Best for passive LP.
    - 1% fee:    $45M TVL, ~14.3% APR, high IL. Only for narrow ranges.
    Recommendation: 0.05% fee tier for concentrated positions ±5% range.",
    confidence=0.87,
    tags=["uniswap", "pool", "usdc", "weth", "fee-tier", "comparison"]
  )
  ✓ Posted insight ins_b8e4 (block 19,847,209, tx 0xe4f...)
```

**UI updates during Phase 2:**
- Knowledge Flow: Two particles animate Alpha → Center (one per insight post)
- Insight Feed: Two cards appear (heuristic + strategy)
- Chain Activity: Blocks show eth_call (blue) + insight post (green) transactions
- Positions: Alpha still holding initial balances (no execution yet)
- Efficiency: Insights: 2, Confirms: 0, Reuse: 0%

### Phase 3: Beta Starts + Knowledge Pickup (1:30 - 3:00)

Beta starts ~60s after Alpha (staggered for clarity). The knowledge transfer
happens immediately.

```
[risk-hedger] Starting hedge analysis for 100 ETH long position...

[risk-hedger] Checking knowledge graph for existing research...
→ chain.search_insights(tags=["aave", "rates", "yield"])
  Found 1 insight from yield-scout (confidence: 0.93):
  "Aave V3 USDC: 4.21% supply APR at 78.3% utilization..."

  ✓ This is relevant to my borrow cost calculation.

→ chain.confirm_insight(id="ins_a7f3")
  ✓ Confirmed (block 19,847,212, tx 0xd2e...)

[risk-hedger] Using confirmed data — Aave USDC supply: 4.21%, utilization: 78.3%
[risk-hedger] Inferring borrow rate from rate model: ~5.83% variable

→ chain.search_insights(tags=["uniswap", "pool", "usdc", "weth"])
  Found 1 insight from yield-scout (confidence: 0.87):
  "USDC/WETH Uni V3 pool comparison: 0.05% fee best for concentrated..."

  ✓ Confirming — will use 0.05% fee tier for LP hedge.

→ chain.confirm_insight(id="ins_b8e4")
  ✓ Confirmed (block 19,847,213, tx 0xf3a...)

[risk-hedger] Skipped 4 pool queries and 1 rate query — using Alpha's data.
```

**UI updates — THE KEY MOMENT:**
- Knowledge Flow: Two particles animate Center → Beta (queries found results)
- Knowledge Flow: Two particles animate Beta → Center (confirmations)
- **AHA ANIMATION:** Both edges pulse gold simultaneously. Label: "Knowledge reused — 5 calls saved"
- Insight Feed: Two "CONFIRM" entries appear with green checkmarks
- Efficiency: Insights: 2, Confirms: 2, Reuse: 100%, Saved: 5 calls / ~$0.12

### Phase 4: Both Execute Strategies (2:30 - 4:30)

Both agents execute. The Chain Activity and Positions panels come alive.

**Alpha executes yield strategy:**
```
[yield-scout] Executing split strategy: 60% Aave, 40% Uni V3 LP

→ chain.approve(token="USDC", spender="aave_v3_pool", amount=300000e6)
  ✓ Approved

→ chain.simulate_tx(to="aave_v3_pool", data=<deposit_calldata>)
  ✓ Simulation passed — expected aUSDC: 300,000

→ chain.transfer(protocol="aave_v3", action="deposit", token="USDC", amount=300000)
  ✓ Deposited 300K USDC to Aave V3 (tx 0x...)

→ chain.swap(token_in="USDC", token_out="WETH", amount_in=100000e6, fee=500)
  ✓ Swapped 100K USDC → 55.2 WETH at $1,811.59/ETH

→ chain.add_liquidity(
    protocol="uniswap_v3", pair="USDC/WETH", fee=500,
    tick_lower=-202800, tick_upper=-200800,
    amount0=100000e6, amount1=55.2e18
  )
  ✓ LP position minted (tokenId: 847291, range: [$1,700, $1,900])

💡 Posting strategy insight...
→ chain.post_insight(
    kind="strategy",
    content="500K USDC optimal split: 60% Aave supply (4.21% safe yield),
    40% concentrated Uni V3 LP USDC/WETH 0.05% in [$1,700, $1,900] range
    (~11.8% fee APR). Blended APR: ~7.2%. IL risk: ~2.1% if ETH moves >5%.
    Gas overhead: ~$12 for setup, negligible for passive holding.",
    confidence=0.85,
    tags=["yield", "strategy", "split", "aave", "uniswap", "final"]
  )
```

**Beta executes hedge:**
```
[risk-hedger] Executing delta-neutral hedge...

→ chain.transfer(protocol="aave_v3", action="deposit", token="ETH", amount=100e18)
  ✓ Deposited 100 ETH as Aave V3 collateral
  Health factor: 4.12 | Max borrow (USDC): $182,500

→ chain.transfer(protocol="aave_v3", action="borrow", token="USDC", amount=90000e6)
  ✓ Borrowed 90K USDC against ETH collateral
  Health factor: 2.31 | Liquidation price: $987

→ chain.add_liquidity(
    protocol="uniswap_v3", pair="USDC/WETH", fee=500,
    tick_lower=-202800, tick_upper=-200800,
    amount0=90000e6, amount1=0
  )
  ✓ LP position minted (tokenId: 847294, single-sided USDC entry)

💡 Posting carry trade insight...
→ chain.post_insight(
    kind="causal",
    content="Carry trade: borrow USDC on Aave at 5.83% variable, LP in Uni V3
    USDC/WETH 0.05% for ~11.8% fee APR. Net carry: ~5.97%. Risk: health factor
    must stay above 2.0 to survive 15% ETH drawdown. At HF=2.31, liquidation
    triggers at ~$987/ETH (current ~$1,812). Buffer: 45.5% drawdown.",
    confidence=0.88,
    tags=["hedge", "carry-trade", "aave", "borrow", "uniswap", "risk", "health-factor"]
  )
```

**UI updates during Phase 4:**
- Chain Activity: Blocks dense with transactions (blue → rose → green)
- Positions: Numbers changing with every block
  - Alpha: USDC 100K → swap → WETH 55.2, aUSDC 300K, LP position
  - Beta: ETH 10 (100 deposited), debtUSDC 90K, LP position
- Knowledge Flow: More insight posts flowing in
- Insight Feed: Strategy and causal entries appearing

### Phase 5: Cross-Pollination (4:00 - 5:00)

Alpha picks up Beta's carry trade insight:

```
[yield-scout] Reviewing knowledge graph for new strategies...

→ chain.search_insights(tags=["yield", "strategy", "carry"])
  Found 1 new insight from risk-hedger (confidence: 0.88):
  "Carry trade: borrow USDC at 5.83%, LP at 11.8% = 5.97% net carry..."

  Analyzing: This carry trade yields 5.97% net vs my blended 7.2% —
  but the carry trade is on BORROWED capital, so effective return on
  equity is higher. For my 40% Uni allocation, the borrow-and-LP
  approach would yield: 11.8% - 5.83% = 5.97% on 200K USDC,
  vs 11.8% raw on my 100K USDC LP. Different risk profiles.

→ chain.confirm_insight(id="ins_c9f5")
  ✓ Confirmed

💡 Posting meta-insight...
→ chain.post_insight(
    kind="heuristic",
    content="For yield optimization above $200K: pure LP (11.8%) beats
    borrow-and-LP carry (5.97% net) on raw APR, but carry trade uses
    leverage — effective ROE depends on collateral ratio. Below 2.0 HF,
    carry trade risk/reward degrades sharply. Recommend pure LP for
    passive strategies, carry for active risk management.",
    confidence=0.91,
    tags=["yield", "meta-strategy", "carry-trade", "comparison", "heuristic"]
  )
```

**UI updates:**
- Knowledge Flow: Final cross-pollination animation
- Third "aha" moment: Alpha confirms Beta's insight AND posts a synthesis
- Efficiency: Knowledge reuse rate climbing

### Phase 6: Summary (5:00 - 5:30)

Both agents complete. The UI transitions to a summary view:

```
┌──────────────────────────────────────────────────────────┐
│  DEMO COMPLETE                                           │
│                                                          │
│  Knowledge Graph                    Efficiency           │
│  ├── Insights posted:      5        Calls saved:    8    │
│  ├── Cross-confirms:       3        Turns saved:    ~4   │
│  ├── Unique tags:         14        Cost saved:   $0.47  │
│  └── Knowledge reuse:    60%        Time saved:   ~45s   │
│                                                          │
│  Alpha (Yield Scout)                Beta (Risk Hedger)   │
│  ├── Final APR: 7.2% blended       ├── Health Factor: 2.31│
│  ├── Positions: Aave + Uni LP      ├── Carry: 5.97% net │
│  ├── Insights posted: 3            ├── Insights posted: 2│
│  └── Cost: $1.42                   └── Cost: $0.98       │
│                                                          │
│  On-chain record: 5 insights, 3 confirmations, 0 challenges│
│  Chain blocks produced: ~150 | Total gas: 4.8M           │
└──────────────────────────────────────────────────────────┘
```

## What Needs to Exist in Roko (DeFi Demo)

### Already Built — verified working

| Component | Where | Implementation detail |
|-----------|-------|-----------------------|
| mirage-rs fork | `apps/mirage-rs/` | `--features chain` enables knowledge graph RPC. Startup writes status to `/tmp/mirage-{port}-status.json`. |
| chain_postInsight RPC | `apps/mirage-rs/src/chain_rpc.rs:369-433` | Posts to `chain.knowledge` store, broadcasts WebSocket event. Gated by `toggles.knowledge`. |
| chain_searchInsights RPC | `apps/mirage-rs/src/chain_rpc.rs:463-514` | HDC projection + top-K similarity search. Returns matching insights. |
| chain_confirmInsight RPC | `apps/mirage-rs/src/chain_rpc.rs:532-572` | State machine transition Active → Confirmed. Broadcasts event. |
| WebSocket streaming | `apps/mirage-rs/src/http_api/ws.rs` | `/api/ws?insights=true&pheromones=true`. Channels: insight, pheromone, agent, prediction. Ping/pong 30s/90s. |
| 14 chain tool defs | `crates/roko-chain/src/tools.rs` | Registered in ROKO_BUILTIN_TOOLS with full JSON schemas |
| Chain tool handlers | `crates/roko-cli/src/chain_handler.rs` | `chain.balance` → real `eth_call`. `chain.swap` → real Uniswap V3 `exactInputSingle` ABI encoding. `chain.get_pool_info` → real `slot0()` + `liquidity()` queries. |
| Witness engine | `crates/roko-chain/src/witness.rs:53-79` | `witness_on_chain()` → sign + submit tx with marker bytes → wait for receipt |
| HTTP REST API | `apps/mirage-rs/src/http_api/mod.rs` | 50+ routes: `/api/health`, `/api/knowledge/entries`, `/api/pheromones`, `/api/stats`, etc. |
| Agent dispatch | `crates/roko-agent/src/dispatcher/mod.rs` | Tool loop works with registered handlers |
| Demo app 2-pane layout | `demo/demo-app/src/lib/scenarios.ts` | Race scenario provides the pattern |
| Canvas charts | `demo/demo-app/src/components/Charts/` | BarChart, CostChart, ParetoChart — all Canvas 2D |
| Three.js viz | `demo/demo-app/src/components/WorkflowConstellation.tsx` | 3D node graph with particles (can repurpose) |

### Needs wiring

| What | Gap | Fix | Effort |
|------|-----|-----|--------|
| **chain.post_insight tool** | Agents have DeFi tools but no tool to call `chain_postInsight` RPC. The RPC method exists in mirage but isn't exposed as an agent tool. | Add `chain.post_insight`, `chain.search_insights`, `chain.confirm_insight` to `tools.rs` + handlers in `chain_handler.rs` that make JSON-RPC calls to mirage. | Medium |
| **ChainClient → fork RPC** | `chain_handler.rs` needs a live `ChainClient` connected to the fork's RPC URL. Currently `AlloyChainClient` may not be instantiated at runtime. | Configure `roko.toml` with `[chain] rpc_url = "http://127.0.0.1:8545"` and wire `AlloyChainClient::new(rpc_url)` in CLI startup. | Small-Medium |
| **Two concurrent agents** | Demo needs two `roko run` invocations with different agent configs running simultaneously. Current scenario `run()` calls `showCmd` sequentially. | For staggered start: run Alpha first, then Beta after a delay (sequential `showCmd` in two panes). For truly concurrent: use `Promise.all` with two `showCmd` calls. Staggered is better for the demo narrative. | Small |
| **useChain hook** | `demo/demo-app/src/hooks/useChain.ts` is a stub returning "not connected". | Replace with real WebSocket connection to `ws://mirage:8545/api/ws?insights=true`. Parse incoming JSON events. Return structured state. | Medium |
| **Wallet prefunding** | Agent wallets need ETH + USDC before agents can transact. | Add a setup step in the scenario `run()` that calls `cast rpc anvil_setBalance` and token transfer from an unlocked whale. Or add mirage startup config for initial balances. | Small |

### Needs building

| Component | Description | Approach | Effort |
|-----------|-------------|----------|--------|
| **KnowledgeFlowPanel.tsx** | Animated graph: two agent nodes ↔ central chain node. Particles on edges. Insight feed below. | Canvas 2D (match existing chart pattern). Subscribe to mirage WS. Animate on `posted`/`confirmed` events. | Large |
| **ChainActivityPanel.tsx** | Mini block explorer. Scrolling block list with color-coded transactions. | Poll `eth_blockNumber` + `eth_getBlockByNumber` every 2s. Simple DOM list with CSS transitions. | Medium |
| **LivePositionsPanel.tsx** | Two portfolio cards with live token balances, positions, APR, health factor. | Poll mirage RPC per-block. Use PhosphorNumber for animated values. Two Pane components. | Medium |
| **EfficiencyBar.tsx** | Bottom metrics bar. Insights, confirms, reuse rate, cost saved. | Mosaic + MosaicCell pattern. Computed from WS event counters. | Small |
| **Demo scenario (scenarios.ts)** | New scenario entry with 2-pane layout, phased execution, panel integration. | Follow Race scenario pattern. Stagger Alpha (full run) then Beta (with knowledge). | Medium |
| **Mirage startup script** | Shell script to boot mirage, fund wallets, verify readiness. | Bash script using `cast` + `curl`. Check `/tmp/mirage-{port}-status.json` for ready state. | Small |

### Might be broken — verify before building

| Concern | Check | Command |
|---------|-------|---------|
| Alloy compilation | Needs rustc 1.91+ | `cargo build -p roko-chain --features alloy_impl` |
| Mirage chain features | Feature flag may not be default | `cargo build -p mirage-rs --features chain` |
| Uniswap V3 ABI in chain_handler | Swap handler encodes `exactInputSingle` — verify it matches current router | Read `chain_handler.rs` swap handler, compare ABI selector to Uniswap docs |
| Aave V3 interactions | Chain tools may not have Aave-specific ABI encoding | Check if `chain.transfer` with protocol="aave_v3" is handled or needs new code |
| Concurrent nonce management | Two wallets = no conflict, but verify mirage handles concurrent tx submission | Submit 2 txs from different wallets in same block via `cast send` |
| WebSocket backpressure | 2s blocks + multiple tx per block = ~30 events/min. Well within 256 channel capacity. | Connect wscat, watch for `{"type":"lagged"}` messages |

---

# Implementation Priority

Build Demo A first. It exercises the core value prop (knowledge sharing between agents)
with minimal infrastructure requirements. Demo B adds the chain layer on top.

## Phase 1: Demo A (Knowledge Transfer — no chain)

1. Verify distillation pipeline works end-to-end:
   `roko run` → episodes → `roko learn distill` → knowledge entries → playbook injection
2. Add knowledge panel React component
3. Add comparison metrics bar
4. Build scenario in scenarios.ts
5. Test with two sequential `roko run` commands
6. Polish timing and animations

**Estimated effort: 3-5 days**

## Phase 2: Demo B (Chain Intelligence — DeFi)

1. Build + verify mirage-rs with `--features chain`
2. Add insight bridge tools (post, search, confirm)
3. Wire ChainClient to fork RPC
4. Build wallet funding script
5. Test agent flow manually (two terminals)
6. Build KnowledgeFlowPanel (the centerpiece)
7. Build ChainActivityPanel, LivePositionsPanel, EfficiencyBar
8. Build scenario in scenarios.ts
9. Polish animations, especially the "aha" moment
10. Test end-to-end with live LLMs

**Estimated effort: 8-12 days**

## Phase 3: Polish

1. Record backup video in case live demo fails
2. Add error recovery (agent timeout, mirage restart, WS disconnect)
3. Add "demo mode" flag that pre-seeds some knowledge to guarantee the transfer moment
4. Tune agent prompts to reliably produce insight posts

---

# Open Questions

1. **Shared workspace vs separate workspaces (Demo A)?** Copying `.roko/neuro/` between dirs
   works but is hacky. Better option: point both agents at the same `roko serve` instance
   which manages a single knowledge store. Does the serve control plane support this?

2. **Staggered vs concurrent (Demo B)?** Staggered is better for narrative (clearly see
   Alpha finish → distill → Beta start with knowledge). Concurrent is more impressive
   but the knowledge transfer moment is harder to spot. **Recommend: staggered with ~60s delay.**

3. **Which block to fork?** Pin a specific block for reproducibility, or fork latest for
   freshness? **Recommend: pin a block, document the rates at that block, so the demo
   narrative matches the actual numbers.**

4. **Fallback if agent doesn't post insights?** The prompt says "post to knowledge graph"
   but the LLM might not comply. **Mitigation: pre-seed 1-2 insights before Beta starts
   as a safety net. If Alpha actually posts, great. If not, the pre-seeded ones work.**

5. **Cost per demo run?** With Sonnet for both agents, estimate ~$2-5 per run for Demo A,
   ~$5-15 for Demo B (more tool calls, more turns). With Haiku, ~10x cheaper but less
   impressive reasoning in terminal output.
