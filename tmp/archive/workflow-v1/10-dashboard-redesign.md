# PRD-10 — Dashboard Redesign

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Surface**: nunchi-dashboard (React + Vite) + `roko-serve` HTTP/SSE/WebSocket
**Prerequisites**: PRD-00 through PRD-08

---

## 0. Scope

This PRD specifies the workflow-related surface area in the redesigned Nunchi dashboard. It builds on the visual language from doc-3 (the optimal redesign spec) but focuses specifically on the workflow subsystem: how users discover, configure, run, observe, fork, and publish Workflows; how Triggers and Workspaces are managed; and how the dashboard renders live runs.

The visual config wizard (the drag-and-drop "video-game" UX) gets its own PRD (PRD-11). This document covers the read-and-operate surface.

---

## 1. Information Architecture

The dashboard's six-destination navigation from doc-3 is augmented with a new **Work** destination focused on workflows. The destinations:

| Destination | Pages | Workflow-related |
|---|---|---|
| **Pulse** | Dashboard, Event Stream, Network Pulse | Yes — pulse shows active runs |
| **Agents** | Fleet, Detail, Create, Templates, Groups, Network | Tangential |
| **Work** | **Library, Editor, Runs, Triggers, Marketplace** | Primary |
| **Knowledge** | Store, Resonance, Lineage, Stigmergy, Dreams | Yes — knowledge bundles feed workflows |
| **Arena** | Browser, Leaderboard, Benchmarks, Experiments, Bounties | Tangential |
| **System** | Workspaces, Providers, Costs, Deployments, Settings | Yes — Workspaces is here |

The current "Forge" + sidebar PRD/Plans/Research/Execution/Replay collapses into **Work** with a workflow-centric model.

---

## 2. Topbar — Workspace Switcher

Per doc-3, the topbar gets a workspace switcher dropdown (left of the search button):

```
[ 🅦 nunchi-dashboard ▾ ]   ⌘K   🔔   block 1.2M   LOCAL DEV   LIVE   CRISIS    @wpank
```

Click reveals:

```
┌─ Switch workspace ─────────────────────────────────┐
│  recent                                             │
│  ◉ nunchi-dashboard   active                        │
│    /Users/will/dev/nunchi/nunchi-dashboard          │
│    47 workflows  3 active runs  $4.12 today        │
│  ○ roko                                             │
│    /Users/will/dev/nunchi/roko/roko                 │
│    62 workflows  0 active runs  $1.20 today        │
│  ○ korai                                            │
│                                                     │
│  templates  web-app  rust-crate  research  ...      │
│  [+ New workspace]    [browse all]                  │
└─────────────────────────────────────────────────────┘
```

Switching is instant. The active workspace's data context drives every page.

---

## 3. Pulse — Workflow Visibility on Dashboard

The redesigned Pulse dashboard from doc-3 §Pulse already has an "Active Work" section. Augmented with workflow specifics:

- **Active Runs strip**: top-of-page horizontal strip showing 0–3 active runs as live cards with state-graph thumbnails, cost / time progress, and a one-tap "open" link.
- **Pending Human Input badge**: red dot in the topbar when any run awaits human input; click jumps to the prompt.
- **Recent completions**: 5–10 most recent finished runs with status, cost, key output (e.g., "10 PRDs created"). Hover shows graph thumbnail; click opens the run inspector.
- **Trigger Health**: a compact bar of trigger types with health dots (green = firing as expected, amber = no fires in expected window, red = errored). Click opens Trigger Manager.

---

## 4. Work — Library

URL: `/work/library`

The DAW-instrument-rack equivalent: browse Workflows by category, install/fork from marketplace, run with parameters.

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Work / Library                                                           │
│                                                                           │
│  Search: [_______________]  Filter: [installed][marketplace][custom][...]│
│                                                                           │
│  Categories                  │  Cards (selected category: All)            │
│  ─────────────              │  ─────────────────────────────────────────  │
│  All                  47    │   ┌──────────────┐ ┌──────────────┐         │
│  ▸ Authoring           8    │   │ doc-ingest   │ │ deploy       │         │
│  ▸ Verification       11    │   │ ████████░░   │ │ ███░░░░░░░   │         │
│  ▸ Research            5    │   │ 47 runs      │ │ 12 runs      │         │
│  ▸ Execution           6    │   │ $34 last 30d │ │ $4 last 30d  │         │
│  ▸ Deploy             10    │   │ ⏱ 8m typical │ │ ⏱ 5m typical │         │
│  ▸ Operations          7    │   │ [Run] [Edit] │ │ [Run] [Edit] │         │
│  ▸ Maintenance         5    │   └──────────────┘ └──────────────┘         │
│  ▸ Code Intelligence   5    │                                              │
│  ▸ Knowledge           7    │   ┌──────────────┐ ┌──────────────┐         │
│  ▸ Observation         6    │   │ research-... │ │ visual-gate  │         │
│  ▸ Communication       5    │   │ ...          │ │ ...          │         │
│  ▸ Workflow Meta       7    │   └──────────────┘ └──────────────┘         │
│                              │                                              │
│  [+ New Workflow]            │   ... more cards ...                         │
│  [→ Marketplace]             │                                              │
│  [→ My Forks]                │                                              │
└──────────────────────────────────────────────────────────────────────────┘
```

Workflow cards show:
- Name and version.
- Health dot (green / amber / red based on recent run success rate).
- Macros count + slot count.
- Recent run count and cost.
- Estimated typical run time.
- "Run" CTA opens a parameter overlay; "Edit" opens the visual config wizard (PRD-11).

Right-click context menu: Run, Edit, Fork, Publish, Benchmark, Capabilities, Remove.

---

## 5. Work — Editor

URL: `/work/editor/<workflow-name>`

Two views over the same Workflow, toggleable per doc-3 §Composability:

### 5.1 Recipe View (default)

Linear, Apple-Shortcuts-like step list. Beginners use this:

```
┌─ doc-ingest@1.0.0 ────────────────────────────────────────────────────────┐
│  Recipe ▮  Graph                                          [Save] [Run]   │
│                                                                           │
│  Macros        ○ enable_audit              [✓]                            │
│                ○ enable_web_research       [✓]                            │
│                ○ max_refine_iterations     [2 ◀▶]                         │
│                ○ synthesizer_model         [opus-4-7 ▾]                   │
│                ○ cluster_granularity       [auto ▾]                       │
│                ○ budget_usd                [$5.00 ▮▮▮▮░░░░░░]              │
│                                                                           │
│  Slots         ○ researcher                [perplexity-search@^1 ▾]       │
│                                                                           │
│  Steps                                                                    │
│  1. Walk source dir                                                       │
│     fs-walk                                                               │
│  2. Segment markdown by heading                                           │
│     markdown-segment                                                      │
│  3. Classify segments                                                     │
│     markdown-classify    role: scribe                                     │
│  4. Cluster into PRD candidates                                           │
│     doc-cluster          granularity: auto                                │
│  5. Synthesize PRDs (parallel per cluster)                                │
│     prd-synthesize       role: strategist                                 │
│  6. Web research enrichment   ⚙ if enable_web_research                    │
│     {{ slot.researcher }}                                                 │
│  7. Audit findings                                                        │
│     prd-audit                                                             │
│  8. Refine loop  ↻  until findings clear, max {{ macros.max_refine_iter }}│
│     prd-synthesize                                                        │
│  9. Generate plans                                                        │
│     prd-plan                                                              │
│ 10. Persist artifacts                                                     │
│     artifact-persist                                                      │
│ 11. Index to knowledge                                                    │
│     knowledge-ingest                                                      │
│ 12. Produce report                                                        │
│     run-report                                                            │
│                                                                           │
│  [Validate]  [Run]  [Fork]  [Publish]                                     │
└──────────────────────────────────────────────────────────────────────────┘
```

Steps are reorderable via drag (if not locked by data dependencies). Each step has an inline expand for params. Conditional steps show a small ⚙ icon with the condition explained in plain language.

### 5.2 Graph View

Full state graph, drag-and-drop authoring. Specified in PRD-11.

---

## 6. Work — Runs

URL: `/work/runs`

Three columns:

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Work / Runs                                                              │
│                                                                           │
│  Filter: [active][recent][failed][by workflow ▾][by trigger ▾]            │
│                                                                           │
│  Run List               │  Run Detail              │  Inspector            │
│  ─────────────         │  ────────────────────     │  ─────────────────    │
│  ⠦ doc-ingest          │  wf_01HGZK7B...           │  Selected node:       │
│    wf_01HG..  53%      │  doc-ingest@1.0.0         │   synthesize-c-7      │
│  ⏸ deploy-rc           │  trigger: manual          │                       │
│    wf_01HG..  paused   │  started 4m 12s ago       │  Module:              │
│  ✗ deploy-rc           │  $1.84 / $10.00 budget    │   prd-synthesize@1.0.0│
│    wf_01HG..  failed   │                            │                       │
│  ✓ test-run            │  [Pause][Cancel][Detach]  │  Status: running      │
│    wf_01HG..  passed   │                            │  Started: 12s ago     │
│  ✓ doc-ingest          │  ┌─ State Graph ─────┐    │                       │
│    wf_01HG..  passed   │  │                   │    │  Input (truncated):   │
│                         │  │     [graph viz]   │    │  {                    │
│  ─ load more ─          │  │                   │    │   "cluster": {...},   │
│                         │  └───────────────────┘    │   "context_bundle":...│
│                         │                            │  }                    │
│                         │  ┌─ Nodes ───────────┐    │                       │
│                         │  │  ✓ walk     0.1s  │    │  Output: pending      │
│                         │  │  ✓ segment  0.4s  │    │                       │
│                         │  │  ✓ classify 12s   │    │  Tokens: 4,231 in     │
│                         │  │  ⠋ synthesize  ←  │    │  Cost: $0.18          │
│                         │  │  □ enrich         │    │                       │
│                         │  │  ...              │    │  Episode artifact:    │
│                         │  └───────────────────┘    │   ep_01HGZK7B...      │
│                         │                            │                       │
│                         │  ┌─ Logs ────────────┐    │                       │
│                         │  │  > synthesize ... │    │                       │
│                         │  │  > synthesize ... │    │                       │
│                         │  └───────────────────┘    │                       │
│                         │                            │                       │
│                         │  Tabs: [Graph][Artifacts] │                       │
│                         │  [Episodes][Logs][Trace]  │                       │
└──────────────────────────────────────────────────────────────────────────┘
```

Live updates over WebSocket. Selecting a node in the graph (or the node list) populates the right inspector with full module-level detail.

The **Artifacts tab** previews artifacts: markdown rendered with monospace, JSON pretty-printed, images displayed, diffs syntax-highlighted. Each artifact has a "Download" and "Lineage" button.

The **Trace tab** is a Gantt waterfall showing every node's duration, queue time, and retries.

---

## 7. Work — Triggers

URL: `/work/triggers`

A two-pane layout: trigger list + trigger detail.

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Work / Triggers                                            [+ New Trigger]│
│                                                                           │
│  Filter: [enabled][by kind][by workflow]                                  │
│                                                                           │
│  reingest-on-change   fs-watch  →  doc-ingest  47/d   100%   3s ago      │
│  weekly-research      cron      →  research-sweep  1/w  100%  2d ago    │
│  pr-review            github    →  code-review   3/d   95%   3h ago     │
│  ask-roko-on-slack    slack     →  slack-respond 8/d   100%  1d ago     │
│  nightly-canary       cron      →  canary       disabled                 │
│                                                                           │
│  ─ trigger detail ─                                                       │
│  reingest-on-change                                                       │
│  Kind: folder-watch     Path: tmp/ux-refresh    Recursive: yes            │
│  Patterns: **/*.md     Debounce: 30s     Concurrency: queue (depth 16)    │
│                                                                           │
│  Binding:                                                                 │
│    workflow = doc-ingest @ ^1                                             │
│    input.source_dir = "tmp/ux-refresh"                                    │
│    input.incremental = true                                               │
│    macros.enable_web_research = false                                     │
│                                                                           │
│  Health:                                                                  │
│    fired today: 47    filtered out: 6    dispatched: 41    success: 41    │
│    last fired: 3s ago    last dispatched: wf_01HGZK7B...                  │
│                                                                           │
│  Recent events:                                                           │
│    14:23:01  modified  tmp/ux-refresh/40-pages.md           dispatched   │
│    14:22:50  created   tmp/ux-refresh/40-pages/05-arena.md  filtered     │
│    ...                                                                    │
│                                                                           │
│  [Test Fire]  [Edit]  [Disable]  [Remove]                                │
└──────────────────────────────────────────────────────────────────────────┘
```

`+ New Trigger` opens a guided wizard (kind → source config → workflow + binding → policy → review). For advanced users, a "raw TOML" toggle reveals the underlying TOML for direct editing.

---

## 8. Work — Marketplace

URL: `/work/marketplace`

Per PRD-12. Browse, preview, install, fork community Workflows / Modules / Triggers. The marketplace tab carries:

- Faceted search (kind, category, tag, capability requirements).
- Featured (editorial picks).
- Trending (install velocity).
- Recently updated.
- Verified Run badge filter.
- Per-artifact page with description, capabilities, macros, slots, install count, fork count, comments, version history, capability disclosure.
- "Preview against sample input" — runs the artifact in a sandbox with provided fixture, returns within ~30s.
- "Fork & Edit" / "Install" buttons.

---

## 9. System — Workspaces

URL: `/system/workspaces`

Manage all registered workspaces.

```
┌──────────────────────────────────────────────────────────────────────────┐
│  System / Workspaces                                  [+ New Workspace]  │
│                                                                           │
│  active: nunchi-dashboard                                                 │
│                                                                           │
│  Name                  Path                          Last Opened    Tags  │
│  ─────────────────────────────────────────────────────────────────────── │
│  ◉ nunchi-dashboard   /Users/.../nunchi-dashboard   active          web   │
│  ○ roko               /Users/.../roko/roko          12m ago         rust  │
│  ○ korai              /Users/.../korai              2h ago          chain │
│  ○ daeji              /Users/.../daeji              1d ago          defi  │
│                                                                           │
│  Templates                                                                │
│   default  web-app  rust-crate  research  multi-agent                    │
│                                                                           │
│  ─ workspace detail ─                                                     │
│  nunchi-dashboard                                                         │
│  schema_version: 1   created: 2026-04-25                                  │
│  extends: ~/.roko/workspaces/templates/web-app                            │
│                                                                           │
│  Capabilities                                                             │
│   fs.read  fs.write  net  shell  llm                  chain.write off    │
│                                                                           │
│  Models                                                                   │
│   strategist: claude-opus-4-7    researcher: claude-sonnet-4-6           │
│                                                                           │
│  Deploy targets                                                           │
│   railway (default)   fly-staging   vercel-prod                           │
│                                                                           │
│  Knowledge sharing                                                        │
│   share_with: tag:nunchi    import_from: roko                             │
│                                                                           │
│  [Edit]  [Open in TUI]  [Open in Terminal]  [Export]  [Remove]            │
└──────────────────────────────────────────────────────────────────────────┘
```

`Edit` opens a form view of `workspace.toml` with explanations per field; "raw TOML" toggle for direct editing. `Export` downloads a transferable bundle.

---

## 10. Real-Time Plumbing

The dashboard receives:

- **WebSocket `/ws/events`** — every `WorkflowEvent` (per PRD-02 §8) for the active workspace, with run-id filters.
- **WebSocket `/ws/runs/<run-id>`** — focused stream for one run (used by run inspector).
- **SSE `/sse/triggers`** — trigger fire and dispatch events.
- **SSE `/sse/cost`** — live cost ticks for the active workspace.
- **HTTP `/api/v1/workflows`, `/runs`, `/triggers`, `/workspaces`, `/artifacts`, `/episodes`, ...** — REST for queries.

Every page is fully reactive: the run list updates without refresh; the graph view animates state transitions; the cost gauge ticks live.

---

## 11. Visual Style (per doc-3)

Glass morphism on every panel (3 levels). Spring-physics motion (Framer Motion, stiffness/damping per tier). Rose accent on actives. Monospace data. Tabular nums. Animated number ticks. Breathing pulses on live indicators. Stagger-children at 40ms on list mounts.

Workflow-specific visual elements:
- **State graph node colors** match the TUI scheme (jade/cyan/amber/crimson/violet).
- **Edge animations**: traversed edges flash rose during the transition; pruned edges fade to ghost.
- **Macro sliders / knobs**: render as small DAW-style controls (rotary knobs for floats, toggles for bools, segmented controls for enums).
- **Run cost gauge**: arc gauge with sparkline trailing, ticking with each charge.
- **Trigger fire indicator**: per-trigger pulse animation when an event arrives, even if filtered out.

---

## 12. Authoring Surfaces (Brief)

The dashboard is the primary authoring surface:

- **Workflow Editor** (recipe + graph) — PRD-11 details the graph view authoring.
- **Module Authoring** (TOML + impl-tier picker) — for users authoring Modules in any of the four tiers.
- **Trigger Authoring** — wizard + raw TOML.
- **Profile Authoring** (visual-gate2) — same engine.
- **Workspace Template Authoring** — save current workspace as template, share to marketplace.

All authoring carries live validation: schema errors, capability mismatches, type-incompatible wiring all surface as inline warnings before save.

---

## 13. Replacing Existing Pages

The current dashboard has Forge / PRDs / Plans / Research / Execution / Replay / Knowledge / Arena / Measurements / Treasury. After this redesign:

| Current | New | Notes |
|---|---|---|
| Forge / PRDs (kanban) | Work / Library + filter "produces=prd" | Kanban view available as a Recipe-view variant |
| Forge / Plans | Work / Library + filter "produces=plan" | Plan execution merges into Runs |
| Forge / Research | Work / Library + filter "category=research" | The "research launcher" is now the run-with-params overlay for `research-sweep` |
| Forge / Execution | Work / Runs | Generalized to all runs |
| Forge / Replay | Work / Runs / Replay action | Replay becomes a per-run action |
| Knowledge | unchanged (Knowledge destination) | Knowledge bundles surface in Workflow editor |
| Arena | unchanged (Arena destination) | Benchmarks become a workflow category |
| Measurements / Evals | merged into Work / Library "category=verification" | Eval Profiles are Workflows |
| Treasury | System / Treasury | unchanged |

The result: a more uniform, learnable, composable surface. One mental model: workflows.

---

## 14. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Topbar workspace switcher lists all registered workspaces and switches data context on click. | Multi-workspace test. |
| Work / Library renders all installed Workflows with categories matching PRD-06. | Snapshot test. |
| Run inspector receives WebSocket updates and re-renders within 200ms of event emission. | Latency test. |
| Recipe view of `doc-ingest` renders all 12 steps with correct conditional indicators. | Visual regression. |
| Trigger Manager creates a new trigger via wizard; daemon picks it up within 5s. | End-to-end test. |
| Workspace settings page round-trips workspace.toml without semantic loss. | Round-trip test. |
| Marketplace install flow shows capability disclosure before install. | Disclosure test. |
| State-graph node colors update live as the run progresses. | Visual regression with animation. |
| Cost gauge animates value-tick on each charge. | Visual test. |
| Old URLs (`/app/forge/prds`, `/app/forge/plans`, etc.) redirect to the new pages. | Redirect test. |

---

## 15. Open Questions

- Should the dashboard support multi-pane workspaces (split runs side-by-side, like Bloomberg Terminal)? Per doc-3's IA recommendations, yes — savable layouts are a power-user feature; ship in v1.1.
- Should the macro controls allow per-run overrides without editing the Workflow? Yes — the run-with-params overlay already does this; macros are tunable per-run.
- Should there be a "workspace dashboard customization" (drag panels, save layouts)? Yes; aligns with doc-3 §Workspaces. Ship in v1.1.
- How do we render large state graphs (50+ nodes)? Auto-cluster sub-workflow internals; expand on click; minimap in corner.
- Should the dashboard support multiplayer (multiple users editing the same Workflow concurrently)? Out of scope for v1; revisit when a team feature emerges.
