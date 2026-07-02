# PRD-09 — TUI Redesign

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-cli` (`tui` submodule, replacement of existing F1–F7 layout)
**Prerequisites**: PRD-00 through PRD-08

---

## 0. Scope

The current TUI (`roko dashboard`, `crates/roko-cli/src/tui/`) is a 7-tab fixed layout (F1–F7) targeting the old prd/plan/research surface. This PRD redesigns it around the workflow subsystem — workflow library, run inspector, trigger manager, workspace switcher, plus an embedded "live performance" view inspired by DAW transport panels for actively running workflows.

The TUI uses ratatui. It runs locally (no server required) and shares the same backend types as the CLI and dashboard.

---

## 1. Core Layout

```
┌─ roko ─ workspace: nunchi-dashboard ─ providers: ✓ ─ daemon: ✓ ─ cost today: $4.12 ─┐
│                                                                                      │
│  [F1 Pulse] [F2 Workflows] [F3 Runs] [F4 Triggers] [F5 Knowledge] [F6 System]        │
│                                                                                      │
│  ┌────────────────────────────────────────────────────────────────────────────────┐  │
│  │                                                                                │  │
│  │                          (active tab content)                                  │  │
│  │                                                                                │  │
│  └────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                      │
│  ┌─ Transport ──────────────────────────────────────────────────────────────────┐   │
│  │  ⠦ doc-ingest   53%  $1.84/$10  4m 12s  [pause] [cancel] [detach]             │   │
│  │  ⏸ deploy-rc   pending human input  [respond]                                 │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
│  [?] help  [/] palette  [g] go to  [w] workspace  [c] create  [q] quit              │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

The bottom **Transport** strip is always visible: it shows up to 3 active runs with one-press controls (pause, cancel, detach, respond-to-human-input). Like a DAW transport bar — your active work is never more than a key away.

---

## 2. Tabs

### F1 Pulse — Workspace Overview

The default tab. A glance at workspace health.

```
┌─ Pulse ─────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ◉ Workspace Health                                                          │
│    daemon ✓   providers ✓   triggers 4 active   workflows 47   modules 124   │
│                                                                              │
│  ◉ Active Runs (2)                                                           │
│    ⠦ doc-ingest         wf_01HGZK..  53%  $1.84   4m elapsed                 │
│    ⏸ deploy-rc          wf_01HGZP..  human input pending                     │
│                                                                              │
│  ◉ Pending Triggers                                                          │
│    cron       weekly-research-sweep    next: in 2d 14h                       │
│    fs-watch   reingest-on-change       last: 12m ago, no changes             │
│    github     pr-comment-review        webhook ready                         │
│    slack      slack-respond            webhook ready                         │
│                                                                              │
│  ◉ Recent Completions (5)                                                    │
│    ✓ doc-ingest   wf_01HG..  12m ago  $1.84  10 PRDs / 10 plans              │
│    ✓ test-run     wf_01HG..  47m ago  $0.00  142 passed / 0 failed           │
│    ✗ deploy-rc    wf_01HG..  1h ago   $0.30  smoke-test failed               │
│    ✓ visual-gate  wf_01HG..  3h ago   $0.45  92/100                          │
│    ✓ doc-ingest   wf_01HG..  yesterday $0.20 incremental, 1 PRD updated       │
│                                                                              │
│  ◉ Cost (last 7 days): $14.62   ◉ Episodes: 312    ◉ Knowledge: 1,847 entries│
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

Press Enter on any item to drill into the relevant tab. The Pulse tab is the only one with auto-refresh (1s).

### F2 Workflows — Library + Editor

Two-pane: workflow list (left, 30%), detail+inline edit (right, 70%).

```
┌─ Workflows ────────────────────────────────────────────────────────────────┐
│ search: __________                                                          │
│ filter: [installed] [catalog] [tagged: ___________________]                 │
│                                                                             │
│ ▶ doc-ingest@1.0.0       │  doc-ingest @ 1.0.0  (builtin)                  │
│   prd-draft@1.0.0        │                                                  │
│   prd-plan@1.0.0         │  Ingest a directory of markdown into PRDs,      │
│   research-sweep@1.0.0   │  plans, and tasks.                               │
│   plan-execute@1.0.0     │                                                  │
│   visual-gate@1.0.0      │  Macros:                                         │
│   code-review@1.0.0      │   enable_audit         bool   default: true     │
│   deploy@1.0.0           │   enable_web_research  bool   default: true     │
│   smoke-test@1.0.0       │   max_refine_iter      int    default: 2        │
│   ...                    │   synthesizer_model    model  default: opus-4-7 │
│                          │   cluster_granularity  enum   default: auto     │
│   [+ New Workflow]       │   budget_usd           money  default: 5.00     │
│   [→ Marketplace]        │                                                  │
│                          │  Slots:                                          │
│                          │   researcher  optional  default: perplexity-... │
│                          │                                                  │
│                          │  Capabilities: fs.read, fs.write, llm, net      │
│                          │  Estimated cost: $0.20–$8.00 per typical run    │
│                          │                                                  │
│                          │  [r] Run    [e] Edit    [f] Fork    [v] Validate│
│                          │  [b] Benchmark  [c] Capabilities  [→] Graph view│
│                          │                                                  │
│                          │  Last 5 runs:                                    │
│                          │  ✓ 12m ago  $1.84  10 PRDs                       │
│                          │  ✓ 1d ago   $0.20  incremental, 1 PRD            │
│                          │  ✓ 3d ago   $1.12  initial 6 PRDs                │
│                          │  ...                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

Press `r` to launch with a parameter prompt overlay (filled with macros). Press `→` to enter the **Graph View** — a state-graph visualization rendered with ratatui-canvas (see §6).

### F3 Runs — Run Inspector

List of active and recent runs (left), detail (right).

```
┌─ Runs ─────────────────────────────────────────────────────────────────────┐
│ filter: [active] [recent] [failed]                                          │
│                                                                             │
│ ▶ ⠦ doc-ingest    │  wf_01HGZK7B9XVJ4P8TRYM3N8DSWE   doc-ingest@1.0.0      │
│    wf_01HGZK..    │  trigger: manual    workspace: nunchi-dashboard         │
│   ⏸ deploy-rc     │  started: 4m 12s ago    estimated total: 8m 30s         │
│    wf_01HGZP..    │  cost: $1.84 / $10.00 budget                            │
│   ✗ deploy-rc     │  capabilities used: fs.read, fs.write, llm, net         │
│    wf_01HGZN..    │                                                         │
│   ✓ test-run      │  Nodes:                                                 │
│    wf_01HGZM..    │   ✓ walk          0.1s                                  │
│   ✓ doc-ingest    │   ✓ segment       0.4s                                  │
│    wf_01HGZJ..    │   ✓ classify      12.3s   $0.31                         │
│                   │   ✓ cluster        4.1s   $0.18                         │
│                   │   ⠋ synthesize    in flight (8/12 done)        $1.20    │
│                   │   □ enrich         queued                                │
│                   │   □ audit          queued                                │
│                   │   □ refine-loop    queued                                │
│                   │   □ plan           queued                                │
│                   │   □ persist        queued                                │
│                   │   □ index          queued                                │
│                   │   □ report         queued                                │
│                   │                                                         │
│                   │  Artifacts (so far):                                    │
│                   │   art_a3f4..  cluster_summary.md     prd-cluster        │
│                   │   art_b2c8..  classifications.json   classify-output    │
│                   │   ...                                                   │
│                   │                                                         │
│                   │  Output stream:  [F]ollow  [j/k] scroll  [/] search     │
│                   │  > [synthesize] cluster 7: writing PRD for 60-mar...    │
│                   │  > [synthesize] cluster 8: parsing source segments...   │
│                   │                                                         │
│                   │  [p] pause  [c] cancel  [d] detach  [r] respond         │
│                   │  [→] graph view  [a] artifacts panel  [l] full logs     │
└─────────────────────────────────────────────────────────────────────────────┘
```

The right pane has tabbed sub-views:

- **Overview** (default) — node list + status + costs.
- **Graph** — state-graph rendered with live node statuses.
- **Artifacts** — produced artifacts with previews (markdown rendered, JSON pretty).
- **Episodes** — per-Module episodes.
- **Logs** — full event stream, searchable, filterable by level.
- **Trace** — node-by-node timing waterfall.

### F4 Triggers — Trigger Manager

```
┌─ Triggers ────────────────────────────────────────────────────────────────┐
│  search: ________  kind: [all|cron|fs-watch|github|webhook|slack|...]      │
│                                                                            │
│  enabled  name                  kind         workflow         last fired   │
│  ────────────────────────────────────────────────────────────────────────  │
│  [✓]     reingest-on-change     fs-watch     doc-ingest       12m ago      │
│  [✓]     weekly-research        cron         research-sweep   2d ago       │
│  [✓]     pr-review              github       code-review      3h ago       │
│  [✓]     ask-roko-on-slack      slack        slack-respond    1d ago       │
│  [ ]     nightly-deploy-canary  cron         canary           disabled     │
│  [✓]     deploy-on-tag          github       deploy           never        │
│                                                                            │
│  Health:                                                                   │
│   reingest-on-change   filter pass: 87%  dispatch ok: 100%  events: 47/d   │
│                                                                            │
│  [enter] inspect  [t] test-fire  [e] edit  [n] new  [d] disable  [r] remove│
└────────────────────────────────────────────────────────────────────────────┘
```

`enter` opens a trigger inspector with bind details, last 50 events with filter outcomes and dispatch run IDs (clickable into F3).

### F5 Knowledge — Knowledge Browser

The current Knowledge panel from the existing TUI, expanded:

- Entries list with type, confidence, age, decay state.
- Resonance graph (rendered with ratatui-canvas).
- Lineage walker.
- Stigmergy field view.
- Dream cycle history.

Largely the existing TUI's knowledge tab, with the addition of **knowledge bundle management** (create / edit / publish a bundle for use as Workflow context).

### F6 System — Workspace + Daemon + Providers + Costs

```
┌─ System ──────────────────────────────────────────────────────────────────┐
│                                                                            │
│  ◉ Workspace: nunchi-dashboard   /Users/will/dev/nunchi/nunchi-dashboard   │
│    schema: 1   created: 2026-04-25                                          │
│    extends: ~/.roko/workspaces/templates/web-app                           │
│    capabilities granted: fs.read fs.write net llm shell                    │
│                                                                            │
│  ◉ Daemon                                                                  │
│    status: running   pid: 47129   started: 14h ago                         │
│    triggers hosted: 4   workspaces hosted: 3                               │
│    cpu: 0.4%   mem: 86 MB                                                  │
│    [logs] [restart] [stop]                                                 │
│                                                                            │
│  ◉ Providers                                                               │
│    anthropic    ✓  $42.18 last 30d   12 routes configured                  │
│    openai       ✓  $11.42 last 30d   3 routes configured                   │
│    ollama       ✓  $0     last 30d   local                                 │
│    perplexity   ✓  $4.20  last 30d                                         │
│                                                                            │
│  ◉ Cost (last 30d) — $58.30                                                │
│    by-workflow: doc-ingest $34, code-review $12, research-sweep $8, ...    │
│    by-model:    opus $44, sonnet $11, haiku $2, perplexity $4, ...         │
│                                                                            │
│  ◉ Index                                                                   │
│    code-intel: 12,419 symbols, last build 3h ago                           │
│    knowledge:   1,847 entries, last GC yesterday                           │
│                                                                            │
│  [w] switch workspace  [d] daemon controls  [p] provider config            │
│  [c] cost detail       [b] backups          [s] settings                   │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Universal Keys

Always-active key bindings (not tab-specific):

```
?            help overlay
/            command palette (fuzzy across workflows, runs, artifacts, pages)
g <letter>   go to: gp pulse, gw workflows, gr runs, gt triggers, gk knowledge, gs system
w            workspace switcher (overlay)
c            create new (overlay: workflow, trigger, profile, workspace)
.            quick run launcher (last-used workflow, edit args, run)
:            command line (typed commands like `:run doc-ingest`)
q            quit
^c           cancel current focus action
^r           refresh
F1–F6        jump to tab
Tab / S-Tab  cycle between panes within the active tab
```

Per-tab keys are documented in `?`.

---

## 4. Command Palette

Press `/` (or `Cmd+K` on macOS terminals that pass through). Behaves like Linear/Raycast:

```
┌─ Command Palette ─────────────────────────────────────────────────────────┐
│  > deploy_                                                                 │
│  ────────────────────────────────────────────────────────────────────────  │
│  Run workflow: deploy                                            ⌘ ⏎       │
│  Run workflow: deploy-railway                                              │
│  Workflow: deploy@1.0.0    (info)                                          │
│  Trigger: deploy-on-tag    (manage)                                        │
│  Recent run: wf_01HGZN... deploy   1h ago                                  │
│  Page: System / Deploy targets                                             │
│  Action: Create deploy trigger                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

Scoped prefixes (per doc-3 §navigation): `>` for actions, `@` for entities, `#` for pages. Recents pinned at top. Shortcuts shown inline.

---

## 5. Workspace Switcher

Press `w`:

```
┌─ Switch Workspace ────────────────────────────────────────────────────────┐
│  recents:                                                                  │
│    nunchi-dashboard   /Users/will/dev/nunchi/nunchi-dashboard    active    │
│    roko               /Users/will/dev/nunchi/roko/roko                     │
│  all:                                                                      │
│    korai              /Users/will/dev/nunchi/korai                         │
│    daeji              /Users/will/dev/nunchi/daeji                         │
│  templates:                                                                │
│    web-app rust-crate research multi-agent default                         │
│  [n] new workspace  [t] from template  [enter] open                        │
└────────────────────────────────────────────────────────────────────────────┘
```

Switching is instant: the TUI reattaches to the new workspace's daemon view; the active tab stays the same but its content updates.

---

## 6. State-Graph View (`→` from Workflows or Runs)

ratatui-canvas-rendered graph, showing nodes and edges as ASCII boxes and lines, with live colors:

- **Green** — node completed.
- **Cyan flashing** — node in flight.
- **Yellow** — node queued.
- **Red** — node failed.
- **Magenta** — node awaiting human input.
- **Dimmed** — node pruned by failed conditional.

```
                ┌──────────┐
                │   walk   │ ✓ 0.1s
                └────┬─────┘
                     │
                ┌────▼─────┐
                │ segment  │ ✓ 0.4s
                └────┬─────┘
                     │
                ┌────▼─────┐
                │ classify │ ✓ 12.3s  $0.31
                └────┬─────┘
                     │
                ┌────▼─────┐
                │  cluster │ ✓ 4.1s   $0.18
                └────┬─────┘
                     │ FanOut(12)
        ┌────────────┼────────────┐
        │            │            │
   ┌────▼────┐  ┌────▼────┐  ┌────▼────┐
   │synth #1 │  │synth #2 │  │synth #7 │   (8 of 12 in flight)
   │ ✓ 18s   │  │ ✓ 22s   │  │ ⠋ 12s   │
   └────┬────┘  └────┬────┘  └────┬────┘
        │            │            │
        ⋮            ⋮            ⋮
```

Arrow keys move focus between nodes; `enter` drills into the node (its module, params, output, episode). `e` toggles edge labels showing conditions and mappings.

---

## 7. Visual Wizard (`c` key from Workflows)

The TUI's authoring path is a **wizard** that emits Workflow TOML. The full visual / drag-and-drop authoring lives in the dashboard (PRD-11). The TUI version is keyboard-first:

```
┌─ New Workflow ────────────────────────────────────────────────────────────┐
│ name:     ____________________                                             │
│ template: ( ) blank ( ) doc-ingest ( ) deploy ( ) custom                   │
│                                                                            │
│ Modules to include (toggle):                                               │
│  [✓] fs-walk          [✓] markdown-classify    [✓] doc-cluster             │
│  [✓] prd-synthesize   [ ] web-enrich          [✓] prd-audit                │
│  [ ] citation-check   [✓] prd-plan            [✓] artifact-persist         │
│                                                                            │
│ Macros to expose:                                                          │
│  [+ add macro]                                                             │
│                                                                            │
│ Slots:                                                                     │
│  [+ add slot]                                                              │
│                                                                            │
│ Output preview (live TOML):                                                │
│  [workflow]                                                                │
│   name = "..."                                                              │
│   ...                                                                      │
│                                                                            │
│  [s] save  [v] validate  [e] edit raw TOML  [→] graph view                 │
└────────────────────────────────────────────────────────────────────────────┘
```

The wizard is the rapid-iteration mode; the full DAW-style canvas is in the dashboard.

---

## 8. Performance / Live Mode

For ongoing runs, a **Performance Mode** can be invoked (`p` from F1 or F3) that takes over the whole screen with a maximalist live view:

```
╔══════════════════════════════════════════════════════════════════════════╗
║                            doc-ingest                                    ║
║                                                                          ║
║   ╔══════════════════════════════════════════════════════════════════╗   ║
║   ║                                                                  ║   ║
║   ║                     [GRAPH ANIMATION]                            ║   ║
║   ║                                                                  ║   ║
║   ╚══════════════════════════════════════════════════════════════════╝   ║
║                                                                          ║
║   Cost  ████████████░░░░░░░░░░  $1.84 / $10.00                            ║
║   Time  ███████████████░░░░░░░  4m 12s / 8m 30s                           ║
║   PRDs  ████░░░░░░░░░░░░░░░░░░  2 / 12 expected                           ║
║                                                                          ║
║   ⠦ synthesize cluster 7  …writing PRD §3.2 — Information Architecture   ║
║                                                                          ║
║   [esc] back   [c] cancel   [d] detach                                   ║
╚══════════════════════════════════════════════════════════════════════════╝
```

This is the "watch the band play" view — useful for long-running expensive runs where the user wants ambient situational awareness.

---

## 9. Live File Watcher (existing) Integration

The existing TUI's `notify::RecommendedWatcher` (`tui/fs_watch.rs`) is repurposed: it watches `<workspace>/.roko/runs/`, `<workspace>/.roko/episodes.jsonl`, `<workspace>/.roko/artifacts/`, and triggers TUI re-render when those change. The TUI is therefore reactive to the daemon's progress without polling.

---

## 10. Theming & Style

Same ROSEDUST palette as the dashboard (per doc-3): rose accent on active elements, jade for success, amber for warning, crimson for error, violet for knowledge, sapphire for active modules. Monospace throughout (the terminal). Glass morphism is a no-op in TUI — borders stand in.

User-toggleable density (compact / comfortable / spacious) via Settings, persisted in `~/.roko/config.toml`.

---

## 11. Acceptance Criteria

| Criterion | Verification |
|---|---|
| TUI launches via `roko tui` and renders the F1 Pulse tab. | Smoke test. |
| F1 auto-refreshes every 1s without flicker. | Visual / load test. |
| F3 streams live run output without buffering > 100ms. | Latency test. |
| Workspace switcher (`w`) changes the underlying workspace; tab content updates. | Multi-workspace test. |
| Command palette (`/`) finds workflows, runs, artifacts, pages with frecency ranking. | Search test. |
| State-graph view renders for an active run with live node colors. | Visual snapshot. |
| Visual wizard for new workflow emits valid TOML that passes `roko workflow validate`. | Round-trip test. |
| TUI reacts within 200ms to file-watcher-triggered changes in `.roko/`. | Latency test. |
| Performance Mode renders without overflow on 80×24 terminals. | Min-size test. |
| All keys documented in `?` overlay; nothing undocumented. | Keyboard-spec invariant. |

---

## 12. Open Questions

- Should the TUI support remote workspaces (SSH-tunneled, or daemon-on-another-machine)? Useful for monitoring deployed workers; defer to v1.1.
- Should there be a "split mode" with two tabs visible side by side? Probably yes for the Runs + Logs combo; ship in v1.
- Should typing in the palette show inline previews (e.g., hovering a workflow shows its description in the bottom strip)? Yes.
- Should the TUI expose a "scripted" mode (`roko tui --script <path>`) that runs a key sequence for screencasts or onboarding tours? Out of scope for v1.
