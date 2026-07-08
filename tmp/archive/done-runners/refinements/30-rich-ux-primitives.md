# Rich UX Primitives

> **TL;DR**: Great agent UX goes beyond "render a chat box." It
> requires a vocabulary of primitives — annotations, time
> scrubbing, explanation pop-outs, uncertainty visualization,
> alternative renderings — that turn agent activity into a
> *legible*, *interactive*, *replayable* experience. This doc
> proposes ten UX primitives, each with a concrete definition,
> the data it needs from the substrate, and where it lives in the
> product. Together they describe the world-class, expressive UI
> the user asked for.

> **For first-time readers**: Primitives here are the small, reusable
> UX building blocks — not full pages. Tool-call banners,
> heuristic footnotes, uncertainty bars, replay scrubbers, and so on.
> Each one is a self-contained affordance. The pages in 29 (web UI)
> and the TUI tabs in 23 assemble these primitives. This doc is the
> vocabulary they share.

## 1. What makes agent UX different

Traditional app UX is mostly request-response: user takes an
action, app reflects a new state. Agent UX has three extra
dimensions:

1. **Latency**: agents take seconds to minutes; users need to
   feel progress.
2. **Uncertainty**: agents are probabilistic; users need to
   calibrate their trust.
3. **Causality**: agents make chains of decisions; users need to
   see *why*.

A rich agent UX exposes all three without overwhelming. The
primitives below each handle one or more.

## 2. The ten primitives

### 2.1 Reasoning streams

As the agent thinks, render a *reasoning stream* alongside the
primary response. Token-level streaming of the "plan" or
"approach" the agent is forming. Collapsible by default;
expanded for users who want to watch.

Data source: `topic:agent.reasoning` — a Bus topic each agent
publishes to. Contains draft plans, considered alternatives,
discarded paths.

Rendering: a subtle sidebar or footer strip. Text fades in as
it's generated. Dims when the agent is acting (tool call, file
write) and relights when it's thinking again.

### 2.2 Tool-call affordances

Every tool invocation renders a banner:

```
[tool: cargo.test]
  ran 47 tests, 3 failed (2.3s)
  [show output] [rerun] [explain]
```

Clickable. `show output` reveals stdout/stderr. `rerun` re-calls
the tool with the same args. `explain` shows *why* this tool was
chosen over alternatives.

Data source: `topic:tool.started`, `topic:tool.completed`,
`topic:tool.failed`.

### 2.3 Gate badges

Gates produce pass/fail/warn outcomes. Each outcome is a badge:

```
[✓ unit] [✓ type] [⚠ style: 2 lints] [✗ diff: unapproved paths]
```

Clickable. Tapping `style` opens the lint output. Tapping `diff`
opens the paths that failed the allow-list.

Data source: the gate pipeline publishes detailed results to
`topic:gate.*`. The projection `gate_pipeline` surfaces it in
State.

### 2.4 Heuristic footnotes

Whenever an agent's response was shaped by specific heuristics,
show them as footnotes — small numbers inline, details on hover
or click:

```
The test is flaky due to timing¹. Add logging before touching the
logic itself².

---
¹ Heuristic h.42 — "flaky tests usually have timing dependencies"
  calibration: 0.81 (32 trials). Source: Kernighan & Pike §5.2.
² Heuristic h.87 — "log before changing logic in flaky code"
  calibration: 0.74 (19 trials). Source: local episodes.
```

Data source: the episode records which heuristics were injected
into the prompt; the Composer publishes a `heuristic.applied`
Pulse per each.

This is the single most differentiating UX feature. No other
agent framework shows *why the agent said what it said* at this
fidelity.

### 2.5 Uncertainty bars

When the agent emits a decision, it can also emit an uncertainty
estimate. Render as a small bar under the decision:

```
Apply patch to src/core.rs                 confidence ████░░░ 0.61
```

Below a threshold, the UI highlights the decision with a
yellow background and prompts the user for explicit approval.
Above a threshold, the decision is applied automatically (if
auto-mode is on).

Data source: the agent's own prediction confidence, published on
`topic:decision.emitted`.

### 2.6 Replay scrubber

Every episode has a timeline. Render it as a horizontal track
with events as ticks:

```
[Episode ep_123: "fix failing test" — 3m24s]
├─●─●─●─────●───●─●─────●──●
  │ │ │     │   │ │     │  └─ episode completed
  │ │ │     │   │ │     └──── gate unit ✓
  │ │ │     │   │ └────────── tool cargo.test
  │ │ │     │   └──────────── file write src/core.rs
  │ │ │     └──────────────── tool fs.read
  │ │ └──────────────────────── heuristic h.42 applied
  │ └────────────────────────── tool git.status
  └──────────────────────────── episode started
```

Scrub to any point; the chat, diff, and state panels rewind to
that moment. Combined with `replay` CLI, this is time travel
through agent decisions.

Data source: the episode's full pulse stream plus substrate
snapshots.

### 2.7 Alternative renderings

Some data has multiple good views. Let the user choose:

- **Plan DAG**: tree / graph / timeline / kanban board.
- **Episode detail**: chronological / causal / by-tool / by-file.
- **Heuristic library**: list / clustered / by-domain / by-calibration.

Each rendering shares the same underlying projection; just
swaps the component. Adding a rendering is a plugin (`29` §11.3).

### 2.8 Confidence-weighted aggregation

When multiple agents produce answers, show them with confidence
weighting rather than majority vote:

```
Consensus view: "the bug is in the retry loop"  [confidence: 0.78]
  @researcher:    "retry loop"        0.81 ↑
  @implementer:  "retry loop"        0.74 ↑
  @reviewer:     "config handling"   0.52 ↓ (minority view)

[show all three] [see minority evidence]
```

The minority view is *always* accessible — one click away, never
hidden. This operationalizes `13-collective-intelligence-c-factor.md`
and `14-worldview-validation.md` §5 in the UI.

### 2.9 Progressive disclosure

Most users want the short answer. Power users want the full
chain. Both are served by layering:

```
"The test is flaky. Add logging first." [▸ show reasoning]
                                        [▸ show heuristics]
                                        [▸ show trace]
                                        [▸ show cost]
```

Each toggle expands inline. No separate panels, no modal dialogs,
no navigation loss. The user digs exactly as far as they care to.

### 2.10 Spatial memory

Each user has a consistent spatial map across sessions:

- Chat always on the right; plan DAG always on the left.
- Settings always accessed from the gear icon in the corner.
- `cmd+k` always opens the command palette.

Muscle memory builds. Users stop having to look for things.

This is a design *discipline*, not a feature. But it is a
primitive because every other primitive sits on top of it.

## 3. Annotations as first-class objects

Users should be able to *annotate* anything: an episode, a
heuristic, a plan, a diff. Annotations are Engrams too:

```rust
pub struct Annotation {
    pub target: EngramHash,
    pub author: PrincipalId,
    pub kind: AnnotationKind,
    pub body: String,
    pub timestamp: Timestamp,
}

pub enum AnnotationKind {
    Note,
    Correction,
    Confirmation,
    Question,
    Followup,
}
```

- User sees a wrong heuristic calibration → annotates as
  *Correction*. The Calibrator de-weights the offending trial.
- User reads an episode summary and disagrees → annotates as
  *Correction*. A future distillation incorporates the feedback.
- User wants to revisit later → annotates as *Followup*. A
  dashboard tile shows pending followups.

Annotations connect human feedback to substrate in a way that
unifies all the "thumbs up/down" patterns from other products
into one primitive.

## 4. Explainability panel

A single persistent panel (keyboard shortcut: `?`) that always
shows:

- What the agent is currently doing.
- What heuristics are active.
- What gates are about to run.
- What tools are available.
- What budget remains.
- What the agent doesn't know yet.

Users can open it anywhere. It's always up to date. It's the
`/explain` slash command as a permanent affordance.

## 5. Voice and audio

Optional but valuable:

- **Voice input** in chat, via browser SpeechRecognition API.
- **Voice output** for progress ("your agent finished the test
  fix") so users can multitask.
- **Ambient sound design**: subtle tones for gate pass / fail /
  cost-milestone. Accessibility-friendly, toggleable.

Done well, these are differentiators. Done poorly, they are
annoying. Ship toggleable, off by default.

## 6. Collaborative presence

When multiple users share a Roko instance:

- **Cursors** in shared views (who's viewing what).
- **Live edits** in PRD drafts (like Google Docs-level concurrency).
- **Comments** threaded on any target.
- **@mentions** that notify a specific user.

Most of this comes free from the StateHub + realtime surface.
The UI just has to surface it.

## 7. Keyboard-first everything

Every interaction has a keyboard path. Discoverable via `?`
overlay (like Linear, Notion):

```
[Keyboard shortcuts]
Global
  cmd+k         Command palette
  cmd+/         Toggle explainability panel
  g h           Go to Home
  g c           Go to Chat
  g p           Go to Plans
  g b           Go to Beliefs
  g s           Go to Settings
...
```

Power users never touch the mouse. New users discover gradually.

## 8. Notification discipline

Don't notify unless it matters. Criteria for notification:

- A gate failure the user needs to see.
- A budget threshold hit.
- An approval checkpoint requires a human.
- A long-running task completed (and the user is not currently
  watching).

Never notify for:

- Successful episodes (dashboard is enough).
- Intermediate tool calls (dashboard is enough).
- "Your agent is working" (the agent is always working).

Notifications respect OS focus state, do-not-disturb, and user
preferences.

## 9. Undo visibility

Undo must be *visible* to feel safe. When the user applies a
change, a subtle toast shows:

```
✓ Applied diff to src/core.rs. [undo] (fades after 10s)
```

After 10s it fades but isn't gone — `cmd+z` still works.
`roko session history` shows all applied changes with
timestamps.

## 10. Error recovery

Errors are UX. When something fails:

- Big red block is bad. Inline, calm indicator is better.
- Always show *what* failed, *why*, and *what to try*.
- Offer a one-click retry when safe.
- Link to docs.
- Capture reproduction data for bug report (opt-in) automatically.

Consider the `roko` equivalent of Rust compiler errors: dense,
actionable, linked, teachable.

## 11. Mobile-specific patterns

- Mobile UI is *read-biased*. Observation > operation.
- Voice-to-text for the occasional prompt.
- Push notifications for approvals and completions.
- Offline: queue approvals, sync when back online.
- Large tap targets, short labels.

Users should be able to unlock their phone while commuting, see
a green check, and feel confident the agent finished the test
fix.

## 12. The primitives together

Imagine a user watching a failing test get fixed:

1. They type `/fix failing test` in chat.
2. Reasoning stream appears: "reading failing test... the test
   checks timing..."
3. Tool-call banner: `[tool: fs.read tests/core.rs] (0.1s)`.
4. Heuristic footnote: "Applied h.42: flaky tests often have
   timing issues."
5. Uncertainty bar: the fix has confidence 0.73.
6. Gate badges update live: `[✓ unit] [✓ type] [⚠ clippy]`.
7. Per-hunk diff appears with accept/reject.
8. User accepts. Toast: "applied, [undo]".
9. Episode appears in the replay scrubber.
10. A small banner: "this took 2 min, $0.04; want a heuristic
    from this episode?" — user clicks yes, a new heuristic gets
    drafted for review.

Every step is a primitive. The sum is an *experiential* UI — the
user *feels* the agent working, understands *why* it did what it
did, and can *shape* what it learns. This is the product.

## 13. The research that informs these primitives

Worth grounding these in real HCI and UX research (following
the `16-research-to-runtime.md` spirit):

- Tufte on explanatory visualization (reasoning streams, replay).
- Nielsen heuristics (visibility of status, user control, undo).
- Norman's affordances (tool-call banners are clickable because
  they look clickable).
- Tognazzini's first principle of interaction: system state is
  always visible.
- Shneiderman's direct manipulation (per-hunk diff acceptance).
- Progressive disclosure (Nielsen again).
- Krug's "don't make me think" (spatial memory).

Each primitive cites. Each cite becomes a Paper Engram and the
primitive's effectiveness becomes a Claim to replicate.

## 14. Priority

Most-impact primitives first:

1. **Streaming + tool-call banners**. In chat only. One week.
2. **Gate badges**. Inline. Three days.
3. **Heuristic footnotes**. One week.
4. **Replay scrubber**. Two weeks.
5. **Explainability panel**. One week.
6. **Uncertainty bars**. One week.
7. **Annotations + undo**. Two weeks.
8. **Keyboard shortcuts + command palette**. One week.
9. **Alternative renderings + confidence-weighted aggregation**.
   Two weeks.
10. **Voice / collaborative presence / mobile polish**. Ongoing.

A quarter of focused UX work lands primitives 1–8. The difference
between a "usable" agent UI and a *great* one is almost entirely
in this list. Each primitive is small; together they are a
product.

## 15. Primitive data dependencies

Each primitive depends on specific upstream data. If the upstream
isn't shipping the data, the primitive is decorative. Explicit
table:

| Primitive | Requires | Home doc for upstream |
|---|---|---|
| Reasoning streams | `agent.reasoning` Pulses | 02, 03 |
| Tool-call banners | `tool.*` Pulses | 02, 03 |
| Gate badges | `gate_pipeline` projection | 26 |
| Heuristic footnotes | `heuristic.applied` Pulses + `heuristic_library` projection | 14, 26 |
| Uncertainty bars | `decision.emitted` Pulses with confidence | 10 §5 |
| Replay scrubber | Full episode Pulse stream + Substrate snapshots | 02, 12 |
| Alternative renderings | Projection `State` shape | 26 |
| Confidence-weighted aggregation | `consensus.*` Pulses | 13 §5 |
| Progressive disclosure | Composer's layer provenance | 04, 14 |
| Spatial memory | UI framework; no substrate dep | — |

A primitive whose upstream isn't ready degrades gracefully: gate
badges work without heuristic footnotes; reasoning streams work
without uncertainty bars. Ship in the order of readiness.

## 16. Keyboard-shortcut registry

One registry, shared across TUI and web, avoids collisions:

| Shortcut | Action | Surface |
|---|---|---|
| `cmd+k` / `ctrl+k` | Open command palette | TUI, Web |
| `cmd+/` / `ctrl+/` | Toggle explainability panel | Web |
| `?` | Help overlay | TUI, Web |
| `/` | Slash-command focus | TUI, Web Chat |
| `:` | Command palette (alt) | TUI |
| `q` | Quit / close | TUI |
| `j` / `k` | Navigate down / up | TUI |
| `g h` / `g c` / `g p` / `g b` / `g s` | Go Home / Chat / Plans / Beliefs / Settings | Web |
| `cmd+z` / `ctrl+z` | Undo last applied change | TUI, Web |
| `cmd+enter` | Send prompt | Web Chat |
| `cmd+shift+enter` | Send prompt with `--plan` mode | Web Chat |
| `[` / `]` | Previous / next tab | TUI |

Registered in `roko-ui::shortcuts` (single source of truth). Users
customize via `~/.roko/keybindings.json`. Plugins register new
shortcuts via the same registry with collision warnings.

## 17. Color / motion discipline

- **Semantic color**: green for pass / active / healthy; red for
  fail / error; yellow for warning / approval-required; blue for
  info / neutral progress. Never decorative.
- **High-contrast mode** always tested — color-blind users
  shouldn't miss critical state.
- **Reduced motion**: respects OS/browser preference. Disable
  token-streaming typewriter effect, fade transitions, etc.
- **Icon-plus-text for status**: never color-only. `✓ passed`,
  not green square alone.
- **Motion duration bounded**: no animation longer than 300 ms;
  nothing loops indefinitely (prevents dashboard fatigue).

## 18. Failure-mode gallery

Each primitive should degrade gracefully on upstream loss:

| Primitive | Upstream loss | Degraded behavior |
|---|---|---|
| Tool-call banners | `tool.*` Pulses drop | Show "tool call ..." placeholder |
| Gate badges | Gate pipeline stalls | Badges stay at last state; add "stale" marker |
| Heuristic footnotes | Heuristic retrieval fails | Drop footnotes; don't block text |
| Uncertainty bars | No confidence published | Omit the bar |
| Replay scrubber | Episode's Pulse stream lost | Disable scrubber; show "replay unavailable" |
| Alternative renderings | Projection stops updating | Freeze current; show "disconnected" badge |

A user who loses WiFi mid-session sees a degraded but coherent UI,
not a broken one. On reconnect, cursors (27 §6) catch up.

## 19. Primitives in the TUI specifically

Several primitives need TUI-native renderings, not just web:

- **Tool-call banners**: single-line box with color bar on left.
- **Gate badges**: compact row of colored brackets.
- **Reasoning streams**: right-sidebar pane (toggle with `r`).
- **Replay scrubber**: bottom-bar timeline with tick glyphs.
- **Heuristic footnotes**: inline superscript numbers; press `f`
  to expand footnote pane.
- **Uncertainty bars**: unicode block characters `▁▂▃▄▅▆▇█`.

Implementation shares state with web via StateHub projections, so
the same Engrams and Pulses drive both. Difference is purely
rendering.

## 20. Cross-references

- Data contracts each primitive consumes:
  `02-engram-vs-pulse.md`, `03-bus-as-first-class.md`.
- Projections that feed most primitives:
  `26-statehub-rearchitecture.md`.
- TUI-specific implementation:
  `23-user-ux-running-agents.md` §5.
- Web-specific implementation:
  `29-web-ui-architecture.md` §4.
- Mobile adaptations:
  `29-web-ui-architecture.md` §20.
- Accessibility:
  `23-user-ux-running-agents.md` §13, `29-web-ui-architecture.md` §9.
- The observability behind ambient audio / notifications:
  `33-observability-telemetry.md` §8.
