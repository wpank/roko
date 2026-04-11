# 17 — Demo: Complete the VC Story

> Cross-cutting plan covering `tmp/workflow/demo/`. Strategy + implementation. Lower priority than 01–16.

---

## Status (2026-05-01)

**PARTIAL & STRATEGY DRIFTING.**

The demo docs propose a **CLI-first** VC demo (3 minutes, four primitives: identity, cost prediction, shared knowledge, durability) with a `--share` URL artifact and supplementary `roko dashboard` web UI.

The repo today shows **heavy investment in the web demo app** (`demo/demo-app/` — 15 scenarios, BlockTicker, ISFR, scenario-runners, real-PTY terminals) and **partial investment** in the CLI-first VC story (`roko run --share` exists; `share.rs`, `shared_runs.rs` route present).

Decision needed before this plan can fully land: **is the demo strategy still CLI-first, or has it shifted to the web demo as the primary surface?**

---

## Goal Options

The user must pick one of:

### Option A — Stick with the original CLI-first strategy

Polish `roko run` output to match `DEMO-FLOW.md` (Clack-style sections, surfaced predict/knowledge/resume/share). Treat the web demo app as supplementary.

### Option B — Pivot to web-first

Make `demo/demo-app/` the canonical demo. Polish the scenarios + scripted beats to match the docs' four-primitives narrative. CLI is supporting cast.

### Option C — Both, with explicit roles

CLI for "live" investor demo (3 min). Web for diligence follow-up + technical deep dive. Each polished for its role.

This plan assumes **Option C** (most aligned with current repo trajectory and docs intent). Adjust if the user chooses A or B.

---

## What Each Surface Needs

### CLI surface (per `DEMO-FLOW.md` + `DEMO-BUILD.md` T0.1–T0.3)

Required in `roko run` output:

```
◆ Plan
│  Implement add(a, b: i32) -> i32 with:
│    • input validation
│    • error handling
│    • unit tests
└──

◇ Predict                      $0.18 · 3 turns · 23s
│   model: claude-sonnet-4
│   route: bandit (5 prior runs)

◇ Knowledge                    8 entries loaded
│   from: 3 prior implementer runs
│   relevance: 0.84

◇ Run                          ◐ implementing
│   ⚙ read_file Cargo.toml
│   ⚙ edit_file src/lib.rs (+24 -3)
│   ⚙ bash cargo test (3 passed)

✔ Gates                        all passed
│   compile · 1.2s · ✔
│   clippy  · 0.8s · ✔
│   test    · 3.1s · ✔ (3/3)

◆ Done                         $0.13 · 3 turns · 28s
│   actual vs predicted: -28% cost · +22% time
│
│   Share: nunchi://run/abc123
│   Or:   https://share.nunchi.dev/r/abc123
```

Required CLI features:

1. **Predict block** — model + route + cost estimate
2. **Knowledge block** — count + source + relevance
3. **Run block** — live tool calls (already done via plan 16)
4. **Gate block** — verdicts (already done)
5. **Done block** — actual vs predicted delta
6. **Share** — `--share` flag prints URL after success
7. **Resume** — `Ctrl+C` saves checkpoint; `roko run --resume <id>` continues without losing work

### Web surface (per `DEMO-IMPLEMENTATION.md` + UI-AUDIT.md)

Per current repo state:

- 15 scenarios are wired (prd-pipeline, knowledge-transfer, gate-retry, providers, etc.)
- Block ticker (live newHeads via WebSocket) is present
- ISFR Panel is present
- Top nav, dashboard, demo page are present

What's missing from the docs' visual spec:

1. **Pulse Globe** (cold-open hero animation: 5K arcs, 6 Lens colors, UnrealBloomPass) — spec'd in DEMO-VISUAL-SPEC.md, not built
2. **Terrain knowledge viz** (d3-contour + Canvas 2D, compounding=elevation, demurrage=erosion) — spec'd, not built
3. **Bloomberg Two-Tape** live benchmark widget (400×300px corner overlay) — spec'd, not built
4. **Hybrid receipt** (dark canvas + cream receipt card flip + downloadable PDF) — spec'd, not built
5. **Crushed Bar** cost comparison (3.3% vs 100% Tufte-correct) — spec'd, not built
6. **Tokyo Night** terminal theme + Geist Mono font — partially adopted
7. **Scripted resume beat** — no scenario currently performs Ctrl+C + `--resume`
8. **Predicted vs actual** display in scenario output

---

## Implementation Steps — Option C (CLI + web tracks)

### Track 1 (CLI) — Estimated effort: M (1 week)

#### Step 1.1 — Output formatter module

```rust
// crates/roko-cli/src/output_format/mod.rs
pub trait RunOutputFormatter {
    fn plan(&mut self, plan: &PlanBlock);
    fn predict(&mut self, predict: &PredictBlock);
    fn knowledge(&mut self, knowledge: &KnowledgeBlock);
    fn run_started(&mut self);
    fn tool_call(&mut self, call: &ToolCallEvent);
    fn gate_verdict(&mut self, verdict: &GateVerdict);
    fn done(&mut self, summary: &DoneBlock);
    fn share_url(&mut self, url: &str);
}

pub struct ClackStyle { theme: Theme, terminal: InlineTerminal }
pub struct PlainStyle { writer: Box<dyn Write> }
```

`roko run` calls `formatter.predict(...)` after `PromptAssemblyService::assemble` returns diagnostics; calls `formatter.knowledge(...)` from same source.

#### Step 1.2 — Predict block source

`PredictBlock { model, route_source, estimated_cost, estimated_turns, estimated_seconds }`. Sources:

- `model` from `ResolvedRuntimeConfig` or `CascadeRouter::select_for_frequency_among` result (per plan 08)
- `route_source` is "bandit (N prior runs)" or "default (no observations)"
- `estimated_cost` from `roko_learn::predict::estimate_cost(role, task_complexity)` (build a simple regression on `efficiency.jsonl`)
- `estimated_turns`/`estimated_seconds` from `task_metrics.jsonl` p50

#### Step 1.3 — Knowledge block source

`KnowledgeBlock { entry_count, source_summary, relevance }`. Sources:

- `assembled.diagnostics.knowledge_ids.len()` (per plan 02)
- Source summary from neuro store: `for id in knowledge_ids: store.entry(id).source`
- Relevance from store query result

#### Step 1.4 — Done block (predicted vs actual)

`DoneBlock { actual_cost, predicted_cost, actual_turns, predicted_turns, actual_seconds, predicted_seconds }`. Compute delta strings: `"-28% cost · +22% time"`.

#### Step 1.5 — Resume polish

`roko run --resume <run_id>` already nominally exists via the resume infrastructure. Verify:

- `Ctrl+C` mid-run prints `Saved checkpoint to .roko/state/run-<id>.json. Resume with: roko run --resume <id>`
- `roko run --resume <id>` resumes from the checkpoint without re-running completed phases
- Predicted-vs-actual reflects the **partial** run (delta on resume start, then continued)

#### Step 1.6 — Share URL polish

`crates/roko-cli/src/share.rs` already produces share output. Make `--share` print:

```
Share: nunchi://run/abc123        (deep link)
       https://share.nunchi.dev/r/abc123    (web)
```

If `share.nunchi.dev` is not reachable, fall back to local: `http://127.0.0.1:7777/runs/abc123`.

#### Step 1.7 — Tokyo Night terminal theme

Per `DEMO-VISUAL-SPEC.md`, switch the demo theme to Tokyo Night palette:

```rust
// crates/roko-cli/src/inline/themes/tokyo_night.rs
pub fn tokyo_night() -> Theme {
    Theme {
        text: Color::Rgb(192, 202, 245),
        text_dim: Color::Rgb(149, 154, 171),
        bg: Color::Rgb(26, 27, 38),
        accent: Color::Rgb(122, 162, 247),       // electric blue
        success: Color::Rgb(158, 206, 106),
        error: Color::Rgb(247, 118, 142),
        warning: Color::Rgb(224, 175, 104),
        info: Color::Rgb(187, 154, 247),
    }
}
```

Selected via `ROKO_THEME=tokyo-night` env var or `[ui].theme = "tokyo-night"` config.

### Track 2 (Web) — Estimated effort: L (2-3 weeks)

#### Step 2.1 — Decide what's actually needed for the demo

The web app already has 15 scenarios. Decide which 3-5 are demo-critical:

- **`prd-pipeline`** (PRD → plan → execute end-to-end) — yes
- **`knowledge-transfer`** (cross-agent knowledge reuse) — yes (matches "Beat 3" of DEMO-FLOW.md)
- **`gate-retry`** (gate failure → autofix) — yes
- **`providers`** (live multi-provider race) — yes
- **`chain-intelligence`** (Nunchi L1 block streaming) — yes (the differentiator)

Cut or de-prioritize: `mirage`, `isfr-agents`, `dream-consolidation`, `explore`, `chat` (chat better demoed in CLI).

#### Step 2.2 — Implement Pulse Globe (cold-open)

Per `DEMO-VISUAL-SPEC.md` § Pulse Globe:

- `three-globe` + `Three.js` + `UnrealBloomPass`
- 5K arcs at 60fps
- 6 Lens colors mapped to subsystems
- Cold-open animation when landing page loads

File: `demo/demo-app/src/components/PulseGlobe.tsx` + `pulse-globe.css`.

Use existing `@react-three/fiber` if installed; otherwise `npm install three three-globe @react-three/fiber @react-three/drei`.

(Use `yarn add three three-globe @react-three/fiber @react-three/drei` per project convention.)

#### Step 2.3 — Implement Terrain knowledge viz

Per `DEMO-VISUAL-SPEC.md` § View 3:

- `d3-contour` for elevation contours
- Canvas 2D for rendering
- Map: knowledge entry confidence → elevation; demurrage decay → erosion
- 60fps on M1

File: `demo/demo-app/src/components/KnowledgeTerrain.tsx`.

#### Step 2.4 — Implement Bloomberg Two-Tape benchmark widget

Per `DEMO-VISUAL-SPEC.md` § 7:

- 400×300px corner overlay
- Side-by-side: Roko vs LangGraph live ticker
- p<0.01 winner declaration

File: `demo/demo-app/src/components/BenchmarkTicker.tsx`. Backend serves at `GET /api/benchmark/live` (synthetic for demo if real benchmark not running).

#### Step 2.5 — Hybrid receipt (PDF artifact)

Per `DEMO-VISUAL-SPEC.md` § 6:

- Dark canvas (running) → cream receipt card (done) flip animation
- "Download PDF" button generates a PDF via browser print or a server-side service

File: `demo/demo-app/src/components/RunReceipt.tsx` + PDF endpoint at `GET /api/runs/{id}/receipt.pdf`.

#### Step 2.6 — Crushed Bar cost chart

Per `DEMO-VISUAL-SPEC.md` § 6:

- Tufte-correct at 30× ratio (`3.3%` vs `100%`)
- On-viewport animation
- Canvas 2D

File: `demo/demo-app/src/components/CrushedBarCost.tsx`.

#### Step 2.7 — Add scripted resume beat to scenarios

`demo/demo-app/src/lib/scenario-runners/resume-checkpoint.ts`:

1. Spawn `roko run "implement add(a, b)"` PTY
2. Wait 5 seconds
3. Send Ctrl+C
4. Show checkpoint save message + resume command
5. Spawn `roko run --resume <id>` PTY
6. Show continued execution

Add to scenario list in `Demo/index.tsx`.

#### Step 2.8 — Adopt Tokyo Night + Geist on web

Update `demo/demo-app/src/styles/tokens.css` with Tokyo Night palette. Replace ROSEDUST in scenario UIs.

`yarn add @vercel/geist` (or load Geist via Google Fonts CDN).

### Track 3 (Cross-cutting) — Estimated effort: S (3 days)

#### Step 3.1 — `roko share doctor`

CLI command: `roko share doctor` checks the share endpoint, prints diagnostic:

- Local share server reachable? (if `roko serve` running)
- Remote `share.nunchi.dev` reachable?
- Auth token configured?
- Last share URL produced

Useful for live demo — operator can verify the share link works before presenting.

#### Step 3.2 — Pre-demo rehearsal script

`scripts/demo-rehearsal.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "[1/5] Verifying environment..."
roko config doctor
echo "[2/5] Verifying share..."
roko share doctor
echo "[3/5] Pre-warming knowledge store..."
roko run "implement multiply(a, b)" --quiet || true   # populates knowledge for the second-agent beat
echo "[4/5] Pre-warming router..."
for _ in $(seq 5); do roko run "fix typo in README" --quiet || true; done
echo "[5/5] Ready. Run: roko run \"implement add(a, b: i32) -> i32 with input validation\" --share"
```

Run this 10 minutes before any pitch. Avoids "first-call slowness" embarrassment.

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #2 Inline prompt strings | Demo scenarios hardcoding "implement add(a, b)" | Centralize demo prompts in `demo/demo-app/src/lib/demo-prompts.ts` |
| #3 Build another runtime | Adding a "demo mode" flag that diverges from production | Demo uses production code paths; no special-casing |
| #5 Hardcoded role behavior | Demo scenarios using a hidden "demo" role | Scenarios use real roles (implementer, etc.) |
| #10 God file | `Demo/index.tsx` ballooning past 1K LOC | Each scenario in its own file under `scenario-runners/` |

---

## Things NOT To Do

1. **Don't fake the demo.** The cost numbers, knowledge counts, predicted vs actual must be **real** outputs from the actual binary. Faking is the fastest way to lose credibility.
2. **Don't add a "demo" feature flag.** The demo is the production path.
3. **Don't leave the share URL ambiguous.** If `share.nunchi.dev` is not yet operational, decide: build the service, OR fall back to local-only and explain.
4. **Don't promise the Pulse Globe / Terrain viz on the schedule unless the team has a graphics specialist.** Both are tricky; budget L not M.
5. **Don't mix demo polish with retirement (plan 12).** Demo polish runs against `WorkflowEngine` once 11 lands. Don't wait, but don't try to retire stuff while demoing.
6. **Don't break the existing 15 scenarios.** They're shipped; web demo is in active use. Additive changes only.
7. **Don't depend on third-party APIs for the cold-open.** Pulse Globe should render from local data, not require API calls during the demo.
8. **Don't introduce a separate visual design system.** Use Tokyo Night across CLI + web, not different palettes per surface.

---

## Tests / Proof Criteria

- [ ] `roko run "implement add(a, b)"` produces output matching the `DEMO-FLOW.md` § Beat 1 transcript (visual diff)
- [ ] `roko run --resume <id>` after Ctrl+C continues without re-running completed phases
- [ ] `roko run --share` prints both `nunchi://run/<id>` and HTTPS share URL
- [ ] Demo rehearsal script (`scripts/demo-rehearsal.sh`) runs end-to-end in < 60 seconds
- [ ] Web demo: clicking "knowledge-transfer" scenario shows two consecutive PTY runs with the second using lower cost (within 30% of expected delta)
- [ ] Web demo: clicking "resume-checkpoint" scenario shows successful resume after Ctrl+C
- [ ] Pulse Globe (if built) renders 5K arcs at ≥ 30 FPS on a typical M1 Mac
- [ ] Knowledge Terrain (if built) renders without dropped frames during the 3-minute demo
- [ ] All built-but-unused web components (PulseGlobe, Terrain, BenchmarkTicker, RunReceipt, CrushedBar) appear in at least one demo scenario

---

## Dependencies

- **Plan 02 (PromptAssembly)** — for `prompt_section_ids` + `knowledge_ids` to surface in CLI output
- **Plan 03 (FeedbackService)** — for cost/predict data to be available
- **Plan 04 (PersistenceService)** — for resume to work robustly
- **Plan 08 (CascadeRouter)** — for the "bandit (5 prior runs)" route source string
- **Plan 10 (Observability)** — for share page to read from canonical `RuntimeProjection`
- **Plan 16 (CLI/TUI rendering)** — for the inline primitives the new formatter uses

Track 1 (CLI) blocks on these. Track 2 (web) is largely independent, can start anytime.

---

## Estimated Effort

**L overall.**

- Track 1 (CLI polish): M (1 week)
- Track 2 (web polish): L (2-3 weeks; dominated by Pulse Globe + Terrain)
- Track 3 (cross-cutting): S (3 days)

Recommendation: do Track 1 + Track 3 first (matches the docs' "CLI primary" priority). Track 2 is for after the pitch is rehearsed and the CLI demo is bulletproof.
