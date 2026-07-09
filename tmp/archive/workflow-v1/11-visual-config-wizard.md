# PRD-11 — Visual Config Wizard ("Video-Game" Authoring)

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Surface**: nunchi-dashboard (React + Vite, Web Audio API, Framer Motion, react-flow)
**Prerequisites**: PRD-00 through PRD-10

---

## 0. Scope

This PRD specifies the dashboard's drag-and-drop visual authoring environment for Workflows, Modules (Composition tier), Triggers, and Profiles. It is the "video-game" authoring UX — the Ableton/Bitwig analog where users patch primitives together visually, promote macros, fill slots, and preview live, with sound, motion, and direct manipulation as first-class citizens.

The visual editor is the dashboard's flagship authoring surface. The TUI's wizard (PRD-09 §7) is the keyboard-first companion; the editor here is the mouse-and-keyboard expressive surface.

Design lineage: Ableton Live Session view + Bitwig Grid + n8n + TouchDesigner + Figma + Apple Shortcuts. The synthesis from `uxresearch.md` Topics 3 (DAW composability) and 4 (creativity-inducing tools) directly informs this design.

---

## 1. Layout

```
┌──────────────────────────────────────────────────────────────────────────┐
│ [≡] doc-ingest@1.0.0                                    [Save] [Run] [Pub]│
├──────────────────────────────────────────────────────────────────────────┤
│  Palette  │             Canvas               │  Inspector                 │
│  ──────── │  ──────────────────────────────  │  ──────────────────────    │
│           │                                   │                            │
│  Modules  │     ┌──────┐                      │  Selected:                 │
│   ▸ Auth  │     │ walk │                      │   prd-synthesize           │
│   ▸ Verif │     └──┬───┘                      │                            │
│   ▸ Resch │        │                          │  Module: prd-synthesize    │
│   ▸ Exec  │     ┌──▼───────┐                  │  Version: ^1               │
│   ▸ Deplo │     │ segment  │                  │  Capabilities: llm         │
│   ▸ ...   │     └──┬───────┘                  │                            │
│           │        │                          │  Params:                   │
│  Filters  │     ┌──▼───────┐                  │   role:    strategist      │
│  Slots    │     │ classify │                  │   model:   {{ macro.synth..│
│  Loops    │     └──┬───────┘                  │   max_tok: 8000           │
│  Branches │        │                          │   temp:    0.4            │
│  Human    │     ┌──▼───────┐                  │                            │
│           │     │  cluster │                  │  Connections:              │
│  Macros   │     └──┬───────┘                  │   in:  cluster.output      │
│   [+]     │        │                          │   out: enrich.input,       │
│           │     ◉ ── FanOut over clusters     │        audit.input         │
│  Slots    │        │                          │                            │
│   [+]     │     ┌──▼─────────┐                │  Parallelism: per-fan-out  │
│           │     │ synthesize │ ←  selected    │                            │
│  Triggers │     └──┬─────────┘                │  Estimated cost: $0.18     │
│   [+]     │        │                          │  Estimated time: 18s       │
│           │     ┌──▼─────┐                    │                            │
│           │     │ enrich │  ⚙ if web_research │  [Promote to macro]        │
│           │     └──┬─────┘                    │  [Test isolated]           │
│           │        │                          │  [View source TOML]        │
│           │     ┌──▼─────┐                    │                            │
│           │     │ audit  │                    │                            │
│           │     └──┬─────┘                    │                            │
│           │        ↻ refine until clear      │                            │
│           │        │                          │                            │
│           │     ┌──▼─────┐                    │                            │
│           │     │  plan  │                    │                            │
│           │     └──┬─────┘                    │                            │
│           │     ◉ ── FanIn (concat)           │                            │
│           │        │                          │                            │
│           │     ┌──▼──────┐                   │                            │
│           │     │ persist │                   │                            │
│           │     └─────────┘                   │                            │
│           │                                   │                            │
│  Sound ♪  │  ▣ Recipe view  ▤ Graph view     │                            │
│  Density  │  Zoom: 100%    Auto-layout: on   │                            │
└──────────────────────────────────────────────────────────────────────────┘
```

Three columns:
- **Palette** (left, ~240px): draggable Modules, Slots, Loops, Branches, Human-input nodes; Macros & Slots & Triggers list for the current Workflow.
- **Canvas** (center): the state graph — the playground.
- **Inspector** (right, ~360px, collapsible): properties of the selected node / edge / macro / slot.

Bottom strip: view toggle (Recipe / Graph), zoom, auto-layout toggle, sound toggle, density.

---

## 2. The Canvas

### 2.1 Nodes as Cards

Each node is a card with:
- Header strip with module name + version + status badge.
- Input port row (top edge, one port per typed input).
- Body: collapsed compact info (params summary), expandable to show full param list.
- Output port row (bottom edge, one port per typed output).
- Footer with cost / time estimates and capability badges.
- Quick actions on hover: pin, fork-this-step, replace-module, mute (skip during run), solo (run only this).

Node colors carry meaning:
- **Sapphire** — Module nodes.
- **Violet** — Sub-Workflow nodes.
- **Rose** — HumanInput / Wait nodes.
- **Amber** — Branch / FanOut / FanIn / Loop control nodes.
- **Slate** — Slot placeholders.
- **Glass** — pruned-by-condition (visible but ghosted).

### 2.2 Edges as Cables

Edges are cables. Per Bitwig: **one cable type, polyphonic by default, with adapters at type-mismatch boundaries** (research §Topic 3). Cable colors encode payload type:

- **Rose**: doc/markdown.
- **Jade**: code/diff.
- **Sapphire**: structured data (JSON object).
- **Amber**: evidence (visual-gate2).
- **Violet**: knowledge engram.
- **White**: generic.

Cable thickness encodes batch size. A FanOut edge fans into a thicker cable; downstream nodes are visibly fed batches.

Cable behavior:
- Hover a cable: shows last value preview in a tooltip (live, sampled at last execution).
- Click an in-flight cable during a run: opens a stream viewer of values flowing through.
- Drag from output port to input port: creates a cable; if types match, instant; if not, prompts for an adapter from the registry (auto-selects unambiguous adapters; lets user choose otherwise).

### 2.3 Conditions on Edges

A diamond marker on an edge indicates a condition. Click to edit. Plain-language editor with a fall-through to the Expr language:

```
when:  enable_web_research is true   AND   classify.confidence > 0.8
```

Color-coded: green for "expression valid"; amber for "valid but never true given current macros"; red for "unparseable".

### 2.4 Slots Visualized

A Slot is a card with a dotted border, labeled with the accepted type and optionally a default filling. Drag any compatible Module from the palette onto the slot to fill it. Right-click a filled slot to revert to default or change.

```
┌╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴┐
│ slot: researcher │
│ accepts: web-r…  │
│ default:         │
│  perplexity-…    │
└╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴┘
```

### 2.5 Loops & Branches

A `Loop` node renders as a rectangle wrapping its body subgraph, with a curved arrow returning to the body's entry. The `until` predicate is shown on the return arrow. Click to set max_iterations and edit predicate.

A `Branch` node has multiple output ports (one per outgoing edge); each port carries its own condition.

### 2.6 Sub-Workflows

A `SubWorkflow` node renders compact in the parent canvas. Double-click to descend into the sub-workflow's own canvas (breadcrumb shows the nesting). Editing the sub-workflow updates it everywhere it's used.

For shared sub-workflows (used in 5+ parents), edits prompt: "This sub-workflow is used in 5 places. Edit affects all. Continue / fork copy?"

---

## 3. Macro Promotion (DAW Rack Macros)

The signature gesture: take an internal Module's parameter and promote it to a Workflow-level Macro.

```
1. Right-click a parameter in the Inspector → "Promote to macro"
2. Inline modal:
   ┌──────────────────────────────────┐
   │  Promote temperature to macro    │
   │  ────────────────────────────    │
   │  Macro name:    temperature     │
   │  Label:         Synth temp      │
   │  Description:   ...             │
   │  Kind:          Float           │
   │  Min:  0.0    Max: 1.0          │
   │  Default: 0.4                   │
   │  Bind also to: [+ add binding]  │
   │  ────────────────────────────    │
   │  [Cancel]  [Promote]            │
   └──────────────────────────────────┘
3. Resulting in:
   - A new MacroDef in workflow.macro
   - The original param value replaced with `{{ macro.temperature }}`
   - The Macro added to the palette's Macros list
```

A single macro can bind to multiple internal params (the "Bind also to" pulls additional targets). The visual editor highlights all bound params when the macro is hovered.

Macro controls render as DAW-style:
- Floats / Money: rotary knob (with text input fallback).
- Integers: stepper.
- Booleans: toggle switch.
- Enums: segmented control.
- Strings: text input.
- ModelRef: dropdown searchable.
- AgentRef: avatar-tile dropdown.

---

## 4. Live Preview (per Bret Victor §Topic 4)

### 4.1 Last-Value Preview

After any successful run of the Workflow, the canvas remembers each node's last output. Hovering a node shows a preview of its last output. This is per-node Cursor-style "what did the agent produce".

For markdown outputs, render the markdown. For JSON, pretty-print. For images, show thumbnail. For diffs, render with syntax highlighting.

### 4.2 Test-Isolated

Right-click any node → "Test in isolation". Opens a side panel:

```
Test:  prd-synthesize
Inputs (provide manually or from last-run-cache):
  cluster:   [paste JSON | use last-run]
  context_bundle: kb_nunchi_philosophy
Macros (current workspace bindings shown):
  ...
[Run isolated]
```

Runs only that Module in a sandbox, returns the output for inspection. No other Modules execute. Useful for tweaking prompts and seeing effects immediately. The "immediate connection" between thinking and seeing.

### 4.3 Run-Up-To-Here

Right-click any node → "Run up to here". Engine runs the partial graph from entry to that node, halts, returns the intermediate state. Lets the user inspect what reaches a given point without running the whole pipeline.

---

## 5. Multiple Views Over the Same Object

Per doc-3 §Composability: every Workflow has a **Recipe view** (linear) and a **Graph view** (DAG). Single keystroke (`v`) toggles. The underlying data is identical. Recipe view is the beginner / read-only-friendly mode; Graph is the power view.

In the Recipe view, conditional branches collapse into "If X, then run …" lines; loops collapse into "Repeat until Y" sections. Editing in Recipe view is constrained (you can reorder linear sections, adjust params); structural changes require Graph view.

A third **Timeline view** (per doc-3) shows the same Workflow's last run as a Gantt waterfall — nodes laid out along time. Useful for visualizing parallelism and bottlenecks.

---

## 6. Sound

Web Audio API generated tones, off by default. Six total per doc-3:

1. **Cable plug-in** — perfect fifth, 60ms triangle.
2. **Cable mismatch / adapter prompt** — minor second, 80ms.
3. **Validation success** — short major chord arpeggio.
4. **Validation failure** — low resonant tone, 200ms.
5. **Run start** — octave, 120ms with reverb tail.
6. **Run complete** — high sine ping, 60ms.

Volume slider, per-event mute, global mute. Headphones-friendly default volume.

The principle (research §Topic 1): juice the user's *actions* and the agent's *completions*, never ambient state.

---

## 7. Motion

Spring physics throughout (Framer Motion):
- Node drag: spring with stiffness 500, damping 30 (responsive).
- Node insert: spring stiffness 200, damping 28 (expressive); scales from 0.9 → 1.0.
- Cable connect: cable draws from source to target with a 200ms ease, brief flash on completion.
- Validation error highlight: shake animation with limited amplitude.
- Macro promotion: source-param card morphs into a new Macro card via FLIP animation.
- Recipe ↔ Graph view toggle: 400ms crossfade with positional FLIP.
- Run-state changes: edges flash rose during traversal; nodes pulse cyan when in flight.
- Stagger-children: 40ms intervals on list mounts (palette categories, run history, ...).

All animations respect `prefers-reduced-motion`. A single mute-motion toggle in Settings overrides for users who prefer no animation.

---

## 8. Keyboard

Keyboard-first composition is an explicit goal. Every mouse interaction has a keyboard equivalent:

```
Cmd+K               command palette (within editor: filtered to authoring actions)
Cmd+/               search palette of Modules to add
Cmd+E               toggle Recipe / Graph view
Cmd+R               run the workflow
Cmd+Shift+R         test-isolated the selected node
Cmd+G               group selection into a sub-workflow
Cmd+Shift+G         ungroup a sub-workflow into its components
Cmd+D               duplicate selected node(s)
Cmd+Z / Shift+Cmd+Z undo / redo
Cmd+S               save
Cmd+Shift+P         publish to marketplace
Cmd+M               promote selected param to macro
Tab                 cycle selection through outgoing edges of current node
Arrow keys          move selected node
Drag with Alt       duplicate-and-drag
Drag with Shift     constrain to axis
Esc                 deselect / collapse modal
.                   focus the palette search
[ / ]               collapse / expand inspector
,                   focus inspector
```

---

## 9. Error Handling at Author Time

Validation runs continuously as the user edits. Errors are inline (red glow on offending node + edge), not modal. The bottom strip aggregates errors:

```
✗ 2 errors  ⚠ 1 warning   [view]
```

Click `[view]` to open an issues panel listing all problems with click-to-jump.

Common errors and their inline UX:
- **Type mismatch on cable**: red cable; clicking offers an adapter or prompts to fix.
- **Required slot empty**: slot card has red dotted border with "Fill this slot" CTA.
- **Required macro unbound**: macro chip in palette is red.
- **Cycle detected**: cycle nodes pulse red; "Wrap in Loop?" suggestion offered.
- **Capability not granted**: node has amber capability badge; click for "Grant in workspace settings" CTA.
- **Cost estimate exceeds budget**: cost gauge in topbar amber; suggestion to lower budget or trim.

---

## 10. The Palette

The Palette is a virtualized scrolling list with categories collapsible. Each Module entry shows:
- Icon (per category).
- Name @ version.
- One-line description.
- Capability badges (small).
- Recently-used badge if used in the last 7 days.

Search at the top with fuzzy matching across name, description, tags, and inferred semantics ("the module that does X" — the search learns over time from the user's past queries).

Drag a Module onto the canvas to add. Drag onto an empty Slot to fill it. Drag onto an existing edge to insert mid-cable (the editor splits the cable around the new node, prompting for type adapters if needed).

Below the Modules tab, the Palette has tabs:
- **Modules**.
- **Filters / Branches / Loops / Human / Wait** (control-flow primitives).
- **Macros** (this Workflow's macros, draggable as references into params).
- **Slots** (this Workflow's slots).
- **Triggers** (this Workflow's triggers).
- **Snippets** (saved sub-graphs that can be pasted; user-created templates of common patterns).

---

## 11. Snippets

A user can select a region of the canvas and "Save as snippet". Snippets are reusable graph fragments stored at workspace or user level. A snippet inserts as a sub-workflow or as inline graph (user choice on paste).

Examples of useful snippets:
- "Fan out 3 ways, vote, take majority" pattern.
- "Run module then audit then refine" pattern.
- "Try cheap, escalate to expensive on fail" pattern.

Snippets are first-class marketplace artifacts (PRD-12).

---

## 12. Direct-Manipulation for Async (per Hutchins-Hollan-Norman)

The visual editor specifically counters the "linguistic gulf" of chat-based agent UI:
- **Plans / synthesized PRDs / diffs are draggable model-world objects**, not chat replies.
- **Live agent scratchpads** appear inline in the canvas: while a Module is running, its node displays its in-progress output as a small streaming text overlay.
- **Inline interrupt + steer**: pause a running Module from its node; edit its params; resume.
- **Branching by default**: every "Run" produces a branch in run history; users can switch among branches by selecting from the run-history strip; all branches preserved.
- **Reversible everything**: undo applies to authoring; for runs, "rollback to before this run's artifacts" is a one-tap action that detaches just-produced artifacts from the canonical store.

---

## 13. Presence & Multiplayer

Light presence in v1 (deferred multiplayer to v2):
- Cursor of other open viewers shown if the same Workflow is open in another browser session by the same user.
- Comments anchored to nodes (Figma-style); other team members can reply in v2.

Full multiplayer editing (Figma-style live cursor, conflict-free editing) is a v2 effort.

---

## 14. Authoring Modes

Two modes selectable from the editor toolbar:

- **Compose**: drag nodes, wire cables, edit params. Default. The "playing music" mode.
- **Inspect**: read-only canvas with all the run-history overlay enabled. Click anything to see deep details. Useful for reviewing a Workflow before forking.

A third **Live** mode is the "watch a run" mode (PRD-09 Performance Mode in dashboard skin) — the canvas overlays the latest run's animation in real time.

---

## 15. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Drag a Module from palette onto canvas → node appears at drop point with correct ports. | Manual + e2e. |
| Drag from output port to input port → cable created; type mismatch prompts adapter selection. | Type-system test. |
| Promote-to-macro replaces param value with macro reference and adds MacroDef. | Round-trip test. |
| Recipe ↔ Graph toggle preserves all data; resulting TOML is byte-identical (modulo whitespace). | Round-trip test. |
| Test-isolated runs the selected Module against fixture input and returns within 30s. | Latency test. |
| Live preview on hover shows last-run output for that node. | Snapshot. |
| Validation errors render inline; bottom-strip aggregates them. | Synthetic invalid graph. |
| Sound effects play on cable connect / validate / run-start / run-complete; mute persists. | Audio test. |
| Spring animations respect `prefers-reduced-motion`. | Accessibility test. |
| Keyboard equivalents for every mouse action documented in `?` overlay. | Spec invariant. |
| Snippet save & paste round-trip a sub-graph correctly. | Round-trip. |

---

## 16. Open Questions

- Should the editor support real-time concurrent editing in v1? Probably no; the implementation cost is high and the typical user is solo. Comments + presence cursor is enough.
- Should we render very large Workflows (50+ nodes) with auto-clustering / minimap? Yes — react-flow has minimap; auto-cluster sub-workflow internals on first view.
- Should there be an "AI assist" mode that suggests next steps as the user authors? Useful but rich behavior; specify in v1.1 (uses `workflow-author` Workflow under the hood).
- Should the editor expose a code-mode with raw TOML side-by-side with the canvas (live bidirectional sync)? Yes — power users want this; aligns with TouchDesigner's "Network and parameters" model.
- Should there be a way to attach voice-recorded notes to nodes? Useful for capturing intent during authoring; defer to v1.1.
