# Roko Pitch Demo — Series A Immersive Site

## Overview

A 3–5 page, full-screen, immersive HTML experience for the opening of VC pitch meetings.
Navigate with right arrow key or a subtle arrow at the bottom of the screen.
Each page is a self-contained, full-viewport experience — no scrolling, no chrome.

**Audience:** Series A investors. Technical enough to appreciate architecture, but the
demo must communicate *why this matters* before *how it works*.

**Tone:** Confident, minimal, cinematic. Think Apple keynote meets developer tool demo.
Not a slide deck — an experience.

---

## Pages

### Page 1 — "The Problem" (Hook)

**Goal:** Establish the pain in 5 seconds. Make them feel it.

**Concept:** Dark screen. A single prompt blinks into existence, typed out character by
character. Something like:

> "Build me a feature that reads user feedback, generates a plan, implements it,
> tests it, and deploys it."

Then: a cascade of what happens today — fragmented tools, manual handoffs, broken
context, lost learning. Visualized as disconnected nodes drifting apart, or a timeline
that keeps restarting from zero.

**Key message:** Every agent run today is amnesiac. Work is lost. Context is destroyed.
There is no compound learning.

**Interactions:**
- The prompt types itself on load (typewriter effect)
- Nodes/fragments animate in after the prompt completes
- Subtle particle drift or entropy animation in the background
- Hovering over fragments could reveal labels: "lost context", "manual handoff", "no memory"

---

### Page 2 — "The Loop" (Core Insight)

**Goal:** Reveal the universal loop. This is the "aha" moment.

**Concept:** The fragmented nodes from Page 1 snap together into a circular loop.
Animate the transition — chaos becomes order. The loop pulses with light flowing
through it:

```
query → score → route → compose → act → verify → write → react
```

Each node in the loop lights up in sequence, showing data flowing through.
Below or around the loop, show the single architecture line:

> **1 noun. 6 verbs.** Signal → Substrate, Scorer, Gate, Router, Composer, Policy.

**Key message:** One universal pattern. Every agent operation — from a single prompt
to a 200-task plan — runs through the same loop. Simple core, infinite composition.

**Interactions:**
- Loop assembles with a satisfying snap animation
- Light/energy flows through the loop continuously
- Each verb node glows as the pulse passes through it
- Click/hover a node to see a one-line description tooltip
- Optional: small "zoom in" on a node to show real code (2-3 lines max)

---

### Page 3 — "Self-Hosting" (The Demo)

**Goal:** Show, don't tell. Roko develops itself. This is the money page.

**Concept:** Split the screen or use a layered visualization. On one side (or layer),
show the *intent* — a PRD or task description in plain English. On the other side,
show what happens — the cascade of agent actions, gates passing, code being written,
tests running.

Walk through the actual self-hosting workflow:

```
idea → draft → plan → execute → gate → learn → iterate
```

Each step lights up with real artifacts:
- **idea:** A one-liner appears
- **draft:** A PRD document materializes
- **plan:** A task DAG fans out (show 5-8 tasks with dependency arrows)
- **execute:** Agents spawn, work bars fill, code diffs flash
- **gate:** Green checkmarks cascade (compile, test, clippy, diff)
- **learn:** Efficiency metrics tick up, cascade router adapts
- **iterate:** The loop feeds back into itself

**Key message:** Roko doesn't just run agents. It *is* an agent that improves itself.
The system that builds features is the same system that builds *itself*.

**Interactions:**
- Each step in the workflow animates in sequence (auto-play with ~2s per step)
- Click any step to expand it and see real data/artifacts
- The DAG visualization is interactive — hover tasks to see descriptions
- Gate results show real metrics (pass/fail with values)
- A small counter shows: "177K lines of code. 18 crates. Built by itself."

---

### Page 4 — "Scale" (Architecture Depth)

**Goal:** Show this isn't a toy. Show the system's depth and how it compounds.

**Concept:** Zoom out from the single loop to show the full system. An architectural
constellation — 18 crates as nodes in a galaxy-like visualization, with connections
showing dependencies and data flow.

Key subsystems highlighted:
- **Core kernel** (center): Signal + 6 traits
- **Agent dispatch** (ring 1): 8 LLM backends, pools, MCP
- **Knowledge** (ring 2): Neuro store, dreams, distillation
- **Learning** (ring 3): Episodes, playbooks, cascade routing, experiments
- **Infrastructure** (ring 4): HTTP control plane (85 routes), TUI, sidecar

Numbers that matter:
- 18 crates, ~177K LOC
- 85+ HTTP API routes
- 8 LLM backend integrations
- 11 gate types, 7-rung pipeline
- Adaptive model routing that learns from every run

**Key message:** This is a platform, not a wrapper. Deep infrastructure that compounds
with every agent run.

**Interactions:**
- Constellation assembles from center outward
- Hover a crate-node to see its role and line count
- Connection lines pulse to show data flow direction
- Click a ring to expand it and see sub-components
- Animated counters tick up to final numbers

---

### Page 5 — "Vision" (Close)

**Goal:** Leave them wanting more. Set up the live demo that follows.

**Concept:** The constellation from Page 4 contracts back into the single loop from
Page 2. But now the loop is surrounded by a growing spiral — each revolution
represents a cycle of self-improvement.

The spiral grows outward, each ring slightly brighter than the last.

Center text fades in:

> **Agents that build themselves.**

Then below:

> "Let me show you."

This is the transition to the live demo portion of the pitch.

**Key message:** This is just the beginning. The system gets better every time it runs.
Now let's see it live.

**Interactions:**
- Spiral growth animation (smooth, 3-4 seconds)
- Text fades in with weight
- Subtle ambient glow/pulse continues — the system is alive
- Minimal — this page is about feeling, not information

---

## Technical Implementation

### Stack
- Single HTML file (or small set of files) — no build step needed
- Vanilla JS + CSS animations (or lightweight lib like GSAP for timeline control)
- Canvas or SVG for node/graph visualizations
- CSS custom properties for theming
- No framework — keep it fast, portable, and self-contained

### Navigation
- Right arrow key → next page
- Left arrow key → previous page
- Small, subtle arrow indicator at bottom-center of screen
- Dot indicators (like iOS home screen) showing current page
- Smooth crossfade or slide transition between pages
- Keyboard: also support Space for next

### Design Language
- **Background:** Deep black (#0a0a0a) or very dark navy
- **Primary accent:** Electric blue or cyan (#00d4ff range)
- **Secondary accent:** Warm amber or gold for emphasis moments
- **Typography:** Monospace for code/data, clean sans-serif (Inter, SF Pro) for headlines
- **Motion:** Smooth, purposeful. Ease-out curves. Nothing bouncy or playful.
  This is infrastructure — it should feel solid, inevitable.
- **Spacing:** Generous. Let things breathe. Every element earns its place.

### Responsive
- Optimized for 16:9 display (projector/external monitor)
- Should also work on laptop screen (13-16")
- Not mobile-optimized (this is for in-person pitch meetings)

---

## Content Priorities

### What to emphasize
1. **Self-improvement loop** — the system gets better every time it runs
2. **Universal architecture** — one pattern, infinite composition
3. **Real, working software** — 177K LOC, not a prototype
4. **Compound learning** — episodes, playbooks, cascade routing
5. **Platform depth** — 18 crates, 85 routes, 8 backends

### What to avoid
- Too much text on any page
- Jargon without visual support
- Comparing to competitors (let the demo speak)
- Claiming things that aren't wired yet
- Blockchain/chain references (Phase 2+, don't mention)

### Numbers to feature
- 177K lines of Rust
- 18 crates
- 85+ HTTP API routes
- 8 LLM backend integrations
- 11 gate types
- 7-rung validation pipeline

---

## Open Questions

- [ ] Exact color palette — should it match any existing Nunchi/Roko branding?
- [ ] Logo/wordmark to include?
- [ ] Should Page 3 use real recorded data or simulated?
- [ ] Audio/sound design? (subtle ambient, keystroke sounds)
- [ ] How long should the full auto-play take? (target: 60-90 seconds if uninterrupted)
- [ ] Should there be a "skip to live demo" escape hatch?

---

## File Structure (planned)

```
tmp/demo-uis/
├── CONTEXT.md          ← this file
├── index.html          ← main entry point
├── styles.css          ← global styles + page-specific
├── app.js              ← navigation, transitions, state
├── pages/
│   ├── problem.js      ← Page 1 animations + content
│   ├── loop.js         ← Page 2 loop visualization
│   ├── self-host.js    ← Page 3 workflow demo
│   ├── scale.js        ← Page 4 architecture constellation
│   └── vision.js       ← Page 5 closing
└── assets/
    └── (fonts, any static assets)
```

---

## Next Steps

1. Nail down visual identity (colors, fonts, logo)
2. Build Page 2 first (the loop) — it's the conceptual center
3. Build navigation shell + transitions
4. Build remaining pages in order: 1, 3, 4, 5
5. Polish transitions and timing
6. Test on target display setup
