# Redesign Proposals: What Proper Design Looks Like

For each page, what a from-scratch design should look like — not bandaids on existing layout, but the right answer. Grounded in the visual-iteration-prompt.md principles: information foraging, progressive disclosure, single-screen, one focal point, bigger text, start empty → transition in.

---

## Principle: Every Page Has One Job

| Page | Job |
|------|-----|
| Landing | Convince: Roko is real, impressive, and ready |
| Demo | Prove: request → PRD → plan → tasks → routed/gated execution |
| Dashboard | Operate: health, cost, fleet, knowledge at a glance |
| Bench | Evidence: model comparison, gate pass rates, cost tradeoffs |
| Explorer | Observe: live events, episodes, provider health |
| Builder | Create: type a request → Roko builds it |
| Terminal | Power-user: raw shell access |

---

## 1. Explorer — Currently 4.0/10, biggest opportunity

### Current: empty mosaic over void
The explorer page shows 6 stat values in a mosaic and then 80% void below. Tabs switch the data source but there's nothing to look at.

### Proper design: Combined Activity Stream

One screen, no scrolling. Three zones:

```
┌────────────────────────────────────────────────────────┐
│  EXPLORER                              ● LIVE  [Refresh]│
├──────────┬──────────┬──────────┬──────────┬────────────┤
│  STATUS  │  UPTIME  │ VERSION  │  AGENTS  │  PROVIDERS │
│  online  │  8h 16m  │  0.1.0   │    3     │   4/5 ok   │
├──────────┴──────────┴──────────┴──────────┴────────────┤
│                                                         │
│  ┌─ RECENT EPISODES ──────────────────────────────────┐ │
│  │ ep-020  rustsmith  wire-chain     PASS  $0.024  2s │ │
│  │ ep-019  fullstack  add-health     PASS  $0.017  3s │ │
│  │ ep-018  auditor    review-prd     PASS  $0.031  4s │ │
│  │ (expandable rows with output preview)              │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  ┌─ PROVIDER HEALTH ──┐  ┌─ RECENT EVENTS ───────────┐ │
│  │ ● Anthropic  ok    │  │ 14:23  gate_passed  T1    │ │
│  │ ● OpenAI     ok    │  │ 14:22  agent_dispatched   │ │
│  │ ● Google     ⚠     │  │ 14:21  plan_started       │ │
│  │ ● Ollama     ok    │  │ 14:20  gate_passed  T2    │ │
│  │ ○ Perplexity down  │  │ ...                       │ │
│  └────────────────────┘  └───────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

Key changes:
- Remove tabs. Show everything at once — the page has enough room.
- Episodes get 40% of vertical space as the main content
- Providers + Events share the bottom 30% as two side-by-side panels
- Auto-poll every 10 seconds
- Start with empty panels and "waiting for data..." messages
- New episodes slide in from top with fade animation
- Real provider names from API

---

## 2. Knowledge Graph — Currently 4.5/10

### Current: tiny dots in a vast dark canvas
The canvas occupies 70% of the viewport and shows barely visible dots with placeholder text labels.

### Proper design: Graph + Entry List split

```
┌────────────────────────────────────────────────────────┐
│  KNOWLEDGE      18 shards · 28 links · density 5      │
├────────────────────────────┬───────────────────────────┤
│                            │                           │
│   Knowledge Graph          │  ENTRIES                  │
│   (larger nodes,           │  ├ gate.compile    ●●●●○  │
│    real domain labels,     │  ├ gate.test       ●●●○○  │
│    visible glow,           │  ├ agent.dispatch   ●●●●● │
│    click to highlight)     │  ├ plan.validation  ●●●○○ │
│                            │  ├ config.routing   ●●○○○ │
│                            │  └ ...                    │
│                            │                           │
│                            │  ● = confidence level     │
├────────────────────────────┴───────────────────────────┤
│  DATASETS section (compact)                            │
└────────────────────────────────────────────────────────┘
```

Key changes:
- 50/50 split: graph on left, entry list on right
- Graph nodes 3x larger, with real knowledge domain names as labels
- Fix glow rendering (B18) so nodes have visual weight
- Entry list shows confidence as dot-based micro-visualization
- Click a node in the graph to highlight its entry in the list
- Cap animation at 30fps, stop when energy < threshold (B19, B20)
- Start empty → nodes fade in as data loads

---

## 3. Demo Page — Currently 6.5/10

### Current: everything-at-once information dump
Pipeline, constellation, tasks, terminal output, stats, timeline, gates — all visible simultaneously. Too many competing focal points.

### Proper design: Phase-driven progressive disclosure

The page should change based on the current pipeline phase:

**Before play (idle):**
```
┌────────────────────────────────────────────────────────┐
│  scenario tabs                                         │
├────────────────────────────────────────────────────────┤
│                                                         │
│     BTC Funding Alert                                   │
│     Build a CLI that fetches BTC funding rates...       │
│                                                         │
│           ▶  START                                      │
│                                                         │
│     ○ idea  ○ PRD  ○ plan  ○ tasks  ○ execute  ○ verify│
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Clean. Calm. One focal point: the scenario description and play button.

**During idea/PRD phase:**
```
┌────────────────────────────────────────────────────────┐
│  ● idea  ○ PRD  ○ plan  ○ tasks  ○ execute  ○ verify  │
├──────────────────────────────┬─────────────────────────┤
│                              │                         │
│  IDEA                        │  TERMINAL (minimized)   │
│  "Build a CLI that fetches   │  $ roko prd idea "..."  │
│   BTC funding rates..."      │  ✓ idea captured        │
│                              │                         │
│  → next: generating PRD      │                         │
│                              │                         │
└──────────────────────────────┴─────────────────────────┘
```

Only relevant panels visible. Terminal as evidence (subordinate, right side), artifact as story (dominant, left side).

**During execute phase (tasks running):**
```
┌────────────────────────────────────────────────────────┐
│  ● idea  ● PRD  ● plan  ● tasks  ◉ execute  ○ verify  │
├────────────────────────────────────────────────────────┤
│  TASKS                                                  │
│  ✓ Define CLI contract      T1 haiku    $0.003    1.2s │
│  ✓ Implement core fetcher   T2 sonnet   $0.017    3.4s │
│  ◉ Add email integration    T2 sonnet   running...     │
│  ○ Wire configuration       T1 haiku    pending        │
│  ○ Integration tests        T3 opus     pending        │
│  ○ Final verification       T1 haiku    pending        │
├────────────────────────────────────────────────────────┤
│  TERMINAL (compact)          │  GATES  ✓compile ✓test  │
└──────────────────────────────┴─────────────────────────┘
```

Task board is the dominant element. Each task shows model tier, cost, duration, status. Terminal and gates are compact evidence at the bottom.

Key changes:
- Page starts empty/calm before play
- Panels appear/expand as the pipeline progresses
- One focal point per phase
- Terminal is always subordinate (20-30% of space, not 50%)
- Phase rail is prominent and large (not tiny text at bottom)
- Text is at least 13px for task descriptions

---

## 4. Dashboard Fleet — Currently 6.0/10

### Current: tiny topology dots, cards cut off requiring scroll

### Proper design: Compact topology + agent cards in one screen

```
┌────────────────────────────────────────────────────────┐
│  FLEET                                                  │
├──────┬──────┬──────┬──────┬────────────────────────────┤
│ Total│Active│ Jobs │Tasks │  AGENT TOPOLOGY             │
│  3   │  3   │  0   │ 827  │  (force graph, 40% width)  │
├──────┴──────┴──────┴──────┤  (larger nodes, spread)    │
│                           │                             │
│  AGENTS                   │                             │
│  ┌─ rustsmith ──────────┐ │                             │
│  │ implementer · T1     │ │                             │
│  │ 247 tasks · $0.42    │ │                             │
│  │ active 2m ago        │ │                             │
│  └──────────────────────┘ │                             │
│  ┌─ ethdev ─────────────┐ │                             │
│  │ ...                  │ │                             │
│  └──────────────────────┘ │                             │
└───────────────────────────┴─────────────────────────────┘
```

Key changes:
- 60/40 split: agents list on left, topology graph on right
- Agent cards are compact (3 lines each) to fit without scrolling
- Topology nodes 3x larger, spread to use more canvas space
- Topology continues animating slowly (1fps after settling) instead of freezing at 120 frames
- No scrolling needed — everything fits in viewport

---

## 5. Dashboard Entries — Currently 6.0/10

### Current: stat cards over empty void

### Proper design: Stats + Entry Table with demo data

The existing design is actually fine structurally — stat cards + data table is correct for this page. The problems are:
1. Fallback data isn't populating the table
2. Too much void when empty

Fix path (not a redesign, but a data wiring fix):
- Ensure `useApiWithFallback` data flows to the entries state
- Pre-populate with `DEMO_KNOWLEDGE_ENTRIES` in offline mode
- Table fills the remaining viewport space with entries
- If truly empty, show "Knowledge entries appear here as agents accumulate domain expertise"

---

## 6. Dashboard Routing — Currently 5.5/10

### Current: stat cards over empty void

### Proper design: Stats + Model Routing Visualization

```
┌────────────────────────────────────────────────────────┐
│  CASCADE ROUTER                                         │
├──────────┬──────────┬──────────┬───────────────────────┤
│  MODELS  │   OBS    │AVG CONF  │  ROUTING DISTRIBUTION │
│    4     │   847    │  78.3%   │  ████████░░  haiku 62%│
│          │          │          │  ████░░░░░░  sonnet 28│
│          │          │          │  █░░░░░░░░░  opus 8%  │
│          │          │          │  ░░░░░░░░░░  other 2% │
├──────────┴──────────┴──────────┴───────────────────────┤
│  MODEL ROUTING TABLE                                    │
│  Role          Model           Confidence   Observations│
│  implementer   claude-haiku    82%          412         │
│  researcher    claude-sonnet   76%          198         │
│  reviewer      claude-opus     91%          57          │
│  planner       claude-sonnet   74%          180         │
├────────────────────────────────────────────────────────┤
│  updated 2 minutes ago                                  │
└─────────────────────────────────────────────────────────┘
```

Key changes:
- Horizontal bar chart for routing distribution (inline, not a separate pane)
- Table pre-populated with demo data in offline mode
- Stats show real values from demo data, not zeros
- Fill the viewport — no void

---

## 7. Builder — Currently 5.0/10

### Current: preset buttons + empty terminal + rainbow bar

### Proper design: Progressive disclosure, terminal starts minimal

**Before build:**
```
┌────────────────────────────────────────────────────────┐
│  BUILDER                              model: haiku ▼   │
├────────────────────────────────────────────────────────┤
│  preset: calculator · REST API · md-html · dedup ·     │
│          commitgen · web scraper · test harness · ...   │
│                                                         │
│                                                         │
│         describe what to build...                       │
│         ──────────────────────────── [Build]            │
│                                                         │
│     gates: ○ compile  ○ test  ○ clippy  ○ diff         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Clean. Input-focused. No terminal until needed.

**During build:**
```
┌────────────────────────────────────────────────────────┐
│  BUILDER                              model: haiku ▼   │
├────────────────────────────────────────────────────────┤
│  building: "Build a CLI calculator in Rust"            │
├──────────────────────────────┬─────────────────────────┤
│  TERMINAL                    │  FILES                  │
│  $ roko run "Build a CLI..." │  + src/main.rs          │
│  → generating plan...        │  + Cargo.toml           │
│  → compiling...              │  + README.md            │
│  ✓ compile passed            │                         │
│  → testing...                │                         │
│                              │                         │
├──────────────────────────────┴─────────────────────────┤
│  gates: ✓ compile  ◉ test  ○ clippy  ○ diff           │
└─────────────────────────────────────────────────────────┘
```

Terminal expands when build starts. Files list appears as files are detected.

Key changes:
- Remove rainbow `tput colors` bar
- Terminal hidden before build, expands on build start
- Preset buttons wrap (flex-wrap)
- Gate bar always visible (not conditional)
- Input area is the focal point before build
- Terminal + files are the focal point during build
- Bigger text: prompt input at 14px, terminal at 13px

---

## 8. Terminal — Currently 5.0/10

### Current: one small terminal pane, rest is void

### Proper design: Full-height terminals

```
┌────────────────────────────────────────────────────────┐
│  TERMINAL                           [1] [2] [4]  [+]  │
├────────────────────────────────────────────────────────┤
│                                                         │
│  $ _                                                    │
│                                                         │
│  (terminal fills all available space)                   │
│                                                         │
│                                                         │
│                                                         │
│                                                         │
│                                                         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Key changes:
- Terminal pane fills all available vertical space (`flex: 1` or `height: calc(100vh - nav)`)
- Don't run `tput colors` on connect — no rainbow bar
- Per-terminal close (X) button
- Column buttons labeled (accessibility)
- Multiple terminals split the available space equally

---

## Cross-Cutting Design Rules

### Typography
- Body text: minimum 13px
- Labels: minimum 11px
- Table data: minimum 12px
- Mosaic values: keep current large serif (good)
- Canvas labels: minimum 10px with legibility testing

### Empty States
Every data-dependent panel must have an empty state message:
- Before data loads: "Loading..."
- When empty: descriptive message about what will appear
- Messages use `--text-dim` at 12px mono

### Progressive Disclosure
- Pages start calm/empty
- Content appears as data arrives or interactions happen
- No content dump on first render
- Transition: fade-up animation (already exists in rosedust.css `.reveal`)

### Single Screen
- Every page must fit in 1100px viewport height (excluding nav)
- If content exceeds, compress or use progressive disclosure
- Never require scrolling for the primary content

### Color Consistency
- Dark backgrounds only: `--bg-void`, `--bg-raised`, `--glass-bg`
- No bright/white backgrounds on any panel
- Status: green for healthy, rose for active, bone for value, amber for warning
