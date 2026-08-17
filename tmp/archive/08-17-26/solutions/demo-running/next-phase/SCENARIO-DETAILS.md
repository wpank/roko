# Scenario Details — Full Spec

Each scenario is specified with enough detail to implement directly.

---

## Scenario 1: Pipeline

### Identity
```
id: "pipeline"
title: "Pipeline"
subtitle: "Idea → code in one shot"
category: "pipeline"
accent: "rose"
icon: "pipeline"
panes: 1
labels: ["agent"]
durationHint: "~90s"
```

### Commands
```typescript
commands: [
  {
    id: "generate",
    command: 'roko prd pipeline "Build a Rust CLI that converts temperatures between Celsius and Fahrenheit"',
    description: "Generate PRD + implementation plan from a single idea",
    timeout: 90_000,
  },
  {
    id: "execute",
    command: "roko plan run .roko/plans --max-retries 1",
    description: "Execute plan: agents write code, gates validate",
    timeout: 180_000,
  },
  {
    id: "results",
    command: "roko status",
    description: "View results: signals, episodes, artifacts",
    timeout: 10_000,
  },
]
```

### Sidebar Layout
```
┌─────────────────────────┐
│ PIPELINE                │
│ ┌─────┐ ┌─────┐ ┌─────┐│
│ │ PRD │→│PLAN │→│ RUN ││
│ │  ●  │ │  ○  │ │  ○  ││
│ └─────┘ └─────┘ └─────┘│
├─────────────────────────┤
│ MODEL    claude-haiku   │
│ COST     $0.04          │
│ TOKENS   12,340         │
│ TIME     47s             │
├─────────────────────────┤
│ GATES                   │
│ ✔ compile               │
│ ✔ clippy                │
│ ○ test                  │
├─────────────────────────┤
│ TASKS                   │
│ ✔ scaffold project      │
│ ● implement converter   │
│ ○ add CLI args          │
│ ○ write tests           │
└─────────────────────────┘
```

### Prompt Variants
The presenter can rotate prompts. All are designed to be small, fast, and gate-friendly:
1. "Build a Rust CLI that converts temperatures between Celsius and Fahrenheit"
2. "Build a Rust library that validates email addresses"
3. "Build a Rust CLI that generates random passwords with configurable length"
4. "Build a Rust CLI that counts words and lines in a file"

### Timing Budget
- `prd pipeline`: 30-60s (1 LLM call for PRD, 1 for plan generation)
- `plan run`: 30-120s (1-4 agent tasks, each 15-30s)
- `status`: <1s
- **Total: 60-180s**

### Error Recovery
- If `prd pipeline` fails: show error in sidebar, presenter can retry with different prompt
- If `plan run` fails: gates show red, sidebar shows which task failed, presenter can click "results" to see partial output

---

## Scenario 2: Showdown

### Identity
```
id: "showdown"
title: "Showdown"
subtitle: "Same task, different providers"
category: "comparison"
accent: "teal"
icon: "race"
panes: 2
labels: ["provider-a", "provider-b"]
durationHint: "~60s"
agents: [
  { name: "Provider A", role: "challenger", model: "anthropic" },
  { name: "Provider B", role: "challenger", model: "openai" },
]
```

### Commands
```typescript
commands: [
  {
    id: "race",
    command: 'roko run "Build a Rust function that checks if a string is a palindrome"',
    description: "Same prompt, two providers — who's faster, cheaper, better?",
    timeout: 120_000,
    target: { panes: [0, 1] },  // runs in both simultaneously
  },
  {
    id: "compare",
    command: "roko learn efficiency",
    description: "Compare: cost, tokens, quality, speed",
    timeout: 10_000,
    target: "all",
  },
]
```

### Sidebar Layout
```
┌─────────────────────────┐
│ SHOWDOWN                │
│                         │
│ ┌──────────┬──────────┐ │
│ │PROVIDER A│PROVIDER B│ │
│ │anthropic │ openai   │ │
│ ├──────────┼──────────┤ │
│ │ TIME  42s│ TIME  38s│ │
│ │ COST $0.08│COST $0.05│ │
│ │ TOKENS 15K│TOKENS 12K│ │
│ │ GATES  ✔ │ GATES  ✔ │ │
│ └──────────┴──────────┘ │
│                         │
│ WINNER: Provider B      │
│ 10% faster, 37% cheaper │
├─────────────────────────┤
│ PROMPT                  │
│ "Check if a string is a │
│  palindrome"            │
└─────────────────────────┘
```

### Provider Selection
Providers are selected from the ConfigWidget. Defaults:
- Pane 0: `anthropic` (Claude)
- Pane 1: `openai` (GPT)

Alternative pairs: `anthropic` vs `gemini`, `openai` vs `zhipu`, `moonshot` vs `openai`

Each pane gets its own workspace via `ctx.createWorkspace()`.

### How Winner Is Determined
1. **Both pass gates**: Winner = lower cost. Tie-break = faster time.
2. **One passes gates**: That one wins regardless of cost/speed.
3. **Neither passes**: No winner. Sidebar shows "Draw — both failed gates."

### Prompt Variants
1. "Build a Rust function that checks if a string is a palindrome"
2. "Build a Rust function that computes the nth Fibonacci number"
3. "Build a Rust function that reverses a linked list"
4. "Build a Rust function that finds the longest common substring"

---

## Scenario 3: Swarm

### Identity
```
id: "swarm"
title: "Swarm"
subtitle: "4 DeFi agents, 1 rate"
category: "chain"
accent: "amber"
icon: "chain"
panes: 4
labels: ["lending-scout", "staking-scout", "aggregator", "validator"]
mirageBar: true
durationHint: "~120s"
agents: [
  { name: "Lending Scout", role: "scout", protocol: "Aave + Compound" },
  { name: "Staking Scout", role: "scout", protocol: "Lido + Rocket Pool" },
  { name: "Rate Aggregator", role: "aggregator" },
  { name: "Validator", role: "validator" },
]
```

### Commands
```typescript
commands: [
  {
    id: "health",
    command: 'curl -sf localhost:8545/health && echo "chain ready"',
    description: "Verify Anvil fork is live",
    timeout: 5_000,
    target: { pane: 0 },
  },
  {
    id: "launch",
    command: 'roko run "<agent-prompt>"',  // different prompt per pane
    description: "Launch all 4 agents simultaneously",
    timeout: 180_000,
    target: "all",  // each pane gets its role-specific prompt
  },
  {
    id: "results",
    command: "roko learn all",
    description: "View computed ISFR and agent metrics",
    timeout: 10_000,
    target: { pane: 0 },
  },
]
```

### Agent Prompts
Each pane gets a role-specific prompt. Prompts are SHORT (under 100 chars for display, full prompt injected via system prompt template):

**Pane 0 — Lending Scout**:
```
"Scout lending rates from Aave V3 and Compound V3 on local fork. Write rates to knowledge store."
```

**Pane 1 — Staking Scout**:
```
"Scout ETH staking yields from Lido and Rocket Pool on local fork. Write yields to knowledge store."
```

**Pane 2 — Rate Aggregator**:
```
"Read scout findings from knowledge store. Compute volume-weighted ISFR. Write result to knowledge store."
```

**Pane 3 — Validator**:
```
"Validate the computed ISFR against historical bounds. Flag anomalies. Sign off if valid."
```

### Sidebar Layout
```
┌─────────────────────────┐
│ SWARM                   │
│                         │
│ ┌─────────────────────┐ │
│ │ LENDING    scanning  │ │
│ │ STAKING    scanning  │ │
│ │ AGGREGATOR waiting   │ │
│ │ VALIDATOR  waiting   │ │
│ └─────────────────────┘ │
├─────────────────────────┤
│ RATES DISCOVERED        │
│ Aave V3 USDC:   3.2%   │
│ Compound V3:    2.8%   │
│ Lido stETH:     3.8%   │
│ Rocket Pool:    3.5%   │
├─────────────────────────┤
│ COMPUTED ISFR           │
│ ━━━━━━━━━━━━━ pending   │
├─────────────────────────┤
│ CHAIN                   │
│ Block: 21,234,567       │
│ Chain ID: 31337         │
│ Status: ● connected     │
├─────────────────────────┤
│ KNOWLEDGE               │
│ Entries: 0 → 12         │
│ Agents: 4 active        │
└─────────────────────────┘
```

### Chain Configuration
Contract addresses come from `roko.toml` `[chain]` section, not hardcoded in prompts:
```toml
[chain]
rpc_url = "http://127.0.0.1:8545"
chain_id = 31337
# Agent prompts reference these via system prompt template
[chain.contracts]
aave_v3_pool = "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"
compound_v3_usdc = "0xc3d688B66703497DAA19211EEdff47f25384cdc3"
lido_steth = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84"
rocketpool_reth = "0xae78736Cd615f374D3085123A210448E74Fc6393"
```

### Pre-Conditions
- mirage-rs running and healthy at localhost:8545
- `cast` available on PATH (for chain queries inside agent tools)
- Wallet funded (pre-warmed, no explicit funding step)

---

## Scenario 4: Memory

### Identity
```
id: "memory"
title: "Memory"
subtitle: "Learn once, remember forever"
category: "learning"
accent: "emerald"
icon: "knowledge"
panes: 2
labels: ["cold-start", "with-knowledge"]
durationHint: "~90s"
agents: [
  { name: "Agent Alpha", role: "builder", note: "no prior knowledge" },
  { name: "Agent Beta", role: "builder", note: "inherits Alpha's knowledge" },
]
```

### Commands
```typescript
commands: [
  {
    id: "cold-build",
    command: 'roko run "Build a Rust CLI that parses CSV files and outputs JSON"',
    description: "Agent Alpha builds from scratch (cold start)",
    timeout: 120_000,
    target: { pane: 0 },
  },
  {
    id: "transfer",
    command: "transfer-knowledge",  // special: not a CLI command, handled internally
    description: "Copy Alpha's knowledge to Beta's workspace",
    timeout: 10_000,
  },
  {
    id: "warm-build",
    command: 'roko run "Build a Rust CLI that parses YAML files and outputs JSON"',
    description: "Agent Beta builds with Alpha's knowledge (warm start)",
    timeout: 120_000,
    target: { pane: 1 },
  },
  {
    id: "compare",
    command: "roko learn efficiency",
    description: "Compare: did knowledge make Beta faster/cheaper?",
    timeout: 10_000,
    target: "all",
  },
]
```

### Sidebar Layout
```
┌─────────────────────────┐
│ MEMORY                  │
│                         │
│ ALPHA (cold)  BETA (warm)│
│ ┌──────────┬──────────┐ │
│ │ TIME  72s│ TIME  --s│ │
│ │ COST $0.09│COST  -- │ │
│ │ TOKENS 18K│TOKENS -- │ │
│ │ GATES  ✔ │ GATES  ○ │ │
│ └──────────┴──────────┘ │
├─────────────────────────┤
│ KNOWLEDGE               │
│ ▓▓▓▓▓▓▓▓░░░░ 12 entries │
│ ↓ transferred ↓         │
│ ▓▓▓▓▓▓▓▓░░░░ 12 entries │
├─────────────────────────┤
│ DELTA                   │
│ Time:   -38% ↓          │
│ Cost:   -42% ↓          │
│ Tokens: -35% ↓          │
│ Quality: same           │
└─────────────────────────┘
```

### Knowledge Transfer Mechanism
The "transfer" step copies:
```bash
cp -r ${alphaDir}/.roko/neuro/ ${betaDir}/.roko/neuro/
cp -r ${alphaDir}/.roko/learn/ ${betaDir}/.roko/learn/
```

This runs via `execCmd` (not `showCmd`) but the sidebar shows a "Transferring knowledge..." animation with a progress indicator. The command list shows it as a step with a custom icon (arrows).

### Prompt Pairing
The two prompts must be related (so knowledge transfers) but different (so it's not just caching):
- **Pair A**: CSV parser → YAML parser (file parsing patterns transfer)
- **Pair B**: HTTP client → WebSocket client (networking patterns transfer)
- **Pair C**: JSON validator → TOML validator (schema validation patterns transfer)

### What Makes This Demo Work
The "delta" panel at the bottom is the punchline. If Beta is measurably faster/cheaper (which it should be, since the knowledge store provides context that reduces exploration), the audience sees concrete numbers proving learning works.

If the delta is negligible or negative (Beta was slower), the sidebar should still show the comparison honestly. The demo is about capability, not guaranteed improvement.

---

## Scenario 5: Oracle

### Identity
```
id: "oracle"
title: "Oracle"
subtitle: "DeFi data meets agent intelligence"
category: "chain"
accent: "violet"
icon: "chain"
panes: 2
labels: ["data-agent", "strategy-agent"]
mirageBar: true
durationHint: "~90s"
agents: [
  { name: "Data Agent", role: "collector", protocol: "DeFi" },
  { name: "Strategy Agent", role: "analyst" },
]
```

### Commands
```typescript
commands: [
  {
    id: "chain-check",
    command: 'curl -sf -X POST localhost:8545 -H "Content-Type:application/json" -d \'{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}\' | jq -r .result',
    description: "Check chain connection and current block",
    timeout: 5_000,
    target: { pane: 0 },
  },
  {
    id: "collect",
    command: 'roko run "Query Aave V3 and Compound lending rates on the local Anvil fork. Write a structured analysis to the knowledge store."',
    description: "Data Agent: collect on-chain DeFi rates",
    timeout: 120_000,
    target: { pane: 0 },
  },
  {
    id: "analyze",
    command: 'roko run "Read the DeFi rate analysis from the knowledge store. Recommend optimal USDC allocation across protocols for maximum yield."',
    description: "Strategy Agent: analyze data and recommend allocation",
    timeout: 120_000,
    target: { pane: 1 },
  },
]
```

### Sidebar Layout
```
┌─────────────────────────┐
│ ORACLE                  │
│                         │
│ ● DATA AGENT            │
│   scanning protocols... │
│                         │
│ ○ STRATEGY AGENT        │
│   waiting for data...   │
├─────────────────────────┤
│ RATES FOUND             │
│ Aave V3 USDC:  3.21%   │
│ Compound V3:   2.84%   │
│ Aave V3 DAI:   3.05%   │
├─────────────────────────┤
│ RECOMMENDATION          │
│ ━━━━━━━━━━━━━ pending   │
├─────────────────────────┤
│ CHAIN                   │
│ Block: 21,234,567       │
│ Chain: Anvil (31337)    │
│ Status: ● connected     │
├─────────────────────────┤
│ KNOWLEDGE FLOW          │
│ Data Agent → neuro/     │
│ neuro/ → Strategy Agent │
│ Entries: 0              │
└─────────────────────────┘
```

### Narrative Arc
1. **Chain check**: Quick health check. Block number proves it's a real fork, not a mock. <5s.
2. **Data collection**: Agent queries on-chain state via `cast` or direct RPC calls. Writes structured findings to `.roko/neuro/`. The audience sees the agent reading smart contracts in real time.
3. **Strategy analysis**: Second agent reads the first agent's findings and produces an actionable recommendation. This shows agent-to-agent communication via the knowledge store.

The story: "Agent 1 collected data. Agent 2 used that data to make a decision. Both worked autonomously. The data is verifiable on-chain."

### Pre-Conditions
Same as Swarm:
- mirage-rs running at localhost:8545
- `cast` on PATH
- Pre-warmed wallet (no funding step)

---

## Timing Summary

| Scenario | Steps | Best Case | Worst Case | Live Demo Target |
|---|---|---|---|---|
| Pipeline | 3 | 60s | 180s | <2 min |
| Showdown | 2 | 45s | 120s | <90s |
| Swarm | 3 | 90s | 180s | <3 min |
| Memory | 4 | 120s | 240s | <3 min |
| Oracle | 3 | 60s | 180s | <2 min |

**Total for all 5**: 6-12 minutes. Feasible for a 15-30 minute talk with commentary between each.

---

## Migration Path

### Phase 1: Implement new scenarios alongside old ones
- Add 5 new scenario files in `scenario-runners/`
- Add to `allScenarios` array
- Old scenarios remain accessible but moved to end of tab bar
- Test new scenarios work

### Phase 2: Remove old scenarios
- Delete 14 old scenario files
- Update `index.ts`
- Remove unused helpers and module-level state
- Clean up dead imports

### Phase 3: Sidebar redesign
- Implement scenario-aware sidebar panels
- Add winner logic for Showdown
- Add delta computation for Memory
- Add agent cards for Swarm/Oracle

### Phase 4: Polish
- "Run All" auto-advance
- Skip animations toggle
- Demo analytics
- `dev.sh demo` one-shot start command
