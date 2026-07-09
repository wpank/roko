# Visual Audit

Per-page visual ratings from Playwright screenshots + source code review.
Screenshots captured at 1600x1100 viewport, headless Chromium.
All pages: **0 console errors**.

---

## Rating Scale

- **9.5–10**: VC-grade. Zero improvements possible. Ship it.
- **8–9.4**: Strong. Minor polish needed.
- **6–7.9**: Functional but rough. Several visible issues.
- **4–5.9**: Broken or barren. Major work needed.
- **1–3.9**: Non-functional or completely empty.

---

## Page Ratings

### 1. Landing `/` — 7.5/10 → potential 9.5

**What works:**
- NieR-inspired title screen is visually striking
- Armillary sphere 3D scene renders correctly
- "nunchi" in display serif with "the agent coordination plane" subtitle
- START button with glass styling
- Corner ornaments and ambient particles

**Issues:**
- `ConnectScreen` overlay blocks the entire page for up to 2 seconds while the health probe runs (fixed `z-index: 9999`). Even on fast connections, there's a flash of black overlay before the scene appears.
- `CrushedBar` values are hardcoded (`naiveValue={44.86}`, `actualValue={1.42}`) — never update from API data even when server is live.
- No loading skeleton while metrics fetch. Numbers pop in after a delay.
- Version text only renders conditionally — in demo mode nothing appears, leaving dead space below CTAs.
- IntersectionObserver for `.reveal` animations may have already disconnected by the time elements scroll into view (if any scrolling is added).

**Fixes to reach 9.5:**
- Add instant dismiss of ConnectScreen once Three.js scene is loaded
- Show metrics with smooth count-up animation from 0 (already has `useCountUp` but the timing is off)
- Add version fallback text ("v0.1.0") when API is offline
- Remove CrushedBar hardcoded values or compute from real data

---

### 2. Demo `/demo` — 7.0/10 → potential 9.5

**What works:**
- Scenario tabs across the top with clear labels
- Terminal panes render correctly with color theme
- PRD Pipeline panel below with step visualization
- Header with play/pause/reset/speed controls
- Keyboard shortcut bar

**Issues:**
- Speed button is cosmetic — `0.5x / 1x / 2x / 4x` label changes but nothing speeds up or slows down
- Terminal area at top is cramped when pipeline panel is shown (`max-height: 196px`)
- Pipeline panel dominates the bottom half — too tall for the information density
- Shortcut bar text is barely readable at small size
- Server status shows green even when server is down (first-check demo mode)
- `chainWs` WebSocket opens for every demo page load regardless of scenario
- Gate bar pops in/out with layout shift
- When no scenario is running, empty state says "press PLAY to begin" but the Play button has no arrow icon, just text

**Fixes to reach 9.5:**
- Wire speed control to actually affect timing
- Rebalance terminal/pipeline split ratio (70/30 instead of 50/50)
- Make shortcut bar slightly larger or add tooltips
- Fix server health first-check behavior
- Always render GateBar container (show "waiting..." when empty)
- Add play arrow icon to the Play button

---

### 3. Dashboard - Cost `/dashboard` — 8.0/10 → potential 9.5

**What works:**
- Clean mosaic of 6 stat cards: Status, Uptime, Version, C-Factor, Total Cost, Episodes
- C-Factor Breakdown pane with horizontal bar metrics
- Model Routing pane with provider distribution
- Activity section at bottom
- Rose/bone color palette consistently applied

**Issues:**
- When offline, Provider Health pane renders as empty card (no content, no empty-state message)
- When offline, all 5 C-Factor breakdown bars are 0% width — section looks broken rather than showing a placeholder
- All 7 API calls in `Promise.all` — one 500 blocks all data
- `pulse-dot` keyframe injected as inline `<style>` tag on every render (should be in CSS)
- `useCountUp` animation always starts from 0, not from previous value on update
- "Online" status text uses italic serif font — unusual choice compared to the mono font used elsewhere in the design system

**Fixes to reach 9.5:**
- Add empty-state messages in Provider Health and C-Factor panes
- Separate API calls so one failure doesn't block others
- Move `pulse-dot` to CSS file
- Fix `useCountUp` to animate from current value
- Standardize status text font

---

### 4. Dashboard - Fleet `/dashboard/fleet` — 7.5/10 → potential 9.5

**What works:**
- Mosaic with Total, Active, Job Descriptions, Tasks Done
- Agent topology graph with force-directed layout
- Agent Grid with per-agent details
- Rose/bone visual consistency

**Issues:**
- No loading skeleton — page starts blank then data appears
- When `agents` array is empty, Agent Grid section shows nothing (no empty-state)
- Topology animation freezes after ~2 seconds (120 frames) — looks broken on static data
- `$0.00` shown for agents with no cost data, indistinguishable from genuinely $0 agents
- Agent topology shows "topology unavailable" text when nodes=0 but the pane title still says "AGENT TOPOLOGY"
- Three agent circles shown very small in center of large canvas — poor use of space

**Fixes to reach 9.5:**
- Add loading skeleton
- Add "No agents registered" empty state
- Keep topology animation running slowly (reduce to 1fps after settling)
- Use different indicators for "no data" vs "$0.00"
- Scale topology nodes to use more canvas space

---

### 5. Dashboard - Knowledge `/dashboard/knowledge` — 6.0/10 → potential 9.0

**What works:**
- Mosaic with Shards, Links, Density stats
- Knowledge Graph canvas area with glow effect
- Force link labels

**Issues:**
- Canvas area is mostly empty — tiny dots barely visible in the center of a large dark canvas
- No loading state while entries fetch
- When `entries.length === 0` (offline), canvas renders nothing — large empty dark void with no message
- Canvas animation runs `requestAnimationFrame(draw)` forever at 60fps — never settles, drains battery
- Dead code in glow effect computation: hex-to-rgba conversion is broken but overridden by shadowColor approach below
- `requestAnimationFrame` polling runs even when tab is not visible
- Force simulation labels ("Well Moci Process Rail Model") appear as Lorem Ipsum-like placeholder text

**Fixes to reach 9.0:**
- Add "No knowledge shards" empty state inside canvas
- Throttle animation: only redraw when simulation energy > threshold
- Remove dead glow conversion code
- Pause animation when tab not visible (Page Visibility API)
- Replace placeholder labels with meaningful knowledge domain names

---

### 6. Dashboard - Entries `/dashboard/entries` — 5.0/10 → potential 9.0

**What works:**
- Has explicit loading/error/empty states (only dashboard page that does!)
- Clean stat card row: Total, Summaries, Avg Entropy, Avg Confidence

**Issues:**
- **Uses `useApi` (no fallback) instead of `useApiWithFallback`** — shows error state when server is offline while all sibling pages show demo data. Inconsistent experience.
- **Undefined CSS variables everywhere:**
  - `--font-serif` → Falls back to browser serif (Times New Roman)
  - `--glass-2-border` → Invisible borders
  - `--raised` → Transparent backgrounds
  - `--text` → Text color falls back to browser default (likely invisible on dark bg)
  - `--fail` → Error messages have no color
- Title "Knowledge Entries" renders in Times New Roman instead of design system serif
- All stat cards show "0" / "0.0" / "—" with no indication these are offline placeholders
- Enormous empty space below the stat cards when table is empty

**Fixes to reach 9.0:**
- Switch to `useApiWithFallback` for offline demo consistency
- Fix CSS variables: `--font-serif` → `--display`, `--glass-2-border` → `--glass-border`, `--raised` → `--bg-raised`, `--text` → `--text-primary`, `--fail` → `--rose-bright`
- Add demo data fallback with realistic entries
- Fill empty space with an illustration or "No entries yet" message

---

### 7. Dashboard - Routing `/dashboard/routing` — 5.5/10 → potential 9.0

**What works:**
- Mosaic with Models, Observations, Avg Confidence
- Has data table for model routing stats

**Issues:**
- Same undefined CSS variables as Entries (`--font-serif`, `--glass-2-border`, `--raised`, `--text`, `--fail`)
- Title "Cascade Router" in Times New Roman
- All stats show "0" / "0%" with zero visual indication of offline state
- When `confidence_stats` is missing, table shows "No model stats found" even if routing is active
- Enormous empty space below — page is mostly void
- Last updated timestamp shows raw ISO string from API, not human-friendly format

**Fixes to reach 9.0:**
- Fix all CSS variable references
- Add demo data with realistic model routing stats
- Format timestamps human-friendly
- Add visual graph of routing distribution over time

---

### 8. Dashboard - Chain `/dashboard/chain` — 7.0/10 → potential 9.0

**What works:**
- "Phase 2" status indicator
- Hash typewriter animation (cool effect)
- "Cryptographic Agent Trail" explainer panel
- Feature list with pipeline integration details
- Gate waterfall visualization
- Good visual density — not too empty

**Issues:**
- "STATUS: Phase 2" is not a status indicator — it's telling the user this feature is in Phase 2 of development, which is confusing in a dashboard context
- Hash typewriter animation leaks (setTimeout/setInterval not cleaned up on unmount — B7)
- Gate history fetched once, never polled — stale data on long sessions
- `episodes` and `gateResults` hardcoded to `847` — same magic number in 4+ components
- "TAMPER-PROOF AGENT HISTORY" heading uses different typography than other dashboard panes

**Fixes to reach 9.0:**
- Rename STATUS to "CHAIN STATUS" and show actual chain connection state
- Fix animation cleanup (B7)
- Add polling for gate history
- Share hardcoded demo values via a `demo-constants.ts` module

---

### 9. Bench `/bench` — 8.0/10 → potential 9.5

**What works:**
- Clean config form: Test Suite, Agent Strategy, Model, Temperature
- Strategy cards with 4 options
- Gate toggle checkboxes
- History table in separate tab
- Live run indicator
- Mosaic with Total Runs, Avg Pass Rate, Total Cost, Gates

**Issues:**
- Strategy cards use `grid-template-columns: repeat(4, 1fr)` — breaks below ~640px viewport width
- History table has no link to BenchRunDetail — `/bench/run/:id` is unreachable from UI
- `/bench/compare` is unreachable from UI (no link)
- Gates configuration hardcoded to `['compile', 'test', 'clippy', 'diff']` — not derived from suite
- Import button uses raw file input with no visual indication of accepted formats

**Fixes to reach 9.5:**
- Add responsive breakpoint for strategy cards
- Add "View" link in history table rows → `/bench/run/:id`
- Add "Compare" button linking to `/bench/compare`
- Add file format hint next to Import button

---

### 10. Bench Showroom `/bench/showroom` — 7.5/10 → potential 9.5

**What works:**
- Scenario cards with selection
- Pass Grid visualization with colored cells
- Cost Chart and Activity Tree panels
- Mosaic with Passed, Failed, Cost, Progress stats
- Play/Stop/Reset controls

**Issues:**
- No speed control for showroom playback
- When no scenario is playing, all results shown immediately — the "showroom" progressive reveal effect is lost
- If `DEMO_BENCH_RUNS` is empty, no empty-state message shown
- Playback interval (800ms per task) is not adjustable

**Fixes to reach 9.5:**
- Add speed control slider
- Always start with empty grid, reveal results progressively
- Add empty-state fallback

---

### 11. Explorer `/explorer` — 5.5/10 → potential 9.0

**What works:**
- Tab bar: Health, Cost, Signals, Episodes, Events
- Mosaic with Status, Uptime, Version, Active Plans, Active Agents, Active Runs
- Correct tab switching with data refresh

**Issues:**
- Enormous empty void below the mosaic — no content rendered for any tab unless data exists
- No auto-polling on any tab — events/episodes accumulate silently
- Episodes limited to 200 items with no pagination or "showing N of M" indicator
- Events limited to 500 items — silently truncated
- Provider names synthesized from count (`['claude', 'openai', 'gemini', 'ollama', 'perplexity']`) — fabricated mapping
- `explorer-page::before` pseudo-element creates a `position: fixed` 2px rose border on left edge — paints on top of AppShell
- Data tables (when they exist) use raw JSON formatting instead of formatted displays
- No loading indicators on tab switch

**Fixes to reach 9.0:**
- Add empty-state per tab with relevant illustration/message
- Add auto-polling (every 10s) for events and episodes
- Add pagination with "showing 1-200 of N" footer
- Remove fixed position pseudo-element border (use scoped border)
- Format data values human-friendly
- Add loading spinners on tab switch

---

### 12. Builder `/builder` — 6.5/10 → potential 9.0

**What works:**
- Task input with submit
- Model picker with strategy selector
- Preset buttons for common tasks
- Terminal output area
- File detection from output

**Issues:**
- `selectedModel` is cosmetic — never passed to the roko CLI command. Model picker is a lie.
- Preset buttons overflow horizontally on typical monitors (15 presets in a single row with no wrap)
- File detection regex is fragile — matches false positives on lines like "created 3 entries"
- `setupWorkspace` can silently fail if `handle.current` is null on first render (guard sets `setupDoneRef = true` prematurely)
- "no project yet" message in file panel is functional but could have a better empty-state design
- Terminal area is bright white background against the dark theme — jarring contrast

**Fixes to reach 9.0:**
- Wire `selectedModel` to the roko CLI command (`--model` flag)
- Wrap preset buttons with `flex-wrap: wrap`
- Improve file detection regex
- Fix `setupWorkspace` null-handle race
- Style terminal to match dark theme

---

### 13. Terminal `/terminal` — 7.0/10 → potential 9.0

**What works:**
- Clean minimal design
- Add/clear terminal buttons
- Multiple terminals supported
- Color theme bar visible

**Issues:**
- No per-terminal close button — can only clear all
- `clearAll` doesn't clean up WebSocket connections explicitly (relies on unmount)
- Terminal numbering restarts from 1 after clear-all
- Color theme bar appears at top of terminal — it's the `tput colors` output, not a decorative element, which could confuse users
- Large empty dark void when no terminals are open
- No terminal title/label customization

**Fixes to reach 9.0:**
- Add per-terminal close (X) button
- Add terminal naming/labeling
- Add descriptive empty state ("Click + to open a terminal session")
- Style the color bar to be less prominent or explain what it is

---

### 14. Jobs `/jobs` — 1.0/10 → potential N/A

**What works:** Nothing. The page is completely black/empty.

**Root cause:** `/jobs` has no route defined in `main.tsx`. The `JobMarket.tsx`, `JobFlowViz.tsx`, and related files listed in `gitStatus` as untracked do not exist on disk. The page falls through to the AppShell default which renders nothing.

**Fix:** Either add a route with a "Coming Soon" placeholder, or remove `/jobs` from the top nav. Currently it's a dead link that shows a black void.

---

## Cross-Cutting Visual Issues

### V1. Undefined CSS variables in 4 dashboard pages — **FIXED (Batch 1)**

CSS aliases added to `:root` in rosedust.css. These variables were used but not defined:

| Used | Should be | Pages affected |
|------|-----------|---------------|
| `--glass-2-border` | `--glass-border` | Layout, CascadeRouter, KnowledgeEntries, ShareView |
| `--raised` | `--bg-raised` | Layout, CascadeRouter, KnowledgeEntries |
| `--font-sans` | `--sans` | Layout |
| `--font-serif` | `--display` or `--serif` | CascadeRouter, KnowledgeEntries, ShareView |
| `--text` | `--text-primary` | CascadeRouter, KnowledgeEntries, ShareView |
| `--fail` | `--rose-bright` | CascadeRouter, KnowledgeEntries |

**Impact:** Invisible borders, transparent backgrounds, wrong fonts, invisible text. Several dashboard pages have text that's effectively invisible on the dark background.

### V2. Inconsistent API hook usage — **PARTIALLY FIXED (Batch 3)**

- ~~**`useApi` (no fallback)**: KnowledgeEntries~~ → **Fixed**, now uses `useApiWithFallback`
- ~~ShareView~~ → **Deleted** (Batch 8)
- **`useApiWithFallback`**: all other pages → demo data when offline

KnowledgeEntries now uses the fallback hook, but the poll effect may still need verification that fallback data actually populates the entries state.

**See also:** [08-VISUAL-REASSESSMENT.md](08-VISUAL-REASSESSMENT.md) for updated ratings.
**See also:** [07-DEEP-SOURCE-AUDIT.md](07-DEEP-SOURCE-AUDIT.md) for V6–V10 (5 new visual issues).

### V3. Hardcoded demo values scattered across 4+ components

`847` episodes, `$1.42` cost, `0.847` C-Factor appear as magic numbers in Landing, CostDashboard, ChainView, Explorer. Not shared via a constant.

### V4. No loading skeletons anywhere

Every page starts blank and fills in data asynchronously. On slow connections, the page flashes empty panes before data appears. Only KnowledgeEntries has explicit loading state.

### V5. `/bench/run/:id` and `/bench/compare` unreachable from UI

Both routes exist but no navigation links point to them.

---

## Summary Ratings

| Page | Current | Potential | Key Blocker |
|------|---------|-----------|-------------|
| Landing | 7.5 | 9.5 | ConnectScreen overlay, hardcoded metrics |
| Demo | 7.0 | 9.5 | Speed control cosmetic, layout balance |
| Dashboard Cost | 8.0 | 9.5 | Empty states, inline styles |
| Dashboard Fleet | 7.5 | 9.5 | Loading states, topology scaling |
| Dashboard Knowledge | 6.0 | 9.0 | Empty canvas, placeholder labels |
| Dashboard Entries | 5.0 | 9.0 | **Undefined CSS vars, invisible text** |
| Dashboard Routing | 5.5 | 9.0 | **Undefined CSS vars, invisible text** |
| Dashboard Chain | 7.0 | 9.0 | Confusing "Phase 2" status |
| Bench | 8.0 | 9.5 | No links to run detail/compare |
| Bench Showroom | 7.5 | 9.5 | No speed control |
| Explorer | 5.5 | 9.0 | Empty void, no pagination |
| Builder | 6.5 | 9.0 | Model picker cosmetic, overflow |
| Terminal | 7.0 | 9.0 | No per-terminal close |
| Jobs | 1.0 | N/A | **Page doesn't exist** |

**Average current: 6.4/10**
**Average potential (excl. Jobs): 9.2/10**
