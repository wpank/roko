# V8-V17 Demo Spec — Nunchi Series A Interactive Presentation

## What You're Building

A single-file HTML interactive presentation for Nunchi's Series A VC pitch. Full-page atmospheric visuals, minimal impactful text, arrow-key navigation between pages. Built with Three.js (imported via importmap from unpkg), ROSEDUST design system, and optional live blockchain data from local mirage devnet.

## CRITICAL BUG FIXES (from V7)

The V7 navigation had TWO bugs that MUST be fixed:

1. **DOMTokenList empty string bug:** `oldPage.classList.add(goingForward ? 'exit-up' : '')` — adding empty string to classList throws. Fix: only add class if the string is non-empty.

2. **Transition lock freeze:** If any error occurs during transition, `transitioning` stays `true` forever, freezing navigation. Fix: wrap transition in try/catch, and use setTimeout to force-release the lock as a safety net.

**Correct goToPage pattern:**
```javascript
function goToPage(idx) {
  if (idx === currentPage || idx < 0 || idx >= TOTAL_PAGES || transitioning) return;
  transitioning = true;

  try {
    const oldPage = document.getElementById('page' + currentPage);
    const newPage = document.getElementById('page' + idx);
    if (!oldPage || !newPage) { transitioning = false; return; }

    const goingForward = idx > currentPage;

    oldPage.classList.remove('active');
    if (goingForward) oldPage.classList.add('exit-up');

    newPage.style.transform = goingForward ? 'translateY(24px)' : 'translateY(-24px)';
    newPage.style.opacity = '0';

    requestAnimationFrame(() => {
      newPage.classList.add('active');
      newPage.style.transform = '';
      newPage.style.opacity = '';
    });

    document.querySelectorAll('.nav-dot').forEach((d, i) => {
      d.classList.toggle('active', i === idx);
    });

    setTimeout(() => {
      if (oldPage) oldPage.classList.remove('exit-up');
      transitioning = false;
    }, 450);

    currentPage = idx;
    onPageEnter(idx);
  } catch(e) {
    console.warn('Navigation error:', e);
    transitioning = false;
  }
}
```

## Page Structure (7 pages, arrow-key navigated)

### Page 0 — Hero
**"Nunchi · Agent Coordination Plane"**

NO numbers. NO statistics. Just the name and the one-sentence thesis. Full-screen atmospheric Three.js background.

Content:
- Title: "Nunchi" (large, italic serif)
- Subtitle: "Agent Coordination Plane" (mono, uppercase, spaced)
- One line: "The infrastructure layer between AI models and the applications they power."
- Bottom: "Series A · 2026" (very small, mono)

### Page 1 — The Plane Between
**"The plane between the planes."**

Show three stacked layers with Nunchi in the middle:

```
APPLICATION LAYER
  LangChain · CrewAI · Mastra · AutoGen · Orkes · AGENTS.md

═══ AGENT COORDINATION PLANE ═══
  Nunchi — identity, routing, knowledge, policy, settlement

EXECUTION LAYER
  Temporal · Keycard · x402 · ERC-8004 · MCP · A2A · Model APIs
```

Visual: Three horizontal bands/layers, floating in space. The middle one glows. Data flows from top through middle to bottom. Nunchi sits in the middle like a nervous system.

Text: "Every adjacent layer is funded. The plane between them is empty. Nunchi fills it."

### Page 2 — Two Components, One Architecture
**"Two components. One architecture."**

Side-by-side split panel:

LEFT — **Roko · Runtime** (open-source, Apache 2.0)
- The cognitive runtime that turns a model call into a governed event
- 18 crates, 177K lines of Rust
- CascadeRouter — cost-aware model routing
- 11 gates, 7 rungs between every call
- NeuroStore — HDC-indexed knowledge
- Deterministic replay

RIGHT — **Nunchi Chain · Verifiable Substrate**
- Agent-native sovereign EVM L1
- Built on Commonware — the open-source Rust blockchain anti-framework ($9M, Haun + Dragonfly)
- Simplex BFT — ~50ms blocks, single-slot finality
- HDC precompile at 0x09 — native semantic search at consensus layer
- ISFR Oracle — DeFi benchmark computed by validators
- Knowledge demurrage — exponential decay, confirmations extend half-life

Bottom text: "Roko runs alone. The chain runs alone. Together, local traces compound into shared memory, attestable reputation, and verifiable settlement across trust boundaries."

### Page 3 — Chain Architecture (Built on Commonware)
**"Built on Commonware."**

Emphasize Commonware heavily. Show the chain as layers/components:

Visual: Stacked layers or exploded 3D diagram showing:
1. **Consensus (Simplex BFT)** — ~50ms blocks, 2 hops notarize, 3 hops finalize
2. **Execution (revm)** — full EVM compatibility, Block-STM parallel execution
3. **Three Native Precompiles:**
   - Agent Registry (0x08) — on-chain agent identity
   - HDC Precompile (0x09) — semantic search at consensus layer
   - ISFR Oracle (0xA01) — DeFi rate computation
4. **Knowledge Layer** — InsightEntry storage, Merkle tree, demurrage
5. **P2P (authenticated)** — encrypted validator communication

Key text about Commonware:
- "Not a framework. An anti-framework. Pick-and-choose Rust primitives."
- `p2p::authenticated` · `cryptography::bls12381` · `runtime::deterministic` · `consensus::simplex`

Block time comparison table (visual):
- Ethereum: 12s
- Solana: 400ms
- Sui: ~390ms
- **Nunchi: ~50ms** (highlight dramatically)

### Page 4 — HDC Interactive
**"Hyperdimensional Computing"**

Step-by-step interactive walkthrough. User clicks/advances through stages:

**Step 1: An agent finds a Uniswap transaction**
```
swap(USDC → WETH, 50,000 USDC, pool 0x88e6...)
gas: 142,891 · block: 19,847,221
```

**Step 2: Encode to hypervector**
Each concept gets a random 10,000-bit base vector, then:
- Bind: swap ⊕ USDC ⊕ WETH = transaction identity
- Permute: shift for sequence (position in episode)
- Bundle: majority vote combines into single episode vector

Visual: Show bits flowing, XOR operations, the vector forming

**Step 3: Post to chain**
InsightEntry { content, hypervector (1,250 bytes), decay params }
→ Transaction → Validator → sm_root updated

**Step 4: Future agent searches**
Query: "Uniswap USDC pool large swap"
→ Encode query to hypervector
→ Hamming distance search across all entries
→ Top-K results in 0.17ms (vs 12ms for float embeddings)

**Step 5: Results compound**
10 confirmations = 6x half-life. Knowledge many agents verify becomes long-lived.

### Page 5 — ISFR: Agent-Native Finance
**"The first of many."**

ISFR = Internet Secured Funding Rate. The first financial instrument native to agent coordination.

**What it is:** A composite DeFi benchmark rate, like SOFR for the internet economy. Computed by validators every 10 seconds.

**Four source classes (visual flow):**
- LENDING (60% weight) — Aave, Compound, Spark, Morpho
- STRUCTURED (25%) — Pendle, Spectra, Term Finance
- FUNDING (10%) — dYdX, Hyperliquid, GMX
- STAKING (5%) — Lido, Rocket Pool, Coinbase

**Visual:** Sources flow into agent nodes, agents flow into chain, chain publishes rate.

**Why it matters:**
- The $668 trillion gap: traditional finance has SOFR. DeFi has nothing comparable.
- "First of many" — ISFR proves the pattern. Next: volatility indices, credit spreads, cross-chain rates.
- Each new index = new revenue line, same infrastructure.

### Page 6 — Collective Intelligence
**"Every agent makes every agent smarter."**

Interactive network visualization with slider (5 → 1000 agents).

**Visual:** Agents as glowing nodes. Knowledge packets flowing between them. As slider increases:
- More connections form
- Knowledge flow accelerates
- Shared insights compound
- Innovation rate goes superlinear (β ≈ 1.15 — when agents double, output grows 115%)

**The stigmergy analogy:** Like ants leaving pheromone trails. Each agent's knowledge deposit strengthens paths for future agents. Demurrage = pheromone evaporation. Confirmation = reinforcement.

**Key text:** "The thousandth agent pulls 30,000 tokens of context the first agent never had."

If mirage is running: show live blocks appearing as pulses through the network.

## ROSEDUST Design Tokens

```css
:root {
  /* Void spectrum */
  --void: #050507;
  --void-2: #08080c;
  --raised: #0c0a0e;
  --mid: #080810;

  /* Borders */
  --border: #181420;
  --border-live: rgba(170,112,136,0.32);

  /* Rose */
  --rose: #aa7088;
  --rose-hot: #cc90a8;
  --rose-neon: #dca5bd;
  --rose-dim: #7a5060;
  --rose-deep: #3a2030;

  /* Bone */
  --bone: #c8b890;
  --bone-hot: #e6d4a6;
  --bone-dim: #8a7a5a;

  /* Dream */
  --dream: #585878;
  --dream-bright: #7878a0;

  /* Accents */
  --jade: #70887a;
  --jade-hot: #9ab5a3;
  --warn: #aa8855;

  /* Text */
  --t1: #988090;
  --t-dim: #584858;
  --ghost: #302830;

  /* Fonts */
  --mono: "JetBrains Mono", ui-monospace, monospace;
  --serif: "Fraunces", serif;
}
```

**Typography:**
- Headlines: `font-family: var(--serif); font-style: italic; font-weight: 300; color: #d8c8d0;`
- Rose emphasis: `color: var(--rose-neon); text-shadow: 0 0 20px rgba(220,165,189,0.3);`
- Body: `font-family: var(--mono); font-weight: 400; font-size: 14px; color: var(--t1);`
- Section tags: `font-size: 10px; text-transform: uppercase; letter-spacing: 0.28em; color: var(--rose-dim);`

**Fonts import:**
```html
<link href="https://fonts.googleapis.com/css2?family=Fraunces:ital,opsz,wght@0,9..144,300;0,9..144,400;0,9..144,500;1,9..144,300;1,9..144,400&family=JetBrains+Mono:ital,wght@0,300;0,400;0,500;1,400&display=swap" rel="stylesheet">
```

**Three.js import:**
```html
<script type="importmap">
{ "imports": { "three": "https://unpkg.com/three@0.160.0/build/three.module.js" } }
</script>
```

**Atmospheric overlays (always include):**
- Film grain (position: fixed, z-index: 9998, opacity: 0.03, mix-blend-mode: overlay)
- Scanlines (repeating-linear-gradient, opacity: 0.05)
- Vignette (radial-gradient from transparent to rgba(0,0,0,0.65))

**Layout rules:**
- `html, body { overflow: hidden; width: 100%; height: 100%; }` — NO scrollbars
- Pages: `position: absolute; width: 100%; height: 100%;`
- Visuals should fill the entire viewport
- Text overlays sit on top of full-page visuals with glass/chassis panels

## Chassis Panel Pattern
```css
.chassis {
  background: linear-gradient(180deg, #0c0a0e, #080810);
  border: 1px solid #1a1622;
  border-radius: 2px;
}
.chassis .head {
  display: flex; justify-content: space-between; align-items: center;
  padding: 12px 20px;
  border-bottom: 1px solid #1a1622;
  background: rgba(6,6,8,0.5);
  font-family: var(--mono);
  font-size: 10px; letter-spacing: 0.22em; text-transform: uppercase;
  color: #7a6a78;
}
.chassis .head .led {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--jade);
  box-shadow: 0 0 6px var(--jade), 0 0 12px rgba(112,136,122,0.3);
  animation: ledPulse 2s ease-in-out infinite;
}
```

## Mirage Connection (Optional Enhancement)

If mirage devnet is running at `http://127.0.0.1:8545` (chain ID 88888), poll for blocks and use them to modulate visuals. Blocks should be visually apparent — pulses, flashes, data updates.

```javascript
let blockNumber = 0;
let blockModulation = 0;

async function pollBlocks() {
  try {
    const r = await fetch('http://127.0.0.1:8545', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', method: 'eth_blockNumber', params: [], id: 1 })
    });
    const d = await r.json();
    const n = parseInt(d.result, 16);
    if (n > blockNumber) {
      blockNumber = n;
      blockModulation = 1.0;
      // Trigger visual pulse across all pages
      onNewBlock(n);
    }
  } catch(e) { /* mirage not running, ignore */ }
}
setInterval(pollBlocks, 1000);

// Decay modulation smoothly
setInterval(() => { blockModulation *= 0.92; }, 50);
```

When new blocks arrive, make them VISUALLY APPARENT:
- Pulse the border of the current visualization
- Flash the block number in a HUD element
- Briefly brighten particle systems
- Show block hash fragments flowing through the scene

## Navigation Pattern

Bottom-center dots + arrow keys + click arrows. Must work flawlessly.

```html
<nav class="page-nav">
  <div class="nav-arrow left" id="navLeft">&larr;</div>
  <div class="nav-dots" id="navDots"></div>
  <div class="nav-arrow right" id="navRight">&rarr;</div>
</nav>
```

Navigation dots generated in JS. Current page highlighted. Keyboard: ArrowRight/Space = next, ArrowLeft = prev.

## Quality Checklist

- [ ] No scrollbars (overflow: hidden on html and body)
- [ ] No JS errors in console (especially no DOMTokenList errors)
- [ ] Arrow keys never freeze or go black
- [ ] All 7 pages render with full-page visuals
- [ ] Three.js scenes properly initialized/cleaned up on page transitions
- [ ] Text is concise, impactful — every word deserves to be there
- [ ] ROSEDUST colors used throughout (no generic blues/greens)
- [ ] Atmospheric overlays present (grain, scanlines, vignette)
- [ ] Fonts loaded (Fraunces + JetBrains Mono)
- [ ] Mirage connection gracefully degrades (no errors if not running)
- [ ] Visuals fill entire viewport
- [ ] Mobile-friendly font sizing with clamp()
