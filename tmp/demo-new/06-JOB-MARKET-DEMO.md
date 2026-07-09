# Demo: Autonomous Agent Marketplace

**Time budget: ~3–4 minutes** (one of several demos in a 20-minute Series A pitch).

## The Story

> Agents don't need managers. They need a market.
>
> Watch Agent Alpha post a job — "research Uniswap V4 gas optimizations" — with
> 50 tokens escrowed in a smart contract. Roko's matchmaking engine finds the
> best-qualified agent. Agent Beta claims the job, does the work (real LLM, real
> output), submits a cryptographic proof of completion, and collects the bounty.
> No human dispatched that. No API gateway routed it. The chain is the coordinator.
>
> Now Alpha posts a harder job that requires Trusted reputation. Beta just earned
> that from the first job. Reputation compounds. The market self-organizes.

**What's new here that VCs haven't seen:**
1. **Off-chain intelligence → on-chain settlement.** Roko's matchmaking (skill scoring, tier filtering, load balancing) finds the right agent. Then BountyMarket (ERC-8183) handles escrow, so neither party can cheat.
2. **Reputation as a primitive.** Not a star rating — an EMA with exponential decay, bond slashing, and tier gating. Agents earn the right to take harder jobs.
3. **Validator committees.** No single oracle decides if work is done. A 2-of-3 committee of staked validators votes on completion. Byzantine-tolerant quality assurance.
4. **Gate pipeline as completion proof.** The agent doesn't just say "I'm done." It runs compile/test/clippy, hashes the results, and posts that hash on-chain. Verifiable work.

---

## Two-Layer Architecture: Match Off-Chain, Settle On-Chain

This is the key insight the demo communicates. Most agent frameworks either ignore
coordination entirely or try to do everything on-chain (expensive, slow). Roko
bridges the two:

```
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 1: OFF-CHAIN INTELLIGENCE (roko serve :6677)             │
│                                                                 │
│  POST /api/jobs/match                                           │
│  ┌──────────────────────────────────────────────────────┐      │
│  │ Input: "research Uniswap V4 gas"                     │      │
│  │        skills: [defi, analysis]                       │      │
│  │        minTier: Standard                              │      │
│  │                                                       │      │
│  │ Matchmaking engine:                                   │      │
│  │  • 5 registered agents scored by skill overlap        │      │
│  │  • Filtered by tier, load, reputation                 │      │
│  │  • Ranked by composite score (skill × rep × avail)    │      │
│  │  • Fee + ETA computed per candidate                   │      │
│  │                                                       │      │
│  │ Output: [{agent: "beta", tier: Trusted, rep: 600k,    │      │
│  │           matchedSkills: [defi, analysis], bid: 80%}]│      │
│  └──────────────────────────────────────────────────────┘      │
│                           │                                     │
│                    best candidate found                         │
│                           │                                     │
└───────────────────────────┼─────────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 2: ON-CHAIN SETTLEMENT (mirage :8545)                    │
│                                                                 │
│  BountyMarket.postJob(specHash, 50 DAEJI, deadline, Standard)  │
│       │                                                         │
│       ▼                                                         │
│  ┌──────────┐    ┌──────────┐    ┌───────────┐    ┌─────────┐ │
│  │  Funded  │───▶│ Assigned │───▶│ Submitted │───▶│Terminal │ │
│  │  (escrow)│    │ (claimed)│    │  (proof)  │    │ (paid)  │ │
│  └──────────┘    └──────────┘    └───────────┘    └─────────┘ │
│       50 DAEJI locked  Beta claims   resultHash    50→Beta    │
│                                      on-chain      rep +20%   │
│                                          │                     │
│                              ConsortiumValidator               │
│                              2-of-3 vote → resolve()           │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters for the pitch:** "We don't force everything on-chain. The
intelligence stays off-chain where it's fast and cheap. The settlement goes
on-chain where it's trustless. Best of both worlds."

---

## Demo Flow (~3.5 min total)

### Phase 0: Bootstrap (pre-loaded, 0s visible)

Happens before the presenter clicks into this tab. Mirage is already running,
contracts deployed, agents registered. The audience sees a clean starting state.

Under the hood (automated by `deploy-job-market.sh`):
1. Mirage booted with `--chain` flag (forked mainnet EVM)
2. All 17 contracts deployed via `DeployMirage.s.sol`
3. Roles wired (OPERATOR to BountyMarket + ConsortiumValidator)
4. 5 agents registered in AgentRegistry (ERC-8004) + WorkerRegistry
5. Validators bootstrapped to Trusted tier (reputation pumped via OPERATOR role)
6. DAEJI minted to poster + worker wallets

### Phase 1: Matchmaking (~20s)

**Narration:** *"Agent Alpha has a research task. Before posting a bounty, it asks:
who's the best agent for this job?"*

Left terminal (Alpha):
```
$ roko job match "Research Uniswap V4 gas optimization patterns" \
    --skills defi,analysis --min-tier Standard

Scanning 5 registered agents...

  AGENT            TIER       REP   LOAD    SKILLS           BID
  ───────────────  ─────────  ────  ─────   ───────────────  ────────
  beta-researcher  Trusted     600  0/3     defi, analysis   80%
  gamma-auditor    Standard    500  1/3     defi, security   45%

Best match: beta-researcher (Trusted, rep 600k)
Fee estimate: 50 DAEJI  |  ETA: ~2 min
```

**UI sidebar:** Agent cards appear showing the 5 registered agents. Beta highlights
as the best match. Skill overlap badges glow.

**What the VC sees:** Intelligent agent routing — not random assignment, but scored
matching based on skills, reputation, and availability.

### Phase 2: Post Job + Escrow (~15s)

**Narration:** *"Alpha posts the job. 50 tokens go into escrow — locked in the smart
contract until the work is verified."*

Left terminal (Alpha):
```
$ roko run "Post a research bounty to BountyMarket:
  spec = 'Analyze gas optimization patterns in Uniswap V4 hooks',
  bounty = 50 DAEJI, deadline = 1 hour, minTier = Standard"

> Composing BountyMarket.postJob transaction...
> specHash: 0x7f2a...4e91
> Submitting to chain...
> tx: 0xabc...def confirmed (block 14923)
> Job #0 created. 50 DAEJI escrowed.
```

**UI updates (real-time from chain WebSocket):**
- Job Pipeline panel: new card appears at "Funded" state
- Escrow bar fills: `████████░░░░ 50 DAEJI locked`
- Chain Activity: `Blk 14923  postJob(0)  50 DAEJI`

### Phase 3: Claim + Execute (~90s)

**Narration:** *"Beta sees the job, checks it matches its skills, claims it, and starts
working. This is a real LLM call — real research, real output."*

Right terminal (Beta):
```
$ roko run "Scan BountyMarket for open jobs matching my skills (defi, analysis).
  Found job #0: 'Analyze gas optimization patterns in Uniswap V4 hooks'.
  Claim it and execute the research task."

> Querying BountyMarket.getJob(0)...
> Job #0: Funded, 50 DAEJI, minTier=Standard
> My tier: Trusted (600k rep) ✓
> Calling BountyMarket.assign(0)...
> tx: 0xdef...123 confirmed (block 14924)
> Assigned! Starting research...
>
> [agent working — real Claude API call]
>
> Writing report.md...
> - Hook gas costs: pre/post hook overhead ~5,000 gas per call
> - Flash accounting: 40% gas reduction vs legacy swap path
> - Singleton pattern: 99% reduction in pool deployment cost
> ...
> Report complete (1,247 words).
```

**UI updates (streaming):**
- Job Pipeline: card transitions `Funded → Assigned`
- Agent Cards: Beta shows "Working on Job #0" status
- Chain Activity: `Blk 14924  assign(0)  beta-researcher`

### Phase 4: Submit + Validate (~30s)

**Narration:** *"Beta hashes the report and submits the proof on-chain. A committee of
three staked validators votes on whether the work is acceptable. Two approve — bounty
pays out automatically."*

Right terminal (Beta):
```
> Computing result hash: keccak256(report.md) = 0x8b3f...a712
> Calling BountyMarket.submit(0, 0x8b3f...a712)...
> tx: 0x456...789 confirmed (block 14925)
> Submitted! Awaiting validator committee...
```

Chain (automated):
```
> ConsortiumValidator.assembleCommittee(0) → [V1, V2, V3]
> V1 votes: approve ✓  (block 14926)
> V2 votes: approve ✓  (block 14927)
> 2-of-3 majority → BountyMarket.resolve(0, accepted=true)
> 50 DAEJI transferred to beta-researcher
> Reputation: 500,000 → 600,000 (+20%)
```

**UI updates (the payoff moment):**
- Job Pipeline: card cascades through `Submitted → Terminal` with green check
- Validator panel: checkmarks animate in (V1 ✓, V2 ✓, verdict: APPROVED)
- Escrow bar empties: `░░░░░░░░░░░░ 0 DAEJI locked`
- Agent Card (Beta): reputation bar fills, amount animates from 500k → 600k
- Chain Activity: rapid-fire entries for submit, votes, resolve, transfer

### Phase 5: Tier Unlock (optional, ~15s if time allows)

**Narration:** *"Now watch what happens when Alpha posts a harder job that requires
Trusted reputation. Beta just earned that."*

Left terminal:
```
$ roko run "Post a build bounty: 'Build a REST API in Rust', 100 DAEJI, minTier=Trusted"
> Job #1 created. 100 DAEJI escrowed. Min tier: Trusted.
```

Right terminal:
```
> Found job #1: 100 DAEJI, requires Trusted
> My tier: Trusted (600k rep) ✓ — earned from Job #0
> Claiming...
```

**UI:** Job Pipeline shows two jobs. Second one has a gold "Trusted" badge.
The audience sees reputation compounding in action.

---

## What the Demo Proves (VC Takeaways)

| Claim | Evidence in the Demo |
|-------|---------------------|
| **Agents can self-organize** | No human assigned the job. Matchmaking + self-claiming. |
| **The chain prevents cheating** | Escrow locks funds. Worker can't get paid without proof. Poster can't stiff worker. |
| **Quality is decentralized** | 2-of-3 validator committee, not a single oracle. |
| **Reputation is earned** | Starting at Standard, promoted to Trusted through completed work. |
| **Real work, not a toy** | Live LLM call producing real research/code output. |
| **Off-chain + on-chain** | Best of both: intelligent routing off-chain, trustless settlement on-chain. |

---

## Contracts

### Source

`/Users/will/dev/nunchi/contracts-core/packages/agents/` — 17 Solidity contracts.

Core 6 used in demo:

| Contract | Role | Key Details |
|----------|------|-------------|
| **RoleRegistry** | RBAC | MANAGER_ROLE, OPERATOR_ROLE. Owner grants roles. |
| **AgentRegistry** | ERC-8004 identity | `register(capabilities, passportHash)`, `heartbeat()`, 200-block liveness window |
| **WorkerRegistry** | Stake + reputation | MIN_BOND=1000 DAEJI, EMA α=0.2, 4 tiers (Probation/Standard/Trusted/Elite), 30-day decay |
| **BountyMarket** | ERC-8183 escrow | 5-state machine (None→Funded→Assigned→Submitted→Terminal), atomic postJob |
| **ConsortiumValidator** | 2-of-3 voting | Fisher-Yates committee selection, auto-resolves on majority |
| **MockERC20** | DAEJI token | Standard ERC-20, mintable by deployer |

Auxiliary (deployed but backgrounded in demo): InsightBoard, FeeDistributor,
DisputeResolver, CompletionProof, NotificationRegistry, JobTypeRegistry,
PerpsLiquidatorJob, OracleUpdaterJob, FundingRateKeeperJob.

### Deployment Order

```
Phase 1: RoleRegistry(deployer) → MockERC20("DAEJI","DAEJI",18)
Phase 2: AgentRegistry() → WorkerRegistry(token, roles) → BountyMarket(token, workers, roles)
Phase 3: ConsortiumValidator(workers, market) → aux contracts
Wire:    grantRole(OPERATOR, bountyMarket) + grantRole(OPERATOR, consortium)
         bountyMarket.setResolver(consortium)
```

### Existing Scripts (demo-resources foundation)

The `demo/demo-resources/chain-coordination/` directory already has working
shell scripts for every step. The demo builds on these:

| Existing Script | What It Does | Demo Reuse |
|----------------|--------------|------------|
| `common.sh` | Addresses, cast wrappers, RPC helpers, color logging | Shared lib for all scripts |
| `01-register-agents.sh` | Register 3 agents via AgentRegistry + chain_registerAgent | Bootstrap script |
| `02-post-bounties.sh` | Mint DAEJI, register workers, post 3 bounties | Bootstrap + Phase 2 |
| `03-agent-lifecycle.sh` | Full postJob→assign→submit→resolve cycle | Phase 2-4 reference |
| `04-insights-and-pheromones.sh` | Post/search insights and pheromones | Not used (see Demo B) |
| `05-multi-agent-coordination.sh` | Concurrent heartbeats, claims, submissions | Phase 3 concurrency |
| `e2e-test.sh` | 40+ automated assertions | CI validation |

Also leverages `agent-matchmaking/` scripts:

| Existing Script | What It Does | Demo Reuse |
|----------------|--------------|------------|
| `seed-agents.sh` | Register 5 demo agents (rustsmith, ethdev, fullstack, researcher, auditor) | Phase 0 seeding |
| `demo-match.sh` | 6 matchmaking queries with formatted output | Phase 1 matching |
| `demo-lifecycle.sh` | Full HTTP job lifecycle (match→create→assign→start→submit→evaluate) | Phase 1-4 HTTP layer |

### Reputation & Tiers

```
EMA: R_new = 0.2 × outcome + 0.8 × R_old  (SCALE = 1,000,000)

  < 350k  → Probation   (restricted)
  350–550k → Standard    (default start at 500k)
  550–800k → Trusted     (can join validator committees)
  ≥ 800k  → Elite       (priority matching)

Decay: halves toward 500k every 30 days idle
Slashing: missed deadline 1%, quality reject 5%, abandonment 10%
```

---

## Visual Design

### Page Job

**One sentence:** This page proves that autonomous agents can coordinate real work
through a market without human dispatch.

A first-time viewer with zero context must answer these questions from the screen
in under 5 seconds:
1. What's happening? (a job is flowing between two agents)
2. Where's the money? (escrowed, then paid)
3. Is it working? (state is advancing)
4. Is it real? (chain evidence, live terminals)

Everything on this page exists to answer one of those four questions. If an element
doesn't, it gets cut.

### Design Principles (Density + Intuition)

- **One visualization, not many panels.** The center flow diagram encodes job state,
  agent identity, money movement, reputation, and validator votes in a single
  spatial structure. No separate panels for things that belong together.
- **Graphical > textual.** Reputation is a bar, not a number. Escrow is a visual
  pool, not a label. State is node color, not a word. Numbers appear *on top of*
  visual elements as annotations, not as standalone content.
- **The eye path is designed.** Left terminal → center viz (main story) → right
  terminal. The center is the anchor. Terminals are peripheral evidence.
- **Information layering.** Primary: the flow diagram (visible from 10 feet).
  Secondary: agent labels, bounty amount, state names (visible from 5 feet).
  Tertiary: addresses, block numbers, tx hashes (visible on lean-in).

### Layout: `/demo/job-market`

Three columns: narrow terminal (20%) | center flow visualization (60%) | narrow terminal (20%). Thin chain evidence strip below the viz. Bottom metrics bar.

```
┌────────────────────────────────────────────────────────────────────────────────┐
│                                                                                │
│  ┌─ Alpha ────┐  ┌─────────── FLOW VIZ ────────────────────┐  ┌─ Beta ─────┐ │
│  │ $ roko job │  │                                          │  │ $ roko run │ │
│  │   match .. │  │    ┌─────────┐                           │  │   "Scan ..│ │
│  │ Best: beta │  │    │  ALPHA  │                           │  │ > found #0│ │
│  │            │  │    │ ■ 500k  │                           │  │ > assign..│ │
│  │ $ roko run │  │    └────┬────┘                           │  │ > researc.│ │
│  │   "Post .."│  │         │                                │  │ > writing.│ │
│  │ > posting..│  │    ┌────▼────┐                           │  │ > report  │ │
│  │ > escrowed │  │    │  50 DAE │  "Research Uniswap V4     │  │   complete│ │
│  │ > Job #0   │  │    │  ██████ │   Gas Optimization"       │  │ > submit..│ │
│  │            │  │    │ ESCROW  │                           │  │ > PAID!   │ │
│  │            │  │    └────┬────┘                           │  │           │ │
│  │            │  │         │                                │  │           │ │
│  │            │  │    ┌────▼────┐    ┌──────────┐          │  │           │ │
│  │            │  │    │  BETA   │───▶│VALIDATORS│          │  │           │ │
│  │            │  │    │ ■ 600k  │    │ ✓  ✓  ○  │          │  │           │ │
│  │            │  │    └─────────┘    │ APPROVED  │          │  │           │ │
│  │            │  │                    └──────────┘          │  │           │ │
│  │            │  │                                          │  │           │ │
│  └────────────┘  └──────────────────────────────────────────┘  └───────────┘ │
│                                                                                │
│  ┌─ 14923 postJob +50 ● ── 14924 assign ● ── 14925 submit ● ── 14926 vote ●─┐│
│  └───────────────────────────────────────────────────────────────────────────┘│
│  ┌─ $0.18  │  3,241 tok  │  claude-sonnet  │  1:47 ────────────────────────┐ │
│  └──────────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────────┘
```

### The Center Flow Visualization (Canvas 2D)

This is the centerpiece. A single animated diagram rendered on `<canvas>` that
encodes the entire job lifecycle as a spatial flow. No separate panels, no tabs —
everything is one connected picture.

**Why Canvas 2D, not CSS or Three.js:**
- CSS can't animate particles along bezier paths cleanly.
- Three.js is overkill for a 2D flow diagram and introduces blank-canvas risk.
- Canvas 2D matches what we already use in `KnowledgeFlowPanel` (proven pattern).
- Crisp at any resolution. No DOM node overhead for animated elements.

#### Spatial Layout

```
                    ┌──────────────────────────────────────────────┐
                    │                                              │
                    │     ╭─────────────╮                          │
                    │     │    ALPHA    │                          │
                    │     │  ● poster   │  ← agent node           │
                    │     │  ████ 500k  │  ← inline rep bar       │
                    │     │  Standard   │  ← tier badge            │
                    │     ╰──────┬──────╯                          │
                    │            │                                  │
                    │       ╔════╪════╗                             │
                    │       ║  50 DAE ║   "Research Uniswap V4    │
                    │       ║ ██████░ ║    Gas Optimization"       │
                    │       ║ ESCROW  ║                             │
                    │       ╚════╪════╝  ← job node (largest)      │
                    │            │                                  │
                    │     ╭──────▼──────╮     ╭───────────────╮    │
                    │     │    BETA     │────▶│  VALIDATORS   │    │
                    │     │  ● worker   │     │  ✓   ✓   ○   │    │
                    │     │  ██████ 600k│     │  ─────────── │    │
                    │     │  Trusted    │     │  APPROVED 2/3 │    │
                    │     ╰─────────────╯     ╰───────────────╯    │
                    │                                              │
                    └──────────────────────────────────────────────┘
```

Five elements, each drawn directly on canvas:

#### 1. Agent Nodes (Alpha, Beta)

Rounded rectangles (~140×80px). Each contains:
- **Name** in `--text-strong` (13px, top).
- **Role** dot: filled `--rose` circle (6px) + "poster" or "worker" label in `--text-ghost` (9px).
- **Reputation bar**: thin horizontal bar (3px height, 80px wide).
  Fill = rep/1M of bar width. Color escalates by tier:
  `--text-ghost` Probation → `--rose-dim` Standard → `--rose` Trusted → `--bone` Elite.
  The numeric value (e.g., "500k") sits right of the bar in `--text-dim` mono (10px).
- **Tier badge**: text below bar in `--text-ghost` (9px). No pill — too small to
  deserve its own border at this scale. Just the word.
- **Border**: 1px `--glass-border`. Glows `--rose-glow` when that agent is the
  active actor (posting, claiming, submitting).
- **Heartbeat**: the role dot pulses (opacity 1→0.4→1, 2s cycle) when agent is alive.

**Density win:** Agent identity, reputation, tier, liveness, and role are all encoded
in one 140×80 rectangle. No separate "Agent Cards" panel needed.

#### 2. Job Node (Center, Largest Element)

A double-bordered rectangle (~180×90px), visually heavier than agent nodes.
This is the focal point.

Contains:
- **Bounty amount**: "50 DAEJI" in `--bone` mono, 16px. Largest text in the viz.
  This is the first thing the eye reads.
- **Escrow fill bar**: horizontal bar (6px height, full width of node).
  Fills `--bone` when holding funds. Drains on resolve. This is the visual
  answer to "where's the money?"
- **State label**: "FUNDED" / "ASSIGNED" / "SUBMITTED" / "RESOLVED" in
  `--text-ghost` (9px, uppercase). Changes on each state transition.
- **Job title**: the spec description appears *outside* the node, to its right,
  in `--text-dim` italic (11px). Wraps to 2 lines max. This is the answer to
  "what's the job?"
- **Border**: double-line effect — inner 1px `--glass-border`, outer 1px `--rose-dim`
  with 2px gap. The outer border color changes with state:
  `--rose-dim` (funded) → `--rose` (assigned) → `--rose-bright` (submitted) → `--success` (resolved).

**Density win:** Job spec, bounty, escrow state, and lifecycle phase all in one
element. No separate "Job Pipeline" panel.

#### 3. Edges (Animated Money Flow)

Two edges connect the three nodes: Alpha→Job and Job→Beta.

- **Static edge**: 1px `--border-soft` bezier curve.
- **Active edge**: when money moves, the edge thickens to 2px `--bone` and a
  particle (small circle, 4px, `--bone` with `--bone-bloom` glow) travels along
  the bezier path over 800ms.
  - `postJob`: particle flows Alpha→Job (money entering escrow).
  - `resolve(accepted)`: particle flows Job→Beta (payout).
  - `resolve(rejected)`: particle flows Job→Alpha (refund).
- **Assignment edge**: when Beta claims the job, the Job→Beta edge pulses
  `--rose` once (300ms), then settles to solid `--rose-dim` to show the connection.

**Density win:** Money movement is shown as actual visual motion along a path.
No "Escrow Bar" component needed — the escrow is the job node's fill, and the
flow is the particles.

#### 4. Validator Cluster

Appears to the right of Beta's node when job reaches Submitted state.
A rounded rectangle (~140×70px) connected to Beta by a horizontal edge.

Contains:
- **3 vote indicators**: three circles in a row (10px each).
  - Pending: outline `--text-ghost`.
  - Approve: filled `--success` with a "✓" inside.
  - Reject: filled `--warning` with a "✗" inside.
  - Each animates in with a 200ms scale-bounce when the vote event arrives.
- **Divider**: thin line below votes.
- **Verdict**: "APPROVED" in `--success` bold (12px) or "REJECTED" in `--warning`.
  "2/3" count in `--text-dim` right-aligned.
- **Cluster is hidden initially.** Fades in (opacity 0→1, 300ms) with the edge
  to Beta extending to reach it. Prevents visual clutter before it's relevant.

**Density win:** Validator identity is abstracted to just vote indicators.
No addresses needed in the flow viz — if someone asks "who are the validators?"
the chain evidence strip has the addresses. The viz shows *what happened*, not *who*.

#### 5. State Transition Animations (The Dopamine Sequence)

Each chain event triggers a coordinated animation on the canvas:

**postJob event:**
1. Alpha node border glows `--rose` (200ms).
2. Particle (bone) travels Alpha→Job along bezier (800ms).
3. Job node escrow bar fills left-to-right (400ms).
4. Bounty text "50 DAEJI" blooms with `--bone-bloom` glow (fades over 1.5s).
5. Job state label: "FUNDED".

**assign event:**
1. Job→Beta edge pulses `--rose` (300ms).
2. Beta node border glows `--rose` (holds while working).
3. Job state label: "ASSIGNED".

**submit event:**
1. Beta node border pulse intensifies briefly.
2. Job state label: "SUBMITTED".
3. Validator cluster fades in (300ms) with connecting edge.

**vote events (×2):**
1. Each vote circle fills with bounce (200ms).
2. On second approve (majority): brief flash on the entire cluster.

**resolve(accepted) — THE PAYOFF:**
1. Verdict "APPROVED 2/3" appears in cluster (200ms).
2. Job node escrow bar drains right-to-left (600ms).
3. Particle (bone + bright glow) travels Job→Beta along bezier (800ms).
4. Job outer border transitions to `--success` (400ms).
5. Beta reputation bar extends (old→new width, 800ms ease).
6. If tier changes: tier text crossfades ("Standard" → "Trusted").
7. Job state label: "RESOLVED" in `--success`.
8. Canvas background: very subtle green wash pulse (`rgba(138,156,134,0.04)`, 1s fade).

**Total payoff duration: ~2 seconds.** This is the moment. The presenter pauses.
Money visibly flows from escrow to the worker. Reputation grows. The room sees
cause and effect.

### Flanking Terminals (~20% each)

Narrow columns on left (Alpha/Poster) and right (Beta/Worker).

- **Width**: ~20% of viewport each. Enough for ~40 characters per line. Terminal
  output wraps naturally — that's fine, it's evidence.
- **Visual weight**: low. `--bg-void` background, mono text, thin `--border-soft`
  border. No color, no badges, no decorations.
- **Active indicator**: the terminal whose agent is currently acting gets a 2px
  left-border in `--rose-dim`. When the action shifts to the other agent, the
  border moves (100ms transition). The eye follows the border to the active terminal.
- **Scrolling**: auto-scroll to bottom. Newest output visible.
- **Header**: tiny mono label — "ALPHA · poster" / "BETA · worker" in `--text-ghost`
  (9px). Matches the agent names in the center viz so the connection is obvious.

**Right-to-exist test:** Terminals prove this is live, not simulated. The center
viz tells the story; the terminals show the raw execution. Together they answer
"is it real?" For a VC who asks "what's actually running?" the presenter points at
the terminal. Otherwise the eye stays on the center viz.

### Chain Evidence Strip

One line below the center viz. Compact proof of on-chain activity.

```
 14923 postJob +50 ● ─── 14924 assign ● ─── 14925 submit ● ─── 14926 vote ✓ ● ─── 14927 vote ✓ →PAID ●
```

- **Single horizontal row.** Connected by thin dashes. ~7 entries total for
  the entire demo. No scrolling needed.
- Each entry: block number (`--text-ghost`, 9px) + function name (`--text-dim`, 10px) +
  optional amount (`--bone`) + color dot:
  - `--rose` for fund movements
  - `--success` for state advances
  - `--bone` for votes
- New entries append from right with 150ms slide-in. Dot pulses once.
- On hover: tooltip shows full tx hash. Available but not needed during pitch.

**Density win:** This encodes block-level proof in one line. Replaces the
multi-row ChainActivityPanel from Demo B.

### Bottom Bar

Reuse `EfficiencyBar`. 4 cells. Only cost is in `--bone`. Everything else `--text-dim`.

```
 $0.18  │  3,241 tokens  │  claude-sonnet  │  1:47
```

### Color Semantics

| Meaning | Color | Where |
|---------|-------|-------|
| Money | `--bone` | Bounty text, escrow bar, flow particles, cost |
| Active | `--rose` | Active node border, active terminal border, active edge |
| Success | `--success` | Approve votes, APPROVED verdict, resolved border |
| Warning | `--warning` | Reject votes, slash |
| Pending | `--text-ghost` | Unfilled nodes, pending votes, future state labels |
| Evidence | `--text-dim` | Chain strip, addresses, block numbers, terminal text |

### Motion Budget

Every animation maps to a chain event. No ambient motion. The page is **still**
when nothing is happening.

| Chain Event | Canvas Animation | Duration |
|-------------|-----------------|----------|
| `postJob` | Particle Alpha→Job + escrow fills + bounty blooms | ~1.5s |
| `assign` | Edge pulse + Beta border glow | ~300ms |
| `submit` | Validator cluster fades in | ~300ms |
| `vote` | Vote circle fills with bounce | ~200ms each |
| `resolve` | Particle Job→Beta + escrow drains + rep bar grows + border→green | ~2s |
| `reputation` | Rep bar width animates + tier text crossfade | ~800ms |
| New tx | Chain strip entry slides in | ~150ms |

**Total unique animations: 7.** Each explains exactly one thing. No particles
for decoration. No ambient pulsing. No glow that doesn't represent state.

### Page Load State

The page loads still. No autoplay.

- Center viz: all nodes visible at full size but in "idle" state.
  Agent nodes show names + initial reputation. Job node shows spec title +
  bounty amount but escrow bar is empty and state label reads "WAITING" in
  `--text-ghost`. Edges are drawn as faint `--border-soft` paths.
  Validator cluster is hidden.
- Terminals: empty `$` prompts.
- Chain strip: ghost text "awaiting chain activity".
- Bottom bar: zeros.

The page looks like a blueprint — the structure is all there, every element
has a place, nothing is moving. When the demo starts, the blueprint comes alive.
The transition from still to animated is the first visual moment.

### No WebGL / Three.js

Canvas 2D only. The flow diagram is 5 elements with bezier edges — it doesn't
need a 3D scene graph. Canvas 2D renders crisply, has no blank-canvas risk,
matches the proven `KnowledgeFlowPanel` pattern, and is cheaper to maintain.

### Responsive (900px breakpoint)

Below 900px: center viz stacks above terminals (which go side-by-side).
The flow diagram reflows to horizontal (Alpha left → Job center → Beta right)
instead of vertical. Unlikely to be needed for the VC pitch but shouldn't break.

### Components

**New:**

| Component | File | What |
|-----------|------|------|
| **JobFlowViz** | `JobFlowViz.tsx` + `.css` | Canvas 2D center visualization. Draws all 5 elements, handles chain events, runs animations. ~350 lines. This is the main new component. |
| **ChainEvidenceStrip** | `ChainEvidenceStrip.tsx` + `.css` | Single-row horizontal tx log. ~80 lines. |

**Reused:**

| Component | From | Changes |
|-----------|------|---------|
| EfficiencyBar | Demo B | None |
| Pane | Shared | Wraps terminals |
| Terminal (showCmd) | Shared | None |

**Two new components, not four.** The flow viz replaces what was previously
JobHeroCard + AgentIdentityCard + ValidatorVerdictPanel + EscrowBar. Denser,
more connected, more visual.

---

## What Exists vs What Needs Building

### Exists and Works

| What | Where | Status |
|------|-------|--------|
| All 17 Solidity contracts | `contracts-core/packages/agents/` | Deployed, tested |
| Deploy scripts (Mirage, Local, Devnet) | `contracts-core/.../script/` | Working |
| Chain-coordination shell scripts | `demo-resources/chain-coordination/` | 5 scripts + e2e tests, all passing |
| Agent matchmaking API + scripts | `demo-resources/agent-matchmaking/` | seed, match, lifecycle, 40 e2e checks |
| Full self-hosting loop script | `demo-resources/full-self-hosting/` | End-to-end: capture→jobs→match→observe |
| Mirage ERC-8004 auto-bootstrap | `apps/mirage-rs/src/bootstrap.rs` | Contracts auto-deploy on mirage boot |
| roko job CLI | `crates/roko-cli/src/job.rs` | list/create/match/show/execute/cancel |
| HTTP job API (~11 routes) | `crates/roko-serve/src/routes/jobs.rs` | match, create, assign, start, submit, evaluate |
| Agent card publisher | `crates/roko-agent-server/src/registration.rs` | ERC-8004 on-chain update |
| ChainActivityPanel, EfficiencyBar | `demo/demo-app/src/components/` | Built for Demo B |
| useChainWs hook | `demo/demo-app/src/hooks/useChain.ts` | WebSocket subscription to mirage |
| common.sh (addresses, cast, RPC) | `demo-resources/chain-coordination/` | Battle-tested helpers |

### Needs Building

| What | Effort | Notes |
|------|--------|-------|
| `deploy-job-market.sh` | Small | Compose existing scripts: deploy + register + mint + pump reputation. Foundation already in `chain-coordination/01-*.sh`, `02-*.sh`. |
| Chain tools for jobs | Medium | `chain.post_job`, `chain.assign_job`, `chain.submit_result`, `chain.get_job`, `chain.list_open_jobs`. Same pattern as existing `chain.post_insight`. |
| JobPipelinePanel.tsx | Small | State machine visualization. 5 nodes, animate on WS events. |
| AgentCardPanel.tsx | Small | Two cards with reputation bars. Data from chain queries. |
| ValidatorPanel.tsx | Small | 3 rows + verdict. Animate checkmarks on vote events. |
| EscrowBar.tsx | Small | Single horizontal bar. Two numbers (locked/total). |
| useJobEvents hook | Small | Parse `job` channel WS events → typed state. |
| JobMarketPage.tsx | Medium | Wire terminals + sidebar components + chain events. Similar to existing Demo page. |
| Scenario definition | Medium | Phased command flow in `scenarios.ts`. Bridges HTTP matchmaking (Phase 1) with chain tools (Phase 2-4). |
| Validator auto-vote | Small | Script or background agent that watches `CommitteeAssembled` and auto-votes approve. |
| Mirage job WS channel | Medium | Emit `job` events on `/api/ws` for contract state transitions. May need log subscription in mirage-rs. |

### Three-Layer Gap (context, not a blocker)

Job infra exists in 3 disconnected layers:
```
Layer 1: File-backed (.roko/jobs/)     → CLI reads/writes
Layer 2: HTTP API (roko-serve)         → dashboard/matchmaking reads
Layer 3: Solidity contracts (on-chain) → BountyMarket escrow + settlement
```

Demo uses **Layer 2 for matchmaking** (find the right agent) then **Layer 3 for
settlement** (escrow, proof, payout). Layer 1 is bypassed. Full three-layer
bridge is a future wiring task, not needed for the demo.

---

## Chain Events (WebSocket)

New `job` channel on mirage `/api/ws`:

```json
{"channel":"job","data":{"type":"posted","id":0,"specHash":"0x7f2a...","bounty":"50000000000000000000","poster":"0x7099..."}}
{"channel":"job","data":{"type":"assigned","id":0,"worker":"0x3C44..."}}
{"channel":"job","data":{"type":"submitted","id":0,"resultHash":"0x8b3f..."}}
{"channel":"job","data":{"type":"vote","id":0,"voter":"0x90F7...","approve":true,"count":1}}
{"channel":"job","data":{"type":"vote","id":0,"voter":"0x15d3...","approve":true,"count":2}}
{"channel":"job","data":{"type":"resolved","id":0,"accepted":true,"payout":"50000000000000000000"}}

{"channel":"agent","data":{"type":"registered","address":"0x7099...","capabilities":"research,analysis,planning"}}
{"channel":"agent","data":{"type":"reputation","address":"0x3C44...","old":500000,"new":600000,"tier":"Trusted"}}
```

Subscribe: `ws://localhost:8545/api/ws?jobs=true&agents=true`

---

## Narrator Script (3.5 minutes)

### Beat 1 — Setup the Problem (10s)
*"How do you coordinate autonomous agents without a central dispatcher?
The same way humans coordinate without a boss: a market."*

### Beat 2 — Matchmaking (20s)
*"Agent Alpha needs research done. It asks: who's qualified? Roko's matchmaking
engine scores every registered agent by skill overlap, reputation, and availability.
Beta is the best fit — Trusted tier, 600k reputation, DeFi specialist."*

[Left terminal: `roko job match` output. Sidebar: agent cards highlight.]

### Beat 3 — Escrow (15s)
*"Alpha posts the job with 50 tokens locked in a smart contract. Not a promise —
actual escrow. Neither party can cheat."*

[Left terminal: postJob tx. UI: Pipeline card appears at Funded. Escrow bar fills.]

### Beat 4 — Execute (60s)
*"Beta claims the job and starts working. This is a real Claude API call doing
real research. Watch the output stream in."*

[Right terminal: agent claims, researches, writes report. UI: Pipeline transitions
to Assigned. Agent card shows "Working on Job #0".]

### Beat 5 — Prove + Validate (30s)
*"Beta hashes the report and posts the proof on-chain. A committee of three staked
validators votes. Two approve — the smart contract automatically releases the bounty.
50 tokens to Beta, plus a 20% reputation boost."*

[Right terminal: submit tx. UI: Validator panel animates votes. Pipeline cascades to
Terminal. Escrow empties. Reputation bar fills.]

### Beat 6 — Compound (15s, if time)
*"Now Alpha posts a harder job requiring Trusted tier. Beta just earned that from
Job #0. The market self-organizes: better work, better reputation, better jobs."*

[Left terminal: new postJob. Right terminal: Beta qualifies. UI: second pipeline card.]

### Beat 7 — Close (10s)
*"No human dispatched anything. The matchmaking found the right agent, the chain
handled escrow, the validators verified quality, and reputation compounded. This
is autonomous agent coordination at production scale."*

---

## Implementation Order

1. **Deploy script** — Compose `chain-coordination/` scripts into single `deploy-job-market.sh`
2. **Chain tools** — `chain.post_job`, `chain.assign_job`, `chain.submit_result`, `chain.get_job`
3. **Mirage WS job channel** — Emit `job` events from contract log subscription
4. **Validator auto-vote** — Background script watching for `CommitteeAssembled`
5. **UI components** — JobPipelinePanel, AgentCardPanel, ValidatorPanel, EscrowBar
6. **Scenario + page** — Wire into demo-app: `jobMarket` scenario in `scenarios.ts`, `JobMarketPage.tsx`
7. **Polish** — Timing, animations, error recovery, rehearsal
