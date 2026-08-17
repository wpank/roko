# Next-Gen Demo App: Full Specification

A complete from-scratch rebuild of the Roko demo surface. Not a patch on the existing app — a new application designed with the full context of what worked, what broke, and what investors need to see.

**Target:** `/Users/will/dev/nunchi/roko/roko/demo/demo-current/` (empty, ready)
**Reference:** `/Users/will/dev/nunchi/roko/roko/demo/demo-app/` (current, for API patterns and scenario logic)
**Landing page:** `/Users/will/Downloads/nunchi_5.html` (use as-is for the marketing/investor deck page)
**Backend:** `roko-serve` at `:6677` (~235 REST/SSE/WS endpoints)
**Stack:** React 19, React Router 7, Vite 6, TypeScript, xterm.js, Three.js (minimal)

---

## Part 0: Core UX Philosophy

This is the most important section. Everything else flows from these principles.

### 0.1 Someone Else Demos This

The demo app must work for **anyone**, not just Will. A board member, a potential hire, an investor's technical advisor — anyone should be able to:

1. Click "Start" on the landing page
2. Understand where they are and what they're looking at within 3 seconds of any page load
3. Know what to click next at every moment
4. Run a complete demo without reading documentation, watching a tutorial, or asking questions
5. Explain what they saw to someone else afterward

**This is the single hardest constraint.** It means:
- No jargon without inline explanation (e.g., "Gate" needs a tooltip: "Automated verification — compile, test, lint")
- No blank screens — if data is loading, show what's happening; if data is missing, explain why and what to do
- No mystery buttons — every button label says what will happen when you click it
- No hidden state — if the system is in a particular mode, that mode is visible on screen

### 0.2 Do Fewer Things, Perfectly

The old app has 6 sections, 20 views, 15 scenarios, 80+ files. Most are broken, half-connected, or visually inconsistent. The new app does **4 things**:

1. **Show how Roko turns a request into verified code** (Orchestrate)
2. **Show the operational control plane** (Observe)
3. **Show the economic evidence** (Evaluate)
4. **Let someone try it themselves** (Build)

That's it. Each section should feel *complete* — not a dashboard of partially-wired widgets, but a focused, polished experience that tells one story really well.

### 0.3 Guided Experience with Escape Hatches

The app has two layers:

**Layer 1 — The guided path** (what 90% of viewers use):
- Linear, obvious, breadcrumb-clear navigation
- Each section starts with a hero moment that explains what you're about to see
- One primary action per screen (a big "Start" button, a "Run Benchmark" button)
- Contextual labels that narrate what's happening ("Routing task to claude-sonnet-4 because complexity exceeds T1 threshold...")

**Layer 2 — The depth** (for the technical person who wants to dig in):
- Expandable rows, detail panels, raw JSON views
- Parameter tuning (temperature, model, gates)
- Terminal output for those who want to see the actual CLI
- But this never clutters the primary view — it's always one click deeper

### 0.4 Visual Feedback Everywhere

Every action should produce immediate, visible feedback. Nothing should feel dead or unresponsive.

| User action | Immediate feedback |
|---|---|
| Click a scenario card | Card highlights, description animates in, CTA appears |
| Click "Start" | Phase rail lights up, terminal appears with first command |
| Task starts executing | Task row pulses, model badge appears, timer starts |
| Gate passes | Green check fades in with a micro-animation |
| Gate fails | Rose flash, failure reason appears inline |
| Tab switch | Fade transition, content slides in |
| Server disconnects | Status pill turns amber, "Reconnecting..." label appears |
| Server reconnects | Status pill turns green, "Connected" label, stale data refreshes |
| Hover over metric | Tooltip explains what the number means |
| Data loads | Skeleton → real data with fadeUp |
| No data available | EmptyState with actionable message ("No runs yet — configure and run your first benchmark above") |

### 0.5 Self-Diagnosing

When something goes wrong, the app tells you what happened and how to fix it. No blank screens, no spinning forever, no mystery failures.

**Connection status** is always visible in the top nav:
- `● LIVE 8h 16m` — connected, everything real-time
- `● SEED DATA` — server not running, showing demo data (with tooltip: "Start roko serve to see live data")
- `● RECONNECTING...` — lost connection, attempting to restore
- `● OFFLINE` — can't reach server (with tooltip: "Check that roko serve is running on port 6677")

**Error states** are informative, not generic:
- Instead of "Error": "Could not load agent fleet — server returned 500. Try refreshing or check roko serve logs."
- Instead of blank: "No episodes recorded yet. Run a plan with `roko plan run plans/` to see agent activity here."
- Instead of crash: Error boundary catches it, shows the component name, the error, and a "Reload section" button

**Health bar** in developer mode (toggle in settings or via query param `?debug=true`):
- Shows API response times, SSE connection state, terminal session IDs, data mode (live vs seed)
- Collapsed by default, expandable from bottom of screen
- Gives a presenter instant visibility into what's working and what's not

### 0.6 Time-to-Wow

"Time to Wow" (TtW) is the elapsed time from entering the app to experiencing the first moment that proves the product works. Research shows users who hit a WOW moment early are 81% more likely to retain.

**For this app, TtW must be under 90 seconds:**
1. Landing page loads (0s)
2. Click "Start" → app shell loads with 4 nav links and LIVE/SEED status (2s)
3. Orchestrate page shows 3 scenario cards with clear descriptions (3s)
4. Click a scenario → description appears, single "START" button (5s)
5. Click START → terminal appears, phase rail lights up, first CLI command runs (8s)
6. PRD generates in real-time, artifact panel populates (20s)
7. Plan generates, task board appears with tier routing visible (35s)
8. First task completes with gate passes visible (50s)
9. All tasks complete, summary mosaic shows cost/time/pass-rate (75s)

The WOW moment is step 9: **the viewer just watched a one-sentence request become a verified, multi-file implementation in 75 seconds for $0.04.** Everything before it serves this moment. Everything after it deepens it.

**The second WOW moment is the Pareto chart in Evaluate** — the viewer sees that Roko's routing achieves the same quality at a fraction of the cost. This is the economic thesis.

**The third WOW moment is Build** — the viewer types their own request and watches it happen. This is the "it's real, not a recording" proof.

### 0.7 Narrative Arc

The app tells a story. Each section builds on the last:

1. **Orchestrate**: "Here's what Roko does — it takes a request and turns it into verified code. Watch."
   - Hero moment: the user sees a prompt become a PRD, become a plan, become running tasks, become verified output
   - Takeaway: *this works end-to-end, autonomously*

2. **Observe**: "Here's what's happening under the hood — the agents, the routing, the knowledge."
   - Hero moment: the fleet topology shows 3 agents with different roles, the routing table shows cost-aware model selection
   - Takeaway: *this is a real system with operational depth, not a script*

3. **Evaluate**: "Here's the evidence — benchmarks, cost curves, quality metrics."
   - Hero moment: Pareto frontier shows Roko achieving 96% pass rate at 40% of naive cost
   - Takeaway: *this isn't just working, it's economically superior*

4. **Build**: "Try it yourself."
   - Hero moment: the viewer types a prompt and watches Roko build it
   - Takeaway: *this is real, and I could use it right now*

Every section has a one-sentence summary at the top (in Fraunces italic, 24px, centered) that tells you exactly what you're looking at. Every section ends with a subtle visual cue pointing to the next section ("Next: see the operational control plane →").

### 0.8 What "Intuitive" Means Concretely

- **Navigation**: 4 items, always visible, current one highlighted. No hamburger menus, no hidden drawers.
- **Layout**: Every page uses the same grid system. Content is centered in a max-width container. No edge-to-edge chaos.
- **Color**: Green means good. Rose/red means failed. Amber means in-progress or warning. Blue-grey means pending. These are the ONLY semantic colors. No exceptions.
- **Icons**: Minimal. ● filled = done, ◉ ring = active, ○ hollow = pending. ✓ = pass. ✕ = fail. These are the only icons needed.
- **Labels**: Every non-obvious UI element has a label. Every label uses the same mono 10px uppercase style. Labels are never ambiguous — "PASS RATE" not "RATE", "TOTAL COST" not "COST", "COMPILE GATE" not "COMPILE".
- **Affordances**: Buttons look like buttons (border, hover state). Links look like links (underline on hover). Cards look clickable (hover lift, cursor pointer). Disabled controls look disabled (opacity 0.4, no pointer events).
- **Whitespace**: Generous. Pages breathe. Sections are separated by 48–64px. Content groups within sections by 24–32px. Never cramped.
- **Consistency**: The same data (e.g., cost in dollars) always looks the same everywhere — same font, same color (bone), same format ($0.042). A gate bar looks identical in Orchestrate, Evaluate, and Build.

### 0.9 The Right-to-Exist Test

Every element on every screen must answer this question: **"What decision or action does this enable that would be impossible or meaningfully harder without it?"** If the answer is "none," remove it.

This is the practical synthesis of Tufte's data-ink ratio ("maximize the share of ink devoted to data"), Nielsen's aesthetic minimalism heuristic, and the escalation-of-commitment anti-pattern (elements that survive into production because they were always there, not because they earn their space).

**Elements that commonly fail this test:**
- Decorative illustrations in empty states that explain nothing and guide nobody
- "Welcome, User!" headers that consume a content row
- Progress bars on static pages where nobody is waiting
- Horizontal rules between sections already separated by whitespace
- Status badges that always say "Active" and never change
- Metric cards that show the same value every time with no baseline comparison (the number exists but conveys nothing)
- Logos in corners of authenticated products — the user knows what product they're using
- Grid lines in charts that add visual noise beyond what axis scales already communicate

**Apply this test ruthlessly.** The old demo app has ~80 files and 20 views. Most exist because they were built, not because they earned their place. The new app has ~50 files and 4 sections. Every element must justify itself.

### 0.10 Data-Ink Ratio

From Edward Tufte: `data-ink ratio = ink used to display data / total ink in the graphic`. Maximize this ratio.

**Practical rules for this app:**
- Remove chart grid lines — axis scales are sufficient
- No chart borders or panel backgrounds where the grid itself provides framing
- No legend labels that duplicate axis labels — use direct labeling on data points
- No 3D effects, shadows, or gradients on chart elements — they consume visual processing without adding information
- No pie charts for more than 3 segments — bar lengths are easier to compare than slice areas
- No decorative tick marks or excessive axis labels
- If a label and an icon say the same thing, keep one

### 0.11 Cognitive Load Management

John Sweller's cognitive load theory identifies three types:

1. **Intrinsic load** — the inherent difficulty of the task (a developer diagnosing a production incident already has high intrinsic load). We can't reduce this.
2. **Extraneous load** — cognitive effort imposed by *how* information is presented, not the task itself. **This is what we control.** Sources: inconsistent terminology, poor hierarchy, visual noise, unpredictable interactions, requiring users to remember information across page transitions.
3. **Germane load** — effort a user willingly invests in understanding the domain. Good tools make this productive: a well-designed routing table teaches model selection; a well-designed gate bar builds understanding of verification.

**Design implications:**
- **Miller's Law**: Working memory holds ~7±2 chunks simultaneously. Dashboards with more than 5-9 independent decision-making elements exceed working memory capacity. The solution is *chunking*: grouping related elements into a single perceived unit. The Mosaic grid is a chunk. The GateBar is a chunk. The Timeline is a chunk.
- **Visual hierarchy offloads working memory**: When importance is encoded by size, color saturation, and position, the user doesn't need to hold a mental priority list.
- **Consistency is cognitive load reduction**: Learned visual patterns transfer across pages. If the GateBar looks the same in Orchestrate, Evaluate, and Build, the user learns it once and recognizes it everywhere.
- **Recognition over recall**: Persist filter state in URLs. Display recently-used queries. Show the current parameters used to generate a view. Never require users to remember how they produced a view they want to return to.

### 0.12 Design Principles from the Best Developer Tools

These are the principles the best-in-class tools (Vercel, Linear, Datadog) follow. Each is a hard rule for this app.

**1. Performance is the first design decision, not the last.**
No amount of beautiful animations compensates for slow loads. Preconnect to API origins. Memoize components. Prefetch likely next-states. SWR (stale-while-revalidate) for all data: show last-known state instantly, update when fresh data arrives. Skeleton loaders only when the shape of data is unknown. Spinners as a last resort.

**2. Empty states are actionable, not decorative.**
When Vercel shows an empty deployment screen, it displays `git push origin main` in monospace font with a copy button. Not a rocket illustration. Not generic copy. The exact terminal command needed. For Roko: every empty state shows the `roko` CLI command that would populate that panel. "No episodes yet. Run `roko plan run plans/` to see agent activity here."

**3. Information density should match the user, not the designer's aesthetic.**
Developers scan, not read. Dense table views with sort/filter beat card grids with three visible fields. Linear and Vercel both default to high density — more rows per viewport, tighter spacing, smaller type than consumer apps. This is deliberate.

**4. Show the equivalent CLI command for every action.**
For every action the UI performs, show the CLI equivalent in dim text nearby. `"roko run 'Build a CLI calculator' --model haiku"` below the Build button. This builds trust (users understand what the UI is doing) and teaches the CLI (reducing future dependency on the web UI).

**5. Status must be visible without focus switching.**
Vercel shows deployment status in the browser tab favicon, page title prefix, and timeline simultaneously. For Roko: the TopNav StatusPill, the document title prefix (`[LIVE] Roko` or `[SEED] Roko`), and the per-section indicators must all be consistent and always visible.

**6. Refresh rate must match data change frequency.**
Don't auto-refresh at 5-second intervals if data changes per-minute — it creates constant peripheral motion without informational value. SSE events push real changes. Polling intervals match actual data staleness: health probes every 30s, episode list every 10s, historical data never auto-refreshes.

### 0.13 UX Anti-Patterns to Explicitly Avoid

These are concrete failure modes, each with a specific description of what it looks like, why it's bad, and what to do instead. **Implementers must treat these as hard prohibitions.**

#### Layout & Hierarchy

**AP-01: The Equal-Weight Grid.** Every widget the same size and visual weight. A "Total Cost" KPI sits in the same 200×200 box as "Last Login." Fix: primary KPI should be visually 2-3× more prominent than secondary metrics. Use size, weight, and position to encode importance. Apply the Z-pattern: top-left gets the most attention — place primary KPIs there.

**AP-02: Navigation That Eats Content.** Persistent 260px sidebar with 40+ items. On 1280px, 20% of space is navigation, not data. Fix: horizontal top nav, 56px tall, with 4 items. No sidebar. 100% of horizontal space for content.

**AP-03: Scroll Dependency.** Critical information below the fold, requiring scroll to discover. Fix: the most important metric and the primary CTA must be visible on load without scrolling, on a 768px-tall viewport.

#### Information & Data

**AP-04: Context-Free Metrics.** "Latency: 340ms" means nothing. "Latency: 340ms (p99, baseline 120ms, SLA 500ms)" means something. Fix: every metric has a comparative frame — vs baseline, vs target, vs naive alternative. "$0.042" alone is meaningless. "$0.042 — 60% less than all-opus" tells a story.

**AP-05: Charts That Don't Answer a Question.** Metrics that exist because data is available, not because anyone defined what question the chart answers. Fix: before creating any visualization, write the question it answers in one sentence. If you can't, don't build it.

**AP-06: Stacked Charts.** Stacked area/bar charts make it visually impossible to read individual series values — every series above the bottom is distorted by cumulative area beneath. Fix: two separate, vertically-aligned single-axis charts.

**AP-07: Dual Y-Axis Charts.** Create spurious correlations and confuse scale. Fix: separate charts, aligned on time axis.

#### Feedback & State

**AP-08: Loading State Theater.** Full-page loading spinners while waiting for data that could be served stale. Skeleton shapes that don't match real data (skeleton has 3 rows, data has 50). Progress bars unrelated to actual progress. Fix: SWR for stale data, correctly-shaped skeletons, determinate progress when possible.

**AP-09: Hiding Actions Behind Hover.** Edit/delete buttons that only appear on row hover. Users cannot know what actions exist without surveying every element with their cursor. Fix: always-visible action buttons (possibly dimmed), or a single action menu icon (⋯) that's always present.

**AP-10: Silent State Transitions.** A metric updates from 94% to 87% without any visual signal. The user, not focused on that panel, misses it entirely (change blindness). Fix: when a value changes, a 200-300ms highlight animation draws the eye. Brief, functional, not decorative.

**AP-11: Non-Streaming Output.** Collecting all CLI output and displaying after completion. CLI users expect to see output as it's produced. Fix: stream terminal output via WebSocket, character by character, in real-time.

#### Interaction

**AP-12: Mouse-Only Interactions.** Requiring mouse for common operations in a tool whose users are keyboard-centric. Fix: keyboard shortcuts for all primary actions. Space to play/pause, N for next step, R for reset, 1-3 for scenarios, ? for help overlay.

**AP-13: Animation That Adds Latency.** Page transitions, slide-in panels, loading shimmer persisting longer than the actual load. CLI users have zero tolerance for UI latency added by decoration. Fix: animations under 300ms, never blocking interaction, skip-able during fast navigation.

**AP-14: Feature-First Demo Narrative.** Leading with "here are all the things we do" instead of "here is the problem we solve, in real-time." Fix: the Orchestrate page opens with a scenario description (the problem), then shows the solution happening. The feature inventory is never shown — it's experienced.

#### Content

**AP-15: Generic Placeholder Data.** "Demo Company Inc", timestamps from 2020, lorem ipsum. Signals prototype, not product. Fix: realistic seed data — diverse names, plausible dollar amounts, recent timestamps, realistic volume (dozens of entries, not 3).

**AP-16: Explaining While Navigating.** Talking about features while clicking through menus splits audience attention. Fix: demonstrate the action, pause, then explain what they saw. The UI should do the explaining through contextual labels, not require verbal narration.

**AP-17: Showing Too Much.** Covering every feature in a 20-minute slot. The fewer capabilities demoed, the more each impresses. Abundance signals complexity; focus signals confidence. Fix: 4 sections, 3 scenarios, each done completely and polished.

---

## Part 1: Architecture Decisions

### 1.1 App Structure

```
nunchi_5.html (standalone landing page)
  └─ "Start" button → /app (React SPA)

/app (React SPA with internal routing)
  ├─ /app/orchestrate  — The Pipeline (primary demo)
  ├─ /app/observe      — The Control Plane (operational dashboard)
  ├─ /app/evaluate     — The Evidence (benchmarks & economics)
  └─ /app/build        — The Terminal (real shell + builder)
```

**Why 4 sections instead of 6:**
- "Demo" and "Dashboard" were showing the same system from different angles → merge into "Orchestrate" (pipeline + live state)
- "Explorer" was a weaker version of Dashboard → fold into "Observe"
- "Bench" stays as "Evaluate" — it tells the cost/evidence story
- "Builder" and "Terminal" merge into "Build" — one section for hands-on interaction
- No standalone "Terminal" page (it's a panel within Build, not a destination)

### 1.2 Data Mode: Live-First

- App connects to `roko-serve` at startup
- All data is real API data from the running server
- Seed data is a fallback, not the primary mode
- When seed data is shown, it's explicitly labeled with a small badge
- The server probe re-checks every 30 seconds (not a one-shot singleton)

### 1.3 Terminal: Real WebSocket PTY

- Real terminal sessions via `ws://host/ws/terminal/:sessionId`
- No `tput colors` rainbow bar — clear terminal after capability probe
- Listener registration separated from WebSocket lifecycle (fix B1)
- Terminal is embedded within pages, not a standalone destination

### 1.4 Progressive Complexity

The app should be navigable by someone who has never seen Roko:
1. **Orchestrate** first — shows the core story (request → verified output)
2. **Observe** second — shows operational depth (fleet, costs, knowledge)
3. **Evaluate** third — shows economic evidence (benchmarks, Pareto)
4. **Build** last — hands-on power user surface

Each section's TopNav link includes a subtle subtitle on hover:
- ORCHESTRATE → "Watch it build"
- OBSERVE → "See what's running"
- EVALUATE → "The evidence"
- BUILD → "Try it yourself"

### 1.5 Contextual Narration

Throughout the app, **contextual labels** explain what's happening and why. These are not decorative — they're the primary way a first-time viewer understands the system.

Examples:
- On the task board, when a task is routed to haiku: `"T1 · haiku — low complexity, simple scaffolding"` (not just "T1 · haiku")
- On a gate passing: `"✓ compile — 0 errors, 0 warnings"` (not just "✓ compile")
- On the routing table: `"claude-sonnet-4 selected for this tier because pass-rate > 94% and cost-per-success < $0.02"` (not just "claude-sonnet-4")
- On cost metrics: `"$0.042 total — 60% less than running all tasks on opus"` (comparative framing)

These labels are always in `--text-soft` color, mono 11px, below or beside the primary element. They never compete for attention — they're there when you look, invisible when you don't.

### 1.6 Inline Glossary

First-time appearance of domain terms gets an inline explanation. This is done via a `<Term>` component that renders a dotted underline and a tooltip on hover:

| Term | Tooltip |
|---|---|
| Gate | "Automated verification step — compile, test, lint, or diff check" |
| Tier / T1 / T2 / T3 | "Model routing tier — T1 is fast/cheap (haiku), T2 is balanced (sonnet), T3 is powerful/expensive (opus)" |
| PRD | "Product Requirements Document — describes what to build and the acceptance criteria" |
| C-Factor | "Composite quality score — combines gate pass rate, cost efficiency, speed, and learning rate" |
| Cascade Router | "Automatic model selection — picks the cheapest model likely to pass all gates" |
| Episode | "A complete agent interaction — prompt in, code out, gates checked, results recorded" |
| Neuro Store | "Durable knowledge base — insights the system has learned and reuses across tasks" |
| Dream Cycle | "Offline consolidation — compresses and distills knowledge during idle periods" |

The `<Term>` component only shows the tooltip on the first occurrence per page session. After that it renders as normal text. This prevents tooltip fatigue.

---

## Part 2: Design System — ROSEDUST v2

The design system is extracted from `nunchi_5.html` and refined into a reusable component library. The implementation details below are based on research of what Vercel (Rauno Freiberg), Linear, Stripe, Raycast, and Figma do to make dark interfaces feel world-class.

### 2.1 Design Tokens

```css
/* ── FILE: src/design/tokens.css ── */

:root {
  /* ─── Backgrounds ───
   * ROSEDUST uses a cool-rose tinted dark, not pure gray.
   * The slight hue in backgrounds prevents the "dead gray" feeling.
   * Each step is ~3% OKLCH lightness apart — this creates distinct
   * surface elevation layers that feel physical without being obvious.
   */
  --bg-void: #060608;
  --bg-raised: #0a0810;
  --bg-mid: #080810;
  --bg-deeper: #040406;
  --bg-glass: rgba(8, 8, 12, 0.45);
  --bg-glass-hover: rgba(58, 32, 48, 0.14);
  --bg-glass-active: rgba(58, 32, 48, 0.32);

  /* ─── Borders ───
   * 1px borders at low opacity are the signature of premium dark UIs.
   * Use rgba white on dark surfaces — it adapts correctly to any
   * background color automatically.
   */
  --border: rgba(255, 255, 255, 0.07);
  --border-soft: rgba(255, 255, 255, 0.04);
  --border-strong: rgba(255, 255, 255, 0.14);
  --border-active: var(--rose-glow);

  /* ─── Rose spectrum (primary accent) ─── */
  --rose: #aa7088;
  --rose-bright: #cc90a8;
  --rose-glow: #dca5bd;
  --rose-dim: #7a5060;
  --rose-deep: #3a2030;
  --rose-ember: #482838;

  /* ─── Bone spectrum (value/cost/provenance) ─── */
  --bone: #c8b890;
  --bone-bright: #d8c8a0;
  --bone-dim: #8a7a5a;

  /* ─── Text ───
   * Primary text is NOT pure white (#fff). Pure white on dark bg
   * creates 21:1 contrast ratio — causes eye strain in extended sessions.
   * Use ~90% opacity or #d8c8d0 (tinted off-white). This is what
   * every Apple OS dark mode and every premium tool uses.
   */
  --text-primary: #c8b8c0;
  --text-strong: #d8c8d0;
  --text-soft: #988090;
  --text-dim: #6a5a68;
  --text-ghost: #3a303a;

  /* ─── Semantic ─── */
  --dream: #7a7a98;
  --dream-bright: #9494b4;
  --dream-deep: #282848;
  --success: #7a8a78;
  --warning: #c89a68;
  --danger: #cc5555;

  /* ─── Typography ─── */
  --mono: "JetBrains Mono", ui-monospace, monospace;
  --display: "Fraunces", "Times New Roman", serif;

  /* ─── Spacing ─── */
  --gap-xs: 4px;
  --gap-sm: 8px;
  --gap-md: 16px;
  --gap-lg: 24px;
  --gap-xl: 40px;
  --gap-2xl: 64px;

  /* ─── Radii ─── */
  --radius-none: 0;  /* default — ROSEDUST uses sharp corners */

  /* ─── Shadows ───
   * On dark backgrounds, outer box-shadows alone are nearly invisible.
   * Always pair shadow with a subtle border. The inset top-edge
   * highlight (specular) creates physical depth — this one detail
   * separates premium from flat.
   */
  --shadow-sm:
    0 1px 3px rgba(0, 0, 0, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.06);
  --shadow-md:
    0 4px 16px rgba(0, 0, 0, 0.5),
    inset 0 1px 0 rgba(255, 255, 255, 0.06);
  --shadow-lg:
    0 12px 40px rgba(0, 0, 0, 0.6);
  --shadow-glow-rose:
    0 0 0 1px rgba(220, 165, 189, 0.3),
    0 0 20px rgba(170, 112, 136, 0.15);

  /* ─── Motion ───
   * Vercel's standard: cubic-bezier(.2, .8, .2, 1) — "fast-in, gentle-out"
   * Use ease-out, never ease or ease-in-out for hover effects.
   * ease has a slow start that introduces perceived latency.
   */
  --ease-snappy: cubic-bezier(0.2, 0.8, 0.2, 1);
  --ease-expo: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-out: cubic-bezier(0, 0, 0.2, 1);

  --duration-instant: 80ms;   /* color/opacity changes */
  --duration-fast: 150ms;     /* border, transform, hover */
  --duration-normal: 220ms;   /* tooltip, dropdown, panel */
  --duration-slow: 350ms;     /* page transition, modal */

  /* ─── Focus ───
   * Double-ring pattern: dark gap + accent ring.
   * The dark gap ensures the ring is visible on any background.
   */
  --focus-ring:
    0 0 0 2px var(--bg-void),
    0 0 0 4px rgba(220, 165, 189, 0.7);
}
```

### 2.2 Typography Scale

```
Display heading:  Fraunces italic 300, 46-82px, tracking -0.022em, line-height 1.1
Section heading:  Fraunces italic 400, 30px, tracking -0.012em, line-height 1.15
Page hero:        Fraunces italic 300, 24px, tracking -0.008em, line-height 1.4
Body:             Fraunces 400, 16px, line-height 1.7
Body large:       Fraunces 400, 19px, line-height 1.62

Label:            JetBrains Mono 500, 11px, tracking 0.08em, uppercase
Label small:      JetBrains Mono 500, 10px, tracking 0.06em, uppercase
Mono value:       JetBrains Mono 400, 14px, tracking 0.02em
Mono large value: Fraunces italic 400, 38px, tracking -0.015em
```

**Critical: font-weight on dark backgrounds.**
The same weight reads lighter on dark backgrounds than light. Body text that looks normal at `400` on light bg looks thin and weak on dark bg. ROSEDUST uses **400 for body text** (Fraunces has optically thicker stems than most sans-serifs) and **500 for mono labels** (JetBrains Mono at 400 looks too light at small sizes on dark bg). If we used Inter/system sans-serif, we'd need 500 for body.

**Letter-spacing rules:**
```
32px+:    -0.02em to -0.03em (pull in — large type needs tightening)
20-30px:  -0.01em (barely perceptible)
14-16px:   0 to +0.01em (default tracking)
11-13px:  +0.02em to +0.04em (open up — small type needs air)
UPPERCASE: +0.06em to +0.10em (always open — caps need significant tracking)
```

**Line-height ratios:**
```
Headings (24px+): 1.1 – 1.2 (tight, creates visual weight)
Body (14-19px):   1.5 – 1.7 (comfortable reading)
Mono/code:        1.7 – 1.8 (open — mono is inherently denser)
Labels (10-12px): 1.4 (compact but legible)
```

**Minimum sizes:**
- Body text: 13px
- Table data: 12px
- Labels: 11px
- Canvas labels: 10px (with legibility testing)

### 2.2a Craft Details That Separate Good from World-Class

These are the specific details most implementations miss. Each one is small; together they create the feeling of "this was made by someone who cares."

**1. Specular top-edge highlight on every elevated surface.**
Every `<Pane>`, every card, every elevated panel gets: `inset 0 1px 0 rgba(255, 255, 255, 0.06)`. This simulates a light source above the panel. At 6% opacity it's almost invisible — removing it immediately makes panels feel flatter. This is included in `--shadow-sm` and `--shadow-md`.

**2. Borders use rgba white, not hex colors.**
`rgba(255, 255, 255, 0.07)` adapts correctly to any background. Hex borders (`#1a1622`) break when the surface color changes. All ROSEDUST borders use the rgba-white approach.

**3. Hover transforms are small.**
Card hover: `translateY(-2px)` and `scale(1.005)`, not `translateY(-8px)` and `scale(1.05)`. Large values feel theatrical at dashboard density. The shadow increases on hover but the element barely moves:
```css
.card {
  transform: translateY(0);
  box-shadow: var(--shadow-sm);
  transition: transform var(--duration-fast) var(--ease-snappy),
              box-shadow var(--duration-fast) var(--ease-snappy);
  will-change: transform;
}
.card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}
```

**4. Active/pressed state has asymmetric timing.**
Press is faster than release — creates the physical sensation of pressing a button:
```css
.button:active {
  transform: scale(0.97) translateY(1px);
  transition-duration: 50ms; /* snappier on press */
}
.button {
  transition: transform 120ms var(--ease-snappy); /* slower release */
}
```

**5. Value change animations.**
When a metric updates (cost, pass rate, timer), a brief 200ms highlight draws the eye. Without this, users miss updates entirely (change blindness):
```css
@keyframes value-flash {
  0% { color: var(--bone-bright); }
  100% { color: inherit; }
}
.value-updated { animation: value-flash 300ms var(--ease-out); }
```

**6. Staggered list entrance.**
When task rows, episode entries, or agent cards appear, they stagger in with 40ms delay between items. This is fast enough to not feel slow (80-150ms would be too dramatic) but creates perceptible motion:
```css
.list-item {
  opacity: 0;
  transform: translateY(8px);
  animation: fadeUp 200ms var(--ease-expo) forwards;
}
.list-item:nth-child(1) { animation-delay: 0ms; }
.list-item:nth-child(2) { animation-delay: 40ms; }
.list-item:nth-child(3) { animation-delay: 80ms; }
/* ... or use CSS custom property: animation-delay: calc(var(--i) * 40ms); */
```

**7. Skeleton shimmer calibrated for dark UI.**
Consumer apps use `0.1 → 0.2` opacity. Developer tools use the lower range — subtle, not flashy:
```css
@keyframes shimmer {
  0%   { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
.skeleton {
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.03) 0%,
    rgba(255, 255, 255, 0.07) 40%,
    rgba(255, 255, 255, 0.03) 80%
  );
  background-size: 200% 100%;
  animation: shimmer 1.8s ease-in-out infinite;
}
```

**8. Tooltip entrance combines opacity AND transform.**
Pure opacity fade looks like a "flash from nowhere." Adding 4px Y travel + 0.97 scale makes appearance feel physical:
```css
.tooltip {
  opacity: 0;
  transform: translateY(4px) scale(0.97);
  transition: opacity 120ms var(--ease-out),
              transform 120ms var(--ease-snappy);
}
.tooltip[data-visible] {
  opacity: 1;
  transform: translateY(0) scale(1);
}
```

**9. Glass panels use saturate() in backdrop-filter.**
`backdrop-filter: blur(12px) saturate(180%)` makes the blurred content behind glass look intentionally vivid rather than washed out. This is how macOS menu bars achieve their premium feel. Without `saturate()`, glass panels look dull.

**10. Never use `transition: all`.**
Always list specific properties: `transition: transform 150ms, box-shadow 150ms, opacity 150ms`. `all` transitions properties you don't intend (padding, margin, color) causing micro-jank. It's also slower — the browser must check every animatable property.

**11. `will-change` on elements that animate on hover.**
Any element that transforms on hover should have `will-change: transform` in its default state. Without this, the first hover shows a compositing hitch as the browser promotes the layer. Especially visible in Chrome on lower-powered machines.

**12. Motion duration scales: never exceed 400ms for UI feedback.**
500ms+ reads as broken. The only exception is page-level transitions, which can go up to 400ms max. Tooltips at 120ms, dropdowns at 180ms, modals at 300ms.

### 2.3 Reusable Components

Every component below maps to a visual pattern from `nunchi_5.html`. The implementation should be a flat set of `.tsx` files in `src/design/`.

#### `<Pane>` — Glass panel container
```
┌─ ● LABEL ────────────────────── badge ─┐
│                                         │
│  content                                │
│                                         │
├─────────────────────────────────────────┤
│  footer                                 │
└─────────────────────────────────────────┘
```
Props: `label`, `badge?`, `footer?`, `led?: 'rose' | 'bone' | 'dream' | 'success' | 'warning'`, `flat?: boolean`
- `led` renders the 5px glowing dot before the label
- `flat` removes body padding
- Left rose border: 2px solid var(--rose-dim) with glow
- Background: `var(--bg-glass)` with 1px `var(--border)` border
- Header: mono 10.5px uppercase tracking 0.06em, dim text
- **Specular highlight**: `inset 0 1px 0 rgba(255,255,255,0.06)` on the body
- **Glass**: `backdrop-filter: blur(12px) saturate(180%)`
- **Hover**: border shifts to `var(--border-strong)`, 150ms ease-out
- **Entrance**: `fadeUp` animation, 200ms, staggered if multiple panes

#### `<Mosaic>` — 1px-gap metric grid
```
┌────────┬────────┬────────┐
│ LABEL  │ LABEL  │ LABEL  │
│ Value  │ Value  │ Value  │
│ sub    │ sub    │ sub    │
└────────┴────────┴────────┘
```
Props: `columns: 2 | 3 | 4 | 5 | 6`, `children: MosaicCell[]`
- Gap: 1px with `var(--border)` background showing through
- Cell: padding 30px 28px, bg `var(--bg-glass)`
- **Specular**: each cell gets `inset 0 1px 0 rgba(255,255,255,0.04)` — lighter than Pane since cells are smaller
- Cells appear with 40ms stagger per cell on mount

#### `<MosaicCell>` — Single metric cell
Props: `label: string`, `value: string | number`, `sub?: string`, `color?: 'rose' | 'bone' | 'dream' | 'success'`
- Label: mono 10px uppercase tracking 0.28em, dim text
- Value: Fraunces italic 400, 38px, bone-bright by default
- Sub: Fraunces 300, 14px, soft text

#### `<Led>` — Glowing status dot
Props: `color: 'rose' | 'bone' | 'dream' | 'success' | 'warning'`, `pulse?: boolean`
- 5px circle with color fill
- **Glow**: three-layer box-shadow:
  - `0 0 0 1px` at 40% opacity (tight ring — separates "pro glow" from "amateur glow")
  - `0 0 8px` at 25% opacity (soft outer glow)
  - The tight ring is what makes it look engineered vs. cheap
- Optional pulse: `2.2s ease-in-out infinite` — opacity 1 → 0.4 → 1
- When state changes (e.g., pending → running), animate color with 200ms transition

#### `<StagLabel>` — Section stage label
```
——  01  SECTION NAME
```
Props: `num?: string`, `label: string`
- Mono 11px uppercase, tracking 0.32em, dim text
- Decorative dash prefix in rose-dim
- 80px margin-bottom

#### `<Axiom>` — Pull quote / key message
```
                ——  AXIOM  ——
     "The model is the same.
      The system is the variable."
   → Roko mediates policy, routing, state...
```
Props: `label?: string`, `quote: string`, `corollary?: string`
- Center-aligned
- Quote: Fraunces italic 300, 28-46px clamp, strong text
- Corollary: Fraunces 300, 16px, soft text
- 90px vertical margin

#### `<Table>` — Data table
Props: `columns: Column[]`, `rows: Row[]`, `onRowClick?`, `dense?: boolean`
- Full-width, mono 13px
- Headers: mono 10px uppercase tracking 0.06em, dim text, no border — separated by whitespace
- First column: Fraunces italic 16px, strong text (named entities deserve emphasis)
- Rows: 14px padding (`dense`: 10px), 1px `var(--border-soft)` bottom border
- Hover: `var(--bg-glass-hover)`, transition 80ms (color changes should be instant-feeling)
- Clickable rows: `cursor: pointer`, subtle translateY(-1px) on hover
- **Expandable rows**: if `onRowClick` provided, add a small chevron icon that rotates on expand
- **Stagger**: rows fade in with 40ms stagger per row on mount
- **Empty**: if 0 rows, render `<EmptyState>` inside the table body area

#### `<GateBar>` — Gate status strip
```
  ✓ COMPILE   ✓ TEST   ◉ CLIPPY   ○ DIFF
```
Props: `gates: { name: string, status: 'pass' | 'fail' | 'running' | 'pending' }[]`
- Always rendered (no conditional mount — pending gates are visible as dim placeholders)
- Pass: success color + check appears with 150ms scale-up from 0.8→1.0
- Fail: rose-glow + brief 200ms flash animation to draw attention
- Running: bone color + LED pulse
- Pending: `--text-ghost` — visible but clearly inactive
- Mono 10px uppercase tracking 0.06em
- Horizontal flex, gap 24px, centered
- Status transitions animate: icon morphs (○ → ◉ → ✓) with 120ms crossfade

#### `<Timeline>` — Horizontal phase rail
```
  ● IDEA ——— ● PRD ——— ● PLAN ——— ◉ TASKS ——— ○ RUN ——— ○ DONE
```
Props: `phases: string[]`, `current: number`, `failed?: number`
- Horizontal flex with 1px connecting lines between dots
- Done: filled dot in success color, connecting line to next is solid
- Current: outlined ring in rose-glow with LED-style pulse, connecting line is rose
- Pending: dim outline in `--text-ghost`, connecting line is `--border-soft`
- Failed: rose-bright filled dot + glow
- Labels: mono 10px uppercase tracking 0.06em, positioned below dots
- **Transition**: when phase advances, the new dot fills with a 200ms scale-up from center, the connecting line "draws" left-to-right with 300ms ease-out
- This is the single most visible animation in the app — it must feel satisfying, like progress completing

#### `<Terminal>` — xterm.js wrapper
Props: `sessionId: string`, `onReady?: (handle: TerminalHandle) => void`
- Full-height flex fill
- ROSEDUST xterm theme from rosedust-theme.ts
- No rainbow bar on connect
- Close button, status indicator
- Resize observer + FitAddon

#### `<Skeleton>` — Loading placeholder
Props: `height?: number`, `width?: string`, `variant?: 'text' | 'cell' | 'pane'`
- Shimmer: `linear-gradient(90deg, rgba(255,255,255,0.03) 0%, rgba(255,255,255,0.07) 40%, rgba(255,255,255,0.03) 80%)`
- `background-size: 200% 100%`, animation: `shimmer 1.8s ease-in-out infinite`
- `variant: 'text'` → height: 14px, border-radius: 2px (mimics a text line)
- `variant: 'cell'` → fills MosaicCell shape with matching padding
- `variant: 'pane'` → fills full Pane body area
- **Must match the shape of the real content.** A skeleton that has 3 rows when the real data has 50 is worse than no skeleton. When in doubt, use a single rectangle matching the container height.

#### `<EmptyState>` — Informative empty state
Props: `message: string`, `action?: string`, `hint?: string`
- Centered, mono 12px, dim text
- 40px padding
- `message`: what's empty ("No benchmark runs yet")
- `action`: what to do about it ("Configure and run your first benchmark above")
- `hint`: technical detail for debugging ("API returned 404 on /api/bench/runs")

#### `<Term>` — Inline glossary tooltip
Props: `label: string`, `tooltip: string`
- Renders text with dotted underline in `--text-soft`
- Hover shows a max-width 280px tooltip (dark bg, bone text, mono 11px)
- Tracks whether this term has been shown this session — after first hover, underline fades away
- Used for domain jargon: Gate, Tier, PRD, C-Factor, etc.

#### `<StatusPill>` — Connection status indicator
Props: `mode: 'live' | 'seed' | 'reconnecting' | 'offline'`, `uptime?: string`
- Used in TopNav to show current data mode
- `live`: green LED + "LIVE 8h 16m" (uptime)
- `seed`: bone LED + "SEED DATA" + tooltip: "Start `roko serve` to see live data"
- `reconnecting`: amber pulse LED + "RECONNECTING..."
- `offline`: dim LED + "OFFLINE" + tooltip: "Check that `roko serve` is running on port 6677"

#### `<HealthBar>` — Developer debug panel
Props: (reads from ApiContext)
- Collapsed strip at bottom of viewport, 24px height, mono 10px
- Shows: API latency, SSE state, terminal sessions, data mode, last probe time
- Click to expand: full response log, error history, seed data indicators
- Activated via query param `?debug=true` or keyboard shortcut `D`
- Not shown by default in demo mode

#### `<SectionHero>` — Section intro line
Props: `line: string`, `cue?: { label: string, section: string }`
- Centered Fraunces italic 24px, `--text-soft`, 48px margin top/bottom
- Optional bottom-right cue: "Next: see the evidence →  EVALUATE" in mono 11px, `--text-dim`
- Used at top of every page section

### 2.4 Atmospheric Layers

These are full-page overlays that give the app the cinematic quality of nunchi_5.html. Without these, the interface feels "too clean" — digital rather than crafted. Adding them takes minutes and immediately shifts quality perception.

**1. Grain texture** (the most important atmospheric layer):
```css
/* SVG filter method — performant, no image assets */
.grain-overlay {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 9999;
  opacity: 0.04;  /* 0.03–0.06 range for developer tools — above 0.08 is visible as texture */
  mix-blend-mode: overlay;
}
```
```html
<svg style="position:absolute;width:0;height:0">
  <filter id="noise">
    <feTurbulence type="fractalNoise" baseFrequency="0.65" numOctaves="3" stitchTiles="stitch" />
    <feColorMatrix type="saturate" values="0" />
  </filter>
</svg>
<div class="grain-overlay" style="filter:url(#noise)" />
```
Key calibrations:
- `baseFrequency: 0.65` — film grain feel (lower = coarser, higher = fine static)
- `numOctaves: 3` — organic-looking fractal pattern
- `opacity: 0.04` — barely perceptible, but removing it is immediately noticeable

**2. Scanlines:**
```css
.scanlines {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 9998;
  background: repeating-linear-gradient(
    0deg,
    transparent 0px,
    transparent 2px,
    rgba(0, 0, 0, 0.45) 2px,
    rgba(0, 0, 0, 0.45) 3px
  );
  opacity: 0.06;
}
```

**3. Vignette:**
```css
.vignette {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 9997;
  background: radial-gradient(ellipse at center, transparent 50%, rgba(6, 6, 8, 0.72) 100%);
}
```

**4. Ambient particles** — Three.js fixed canvas, very subtle floating dots. Optional — only include if it doesn't add >2ms per frame. If it does, skip it. The grain layer alone provides 80% of the atmospheric effect.

All layers use `pointer-events: none` and fixed positioning. Total GPU cost is negligible — these are static overlays with no per-frame computation except the Three.js particles.

### 2.5 Animations

```css
/* ── Core keyframes ── */

/* LED pulse — used on status dots, active phase indicators */
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
/* 2.4s ease-in-out infinite */

/* Element entrance — used on panes, cards, list items */
@keyframes fadeUp {
  from {
    opacity: 0;
    transform: translateY(12px);  /* 12px, not 16 — subtler */
  }
}
/* 200ms var(--ease-expo) forwards */
/* Stagger: calc(var(--i, 0) * 40ms) delay per item */

/* Simple fade — used on tab content switches */
@keyframes fadeIn {
  from { opacity: 0; }
}
/* 150ms ease-out */

/* Loading shimmer — see Skeleton component spec */
@keyframes shimmer {
  0%   { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
/* 1.8s ease-in-out infinite */

/* CRT flicker — very subtle, long period, barely perceptible */
@keyframes flicker {
  0%, 98% { opacity: 1; }
  99%     { opacity: 0.97; }
}
/* 11s linear infinite */

/* Value change highlight — draws attention to updated metrics */
@keyframes value-flash {
  0%   { color: var(--bone-bright); text-shadow: 0 0 8px rgba(216, 200, 160, 0.3); }
  100% { color: inherit; text-shadow: none; }
}
/* 300ms var(--ease-out) */

/* Gate pass — celebratory but restrained */
@keyframes gate-pass {
  0%   { transform: scale(0.8); opacity: 0; }
  60%  { transform: scale(1.05); }
  100% { transform: scale(1); opacity: 1; }
}
/* 200ms var(--ease-snappy) */

/* Phase rail line draw — line extends left to right */
@keyframes line-draw {
  from { transform: scaleX(0); transform-origin: left; }
  to   { transform: scaleX(1); }
}
/* 300ms var(--ease-out) */
```

**Animation rules (hard constraints):**
- Never exceed 400ms for any UI feedback animation
- Use `var(--ease-out)` or `var(--ease-snappy)` for all hover/interaction effects — never `ease` (slow start = perceived latency)
- Use `will-change: transform, opacity` on any element that animates on hover
- Always transition specific properties, never `transition: all`
- Context-dependent motion: high-frequency actions (repeated clicks, typing) get 0ms or 80ms transitions. Low-frequency actions (first page load, phase completion) get the full entrance animations.
- If the user has `prefers-reduced-motion`, disable all animations except color transitions

### 2.6 State Colors

| State | Color | CSS variable | Usage |
|-------|-------|-------------|-------|
| Active/running | Rose glow | `--rose-glow` | Active tasks, current phase |
| Value/cost/money | Bone bright | `--bone-bright` | Dollar amounts, efficiency metrics |
| Passing/healthy | Success green | `--success` | Gate pass, server connected |
| Warning/attention | Warning amber | `--warning` | Degraded, retrying |
| Failed/error | Rose bright | `--rose-bright` | Gate fail, crash |
| Pending/inactive | Text dim | `--text-dim` | Queued, not started |
| Info/neutral | Dream blue | `--dream-bright` | Background context |

**Color must always be redundant.** Never use color alone to convey state — always pair with an icon (✓, ✕, ◉, ○) and/or text label. This prevents color-blindness issues (affects 1 in 12 men) without requiring a separate accessibility mode.

### 2.7 Focus & Keyboard Interaction

Every interactive element must have a visible focus state for keyboard navigation. The ROSEDUST focus style uses the double-ring pattern:

```css
/* Apply globally */
:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
  /* = 0 0 0 2px var(--bg-void), 0 0 0 4px rgba(220,165,189,0.7) */
}
```

The dark inner ring (2px) creates a gap that ensures the rose outer ring (4px) is visible against any background. This is the same pattern Vercel and Linear use.

**Keyboard shortcut overlay** — pressing `?` anywhere shows a help overlay listing all shortcuts:
```
SPACE       Play / Pause
N           Next step (when paused)
R           Reset scenario
1 / 2 / 3   Select scenario
D           Toggle debug panel
?           Show this help
```

The overlay is a centered Pane with glass background, fadeIn 150ms entrance, Escape or click-outside to dismiss.

### 2.8 Performance Budget

World-class tools feel instant. These are hard limits:

| Metric | Budget |
|---|---|
| First Contentful Paint | < 1.0s |
| Largest Contentful Paint | < 1.5s |
| Total JS bundle (gzipped) | < 200KB (excluding xterm.js) |
| Time to Interactive | < 2.0s |
| Hover feedback latency | < 16ms (1 frame at 60fps) |
| API → render latency | < 100ms perceived |
| Font loading strategy | `font-display: swap` + preconnect to Google Fonts |

**SWR (stale-while-revalidate) everywhere**: Show last-known data instantly, update when fresh data arrives. The Skeleton loader only appears when the shape of data is truly unknown (first ever load). On subsequent loads, stale data displays immediately.

**Preconnect at app shell mount:**
```html
<link rel="preconnect" href="http://localhost:6677" />
```

**Font loading**: Fraunces and JetBrains Mono via Google Fonts with `display=swap`. Add `<link rel="preload">` for the two font weights used most (Fraunces italic 400, JetBrains Mono 500).

---

## Part 3: App Shell

### 3.1 Layout

```
┌─────────────────────────────────────────────────────────┐
│  ◆ ROKO     ORCHESTRATE  OBSERVE  EVALUATE  BUILD  ● LIVE │  ← TopNav
├─────────────────────────────────────────────────────────┤
│                                                           │
│  page content (fills remaining viewport)                  │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

**TopNav** (fixed, 56px height):
- Brand: diamond mark + "ROKO" in mono, bone color
- Nav links: mono 11px uppercase, tracking 0.06em, centered
- Active link: rose-glow text, 2px bottom border with glow (`0 2px 8px rgba(220,165,189,0.3)`)
- Inactive links: `--text-dim`, hover → `--text-soft` in 80ms
- Hover subtitle: each link shows a 3-4 word description on hover (see 1.5), fade-in 120ms
- Right: `<StatusPill>` — live/seed/reconnecting/offline with tooltip and uptime
- Background: `backdrop-filter: blur(16px) saturate(180%)`, `background: rgba(6,6,8,0.85)`
- Bottom border: `1px solid var(--border-soft)`
- **Document title prefix**: Updates to match status — `[LIVE] Roko` or `[SEED] Roko` — so a presenter alt-tabbing to the browser sees connection state in the taskbar

The TopNav is the single source of truth for "is this working?". The StatusPill answers the question a presenter always asks: "is it connected to the real backend?"

**Body**: `calc(100vh - 56px)`, no scroll on main container. Each page manages its own internal scroll if needed.

### 3.2 Routing

```typescript
// src/main.tsx
<Routes>
  <Route element={<AppShell />}>
    <Route index element={<Navigate to="/app/orchestrate" />} />
    <Route path="orchestrate" element={<OrchestratePage />} />
    <Route path="observe" element={<ObservePage />} />
    <Route path="observe/:section" element={<ObservePage />} />
    <Route path="evaluate" element={<EvaluatePage />} />
    <Route path="build" element={<BuildPage />} />
  </Route>
</Routes>
```

### 3.3 Data Layer

```typescript
// src/data/api.ts
// Single module for all API communication. No module-level singletons.

interface ApiConfig {
  baseUrl: string;
  onStatusChange: (live: boolean) => void;
}

class RokoApi {
  private live: boolean = false;
  private lastProbe: number = 0;
  private probeTTL = 30_000; // re-probe every 30s

  async probe(): Promise<boolean> { /* ... */ }

  async get<T>(path: string, fallback?: T): Promise<T> {
    // 1. Check probe freshness
    // 2. If live, fetch from server
    // 3. On network/HTTP error, return fallback
    // 4. On JSON parse error, throw (programming bug)
    // 5. Track data mode via sliding window
  }

  async post<T>(path: string, body: unknown): Promise<T | null> {
    // Returns null on failure (not {} as T)
  }
}

// React hook
export function useApi() {
  const api = useContext(ApiContext);
  return api; // { get, post, isLive, dataMode }
}
```

**SSE wrapper** with `enabled` prop, reconnect guard, clean-close detection:
```typescript
// src/data/use-sse.ts
export function useSSE(path: string, opts?: { enabled?: boolean }) {
  // See section 3.5 of 10-MASTER-CHECKLIST.md for the proper implementation
}
```

**Terminal hook** with separated listener registration:
```typescript
// src/data/use-terminal.ts
// Register xterm listeners ONCE outside connectWs()
// Use wsRef.current inside callbacks
// See section 2.1 of 10-MASTER-CHECKLIST.md
```

---

## Part 4: Pages

### 4.1 Orchestrate — "Watch Roko build something real"

**Page job:** Prove that a single request becomes a governed, routed, verified implementation.

**Hero line** (centered, Fraunces italic 24px, `--text-soft`, shown at top of page):
> "One request. Autonomous planning, routing, execution, and verification."

**One sentence for a first-time viewer:** "You type what you want built. Roko writes a PRD, generates a plan, routes tasks to the right models, verifies each with compile/test/clippy gates, and shows you every decision."

**Section end cue** (shown after completion phase, bottom-right, mono 11px, `--text-dim`):
> "Next: see what's running under the hood →  OBSERVE"

#### Layout: Phase-driven progressive disclosure

The page changes based on pipeline phase. It does NOT show everything at once.

**Phase 0 — Idle (before play):**
```
┌────────────────────────────────────────────────────────────┐
│  ┌─────────────────┐ ┌─────────────────┐ ┌──────────────┐ │
│  │ Simple status CLI│ │ GitHub release  │ │ BTC funding  │ │  ← scenario cards
│  │ Quick, local     │ │ Medium, API     │ │ Complex, DeFi│ │
│  └─────────────────┘ └─────────────────┘ └──────────────┘ │
│                                                            │
│         Build a CLI that fetches BTC funding rates          │  ← scenario description
│         from Hyperliquid and emails me an alert             │     Fraunces italic 30px
│         when funding flips negative.                        │
│                                                            │
│                    ▶  START LIVE RUN                        │  ← single CTA
│                                                            │
│  ○ idea   ○ prd   ○ plan   ○ tasks   ○ run   ○ verify      │  ← phase rail
│                                                            │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ roko prd plan btc-funding-alert-cli                   │ │  ← the CLI command
│  │ (mono 12px, bone-dim, read-only preview)              │ │     that will be run
│  └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

Nothing else. No terminals, no metrics, no sidebars. Clean and calm.

Each scenario card explains what it demonstrates in plain language:
- **Simple**: "3 tasks, 1 model, ~5 seconds — see the basic pipeline"
- **Medium**: "5 tasks, 2 models, API integration — see multi-tier routing"
- **Complex**: "6 tasks, 3 models, DeFi + email — see the full system"

The scenario description (Fraunces italic 30px) acts as the hero moment for idle phase — it makes you curious about what's going to happen.

**Phase 1-2 — Idea → PRD (artifact generation):**
```
┌────────────────────────────────────────────────────────────┐
│  ● idea   ◉ prd   ○ plan   ○ tasks   ○ run   ○ verify     │
├───────────────────────────────┬────────────────────────────┤
│                               │                            │
│  GENERATED PRD                │  TERMINAL                  │
│                               │  $ roko prd idea "..."     │
│  BTC Funding Alert CLI        │  ✓ idea captured           │
│  ───────────────────          │  $ roko prd draft new...   │
│  Build a CLI that fetches...  │  → generating PRD...       │
│                               │                            │
│  Requirements: 5              │                            │
│  Acceptances: 4               │                            │
│  Slug: btc-funding-alert-cli  │                            │
│                               │                            │
│  → next: generating plan      │                            │
│                               │                            │
└───────────────────────────────┴────────────────────────────┘
```

60/40 split. Artifact on left (the story), terminal on right (the evidence).

Each artifact field has a brief explanation of *why* it matters:
- Requirements count: "5 requirements extracted from your one-sentence request"
- Slug: "This becomes the project directory and plan identifier"

The terminal panel header says "EVIDENCE" not "TERMINAL" — framing it as proof, not just output.

**Phase 3 — Plan generated:**
```
┌────────────────────────────────────────────────────────────┐
│  ● idea   ● prd   ◉ plan   ○ tasks   ○ run   ○ verify     │
├───────────────────────────────┬────────────────────────────┤
│                               │                            │
│  GENERATED PLAN               │  ROUTING & MODELS          │
│                               │  ┌──────┬──────┬──────┐   │
│  BTC FUNDING ALERT CLI  17%   │  │ T1   │ T2   │ T3   │   │
│  btc-funding-alert-cli        │  │  2   │  3   │  1   │   │
│                               │  │haiku │sonnet│ opus │   │
│  Split the stage job into:    │  └──────┴──────┴──────┘   │
│  DeFi data ingestion, flip    │                            │
│  detection, email integration,│  GATES                     │
│  orchestration, verification. │  ○ compile  ○ test         │
│                               │  ○ clippy   ○ diff         │
│  6 tasks · 3 tiers · ~$0.05   │                            │
│                               │                            │
└───────────────────────────────┴────────────────────────────┘
```

**Phase 4-5 — Tasks running:**
```
┌────────────────────────────────────────────────────────────┐
│  ● idea  ● prd  ● plan  ● tasks  ◉ run   ○ verify         │
├────────────────────────────────────────────────────────────┤
│  TASK BOARD                                          1/6   │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ● DONE   Define CLI contract and dry-run config      │  │
│  │          T1 · implementer · claude-haiku-4-5  · 1.2s │  │
│  │          ✓ compile  ✓ test                           │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ ◉ RUN    Implement DeFi data fetcher                 │  │
│  │          T2 · implementer · claude-sonnet-4  · ...   │  │
│  │          ◉ compile  ○ test                           │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ ○ PEND   Add email notification module               │  │
│  │          T2 · implementer · claude-sonnet-4          │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ ○ PEND   Wire configuration and CLI args             │  │
│  │ ○ PEND   Integration tests with dry-run mode         │  │
│  │ ○ PEND   Final verification and smoke test           │  │
│  └──────────────────────────────────────────────────────┘  │
├──────────────────────────────┬─────────────────────────────┤
│  TERMINAL (compact, 30%)     │  METRICS                    │
│  $ roko plan run plans/      │  Cost: $0.024               │
│  → T2 implementing...        │  Time: 12.4s                │
│  [agent output streaming]    │  Tokens: 14,523             │
│                              │  Pass rate: 100%            │
└──────────────────────────────┴─────────────────────────────┘
```

Task board is the dominant element (70% of space). Terminal is compact evidence at bottom-left. Metrics at bottom-right.

Each task row in the board tells a micro-story:
- **Model badge with reason**: `"T2 · sonnet — complex implementation, requires reasoning"` (not just "sonnet")
- **Gate status inline**: `"✓ compile · ✓ test"` directly in the row, not a separate panel
- **Cost and duration live**: updates in real-time as the task runs, not just at completion
- **Expandable detail**: click a task row to see the agent's tool calls, the diff, the gate output

The metrics panel (bottom-right) uses comparative framing:
- `"$0.024 total"` + dim label: `"vs $0.18 if all on opus"`
- `"12.4s elapsed"` + dim label: `"3.1s avg per task"`
- `"100% gate pass"` + dim label: `"6 of 6 verified"`

**Phase 6 — Verification complete:**
```
┌────────────────────────────────────────────────────────────┐
│  ● idea  ● prd  ● plan  ● tasks  ● run   ● verify         │
├────────────────────────────────────────────────────────────┤
│                                                            │
│         ✓  All 6 tasks completed                           │
│         $0.042 total · 18.3s · 6/6 gates passed            │
│                                                            │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┐              │
│  │ COST │TOKENS│ TIME │GATES │MODEL │TASKS │              │
│  │$0.042│18.2K │18.3s │ 6/6  │3 tier│ 6/6  │              │
│  └──────┴──────┴──────┴──────┴──────┴──────┘              │
│                                                            │
│  TASK SUMMARY                                              │
│  ✓ Define CLI contract       T1 haiku   $0.003   1.2s     │
│  ✓ Implement DeFi fetcher    T2 sonnet  $0.017   3.4s     │
│  ✓ Add email module          T2 sonnet  $0.012   2.8s     │
│  ✓ Wire configuration        T1 haiku   $0.002   0.9s     │
│  ✓ Integration tests         T3 opus    $0.006   8.2s     │
│  ✓ Final verification        T1 haiku   $0.002   1.8s     │
│                                                            │
│  ┌────────────────────────────────────────────────────────┐│
│  │ ✓ COMPILE 5  ✓ TEST 5  ✓ CLIPPY 2  — DIFF 0          ││
│  └────────────────────────────────────────────────────────┘│
└────────────────────────────────────────────────────────────┘
```

#### Orchestrate: Implementation Spec

**Files to create:**
```
src/pages/Orchestrate.tsx        — Page component, phase state machine
src/pages/Orchestrate.css        — Phase-specific layouts
src/pages/orchestrate/
  IdlePhase.tsx                  — Scenario selector + description + CTA
  ArtifactPhase.tsx              — PRD/Plan artifact display + terminal
  TaskPhase.tsx                  — Task board + compact terminal + metrics
  CompletionPhase.tsx            — Summary mosaic + task results
  ScenarioCard.tsx               — Scenario selection card
  TaskRow.tsx                    — Single task in the task board
```

**State machine:**
```typescript
type Phase = 'idle' | 'idea' | 'prd' | 'plan' | 'tasks' | 'running' | 'complete';

interface OrchestrateState {
  phase: Phase;
  scenario: Scenario;
  prd: PrdArtifact | null;
  plan: PlanArtifact | null;
  tasks: TaskState[];
  metrics: Metrics;
  gates: Gate[];
  terminalSessionId: string;
}
```

**What earns its space on this page:**
| Element | Why it exists | What it proves |
|---|---|---|
| Scenario cards | Choice + context | "This isn't one demo — it scales across complexity levels" |
| Phase rail | Progress + position | "This is a structured pipeline, not a script" |
| Terminal | Evidence | "These are real CLI commands producing real output" |
| Task board | Depth + routing | "Different models for different tiers — cost-aware" |
| Gate bar | Verification | "Every output is automatically verified" |
| Metrics mosaic | Economics | "This cost $0.04 and took 18 seconds" |

**What does NOT belong on this page:** Provider health, fleet topology, knowledge graph, dream cycles, benchmark history, model catalog, raw JSON. Those belong in Observe/Evaluate, not here.

**Scenarios (3, progressively complex):**
1. **Simple** — "Build a status CLI that prints system uptime"
   - 3 tasks, 1 tier (haiku), ~$0.01, 5 seconds
   - Shows: basic pipeline flow
2. **Medium** — "Build a GitHub release watcher with email alerts"
   - 5 tasks, 2 tiers (haiku + sonnet), ~$0.03, 12 seconds
   - Shows: multi-tier routing, API integration
3. **Complex** — "Build a BTC funding alert CLI from Hyperliquid"
   - 6 tasks, 3 tiers (haiku + sonnet + opus), ~$0.05, 18 seconds
   - Shows: full routing, DeFi integration, email, multi-gate verification

**Playback controller (DemoController from 10-MASTER-CHECKLIST.md section 1.1):**
- Speed: 0.5x / 1x / 2x / 4x
- Pause: halts mid-sleep, resumes exactly where stopped
- Step: blocks at waitForStep() boundaries
- Reset: kills everything, returns to idle
- All timing goes through `controller.sleep(ms)` — no raw setTimeout in scenarios

**Keyboard shortcuts:**
- Space: play/pause
- N: next step (when paused at step boundary)
- R: reset
- 1-3: select scenario

### 4.2 Observe — "The Control Plane"

**Page job:** Show operational health, cost efficiency, agent fleet, knowledge, and model routing in one screen.

**Hero line** (centered, Fraunces italic 24px, `--text-soft`):
> "The system behind the system. Agents, routing, knowledge — all live."

**One sentence:** "Everything Roko is doing right now — health, agents, costs, knowledge, routing decisions — on one screen."

**Section end cue** (bottom-right, mono 11px, `--text-dim`):
> "Next: see the economic evidence →  EVALUATE"

**Tab descriptions** (shown as subtitle under each tab name):
- Status: "Health and activity"
- Fleet: "Agents and topology"
- Knowledge: "What the system knows"
- Routing: "How models are selected"
- Dreams: "Offline consolidation"

**What earns its space on this page:**
| Element | Why it exists | What it proves |
|---|---|---|
| Health mosaic | System status at a glance | "This is a production-grade system that monitors itself" |
| C-Factor breakdown | Quality composite | "Roko quantifies its own performance" |
| Agent cards | Fleet composition | "Multiple specialized agents, not one monolithic model" |
| Topology graph | Relationships | "Agents collaborate and hand off tasks" |
| Knowledge graph | Learning | "The system accumulates and reuses knowledge" |
| Routing table | Economics | "Model selection is data-driven, not hardcoded" |

**What does NOT belong on this page:** Benchmark results (Evaluate), build prompts (Build), scenario playback (Orchestrate), raw API responses, configuration forms.

#### Layout: Tab-based sub-views

```
┌────────────────────────────────────────────────────────────┐
│  OBSERVE    Status · Fleet · Knowledge · Routing · Dreams  │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  (content varies by tab)                                   │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

**Status tab (default):**
```
┌────────────────────────────────────────────────────────────┐
│  ┌────────┬────────┬────────┬────────┬────────┬────────┐  │
│  │ STATUS │ UPTIME │VERSION │C-FACTOR│ COST   │EPISODES│  │
│  │ Online │ 8h 16m │ 0.1.0 │ 0.847  │ $1.42  │  847   │  │
│  └────────┴────────┴────────┴────────┴────────┴────────┘  │
│                                                            │
│  ┌─ C-FACTOR BREAKDOWN ───────┐ ┌─ PROVIDER HEALTH ──────┐│
│  │ Gate pass rate    ████ 93% │ │ ● Anthropic   healthy  ││
│  │ Cost efficiency   ███░ 78% │ │ ● OpenAI      healthy  ││
│  │ Speed             ████ 91% │ │ ● Google      degraded ││
│  │ Reuse rate        ██░░ 64% │ │ ● Ollama      healthy  ││
│  │ Learning rate     ███░ 82% │ │ ○ Perplexity  down     ││
│  └────────────────────────────┘ └────────────────────────┘│
│                                                            │
│  ┌─ RECENT EPISODES ─────────────────────────────────────┐│
│  │ ep-020  rustsmith  wire-chain    PASS  $0.024  2.1s  ││
│  │ ep-019  fullstack  add-health    PASS  $0.017  3.2s  ││
│  │ ep-018  auditor    review-prd    PASS  $0.031  4.1s  ││
│  │ (expandable rows, auto-refresh every 10s)            ││
│  └──────────────────────────────────────────────────────┘│
│                                                            │
│  ┌─ RECENT EVENTS ──────────────────────────────────────┐│
│  │ 14:23  gate_passed     task T1    compile            ││
│  │ 14:22  agent_dispatch  rustsmith  claude-haiku       ││
│  │ 14:21  plan_started    btc-alert  6 tasks            ││
│  └──────────────────────────────────────────────────────┘│
└────────────────────────────────────────────────────────────┘
```

**Fleet tab:**
```
┌────────────────────────────────────────────────────────────┐
│  ┌──────┬──────┬──────┬──────┐                            │
│  │AGENTS│ACTIVE│ JOBS │TASKS │                            │
│  │  3   │  3   │  0   │ 827  │                            │
│  └──────┴──────┴──────┴──────┘                            │
│                                                            │
│  ┌─ AGENTS ────────────────────┐ ┌─ TOPOLOGY ───────────┐ │
│  │ ┌ rustsmith ─────────────┐  │ │                       │ │
│  │ │ implementer · T1       │  │ │   (force-directed     │ │
│  │ │ 247 tasks · $0.42      │  │ │    graph, large       │ │
│  │ │ active 2m ago          │  │ │    nodes, labeled)    │ │
│  │ └────────────────────────┘  │ │                       │ │
│  │ ┌ ethdev ────────────────┐  │ │                       │ │
│  │ │ implementer · T2       │  │ │                       │ │
│  │ │ 312 tasks · $1.18      │  │ │                       │ │
│  │ └────────────────────────┘  │ │                       │ │
│  │ ┌ auditor ───────────────┐  │ │                       │ │
│  │ │ reviewer · T3          │  │ │                       │ │
│  │ │ 268 tasks · $0.89      │  │ │                       │ │
│  │ └────────────────────────┘  │ │                       │ │
│  └─────────────────────────────┘ └───────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

Side-by-side: agent cards left (scrollable), topology canvas right. Nodes are 20px+ radius, labeled, energy-based stop, ID-sorted comparison.

Each agent card explains the agent's role in plain language:
- `"rustsmith · implementer"` + dim: `"Handles T1 scaffolding and simple implementations"`
- `"247 tasks · $0.42"` + dim: `"$0.0017 per task average"`
- Activity indicator: pulsing LED when active, steady when idle, dim when offline

The topology graph has labeled edges showing relationships. On hover, a tooltip explains: "rustsmith → ethdev: 12 task handoffs (scaffolding → implementation)".

**Knowledge tab:**
Split view: graph left, entry list right. Nodes 8+citations*1.5 radius. Glow via shadowColor on actual fill. Energy-based animation stop.

Each knowledge entry shows:
- The insight in plain language (not raw JSON)
- Citation count with explanation: `"Referenced 7 times across 3 plans"`
- Tier badge with tooltip: `"Tier 3 — core knowledge, never evicted"`

**Routing tab:**
Stats mosaic + horizontal bar chart for distribution + role→model table with confidence.

The routing table explains *why* each model is selected:
- `"claude-haiku-4.5"` | `"94% pass rate at T1"` | `"$0.001/task avg"` | `"Best for: scaffolding, config, simple transforms"`
- `"claude-sonnet-4"` | `"97% pass rate at T2"` | `"$0.008/task avg"` | `"Best for: implementation, API integration"`

The bar chart has a toggle: "By count" vs "By cost" — showing that while sonnet handles fewer tasks, it accounts for more spend. This visual is the core routing story.

**Dreams tab:**
Dream consolidation phases + journal entries + cycle progress.

Each dream phase is explained inline:
- Hypnagogia: "Reviewing recent episodes for patterns"
- Imagination: "Generating new strategies from patterns"
- Consolidation: "Compressing insights into durable knowledge"

**Files to create:**
```
src/pages/Observe.tsx            — Tab container
src/pages/observe/
  StatusView.tsx                 — Health + C-factor + episodes + events
  FleetView.tsx                  — Agent cards + topology canvas
  KnowledgeView.tsx              — Graph + entries split
  RoutingView.tsx                — Model routing table + distribution
  DreamsView.tsx                 — Dream consolidation
```

**API endpoints consumed:**
```
GET /api/health                  → status, uptime, version
GET /api/metrics/c_factor        → composite score + sub-metrics
GET /api/managed-agents          → agent list
GET /api/agents/topology         → node/edge graph
GET /api/knowledge/entries       → knowledge store
GET /api/knowledge/edges         → graph edges
GET /api/learn/cascade-router    → routing decisions
GET /api/episodes                → episode log
GET /api/events                  → state events (SSE)
GET /api/providers/health        → provider status
```

### 4.3 Evaluate — "The Evidence"

**Page job:** Show economic evidence that Roko's routing, gating, and caching produce real cost savings and quality improvements.

**Hero line** (centered, Fraunces italic 24px, `--text-soft`):
> "Proof, not promises. Cost, quality, and speed — measured."

**One sentence:** "Run benchmarks, see pass rates, compare models, understand the cost-quality Pareto frontier."

**Section end cue** (bottom-right, mono 11px, `--text-dim`):
> "Next: try it yourself →  BUILD"

#### Layout: Tab-based

```
Tabs: Configure · Live · Results · History · Pareto
```

**Configure tab:**
```
┌────────────────────────────────────────────────────────────┐
│  ┌──────────┬──────────┬──────────┬──────────┐            │
│  │TOTAL RUNS│ PASS RATE│TOTAL COST│  SUITES  │            │
│  │    3     │   96%    │  $0.30   │    4     │            │
│  └──────────┴──────────┴──────────┴──────────┘            │
│                                                            │
│  ┌─ TEST SUITE ──────────────────────────────────────────┐│
│  │ [Smoke]  Learnable Rust   Roko Bench   Codegen        ││
│  │ 5 tasks  6 tasks          8 tasks      10 tasks       ││
│  └──────────────────────────────────────────────────────┘│
│                                                            │
│  ┌─ STRATEGY ────────────────────────────────────────────┐│
│  │ Minimal   Context-Enriched   Neuro-Augmented   [Full] ││
│  └──────────────────────────────────────────────────────┘│
│                                                            │
│  ┌─ MODEL ───────────────────────────────────────────────┐│
│  │ Claude Sonnet 4 ($0.003/1k in, $0.015/1k out)    ▼   ││
│  └──────────────────────────────────────────────────────┘│
│                                                            │
│  ┌─ PARAMETERS ──────────────────────────────────────────┐│
│  │ Temperature: 0.1    Max tokens: 8192    Timeout: 120  ││
│  │ Retries: 1          Gates: ✓compile ✓test ✓clippy     ││
│  └──────────────────────────────────────────────────────┘│
│                                                            │
│  Estimated: $0.05 (5 tasks, full cascade)                  │
│  [EXPORT]  [RUN BENCHMARK]                                 │
└────────────────────────────────────────────────────────────┘
```

**Results tab:**
Same as current but with fixed issues: BenchRunDetail missing-ID guard, token pricing labels, navigation links from history.

Each result row includes comparative context:
- `"3.4s"` + dim: `"2× faster than opus baseline"`
- `"$0.017"` + dim: `"at 100% pass rate"`
- Expandable: click to see the agent's actual output, the diff, the gate log

**History tab:**
Run history table with one-click actions:
- "View" opens full results
- "Compare" adds to comparison set (max 3, preventing same-run selection)
- "Export" downloads JSON
- Each row shows: date, suite, model, pass rate sparkline, total cost

Empty state: "No benchmark runs yet. Configure and run your first benchmark in the Configure tab."

**Pareto tab:**
Interactive scatter plot (cost vs. pass rate). Each dot is a run. Pareto frontier curve highlighted.

The Pareto chart is the centerpiece of the economic argument. It should:
- Label the frontier runs with model names
- Show the "naive" point (all-opus) vs the "routed" point (cascade router)
- Include an annotation line: "Roko's routing achieves 96% quality at 40% of naive cost"
- On dot hover: show run details (date, suite, model, pass rate, cost)

**Files to create:**
```
src/pages/Evaluate.tsx            — Tab container
src/pages/evaluate/
  ConfigureView.tsx               — Suite/strategy/model/params
  LiveView.tsx                    — Active run feed
  ResultsView.tsx                 — Run summary + task table
  HistoryView.tsx                 — All runs with view/compare links
  ParetoView.tsx                  — Cost vs quality scatter
```

### 4.4 Build — "Type a request, Roko builds it"

**Page job:** Direct interaction surface. Type what you want, watch Roko build it in real-time.

**Hero line** (centered, Fraunces italic 24px, `--text-soft`):
> "Your turn. Describe what to build. Roko handles the rest."

**One sentence:** "A prompt input, a terminal, and a file tracker. Type what to build, pick a model, watch it happen."

#### Layout: Two modes

**Before build (input-focused):**
```
┌────────────────────────────────────────────────────────────┐
│  BUILD                                    model: haiku ▼   │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  calculator · REST API · md-html · dedup · commitgen       │  ← presets (flex-wrap)
│  web scraper · test harness · config parser · ...          │
│                                                            │
│                                                            │
│  ┌────────────────────────────────────────────── [Build] ┐ │
│  │ describe what to build...                             │ │  ← input, 16px
│  └───────────────────────────────────────────────────────┘ │
│                                                            │
│  ○ compile   ○ test   ○ clippy   ○ diff                    │  ← gate bar
│                                                            │
└────────────────────────────────────────────────────────────┘
```

**During build (terminal-focused):**
```
┌────────────────────────────────────────────────────────────┐
│  BUILD  "Build a CLI calculator in Rust"   model: haiku    │
├──────────────────────────────────┬─────────────────────────┤
│                                  │                         │
│  TERMINAL                        │  FILES                  │
│  $ roko run "Build a CLI..."     │  + src/main.rs          │
│  → generating plan...            │  + Cargo.toml           │
│  → compiling...                  │  + src/lib.rs           │
│  ✓ compile passed                │  + tests/test_calc.rs   │
│  → testing...                    │                         │
│  ✓ test passed                   │                         │
│                                  │                         │
├──────────────────────────────────┴─────────────────────────┤
│  ✓ compile   ✓ test   ◉ clippy   ○ diff                   │
└────────────────────────────────────────────────────────────┘
```

Terminal takes 65% width, file list 35%. Gate bar at bottom.

The input-focused mode is designed for zero-friction entry:
- Presets are labeled as `"try: calculator · REST API · dedup · ..."` so it's clear they're clickable examples
- Clicking a preset fills the input AND shows a dim preview: `"Will run: roko run \"Build a CLI calculator in Rust\" --model haiku"`
- The model picker explains each option: `"haiku — fast, cheap ($0.001/task) · sonnet — balanced ($0.008/task) · opus — powerful ($0.03/task)"`
- The Build button is disabled until input is non-empty, with placeholder text: "Describe what to build..."

The terminal-focused mode provides real-time narration:
- File tracker shows files as they appear in terminal output: `"+ src/main.rs"` with a fade-in animation
- Gate bar updates in real-time with pass/fail animations
- On completion, a summary appears below the terminal: `"✓ Built in 8.3s · 4 files created · $0.012 · all gates passed"`
- A "Build Again" button appears to return to input mode

**Files to create:**
```
src/pages/Build.tsx               — Build page
src/pages/build/
  PromptInput.tsx                 — Input with presets and model picker
  FileTracker.tsx                 — Files created/modified list
```

---

## Part 5: File Structure

```
demo-current/
├── index.html                    — Vite entry point
├── package.json                  — Dependencies
├── tsconfig.json
├── vite.config.ts               — Proxy /api, /ws to :6677
│
├── src/
│   ├── main.tsx                 — Routes + AppShell
│   │
│   ├── design/                  — Design system components
│   │   ├── tokens.css           — CSS custom properties
│   │   ├── global.css           — Reset, body, atmospheric layers
│   │   ├── Pane.tsx             — Glass panel
│   │   ├── Mosaic.tsx           — Metric grid
│   │   ├── MosaicCell.tsx       — Single metric cell
│   │   ├── Led.tsx              — Status dot
│   │   ├── StagLabel.tsx        — Section label
│   │   ├── Table.tsx            — Data table
│   │   ├── GateBar.tsx          — Gate status strip
│   │   ├── Timeline.tsx         — Phase rail
│   │   ├── Terminal.tsx         — xterm wrapper
│   │   ├── Skeleton.tsx         — Loading placeholder
│   │   ├── EmptyState.tsx       — Empty data message (what, why, what to do)
│   │   ├── Term.tsx             — Inline glossary tooltip
│   │   ├── StatusPill.tsx       — Connection status indicator
│   │   ├── HealthBar.tsx        — Developer debug panel (?debug=true)
│   │   ├── SectionHero.tsx      — Page intro line + next-section cue
│   │   ├── ErrorBoundary.tsx    — Error boundary (component name, error, reload)
│   │   ├── AppShell.tsx         — Nav + body + overlays
│   │   └── TopNav.tsx           — Navigation bar with StatusPill
│   │
│   ├── data/                    — Data layer
│   │   ├── api.ts               — RokoApi class + useApi hook
│   │   ├── use-sse.ts           — SSE hook
│   │   ├── use-terminal.ts      — Terminal handle hook
│   │   ├── terminal-session.ts  — setupWorkspace, showCmd, etc.
│   │   ├── seed-data.ts         — Fallback demo data
│   │   ├── bench-data.ts        — Benchmark seed data
│   │   └── model-catalog.ts     — Model definitions
│   │
│   ├── pages/
│   │   ├── Orchestrate.tsx
│   │   ├── orchestrate/
│   │   │   ├── IdlePhase.tsx
│   │   │   ├── ArtifactPhase.tsx
│   │   │   ├── TaskPhase.tsx
│   │   │   ├── CompletionPhase.tsx
│   │   │   ├── ScenarioCard.tsx
│   │   │   ├── TaskRow.tsx
│   │   │   ├── scenarios.ts      — 3 scenario definitions
│   │   │   └── demo-controller.ts — Playback state machine
│   │   │
│   │   ├── Observe.tsx
│   │   ├── observe/
│   │   │   ├── StatusView.tsx
│   │   │   ├── FleetView.tsx
│   │   │   ├── KnowledgeView.tsx
│   │   │   ├── RoutingView.tsx
│   │   │   └── DreamsView.tsx
│   │   │
│   │   ├── Evaluate.tsx
│   │   ├── evaluate/
│   │   │   ├── ConfigureView.tsx
│   │   │   ├── LiveView.tsx
│   │   │   ├── ResultsView.tsx
│   │   │   ├── HistoryView.tsx
│   │   │   └── ParetoView.tsx
│   │   │
│   │   └── Build.tsx
│   │       └── build/
│   │           ├── PromptInput.tsx
│   │           └── FileTracker.tsx
│   │
│   ├── lib/
│   │   ├── rosedust-theme.ts    — xterm color theme
│   │   ├── config.ts            — SERVE_URL, MIRAGE_URL from env
│   │   ├── gate-detector.ts     — Unified gate pattern matching
│   │   └── types.ts             — Shared TypeScript interfaces
│   │
│   └── styles/
│       └── (empty — all styles in design/tokens.css and global.css)
│
└── public/
    └── (static assets if any)
```

**Total file count: ~50 files** (vs. current ~80). Simpler, more focused, but more intentional about UX polish.

---

## Part 6: Implementation Checklist

### Phase 1: Foundation (design system + app shell)

- [ ] Initialize Vite + React 19 + TypeScript project in `demo-current/`
- [ ] Install dependencies: `react`, `react-dom`, `react-router`, `@xterm/xterm`, `@xterm/addon-fit`, `three`
- [ ] Create `src/design/tokens.css` — all CSS custom properties including shadows (specular highlight), motion (easing curves, durations), focus ring
- [ ] Create `src/design/global.css` — reset, body, atmospheric layers (grain SVG filter, scanlines, vignette), keyframe animations, `:focus-visible` global style, `prefers-reduced-motion` media query
- [ ] Import Google Fonts: Fraunces + JetBrains Mono
- [ ] Create `src/design/Pane.tsx` — glass panel component
- [ ] Create `src/design/Mosaic.tsx` + `MosaicCell.tsx` — metric grid
- [ ] Create `src/design/Led.tsx` — status dot
- [ ] Create `src/design/StagLabel.tsx` — section label
- [ ] Create `src/design/Table.tsx` — data table
- [ ] Create `src/design/GateBar.tsx` — gate status strip (always renders)
- [ ] Create `src/design/Timeline.tsx` — horizontal phase rail
- [ ] Create `src/design/Skeleton.tsx` — loading placeholder
- [ ] Create `src/design/EmptyState.tsx` — empty data message with action and hint props
- [ ] Create `src/design/Term.tsx` — inline glossary tooltip (dotted underline, first-show tracking)
- [ ] Create `src/design/StatusPill.tsx` — live/seed/reconnecting/offline indicator
- [ ] Create `src/design/HealthBar.tsx` — developer debug panel (hidden by default, `?debug=true`)
- [ ] Create `src/design/SectionHero.tsx` — page intro line + next-section navigation cue
- [ ] Create `src/design/ErrorBoundary.tsx` — per-section error boundary with component name and reload button
- [ ] Create `src/design/TopNav.tsx` — navigation bar with StatusPill, tab hover subtitles
- [ ] Create `src/design/AppShell.tsx` — nav + outlet + atmospheric overlays + HealthBar
- [ ] Create `src/main.tsx` — routes
- [ ] Create `vite.config.ts` — proxy /api and /ws to :6677
- [ ] Verify: `npm run build` passes, app renders shell with 4 nav links

### Phase 2: Data layer

- [ ] Create `src/lib/config.ts` — SERVE_URL, MIRAGE_URL from env vars
- [ ] Create `src/data/api.ts` — RokoApi class with probe TTL, error discrimination, sliding window data mode
- [ ] Create `src/data/use-sse.ts` — SSE hook with enabled prop, reconnect guard, clean-close detection
- [ ] Create `src/data/use-terminal.ts` — Terminal hook with separated listener registration, no tput colors
- [ ] Create `src/data/terminal-session.ts` — setupWorkspace, showCmd, trackMetrics, getRoko with promise-based lock
- [ ] Create `src/data/seed-data.ts` — curated seed data for all endpoints
- [ ] Create `src/data/bench-data.ts` — benchmark seed data
- [ ] Create `src/data/model-catalog.ts` — model definitions
- [ ] Create `src/lib/rosedust-theme.ts` — xterm theme
- [ ] Create `src/lib/gate-detector.ts` — unified gate pattern matching
- [ ] Create `src/lib/types.ts` — shared interfaces
- [ ] Verify: API class probes server, falls back to seed data, re-probes on TTL

### Phase 3: Orchestrate page

- [ ] Create `src/pages/orchestrate/demo-controller.ts` — DemoController class (speed, pause, step, sleep, reset)
- [ ] Create `src/pages/orchestrate/scenarios.ts` — 3 scenarios (simple, medium, complex) using DemoController
- [ ] Create `src/pages/orchestrate/ScenarioCard.tsx`
- [ ] Create `src/pages/orchestrate/IdlePhase.tsx` — scenario selector + description + CTA + phase rail
- [ ] Create `src/pages/orchestrate/ArtifactPhase.tsx` — PRD/plan artifact + terminal 60/40
- [ ] Create `src/pages/orchestrate/TaskRow.tsx` — single task with status, model, cost, duration, gates
- [ ] Create `src/pages/orchestrate/TaskPhase.tsx` — task board + compact terminal + metrics
- [ ] Create `src/pages/orchestrate/CompletionPhase.tsx` — summary mosaic + task results
- [ ] Create `src/pages/Orchestrate.tsx` — phase state machine, keyboard shortcuts
- [ ] Create `src/pages/Orchestrate.css` — phase-specific layouts with transitions
- [ ] Verify: full pipeline flow from idle → select scenario → play → phases advance → completion
- [ ] Verify: speed button changes pacing, pause halts mid-sleep, step works, reset returns to idle
- [ ] Verify: all 3 scenarios run without crashes

### Phase 4: Observe page

- [ ] Create `src/pages/observe/StatusView.tsx` — health + C-factor breakdown + episodes + events
- [ ] Create `src/pages/observe/FleetView.tsx` — agent cards + topology canvas (Promise.all polling, sorted comparison, energy-based stop)
- [ ] Create `src/pages/observe/KnowledgeView.tsx` — graph + entry list split (fixed glow, capped animation)
- [ ] Create `src/pages/observe/RoutingView.tsx` — routing distribution + role table
- [ ] Create `src/pages/observe/DreamsView.tsx` — dream phases + journal
- [ ] Create `src/pages/Observe.tsx` — tab container with sub-views
- [ ] Verify: all tabs render with seed data and with live data
- [ ] Verify: no animation CPU drain (energy-based stop), no layout shifts

### Phase 5: Evaluate page

- [ ] Create `src/pages/evaluate/ConfigureView.tsx` — suite/strategy/model/params
- [ ] Create `src/pages/evaluate/LiveView.tsx` — active run feed via SSE
- [ ] Create `src/pages/evaluate/ResultsView.tsx` — run summary + task table + cost breakdown
- [ ] Create `src/pages/evaluate/HistoryView.tsx` — all runs with view/compare links
- [ ] Create `src/pages/evaluate/ParetoView.tsx` — cost vs quality scatter
- [ ] Create `src/pages/Evaluate.tsx` — tab container, useBench hook
- [ ] Verify: configure → run → results flow, export/import, comparison (no same-run selection)

### Phase 6: Build page

- [ ] Create `src/pages/build/PromptInput.tsx` — input with presets (flex-wrap), model picker
- [ ] Create `src/pages/build/FileTracker.tsx` — file list that updates as files are detected in terminal output
- [ ] Create `src/pages/Build.tsx` — two-mode layout (input-focused → terminal-focused)
- [ ] Verify: preset fills input, build command includes selected model, gates update, terminal fills space

### Phase 7: Polish & UX Pass

- [ ] Typography audit — all text ≥ 11px, table ≥ 12px, body ≥ 13px
- [ ] Empty states — every data-dependent panel has EmptyState with message + action + hint
- [ ] Error boundaries — every major section wrapped in ErrorBoundary with component name + reload button
- [ ] Loading states — Skeleton components while data loads (no blank screens ever)
- [ ] Glossary pass — every domain term (Gate, Tier, PRD, C-Factor, etc.) wrapped in `<Term>` on first use
- [ ] Hero lines — every section has SectionHero with intro line + next-section cue
- [ ] Contextual labels — every metric has unit label, every routing decision has reason, every cost has comparative frame
- [ ] Feedback pass — every button click, tab switch, card selection produces immediate visual response
- [ ] Hover states — every interactive element has hover feedback, every metric has explanatory tooltip
- [ ] Keyboard navigation — all interactive elements have focus styles, tabIndex, aria-labels
- [ ] Transitions — fadeUp on phase transitions in Orchestrate, fadeIn on tab switches
- [ ] StatusPill — verify all 4 modes work (live, seed, reconnecting, offline) with correct tooltips
- [ ] HealthBar — verify `?debug=true` activates it, shows useful diagnostic info
- [ ] Error messages — audit every catch/error path, ensure user-facing message with what happened + what to do
- [ ] Responsive — graceful degradation at 1024px and 768px
- [ ] "Someone else" test — can a person unfamiliar with roko navigate and understand every screen in < 3 seconds?
- [ ] `npm run build` — 0 errors, 0 warnings
- [ ] Visual audit — every page ≥ 8.0 on the rubric from visual-iteration-prompt.md
- [ ] Usability audit — every page ≥ 8.0 on the usability rubric from Part 9.1
- [ ] Craft audit — verify all 12 craft details from section 2.2a are implemented:
  - [ ] Specular top-edge highlights on all elevated surfaces
  - [ ] Borders use rgba white, not hex
  - [ ] Hover transforms are ≤2px translateY
  - [ ] Active/pressed has asymmetric timing (50ms press / 120ms release)
  - [ ] Value changes trigger flash animation
  - [ ] List items stagger in at 40ms intervals
  - [ ] Skeleton shimmer uses 0.03–0.07 opacity range
  - [ ] Tooltips combine opacity + transform + scale
  - [ ] Glass panels use `saturate(180%)` in backdrop-filter
  - [ ] No `transition: all` anywhere — always specific properties
  - [ ] `will-change: transform` on hover-animated elements
  - [ ] No animation exceeds 400ms
- [ ] Atmospheric layers — grain visible at 4% opacity, scanlines at 6%, vignette present
- [ ] Focus ring — double-ring pattern visible on Tab navigation through all interactive elements
- [ ] `?` shortcut — keyboard help overlay appears and lists all shortcuts
- [ ] Document title — prefix updates with connection status ([LIVE] / [SEED])
- [ ] Performance — FCP < 1.0s, LCP < 1.5s, hover feedback < 16ms (test on a mid-range laptop, not just M3 Max)

---

## Part 7: What NOT to Build

Things that were in the old app and should NOT be carried forward:

**Architecture anti-patterns:**
1. **No ConnectScreen overlay** — dead code, never used
2. **No auto-play on load** — page starts calm, user clicks Start
3. **No `rawSleep()`** — all timing through DemoController
4. **No module-level singletons** (PlaybackController, TimelineStepper, globalSpeed) — use class instances
5. **No `{} as T` return from post failures** — return null
6. **No cumulative `_seedCount`/`_nonSeedCount`** — use sliding window
7. **No `tput colors` rainbow bar** — clear after probe
8. **No duplicate routes** — one route per view
9. **No standalone Terminal page** — terminal is embedded in Build and Orchestrate
10. **No inline `<style>` tags** — all animations in CSS files
11. **No `void setFoo` lint suppressions** — delete unused code, don't suppress

**UX anti-patterns:**
12. **No blank screens** — every loading state has a Skeleton, every empty state has an EmptyState with action text
13. **No mystery buttons** — every button label says what happens when clicked. "Build" not "Go". "Run Benchmark" not "Start".
14. **No unexplained jargon** — first use of domain terms gets a `<Term>` tooltip
15. **No silent failures** — every error produces a visible message with what happened and what to do
16. **No raw technical data** — numbers always have labels, units, and comparative context
17. **No dead-end screens** — every page suggests what to do next (section cue, action button, empty state hint)
18. **No 8px/9px/10px body text** — minimum 11px labels, 12px table, 13px body
19. **No `position: fixed` on page-level elements** — use `position: absolute` relative to page container
20. **No WorkflowConstellation (Three.js)** on Orchestrate — the task board tells the story better

---

## Part 8: Migration Notes

### From demo-app → demo-current

| demo-app file | demo-current equivalent | Notes |
|---|---|---|
| `lib/scenarios.ts` (75K) | `pages/orchestrate/scenarios.ts` | Rewrite with DemoController, 3 scenarios instead of 15 |
| `hooks/useTerminal.ts` | `data/use-terminal.ts` | Fix B1 (listener sep), B4 (status), B30 (timer), B31 (decoder) |
| `hooks/useApiWithFallback.ts` | `data/api.ts` | Fix B5 (TTL), B6 (error discrimination), U12-U14 |
| `hooks/useSSE.ts` | `data/use-sse.ts` | Fix B27-B29 |
| `hooks/useBench.ts` | Inline in Evaluate page | Fix B8 (double-start), simplify |
| `styles/rosedust.css` | `design/tokens.css` + `design/global.css` | Extract tokens, clean up aliases |
| `components/Pane.tsx` | `design/Pane.tsx` | Same concept, cleaner implementation |
| `components/Mosaic.tsx` | `design/Mosaic.tsx` | Same concept |
| `components/TopNav.tsx` | `design/TopNav.tsx` | 4 links instead of 6, fix A4 |
| `pages/dashboard/Layout.tsx` | `pages/Observe.tsx` | Tab container, fix A5, V9 |
| `pages/dashboard/CostDashboard.tsx` | `pages/observe/StatusView.tsx` | Fix U7, U8, U9, PERF1, PERF2 |
| `pages/dashboard/AgentFleet.tsx` | `pages/observe/FleetView.tsx` | Fix B36, B37, B38, PERF3 |
| `pages/dashboard/KnowledgeGraph.tsx` | `pages/observe/KnowledgeView.tsx` | Fix B18, B19, B20 (already done in current) |
| `pages/Demo.tsx` (700 lines) | `pages/Orchestrate.tsx` (~200 lines) + sub-components | Phase state machine, no 16-field context |
| `lib/playback-controller.ts` | `pages/orchestrate/demo-controller.ts` | Single class, no dead auto-mode |

### Bugs automatically fixed by the new architecture

| Bug | How it's fixed |
|---|---|
| B1 (listener accumulation) | Listeners registered once, outside connectWs() |
| B2 (Promise.all null guards) | 3 clean scenarios, no raw Promise.all with unchecked entries |
| B4 (onerror sets 'connected') | Fixed in use-terminal.ts |
| B5 (probe cached forever) | 30s TTL in RokoApi |
| B6 (catch swallows all errors) | Error discrimination in api.ts |
| B7 (ChainView timer leak) | ChainView not in new app (chain is Phase 2+) |
| B8 (useBench double-start) | Clear interval at start of startRun |
| B9/B32 (resolveRoko TOCTOU) | Promise-based lock in terminal-session.ts |
| B10 (health false positive) | No fake 'connected' state |
| B11 (null handles) | Readiness check before play |
| B12 (useEffect no deps) | Proper dependency arrays |
| B13/B14/B34 (unmount leaks) | AbortController in Orchestrate |
| B15 (interval not in try/finally) | ScenarioCleanup class |
| B16 (duplicate routes) | No duplicate routes |
| B17 (reconnectTimer uninit) | Explicit undefined init |
| B21/B22 (GateWaterfall) | roundRect polyfill, clear on empty |
| B27 (useSSE timer leak) | clearTimeout before reassign |
| B28/B29 (SSE clean close/offline) | enabled prop, readyState check |
| B30 (reconnectTimer leak) | clearTimeout before reassign |
| B31 (TextDecoder per message) | Created once outside handler |
| B35 (ciBlocks/ciPositions useState) | Module-level constants |
| P1/P2 (speed button, rawSleep) | DemoController.sleep() |
| P3 (pause during showCmd) | DemoController respects pause |
| P4 (race/providers bypass controls) | All scenarios use DemoController |
| P6 (stale callbacks) | Generation counter guard |
| U9 (inline keyframe) | CSS file |
| U11/DC1 (ConnectScreen dead) | Not included |
| DC2-DC6 (dead code) | Not carried forward |

---

## Part 9: Visual Quality Targets

Each page should score ≥ 8.0 on the 10-point rubric from `visual-iteration-prompt.md`:

| Criterion | Target | How |
|---|---|---|
| First-time comprehension | 9 | Phase-driven progressive disclosure, one focal point per phase |
| Signal-to-noise | 9 | Every element earns its space per right-to-exist rule |
| Information scent | 9 | Contextual narration labels, comparative framing, inline glossary |
| Artifact clarity | 9 | PRD, plan, tasks, routing, gates are visually distinct |
| State legibility | 9 | Color + icon + position for pending/running/done/fail |
| Visual hierarchy | 9 | One focal point per viewport, secondary detail subordinate |
| Rosedust fidelity | 9 | Typography, spacing, color match nunchi_5.html reference |
| Investor-grade polish | 9 | Alignment, type scale, contrast feel Series A ready |
| WebGL value | 8 | Topology graph only where it explains fleet structure |
| Technical honesty | 10 | No fake artifacts, no broken state, no console errors |

**Overall target: 9.0+ average across all pages.**

### 9.1 Usability Quality Targets

Beyond visual polish, these are the usability standards:

| Criterion | Target | How |
|---|---|---|
| Zero-instruction navigation | 10 | Anyone can find and use every feature without reading docs |
| Error recovery | 9 | Every error state has: what happened, why, what to do next |
| Load-state coverage | 10 | No blank screens — every async operation shows loading/skeleton |
| Empty-state coverage | 10 | Every data-dependent panel has actionable EmptyState |
| Feedback latency | 9 | Every click produces visible response in <100ms |
| Self-diagnosing | 9 | StatusPill + HealthBar + informative errors = always know system state |
| Demo-able by anyone | 9 | Non-technical person can run full demo and explain what they saw |
| Narrative coherence | 9 | Sections build on each other, hero lines + cues create story arc |
| Glossary coverage | 9 | Every domain term has a `<Term>` tooltip on first use |
| Affordance clarity | 9 | Every interactive element looks interactive, disabled elements look disabled |
