# A9: Demo rehearsal — three end-to-end flows

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette — bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting

1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

---

## What this task produces

Three scripted demo flows that verify the dashboard works end-to-end against a running `roko-serve` instance. Each flow is a numbered checklist of exact steps with expected visual outcomes. If roko-serve is not available, fallback outcomes are noted.

This is a QA and rehearsal task, not a coding task. The output is a confirmed pass/fail record and a prioritized bug list.

**Depends on:** Tasks A1–A8 (everything must be wired).

---

## Pre-requisites

### Terminal 1 — start the backend

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo run -p roko-cli -- serve
```

Verify it is running before proceeding:

```bash
curl -s http://localhost:6677/api/health | python3 -m json.tool
```

Expected output:
```json
{
  "status": "ok",
  "version": "...",
  "uptime_secs": ...
}
```

If roko-serve is not available, you can still run all three flows. The dashboard degrades gracefully — showing "Offline" indicators and empty states rather than crashing.

### Terminal 2 — start the dashboard

```bash
cd /Users/will/dev/nunchi/nunchi-dashboard
npm run dev
```

Open `http://localhost:5173/` in Chrome or Firefox. Keep DevTools Console open throughout.

---

## Flow 1: Landing page → Observatory tour

Goal: walk a first-time viewer through the landing page and into the live data views.

### Steps

- [ ] **1.1** Open `http://localhost:5173/`
  - You should see: hero section with animated gradient, Nunchi logo, title text, and "Launch dashboard" button
  - You should see: four stats that animate from 0 to their target values (count-up effect) within ~1 second of page load
  - Verify: no console errors during load

- [ ] **1.2** Scroll to the architecture explorer section
  - You should see: three tier buttons labeled "T0 — Reflexive", "T1 — Adaptive", "T2 — Reflective"
  - Click **T0 — Reflexive**
  - You should see: the detail panel on the right updates to show gate pipeline components (e.g., "compile", "test", "clippy")
  - Click **T2 — Reflective**
  - You should see: the detail panel updates to show prompt experiments and cascade router info

- [ ] **1.3** Continue scrolling to the context auction section
  - You should see: five horizontal bars labeled "Neuro", "Task", "Research", "Playbook", "System"
  - You should see: bars animate in staggered sequence (each bar starts ~100ms after the previous)
  - The bars should use ROSEDUST colors — no default Tailwind blues or greys

- [ ] **1.4** Continue scrolling to the stigmergy canvas
  - You should see: approximately 30 moving dots on a dark background
  - You should see: thin proximity lines appear between dots that are close to each other
  - You should see: trail fading effect as dots move (lines fade over ~2 seconds)

- [ ] **1.5** Click **Launch dashboard**
  - You should be navigated to `http://localhost:5173/app/chat`
  - You should see: left nav, top bar, and right panel all render simultaneously
  - The right panel should show WS connection status (green dot if backend running, grey if not)

- [ ] **1.6** Click **Live agents** in the left nav (under Observatory)
  - URL becomes: `http://localhost:5173/app/observatory/agents`
  - **If roko-serve is running:** you should see agent rows with PID numbers and "running" badges
  - **If roko-serve is not running:** you should see an error state with a "Retry" button (not a blank page)

- [ ] **1.7** Click **Plans** in the left nav
  - URL becomes: `http://localhost:5173/app/observatory/plans`
  - **If plans exist:** you should see plan cards with title, status badge, and a progress bar
  - Click on any plan title — a modal or drawer should open showing the task list
  - **If no plans exist:** you should see "No plans found" empty state with a description

- [ ] **1.8** Click **Learning** in the left nav
  - URL becomes: `http://localhost:5173/app/observatory/learning`
  - You should see at minimum: a C-Factor card and a cascade router card
  - **If roko-serve has data:** numbers are populated (C-Factor shows a percentage, e.g., "+12.3%")
  - **If no data:** cards show "—" or "0" — they must not crash or show undefined

- [ ] **1.9** Click **Conductor** in the left nav
  - URL becomes: `http://localhost:5173/app/observatory/conductor`
  - You should see a diagnosis list or "No diagnoses" empty state

- [ ] **1.10** Click **Costs** in the left nav
  - URL becomes: `http://localhost:5173/app/observatory/costs`
  - You should see four summary cards: Total cost, Tokens in, Tokens out, Avg latency
  - **If efficiency data exists:** the sparkline below the cards renders with data points

### Flow 1 result

- [ ] All steps passed
- [ ] Bugs found (list here with route, symptom, and severity):

---

## Flow 2: Self-hosting workflow (Atelier + CLI)

Goal: demonstrate the PRD → plan → execute loop using both the dashboard UI and the CLI in parallel.

### Steps

- [ ] **2.1** Navigate to `http://localhost:5173/app/atelier`
  - You should see: three summary cards — PRDs, Plans, Velocity
  - The "Self-hosting workflow" card below should list the four CLI commands
  - Numbers may be zero if roko-serve has no data yet

- [ ] **2.2** Click **PRDs** in the left nav (under Atelier)
  - URL becomes: `http://localhost:5173/app/atelier/prds`
  - You should see: the idea capture form and the filter tabs ("All", "Ideas", "Drafts", "Published")
  - Verify: clicking each tab updates the list (or shows "No X PRDs" empty state)

- [ ] **2.3** Type the following into the quick idea input field and click **Capture**:
  ```
  Add health check endpoint to roko-agent-server
  ```
  - You should see: a green toast "Idea captured" slides in from the right edge and fades after ~3 seconds
  - **If roko-serve is running:** open Terminal 1 and verify the idea is persisted:
    ```bash
    ls /Users/will/dev/nunchi/roko/roko/.roko/prd/ideas/
    ```
    You should see a new file named with today's date or the idea slug.
  - **If roko-serve is not running:** you should see a red toast "Failed: ..." — not a crash

- [ ] **2.4** (Requires a draft PRD to exist) Find a PRD with the "draft" badge and click **Promote**
  - You should see: a green toast "PRD promoted"
  - You should see: the badge on that PRD row changes from "draft" (yellow) to "published" (green) without a full page reload
  - **Fallback:** if no draft PRDs exist, run from Terminal 1:
    ```bash
    cd /Users/will/dev/nunchi/roko/roko
    cargo run -p roko-cli -- prd draft new "test-promote"
    ```
    Then refresh the PRD browser.

- [ ] **2.5** (Requires a published PRD without a plan) Find a published PRD without a "has plan" badge and click **Generate plan**
  - You should see: a green toast "Plan generation started"
  - Wait 5–10 seconds, then navigate to `/app/observatory/plans`
  - You should see: the newly generated plan appears in the plan list
  - **Fallback:** if no published PRD exists without a plan, create one:
    ```bash
    cargo run -p roko-cli -- prd idea "Wire neuro store into cascade router"
    cargo run -p roko-cli -- prd draft new "neuro-cascade-wire"
    # then promote via dashboard
    ```

- [ ] **2.6** Navigate to `http://localhost:5173/app/observatory/plans`
  - You should see: the list includes any plan generated in step 2.5
  - If you click **Execute** on a plan, you should see: a toast "Plan execution started"

- [ ] **2.7** Click **Execute** on a plan (any plan that is not already running)
  - Immediately navigate to `http://localhost:5173/app/atelier/execution`
  - You should see: WebSocket events streaming in within a few seconds
  - You should see at least one of: `run_started`, `plan_started`, `agent_output`
  - Events should be displayed newest-first (most recent at the top)
  - Each row should have: a colored badge (event type), truncated JSON payload, and a timestamp

- [ ] **2.8** Watch the execution monitor for 30 seconds
  - You should see: the event list grows as new events arrive — no manual refresh needed
  - You should see: the right panel's event rate sparkline shows activity spikes during execution
  - You should see: the "Active agents" section in the right panel shows agent rows as agents spawn

- [ ] **2.9** Navigate to `http://localhost:5173/app/chat`
  - Type into the input: `Summarize the current execution status`
  - Press Enter (not Shift+Enter)
  - You should see: your message appears as a right-aligned rose-colored bubble
  - You should see: a three-dot typing indicator appears while the run is pending
  - When the run completes, you should see: an agent response appears as a left-aligned grey bubble
  - Verify: Shift+Enter inserts a newline in the textarea instead of sending

### Flow 2 result

- [ ] All steps passed
- [ ] Bugs found (list here with route, symptom, and severity):

---

## Flow 3: Full navigation audit

Goal: click through every single route to verify nothing is broken.

### Steps

- [ ] **3.1** Navigate to each route in order. For each, verify:
  1. Page renders without a blank white screen or "Placeholder" text
  2. No console errors appear
  3. Left nav highlights the correct item with an active indicator
  4. Breadcrumb or page title in the top bar matches the route name

| # | URL | Expected content | Notes |
|---|---|---|---|
| 1 | `http://localhost:5173/` | Landing page with hero | Count-up animation |
| 2 | `http://localhost:5173/app/chat` | Chat with textarea input | Shift+Enter for newline |
| 3 | `http://localhost:5173/app/research` | Research form + history | 2 mock history entries |
| 4 | `http://localhost:5173/app/observatory/agents` | Agent list or error state | — |
| 5 | `http://localhost:5173/app/observatory/plans` | Plan list or empty | — |
| 6 | `http://localhost:5173/app/observatory/learning` | C-Factor + experiments | No crash when empty |
| 7 | `http://localhost:5173/app/observatory/conductor` | Diagnoses or empty | — |
| 8 | `http://localhost:5173/app/observatory/costs` | Cost summary cards | — |
| 9 | `http://localhost:5173/app/network/agents` | Force graph or empty | — |
| 10 | `http://localhost:5173/app/network/pheromones` | Heatmap with type selector | Always renders |
| 11 | `http://localhost:5173/app/network/knowledge` | Search input + results | Empty: "No matches" |
| 12 | `http://localhost:5173/app/marketplace` | Job board with filters | — |
| 13 | `http://localhost:5173/app/marketplace/create` | Job creation form | Required fields |
| 14 | `http://localhost:5173/app/marketplace/job-001` | Job detail with timeline | Status: open |
| 15 | `http://localhost:5173/app/marketplace/job-003` | Completed job + evaluation | Has evaluation score |
| 16 | `http://localhost:5173/app/studio/overview` | 3 agent cards with gauges | Live/offline badges |
| 17 | `http://localhost:5173/app/studio/strategy` | Strategy form | 3 selects + 3 gauges |
| 18 | `http://localhost:5173/app/studio/keys` | Key list + generate form | 2 mock keys |
| 19 | `http://localhost:5173/app/studio/deploy` | Setup steps + connection | Copy buttons visible on hover |
| 20 | `http://localhost:5173/app/atelier` | Summary cards | 3 cards |
| 21 | `http://localhost:5173/app/atelier/prds` | Filter tabs + PRD list | 4 tabs |
| 22 | `http://localhost:5173/app/atelier/execution` | Event stream | Empty msg if no events |
| 23 | `http://localhost:5173/app/settings` | Config viewer + auth | Read-only config note |

- [ ] **3.2** Test sidebar collapse
  - Click the collapse chevron at the bottom-left of the left nav
  - You should see: nav shrinks to ~60px wide, showing icons only (labels hidden)
  - You should see: main content expands to fill the freed space
  - Click the chevron again — nav expands back to full width with labels visible

- [ ] **3.3** Test responsive at 1024px
  - Resize browser window to exactly 1024px wide (use DevTools Device Toolbar)
  - You should see: the left nav automatically collapses to icon-only mode without clicking
  - You should see: the right panel disappears
  - You should see: main content fills the full available width

- [ ] **3.4** Test responsive at 1280px
  - Resize browser window to exactly 1280px wide
  - You should see: the right panel hides
  - You should see: the left nav stays at full width (labels still visible)
  - Resize back to 1440px — right panel reappears

- [ ] **3.5** Test toast notifications
  - Navigate to `http://localhost:5173/app/marketplace/create`
  - Click Submit without filling in the title field
  - You should see: a red error toast "Title is required" slides in from the right edge
  - The toast should disappear after ~4 seconds without interaction

- [ ] **3.6** Test dark theme consistency
  - Check every page in the list above for white or light backgrounds
  - All page backgrounds should be ROSEDUST dark tones (very dark grey, not white)
  - Check: modals, form inputs, badges, buttons, dropdown menus
  - Check: no default Tailwind blue or green colors visible (only ROSEDUST rose, bone, and muted tones)

- [ ] **3.7** Test WebSocket disconnect/reconnect
  - Verify roko-serve is running and the right panel shows "WS connected" with a green dot
  - Kill roko-serve (Ctrl+C in Terminal 1)
  - You should see within ~5 seconds: right panel shows "WS disconnected" with a grey dot
  - Restart roko-serve: `cargo run -p roko-cli -- serve`
  - You should see within ~5 seconds: right panel shows "WS connected" with a green dot
  - You should see: a green toast "WebSocket reconnected" (this toast should NOT appear on the initial connection — only the reconnect)

- [ ] **3.8** Test keyboard shortcuts
  - Click away from any input field to ensure nothing is focused
  - Press `/`
  - You should see: the global search input (if present) receives focus
  - Type something in the search input
  - Press `Escape`
  - You should see: the search input loses focus

### Flow 3 result

- [ ] All 23 routes render correctly
- [ ] Sidebar collapse works (expand and collapse)
- [ ] Responsive breakpoints work at 1024px and 1280px
- [ ] Toast notifications work (error on empty form submit)
- [ ] Dark theme is consistent on all pages
- [ ] WS reconnect toast fires on reconnect but not initial connection
- [ ] Keyboard shortcuts work
- [ ] Bugs found (list here with route, symptom, and severity):

---

## Bug triage

After running all three flows, categorize bugs found:

### P0 — Blocks demo
Pages that crash, show blank white screens, or throw uncaught errors in the console.

_List bugs here._

### P1 — Visible during demo
Wrong data shown, broken layout, missing loading/error states, buttons that don't respond.

_List bugs here._

### P2 — Cosmetic
Minor spacing, font weight, or color inconsistencies that would only be noticed on close inspection.

_List bugs here._

### Fix format

For each bug, record:

1. **File** (absolute path)
2. **Location** (line number or component name)
3. **Symptom** (what the user sees)
4. **Fix** (what to change)
5. **Effort** (one-liner / small / medium)

---

## Verification

This task is complete when:

- [ ] All three flows have been executed in full
- [ ] All bugs are recorded with severity
- [ ] All P0 bugs are fixed or confirmed not to exist
- [ ] `npm run typecheck` passes after any code fixes
- [ ] `npm run dev` starts clean (no errors in the terminal)
- [ ] Console is clean on every route in flow 3
