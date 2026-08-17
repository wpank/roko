# Visual Reassessment (Post-Batch Fixes)

Screenshots taken after applying Batches 1–8 from the fix plan (CSS aliases, duplicate route removal, KnowledgeEntries fallback, server health fix, ChainView timer leak, Builder model selection, Share loading state, ShareView deletion).

Screenshots: `/tmp/roko-audit-1777443964541/`
Console errors: **0 across all 14 pages**
Viewport: 1600×1100, headless Chromium

---

## Per-Page Ratings (using visual-iteration-prompt.md rubric)

### 1. Landing `/` — 7.5/10

**Screenshot observations:**
- "nunchi" title in display serif, centered, with subtitle "the agent coordination plane"
- START button with glass border visible
- Armillary sphere 3D scene renders correctly — small ornament objects orbiting
- Extremely dark overall — subtle atmospheric effects work well
- Top nav visible with ROKO brand and nav links

**What's wrong:**
- The entire page is very dark — subtitle text nearly invisible at this screenshot resolution
- No metrics/stats visible on first viewport (crushed bar, cost comparisons are below fold or absent)
- START button is small relative to the viewport — could be more prominent
- No version indicator visible when server is offline
- The page relies entirely on atmosphere — no data, no proof points above the fold

**What would make 9.5:**
- Metrics/proof points visible above fold (episodes processed, cost saved, agents coordinated)
- START button larger and more prominent
- Subtitle text slightly brighter (currently `--text-soft` is too dim at a6a098)
- Version fallback when offline

---

### 2. Demo `/demo` — 6.5/10

**Screenshot observations:**
- Scenario tabs across top (with "BTC Funding Alert" active tab in rose)
- Left panel: PRD pipeline showing "Ready to generate: BTC funding alert" with idea text
- Right panel: Workflow constellation (3D graph with rose-colored nodes)
- Bottom panels: "BTC Funding Alert CLI" task card and "DEFINE CLI CONTRACT AND DRY-RUN CONFIG" section
- Status bar at bottom with pipeline phase rail
- Dense layout with many panels

**What's wrong:**
- Way too much information crammed into one screen — cognitive overload
- Text is too small throughout — labels, task descriptions, pipeline phase text all tiny
- The 3D constellation on the right is cool but purpose is unclear to first-time viewer
- Pipeline phase rail at bottom is too small to read
- Multiple competing focal points — eye doesn't know where to start
- Task card text is dense and small
- No clear visual hierarchy — everything competes for attention equally
- The page doesn't fit on one screen conceptually even though it technically fits in the viewport

**What would make 9.5:**
- Progressive disclosure: start empty, reveal panels as pipeline progresses
- One dominant focal point (the pipeline status) with everything else subordinate
- Larger text throughout — minimum 12px for body text, 14px for important labels
- Fewer panels visible at once — show relevant panels for the current phase
- Clearer visual hierarchy between primary (pipeline progress) and secondary (terminal output, stats)

---

### 3. Dashboard - Cost `/dashboard` — 7.5/10

**Screenshot observations:**
- Tab bar: Cost (active), Fleet, Knowledge, Entries, Routing, Chain
- Mosaic row: Online, 8h 15m, 0.847, $1.42, 847
- C-Factor Breakdown pane (left) with horizontal bar metrics — values visible
- Model Routing pane (center) — table with provider routing stats
- Activity pane (bottom right) with cost data
- Provider health indicators visible

**What's wrong:**
- The Activity/cost panes at bottom-right have bright/light backgrounds that break the dark theme — jarring white-ish panels against the void background
- C-Factor breakdown bars look functional but the label text is very small
- "Online" status uses italic serif which is inconsistent with the mono design language
- Provider health grid could use more visual emphasis (the status dots are tiny)
- Some panes have too much negative space at bottom

**What would make 9.5:**
- Fix the bright-background panes to use glass/void background instead
- Larger label text in C-Factor bars
- Status indicator in mono font
- Better use of vertical space — no empty voids below panes

---

### 4. Dashboard - Fleet `/dashboard/fleet` — 6.0/10

**Screenshot observations:**
- Mosaic: 3 (agents), 3 (something), 0, 827
- Agent topology in center — 3 rose-colored circles connected by lines, tiny in a large canvas
- Force-directed layout has settled
- Bottom: agent cards cut off at viewport edge — requires scrolling

**What's wrong:**
- Topology graph uses maybe 10% of the available canvas space — 3 tiny dots in a vast dark void
- Agent cards at bottom are cut off — page requires scrolling (violates "fit on one screen" requirement)
- Large empty regions above and below the topology
- Mosaic values don't have clear context — "3" means what? Labels too small to read at screenshot size
- The topology graph looks empty/unimpressive with only 3 nodes

**What would make 9.5:**
- Scale topology nodes to fill more of the canvas (increase node size and force spread)
- Reduce topology canvas height to make room for agent cards without scrolling
- Larger mosaic labels
- Show agent cards in a horizontal strip or compact grid that fits in viewport

---

### 5. Dashboard - Knowledge `/dashboard/knowledge` — 4.5/10

**Screenshot observations:**
- Mosaic: 18 (entries), 28 (links), 5 (density)
- Knowledge Graph canvas: large dark area with barely visible tiny dots and faint connection lines
- Force labels visible but extremely small ("Well Moci Process Rail Model" — placeholder text)
- "DATASETS" section at bottom barely visible
- Overall impression: mostly empty dark void with scattered barely-visible dots

**What's wrong:**
- Graph nodes are tiny — barely visible even at 1600px width
- Labels are placeholder text ("Well Moci Process Rail Model") — not real knowledge domain names
- Canvas occupies ~70% of the viewport but conveys almost nothing
- No loading state, no empty-state message
- The glow effect doesn't render (dead code — B18)
- Animation runs at 60fps forever (B19) draining CPU
- Overall the page looks broken or placeholder — not VC-grade

**What would make 9.0:**
- Larger nodes with visible labels of actual knowledge domains
- Fix glow rendering so nodes have visual weight
- Reduce canvas height, show knowledge entries list alongside
- Use real domain names from demo data instead of placeholder text
- Add loading/empty states
- Stop animation when settled

---

### 6. Dashboard - Entries `/dashboard/entries` — 6.0/10 (improved from 5.0)

**Screenshot observations:**
- "Knowledge Entries" title now renders in correct serif font (CSS alias fix worked!)
- Stats: 0 Total, 0 Domains, 0.0 Avg Citations, — Avg Confidence
- "No knowledge entries found" message visible in the table area
- Rest of page is dark void

**What improved:**
- Title font is correct now (--font-serif alias works)
- Borders and backgrounds now use correct CSS variables
- Text color is now visible (--text alias works)

**What's still wrong:**
- Shows zeros for all stats. Investigation: `DEMO_KNOWLEDGE_ENTRIES` has the right shape (18 entries with id/domain/citations/label) and `useApiWithFallback.get('/api/knowledge/entries')` should return it when offline. The `poll()` function in the useEffect should set `entries` from this data. The zeros in the screenshot likely result from a timing issue: the server probe (`probeServer()`) runs async, and during the initial render the probe hasn't resolved yet. `_serverLive` is `null`, so `get()` tries the real API, fails (server offline), catches the error, and returns fallback data. This should work. But the `useEffect` deps are `[]` (missing `get`) which means it captures the initial `get` closure — which is correct since `get` is `useCallback([api])` and `api` is stable. **Verdict:** The fallback works correctly. The zeros appear because the Vite dev server proxies `/api` to `localhost:6677` where `roko serve` IS running (nav shows "LIVE 8H 31M"). The real API returns an empty array (no knowledge entries in the live server's datastore). The fallback only activates when the server is unreachable. This is correct behavior, but it means the page looks sparse in dev when the server has no data. **The real problem: there's no "show demo data when the server is live but the endpoint returns empty" logic.** The page needs a "seed data when empty" behavior, not just a "seed data when offline" behavior.
- Enormous empty void below the stats row — 70% of the page is void
- All 4 stat cards show zero/null values which looks broken
- No visual richness — just stat cards + empty table

**What would make 9.0:**
- Verify that fallback data actually populates (may need to refactor the useEffect)
- Fill the empty space — show a knowledge entry visualization or larger cards
- Pre-populate with demo data so the page looks alive offline

---

### 7. Dashboard - Routing `/dashboard/routing` — 5.5/10

**Screenshot observations:**
- "Cascade Router" title in correct serif font (CSS alias fix worked!)
- Stats: 0 (models), 0 (observations), 0% (avg confidence)
- Table area: "No model stats found" with column headers
- Massive void below

**What improved:**
- Title font correct
- CSS variable references resolve

**What's still wrong:**
- All stats at zero — same issue as Entries, fallback data not populating the view
- Page is 80% void — one thin strip of content at top
- "No model stats found" is technically correct but looks broken for a demo
- The page feels like a placeholder, not a real feature

**What would make 9.0:**
- Pre-populate with demo router data (4 models, routing distribution, confidence scores)
- Add a routing visualization — model flow diagram or Sankey chart
- Reduce void — either fill with content or shrink the page container

---

### 8. Dashboard - Chain `/dashboard/chain` — 7.0/10

**Screenshot observations:**
- "Phase 2" status in mosaic (first cell, large italic serif)
- Two more mosaic cells showing "0" values
- "TAMPER-PROOF AGENT HISTORY" pane with chain icon and "Cryptographic Agent Trail" heading
- Features list with checkmarks
- Page extends below viewport — requires scrolling for full content

**What's wrong:**
- Requires scrolling — features list, gate waterfall, and hash display are below fold
- "Phase 2" as a status indicator is confusing — it means development phase, not operational status
- Two mosaic cells show "0" which looks broken (episodes and gate results when offline)
- The page tells a story but doesn't fit in one screen

**What would make 9.0:**
- Compress: combine explanation + features into one pane
- Show gate waterfall inline with stats instead of in a separate scrollable section
- Use demo data for episode/gate result counts
- Rename "Phase 2" to something more descriptive

---

### 9. Bench `/bench` — 8.0/10

**Screenshot observations:**
- "Benchmark Lab" title in italic serif — looks good
- Subtitle "Configure, run, and analyze agent evaluations"
- Mosaic: 3 (runs), 100% (pass rate), $0.30 (cost), 4 (gates)
- Test suite selector (Smoke selected)
- Strategy cards: Minimal, Context Enriched, Neuro Augmented, Full Cascade
- Model selector dropdown
- History table at bottom

**What's good:** Best-designed page in the app. Clean layout, clear purpose, good information density.

**What's wrong:**
- Strategy card "Full Cascade" has rose/pink highlight which works
- Bottom half of the page (below config) is somewhat empty
- No link to `/bench/run/:id` from history table
- No link to `/bench/compare` anywhere

**What would make 9.5:**
- Add "View" links in history rows
- Add "Compare Runs" button near history
- Slightly more compact config section to show more history rows

---

### 10. Bench Showroom `/bench/showroom` — 7.0/10

**Screenshot observations:**
- "Bench Showroom" title with subtitle
- Mosaic: 6 (passed), 0 (failed), $0.000 (cost), 6/6 (progress)
- Scenario cards: "Learnable Rust — Claude Haiku" selected, two more cards
- Play/Stop/Reset buttons
- Three panes: Pass Grid (colored cells), Cost Chart, Activity Tree

**What's wrong:**
- Pass grid cells are visible but small — hard to distinguish pass/fail colors
- Cost chart and Activity tree panes are mostly empty with small content
- No speed control
- The mosaic "$0.000" looks wrong (probably should show actual costs from demo data)

**What would make 9.5:**
- Larger pass grid cells
- Pre-populated cost/activity data
- Speed control for playback

---

### 11. Explorer `/explorer` — 4.0/10

**Screenshot observations:**
- "EXPLORER" label at top left, tabs: Health (active), Cost, Episodes, Events
- "Refresh" button at top right
- Mosaic: online, 8h 16m, 0.1.0, 0, 3, 0
- Below the mosaic: ENORMOUS empty void — literally 80% of the page is blank dark space
- "PROVIDERS" section barely visible at bottom left

**What's wrong:**
- This is the worst page in the app. 80% void is not acceptable for any demo
- Mosaic values are useful but the body has almost no content
- Provider section at bottom is tiny and barely visible
- No auto-polling (U1)
- Fabricated provider names (U5)
- No empty states on any tab (U3)
- The page looks abandoned/incomplete

**What would make 9.0:**
- Fill the void: show a combined view of recent episodes + events + provider status
- Larger provider cards with health indicators
- Episode/event stream with formatting (not raw JSON)
- Auto-refresh
- Real provider names from API

---

### 12. Builder `/builder` — 5.0/10

**Screenshot observations:**
- Header: "BUILDER" with model dropdown and preset buttons
- Preset row: many small buttons in a single row (some visible: calculator, REST API, md-html, etc.)
- Files panel on left: "no project yet"
- Terminal area: large dark area with rainbow color bar at top
- Input area at bottom (not clearly visible)
- Gate bar showing compile/test/clippy/diff as pending

**What's wrong:**
- Rainbow color bar from `tput colors` is extremely jarring — looks like a rendering bug (V6)
- Preset buttons overflow (too many in one row)
- Terminal dominates the page but shows nothing useful until a build starts
- "no project yet" is the correct empty state text but could be more inviting
- The page doesn't communicate what the Builder does on first visit
- Model selection now works (fix applied) but the UX of the dropdown isn't clear in screenshot

**What would make 9.0:**
- Remove rainbow color bar (don't run `tput colors` on connect)
- Preset buttons with flex-wrap
- Terminal starts hidden/minimal, expands when a build begins
- Add a brief one-liner about what Builder does
- Progressive disclosure: show terminal + files only after a build starts

---

### 13. Terminal `/terminal` — 5.0/10

**Screenshot observations:**
- "TERMINAL" header with layout buttons (1/2/4 columns) and + button
- One terminal pane with rainbow color bar at top
- Rest of page is dark void below the terminal

**What's wrong:**
- Same rainbow color bar issue (V6)
- Single terminal uses only ~15% of vertical space, rest is void
- No per-terminal close button
- Layout buttons (1/2/4) are small and unlabeled
- Empty state when no terminals open isn't shown (but current state has one terminal)

**What would make 9.0:**
- Terminal should fill available space (`flex: 1`)
- Remove rainbow bar
- Per-terminal close button
- Terminal labels/titles

---

### 14. Share `/share/test-token` — 6.5/10

**Screenshot observations:**
- "Receipt not found" in italic serif, centered
- Subtitle "This share link may have expired or is invalid." in mono
- Clean, centered layout
- But mostly void — just two lines of text centered in a dark page

**What's good:** The loading state fix works (shows "Loading receipt..." before this).

**What's wrong:**
- Very sparse — just error text in a void. No visual richness.
- Duration fallback hardcoded to '4s' (misleading if it were to show)
- No link back to the main app or any navigation hint

**What would make 9.0:**
- Add a link back ("← Back to Dashboard" or "← Return to Roko")
- More descriptive error with suggestions
- The success state (with receipt) would rate higher but we can't test without a real token

---

## Summary Ratings (Post-Fix)

| Page | Before | After | Potential | Key Remaining Blocker |
|------|--------|-------|-----------|----------------------|
| Landing | 7.5 | 7.5 | 9.5 | No proof points above fold |
| Demo | 7.0 | 6.5 | 9.5 | Cognitive overload, tiny text |
| Dashboard Cost | 8.0 | 7.5 | 9.5 | Bright background panes break theme |
| Dashboard Fleet | 7.5 | 6.0 | 9.5 | Tiny topology, cards need scrolling |
| Dashboard Knowledge | 6.0 | 4.5 | 9.0 | Canvas mostly empty, placeholder labels |
| Dashboard Entries | 5.0 | 6.0 | 9.0 | Still shows zeros, empty void |
| Dashboard Routing | 5.5 | 5.5 | 9.0 | All zeros, massive void |
| Dashboard Chain | 7.0 | 7.0 | 9.0 | Requires scrolling |
| Bench | 8.0 | 8.0 | 9.5 | No links to detail/compare |
| Bench Showroom | 7.5 | 7.0 | 9.5 | Small cells, no speed control |
| Explorer | 5.5 | 4.0 | 9.0 | **80% void, worst page** |
| Builder | 6.5 | 5.0 | 9.0 | Rainbow bar, preset overflow |
| Terminal | 7.0 | 5.0 | 9.0 | Rainbow bar, doesn't fill space |
| Share | N/A | 6.5 | 9.0 | Sparse error page |

**Average current: 6.1/10** (down from 6.4 — tighter grading with screenshots)
**Average potential: 9.2/10**

---

---

## Additional Pages/Views Audited

### 15. Bench Run Detail `/bench/run/br-001` — 7.5/10

**Screenshot observations:**
- "Run br-001" title with metadata (model, started, status)
- Mosaic: 100%, $0.288, $0.041, 7/8, 41.1s — good data density
- Task results table with pass/cost/tokens/model/duration columns
- Cost per task chart below
- Requires scrolling for full content

**What's wrong:**
- Unreachable from UI — must type URL manually (no link from Bench history)
- Cost chart at bottom requires scrolling
- Token cost rates are wrong (B24 — 3x off for haiku)
- "Loading run undefined..." if ID is missing (B23)

**What would make 9.0:**
- Add "View" links from bench history table
- Fix cost rates
- Fit key content in one screen

---

### 16. Bench Compare `/bench/compare` — 7.0/10

**Screenshot observations:**
- "Compare Runs" title
- Two run selectors: br-001 vs br-003
- Comparison table showing config diffs (model, strategy, temperature)
- Task-by-task comparison section below

**What's wrong:**
- Unreachable from UI — must type URL manually
- Can select same run for both A and B (B25)
- Can get stuck in loading state if compare endpoint fails (B26)
- No "need 2+ runs" empty state (U6)

**What would make 9.0:**
- Add link from bench page
- Prevent same-run selection
- Add empty state

---

### 17. Explorer Episodes Tab — 4.5/10

**Screenshot observations:**
- Search bar at top
- One tiny episode entry line visible
- Massive dark void below (90% of page is empty)
- Episode data barely visible — very small text

**What's wrong:**
- Almost entirely empty
- Episode row text is tiny
- Search matches field names, not just values (U2)
- No empty-state message when no results (U3)
- Array index as React key (U4)

---

### 18. Explorer Events Tab — 0/10 **CRASH**

**Screenshot observations:**
- **Full page crash.** "Something went wrong" with TRY AGAIN button.
- All content, navigation, and functionality gone.
- ErrorBoundary catches a React render error.

**This is a critical bug (B39).** Any user clicking the Events tab will crash the entire app. Must be fixed before any demo.

---

## Summary Ratings (Post-Fix, All Views)

| Page/View | Rating | Potential | Key Blocker |
|-----------|--------|-----------|-------------|
| Landing | 7.5 | 9.5 | No proof points above fold |
| Demo | 6.5 | 9.5 | Cognitive overload, tiny text |
| Dashboard Cost | 7.5 | 9.5 | Bright background panes |
| Dashboard Fleet | 6.0 | 9.5 | Tiny topology, cards need scroll |
| Dashboard Knowledge | 4.5 | 9.0 | Empty canvas, placeholder labels |
| Dashboard Entries | 6.0 | 9.0 | Shows zeros, empty void |
| Dashboard Routing | 5.5 | 9.0 | All zeros, massive void |
| Dashboard Chain | 7.0 | 9.0 | Requires scrolling |
| Bench Config | 8.0 | 9.5 | No links to detail/compare |
| Bench Showroom | 7.0 | 9.5 | Small cells, no speed control |
| Bench Run Detail | 7.5 | 9.0 | Unreachable from UI |
| Bench Compare | 7.0 | 9.0 | Unreachable from UI |
| Explorer Health | 4.0 | 9.0 | **80% void** |
| Explorer Episodes | 4.5 | 9.0 | 90% void, tiny text |
| Explorer Events | **0** | 9.0 | **CRASH (B39)** |
| Builder | 5.0 | 9.0 | Rainbow bar, preset overflow |
| Terminal | 5.0 | 9.0 | Rainbow bar, doesn't fill space |
| Share | 6.5 | 9.0 | Sparse error page |

### 19. Bench History Tab — 7.5/10

**Screenshot observations:**
- "RUN HISTORY" table header
- 3 runs visible: br-001, br-002, br-003
- Columns: suite, model, tasks, pass rate, cost, EXPORT button
- Clean, functional table layout

**What's wrong:**
- No "VIEW" link to `/bench/run/:id` — detail page unreachable
- No "COMPARE" button or link
- Export button present but no import-from-file indicator
- Bottom 50% is void below the 3-row table

**What would make 9.0:**
- Add "View" link per row → `/bench/run/:id`
- Add "Compare" button (compare 2+ selected rows)
- Show summary stats below table (total runs, avg pass rate)

---

### 20. Landing Full Page — 7.5/10

The landing page has no below-fold content. The entire page IS the single-screen title card with "nunchi / the agent coordination plane / START". No additional sections, no metrics, no proof points beyond the title. This is clean but under-utilized — the most important screen has no data showing Roko is real.

---

**Average current: 5.7/10** (20 views)
**Average potential: 9.2/10**
**Views below 5.0: 3** (Knowledge Graph, Explorer Health, Explorer Events)
**Crashing views: 1** (Explorer Events)

---

## Top 10 Highest-Impact Improvements (ordered by visual impact)

1. **Explorer page redesign** — fill the void with actual content (4.0 → 8.5)
2. **Remove rainbow color bars** from Builder + Terminal (jarring artifact)
3. **Knowledge Graph** — larger nodes, real labels, fix glow rendering (4.5 → 8.0)
4. **Demo page progressive disclosure** — start empty, show relevant panels per phase (6.5 → 9.0)
5. **Dashboard Fleet** — scale topology, fit cards without scrolling (6.0 → 8.5)
6. **Dashboard Entries/Routing** — verify fallback data populates, fill void (5.5/6.0 → 8.5)
7. **Typography pass** — minimum 12px body text across all pages
8. **Dashboard Chain** — compress to fit one screen
9. **CostDashboard** — fix bright-background panes to match dark theme
10. **Builder presets** — flex-wrap, terminal starts minimal
