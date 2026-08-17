# Demo Redesign v2 — From Scratch

**Date**: 2026-05-04
**State**: Design doc, nothing implemented

---

## What the demos must prove

The pitch thesis (01-thesis.md) names four primitives the demo must prove live:

1. **Cost prediction** — 10-30x cost reduction via stacked mechanisms (prompt caching, tier routing, gate pre-screening). The Princeton HAL 50x number is a cited claim; the demo needs to show roko's cascade router delivering measurable savings vs. naive dispatch.
2. **Shared knowledge** — "The thousandth agent joins smarter than the first." Knowledge compounds across runs. An agent with access to prior knowledge performs measurably better than one without.
3. **Identity + coordination** — Multiple agents with distinct roles collaborate to produce a result no single agent could. The ISFR pipeline is the canonical example: scouts, aggregators, validators each have a specialty.
4. **Durability** — Work persists. Gate results, episode logs, knowledge entries, efficiency metrics — the system remembers what happened and uses it.

The audience is **technical and knows the space** — they've used Cursor, Devin, Claude Code. The demo doesn't need to explain "what is an AI agent." It needs to show what roko does *differently*: coordination, cost control, knowledge compounding, verifiable on-chain execution.

The tagline from the pitch: **"The model is the same. The system is the variable."**

Every demo should make this tangible: same model, better outcomes because the *system* around the model is smarter.

---

## Why the current demos fail at this

The 14 current scenarios don't prove any of these four primitives convincingly:

- **Cost prediction**: The "race" and "provider-race" scenarios run the same prompt on different providers, but there's no cascade routing visible. The audience sees two terminals, not a cost comparison. `roko learn efficiency` dumps raw JSONL that nobody can parse visually.
- **Shared knowledge**: "knowledge-transfer" copies `.roko/neuro` between workspaces, but the audience never sees *what* was learned or *how* it helped. The metrics (if they even differ) aren't shown in a way that tells a story.
- **Identity + coordination**: "isfr-agents" has 8 panes but the agents don't visibly coordinate — they all run independently and the audience can't tell if they're talking to each other.
- **Durability**: No scenario shows persistence. Every run starts cold. There's no "look, roko remembers what happened last time."

Beyond the thesis problems, the demos also have execution problems:
- They take 5-30 minutes each (audience is gone after 2)
- They break 50% of the time (slug mismatches, API key issues, workspace conflicts)
- They require 8-12 clicks per scenario
- The terminal output is the *only* visual — there's no dashboard, no chart, no comparison widget

---

## Design principles

1. **Prove the thesis, not the features.** Each demo maps to one of the four primitives. If a feature doesn't serve a thesis primitive, it doesn't get a demo.

2. **The sidebar IS the demo.** Terminal output is supporting evidence. The sidebar panel should tell the story — numbers updating live, comparison bars animating, knowledge meters filling. The audience watches the sidebar; the terminal is for credibility ("look, that's a real shell").

3. **One click, one result.** Each scenario has at most 3 commands, but the first command should produce the punchline. Step 2-3 are "let's look closer." The presenter should be able to click once, talk for 60 seconds while it runs, and then have something to point at.

4. **Under 2 minutes per scenario.** Real LLM calls, but small prompts, fast models, and streaming output so there's always motion. If a model call takes >30s, the sidebar must be showing live progress.

5. **No hidden fragility.** No slugs, no hardcoded paths, no module-level singletons, no shared workspaces, no external services that might be down (except mirage-rs for chain demos, pre-warmed). Each scenario is fully isolated and always resetable.

6. **Show the delta, not the feature.** Don't show "roko has a knowledge store." Show "Agent B was 40% faster because of Agent A's knowledge." Don't show "roko supports multiple providers." Show "same prompt, Anthropic: $0.12 / OpenAI: $0.04 / routed: $0.02."

---

## The 5 Scenarios

### 1. "Cost" — The System Is the Variable

**Thesis primitive**: Cost prediction
**One-line pitch**: Same model, same task — roko's routing cuts cost 3-5x.
**What makes it click**: The audience sees three numbers side by side. That's it.

**Layout**: 3 columns (panes or virtual), no terminals needed for 2 of them

```
┌─────────────────┬─────────────────┬─────────────────┐
│   NAIVE          │   CASCADE        │   ΔELTA         │
│                  │                  │                 │
│   $0.14          │   $0.03          │   -78%          │
│   18,200 tok     │   4,100 tok      │   -77%          │
│   42s            │   11s            │   -73%          │
│   ✔ gates pass   │   ✔ gates pass   │   same quality  │
│                  │                  │                 │
│ claude-sonnet    │ haiku→sonnet     │   3 tiers used  │
│ (single model)   │ (cascade routed) │   2 cached      │
└─────────────────┴─────────────────┴─────────────────┘
```

**How it works**:
- Two panes (left=naive, right=cascade), plus a comparison panel (right sidebar or third column)
- **Same prompt** dispatched twice: once with `--no-cascade` (force a single model), once normally (cascade router picks tiers)
- Prompt: something small and gate-friendly — `"Build a Rust function that checks if a number is prime"`
- Both run in independent workspaces
- The sidebar live-updates cost, tokens, time as the commands execute
- When both complete, the delta column appears with percentage savings highlighted

**Commands (2)**:
| # | Command | Target | What |
|---|---|---|---|
| 1 | Run both | panes 0+1 | Naive dispatch vs cascade routing, same prompt, simultaneously |
| 2 | Compare | sidebar | `roko learn efficiency` in both → delta panel appears |

**Why this works for the pitch**: The thesis claims 10-30x cost reduction from stacked mechanisms. This demo shows it live with real numbers. The audience doesn't need to understand *how* cascade routing works — they see the cost go from $0.14 to $0.03.

**CLI changes needed**:
- `--no-cascade` flag on `roko run` (or `--single-tier`) to force naive single-model dispatch
- Alternatively, `--provider <X> --model <Y>` already works for this — just pin to one expensive model on the left

**Key decisions**:
- The "naive" side should use a mid-tier model (sonnet or gpt-4o) at full price
- The "cascade" side should use the cascade router, which will try haiku first and only escalate if gates fail
- Prompt must be small enough that haiku can handle it → the cascade router wins by *not* escalating
- If both sides use the same model (because the cascade router picks sonnet anyway), the cost savings come from prompt caching and token optimization, which are still visible

---

### 2. "Pipeline" — Idea to Working Code

**Thesis primitive**: Durability (system persists work, validates, learns from each run)
**One-line pitch**: Describe what you want. Get working code back, validated.
**What makes it click**: The pipeline visualization fills in as each stage completes.

**Layout**: 1 terminal + sidebar pipeline widget

```
┌────────────────────────────────────┬──────────────────┐
│                                    │ PIPELINE          │
│   [terminal: agent output          │                  │
│    streaming as it works]          │  ● IDEA          │
│                                    │  ● PRD           │
│                                    │  ● PLAN          │
│                                    │  ◐ BUILD (2/4)   │
│                                    │  ○ VALIDATE      │
│                                    │                  │
│                                    ├──────────────────┤
│                                    │ GATES            │
│                                    │ ✔ compile        │
│                                    │ ✔ clippy         │
│                                    │ ◐ test           │
│                                    ├──────────────────┤
│                                    │ METRICS          │
│                                    │ Cost: $0.06      │
│                                    │ Tokens: 8,200    │
│                                    │ Time: 34s        │
│                                    │ Model: haiku     │
│                                    ├──────────────────┤
│                                    │ ARTIFACTS        │
│                                    │ src/main.rs      │
│                                    │ src/lib.rs       │
│                                    │ Cargo.toml       │
└────────────────────────────────────┴──────────────────┘
```

**How it works**:
- **One command** does everything: `roko build "Build a Rust CLI that converts temperatures"`
- This is a new unified command that replaces the current 4-step `prd idea` → `prd draft` → `prd plan` → `plan run` pipeline
- The sidebar shows a pipeline visualization that updates in real time:
  - IDEA captured → PRD generated → plan created → tasks executing (with per-task status) → gates running → done
- Terminal shows streaming agent output (what the agent is writing, what files it's creating)
- On completion: gate results, cost summary, list of created files

**Commands (1-2)**:
| # | Command | What |
|---|---|---|
| 1 | `roko build "Build a Rust CLI that converts temperatures"` | The whole pipeline, one shot |
| 2 | `roko status` | Optional: show what was created (signals, episodes, artifacts) |

**Why this works for the pitch**: This is the "self-hosting" story in miniature. The audience sees roko take a sentence and turn it into code that compiles and passes tests. The pipeline sidebar makes the process visible and structured — it's not just "a terminal did some stuff."

**CLI changes needed**:
- `roko build "<prompt>"` — new top-level command that runs the full pipeline:
  1. Creates a workspace (or uses current)
  2. Generates an internal PRD (no slug, no file)
  3. Generates a plan with tasks
  4. Executes tasks (agents write code)
  5. Runs gates (compile, clippy, test)
  6. Reports results
- This replaces `prd pipeline` from the v1 redesign and goes further — the PRD/plan are internal artifacts, not visible file system state the presenter needs to manage
- Must support streaming progress events to stderr or a sideband channel so the sidebar can update

**Prompt options**:
- "Build a Rust CLI that converts temperatures between Celsius and Fahrenheit"
- "Build a Rust library that validates email addresses"
- "Build a Rust function that solves FizzBuzz with configurable rules"

All are small, deterministic, gate-friendly, and understandable by non-developers.

---

### 3. "Memory" — The Thousandth Agent

**Thesis primitive**: Shared knowledge
**One-line pitch**: Agent B inherits Agent A's knowledge and does the same job 40% faster.
**What makes it click**: Two timers running side by side. One finishes first.

**Layout**: 2 panes + center comparison panel

```
┌────────────────┬────────┬────────────────┐
│ AGENT A (cold) │ DELTA  │ AGENT B (warm) │
│                │        │                │
│ [terminal]     │  ??%   │ [terminal]     │
│                │  ??%   │                │
│                │  ??%   │                │
│                │        │                │
│ TIME: 67s      │ TIME   │ TIME: --       │
│ COST: $0.09    │ COST   │ COST: --       │
│ TOKENS: 14K    │ TOKENS │ TOKENS: --     │
│ GATES: ✔ pass  │ GATES  │ GATES: --      │
│                │        │                │
│ KNOWLEDGE: 8   │ ──→──→ │ KNOWLEDGE: 8+  │
└────────────────┴────────┴────────────────┘
```

**How it works**:
- Pane 0 (left): Agent A builds from scratch. Cold start. No prior knowledge.
- When A completes, knowledge is automatically transferred to B's workspace (visible in the center panel as an animated arrow/flow)
- Pane 1 (right): Agent B builds a related task using A's knowledge
- Center panel shows the comparison *as it develops*: A's numbers are fixed, B's numbers update live
- When B completes, the delta column fills in with percentages

**Commands (2)**:
| # | Command | Target | What |
|---|---|---|---|
| 1 | `roko build "..."` | pane 0 | Cold build. Knowledge transfer happens automatically on completion. |
| 2 | `roko build "..."` | pane 1 | Warm build with inherited knowledge. |

The knowledge transfer happens automatically between steps 1 and 2 — no manual `cp -r`, no "sync-knowledge" click. The scenario runner handles it and the sidebar animates it.

**Prompt pairing**: Related tasks where domain knowledge transfers:
- A: "Build a Rust CLI that parses CSV and outputs JSON"
- B: "Build a Rust CLI that parses TOML and outputs JSON"
- (File parsing, serialization, error handling patterns all transfer)

**Why this works for the pitch**: "The thousandth agent joins smarter than the first." The audience sees concrete numbers proving that accumulated knowledge makes agents better. This is the moat claim: single-tenant orchestrators can't compound knowledge across runs.

**Key decisions**:
- Both prompts must be related enough that knowledge actually transfers (same domain, different specifics)
- Both prompts must be small enough to complete in <60s each
- The knowledge transfer step should be visually dramatic — not a hidden copy, but an animated flow in the sidebar
- If B isn't actually faster (it's non-deterministic), the sidebar should still show honest numbers. The demo is about *capability*, not guaranteed improvement. But the prompts should be chosen to maximize the chance of visible improvement.

---

### 4. "ISFR" — Agent Swarm Computing a Rate

**Thesis primitive**: Identity + coordination
**One-line pitch**: 4 specialized agents compute DeFi's risk-free rate from live on-chain data.
**What makes it click**: A number appears. The ISFR rate. Computed live from Aave, Compound, Lido, Rocket Pool.

**Layout**: 4 terminal panes + bottom rate display + sidebar agent cards

```
┌────────────────┬────────────────┐
│ LENDING SCOUT  │ STAKING SCOUT  │
│ Aave + Compound│ Lido + RPL     │
│ [terminal]     │ [terminal]     │
│ Rate: 3.2%     │ Yield: 3.8%    │
├────────────────┼────────────────┤
│ AGGREGATOR     │ VALIDATOR      │
│ TVL-weighted   │ Bounds + sign  │
│ [terminal]     │ [terminal]     │
│ Computing...   │ Waiting...     │
├────────────────┴────────────────┤
│                                 │
│   ISFR:  3.41%  ✔ validated     │
│   Published at block 21,234,567 │
│                                 │
└─────────────────────────────────┘
```

**How it works**:
- Pre-condition: mirage-rs running with Ethereum mainnet fork (pre-warmed)
- Step 1: Health check — quick chain connectivity proof
- Step 2: Launch all 4 agents simultaneously with one click
  - **Lending Scout**: Reads Aave V3 + Compound V3 supply rates via `cast call` against the fork
  - **Staking Scout**: Reads Lido stETH + Rocket Pool rETH exchange rates
  - **Aggregator**: Waits for both scouts' knowledge entries, computes TVL-weighted median per class, then weighted sum → ISFR
  - **Validator**: Checks computed rate against hardcoded historical bounds (2.5% - 6.0%), validates data freshness, signs off
- When the Validator signs off, the bottom banner shows the final ISFR rate

**Commands (2-3)**:
| # | Command | Target | What |
|---|---|---|---|
| 1 | Health check | pane 0 | `curl` + `cast bn` — proves chain is live |
| 2 | Launch swarm | all panes | 4 agents start simultaneously, each with a role prompt |
| 3 | Results | pane 0 | `roko learn all` — optional, shows episode/knowledge summary |

**Agent prompts** (short, role-specific, injected via system prompt):

Lending Scout:
```
Query Aave V3 USDC supply rate and Compound V3 USDC supply rate on the
local Anvil fork. Use `cast call` to read on-chain. Write rates as
knowledge entries with source, rate (bps), and TVL.
```

Staking Scout:
```
Query Lido stETH and Rocket Pool rETH exchange rates on the local Anvil
fork. Compute implied staking yields. Write as knowledge entries.
```

Aggregator:
```
Read all rate knowledge entries from the neuro store. Compute
TVL-weighted median per class (lending 60%, staking 25%, structured 10%,
funding 5%). Output the composite ISFR rate.
```

Validator:
```
Read the computed ISFR rate. Verify it falls within historical bounds
(250-600 bps). Check all source entries have timestamps within 60s.
Output: VALID or INVALID with reasons.
```

**Why this works for the pitch**: This is the canonical example from the pitch deck. 4 agents with distinct roles (ERC-8004 identities in the full vision) collaborate via shared knowledge to compute a real financial metric from live protocol data. The audience sees the ISFR number appear — that's the punchline.

**Chain setup**:
- Contracts come from `roko.toml [chain.contracts]`, not hardcoded prompts
- mirage-rs pre-warmed at a specific mainnet fork block where all protocols have known state
- `cast` on PATH (part of foundry toolchain)
- BlockTicker strip shows live block numbers

---

### 5. "Oracle" — Agents Meet Smart Contracts

**Thesis primitive**: Cost prediction + Identity (the DeFi expansion story)
**One-line pitch**: An agent reads on-chain state, analyzes opportunities, and stakes a position.
**What makes it click**: A transaction hash appears on screen. Verified on-chain.

**Layout**: 2 panes (data → strategy) + sidebar with chain metrics

```
┌────────────────────────┬────────────────────────┐
│ DATA AGENT             │ STRATEGY AGENT          │
│                        │                        │
│ [terminal:              │ [terminal:              │
│  reading Aave V3...    │  waiting for data...    │
│  USDC supply: 3.21%   │                        │
│  DAI supply: 3.05%    │                        │
│  reading Compound...   │                        │
│  USDC supply: 2.84%]  │                        │
│                        │                        │
├────────────────────────┴────────────────────────┤
│ CHAIN STATUS                                    │
│ ● Block: 21,234,567  Chain: Anvil (31337)       │
│                                                 │
│ KNOWLEDGE FLOW                                  │
│ Data Agent ──→ neuro/ ──→ Strategy Agent        │
│ Entries: 4 written    Entries: 0 read           │
│                                                 │
│ RECOMMENDATION                                  │
│ ━━━━━━━━━━━━━━━━━━━━━ pending                   │
└─────────────────────────────────────────────────┘
```

**How it works**:
- Step 1: Quick chain health check (block number, connectivity)
- Step 2: Data Agent reads on-chain DeFi rates via `cast call`, writes structured analysis to knowledge store
- Step 3: Strategy Agent reads Data Agent's findings from knowledge store, recommends allocation

**Commands (2-3)**:
| # | Command | Target | What |
|---|---|---|---|
| 1 | Health check | pane 0 | Chain connectivity proof |
| 2 | Data collection | pane 0 | Data Agent queries protocols, writes to knowledge store |
| 3 | Strategy analysis | pane 1 | Strategy Agent reads knowledge, recommends allocation |

**Narrative**: This is the "Oracle" story from the pitch docs — agents as intelligent DeFi data sources. The Data Agent collects, the Strategy Agent reasons. Both are autonomous, coordinated via the knowledge store, and operating on verifiable on-chain data.

**The punchline**: Unlike a static oracle feed (Chainlink, Pyth), roko's agents can *reason* about the data — spot anomalies, compare across protocols, recommend actions. They don't just report numbers; they interpret them.

**Why this works for the pitch**: Shows the DeFi expansion lane without requiring the full ISFR swarm. Two agents is enough to demonstrate the pattern: on-chain data → agent intelligence → actionable output. This is the "Cooperative Clearing" concept in miniature.

---

## Required CLI changes

Current roko has `prd idea` → `prd draft` → `prd plan` → `plan run` as separate commands. The demos need fewer, more atomic operations.

### `roko build "<prompt>"`
The hero command. One shot: idea → code → validated.

**Internally**:
1. Create ephemeral PRD (in-memory, no file)
2. Generate plan with tasks (in-memory or `.roko/plans/`)
3. Execute tasks sequentially (agents write code)
4. Run gate pipeline (compile, clippy, test)
5. Record episode, update knowledge store
6. Print summary: files created, gates passed, cost, time

**Streaming output** (critical for demos):
```
● Generating plan...
  3 tasks identified
● [1/3] Scaffolding project...
  Created: Cargo.toml, src/main.rs
● [2/3] Implementing converter...
  Modified: src/main.rs (42 lines)
● [3/3] Adding tests...
  Created: tests/integration.rs
● Running gates...
  ✔ compile (0.8s)
  ✔ clippy (1.2s)
  ✔ test (0.6s)

Done in 34s · $0.06 · 8,200 tokens · 3 files created
```

This output format is what makes the demo work — the audience reads structured progress, not a wall of agent stderr.

### `roko run --no-cascade`
Force single-tier dispatch (skip cascade router). For the Cost demo to show the difference.

Could also be `--tier T2` to force a specific tier, or `--model <exact>` to force a specific model.

### `roko build --knowledge-from <dir>`
Pre-load knowledge from another workspace before building. For the Memory demo.

Replaces the manual `cp -r .roko/neuro/` step. Makes knowledge transfer a first-class concept.

### `roko serve` improvements
The demo UI communicates with `roko serve`. New endpoints or SSE events needed:

- **Build progress events**: SSE stream that emits pipeline stage transitions (idea → prd → plan → task:1 → task:2 → gates → done) so the sidebar can update in real time without polling the terminal output buffer
- **Cost/token tracking**: Events with running cost and token counts per workspace
- **Knowledge stats**: Endpoint to read current entry count for a workspace

---

## Required demo-app changes

### Sidebar as primary visual

The sidebar needs scenario-specific panels, not a generic stats list. Each scenario type gets a custom sidebar component:

| Scenario | Sidebar component | Key visual |
|---|---|---|
| Cost | ComparisonPanel | Three-column cost/time/token comparison with delta percentages |
| Pipeline | PipelinePanel | Vertical state machine (IDEA → PRD → PLAN → BUILD → VALIDATE) with animated transitions |
| Memory | TransferPanel | Two columns with center delta, knowledge meter with transfer animation |
| ISFR | SwarmPanel | 4 agent cards with status + bottom ISFR rate banner |
| Oracle | FlowPanel | Two agent cards with knowledge flow arrows, chain status strip |

### Terminal changes
- **Pre-clear** before each command (no scrollback from previous steps)
- **Auto-scroll** locked to bottom during execution
- **Output buffer**: 128KB (up from 60KB)
- **Remove character-by-character typing**: Just paste the command instantly. The typing animation adds 2-5s of dead time per command and provides zero value.

### Command list
- Max 3 items per scenario
- Each has: icon, label, elapsed time, status (pending/running/pass/fail)
- No "Run All" needed — the scenarios are so short that manual clicking is fine
- Retry button per command if it fails

### Model selection
- ConfigWidget stays, but each scenario has a sensible default
- Cost scenario: fixed to a known expensive model for the naive side, cascade router for the smart side
- All others: use whatever ConfigWidget says

---

## What gets cut

| Old | Replaced by | Why |
|---|---|---|
| prd-pipeline (8 cmds) | Pipeline (1-2 cmds) | `roko build` replaces 4-step PRD dance |
| prd-research-loop (9 cmds) | Cut | Research is a feature, not a demo |
| race (3 cmds) | Cost | Better framing, isolated workspaces |
| gate-retry (6 cmds) | Cut | Gate retry visible inside Pipeline sidebar |
| providers (4 cmds) | Cut | Redundant with Cost |
| provider-race (5 cmds) | Cost | Same idea, better execution |
| explore (12 cmds) | Cut | Read-only commands not demo-worthy |
| knowledge-accumulation (10 cmds) | Memory | Knowledge growth visible in Memory sidebar |
| knowledge-transfer (6 cmds) | Memory | Transfer is the climax of Memory |
| dream-consolidation (6 cmds) | Cut | Dream cycle is a feature, not a demo |
| chat (4 cmds) | Cut | TUI can't reset, timing fragile |
| chain-intelligence (12 cmds) | Oracle | Same concept, 3 cmds not 12 |
| mirage (4 cmds) | Absorbed | Health check is step 1 of chain demos |
| isfr-agents (9 cmds, 8 panes) | ISFR (3 cmds, 4 panes) | Same concept, half the agents |

**14 scenarios → 5 scenarios**
**Average 7 commands → average 2.4 commands**
**8 panes max → 4 panes max**

---

## Demo order and timing

For a 15-minute talk:

| Order | Scenario | Time | Narrative beat |
|---|---|---|---|
| 1 | **Pipeline** | ~2 min | "Here's what roko does: you describe, it builds." (The baseline) |
| 2 | **Cost** | ~90s | "Same task, 78% cheaper. The system routes to the right model." (The wedge) |
| 3 | **Memory** | ~3 min | "Agent B inherits Agent A's knowledge. 40% faster." (The moat) |
| 4 | **ISFR** | ~3 min | "4 agents compute a DeFi rate from live on-chain data." (The vision) |
| 5 | **Oracle** | ~2 min | "Agent reads chain, agent reasons, agent recommends." (The expansion) |

Total: ~12 minutes of demo, ~3 minutes of transition/talking.

For a 5-minute pitch: run Pipeline + Cost only. Under 4 minutes total.
For a 30-minute deep dive: run all 5, with pauses to explore artifacts and discuss architecture.

---

## Pre-warming protocol

All of this should be a single command: `dev.sh demo`

What it does:
1. Starts `roko serve` on :6677
2. Starts mirage-rs on :8545 (for ISFR + Oracle)
3. Starts demo frontend on :5173
4. Runs provider health checks (Anthropic key? OpenAI key? Gemini key?)
5. Pre-creates workspaces for each scenario
6. Reports: "Ready. 3/3 providers available. Chain: block 21,234,567."

What it does NOT do:
- Pre-run any LLM calls
- Cache any responses
- Pre-populate any workspace state

Everything is live.

---

## Open design questions

1. **`roko build` scope**: Should this be a full new top-level command, or a wrapper around `roko run` with `--pipeline` mode? The former is cleaner UX; the latter is less new code.

2. **Knowledge transfer mechanism**: First-class flag (`--knowledge-from <dir>`) vs. `roko knowledge sync <source> <target>` vs. automatic detection? The flag is simplest for demos but `sync` is more general.

3. **Cost demo isolation**: Does `--no-cascade` exist today? If not, what's the simplest way to force single-tier dispatch? Could also just use `--provider anthropic --model claude-sonnet-4-5-20250514` to pin a specific model.

4. **ISFR agent coordination**: The 4 agents need to coordinate (aggregator waits for scouts). This requires either:
   - Polling: aggregator retries `roko knowledge query` until it finds scout entries
   - Dependency: tasks.toml declares agent order and the plan runner handles sequencing
   - Shared workspace: all 4 agents write to the same `.roko/neuro/` and the aggregator prompt says "wait until you find entries from scouts"
   Which approach is most reliable?

5. **Sidebar SSE**: Does `roko serve` currently emit SSE events for build progress? If not, what's the lift to add per-workspace progress events?
