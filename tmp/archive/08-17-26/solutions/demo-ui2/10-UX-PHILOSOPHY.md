# 10. UX Philosophy, Anti-Patterns & Working Methodology

Core UX principles, explicit prohibitions, implementation methodology, and quality checklists. Synthesized from the next-gen spec (Part 0, Part 7) and the agent implementation prompt.

---

## 1. Core UX Principles

### 1.1 Someone Else Demos This

The demo app must work for **anyone**, not just the builder. A board member, a potential hire, an investor's technical advisor — anyone should be able to:

1. Click "Start" on the landing page
2. Understand where they are and what they're looking at within 3 seconds of any page load
3. Know what to click next at every moment
4. Run a complete demo without reading documentation, watching a tutorial, or asking questions
5. Explain what they saw to someone else afterward

This means: no jargon without inline explanation, no blank screens, no mystery buttons, no hidden state.

### 1.2 Do Fewer Things, Perfectly

4 things, done completely:

1. **Orchestrate**: Show how Roko turns a request into verified code
2. **Observe**: Show the operational control plane
3. **Evaluate**: Show the economic evidence
4. **Build**: Let someone try it themselves

Each section should feel *complete* — not a dashboard of partially-wired widgets, but a focused, polished experience that tells one story really well.

### 1.3 Guided Experience with Escape Hatches

**Layer 1 — The guided path** (90% of viewers): Linear navigation, hero moments, one primary action per screen, contextual labels that narrate what's happening.

**Layer 2 — The depth** (technical person who digs in): Expandable rows, detail panels, raw JSON, parameter tuning, terminal output. Always one click deeper — never clutters the primary view.

### 1.4 Visual Feedback Everywhere

| User action | Immediate feedback |
|---|---|
| Click scenario card | Card highlights, description animates in, CTA appears |
| Click "Start" | Phase rail lights up, terminal appears, first command runs |
| Task starts executing | Task row pulses, model badge appears, timer starts |
| Gate passes | Green check fades in with micro-animation |
| Gate fails | Rose flash, failure reason appears inline |
| Server disconnects | Status pill turns amber, "Reconnecting..." label |
| Server reconnects | Status pill turns green, stale data refreshes |
| Data loads | Skeleton → real data with fadeUp |
| No data available | EmptyState with actionable message |

### 1.5 Self-Diagnosing

Connection status always visible in top nav:
- `● LIVE 8h 16m` — connected, real-time
- `● SEED DATA` — server not running, showing demo data
- `● RECONNECTING...` — lost connection, attempting restore
- `● OFFLINE` — can't reach server

Error states are informative: "Could not load agent fleet — server returned 500. Try refreshing or check roko serve logs." Not "Error."

Health bar in developer mode (`?debug=true`): API response times, SSE state, terminal session IDs, data mode.

### 1.6 Time-to-Wow

TtW must be under 90 seconds:

1. Landing page loads (0s)
2. Click "Start" → app shell loads (2s)
3. Orchestrate shows 3 scenario cards (3s)
4. Click scenario → description + CTA (5s)
5. Click START → terminal + phase rail (8s)
6. PRD generates real-time (20s)
7. Plan generates, task board appears (35s)
8. First task completes with gate passes (50s)
9. **WOW: One-sentence request → verified multi-file implementation in 75s for $0.04**

Second WOW: Pareto chart in Evaluate (economic thesis). Third WOW: Build (type your own request, watch it happen — proof it's real).

### 1.7 Narrative Arc

1. **Orchestrate**: "Here's what Roko does." Takeaway: *works end-to-end, autonomously*
2. **Observe**: "Here's what's under the hood." Takeaway: *real system with operational depth, not a script*
3. **Evaluate**: "Here's the evidence." Takeaway: *economically superior*
4. **Build**: "Try it yourself." Takeaway: *this is real, I could use it now*

Every section has a one-sentence summary at the top and a cue pointing to the next section.

### 1.8 What "Intuitive" Means Concretely

- **Navigation**: 4 items, always visible, current highlighted. No hamburger menus, no hidden drawers.
- **Layout**: Same grid system. Content centered in max-width container.
- **Color**: Green = good. Rose/red = failed. Amber = in-progress/warning. Blue-grey = pending. Only semantic colors.
- **Icons**: ● filled = done, ◉ ring = active, ○ hollow = pending, ✓ = pass, ✕ = fail. Only these.
- **Labels**: Every non-obvious element labeled. Mono 10px uppercase. Never ambiguous: "PASS RATE" not "RATE".
- **Affordances**: Buttons look like buttons. Links look like links. Cards look clickable. Disabled = opacity 0.4.
- **Whitespace**: Generous. 48–64px between sections, 24–32px between groups.
- **Consistency**: Same data always looks the same everywhere.

### 1.9 The Right-to-Exist Test

Every element must answer: **"What decision or action does this enable that would be impossible or meaningfully harder without it?"** If "none," remove it.

Elements that commonly fail: decorative empty-state illustrations, "Welcome, User!" headers, progress bars on static pages, status badges that never change, metric cards with no baseline comparison, logos in corners, excessive chart grid lines.

### 1.10 Data-Ink Ratio

From Tufte: maximize `ink used for data / total ink`. Rules:

- Remove chart grid lines — axis scales suffice
- No chart borders where the grid provides framing
- No legend labels that duplicate axis labels — use direct labeling
- No 3D effects, shadows, or gradients on chart elements
- No pie charts for more than 3 segments
- If a label and icon say the same thing, keep one

### 1.11 Cognitive Load Management

From Sweller's cognitive load theory:

- **Miller's Law**: Working memory holds ~7±2 chunks. Dashboards with >9 independent elements exceed capacity. Solution: chunking (Mosaic is a chunk, GateBar is a chunk).
- **Visual hierarchy offloads working memory**: Importance encoded by size, color saturation, position.
- **Consistency reduces load**: Learned patterns transfer across pages.
- **Recognition over recall**: Persist filter state in URLs. Display recently-used queries. Show current parameters.

### 1.12 Design Principles from Best Developer Tools

1. **Performance is the first design decision, not the last.** Preconnect. Memoize. Prefetch. SWR for all data. Skeletons only when shape is unknown. Spinners as last resort.
2. **Empty states are actionable, not decorative.** Show the exact `roko` CLI command that populates the panel.
3. **Information density should match the user.** Developers scan, not read. Dense tables beat card grids.
4. **Show the equivalent CLI command for every action.** Builds trust and teaches the CLI.
5. **Status must be visible without focus switching.** TopNav pill, document title prefix, per-section indicators.
6. **Refresh rate must match data change frequency.** SSE pushes real changes. Don't auto-refresh when data changes per-minute.

---

## 2. UX Anti-Patterns — Hard Prohibitions

These are concrete failure modes. **Implementers must treat these as prohibitions.**

### Layout & Hierarchy

**AP-01: The Equal-Weight Grid.** Every widget same size/weight. Fix: primary KPI 2-3× more prominent. Z-pattern: top-left gets most attention.

**AP-02: Navigation That Eats Content.** 260px sidebar with 40+ items. Fix: horizontal top nav, 56px tall, 4 items. 100% horizontal space for content.

**AP-03: Scroll Dependency.** Critical info below the fold. Fix: most important metric and primary CTA visible on load at 768px height.

### Information & Data

**AP-04: Context-Free Metrics.** "Latency: 340ms" means nothing. Fix: every metric has comparative frame — vs baseline, vs target, vs naive alternative.

**AP-05: Charts That Don't Answer a Question.** Fix: before creating any visualization, write the one-sentence question it answers.

**AP-06: Stacked Charts.** Distort individual series values. Fix: separate vertically-aligned single-axis charts.

**AP-07: Dual Y-Axis Charts.** Create spurious correlations. Fix: separate charts, aligned on time axis.

### Feedback & State

**AP-08: Loading State Theater.** Full-page spinners, wrong-shape skeletons, fake progress bars. Fix: SWR, correctly-shaped skeletons, determinate progress when possible.

**AP-09: Hiding Actions Behind Hover.** Fix: always-visible action buttons (possibly dimmed), or consistent ⋯ menu icon.

**AP-10: Silent State Transitions.** Value changes without visual signal (change blindness). Fix: 200-300ms highlight animation on value change.

**AP-11: Non-Streaming Output.** Fix: stream terminal output via WebSocket, character by character.

### Interaction

**AP-12: Mouse-Only Interactions.** Fix: keyboard shortcuts for all primary actions. Space to play/pause, N for next, R for reset, 1-3 for scenarios, ? for help.

**AP-13: Animation That Adds Latency.** Fix: animations under 300ms, never blocking interaction, skip-able during fast nav.

**AP-14: Feature-First Demo Narrative.** Leading with "here are all the things we do." Fix: open with scenario (the problem), show solution happening.

### Content

**AP-15: Generic Placeholder Data.** "Demo Company Inc", timestamps from 2020, lorem ipsum. Fix: realistic seed data — diverse names, plausible amounts, recent timestamps.

**AP-16: Explaining While Navigating.** Fix: demonstrate, pause, explain what they saw. UI should explain through contextual labels.

**AP-17: Showing Too Much.** Fix: 4 sections, 3 scenarios, each done completely. Abundance signals complexity; focus signals confidence.

---

## 3. What NOT to Build

Explicit items from the old app that should NOT be carried forward:

### Architecture Anti-Patterns
1. No ConnectScreen overlay — dead code
2. No auto-play on load — page starts calm, user clicks Start
3. No `rawSleep()` — all timing through DemoController
4. No module-level singletons (PlaybackController, TimelineStepper, globalSpeed) — use class instances
5. No `{} as T` return from post failures — return null
6. No cumulative `_seedCount`/`_nonSeedCount` — use sliding window
7. No `tput colors` rainbow bar — clear after probe
8. No duplicate routes — one route per view
9. No standalone Terminal page — terminal embedded in Build and Orchestrate
10. No inline `<style>` tags — all animations in CSS files
11. No `void setFoo` lint suppressions — delete unused code

### UX Anti-Patterns
12. No blank screens — every loading state has Skeleton, every empty state has EmptyState
13. No mystery buttons — every label says what happens when clicked
14. No unexplained jargon — first use of domain terms gets tooltip
15. No silent failures — every error produces visible message
16. No raw technical data — numbers always have labels, units, comparative context
17. No dead-end screens — every page suggests what to do next
18. No 8px/9px/10px body text — minimum 11px labels, 12px table, 13px body
19. No `position: fixed` on page-level elements — use `position: absolute` relative to container
20. No WorkflowConstellation (Three.js) on Orchestrate — task board tells the story better

---

## 4. Migration Guide: demo-app → Target

| demo-app file | Target equivalent | Notes |
|---|---|---|
| `lib/scenarios.ts` (75K) | `pages/orchestrate/scenarios.ts` | Rewrite with DemoController, 3 scenarios |
| `hooks/useTerminal.ts` | `data/use-terminal.ts` | Fix listener separation, status, timer, decoder |
| `hooks/useApiWithFallback.ts` | `data/api.ts` | Fix TTL, error discrimination |
| `hooks/useSSE.ts` | `data/use-sse.ts` | Fix timer leaks |
| `hooks/useBench.ts` | Inline in Evaluate | Fix double-start |
| `styles/rosedust.css` | `design/tokens.css` + `design/global.css` | Extract tokens |
| `components/Pane.tsx` | `design/Pane.tsx` | Same concept, cleaner |
| `pages/Demo.tsx` (700L) | `pages/Orchestrate.tsx` (~200L) + sub-components | Phase state machine |
| `lib/playback-controller.ts` | `pages/orchestrate/demo-controller.ts` | Single class |

### Bugs Fixed by Architecture

The new architecture (DataHub + Cell + Motion + State Machine) automatically fixes 30+ catalogued bugs:

| Category | Bugs fixed | How |
|---|---|---|
| Listener leaks (B1, B30) | Listeners registered once, outside connectWs() |
| Timer leaks (B7, B8, B27) | clearTimeout before reassign, interval at start |
| Health false positive (B10) | No fake 'connected' state |
| Reconnect crashes (B9, B17, B32) | Promise-based lock, explicit init |
| Unmount leaks (B13, B14, B34) | AbortController |
| Module-level state (B35) | Module-level constants instead of useState |
| Dead code (DC1–DC6) | Not carried forward |
| Speed control (P1–P4) | DemoController.sleep() respects pause |

---

## 5. Working Methodology

### 5.1 Work in Batches — One Phase Per Session

- **Phase 1**: Foundation (design system + app shell)
- **Phase 2**: Data layer
- **Phase 3**: Orchestrate page
- **Phase 4**: Observe page
- **Phase 5**: Evaluate page
- **Phase 6**: Build page
- **Phase 7**: Polish & UX pass

Always leave things in a buildable, working state before stopping.

### 5.2 The Critical Rule: 100% Before Moving On

Do not leave loose ends. No placeholder components. No `// TODO`. No stubs. Each component must be:
- Fully implemented per spec (all props, states, transitions, edge cases)
- Visually correct (matches CSS values, spacing, typography, colors)
- Tested with Playwright screenshots
- Building without errors (`npm run build`)

Pattern: implement → verify visually → fix → verify again → move on. This produces fewer files per session but every file is production-quality.

### 5.3 Verify with Playwright

After each significant component:

1. Screenshot at 1440×900 viewport
2. Visually inspect against spec mockups
3. Check: specular highlights, grain texture, typography, spacing, colors, hover states, loading/empty states
4. Save to `/tmp/demo-current-screenshots/`

### 5.4 Build Check After Every File

`npm run build` after creating or modifying any file. Never accumulate multiple files before checking. Broken build = stop and fix.

---

## 6. "World Class" Component Checklist

Before marking any component done, verify ALL:

### Visual
- [ ] Colors match tokens.css exactly
- [ ] Typography uses correct font/weight/size/tracking
- [ ] Spacing uses gap tokens (not arbitrary px)
- [ ] Specular highlight (`inset 0 1px 0 rgba(255,255,255,0.06)`) on elevated surfaces
- [ ] Borders use `rgba(255,255,255, 0.04–0.14)`, not hex
- [ ] No pure white (#fff) text — use `--text-strong` at most

### Interaction
- [ ] Hover state ≤150ms using `var(--ease-snappy)` or `var(--ease-out)`
- [ ] Active/pressed: 50ms press, 120ms release
- [ ] Focus-visible with double-ring (`var(--focus-ring)`)
- [ ] No `transition: all` — specific properties
- [ ] Disabled: `opacity: 0.4`, `pointer-events: none`

### Animation
- [ ] Entrance animation (fadeUp, 200ms, staggered if list)
- [ ] No animation exceeds 400ms
- [ ] `prefers-reduced-motion` respected

### Content
- [ ] Labels unambiguous ("TOTAL COST" not "COST")
- [ ] Metrics have comparative context
- [ ] Empty state shows what's empty + what to do
- [ ] Domain terms use tooltip on first use

### Structural
- [ ] No inline styles (except dynamic values)
- [ ] No `// TODO` or placeholder content
- [ ] Error boundary on page-level components
- [ ] Renders correctly with seed data when server offline

---

## 7. Visual Quality Targets

Each page should score ≥ 8.0/10 on:

| Criterion | Weight |
|---|---|
| Layout & spacing | 20% |
| Typography & readability | 15% |
| Color consistency & contrast | 15% |
| Interactive states (hover, active, focus) | 15% |
| Loading/empty/error states | 10% |
| Animation & transitions | 10% |
| Information density & hierarchy | 10% |
| Accessibility (a11y) | 5% |

---

## 8. Inspiration References

- **Linear** — transitions between list and detail, keyboard-first navigation
- **Vercel Dashboard** — real-time deployment status, streaming build logs
- **GitHub Actions** — workflow visualization, step-by-step progress
- **Raycast** — command palette UX, fluid animations
- **Stripe Dashboard** — metric animations, chart transitions
- **Eve Online** — ambient particle fields, glow effects, dark theme with accent colors
- **Factorio** — production chain visualization, real-time flow indicators
- **Notion** — block-based composition, smooth drag-and-drop

---

## 9. Key Metrics for Success

| Metric | Target |
|---|---|
| Time to understand what you're looking at | < 3 seconds |
| Time from page load to first interaction | < 1 second |
| Frame rate during animations | 60fps consistent |
| Bundle size (initial) | < 200kB gzipped |
| Largest Contentful Paint | < 1.5s |
| Components reused across 2+ scenes | > 15 |
| Raw CSS animation hacks | 0 (all through motion tokens) |
| Empty/loading states per data surface | 100% coverage |
