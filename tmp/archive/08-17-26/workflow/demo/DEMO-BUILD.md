# Demo Build Requirements: Nunchi Series A

**Purpose**: What needs to be built, fixed, or polished to make the demo described in DEMO-STRATEGY.md and DEMO-FLOW.md real and reliable. Tiered by priority and effort. Written for someone with zero prior context about the current codebase state.

**Date**: April 2026

---

## 1. Current State Assessment

The demo has two surfaces: a CLI (terminal output) and a web dashboard (React + Vite SPA). Both exist today but neither is demo-ready.

### What Works Today

| Capability | Where | Status |
|-----------|-------|--------|
| `roko serve` HTTP control plane | `crates/roko-serve/`, ~115 API routes | Working, serves backend |
| `roko run "<prompt>"` single task | `crates/roko-cli/src/run.rs` | Working, executes via Claude CLI |
| Gate pipeline (compile, test, clippy) | `crates/roko-gate/`, called from `orchestrate.rs` | Working for Rust projects |
| Session persistence (checkpoint/resume) | `.roko/state/executor.json` | Working via `--resume` flag |
| CascadeRouter (model routing) | `.roko/learn/cascade-router.json` | Working, persists routing state |
| Knowledge store (NeuroStore) | `crates/roko-neuro/` | Working, stores/queries knowledge entries |
| Efficiency events (per-turn metrics) | `.roko/learn/efficiency.jsonl` | Working, records cost/tokens/latency |
| Episode logger (agent turn recording) | `.roko/episodes.jsonl` | Working, records gate results + HDC fingerprints |
| Terminal WebSocket sessions | `useTerminal.ts` → `/ws/terminal/{id}` | Working, real PTY sessions |
| React SPA with 7 pages | `demo/demo-app/src/pages/` | Working but poorly designed (see UI-AUDIT.md) |
| Embedded SPA in binary | `crates/roko-serve/src/embedded.rs` via `rust-embed` | Working, `cargo build` triggers `npm run build` |

### What Doesn't Work / Doesn't Exist

| Capability | Needed For | Current State |
|-----------|-----------|---------------|
| **Clack-style CLI output** | All demo beats | CLI output is raw text, no structured formatting |
| **Cost prediction before execution** | Beat 2 (predict-publish-correct) | Prediction logic exists in CascadeRouter but not surfaced in CLI output |
| **Knowledge loading message in CLI** | Beat 3 (shared knowledge) | Knowledge is loaded internally but not printed to user |
| **`--share` flag producing a URL** | The Stripe moment | Does not exist |
| **Shareable URL page** | Post-demo artifact | Does not exist |
| **`nunchi agents list` formatted output** | a16z Beat 1 | Agent listing exists via API but CLI format is raw |
| **`nunchi audit` command** | a16z Beat 2 | Does not exist as a CLI subcommand |
| **`nunchi replay` formatted output** | a16z Beat 5 | `replay` command exists but output is raw JSON |
| **Demo mode toggle in dashboard** | Technical diligence | Does not exist |
| **Knowledge graph visualization** | Dashboard View 3 | Does not exist |
| **Cost dashboard view** | Dashboard View 1 | Partial (BenchLive has some cost tracking, but not the designed layout) |
| **Agent fleet view** | Dashboard View 2 | Partial (Explorer page has some health data) |
| **Chain view** | Dashboard View 4 | Does not exist (chain is simulated via mirage-rs) |
| **Tokyo Night terminal theme** | CLI aesthetics | Terminal uses ROSEDUST (dusty rose), not Tokyo Night |
| **Geist Sans + Geist Mono fonts** | Dashboard typography | Dashboard uses system fonts + JetBrains Mono |
| **Evolved design tokens** | Dashboard visual quality | Uses ROSEDUST tokens (rose accents, serif display font) |

---

## 2. Build Tiers

### Tier 0: Critical Path (Must-Have for Any Demo)

These items are required for the 3-minute general VC demo to work. Without them, there is no demo.

#### T0.1: Clack-Style CLI Output Formatter

**What**: A Rust module in `roko-cli` that formats agent run output using Clack-style symbols (◆, ◇, │, └, ✔, ✖) with ANSI color codes. Replaces the current raw text output from `roko run`.

**Where**: New module `crates/roko-cli/src/output_format.rs`, called from `run.rs`.

**Sections to format**:
- Agent identity line (cyan, with verification badge)
- Predict line (yellow, with cost/time/route)
- Gates line (green checks or red crosses)
- Knowledge line (purple, with fact count and confidence)
- Progress bar (Unicode block elements, not ASCII)
- Result line (green/red, with actual cost and delta vs predicted)
- Share URL line (blue, underlined)
- Checkpoint line (when interrupted)
- Resume line (when resuming)

**Effort**: 2-3 days. The formatting logic is straightforward; the complexity is in surfacing data that currently only exists internally (predict cost, knowledge loading, gate results).

**Dependencies**: Requires T0.2 (cost prediction surface) and T0.3 (knowledge loading surface).

#### T0.2: Surface Cost Prediction in CLI

**What**: Before dispatching to the LLM backend, compute and print the predicted cost, estimated time, and routing decision.

**Where**: `crates/roko-cli/src/run.rs` or `orchestrate.rs`, immediately before the `dispatch_agent_with` call. The CascadeRouter already computes routing decisions internally — this surfaces them.

**Output format**:
```
◇ Predict
│  $0.043  ·  12.4s  ·  route: haiku → gpt-4o-mini
```

**Effort**: 1-2 days. The routing logic exists; the work is plumbing the prediction to the output formatter.

#### T0.3: Surface Knowledge Loading in CLI

**What**: When the NeuroStore is queried during dispatch enrichment, print a summary of what was loaded.

**Where**: `orchestrate.rs` `dispatch_agent_with` function, after knowledge hints are injected.

**Output format**:
```
◇ Knowledge
│  loaded 9 facts from /finance/q3  (4 agents, 0.93 avg conf)
```

**Effort**: 1-2 days. Knowledge loading happens internally; the work is summarizing and printing it.

#### T0.4: `--share` Flag and Shareable URL

**What**: A `--share` flag on `nunchi run` that, after completion, uploads the run result to the server and prints a URL.

**Components**:
1. A new API endpoint `POST /api/runs/{id}/share` that generates a shareable token
2. A new React page at `/share/:token` that renders the execution timeline, cost breakdown, and ZK proof section
3. The CLI prints the URL after the run completes

**Where**:
- Backend: new route in `crates/roko-serve/src/routes/`
- Frontend: new page in `demo/demo-app/src/pages/Share.tsx`
- CLI: flag handling in `crates/roko-cli/src/run.rs`

**Effort**: 3-5 days. The backend storage is straightforward (serialize run result to JSON, store with a token). The frontend page is a new design (see DEMO-VISUAL-SPEC.md section 6). The CLI change is minimal.

#### T0.5: `--resume` Flag Integration with Formatted Output

**What**: The `--resume` flag already works for plan execution. It needs to work for single `nunchi run` commands and produce formatted output showing the resume point.

**Output format**:
```
◇ Resuming
│  from checkpoint 3/7  ·  $0.012 spent  ·  4 steps remaining
```

**Effort**: 1-2 days if session persistence already works for single runs. If it doesn't, add 2-3 days to wire checkpoint saving per step.

---

### Tier 1: Demo Quality (Required for a16z Meeting)

These items elevate the demo from "working" to "impressive."

#### T1.1: `nunchi agents list` Formatted Output

**What**: A tabular display of agent fleet status matching the a16z demo Beat 1.

**Where**: `crates/roko-cli/src/` — either a new subcommand module or extending the existing `agent list` command.

**Output format**:
```
◆ Agent Fleet  ·  env: prod  ·  4 active

│ NAME           IDENTITY                      STATUS    ATTESTED  TASKS  UPTIME
│ researcher@v2  nhi://acme/researcher.v2      ✔ active  ✔ SPIFFE  47     4h 23m
│ ...
```

**Effort**: 1-2 days. The API route `/api/agents` already exists; the work is formatting the output.

#### T1.2: `nunchi audit` Command

**What**: A new CLI subcommand that runs a multi-step validation pipeline against a deployment, producing formatted step-by-step output with pass/fail indicators.

**Where**: New module `crates/roko-cli/src/audit.rs`.

**Components**:
- Orchestrates a sequence of checks (dependency audit, secret scan, static analysis, integration smoke, load test, compliance snapshot)
- Each check is a gate pipeline invocation
- Includes pre-seeded failure mode for the demo (configurable via a demo flag or environment variable)
- Credential rotation and PR creation via tool calls

**Effort**: 5-7 days. This is the largest single build item. The gate pipeline exists, but the audit command is a new orchestration layer with specific check types.

**Simplification option**: If time is short, the audit command can be a thin wrapper that calls pre-existing gates in sequence and formats the output. The individual checks don't need to be real for the demo — the output can be scripted. But the credential rotation and PR opening should be real (pre-staged in a demo repository).

#### T1.3: `nunchi replay` Formatted Output

**What**: The `replay` command already exists but outputs raw JSON. Format it as a readable event timeline.

**Output format**:
```
◆ Replay  ·  run_4823  ·  from step 05

│ EVENT  TIME         TYPE          DETAIL
│ 47     14:23:01.3   gate_check    integration-smoke: connection refused
│ 48     14:23:01.3   prediction    retry_success_prob: 0.72
│ ...
```

**Effort**: 1-2 days.

#### T1.4: Demo Dashboard Redesign

**What**: Redesign the web dashboard from 7 pages to 4 focused views matching the DEMO-VISUAL-SPEC.md specification. This is a full frontend rewrite.

**Four views**:
1. **Cost Dashboard**: 4 stat cards + cumulative cost chart + routing decisions panel
2. **Agent Fleet**: Grid of agent cards with identity, reputation, current task, cost
3. **Knowledge Graph**: Force-directed graph with d3-force + Canvas 2D
4. **Chain View**: Live block feed + summary stats

**Design system changes**:
- Replace ROSEDUST tokens with the evolved design tokens (DEMO-VISUAL-SPEC.md section 3)
- Replace serif display font (Playfair Display) with Geist Sans
- Replace system monospace with Geist Mono
- Replace dusty rose accent with accent blue (#4A9EFF)
- Implement luminance-based elevation (not color overlays)

**Effort**: 7-10 days for a complete rewrite. 3-5 days for a stripped-down version that only implements the Cost Dashboard and Agent Fleet views (deferring Knowledge Graph and Chain View to Tier 2).

**Dependencies**: Font files for Geist Sans and Geist Mono (available via npm: `geist/font`).

#### T1.5: Demo Mode Toggle

**What**: A "Demo Mode" button in the dashboard that:
- Hides secondary navigation
- Auto-cycles through the 4 views on a 45-second timer
- Shows keyboard shortcuts (1/2/3/4 for views, Space to pause)
- Enlarges stat card numbers by 20%
- Shows "LIVE" indicator in top-right

**Where**: `demo/demo-app/src/components/Layout.tsx` + new `DemoMode.tsx` component.

**Effort**: 1-2 days after T1.4 is complete.

---

### Tier 2: Visual Excellence (Differentiating for Competitive Demo)

These items make the demo visually competitive with Linear, Vercel, and Temporal.

#### T2.1: Knowledge Graph Visualization

**What**: Force-directed graph visualization of the knowledge store.

**Implementation options**:
- **Option A**: `react-force-graph-2d` (vasturiano) — renders to Canvas, supports zoom/pan/hover. Pre-built React wrapper.
- **Option B**: D3 force simulation with custom Canvas 2D rendering — more control, more work.

**Visual design**:
- Nodes = knowledge entries, sized by citation count
- Node color by domain (finance=amber, code=blue, research=purple)
- Edges = citations, opacity proportional to frequency
- Dimming nodes in demurrage decay
- New publications animate in with ripple effect
- Hover shows content summary, source agent, creation date

**Data source**: `GET /api/knowledge/graph` (new endpoint) or `GET /api/neuro/stats` (existing, may need extension).

- **Option C (RECOMMENDED)**: Terrain visualization using d3-contour + Canvas 2D. Compounding = elevation (peaks rise), demurrage = erosion (peaks shrink). ~200 LOC delta. d3-contour ships `contourDensity().weight()`. Position via UMAP/t-SNE. Per-frame KDE on 128x128 grid = 3-6ms on M1 → 60fps.
- **Secondary**: Mycelial/Physarum as landing page hero background. 200k agents at 60fps on integrated GPU.

**Effort**: 3-5 days for Option A. 5-7 days for Option B. 2-3 days for Option C.

#### T2.2: Chain View Simulation

**What**: A live block feed visualization using mirage-rs (the local EVM simulator) to produce realistic block data.

**Components**:
- Backend: mirage-rs runs alongside `roko serve`, producing blocks every 50ms
- New API endpoint: `GET /api/chain/blocks` (SSE stream) or WebSocket
- Frontend: Scrolling block feed with transaction summaries
- Summary stats: active agents, knowledge entries, ZK proofs, average block time

**Effort**: 3-5 days. mirage-rs exists but may need integration work to produce the right event format.

#### T2.3: Shareable URL Page (Full Design)

**What**: The `/share/:token` page from T0.4, implemented to the full DEMO-VISUAL-SPEC.md section 6 specification.

**Components**:
- Execution timeline (vertical, clickable steps)
- Cost breakdown bar (stacked horizontal)
- ZK proof section with verify link
- Agent identity with reputation
- Full dark theme matching dashboard tokens

**Visualization patterns**:
- **Execution trace**: Multi-track, time-aligned, canvas-rendered, hover-synced (Chrome DevTools + Datadog convergent pattern)
- **Computation receipt**: Hybrid mockup (dark canvas + cream receipt card at 340x480px, flip animation, downloadable PDF)
- **Cost comparison**: Crushed Bar pattern (two horizontal bars, 3.3% vs 100%, on-viewport animation)

**Effort**: 3-4 days (frontend only, assuming T0.4 provides the backend).

#### T2.4: Landing Page (nunchi.network)

**What**: The landing page described in DEMO-VISUAL-SPEC.md section 9.

**Components**:
- Hero with animated cost meter ($44.86 → $1.42 side by side)
- 7-section scroll story with viewport-triggered animations
- Code snippet presentation with language tabs and copy-to-clipboard
- Trust signals (GitHub stars, changelog, design partner logos)

**Implementation**: Can be built as a separate Next.js project or as an additional route in the existing Vite SPA. Next.js (deployed on Vercel) is recommended for SEO and OG metadata.

**Effort**: 5-7 days for a polished implementation.

#### T2.5: Pulse Globe (Agent Topology Visualization)

**What**: Three.js globe showing agent coordination arcs fanning out in real-time. `three-globe` library (vasturiano, MIT). Reference: janarosmonaliev/github-globe.

**Where**: `demo/demo-app/src/components/PulseGlobe.tsx`

**Key specs**: 6 Lens colors, UnrealBloomPass, slider drives emission rate AND real BusBridge throughput. 60fps at 5,000 arcs on M1.

**Effort**: 3-5 days.

---

### Tier 2b: Advanced Demo Mechanics (From Research)

These items are from competitive research on demo mechanics that no other agent infrastructure company has used. They are high-risk/high-reward and should only be attempted after Tier 0 and Tier 1 are solid.

#### T2b.1: Personalized Demo URL (`demo.nunchi.dev/martin`)

**What**: A URL that auto-routes to a session pre-loaded with the partner's context. Persists after the meeting. The partner can curl it from their phone. It keeps incrementing as the system runs.

**Components**:
- Server-side: named session routing in `roko-serve`
- Pre-loaded context: partner name, portfolio companies, relevant knowledge entries
- Persistent URL that stays live for 30+ days

**Effort**: 2-3 days.

#### T2b.2: On-Chain Verifier Role for Partner

**What**: During the meeting, the partner scans a QR → receives a temporary `verifier-role` on the L1 → co-signs attestations published during the demo → their identity is permanently in the verifier set on-chain.

**Components**:
- Sign-In-With-Ethereum (SIWE) modal
- Gitcoin-Passport-style stamp with 24-hour TTL
- ERC-8004 attestation co-signature flow
- QR code generation

**Effort**: 3-5 days. Requires mirage-rs integration for local chain.

#### T2b.3: Multi-Day Pre-Meeting Agent Run

**What**: Start Roko running 30 days before the pitch (April 6). Walk in May 6 with a month of execution logs. Shows durable systems over demo-day ephemera.

**Components**:
- A long-running task set (Linux kernel mailing list analysis, GitHub trending, or similar public corpus)
- Log aggregation and display
- The 30-day execution timeline as a scrollable artifact

**Effort**: 1 day to set up, 30 days to run. Minimal engineering, high narrative value.

#### T2b.4: Nunchi Cell PCB (~$30/unit)

**What**: Credit-card-sized PCB in black anodized sleeve. Etched HDC fingerprint pattern, e-ink display, NFC tap to explorer page. Engraved serial `nunchi-cell-0001`.

**Effort**: 2-3 weeks lead time for PCB fabrication. ~$30/unit. Not engineering work — hardware sourcing.

#### T2b.5: Sovereignty Zine (Physical)

**What**: 32-page perfect-bound zine in Berkeley Mono. Contains the pre-meeting essay, technical diagram, curated quotes. One copy, fine-press printed.

**Effort**: 1-2 days writing + 1-2 weeks print lead time. Design work, not engineering.

---

### Tier 2c: Live Benchmark Runner

These items enable a live, apples-to-apples cost/accuracy comparison between Roko and competing frameworks during the pitch meeting.

#### T2c.1: HAL Benchmark Harness Integration

**What**: Wrap Roko's agent dispatch as a `hal-harness` agent_fn so HAL can measure cost/accuracy apples-to-apples. Also wrap LangGraph/AutoGen as baselines.

**Where**: New `demo/benchmarks/` directory with `agents/roko_agent/main.py`, `agents/langgraph_agent/main.py`.

**Benchmark sources**: HAL (arXiv:2510.11977, https://github.com/princeton-pli/hal-harness), GAIA Level-1 (HuggingFace), AppWorld (StonyBrookNLP), τ-bench Airline.

**Effort**: 3-5 days.

#### T2c.2: Live Corner Widget (400×300px)

**What**: OBS-style overlay widget showing side-by-side Roko vs LangGraph benchmark progress. Bloomberg Two-Tape layout (recommended). WebSocket-driven from Weave logs.

**Where**: `demo/demo-app/src/components/BenchWidget.tsx` or standalone HTML.

**Implementation**: agent → litellm callback → Weave logger → tail JSON → Node WebSocket bridge (FastAPI ~40 lines) → React widget.

**Statistical significance**: Sequential proportion z-test, declare WINNER at p<0.01.

**Effort**: 2-4 days.

#### T2c.3: 5-Task Live Demo Subset

**What**: Curated 5-task subset that runs in ≤3 min with parallel=5. Pure Python only (no Docker cold-starts on stage).

**Tasks**: (1) τ-bench Airline task_0 (simple booking, ~20s), (2) τ-bench Airline task_4 (multi-leg + loyalty, ~30s), (3) AppWorld "Play playlist for workout" (~40s), (4) GAIA Level-1 arXiv physics question (~20s), (5) GAIA Level-1 grocery list question (~15s).

**Expected cost**: Roko ~$0.20 total vs LangGraph ~$6-8 → ~30× ratio preserved.

**Effort**: 1-2 days (after T2c.1).

#### T2c.4: Benchmark God-Mode Safeguards

**What**: Auto-skip + mark fail if task hangs >45s. Pre-pull all data before pitch. Pre-record 90s MP4 fallback. Static Pareto-frontier image as backup-of-backup.

**Effort**: 0.5 days.

---

### Tier 3: Polish (Nice to Have)

#### T3.1: Pre-Warmed Cache Automation

**What**: A script that runs before every demo meeting, warming the cache with all demo prompts.

**Where**: `demo/demo-resources/warm-cache.sh`

**Effort**: 0.5 days.

#### T3.2: VHS Recording Pipeline

**What**: VHS `.tape` files for each demo beat, producing high-quality GIF/MP4 recordings.

**Where**: `demo/demo-resources/recordings/`

**Effort**: 1 day.

#### T3.3: Deck Template in Figma

**What**: Figma file with the deck layout, typography, and color tokens matching the demo.

**Effort**: 2-3 days (design work, not engineering).

#### T3.4: `demo-magic` Script

**What**: Shell script using demo-magic for reproducible keystroke playback during the live demo.

**Where**: `demo/demo-resources/demo-magic/`

**Effort**: 0.5 days.

---

## 3. Priority Ordering and Dependencies

```
T0.1 (CLI formatter)
  ↑
T0.2 (cost prediction surface)  ←  required for T0.1
  ↑
T0.3 (knowledge loading surface)  ←  required for T0.1
  ↑
T0.5 (resume formatted output)  ←  required for T0.1
  ↑
T0.4 (--share flag + URL)  ←  can run in parallel with T0.1-T0.3

T1.1 (agents list format)  ←  independent
T1.2 (audit command)  ←  independent, largest item
T1.3 (replay format)  ←  independent
T1.4 (dashboard redesign)  ←  independent, second largest item
T1.5 (demo mode)  ←  depends on T1.4

T2.1 (knowledge graph)  ←  depends on T1.4
T2.2 (chain view)  ←  depends on T1.4
T2.3 (share page design)  ←  depends on T0.4
T2.4 (landing page)  ←  independent
T2.5 (Pulse Globe)  ←  independent

T2c.1 (HAL harness)  ←  independent
T2c.2 (corner widget)  ←  depends on T2c.1
T2c.3 (5-task subset)  ←  depends on T2c.1
T2c.4 (god-mode safeguards)  ←  depends on T2c.1
```

### Recommended Build Order

**Week 1**: T0.2, T0.3, T0.1, T0.5 (CLI output — the primary demo surface)
**Week 2**: T0.4 (--share), T1.1, T1.3 (supporting CLI commands), T1.4 start (dashboard)
**Week 3**: T1.2 (audit command), T1.4 finish (dashboard), T1.5 (demo mode)
**Week 4**: T2.1 (knowledge graph), T2.3 (share page), T2.5 (Pulse Globe), T3.1-T3.4 (polish)
**Week 5**: T2c.1 (HAL harness), T2c.2 (corner widget), T2c.3 (5-task subset), T2c.4 (safeguards)

Total estimated effort: **32-45 engineering days** for a complete, demo-ready build (including live benchmark runner).

### Minimum Viable Demo (If Time Is Short)

If only 1 week is available, build T0.1 + T0.2 + T0.3 + T0.5 only. This gives:
- Formatted CLI output with identity, prediction, knowledge, and resume
- The 3-minute general VC demo works from the terminal alone
- No web dashboard, no shareable URL, no audit command
- Use the VHS recording as backup

This is not ideal but is sufficient for a first meeting where the conversation can carry what the demo doesn't show.

---

## 4. Technical Notes

### Font Installation

Geist fonts are available via npm:
```bash
npm install geist
```

Then import in CSS:
```css
@import 'geist/font/sans';
@import 'geist/font/mono';
```

For the terminal (Ghostty), Geist Mono is available as a system font after installing via `brew tap homebrew/cask-fonts && brew install --cask font-geist-mono` (or download from vercel.com/font).

### Canvas 2D vs Library

All charts should use Canvas 2D directly, not a charting library. Reasons:
- Zero dependency footprint (no Chart.js, Recharts, etc.)
- Full control over animation timing and visual design
- Matches the existing approach in CostChart.tsx and BarChart.tsx
- The knowledge graph can use d3-force for layout calculation while rendering to Canvas 2D

### mirage-rs Integration

mirage-rs is the local EVM simulator in the crate workspace. For the chain view:
- Run mirage-rs as a background task alongside `roko serve`
- Have it produce blocks with knowledge publication events, identity attestations, and ZK proof verifications
- Feed events to the dashboard via SSE (`/api/chain/events`)
- The block data should look identical to what mainnet will produce

### Pre-Seeded Demo Data

The demo requires pre-seeded data for consistent output:
- 4 agent identities in the fleet
- 20+ knowledge entries in the NeuroStore (enough for the graph visualization)
- 50+ episode log entries (enough for cost trend data)
- A demo repository with a pre-staged AWS secret for the audit command
- A demo repository with a failing test for the "fix the failing test" prompt

Create a `demo/demo-resources/seed-data/` directory with all pre-seeded artifacts and a `seed.sh` script that loads them.

---

## 5. Acceptance Criteria

The demo is ready when:

1. **30-second init**: `nunchi init` → first agent output in under 30 seconds
2. **Formatted output**: Every CLI line uses Clack-style symbols with ANSI colors
3. **Cost prediction visible**: Every run shows predict vs actual with delta
4. **Knowledge loading visible**: Second agent shows "loaded N facts from M agents"
5. **Kill and resume**: Ctrl+C saves checkpoint, `--resume` continues from checkpoint
6. **Share URL works**: `--share` produces a URL that opens in a browser
7. **Dashboard loads**: http://localhost:5173 shows the Cost Dashboard view
8. **No errors in console**: Zero JavaScript errors, zero Rust panics during the demo flow
9. **Backup recording exists**: VHS recording of the full 3-minute demo, tested on projector
10. **Cache is warm**: All demo prompts return in under 10 seconds on second run

**Critical note on cost claims**: The $44.86 → $1.42 figures were not directly verifiable in published papers. Reproduce locally on the 5-task HAL subset before the meeting, or cite as "derived from HAL methodology." HAL splits by scaffold pattern, not by framework name — wrap LG/AG as agent_fn to get apples-to-apples numbers.

---

---

## 6. Codebase Reference for Implementation

This section provides the exact file paths, function signatures, and code patterns needed to implement each tier item. For complete codebase context (all crates, all routes, all components), see CODEBASE-CONTEXT.md.

### Where the `roko run` Loop Lives

**File**: `crates/roko-cli/src/run.rs`

The `run_once()` function is the universal execution loop. Every `roko run "<prompt>"` goes through: open substrate → build prompt sections → compose → dispatch agent → run gates → record episode → persist. The function returns a `RunReport` containing `episode_id`, `agent_success`, `gate_verdicts`, and `output_text`.

**To add Clack-style CLI output (T0.1)**: The output formatting needs to intercept each stage of `run_once()` and print structured output. Currently, `run_once()` returns a `RunReport` after completion — there is no streaming output during execution. The CLI wrapper in `run.rs` (the `cmd_run()` function that calls `run_once()`) is where formatted output should be added. Create a new module `crates/roko-cli/src/output_format.rs` with functions like `print_agent_line()`, `print_predict_line()`, `print_gate_line()`, `print_knowledge_line()`, `print_result_line()`, `print_share_line()`. Call these from `cmd_run()` at each stage.

### Where Cost Prediction Lives

**File**: `crates/roko-learn/src/` (CascadeRouter)

The CascadeRouter already computes routing decisions internally — it selects which model to use based on Thompson sampling / LinUCB bandit algorithms. The routing decision includes an estimated cost. This data is computed but NOT currently surfaced to the CLI output.

**To surface cost prediction (T0.2)**: The `dispatch_agent()` function in `run.rs` calls into the CascadeRouter (when providers are configured in `roko.toml`). The router returns a model selection with cost estimate. Capture this return value and pass it to the output formatter before the agent starts executing.

### Where Knowledge Loading Lives

**File**: `crates/roko-cli/src/orchestrate.rs` (the `dispatch_agent_with` function)

During dispatch enrichment, the system queries the NeuroStore for relevant knowledge entries. These are injected into the system prompt as "knowledge hints." Currently this happens silently — no output is printed.

**To surface knowledge loading (T0.3)**: After the NeuroStore query in `dispatch_agent_with()`, collect the returned entries (count, source agents, average confidence, domain path) and pass them to the output formatter.

### Where Session Persistence Lives

**File**: `.roko/state/executor.json`

The plan executor (`crates/roko-orchestrator/`) saves state snapshots after each completed step. The `--resume` flag loads the snapshot and continues from where it stopped. For single `roko run` commands, the equivalent checkpoint mechanism uses the signal substrate (`.roko/engrams.jsonl`) — each completed step is a persisted Signal that can be replayed.

**To format resume output (T0.5)**: When `--resume` is detected, load the snapshot, compute how many steps are complete vs remaining, and print the resume line before continuing execution.

### Where the HTTP Server Routes Are Defined

**File**: `crates/roko-serve/src/routes/mod.rs`

The `build_router()` function constructs the complete Axum router. Routes are organized as nested routers merged together. The fallback handler is `embedded::serve_embedded` which serves the React SPA.

**To add `--share` endpoint (T0.4)**: Add a new route `POST /api/runs/{id}/share` in routes/mod.rs that generates a shareable token and stores the run result. Add `GET /runs/{id}` (already partially exists) to serve the shareable page.

### Where the React SPA Lives

**Directory**: `demo/demo-app/src/`

43 source files, ~3,200 lines. Key patterns:

- **Routing**: `App.tsx` uses React Router v7 with `BrowserRouter`. All routes are children of `<Layout />`.
- **API calls**: All use the `useApi()` hook which wraps `fetch()` with `SERVE_URL` base URL resolution.
- **Terminal**: `useTerminal()` hook manages xterm.js lifecycle + WebSocket PTY sessions.
- **Design tokens**: `src/styles/rosedust.css` defines 34 CSS custom properties.
- **Charts**: Canvas 2D via `BarChart.tsx` and `CostChart.tsx` — DPR-aware, ResizeObserver-responsive.

**To redesign the dashboard (T1.4)**: Replace `src/styles/rosedust.css` with new design tokens (from DEMO-VISUAL-SPEC.md section 3). Replace the 7 pages with 4 focused views. The existing Canvas 2D charting pattern (`CostChart.tsx`, `BarChart.tsx`) should be preserved and extended.

**To add knowledge graph (T2.1)**: Create `src/components/KnowledgeGraph.tsx`. Use d3-force for layout calculation + Canvas 2D for rendering (matching the existing chart pattern). Data from `GET /api/knowledge/entries` and `GET /api/knowledge/edges`.

### Where the Embedded Asset Serving Lives

**File**: `crates/roko-serve/src/embedded.rs`

Uses `rust-embed` with `#[folder = "../../demo/demo-app/dist/"]`. The `serve_embedded()` handler:
1. Strips leading `/` from request path
2. Tries exact file match in embedded assets
3. Falls back to `index.html` (SPA routing)
4. Sets MIME type from file extension
5. Cache-Control: immutable for `/assets/*`, no-cache for everything else

### Where the Build Script Lives

**File**: `crates/roko-serve/build.rs`

Runs during `cargo build`. Steps: check `package.json` exists → skip if `SKIP_FRONTEND_BUILD` set → `npm install` if no `node_modules` → `npm run build` → declare rerun triggers for `src/`, `index.html`, `package.json`, `vite.config.ts`, `tsconfig.json`.

### The 7 Demo Scenarios (currently hardcoded)

**File**: `demo/demo-app/src/lib/demo-scenarios.ts`

7 scenarios defined as static objects: `selfhost`, `builder`, `race`, `providers`, `explore`, `chat`, `mirage`. Each has: `id`, `title`, `subtitle`, `panes` (1/2/4), `panel` (boolean), `promptBar` (boolean), `labels` (per-pane), `steps` (label + sublabel). None of these scenarios execute real commands — the Demo page creates real terminal WebSocket sessions but the "play" button just advances through steps with setTimeout.

### API Endpoints Available for the Dashboard

The React SPA currently uses only 11 of ~115 available endpoints. Key unused endpoints that could power new dashboard views:

| Endpoint | What It Returns | Potential Dashboard Use |
|----------|----------------|----------------------|
| `GET /api/dashboard` | Full dashboard snapshot | Cost Dashboard (View 1) |
| `GET /api/metrics/c_factor` | C-factor efficiency composite | Cost Dashboard metric |
| `GET /api/metrics/model_efficiency` | Per-model efficiency | Cost Dashboard routing panel |
| `GET /api/metrics/gate_rate` | Gate pass/fail rates | Cost Dashboard gate stat |
| `GET /api/agents` | Aggregated agent list | Agent Fleet (View 2) |
| `GET /api/agents/topology` | Agent dependency graph | Agent Fleet connections |
| `GET /api/agents/{id}/stats` | Per-agent stats | Agent Fleet cards |
| `GET /api/knowledge/entries` | Knowledge store entries | Knowledge Graph (View 3) |
| `GET /api/knowledge/edges` | Citation edges | Knowledge Graph edges |
| `GET /api/knowledge/search` | Knowledge search | Knowledge Graph search |
| `GET /api/chain/status` | Chain connection status | Chain View (View 4) |
| `GET /api/chain/agents` | On-chain agent registry | Chain View agent list |
| `GET /api/learn/cascade-router` | Router model weights | Cost Dashboard routing |
| `GET /api/learn/experiments` | A/B experiment state | Advanced analytics |
| `GET /api/learn/adaptive-thresholds` | Gate threshold EMA | Advanced analytics |

---

*Cross-references: CODEBASE-CONTEXT.md (complete technical reference), DEMO-STRATEGY.md (what and why), DEMO-VISUAL-SPEC.md (detailed design), DEMO-COMPETITIVE.md (competitive landscape), DEMO-FLOW.md (beat-by-beat script).*
