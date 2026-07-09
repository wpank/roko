# Roko Demo & UI Audit

A complete description of every user-facing surface in the roko project: the web UI, the demo scripts, and the server behind them. Written for someone encountering this project for the first time.

---

## What is Roko?

Roko is an AI agent toolkit written in Rust (~177K lines of code across 18 crates). Its central thesis is **self-hosting**: roko is meant to develop itself. You describe an idea, and roko turns it into a product requirement document (PRD), generates an implementation plan, dispatches AI agents to write the code, validates the output through a gate pipeline (compile, test, lint, diff), records what happened, and uses that data to improve its next run.

The system has three interfaces:

1. **CLI** (`roko`) — the primary interface. ~100+ subcommands for everything from `roko run "build a calculator"` to `roko prd idea "add dark mode"` to `roko dashboard` (a terminal UI).

2. **HTTP server** (`roko serve`) — an Axum web server on port 6677 exposing ~115 REST/WebSocket/SSE endpoints. This is the control plane — every CLI action has an HTTP equivalent, and dashboards connect to it.

3. **Web UI** (`demo/demo-app/`) — a React single-page application served by `roko serve`. This is the subject of this audit. It's embedded into the Rust binary at compile time so `roko serve` ships everything in one executable with no external dependencies.

There is also a collection of **demo scripts** (`tmp/demo-resources/`) — shell scripts that exercise the CLI and HTTP API to validate and demonstrate the system. These scripts are better tests of the actual product than the web UI is.

---

## Key Concepts (Glossary)

These terms appear throughout the UI and scripts. Understanding them is necessary to evaluate what each page is trying to show.

| Term | What it means |
|------|--------------|
| **PRD** (Product Requirement Document) | A structured work item. Lifecycle: idea → draft → published → plan generated. PRDs are how roko knows what to build. |
| **Plan** | A set of tasks (stored as `tasks.toml`) generated from a PRD. Plans have a DAG structure — tasks can depend on each other and run in parallel where possible. |
| **Agent** | An AI worker. Agents have manifests (name, domain, skills), connect to LLM backends (Claude, GPT, Ollama, etc.), and run as sidecar processes with their own HTTP endpoints. Multiple agents can run simultaneously. |
| **Gate** | A validation check run after an agent completes work. The gate pipeline has 7 "rungs" of increasing strictness. Common gates: compile, test, clippy (Rust linter), diff review. If gates fail, the system can trigger a replan. |
| **Episode** | A recorded unit of work — one agent turn, one gate result, one tool call. Episodes are logged to `.roko/episodes.jsonl` and form the basis of the learning system. |
| **Job** | A marketplace work item. Jobs have a state machine: open → assigned → in_progress → submitted → completed (or rejected → rework). Agents are matched to jobs based on skills, tier, and reputation. |
| **Matchmaking** | The system for finding the right agent for a job. You post a job with required skills and constraints, and the system returns ranked candidates with estimated fees and completion times. |
| **Cascade Router** | An adaptive model routing system. Instead of always using one LLM, the cascade router learns which model works best for which type of task, routing easy tasks to cheap models and hard tasks to expensive ones. |
| **Efficiency** | Per-task metrics: cost in USD, token count, latency, pass/fail. Stored in `.roko/learn/efficiency.jsonl`. The system uses this data to improve routing and budgeting over time. |
| **Provider** | An LLM backend (Anthropic, OpenAI, Ollama, etc.). Providers are configured in `roko.toml` and health-checked by the server. |
| **Sidecar** | Each agent can run as a standalone HTTP server (`roko agent serve`). Sidecars self-register with `roko serve`, accept messages, and can be discovered by other agents. |
| **StateHub** | An internal event bus. Dashboard events (plan started, gate failed, agent idle) flow through StateHub and can be consumed via SSE or WebSocket. |
| **ROSEDUST** | The visual design system. Dark purple-black backgrounds, dusty rose/pink accents, monospace typography, glassy semi-transparent card surfaces. The aesthetic is "atmospheric hacker control room." |

---

## Part 1: The Web UI (`demo/demo-app/`)

### How it's built and served

The web UI is a React 19 + TypeScript single-page application built with Vite 6. It uses React Router 7 for client-side navigation and CSS custom properties (no CSS framework) for styling.

When you run `cargo build`, a build script (`build.rs`) automatically runs `npm run build` inside `demo/demo-app/`. The compiled output (`dist/`) is baked into the Rust binary using `rust-embed`. When `roko serve` starts, it serves the SPA from memory — no filesystem, no Node.js, no separate web server needed.

In development, you can run `npm run dev` in `demo/demo-app/` for hot-reload. Vite proxies API calls (`/api/*`, `/ws/*`, `/health`) to `roko serve` on port 6677.

### Navigation and Layout

Every page shares a common layout:

- **Top bar**: A `◆ roko` logo on the left, 7 navigation tabs in the center (numbered 0-6), and a server health indicator on the right.
- **Health indicator**: A colored dot (green = ok, yellow = degraded, red = offline) that polls `GET /api/health` every 5 seconds.
- **Content area**: Each page fills the space below the top bar.

The 7 pages, in tab order:

| # | Path | Name | One-line description |
|---|------|------|---------------------|
| 0 | `/` | Home | Landing page with links to all other pages |
| 1 | `/demo` | Demo | Scripted pitch demo with 7 scenarios |
| 2 | `/terminal` | Terminal | Multi-pane browser terminal |
| 3 | `/builder` | Builder | Type a prompt, watch it get built |
| 4 | `/explorer` | Explorer | Server health, status, episode log, event stream |
| 5 | `/bench` | Bench | Benchmark configuration and results |
| 6 | `/bench-live` | Live | Real-time benchmark monitor |

---

### Page 1: Home (`/`)

The landing page. Shows the title `◆ roko serve` with subtitle "agent runtime control plane", a server health dot, and a list of links organized into four sections:

- **demo** — Links to the Demo, Benchmark Lab, Live Monitor, Builder, and Terminal pages.
- **explore** — Link to the Explorer page.
- **api** — Direct links to raw JSON API endpoints: `/api/health`, `/api/status`, `/api/episodes`, `/api/terminal/sessions`. Clicking these opens the raw JSON response in the browser.
- **docs** — Link to the OpenAPI specification at `/api/openapi.json`.

**What it tells you about the system**: Very little. It's a directory of links. There are no metrics, no system state preview, no indication of what roko does or whether it's doing anything right now. A first-time visitor would see a dark page with some links and no context.

**What data it uses**: Only the health check poll (for the dot).

**Issues**: The most valuable screen in the app — the first thing you see — is an unformatted link list. The API links go to raw JSON. There's no dashboard summary, no activity feed, no "here's what roko is doing right now."

---

### Page 2: Demo (`/demo`)

A "pitch demo" page designed to walk someone through 7 scenarios showcasing roko's capabilities. The idea is that you'd show this to an investor or potential user, clicking Play on each scenario while narrating.

**Layout**: A tab bar across the top with 7 numbered scenario buttons, a speed control (0.5x to 4x), and a Play button. Below that, an intro band with the scenario's title and subtitle. The main area splits into terminals on the left (1, 2, or 4 xterm.js panes depending on the scenario) and a panel on the right with stats and a timeline.

**The 7 scenarios**:

1. **Self-Hosting** — "Watch roko develop itself — from idea to running code." Five steps: capture idea, draft PRD, generate plan, check status, inspect learning. Single terminal pane.

2. **Build** — "Type a prompt. Roko builds it, validates with gates, shows cost." Three steps: submit prompt, agent builds, gates validate. Single terminal with a prompt bar concept.

3. **Cost Race** — "Same task, two approaches. Left: naive single-model. Right: cascade-routed." Two terminal panes side by side showing the cost difference between naive and smart routing.

4. **Providers** — "One prompt, four providers, simultaneously." Four terminal panes, each running the same prompt on a different LLM (Zhipu GLM-4, OpenAI GPT-4o, Anthropic Haiku, Moonshot v1).

5. **Explore** — "18 crates, 85 routes, 100+ commands." Four terminal panes, each showing a different capability family: workspace commands, learning commands, config commands, knowledge commands. Twelve steps.

6. **Chat** — "Just type roko." Three steps showing the interactive TUI: start, send message, use slash commands.

7. **Mirage** — "Fork any EVM chain locally." Single terminal. No steps defined — this is an empty placeholder for a blockchain feature.

**How playback actually works**: When you press Play, a JavaScript timer advances through the steps. Each step waits a few seconds (adjustable by speed), then the timeline marker moves to the next step. The stats panel (Model, Cost, Tokens, Time) updates with **randomly generated fake numbers** — a random cost increment between $0.02 and $0.10, a random token count, and a calculated time based on step count.

The terminal panes are real xterm.js instances that attempt to connect to the server's WebSocket PTY endpoint. However, **no commands are ever sent to them**. The demo doesn't type anything into the terminals. They just sit there showing a blank or disconnected state.

The right-side panel also includes an empty `<canvas>` element that is never drawn to — it was planned for a visualization but never implemented.

**What it tells you about the system**: The scenario titles and descriptions accurately describe real roko features. The self-hosting loop, multi-provider support, cascade routing, and the CLI command surface all exist in the actual product. But the demo doesn't demonstrate any of them — it simulates progress with fake numbers and empty terminals.

**What data it uses**: None. Everything is client-side simulation.

**Issues**: This page promises a live demonstration but delivers a slideshow with random numbers. The terminals never show actual commands running. The canvas visualization is empty. The Mirage scenario is a stub. None of the 7 scenarios connects to the real roko binary in any way.

---

### Page 3: Terminal (`/terminal`)

A browser-based terminal emulator page. The concept: you get real shell access to the server machine through the browser, with the ability to open multiple terminal panes side by side.

**Layout**: A toolbar with the title "terminal", an info line, and controls. The controls let you add terminals, switch between 1/2/3 column layouts, and clear all sessions. Below the toolbar, terminal panes fill the page. A status bar at the bottom shows the session count.

**How it works**: Each terminal pane uses the `useTerminal` hook, which:
1. Sends a `POST /api/terminal/sessions` request to create a server-side PTY (pseudo-terminal) process.
2. Opens a WebSocket connection to `ws://host/ws/terminal/{id}`.
3. Bridges keyboard input from the xterm.js instance to the WebSocket, and server output back to the terminal display.
4. Handles terminal resize events so the server-side PTY stays in sync with the browser viewport.

This is one of the few pages that connects to real server functionality. When working, you get an actual shell session — you can run commands, see output, use tab completion. The xterm.js terminals support colors, cursor movement, and scrollback.

**What data it uses**: `POST /api/terminal/sessions` (create session), `GET /ws/terminal/{id}` (WebSocket data bridge).

**Issues**: The terminals frequently show "disconnected" because WebSocket connections fail if the server isn't running or if the browser has cached an older version of the JavaScript with incorrect WebSocket paths (a bug that was fixed but persists in browser cache). There's no visual indication of which terminal is focused, no way to rename sessions, and the "Clear All" button destroys all sessions without asking for confirmation.

---

### Page 4: Builder (`/builder`)

The "type a prompt, watch it get built" page. The idea: you describe what you want (e.g., "Build a CLI calculator in Rust") and roko builds it in a temporary directory, showing the process in a terminal and validating the result through gates.

**Layout**: A header with the title "builder", a description, and 5 preset task buttons (calculator, REST API, markdown→HTML converter, file deduplicator, commit message generator). Below that, a split view: file sidebar on the left (listing created files), terminal pane on the right. At the bottom, a gate bar showing 4 validation gates (compile, test, clippy, diff), each with a pass/fail/pending/skip indicator. Below the gate bar, a prompt input with a Build button. A status bar shows the current status and file count.

**How it works**: When you type a prompt and click Build (or click a preset), the page sends `POST /api/run` with the prompt text and a temporary working directory. The API returns a `run_id`. The page is supposed to poll for completion and show results, but the current implementation has a comment reading "In a real implementation we'd poll /api/run/{id}/status here." Instead, it immediately sets all 4 gates to "pass" regardless of what actually happened.

The terminal pane is a real xterm.js instance, but it's connected to its own independent PTY session — it doesn't show the output of the build. The file sidebar only populates if the API returns a `files` field, which the real API doesn't do, so it always shows "no project yet."

**What it tells you about the system**: The gate concept is real — roko does run compile, test, clippy, and diff gates on agent output. The `POST /api/run` endpoint is real and does dispatch an agent to execute the prompt. But the UI doesn't connect these pieces together.

**What data it uses**: `POST /api/run` (submits the build). No polling, no gate checking, no file listing.

**Issues**: The page sends the build request but doesn't follow through. Gates are always shown as passing. The terminal doesn't show build output. The file sidebar is always empty. The prompt input and the terminal are disconnected — they're two separate, unrelated things on the same page.

---

### Page 5: Explorer (`/explorer`)

An observability page for inspecting the running server. Has 4 tabs: Health, Status, Episodes, and Events.

**Health tab**: Shows 6 metric cards: server status (ok/down), uptime (in minutes), version number, count of active plans, active agents, and active runs. Below the cards, a grid of LLM provider status cards — each shows the provider name (e.g., "anthropic", "ollama"), whether it's healthy, and its latency. This tab fetches `GET /api/health`, which returns real data from the running server.

**Status tab**: A raw key-value dump of the `GET /api/status` response. Each field from the JSON response is displayed as a key on the left and value on the right. Object values are shown as stringified JSON. There's no formatting, grouping, or explanation of what any field means.

**Episodes tab**: A searchable, filterable list of episodes — the recorded events from agent work. Each episode has a kind (agent_turn, gate_result, tool_call, plan_step), an agent name, a task name, a cost, and a timestamp. You can search by text and filter by kind. Clicking an episode expands it to show all its fields as key-value pairs. The list is capped at 200 items with no pagination. This fetches `GET /api/episodes`.

**Events tab**: A list of StateHub events — internal system events like "plan started", "gate passed", "agent dispatched". Each shows an event type badge, a truncated payload, and a timestamp. Capped at 500 items. Fetches `GET /api/statehub/events`.

All tabs have a manual Refresh button but no auto-refresh. Data goes stale the moment you load it and stays stale until you click Refresh. There's no subscription to the server's SSE or WebSocket event streams, even though the server supports both (`GET /api/events` for SSE, `GET /ws` for WebSocket).

**What it tells you about the system**: The server tracks a rich set of operational data — health, provider status, agent episodes, system events. The Explorer surfaces this data, but in a raw, unprocessed form.

**What data it uses**: `GET /api/health`, `GET /api/status`, `GET /api/episodes`, `GET /api/statehub/events`.

**Issues**: Provider cards all show "DOWN" when API keys aren't configured, which looks like the system is broken rather than unconfigured. The Status tab is a raw JSON dump. Episode detail is a flat key-value list with no hierarchy. No auto-refresh, no live streaming. No pagination — large datasets are silently truncated.

---

### Page 6: Bench (`/bench`)

A benchmark configuration and results page. The idea: configure a SWE-bench (software engineering benchmark) evaluation, run it, see results, and track learning over time.

**Layout**: A hero section with title "Benchmark Lab" and 4 summary metrics (Total Runs, Pass Rate, Total Cost, Episodes). Below that, 4 tabs: Configure, Results, Self-Learning, Compare.

**Configure tab**: Three configuration sections:
- **Test Suite** — 4 options as selectable cards: Smoke (5 tasks), SWE-lite (25 tasks), SWE-verified (300 tasks), Custom. Each has a description.
- **Agent Strategy** — 4 options: Minimal (basic agent), Context-Enriched (with context bidders), Neuro-Augmented (with knowledge store), Full Cascade (complete pipeline with replan).
- **Model** — A text input defaulting to `claude-sonnet-4-20250514`.
- A "Run Benchmark" button.

**Results tab**: Shows a status banner during/after a run (spinner when running, green on success, red on failure). Below that, 3 metric cards (Pass Rate, Total Cost, Avg Time) and a bar chart showing "Cost per Task" for the last 30 results, with bars colored green for pass and red for fail.

**Self-Learning tab**: Shows how the cascade router is distributing work across models. Displays the number of models in the router and a bar chart of their routing weights. Also has a placeholder for knowledge store stats.

**Compare tab**: Empty. Shows "Run benchmarks with different configurations to compare results here."

**How it works**: When you click Run Benchmark, the page submits `POST /api/run` with a prompt that includes the selected suite and strategy as a prefix (e.g., `[bench:smoke/full-cascade] Build a CLI calculator in Rust`). It then polls `GET /api/run/{id}/status` every 2 seconds until the run finishes. On completion, it refreshes historical efficiency data from `GET /api/learn/efficiency`.

On page load, it fetches historical efficiency data and cascade router model data to populate the charts.

**What it tells you about the system**: The system has a real run-and-poll API, real efficiency tracking, and a real cascade router with model weights. The benchmark concept is real — roko can run tasks and measure pass rate, cost, and time. The self-learning tab shows that the system records and adapts its model routing.

**What data it uses**: `POST /api/run`, `GET /api/run/{id}/status`, `GET /api/learn/efficiency`, `GET /api/learn/cascade-router`.

**Issues**: The suite/strategy selection doesn't change actual behavior — it only changes the prompt prefix string. Each "benchmark" submits a single prompt, not the 5/25/300 tasks the suite descriptions promise. Cost data from the run starts at $0 and only gets real numbers after the efficiency data refresh. The Compare tab is an empty placeholder. The Episodes metric always shows "—".

---

### Page 7: BenchLive (`/bench-live`)

A real-time benchmark observation dashboard. The most visually interesting page.

**Layout**: A header with `◆ Live Bench Monitor`, a pulsing green "live" dot, and an elapsed time counter. Below that, a metrics bar with 5 numbers: Passed %, Cost, Avg/Task, Model, and Tasks completed out of 50. The main area has 3 columns: a Task Grid (50 small colored squares), a Cumulative Cost chart (Canvas 2D line chart), and an Activity Feed (scrolling log of events).

**How it works**: The page has two data sources running simultaneously:

1. **Simulation** (always active): Every 2 seconds, the next pending task in the grid is completed. Each simulated task has a 75% chance of passing and a random cost between $0.02 and $0.17. The result is added to the task grid (square turns green or red), the cost chart extends, and a line is added to the activity feed.

2. **Real data** (attempted): Every 5 seconds, the page polls `GET /api/learn/efficiency` for actual task results and `GET /api/learn/cascade-router` for the current model name. If real data comes back, it overwrites the task grid. However, the simulation continues running in parallel, so the two data sources can conflict.

The Task Grid is a 50-cell grid where each cell represents one benchmark task. Cells start gray (pending), then turn green (pass) or red (fail) as tasks complete. The Cumulative Cost chart is a Canvas 2D line graph that draws the running total of cost as tasks complete. The Activity Feed shows the most recent 50 events with timestamps and per-task costs.

**What it tells you about the system**: The efficiency and router APIs are real. If tasks have actually been run, this page can show real data. But the simulation is always there as a fallback, making it hard to distinguish real results from fake ones.

**What data it uses**: `GET /api/learn/efficiency`, `GET /api/learn/cascade-router`, plus client-side simulation.

**Issues**: The simulation never stops, even when real data is available. The task count is hardcoded to 50 regardless of how many tasks actually exist. The activity feed only shows simulation events. There's no way to start or configure a benchmark from this page — it's observation-only, but it's unclear what you're observing unless a benchmark is actually running. The "live" indicator is always green even when nothing is happening.

---

### Shared Components

The UI has 8 reusable components:

**StatCard** — A rectangular metric display used across almost every page. Shows a large value, a small label below it, and an optional sub-text. Comes in 5 color variants matching the ROSEDUST palette: rose (pink), bone (warm tan), sage (green), fail (red-orange), warn (yellow). This is the only metric pattern in the UI — every number on every page is shown in a StatCard.

**GateBar** — A horizontal row of gate indicators. Each gate shows a status icon (checkmark for pass, X for fail, circle for pending, dash for skip) and the gate name. Only used on the Builder page, always showing 4 gates: compile, test, clippy, diff. In the actual roko system, the gate pipeline has 7 rungs with 11 possible gates, but the UI only shows these 4.

**Timeline** — A vertical list of steps with colored markers. Steps can be "done" (green dot), "active" (pulsing rose dot), or "pending" (gray dot). Each step has a label and an optional detail line. Only used on the Demo page for showing scenario progress.

**CostChart** — A Canvas 2D cumulative line chart. Draws a line showing cost over time with a shaded area underneath. Has a Y-axis with dollar amounts, horizontal grid lines, and axis labels. Uses ResizeObserver to stay responsive. Only used on the BenchLive page.

**BarChart** — A Canvas 2D horizontal bar chart. Each bar has a label on the left, a colored fill proportional to its value, and a numeric value on the right. Supports per-bar colors. Used on the Bench page for both "Cost per Task" and "Model Routing Weights."

**TerminalPane** — A wrapper around an xterm.js terminal instance. Shows a label at the top and a connection status indicator (connecting, connected, disconnected). Manages the terminal lifecycle through the `useTerminal` hook.

**TerminalGrid** — A CSS grid container for TerminalPane components. Accepts a list of sessions and a column count (1, 2, or 3). Used by the Terminal page and the Demo page.

**Layout** — The top-level shell containing the navigation bar and content outlet. Present on every page.

---

### Visual Design: ROSEDUST

The UI uses a custom design system called ROSEDUST, implemented entirely through CSS custom properties (variables). There are no CSS frameworks, no Tailwind, no component libraries.

**Color palette**: Dark and atmospheric. The background is a very deep purple-black (`#1A141A`). Card surfaces use near-invisible white overlays (`rgba(255,255,255,0.02)`) with slightly more visible borders (`rgba(255,255,255,0.06)`), creating a "glass" effect. The primary accent is a dusty rose/mauve (`#AA7088`), with a brighter pink for highlights (`#D48BA0`). Supporting colors: warm tan/bone (`#C8B890`) for neutral data, sage green (`#6A9870`) for success, burnt orange (`#C36E55`) for failure, and gold (`#D4A04A`) for warnings.

**Typography**: Three font families — `General Sans` for UI text, `Zodiak` (a serif) for page titles, and `JetBrains Mono` for code, terminals, and technical values. The overall feel is monospace-heavy.

**Character**: The design wants to feel like a control room or hacker terminal — dark, dense, technical. Everything is low-contrast and atmospheric rather than high-contrast and functional.

---

### What the UI Actually Connects To

Out of the ~115 HTTP endpoints the server exposes, the React UI uses **10**:

| What the UI does | Server endpoint |
|-----------------|----------------|
| Show health dot in nav bar | `GET /api/health` |
| Explorer: server health details | `GET /api/health` |
| Explorer: workspace status | `GET /api/status` |
| Explorer: episode log | `GET /api/episodes` |
| Explorer: event stream | `GET /api/statehub/events` |
| Builder + Bench: submit a run | `POST /api/run` |
| Bench: poll run completion | `GET /api/run/{id}/status` |
| Bench + BenchLive: load efficiency data | `GET /api/learn/efficiency` |
| Bench + BenchLive: load router models | `GET /api/learn/cascade-router` |
| Terminal: create PTY session | `POST /api/terminal/sessions` |
| Terminal: WebSocket data bridge | `WS /ws/terminal/{id}` |

That means the UI surfaces approximately **9% of the server's capabilities**. The remaining 91% — plans, PRDs, agents, jobs, research, knowledge, configuration, deployments, metrics, inference gateway, event streams, and more — has no UI representation at all.

---

## Part 2: The Server's Full API Surface

The server (`roko serve`) runs on port 6677 and exposes ~115 endpoints organized into categories. This section describes every category so you can understand the full scope of what the server can do — and how little of it the web UI currently surfaces.

### Health & Observability (23 endpoints)
The server exposes comprehensive monitoring: a simple health check (`GET /health`) for load balancers, a rich health check (`GET /api/health`) with uptime/version/provider status, a metrics suite (success rate, engagement, c-factor, velocity, coverage, model efficiency — 10 individual metric endpoints plus a Prometheus exporter), a dashboard scaffold, gate pass rates with per-rung breakdowns, episode log, signal log, truth map, retention stats, parity checks, and StateHub snapshots. There are also SSE event streams (`GET /api/events`, `GET /api/sse`) and a full-duplex WebSocket (`GET /ws`) with subscription filtering and ring-buffer replay for real-time observability.

### Plans (15 endpoints)
The complete plan lifecycle: list plans, create plans, get a plan by ID, list tasks within a plan, execute a plan, check execution status, pause/resume execution, view gate results, list task reviews, submit task reviews, view task diffs, chat with a plan's context, estimate cost/time, and generate a plan from a prompt. This is the heart of roko's execution model — plans are DAGs of tasks that agents execute, and the server manages the entire lifecycle.

### PRDs (9 endpoints)
Product requirement documents: capture ideas, list PRDs, get by slug, view coverage report, draft a PRD (agent-driven), promote from draft to published, generate an implementation plan from a PRD, and consolidate PRDs (scan for gaps and duplicates).

### Agents (23 endpoints across two subsystems)
**Managed agents** (12): Register agents, create agents from manifests, get agent details, view capability profiles, start/stop/restart agents, view episode history, proxy logs, send messages, and manage agent tokens.
**Aggregator/topology** (11): Discover agents across sidecars, view the topology graph (who talks to whom), get per-agent stats/skills/heartbeats/traces, list prediction sessions and claims, and browse the knowledge graph (entries, edges, search, kinds).

### Jobs (13 endpoints)
The job marketplace: list jobs, create jobs, get job details, update job fields, delete jobs, assign to an agent, start work, submit results, evaluate submissions (accept/reject), execute immediately, cancel, view statistics, and match jobs to agents (the matchmaking engine that ranks candidates by skill fit, tier, reputation, and load).

### Research (6 endpoints)
Research dispatch: start a background research session on a topic, list research artifacts, enhance a PRD with research context, enhance a plan, enhance tasks, and analyze execution data.

### Learning (10+ endpoints)
The self-improvement system: view efficiency data (per-task cost/tokens/latency), cost projections, per-provider success/failure rates, retry statistics, runtime feedback, cascade router state (which model handles what), cost tier breakdowns, A/B prompt experiments, adaptive gate thresholds, and executor state snapshots.

### Configuration (3 endpoints)
View the current config (with secrets masked), merge partial config updates, and hot-reload from disk.

### Providers & Models (5 endpoints)
List configured LLM providers with health status and model counts, health-check a specific provider, test a provider with a real prompt, list all configured models, and explain model routing decisions for a given context.

### Gateway (4 endpoints)
Centralized inference: send a completion request (the cascade router selects the model), view gateway stats, list available models, and submit/check batch inference jobs.

### Terminal (4 endpoints + WebSocket)
Create PTY sessions, list sessions, destroy sessions, send input to sessions, plus the WebSocket bridge for interactive use.

### Deployments (6 endpoints)
Cloud deployment: create from template, list/get/delete deployments, view logs, proxy tasks to deployed workers.

### Templates (5 endpoints)
Reusable deployment templates: list, create, get by name, delete, and deploy/run.

### Neuro/Knowledge (2 endpoints)
Query the durable knowledge store (the system's long-term memory) by topic.

### Other
Webhooks (GitHub, Slack, generic), secrets/API key management, subscriptions/connectors/feeds (event routing), projections (materialized views with SSE delta streams), heartbeats/network stats, service integrations, team management (members, invitations, roles), on-chain endpoints (agent registry, bounties, chain status), vision loop (screenshot-based agent interaction), dream consolidation (offline learning), shared run transcripts, and a full OpenAPI 3.1.0 specification.

---

## Part 3: Demo Resource Scripts (`tmp/demo-resources/`)

These are shell scripts that exercise the real CLI and HTTP API. Unlike the React UI (which is mostly simulated), these scripts make real API calls, verify real responses, and demonstrate real workflows. They serve as both validation tests and live demo material.

### Directory Structure

```
tmp/demo-resources/
  OVERVIEW.md              # Index and quick-start guide
  AUTOMATION.md            # Detailed automation documentation
  run-all.sh               # Master suite — runs everything (~30s)
  smoke-test.sh            # Fast server validation (~15s)
  bin/                     # Reusable tooling (command dispatcher, startup/shutdown, helpers)
  agent-matchmaking/       # Agent registration, skill matching, job lifecycle
  agent-setup/             # Agent creation, tool config, fleet management
  agent-workflows/         # Live agent sidecar processes
  prd-workflow/            # PRD idea → draft → plan pipeline
  research-workflow/       # Research dispatch and artifact management
  full-self-hosting/       # The complete self-hosting loop
  dashboard-quickstart/    # Setup guide for connecting a dashboard
```

---

### `run-all.sh` — The Master Integration Test

Runs 9 suites sequentially and reports a summary. Takes ~30 seconds. This is the most comprehensive validation of the system — if `run-all.sh` passes, the core product works.

**Suite 1: Smoke Test** — Calls `smoke-test.sh` (described below). Validates server health, agent registration, matchmaking, job lifecycle, PRD capture, research dispatch, and reachability of 28 API endpoints.

**Suite 2: Seed Agents** — Registers 5 canonical demo agents with the server. These agents represent different specialties: a Rust developer ("rustsmith"), an Ethereum/Solidity developer ("ethdev"), a full-stack developer ("fullstack"), a researcher, and a security auditor. Each has defined skills, a trust tier (Expert/Trusted/Verified), a reputation score, and a maximum concurrent job count. All other scripts that test matchmaking depend on these agents existing.

**Suite 3: E2E Test Suite** — 40 individually named assertions covering:
- **Registration**: Register 3 agents, verify they appear in the managed agents list and can be fetched by ID.
- **Matchmaking**: 7 different match queries testing skill filters (rust+p2p, solidity+security), tier filters (Verified, Expert, Pioneer), language filters, and broad (no filter) searches. Verifies candidate counts, ranking order (rustsmith should rank first for Rust tasks), totalFee format, and etaHours type.
- **Job lifecycle**: Create a job, assign it, start work, verify the inflight job count increments, submit results, evaluate and accept. Then a second job that gets rejected, reworked, and re-accepted.
- **Error cases**: Blank job title returns HTTP 400 with `bad_request` error code. Invalid tier "Principal" returns HTTP 400. Double-canceling a job returns HTTP 422 `unprocessable_entity`. Fetching a nonexistent job returns HTTP 404 `not_found`. Numeric reward values get coerced to strings.
- **CLI commands**: Verifies `roko job list`, `roko job create`, and `roko job match` work from the command line.

**Suite 4: PRD CLI** — Tests the PRD workflow via the command line: `roko prd idea "..."` (capture an idea), `roko prd list` (list all PRDs), `roko prd status` (show coverage report).

**Suite 5: PRD API** — Tests the same PRD workflow via HTTP: `POST /api/prds/ideas`, `GET /api/prds`, `GET /api/prds/status`, `GET /api/plans`. Uses Python to parse JSON responses and assert expected shapes.

**Suite 6: Research** — Dispatches a research topic via `POST /api/research/topic` and lists artifacts via `GET /api/research`. Verifies the dispatch returns a research ID.

**Suite 7: Fleet Setup** — Creates an agent manifest via `roko agent create`, registers it via HTTP, lists agents via both CLI and API, and verifies the agent count is > 0.

**Suite 8: Job Lifecycle** — Two scenarios. First: match agents, create a job with the top candidate, walk it through assign → start → submit → evaluate(accept), verify final state is "completed". Second: create another job, walk it through assign → start → submit → evaluate(reject) → resubmit → evaluate(accept), verifying the rejection/rework cycle works.

**Suite 9: Ollama Provider** — If the Ollama LLM provider is configured in `roko.toml`, checks that `GET /api/providers/ollama/health` reports it as healthy. Skips gracefully if Ollama isn't configured.

---

### `smoke-test.sh` — Fast Validation

A quicker version of `run-all.sh` focused on breadth over depth. Tests 6 areas in ~15 seconds:

1. **Server**: Health endpoint returns `ok`.
2. **Agents**: Registers 2 agents and verifies both succeed.
3. **Matchmaking**: A Rust-skill query returns at least 1 candidate with a `totalFee` field. A Pioneer-tier query (unreachable for test agents) returns 0 candidates.
4. **Jobs**: Walks one job through the complete state machine: open → assigned → in_progress → submitted → completed.
5. **PRDs & Research**: Captures an idea, lists PRDs, dispatches research.
6. **Endpoints**: Hits 28 GET endpoints and verifies each returns a 2xx status code. This is a broad reachability test — it doesn't check response contents, just that the server responds successfully. Endpoints checked: health, status, dashboard, managed-agents, agents, jobs, stats, plans, prds, status, research, providers, models, config, episodes, signals, metrics, gates/summary, efficiency, experiments, knowledge, tasks, subscriptions, integrations, deployments, templates, heartbeats, truth_map, parity.

---

### `agent-matchmaking/` — The Marketplace

This directory demonstrates and tests the job matchmaking system — how roko finds the right agent for a task.

**`seed-agents.sh`** (automated) — Registers 5 agents via `POST /api/agents/register`. Each agent has a unique profile:

| Agent | Skills | Tier | Reputation | Max Jobs |
|-------|--------|------|------------|----------|
| rustsmith | rust, p2p, eth, networking | Expert | 94 | 5 |
| ethdev | solidity, eth, defi, evm | Trusted | 82 | 3 |
| fullstack | typescript, react, rust, graphql | Verified | 75 | 4 |
| researcher | defi, tokenomics, governance, analysis | Expert | 91 | 6 |
| auditor | solidity, security, audit, evm | Expert | 97 | 2 |

These agents represent the "fleet" — a team of specialized AI workers available for task assignment. All other matchmaking demos and tests depend on these agents existing.

**`demo-match.sh`** (interactive, 6 pauses) — Walks through 6 matchmaking queries, pausing between each to let a presenter explain what's happening. Each query posts to `POST /api/jobs/match` with different criteria and displays a formatted table of results:

1. Rust + P2P experts (should return rustsmith first)
2. Solidity + security auditors (should return auditor and ethdev)
3. DeFi + analysis (should return researcher)
4. Broad search (no filters — should return everyone)
5. Language filter: Rust (should return rust-skilled agents)
6. Pioneer tier (should return 0 — no agents have Pioneer tier)

Each result row shows: agent ID, tier, reputation, current load, matched skills, and bid amount. This demonstrates that the system doesn't just match by keyword — it ranks candidates by relevance, reputation, and availability.

**`demo-lifecycle.sh`** (interactive, 5 pauses) — Walks through the complete job state machine from creation to completion:

1. **Match**: Find candidates for a Rust task.
2. **Create**: Create a job with the top candidates committed.
3. **Assign**: Assign the job to the top-ranked agent.
4. **Start**: The agent begins work (state: in_progress).
5. **Submit**: The agent submits results with a summary, file artifacts, and gate results (compile: pass, test: pass, clippy: pass).
6. **Evaluate**: Accept the submission (state: completed).

At each step, the script prints the job's state, assigned agent, and reward, showing the state transitions that the job goes through.

**`e2e-test.sh`** (automated, CI-grade) — The most thorough test in the entire demo-resources suite. Runs ~40 individually named assertions. Beyond what's described in the run-all.sh section above, this script specifically verifies: candidate ranking order (rustsmith first for Rust tasks), totalFee formatting as "2500 KORAI" strings, etaHours as integer types, inflight job count incrementing during active work, and the coercion of numeric reward values to strings in API responses.

---

### `agent-setup/` — Creating and Configuring Agents

**`setup-fleet.sh`** (automated) — Creates 3 agent manifests using `roko agent create` (which writes a manifest file to `.roko/agents/<name>/manifest.toml`), then registers each agent with the control plane via HTTP, and prints a fleet status table. The 3 agents: rust-dev (coding domain, rust/systems/networking skills), researcher (research domain, defi/tokenomics/analysis), auditor (coding domain, security/audit).

The README in this directory is a comprehensive reference for agent configuration. It covers:
- **Agent domains**: general, coding, research, chain — each domain implies different default tool profiles.
- **Tool profiles**: In `roko.toml`, you can define which tools each domain can use (e.g., coding agents can use file_write and shell_exec; research agents can't).
- **MCP server config**: Agents can connect to MCP (Model Context Protocol) servers for additional tool capabilities. Configured via `agent.mcp_config` in `roko.toml`.
- **Per-role model routing**: Different roles within an agent can use different LLMs (e.g., coding tasks → Claude Sonnet, code review → Claude Opus with max effort, research → Perplexity).
- **Sidecar startup**: Each agent can run as a standalone HTTP server with endpoints for health, stats, logs, tasks, and messaging.

---

### `agent-workflows/` — Live Agent Processes

These scripts go beyond registration and actually start agent processes.

**`01-single-agent.sh`** (automated) — Starts `roko serve` and one agent sidecar process (`demo-agent`). Waits for both to come up, then verifies the agent registered itself with the control plane by checking `GET /api/agents/demo-agent`. Sends a test message through the serve proxy via `POST /api/agents/demo-agent/message`. Then waits 5 seconds to check for event storm warnings (a known issue where sidecars could flood the event bus). This demonstrates the basic agent→serve registration and communication flow.

**`02-multi-agent.sh`** (automated) — Starts `roko serve` and 3 agent sidecars (alpha, beta, gamma) simultaneously, each on an auto-selected port. Waits for all to register, then verifies each appears in the registry and lists the full fleet via `GET /api/agents`. This demonstrates that multiple agents can run concurrently and self-discover.

**`03-chat-repl.sh`** (interactive) — Starts a single agent sidecar on a fixed port without `roko serve`. Verifies the sidecar's health endpoint, then launches `roko agent chat --agent demo-agent`, which opens an interactive chat REPL. You type messages, the agent (backed by a real LLM) responds. This demonstrates the simplest possible agent interaction — no control plane, no matchmaking, just direct conversation.

---

### `prd-workflow/` — The Product Development Pipeline

**`demo-prd-cli.sh`** (interactive, 4 pauses) — Demonstrates the PRD workflow through CLI commands:
1. Capture 3 ideas: "implement cascade model routing for cost optimization", "add real-time WebSocket events for plan execution", "build a code intelligence MCP server for agents".
2. List all PRDs showing their status (idea/draft/published).
3. Show the PRD status report (coverage across the system).
4. Create 2 jobs (one coding task, one research task) and list all jobs.

**`demo-prd-api.sh`** (interactive, 4 pauses) — The same workflow via HTTP API:
1. `POST /api/prds/ideas` with a text idea.
2. `GET /api/prds` to list all PRDs.
3. `GET /api/prds/status` for the coverage report.
4. `GET /api/plans` to list generated plans.

The README explains the full 5-stage pipeline that these scripts only partially cover: (1) capture idea via `roko prd idea`, (2) draft a PRD via `roko prd draft new` (agent-driven — an LLM generates the full PRD), (3) publish via `roko prd draft promote`, (4) generate plan via `roko prd plan` (an LLM turns the PRD into a `tasks.toml` DAG), (5) execute via `roko plan run`. The scripts demonstrate stages 1-2 and job creation; the full execute loop is shown in `full-self-hosting/`.

---

### `research-workflow/` — Research-Informed Development

**`demo-research.sh`** (interactive, 2 pauses) — Demonstrates research dispatch:
1. Dispatch a research topic via `POST /api/research/topic` (returns a research ID).
2. List artifacts via `GET /api/research` (research output files with titles and slugs).
3. Capture related PRD ideas based on research findings.
4. Create a research job.
5. List all jobs.

The README describes the full research system: topic research at configurable depth (shallow/deep), direct web search via Perplexity, PRD enhancement (automatically weave research citations into a PRD), plan optimization, and episode analysis. Research artifacts include relevance scores, confidence scores, identified gaps, and follow-up recommendations.

---

### `full-self-hosting/` — The Complete Loop

**`demo-full-loop.sh`** (interactive, 3 pauses) — The flagship demo. Shows the entire self-hosting cycle in 4 acts:

**Act 1 — Capture**: Capture 3 PRD ideas and list them. This represents "roko decides what to build next."

**Act 2 — Work Items**: Create 2 jobs (one coding, one research) from the ideas. This represents "roko creates actionable work."

**Act 3 — Match**: Run matchmaking queries for both jobs, showing which agents are qualified. The coding job matches against rust+systems skills; the research job matches against analysis+distributed-systems skills. This represents "roko finds the right agent for each task."

**Act 4 — Observe**: Check system health, job statistics (total/open/completed/failed), the agent fleet, and the presence and size of learning artifact files:
- `.roko/learn/efficiency.jsonl` — Per-turn cost, tokens, and latency data.
- `.roko/episodes.jsonl` — Every agent turn, gate result, and tool call.
- `.roko/learn/cascade-router.json` — Model routing weights (which LLM handles which type of task).
- `.roko/learn/gate-thresholds.json` — Adaptive gate thresholds (gates become stricter or more lenient based on historical pass rates).
- `.roko/learn/experiments.json` — A/B prompt experiment results.

This represents the feedback loop: "roko learns from every run and improves its routing, gating, and prompting."

The README describes the full 10-step loop (idea → research → draft → publish → plan → match → post → execute → learn → iterate) and maps each step to dashboard tabs for a visual walkthrough.

---

### `dashboard-quickstart/` — Connecting a Dashboard

Documentation only. Explains how to connect the Nunchi dashboard (a separate Next.js application) to `roko serve`. Covers environment configuration, Vite proxy setup, what works without the server running (chat with localStorage persistence, mock commands like `/mock`, `/mockcode`, `/mockresearch`, `/mockprd`, settings, theme), and what requires the server (real commands like `/idea`, `/draft`, `/plan`, `/run`, `/coding`, `/research`, `/job`, plus the Plans/PRDs/Jobs/Learning dashboard tabs and fleet management).

---

### `bin/` — The Operational Toolbox

**`common.sh`** — A shared shell library sourced by all other scripts. Provides: path resolution, default environment variables, logging helpers, prerequisite checks (Python, Cargo, roko binary), a pure-Python HTTP client (avoids `curl` dependency — uses `urllib`), a sandboxed JSON expression evaluator, a health-poll wait function, temporary workspace creation, background serve lifecycle management, process management, and a free-port finder.

**`roko-demo`** — The central command dispatcher. Subcommands:
- `doctor` — Check all prerequisites (Python, roko binary, repo structure, optional serve reachability).
- `build` — Run `cargo build -p roko-cli`.
- `serve` — Start `roko serve` in foreground.
- `seed-agents` — Register the 5 canonical demo agents.
- `dashboard-smoke` — The most thorough single-command validation. Registers a unique smoke agent, then runs 16 assertions: health check, dashboard scaffold, managed agent list, paginated agent list, agent registration, cache invalidation (newly registered agent appears in queries), topology graph, projection snapshot, matchmaking with the smoke agent, and a full job lifecycle (create → assign → start → submit → evaluate).
- `verify-local` — The gold standard test. Creates a temporary workspace, starts `roko serve` on a randomly chosen free port, seeds agents, warms the aggregator cache, runs `dashboard-smoke`, then tears everything down. Fully isolated — leaves no side effects.
- `run <name>` — Execute any demo script by short name (e.g., `run match`, `run lifecycle`, `run full`).
- `all` — Seed agents, then run the PRD, research, and full-loop demos sequentially.

**`roko-up.sh`** — Idempotent startup script. Checks if serve is already running; if not, initializes a workspace (`roko init`), starts serve in the background (with PID file and log file), polls health for up to 15 seconds, then seeds the 5 demo agents.

**`roko-down.sh`** — Shutdown script. Reads the PID file to stop serve, or falls back to finding the process by port number.

**`roko-smoke.sh`** — Combined smoke test: runs `dashboard-smoke` for API validation, then creates a temporary workspace and runs CLI commands (`roko init`, `roko status`, `roko doctor`) to validate the CLI path.

---

## Part 4: What the Scripts Test That the UI Doesn't

The gap between what the demo scripts exercise and what the React UI shows is enormous. This table summarizes:

| Capability | Demo Scripts | React UI |
|-----------|-------------|----------|
| **Agent registration & fleet management** | Full: register 5 agents with skills/tier/rep, list fleet, verify by ID | Not surfaced at all |
| **Matchmaking** | Full: 6 query patterns, skill/tier/language filters, candidate ranking, fee/ETA | Not surfaced at all |
| **Job lifecycle** | Full: create, assign, start, submit, evaluate, reject/rework, cancel, error cases | Not surfaced at all |
| **PRD capture & listing** | Full: idea capture, PRD list, status report, via both CLI and HTTP | Not surfaced at all |
| **Research dispatch** | Full: topic dispatch, artifact listing, PRD enhancement | Not surfaced at all |
| **Agent sidecars** | Full: start/stop processes, self-registration, proxy messaging, multi-agent discovery | Not surfaced at all |
| **Agent chat** | Interactive REPL with real LLM responses | Not surfaced at all |
| **Self-hosting loop** | Full 4-act demo with learning file verification | Demo page simulates with fake data |
| **Endpoint reachability** | 28 endpoints validated | UI uses 10 endpoints |
| **Error handling** | 40 assertions including HTTP 400/404/422 cases | No error handling anywhere |
| **Dashboard projections & topology** | Verified in `dashboard-smoke` | Not surfaced at all |
| **Provider health** | Verified in smoke tests | Explorer shows it, but all providers appear "DOWN" when unconfigured |
| **Health monitoring** | Verified in every test suite | Works (green dot + Explorer health tab) |
| **Terminal PTY** | Not tested in scripts | Works when server is running |
| **Run submission & polling** | Not tested in scripts | Bench page works |
| **Benchmark observation** | Not tested in scripts | BenchLive shows simulation + some real data |
| **Builder (prompt → build)** | Not tested in scripts | Partially works (submits but doesn't show output) |

**The demo scripts are the real product validation.** They exercise the core workflows — agent management, matchmaking, job lifecycle, PRDs, research, and the self-hosting loop — through actual API calls with verified responses. The React UI is a thin, mostly decorative layer over a small fraction of the system, with several pages that show simulated data instead of connecting to the real backend.
