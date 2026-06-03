# Demo App Implementation — Agent Prompt

Paste this entire prompt to an agent. When it finishes a batch and commits, paste it again to continue. The agent will read the checklist, find where it left off, and continue.

---

## Your Task

You are implementing a from-scratch React demo app for Roko, an AI agent orchestration toolkit. The complete specification is at:

```
/Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-ui/11-NEXT-GEN-SPEC.md
```

**Read that file first.** It is 2,000 lines covering architecture, design system, page specs, component specs, and implementation checklist. It is your source of truth for everything.

The target directory is:

```
/Users/will/dev/nunchi/roko/roko/demo/demo-current/
```

The existing demo app (reference only — do not modify) is at:

```
/Users/will/dev/nunchi/roko/roko/demo/demo-app/
```

Use `demo-app` as reference for: API endpoint shapes, seed data structures, scenario logic patterns, xterm.js usage, and SSE event formats. Do NOT copy files 1:1 — re-implement from scratch following the spec.

The backend is `roko-serve` at `:6677`. The spec lists all API endpoints consumed. The app must work with seed/fallback data when the server is not running.

---

## How to Work

### 1. Find where you left off

Read the implementation checklist in Part 6 of the spec (Phases 1–7). Check which files already exist in `demo-current/`. The first unchecked phase with missing files is where you start.

If `demo-current/` is empty or has no `package.json`, start at Phase 1.

### 2. Work in batches — one phase per session

Each session, complete ONE phase from the checklist:

- **Phase 1**: Foundation (design system + app shell)
- **Phase 2**: Data layer
- **Phase 3**: Orchestrate page
- **Phase 4**: Observe page
- **Phase 5**: Evaluate page
- **Phase 6**: Build page
- **Phase 7**: Polish & UX pass

If a phase is large (Phase 1 has ~25 items), you may split it into sub-batches, but **always leave things in a buildable, working state before committing.**

### 3. THE CRITICAL RULE: Finish each thing to 100% before moving on

**Do not leave loose ends.** Do not create placeholder components. Do not write `// TODO` comments. Do not stub out functions. Do not move to the next component until the current one is:

- Fully implemented per the spec (all props, all states, all transitions, all edge cases)
- Visually correct (matches the spec's CSS values, spacing, typography, colors)
- Tested with Playwright screenshots to verify it looks world-class
- Building without errors (`npm run build`)

**The pattern is:** implement → verify visually → fix → verify again → move on.

This means each session produces fewer files but every file is production-quality. The alternative (scaffold everything, then fix later) is explicitly prohibited — it's how the old demo-app became a mess of half-wired components.

### 4. Verify with Playwright after every significant component

After implementing each component or page, write and run a Playwright script to:

1. **Screenshot the component/page** at 1440×900 viewport
2. **Visually inspect** the screenshot yourself — check against the spec's mockups
3. **Verify specific craft details:**
   - Specular top-edge highlights visible on elevated surfaces
   - Grain texture visible (subtle but present)
   - Typography matches spec (Fraunces for display, JetBrains Mono for labels)
   - Spacing matches spec (48-64px between sections, 24-32px between groups)
   - Colors match spec (check text isn't pure white, backgrounds have correct tint)
   - Hover states work (screenshot hover states too)
   - Loading/empty states render correctly
4. **Save screenshots** to `/tmp/demo-current-screenshots/` with descriptive names

Example Playwright verification script:
```typescript
import { chromium } from 'playwright';

const browser = await chromium.launch();
const page = await browser.newPage();
await page.setViewportSize({ width: 1440, height: 900 });
await page.goto('http://localhost:5173/app/orchestrate');
await page.waitForLoadState('networkidle');

// Screenshot the full page
await page.screenshot({ path: '/tmp/demo-current-screenshots/orchestrate-idle.png', fullPage: false });

// Screenshot hover state on a card
await page.hover('.scenario-card:first-child');
await page.screenshot({ path: '/tmp/demo-current-screenshots/orchestrate-card-hover.png' });

// Screenshot empty state
await page.goto('http://localhost:5173/app/observe');
await page.screenshot({ path: '/tmp/demo-current-screenshots/observe-status.png' });

await browser.close();
```

Run the dev server (`npm run dev`) in background, then run the Playwright script. **Look at the screenshots.** If something doesn't look right — the spacing is off, text is too small, colors are wrong, the grain layer is missing — fix it before moving on.

### 5. Test demo workflows end-to-end

For pages that have interactive workflows (Orchestrate, Build, Evaluate), you must verify the full flow works:

**Orchestrate:**
- Click scenario card → description appears, CTA visible
- Click START → terminal opens, phase rail advances
- Phases progress through idea → prd → plan → tasks → running → complete
- Speed controls work (if the server isn't running, verify with seed data simulation)
- Reset returns to idle cleanly (no leaked timers, no stale state)
- All 3 scenarios complete without crashes
- Screenshot each phase transition

**Build:**
- Type in the input → Build button enables
- Click a preset → input fills, CLI preview appears
- Click Build → layout switches to terminal-focused mode
- Terminal streams output (or shows appropriate message if server is down)
- Gate bar updates as gates pass/fail
- "Build Again" returns to input mode

**Evaluate:**
- Configure tab shows suite/model/params selectors
- All form controls are interactive and produce visible feedback
- History tab shows seed data runs
- Empty states appear where appropriate with actionable messages

**Observe:**
- All 5 tabs render with seed data
- Tab switching is instant with fade transition
- Mosaic metrics display with correct formatting
- Tables have hover states and are scannable
- Canvas visualizations (topology, knowledge graph) render and stop animating when stable

For each workflow, use Playwright to automate the clicks and screenshot every significant state. If a workflow breaks, fix it before moving to the next page.

### 6. Build check after every file

Run `npm run build` (tsc + vite build) after creating or modifying any file. Never accumulate multiple files before checking. A broken build means you stop and fix before continuing.

### 7. Commit after each completed sub-unit

Ask before committing (per the project's git rules). Suggested commit points:

- Phase 1a: `tokens.css` + `global.css` + fonts working → commit
- Phase 1b: All design system components → commit
- Phase 1c: AppShell + TopNav + routing → commit
- Phase 2: Full data layer → commit
- Phase 3a: DemoController + scenarios → commit
- Phase 3b: All Orchestrate phase components → commit
- Phase 4: All Observe tabs → commit
- Phase 5: All Evaluate tabs → commit
- Phase 6: Build page → commit
- Phase 7a: Typography + empty states + error boundaries → commit
- Phase 7b: Craft audit + atmospheric layers + performance → commit

Use branch: `demo-current-impl` (or whatever branch you're already on).

Commit messages should describe what was built and that it was visually verified:
```
demo-current(phase-1a): Design tokens, global styles, atmospheric layers

- tokens.css: full ROSEDUST v2 token set (colors, shadows, motion, focus)
- global.css: reset, grain SVG filter, scanlines, vignette, keyframes
- Verified: grain visible at 4% opacity, scanlines at 6%, correct fonts loading
```

---

## What "World Class" Means — Checklist Per Component

Before marking any component done, verify ALL of these:

**Visual:**
- [ ] Colors match tokens.css exactly (not eyeballed approximations)
- [ ] Typography uses correct font/weight/size/tracking from spec
- [ ] Spacing uses gap tokens (not arbitrary pixel values)
- [ ] Specular highlight (`inset 0 1px 0 rgba(255,255,255,0.06)`) on elevated surfaces
- [ ] Borders use `rgba(255,255,255, 0.04–0.14)`, not hex colors
- [ ] No pure white (#fff) text anywhere — use `--text-strong` at most

**Interaction:**
- [ ] Hover state with transition ≤150ms using `var(--ease-snappy)` or `var(--ease-out)`
- [ ] Active/pressed state with asymmetric timing (50ms press, 120ms release)
- [ ] Focus-visible state with double-ring pattern (`var(--focus-ring)`)
- [ ] No `transition: all` — always specific properties
- [ ] `will-change: transform` on elements that animate on hover
- [ ] Disabled state at `opacity: 0.4`, `pointer-events: none`

**Animation:**
- [ ] Entrance animation (fadeUp, 200ms, staggered if list)
- [ ] No animation exceeds 400ms
- [ ] `prefers-reduced-motion` respected

**Content:**
- [ ] Labels are unambiguous ("TOTAL COST" not "COST", "PASS RATE" not "RATE")
- [ ] Metrics have comparative context where applicable
- [ ] Empty state shows what's empty, what to do, and optionally a technical hint
- [ ] Domain terms use `<Term>` tooltip on first use per page

**Structural:**
- [ ] No inline styles (except dynamic values like `--i` for stagger index)
- [ ] No `// TODO` or placeholder content
- [ ] Error boundary wrapping if this is a page-level component
- [ ] Renders correctly with seed data when server is offline

---

## Reference Files

| What | Path |
|---|---|
| **Full spec** | `tmp/solutions/demo-ui/11-NEXT-GEN-SPEC.md` |
| **Master checklist (old bugs)** | `tmp/solutions/demo-ui/10-MASTER-CHECKLIST.md` |
| **Visual design reference** | `demo/demo-app/tools/visual-iteration-prompt.md` |
| **Landing page (design source)** | Downloaded `nunchi_5.html` — tokens extracted in spec Part 2.1 |
| **Existing demo app (patterns)** | `demo/demo-app/src/` |
| **Seed data shapes** | `demo/demo-app/src/lib/bench-demo-data.ts`, `demo/demo-app/src/lib/demo-data.ts` |
| **API endpoint list** | Spec Part 4.2 (Observe), `demo/demo-app/src/hooks/` for request patterns |
| **xterm theme** | `demo/demo-app/src/lib/rosedust-theme.ts` |
| **Backend routes** | `crates/roko-serve/src/routes/` (~235 endpoints) |

---

## Anti-Patterns to Avoid (from spec Part 0.13 and Part 7)

These are hard prohibitions. If you catch yourself doing any of these, stop and fix:

1. **No placeholder components.** Don't create `<Foo />` that renders "TODO". Implement it fully or don't create it yet.
2. **No `transition: all`.** Always list specific properties.
3. **No pure white text.** Use `--text-strong` (#d8c8d0) at most.
4. **No hex color borders.** Use `rgba(255,255,255, 0.04–0.14)`.
5. **No animation over 400ms.** Cap at 350ms for most things.
6. **No blank screens.** Every async state has Skeleton, every empty state has EmptyState.
7. **No `// TODO` comments.** Finish or don't start.
8. **No `{} as T` type assertions for empty data.** Return `null` from failed requests.
9. **No module-level singletons.** Use class instances or context.
10. **No `ease` timing function.** Use `ease-out`, `var(--ease-snappy)`, or `var(--ease-expo)`.
11. **No stale data without indication.** If showing cached/seed data, StatusPill must reflect it.
12. **No silent errors.** Every catch produces a user-visible message.
13. **No features from the old app that aren't in the spec.** The spec is the scope. Don't add ChainView, ConnectScreen, WorkflowConstellation, or any of the 14 items in Part 7's "What NOT to Build."

---

## Starting

1. Read the spec: `cat tmp/solutions/demo-ui/11-NEXT-GEN-SPEC.md`
2. Check what exists: `ls demo/demo-current/src/` (or note it's empty)
3. Identify your phase
4. Implement, verify with Playwright screenshots, fix, commit
5. When done with the phase, say what you completed and what's next

Go.
