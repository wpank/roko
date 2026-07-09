# PRD: Nunchi Landing Page · Architecture Explorer · TUI Demo

| Field | Value |
|-------|-------|
| Author | Will (via Claude iteration) |
| Date | 2026-04-21 |
| Status | v0.1 draft — ready for implementation |
| Scope | Marketing site + interactive architecture dashboard + embedded end-to-end demo |
| Target deploy | Railway |
| Target persona | Technical VC, 10-minute demo |
| Reference aesthetic | ROSEDUST ("terminal existentialism") ported to web |
| Reference IA | ccunpacked.dev — clickable, self-guided, dense but legible |

---

## 0. Reader orientation

This document specifies a new website for Nunchi (Roko + Korai). It is NOT a replacement for the TUI; it is a public-facing surface whose job is to take a sophisticated VC from zero context to conviction in ≤10 minutes while being interesting enough that they want to click around afterward.

The site has three surfaces:

1. **Landing / narrative** — scrollytelling, boot-sequence opening, heavy interactive set pieces. This is the demo. This is what gets screenshared.
2. **Architecture explorer** — clickable graph of subsystems (status-colored: working / built-disconnected / not-yet-built). For the curious/technical reader who wants to drill in.
3. **End-to-end demo** — a real (or realistic-feeling) run of a coding agent, embedded via xterm.js or a pre-baked replay, showing the loop in action on a concrete task.

A working prototype exists at `/mnt/user-data/outputs/rosedust-prototype.html` — single file, CDN-only. It proves the aesthetic, the boot sequence, and three of the six interactive set pieces. The full build should replace it with a Next.js 14+ (App Router) codebase deployed to Railway.

---

## 1. Strategic framing

### 1.1 What a VC should walk away with

**Moat-first.** After 10 minutes, the VC should be able to explain to a partner:

1. "Roko is an agent runtime that compounds. Every invocation, it gets smarter and cheaper. Competitors start from scratch every time."
2. "Korai is a purpose-built chain that makes compounding a network effect. The thousandth agent inherits everything the first 999 learned."
3. "First application is yield perpetuals settled against ISFR — the missing benchmark rate for DeFi. $668T TradFi market, <$100M on-chain. The gap is six orders of magnitude and it's structural."
4. "Nobody builds at the intersection of persistent-learning agents + distributed-economic coordination because you have to build both at once. That's the empty quadrant."

Market and tech serve the moat narrative. They are proof points, not top-line messages.

### 1.2 What the page must NOT do

- Open with a dense tech dump. The VC did not come here for an architecture lecture.
- Look like every other AI infra site (purple gradients, white background, Inter, stock photos of glowing brains). ROSEDUST is the moat signal before they've read a word.
- Use bullet-point feature lists. Bullets signal "we made this with a generator." Prose + ASCII + typography signal taste.
- Get cute at the expense of clarity. The aesthetic must earn its complexity by being legible. If a VC gets lost, the aesthetic failed.

### 1.3 Pacing budget (10-minute demo)

| Time | Section | Goal |
|------|---------|------|
| 0:00–0:30 | Boot sequence + hero | Aesthetic hook. They think "this is different." |
| 0:30–2:00 | Moat section | "Session #1000 > session #1. The scaffold is the product. Here's evidence." |
| 2:00–3:30 | The Loop (6 phases) | "Five of these phases don't exist anywhere else. Watch what each one does." |
| 3:30–5:00 | Cognitive Gating demo | Slide prediction error. Watch 80% of ticks cost $0. |
| 5:00–6:30 | VCG auction | Click bidders. Watch the context window reshape. |
| 6:30–8:00 | Network effects slider | Drag from 1 → 1,000 agents. Watch costs drop, gate-pass rise. |
| 8:00–9:30 | End-to-end demo (xterm.js) | Real coding agent: PRD → plan → fail → replan → pass → knowledge deposited. |
| 9:30–10:00 | Outro + ask | Two buttons: "Run it yourself" + "Read the PRDs." |

Total: 10 minutes. If they're engaged, they'll scroll into the architecture explorer afterward on their own.

---

## 2. Design system: ROSEDUST on web

### 2.1 Palette (mandatory — do not substitute)

```
BASE / VOID
  bg_void       #060608   (deepest — violet-undertone, never #000)
  bg_raised     #0c0a0e
  bg_mid        #080810
  bg_warm       #0a0808   (used in "degraded" states)
  border        #181420
  border_active rgba(170,112,136,0.27)
  border_dream  rgba(88,88,120,0.27)

ROSE SPECTRUM (80% of visible color must be rose or variant)
  rose          #aa7088   — primary text emphasis, headers, active data
  rose_bright   #cc90a8   — alerts, danger, glow. Always paired with phosphor bleed.
  rose_dim      #7a5060
  rose_deep     #3a2030
  rose_ember    #482838

BONE (THE ONE NUMBER — used once per screen max)
  bone          #c8b890   — most important element on any screen
  bone_dim      #8a7a5a

TEXT
  text_primary  #988090   — standard readable
  text_dim      #584858
  text_ghost    #302830   — barely visible philosophical fragments
  text_phantom  #201820   — subliminal

SEMANTIC
  dream         #585878   (indigo — dream state, Wired, replaces rose occasionally)
  warning       #aa8855
  success       #70887a   — muted sage, never celebratory

CRT MATERIALITY
  scanline_dark #050507
  phosphor_res  #1a1018
  bleed_rose    rgba(170,112,136,0.09)
```

**Rules (non-negotiable):**
1. 80% rose-dominant on any visible screen.
2. Bone appears once per screen, max. If nothing is critical, bone does not appear.
3. Brightest element is `rose_bright`. Never white.
4. Background is `#060608`, not `#000000`. Pure black is a hole.
5. Everything fades. Nothing snaps. Transitions are always gradual.

### 2.2 Typography

- **Display:** `Fraunces` (variable, optical-sized, italic variants) — for headlines, thesis statements, the serif-italic moments that break up the mono grid.
- **Mono / body:** `JetBrains Mono` (ideal: Berkeley Mono if licensed — it's closest to the intended terminal aesthetic). Never Inter. Never Arial. Never system.
- **Never use sans-serif anywhere unless it's absolutely necessary for UX.** The site is fundamentally a terminal that knows it's a website.

### 2.3 CRT atmosphere layers (always on, non-interactive)

Apply these as fixed-position overlays with `pointer-events: none`:

1. **Scanlines** — `repeating-linear-gradient`, 2px/3px alternation, `opacity: 0.35`, `mix-blend-mode: multiply`, z-index just above content.
2. **Grain** — SVG `feTurbulence` noise, `opacity: 0.08`, `mix-blend-mode: overlay`.
3. **Vignette** — radial gradient, darker at edges, subtle rose tint in upper-left, indigo tint in lower-right.
4. **Flicker** — body-level keyframed `opacity` with sparse dips (every 4-8s), never enough to be annoying.
5. **Phosphor bleed** — text-shadow on all rose_bright and bone elements, glow-scaled to letter height.
6. **Hair-line scroll progress bar** — 1px rose_bright at top of viewport, `box-shadow: 0 0 6px rose_bright`.

All six layers are specified in the prototype's CSS. Port directly.

### 2.4 Motion principles

- **Nothing snaps.** Every transition has an easing curve, minimum 300ms unless it's a hover micro-state.
- **Subtle parallax** on scroll — Three.js camera y offset keyed to `scrollY * 0.004`.
- **Auto-advance** on interactive components when the section comes into view (e.g. the Loop section cycles through phases every 3.5s until the user clicks). On first interaction, auto-advance stops — forever for that session.
- **Reveal on scroll** — `IntersectionObserver`, translateY(20px) → 0, opacity 0→1, 1s duration.
- **Cursor blinks** at the end of the boot sequence — 1s `step-end` infinite.
- **Flicker** is a 8s loop with irregular dips. CRT flicker is never regular.

### 2.5 Layout grammar

- Full-bleed sections, 120px top padding, 80px bottom, 8vw horizontal padding. Sections are stacked `min-height: 100vh`.
- **Section labels:** tiny, all-caps, letter-spaced, preceded by a numerical index ("01 · THE MOAT", "02 · THE LOOP"). Always rose_dim.
- **Headlines** break out of the grid. Italic serif (Fraunces) for key emphasis. Max-width ~20ch for h1, ~22ch for h2.
- **Body copy** stays within 62ch for readability. Lead paragraphs (short, rose-colored, italicized moments) provide rhythm between dense content.
- **Data blocks** are grids of hairline-separated cells (1px border in `--border`), each cell: tiny uppercase label on top (`rose_dim`, letter-spaced), bigger value below (`rose_bright` or `bone`).
- **Interactive components** are framed in 1px borders with matte background (`bg_raised`) against the void (`bg_void`). They always feel like containers on a surface, never floating.

---

## 3. Information architecture

### 3.1 Route structure

```
/                    — Landing page (the 10-min VC journey)
/architecture        — Full subsystem explorer (graph view)
/demo                — Embedded TUI (xterm.js) running a live coding-agent workflow
/prd/:slug           — PRD reader (markdown-rendered) — lightweight, for deep dives
/isfr                — ISFR oracle page (ticker, methodology, validators) — future
```

All routes share the CRT overlay layers and the top nav. The Three.js background on `/` is route-specific; other routes use lighter ambient backgrounds (see §5).

### 3.2 Top nav (fixed, all routes)

```
NUNCHI · ROKO · KORAI                  v0.177k · 29 crates · uptime 847:22:15
```

- Left: wordmark with dot separators. Rose. Bone on dots (only place bone appears in chrome).
- Right: live-feeling meta. The uptime is actually-ticking. The crate count is real.
- On scroll, the nav gets a `backdrop-filter: blur(4px)` and a gradient-to-transparent bottom.

### 3.3 The scroll arc on `/`

Sections are numbered and appear in order:

```
00   HERO                    — Boot sequence → hero h1 → hero meta grid
01   THE MOAT                — Thesis 1 + Thesis 2, compounding chart
02   THE LOOP                — 6-phase walkthrough with auto-advance
03   COGNITIVE GATING        — T0/T1/T2 demo, prediction error slider
04   LEARNABLE CONTEXT       — VCG auction, 8 bidders, reshaping window
05   STIGMERGIC COLLECTIVE   — Network effects slider, HDC playground
06   KORAI · ISFR            — Brief: chain arch, ISFR tick, yield perps market gap
07   THE DEMO                — xterm.js embed, coding agent runs end-to-end
∞    COROLLARY + CTA         — Outro, two CTAs, footer
```

Each section is ≥90vh. Scroll progress bar ticks across the top.

---

## 4. Sections — content + interaction spec

This is where implementation goes deep. For each section: narrative, copy cues, required interactions, data/animations.

### 4.1 Hero (§00)

**Three.js scene:** HDC vector field. ~2500 particles in rose-spectrum colors, distributed in a slightly flattened sphere. Custom shader — additive blending, soft round glow, pulse keyed to elapsed time. Every ~280ms, a "bind" event fires: a random particle links to a nearby neighbor with a faint rose line, visible for ~1.5–3s before fading. This visualizes HDC bind/bundle at the aesthetic level — the viewer doesn't need to know what they're seeing to feel it.

Camera: slowly rotates (0.04 rad/s on Y, gentle X wobble). Y-position keyed to `window.scrollY` for parallax.

Fog: `FogExp2(0x060608, 0.035)` — particles fade into void at distance.

**Sigil block** (top of section, pre-h1):

```
┌─ ROSEDUST // TERMINAL.EXISTENTIAL ─────────────────┐
│                                                    │
│   the scaffold is the product.                     │
│   the network is the moat.                         │
│                                                    │
└────────────────────────────────────────────────────┘
```

Font: JetBrains Mono, 11px, rose_dim. Animated fade-up on load (300ms delay).

**Headline (h1):** "Every other agent framework is *amnesiac.*" — 300-weight Fraunces, "amnesiac" in italic rose_bright with phosphor glow. Staggered load: 600ms delay.

**Sub copy:** 52ch max, text_primary. One sentence per idea. Names "Roko" and "Korai" explicitly. Ends with: "Session #1000 starts smarter than session #1. Agent #1000 inherits everything the first 999 discovered."

**Hero meta grid** — 4 cells, hairline-separated, each with uppercase label + value + unit:

```
SCAFFOLD DELTA (SWE-BENCH)   +22.0 pts
TRADFI ÷ DEFI GAP            6 orders of magnitude
T0/T1 ROUTING AT LAUNCH      ~80% zero-LLM
COLLECTIVE CTX GAIN          +78% same model
```

### 4.2 The Moat (§01)

**Narrative:** Frame the incumbent assumption ("model > scaffold"), demolish it with data, introduce the two theses.

**Copy structure:**

- h2: *"The hundredth time a familiar error arrives, it should cost pennies. Not dollars."*
- Lead paragraph: introduce the wasteful incumbent loop.
- Body paragraph: SWE-bench + Meta-Harness + Cursor-vs-Opus evidence.
- **Axiom block 1** — "Thesis · One" label, italic Fraunces claim, rose_bright corollary.
- **Axiom block 2** — "Thesis · Two" label, same treatment.
- **Compounding chart** — ASCII-rendered sparkline showing divergence between compounding (rose_bright) and linear (text_dim) curves over 100M invocations. Use Unicode block characters: `▁▂▃▅▇█`. The chart is illustrative, not real data — label the x-axis "invocations", y-axis "performance / session".

**No animations beyond scroll reveal.** This is the readable section. The VC is catching their breath between Three.js-heavy sections.

### 4.3 The Loop (§02)

**Three.js background:** REPLACE the hero HDC field with a new scene — or rather, transform the same particle system. Particles now form six loose clusters (one per loop phase), connected by thin rose filaments that pulse in sequence (OBSERVE → GATE → ASSEMBLE → …). This is the same geometry, re-arranged. Transition takes ~3s triggered on scroll-into-view.

*Implementation note: easier to do this as two separate scenes swapped via opacity crossfade than to genuinely morph the particles.*

**Interactive:** 6-column grid of clickable stage cards. Each card has:
- Tiny number (01–06)
- Stage name (all caps, letter-spaced)
- Cost indicator (`μs · $0.00` to `s · $$`)

Below the grid: a detail panel that renders the currently-selected stage:
- Title (Fraunces, 28px, rose_bright)
- Subtitle (12px caps, text_dim)
- Body copy (~70ch, text_primary)
- Tag pills: rose-deep borders for normal, bone-dim borders for the one bone tag per stage

**Auto-advance:** cycles every 3.5s when section in view. Stops on first user interaction.

**Content for all 6 stages:** already written in the prototype (`loopData` array). Port directly.

### 4.4 Cognitive Gating Demo (§03)

**The hero interactive.** This is where the VC gets their "oh" moment.

**Layout:** 2-column (1fr 1.2fr), stacks on mobile.

**Left column — controls:**
- Label: "PREDICTION ERROR"
- Large display value (32px rose_bright with phosphor glow): `0.18`
- Gradient track slider (0 → 100), track is `linear-gradient(to right, success 0%, warning 60%, rose_bright 100%)`. Thumb is a glowing rose_bright square.
- Labels below slider: "FAMILIAR" / "NOVEL"
- Beneath, a tiny probe signal list: "memory hit rate · tool novelty · context divergence · task complexity · gate pass rate · rate volatility · ...11 more"

**Right column — tier distribution:**
- Three rows: T0 / T1 / T2
- Each row: tier name (rose) + desc + bar chart + cost-per-tick
- Bar fills animate with `cubic-bezier(0.22, 1, 0.36, 1)` on slider change
- The "active" (winning) tier gets a border glow and fill gradient brightens to rose_bright

**Below both columns — summary box:**
- "BLENDED COST / 1000 TICKS" — big value (Fraunces 36px bone with glow) — e.g. `$2.77` followed by `vs $48.00 always-frontier` in text_dim
- "REDUCTION" — e.g. `17.3×` in bone

**Math:**
```
pe = slider / 100
t2 = min(0.95, pe ^ 1.7)
t1 = (1 - t2) * (0.2 + pe * 0.4)
t0 = 1 - t1 - t2
blended = (t0 * 0.000 + t1 * 0.002 + t2 * 0.048) * 1000
reduction = 48.00 / max(blended, 0.01)
```

At slider=18: t0≈78%, t1≈17%, t2≈5%, blended≈$2.77, reduction≈17.3×. This is the landing state.

**Subtle addition:** as the slider moves, a small "tick ticker" in the corner of the panel counts up real-time, showing the 16 probes firing. Purely aesthetic — reinforces that this runs constantly, not on demand.

### 4.5 VCG Auction (§04)

**Three.js background:** continues from the Loop section. For this section, the particles re-cluster into 8 tight groups representing bidders, each in its assigned color. Camera pulls back.

**Top element — the context window:**
- Full-width horizontal bar, 36px tall, 1px border in `--border`, divided into 8 colored slices
- Each slice has `flex-grow` keyed to its allocated tokens
- Slice colors match the bidder colors below (rose, rose_dim, rose_deep, bone_dim, rose_ember, dream_dim, rose_bright, dream)
- Above the bar: "CONTEXT BUDGET · 32,000 tokens" (bone for the number, per once-per-screen rule — this is the most important number in the section)

**Below — 8 bidder cards** in a responsive grid (`minmax(220px, 1fr)`):

```
┌─────────────────────────┐
│ NEURO               ■   │   name (rose) · color dot
│ knowledge store         │   desc (dim, small)
│ bid: 847                │   bid (rose_bright)
│ WON  10,880 tok         │   allocated tokens (bone)
└─────────────────────────┘
```

Winning bidders get `border-color: var(--border-active)` and a subtle box-shadow glow.

**Animation:** every 2.4s (when section in view), bids perturb by 0.7×–1.3× multiplier, window reallocates. The VCG rule is: each bidder pays the second-highest price, so truth-telling is dominant. Show this in a small footnote under the window: "bidders pay the 2nd-highest price · truth-telling is optimal."

**Educational hover:** hovering a bidder card shows a tooltip explaining what it bids on. E.g. Neuro: "knowledge entries retrieved via HDC similarity. Bids high when task matches stored episodes."

### 4.6 Stigmergic Collective (§05)

Two interactives stacked. This is the section that proves network effects.

#### 4.6.1 Agent-count slider

- Large single slider, 1 → 1000 agents. Track is 1px rose_deep. Thumb is a 24px tall rose_bright bar with glow.
- Label-count display to the right of the slider: rose/bone, big.
- Below: 4-cell stats grid (same treatment as hero meta):
  - `InsightStore entries` — floor(n * 120 + n^1.4 * 8). Super-linear.
  - `Cross-domain transfers / day` — floor(n * log2(n) * 2.2).
  - `Per-agent gate pass %` — 41 + (78 - 41) * (1 - exp(-n/180)). The bone cell.
  - `Cost per episode` — 0.94 * exp(-n/320) + 0.11.

- Beneath: ASCII grid of filled/empty dots representing agents. 80 cols × 12 rows = 960 cells. Filled cells (circles of various styles: ◉○●◎∘) at positions < n, empty cells (`·` in phantom color) beyond. Each slider tick re-randomizes character choice for a "living" feel.

#### 4.6.2 HDC similarity playground (NEW — not yet in prototype)

**The component:** two text inputs side by side:
- Left: "coding domain" input (placeholder: "verify before commit")
- Right: "trading domain" input (placeholder: "verify before position")

Below each input, a visual representation of the encoded HDC vector — a 64×32 grid of cells colored by bit value (lit rose cells = 1, dark cells = 0). 2,048 cells visualizes a compressed view of the 10,240-bit vector. Deterministic hash of input string drives the bits, so typing the same text always produces the same pattern.

Between/below the two: a **Hamming distance readout** (bone), a similarity score (cosine-equivalent), and a verdict:

```
HAMMING DISTANCE        847 / 10,240   (8.3%)
SIMILARITY              0.917
CROSS-DOMAIN TRANSFER   ✓ TRIGGERED
```

When similarity > 0.85, a small "TRANSFER" badge fires (with a glow pulse) and a sentence appears: *"These encode as structurally equivalent patterns. Knowledge from the trading agent is now available to the coding agent's context window."*

This makes the abstract concept of cross-domain HDC transfer concrete and playable.

### 4.7 Korai · ISFR (§06)

**Light touch** — not a full deep-dive, but the chain story must appear, because the moat is both halves.

**Three.js background:** morphs into a "chain lattice" — particles re-arrange into a tree (Kauri BFT dissemination shape), with periodic "block" pulses propagating through the tree every 400ms (the actual block time). Each pulse is a wave of brightness rippling from the root outward. Mesmerizing.

**Content:**

- h2: *"A chain where the consensus layer is designed for agent cognition."*
- Lead: "400ms blocks. Single-slot finality. Six precompiles at 0xA01–0xA0C. Native InsightStore."
- 3-column grid:
  - **LAYER 0 — CONSENSUS** — Kauri BFT, O(n) messages
  - **LAYER 1 — EXECUTION** — SpecPool EVM, Block-STM, dual-plane
  - **LAYER 2 — KNOWLEDGE** — InsightStore, 6 entry types, automatic decay

- **ISFR ticker widget** — a bone-colored terminal widget showing a live-feeling rate:
  ```
  ISFR · tick 0x00011a7e
  3.72% ▲  +0.03  14-day median
  ```
  The rate ticks every 400ms (the block time). Tiny sparkline showing last 20 ticks.

- **Yield perps gap graphic** — one arresting ASCII visual:
  ```
  TRADFI IR DERIVATIVES         $668,000,000,000,000  ████████████████████████
  ON-CHAIN IR PRODUCTS          $       100,000,000  ▏
                                                       6 orders of magnitude
  ```

That's the whole section. ~90vh.

### 4.8 The Demo (§07)

**This is the kill shot.**

An embedded xterm.js terminal (or pre-baked asciinema replay if live execution is too complex) showing a real Roko invocation:

```
$ roko agent dispatch --task "add retry logic to HttpClient, return typed errors"

[γ tick] OBSERVE
  prediction_error: 0.22 (moderate)
  memory_hit: 0.78 — similar task found (episode 3,847)
  tool_novelty: 0.12 — familiar toolset

[γ tick] GATE → T1 (haiku)
  reason: familiar pattern, moderate surprise

[γ tick] ASSEMBLE — VCG allocation:
  neuro     10,240 tok   (retrieved episode 3,847 + HttpClient context)
  code       5,760 tok
  iter       3,840 tok
  task       3,200 tok
  ...

[γ tick] LLM — claude-haiku-4-5 — 2.1s, $0.003
  → plan: [add retry w/ exponential backoff, type errors via thiserror]

[GATE PIPELINE]
  compile_gate ............. PASS
  clippy_gate .............. PASS
  test_gate ................ FAIL (2 tests missing for retry path)

[γ tick] OBSERVE — prediction_error: 0.58 (escalation)
[γ tick] GATE → T2 (opus)
[γ tick] ASSEMBLE — replan with failure context
[γ tick] LLM — claude-opus-4-7 — 4.8s, $0.04
  → revised: [add 2 tests for retry path, backoff bounded]

[GATE PIPELINE]
  compile_gate ............. PASS
  clippy_gate .............. PASS
  test_gate ................ PASS ✓

[δ tick] REFLECT
  hdc_fingerprint: 0x7f3a...8b2
  episode cluster: "error-handling/retry" (n=23)
  somatic marker: +0.31 confidence for this pattern

[δ tick] CONSOLIDATE (staged)
  insight: "typed errors + bounded backoff is preferred pattern"
  confidence: 0.45 (staging)
  → promote on 2nd validation

[CHAIN]
  insight.insert(0xA03, hdc_vec, half_life=7d)
  tx: 0xa8f2...3d1c · block 847,239

[DONE]
  duration: 7.1s
  cost: $0.043
  tier: T1 → T2 (escalation)
  gates: 3/3
  knowledge deposited: 1 insight (staged)
```

Terminal uses ROSEDUST colors via xterm.js theme config. Cursor blinks. Typing rate: human-feeling (~60ms per character, with occasional pauses). Rose for main text, rose_bright for emphasis lines, success (sage) for PASS, warning (amber) for FAIL, bone for the final cost figure.

**After the run completes**, a small panel slides up from below:
```
THIS IS WHAT JUST HAPPENED:
  · escalated from cheap → frontier model only when gate failed
  · retrieved past episode (3,847) from InsightStore — NOT from scratch
  · deposited a new insight on-chain, available to every future agent
  · session N+1 starts smarter
```

**Controls:** play/pause/restart. A "speed" toggle (1× / 2× / 4×) for impatient VCs.

### 4.9 Outro (§∞)

**Content:**

Large italic Fraunces close:
> *Three of four quadrants are populated. The fourth — persistent learning agents with distributed economic coordination — is empty. Not because it's hard. Because nobody has built the cognitive runtime and the coordination chain **as one integrated system**.*

Two CTAs:
- **→ Run the demo** (bone border, bone text, phosphor glow) — links to `/demo`
- **Read the PRDs** (rose_deep border, rose text) — links to `/prd`

Footer meta row:
```
NUNCHI LABS · 2026        BUILT ON ROKO · SETTLED ON KORAI        ROSEDUST v4.0
```

---

## 5. Three.js scene catalog

One **unified particle system** across the whole page, transformed by scroll position. Not six separate scenes. The transformation is part of the narrative — same particles, different arrangements, as if the system is reorganizing itself.

| Section | Arrangement | Notion |
|---------|-------------|--------|
| Hero | Sphere-ish cloud | HDC vector field, 10,240 "dimensions" |
| Loop | 6 clusters in a ring | Loop phases, connected by pulsing filaments |
| Gate | 3 horizontal bands | T0/T1/T2 layers |
| VCG | 8 colored pools | Bidders, sized by allocation |
| Network | Mesh lattice | Agent graph, density scales with slider |
| Korai | Tree | Kauri BFT tree, block-pulse waves |
| Demo | Dim, out-of-focus | Foreground is the terminal; background recedes |

**Implementation strategy:**
- Single `THREE.Points` system with 2500–5000 particles
- Target positions for each section stored as `Float32Array`s
- Scroll position drives `lerp(currentPos, targetPos, 0.02)` per-frame
- Custom shader handles color, size, glow — this doesn't change per section, only positions do
- Sections also control fog density and camera pose (orbit, tilt, FOV)

**Performance budget:** Hero is the most expensive. Should be <3ms per frame on a 2021 MacBook Air (M1). Test with CPU throttled to 4×. If we exceed budget, reduce particle count before reducing effect quality.

**Fallback:** if `prefers-reduced-motion: reduce`, swap Three.js for a static CSS canvas background — still rose-dominant, still textured, just not animated.

---

## 6. Architecture explorer (`/architecture`)

The deep-dive surface for the technical reader. This is ccunpacked.dev's "click around the source tree."

### 6.1 Structure

- Left pane (25%): hierarchical tree of subsystems, grouped by layer (Runtime, Cognition, Context, Knowledge, Chain, Domains).
- Center pane (50%): currently selected subsystem rendered as a card with:
  - Name, status badge, crate path, LOC count
  - One-paragraph description (from PRD-01 status table)
  - "What it does / How it works / Economic impact / Research basis" (the 4-section pattern used throughout the PRDs)
  - Links to incoming edges ("consumed by") and outgoing edges ("consumes")
- Right pane (25%): mini-map graph view — force-directed SVG with all subsystems as nodes, edges showing data flow. Selected node highlighted. Hover reveals node name.

### 6.2 Status color coding

```
WORKING                rose (saturated, glowing)
BUILT-BUT-DISCONNECTED rose_dim (present but faded)
NOT-YET-BUILT          text_ghost (visible but very dim, dashed border)
```

The honest status gradient becomes a selling point: "here's what exists, here's what's wired, here's what's coming. We're not smoke and mirrors."

### 6.3 Data source

Parse PRD-01's subsystem table (lines ~226–273). The working / built-disconnected / not-yet-built tri-state is explicit. Each row gives name, crate location, and a one-line note. Use this as the initial dataset. Store in `content/architecture.json` or similar.

---

## 7. Technical specification

### 7.1 Stack

- **Framework:** Next.js 14 App Router, TypeScript, React 18.
- **Styling:** CSS Modules + a thin design-token module (`tokens.css` exports the ROSEDUST palette as CSS custom properties). Do NOT use Tailwind's default palette — it will fight the ROSEDUST tokens. If you want Tailwind for layout utilities, configure it with only the ROSEDUST tokens.
- **3D:** `three` (r152+) via npm, managed through `@react-three/fiber` and `@react-three/drei`. The prototype uses r128 from CDN — the real build should use npm + bundler.
- **Animation:** `motion` (formerly Framer Motion) for React component transitions. GSAP optional for complex scroll-driven sequences (consider for Loop → VCG particle morph).
- **Terminal embed:** `xterm.js` + `xterm-addon-fit`. For the scripted demo, use `asciinema-player` as a fallback or primary if live execution is too much.
- **Fonts:** Self-host JetBrains Mono and Fraunces via `next/font`. Do NOT use Google Fonts CDN in prod (privacy + speed).
- **Analytics:** Plausible (privacy-preserving) or none. Do not add GA.

### 7.2 File structure

```
app/
  layout.tsx              — root layout: CRT overlays, nav
  page.tsx                — landing (the scrollytelling page)
  architecture/
    page.tsx              — subsystem explorer
  demo/
    page.tsx              — xterm embed
  prd/
    [slug]/page.tsx       — markdown PRD reader

components/
  boot-sequence.tsx       — CRT boot overlay
  crt-overlay.tsx         — scanlines + grain + vignette + flicker
  nav.tsx
  scroll-progress.tsx
  three/
    particle-system.tsx   — unified Three.js system
    scenes/
      hero-scene.tsx
      loop-scene.tsx
      vcg-scene.tsx
      ...
  sections/
    hero.tsx
    moat.tsx
    loop.tsx
    gate-demo.tsx
    vcg-auction.tsx
    network-effects.tsx
    hdc-playground.tsx
    korai-section.tsx
    demo-terminal.tsx
    outro.tsx
  interactives/
    loop-stages.tsx
    tier-visualizer.tsx
    bidder-card.tsx
    agent-grid.tsx
    hdc-vector.tsx
    isfr-ticker.tsx
  ui/
    axiom.tsx
    meta-grid.tsx
    ascii-chart.tsx
    section-label.tsx

lib/
  tokens.ts               — typed color tokens, spacing, typography
  vcg.ts                  — bid/allocation math
  gate-math.ts            — tier distribution math
  network-math.ts         — compounding math
  hdc.ts                  — bit-vector encoding (hash-based, deterministic)

content/
  loop-stages.json        — copy for the 6 loop phases
  architecture.json       — subsystem graph data
  prd/                    — markdown files for the PRD reader

public/
  fonts/                  — self-hosted
  ascii/                  — static ASCII art fragments
  isfr-replay.cast        — asciinema recording of the demo run

styles/
  globals.css
  tokens.css              — ROSEDUST palette as custom properties
  crt.css                 — overlay layers
```

### 7.3 Boot sequence implementation

The boot overlay (`components/boot-sequence.tsx`):

- Runs ONCE per session (store in `sessionStorage` as `rosedust_boot_seen`).
- 20+ lines appearing over ~2.5–3s with typed-out feel.
- Uses the line list in `lib/boot-log.ts`.
- After completion, fades out over 1.2s and unmounts.

**Critical:** skippable via Esc key, click anywhere, or after 4s if the user hasn't interacted. Never block the actual content.

### 7.4 The xterm demo runner

Two modes:

1. **Replay mode (primary).** Pre-recorded asciinema `.cast` file. Deterministic. Always works. Use `asciinema-player` embedded in a ROSEDUST-themed frame.

2. **Live mode (optional v2).** Server-side: run a containerized Roko instance, stream stdout over websocket. Much harder. Defer until post-launch.

Ship with replay mode. The VC can't tell the difference; the token budget counter and cost accumulator look identical.

### 7.5 Responsive strategy

- **Desktop-first** (this is a VC site; they're on laptops). Design for 1440×900 minimum.
- **Tablet:** collapse 2-column layouts to single column, keep Three.js.
- **Mobile:** aggressive simplification. Drop Three.js entirely. Replace with a gradient-mesh canvas fallback. Collapse the VCG window to vertical stacked bars. Collapse the 6-column loop stages to a swipeable carousel.
- **Text scaling:** `clamp()` on all headline sizes.

### 7.6 Performance targets

- **LCP** < 1.5s on fast 4G
- **CLS** < 0.02
- **Interactive** (TTI) < 3s
- **Bundle size** < 300KB gzipped for the landing route (Three.js will be code-split into the hero chunk)
- **60fps** scroll on M1 MacBook Air with Chrome dev-throttled CPU 4×

### 7.7 Accessibility

- All interactives keyboard-accessible (Tab-navigable, Enter to activate).
- `prefers-reduced-motion` disables flicker, scanlines animation, Three.js animation, auto-advance. Static layout remains, just without motion.
- `aria-label` on every icon-only button.
- Color contrast: rose (#aa7088) on bg_void (#060608) measures ~5.8:1 — meets AA for large text. Body copy (text_primary #988090 on bg_void) measures ~7.2:1 — AAA. The ghost/phantom colors are intentionally below contrast thresholds — they are atmospheric and non-essential. Ensure all critical information is in text_primary or brighter.
- Scanlines at opacity 0.35 via multiply do not affect text contrast measurably, but verify with tooling.

### 7.8 SEO / meta

```
<title>Nunchi · Persistent-learning agents, on-chain knowledge</title>
<meta name="description" content="Roko: a Rust agent runtime that compounds. Korai: a blockchain purpose-built for agent cognition. Session #1000 starts smarter than session #1.">
<meta property="og:image" content="/og-image.png">
```

OG image: a still frame of the hero — Three.js particle field, headline overlaid. Generate once with `@vercel/og` or similar.

### 7.9 Deployment (Railway)

- Dockerfile-based deployment (Next.js standalone output).
- Node 20+ runtime.
- ENV vars: none required for the landing page. When live mode xterm is added, a Roko API endpoint ENV will be needed.
- Domain: `nunchi.xyz` or equivalent (user choice).
- `next.config.js` with `output: 'standalone'` and `images.unoptimized: true` if Railway's CDN complicates image handling.

---

## 8. Content · copy to write or port

Most of the copy is already drafted in the prototype and this PRD. Two sections need fresh writing:

### 8.1 Korai section (§06)

~400 words + three column cards + two widget descriptions. Draft-quality source exists in PRD-07. A writer (human or Claude) should compress it into the form specified in §4.7.

### 8.2 Architecture explorer cards (`/architecture`)

~60 subsystems, each needing a 2-paragraph description in the "what / how / impact / research" structure. Source material is in the 10 PRDs. Can be generated by Claude with a prompt like:

> "Given the PRD text about subsystem X, write a 4-paragraph description for a technical reader. Para 1: what it does. Para 2: how it works mechanically. Para 3: economic/practical impact. Para 4: research basis. Max 400 words total."

Batch this for all 60 subsystems in one pass.

---

## 9. Risks + open questions

### 9.1 Aesthetic risks

- **Risk:** ROSEDUST on web reads as "dark-theme goth" rather than "considered terminal aesthetic." Mitigation: the grain + scanlines + phosphor bleed + Fraunces italic moments are what lifts it. If we drop any one of those four, it collapses into generic dark UI. **Do not drop any.**

- **Risk:** The demo video is the emotional hook, but 10 minutes of staring at a CRT aesthetic can strain eyes. Mitigation: the content has rhythm — interactive moments, quiet text moments, motion moments. No single visual texture dominates for more than 90 seconds.

### 9.2 Technical risks

- **Risk:** Three.js at 2500 particles with custom shaders may stutter on Windows laptops with Intel integrated graphics. Test early on these machines. Have particle count tunable via env or user agent.

- **Risk:** xterm.js replay at the scripted pace may feel contrived. Mitigation: include small random timing jitter in the replay; make it look like a live run.

### 9.3 Strategic risks

- **Risk:** We over-rotate on scaffold > model and a VC who's bullish on models reads it as "anti-model" rather than "orthogonal-to-model." Mitigation: in the Moat copy, explicitly say "better models help us too — we're 22 points ahead of anyone else on every model they release."

- **Risk:** We underplay Korai and the ISFR narrative. Without it, the moat is just "a better agent framework" — the chain is what makes it a network effect. Don't let Korai become an afterthought in the edit pass.

### 9.4 Open questions

- Do we want a sign-up / waitlist form anywhere? Default recommendation: **no**. Adds friction, signals a consumer product. VCs want email or call links, not forms.
- Do we want to show real running metrics from a live Roko instance (not just scripted)? Default recommendation: **yes, eventually, not in v1**. Post-launch enhancement.
- How do we handle browsers without WebGL2? Fallback to CSS-only atmosphere. Gracefully degrade — don't show a broken page.
- Music/audio? Default recommendation: **no**. Many VCs watch demos with sound off. If you add it, it must be off-by-default and subtle (a 528Hz hum, not a soundtrack).

---

## 10. Build plan

### Phase 1: Foundation (week 1)

- Next.js scaffold, token system, font self-hosting, CRT overlay layers
- Nav, boot sequence (with skip), scroll progress
- Hero section with Three.js particle system (one scene only)
- Moat section (static, copy + ASCII chart)

Delivers: the aesthetic is proven, the first ~3 sections of the demo work.

### Phase 2: Interactives (week 2)

- Loop walkthrough (6 stages, auto-advance)
- Cognitive Gating demo
- VCG auction visualizer
- Network effects slider + ASCII grid
- HDC playground

Delivers: all 6 must-have interactives. Full scroll experience functional.

### Phase 3: Demo + polish (week 3)

- Korai section + ISFR ticker
- xterm.js / asciinema demo embed
- Outro + CTAs
- `/architecture` explorer v1
- Mobile responsiveness
- Accessibility audit + fixes
- Performance optimization

Delivers: v1 launchable to Railway.

### Phase 4: Post-launch iteration

- Live xterm mode
- Per-section analytics (heatmaps if we add them)
- `/prd` reader with full markdown
- `/isfr` dedicated page

---

## 11. Deliverables checklist for the implementing Claude

When handing this PRD to Claude Code:

- [ ] Read this PRD in full, plus `/mnt/user-data/uploads/00-design-system.md` for full token definitions
- [ ] Read PRD-01-OVERVIEW.md for the canonical framing of Roko + Korai
- [ ] Open `/mnt/user-data/outputs/rosedust-prototype.html` in a browser and verify the aesthetic before writing any code
- [ ] Scaffold the Next.js app per §7.2
- [ ] Build Phase 1 end-to-end and deploy to Railway staging before moving to Phase 2
- [ ] Every section should be implemented as a standalone component and reviewable in isolation via Storybook or a `/_gallery` route
- [ ] Every interactive component should have at least one `prefers-reduced-motion` test
- [ ] At the end of each phase, run a self-evaluation: does the latest build match the narrative arc described in §4? If not, fix before continuing.

---

## 12. One-line summary

Build a single-page demo that makes a VC understand — in 10 minutes, through a CRT-aesthetic terminal experience — why an agent runtime that compounds on a chain designed for agent cognition is an empty quadrant, and why once filled it can't be caught.
