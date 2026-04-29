# Demo Redesign Checklist

Tracks what exists, what needs to be built, and what's blocked.

---

## Phase 0: Infrastructure (must exist first)

### Server / PTY
- [x] PTY session manager in roko-serve (`terminal.rs`)
- [x] WebSocket endpoint `/ws/terminal/{id}`
- [x] REST API: create/list/destroy sessions
- [x] Static file serving at `/demo/`
- [x] Health check endpoint `/health`
- [ ] **Session auto-cleanup** — stale sessions aren't garbage collected
- [ ] **Session limit** — no cap on concurrent PTY sessions (DoS risk)

### Binary resolution
- [x] `resolveRoko()` in builder.html — probes PATH then local builds
- [x] `resolveRokoPath()` in terminal.html — same pattern
- [ ] **Unify into shared JS module** — currently duplicated in each HTML file
- [ ] **Cache resolution result** in sessionStorage so page reload is instant

### Prompt detection
- [x] Regex-based prompt detection (`/[❯$>%#]\s*$/`)
- [x] Output buffer (2KB ring) per terminal
- [ ] **Test with zsh, bash, fish** — prompt formats differ
- [ ] **Handle multi-line prompts** (e.g. starship with 2-line prompt)

### Command sequencing
- [x] `runCommand()` — type + wait for prompt
- [x] Timeout per command (configurable)
- [x] Pause/resume toggle
- [ ] **Cancel/abort** — stop mid-scenario, kill running command
- [ ] **Error detection** — if command fails, stop sequence and show error

---

## Phase 1: Unified Page Structure

### Layout
- [ ] **Single HTML file** replacing index.html + terminal.html + builder.html
- [ ] **Top bar** — logo + scenario tabs + controls
- [ ] **Main split** — terminal area (flex) + side panel (280px)
- [ ] **Bottom status bar** — cost ticker + model + elapsed + playback controls
- [ ] **Prompt bar** — conditionally shown for builder scenario
- [ ] **Responsive** — graceful degradation on narrow screens

### Navigation
- [ ] **Scenario tabs** — click to switch between demo scenarios
- [ ] **URL hash routing** — `/demo/#builder`, `/demo/#self-hosting`, etc.
- [ ] **Deep linking** — share URL to specific scenario
- [ ] **Keyboard shortcuts** — 1-7 for scenario tabs, Space for pause

### Terminal grid
- [ ] **1-pane layout** (self-hosting, builder, compounding, research)
- [ ] **2-pane layout** (the race)
- [ ] **4-pane layout** (multi-provider, command showcase)
- [ ] **Pane labels** — scenario-specific label per pane
- [ ] **Connection status dots** (green=connected, grey=disconnected)

---

## Phase 2: ROSEDUST Visual System

### CSS tokens
- [ ] **CSS custom properties** — full ROSEDUST palette (see 02-VISUAL-SYSTEM.md)
- [ ] **Glass morphism panels** — glass-1 and glass-2 levels
- [ ] **xterm.js ROSEDUST theme** — 16 ANSI colors mapped to palette
- [ ] **Typography scale** — display, heading, label, value, emphasis classes

### Components
- [ ] **MetricCard** — label + value, glass background
- [ ] **GateIndicator** — pending/pass/fail states with icons
- [ ] **FileEntry** — icon + name, flash animation on creation
- [ ] **ScenarioTab** — active state with rose underline
- [ ] **CostTicker** — tabular nums, digit-slide animation
- [ ] **EventLogEntry** — timestamp + message, phosphor decay fade

### Animations
- [ ] **Digit slide** — cost ticker digits slide up/down on change
- [ ] **File flash** — new files pulse sage-green then fade
- [ ] **Gate transition** — pending → pass/fail with color crossfade
- [ ] **Tab switch** — terminal crossfade (not hard cut)
- [ ] **Pane creation** — glass panel slides in from right

---

## Phase 3: Scenario Implementations

### Scenario 1: Self-Hosting Workflow
- [ ] Command sequence: init → prd idea → prd draft → prd list → prd plan → status
- [ ] Side panel: PRD count, plan count, event log
- [ ] Estimated time: 20-40s depending on agent speed

### Scenario 2: Builder
- [ ] Prompt input bar (visible only in this scenario)
- [ ] 5 preset quick-start cards
- [ ] File tree detection from terminal output
- [ ] Gate status detection from terminal output
- [ ] Cost tracking from DispatchResult
- [ ] Post-build file listing

### Scenario 3: The Race
- [ ] 2-pane split (left=stock, right=roko)
- [ ] Same task dispatched to both
- [ ] Comparison metrics in side panel
- [ ] Winner highlight (bone color)
- [ ] Different `--no-replan` flag for stock pane

### Scenario 4: Multi-Provider
- [ ] 4-pane grid
- [ ] Per-pane provider override via env
- [ ] Graceful "not configured" message if key missing
- [ ] Cost/speed comparison matrix

### Scenario 5: Knowledge Compounding
- [ ] 3 sequential runs in same workspace
- [ ] `roko learn all` between runs
- [ ] Cost declining per run
- [ ] Knowledge entry count in side panel
- [ ] Projected cost extrapolation

### Scenario 6: Command Showcase
- [ ] 4 panes: workspace / learning / agents / knowledge
- [ ] Quick commands that return instantly
- [ ] No side panel (metrics per pane label)

### Scenario 7: Research & PRD
- [ ] Full research → PRD → plan pipeline
- [ ] Requires Perplexity API key for research
- [ ] Fallback: skip research step if no key
- [ ] PRD status in side panel

---

## Phase 4: Polish

### Playback controls
- [ ] **Play/Pause** — toggle command submission
- [ ] **Reset** — destroy terminals, recreate workspace
- [ ] **Speed** — 0.5x / 1x / 2x typing speed
- [ ] **Skip** — jump to next command (abort current wait)

### Robustness
- [ ] **Reconnect** — auto-reconnect WebSocket on disconnect
- [ ] **Timeout handling** — show "timed out" in pane, continue to next command
- [ ] **Server down** — clear "roko serve not running" message with instructions
- [ ] **Terminal resize** — all panes resize correctly on window resize

### Accessibility
- [ ] **Keyboard navigation** — Tab through scenario tabs, Enter to activate
- [ ] **Screen reader** — aria labels on tabs, status, metrics
- [ ] **Reduced motion** — `prefers-reduced-motion` disables animations
- [ ] **High contrast** — bone text on void background already high contrast

### Offline mode
- [ ] **Scripted fallback** — if server unreachable, play pre-recorded demo
- [ ] **Cast file support** — load asciinema `.cast` files for offline playback
- [ ] **Static build** — export demo as standalone HTML+JS for sharing

---

## Phase 5: Advanced Demos (from DEMO-CONCEPTS.md)

### Demo A: The Race (enhanced)
- [ ] Live cost tickers with digit-slide animation
- [ ] Event flashes (rose overlay on Signal/Route/Gate events)
- [ ] Cost waterfall chart (D3 or canvas)
- [ ] Pre-recorded `.cast` fallback

### Demo B: The Fleet
- [ ] 12x8 grid of agent tiles (96 total)
- [ ] Tile colors = model tier
- [ ] Progress bars per tile
- [ ] Signal ripple effects
- [ ] Replay pre-recorded trace at 20x speed
- [ ] Aggregate cost metrics

### Demo C: The Compounding (enhanced)
- [ ] Cost decline chart (line graph)
- [ ] Extrapolation to run 1000
- [ ] Knowledge entry visualization
- [ ] Before/after prompt comparison

### Demo D: The Injection (security demo)
- [ ] Left pane: stock agent sends email (bad)
- [ ] Right pane: roko blocks with capability gate
- [ ] Bottom: D3 force-directed capability graph
- [ ] Animated capability checks

### Demo E: The Replay
- [ ] 4 config variants of same trace
- [ ] Divergence/reconvergence visualization
- [ ] Uses `roko replay <hash> --override key=value`

### Demo F: Live Benchmark
- [ ] Pull HAL leaderboard live
- [ ] Run 3 SWE-bench tasks in parallel
- [ ] Append results
- [ ] QR code to methodology

---

## Dependency Map

```
Phase 0 (infra)
  ↓
Phase 1 (layout)  ←  Phase 2 (visual)
  ↓
Phase 3 (scenarios)
  ↓
Phase 4 (polish)
  ↓
Phase 5 (advanced)
```

Phase 2 can proceed in parallel with Phase 1. Phase 3 requires
both 1 and 2. Phase 5 items are independent and can be done in
any order after Phase 3.

---

## Effort Estimates

| Phase | Effort | Blocked by |
|---|---|---|
| Phase 0 remaining | 2h | Nothing (infra exists, cleanup only) |
| Phase 1 | 4h | Phase 0 |
| Phase 2 | 3h | Nothing (CSS only) |
| Phase 3 (scenarios 1-2) | 4h | Phase 1+2 |
| Phase 3 (scenarios 3-7) | 6h | Phase 3 (1-2) |
| Phase 4 | 4h | Phase 3 |
| Phase 5 (A+C) | 2d | Phase 4 |
| Phase 5 (B+D+E+F) | 1w | Phase 5 (A+C) |

**MVP (scenarios 1+2 + visual system):** ~13h
**Full (all 7 scenarios + polish):** ~23h
**Advanced demos:** +1-2 weeks
