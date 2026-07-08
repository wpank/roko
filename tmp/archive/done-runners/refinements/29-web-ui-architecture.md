# Web UI Architecture

> **TL;DR**: A web UI should sit on top of StateHub (`26`) and the
> realtime surface (`27`) as a client, not as a server in its own
> right. This doc proposes a minimal first-party UI (3 to 5 core
> pages) built with a deliberate tech stack (SvelteKit or React
> via Vite, Tailwind, reactive store synced to StateHub),
> shippable in the same release as the backend, and extensible
> via the same plugin mechanism. Not a complete SaaS — a
> reference implementation that other teams can fork or extend.

> **For first-time readers**: This doc proposes a first-party web UI
> for Roko — five pages (Home, Chat, Plans, Beliefs, Settings), built
> on StateHub projections over the realtime event surface. The
> emphasis is "deliberate small" rather than "kitchen sink." A
> reference implementation that other teams can fork. Read 26 and 27
> first — the web UI is a consumer of the projection layer and
> realtime wire protocol defined there.

## 1. What the web UI is *for*

Not everyone opens a terminal. Not every operator installs a Rust
binary. The web UI serves:

1. **First-time curious users** who want to see Roko in action
   without installing anything.
2. **Non-developer stakeholders** (PMs, managers, executives) who
   want to observe without operating.
3. **Multi-user teams** where a shared single instance needs a
   shared visualization.
4. **Presentations and demos** where a browser window beats a
   terminal.
5. **Mobile viewing** — someone on a phone wants to see if the
   agent is done with the task they started this morning.

Non-goals: replicating the TUI's every feature, being a full IDE,
replacing the CLI for power users.

## 2. Tech stack

Pick a stack with a long-term UX story:

### 2.1 Framework

**SvelteKit** or **React + Vite**. Either works. Opinions:

- SvelteKit: smaller bundles, simpler reactivity, better for a
  dashboard-heavy UI.
- React + Vite: bigger ecosystem, more hires know it, better for
  an expanding UI.

Recommendation: SvelteKit for the reference implementation. Easy
to port later if a bigger team prefers React.

### 2.2 Styling

**Tailwind CSS**. Ubiquitous, consistent, enables fast iteration.
Supplement with **shadcn-svelte** (or shadcn/ui if React) for
high-quality primitives.

### 2.3 State

One **reactive store per StateHub projection**. The client library
from `27` wraps the realtime subscription; each store reflects
the current projection state and updates on deltas.

```ts
import { writable } from 'svelte/store';
import { roko } from '$lib/roko';

export const cohortHealth = writable<CohortHealthState | null>(null);
roko.subscribe("projection:cohort_health", {}, (msg) => {
  if (msg.type === "state") cohortHealth.set(msg.payload);
  if (msg.type === "delta") cohortHealth.update(s => applyDelta(s, msg.payload));
});
```

Same pattern for every projection. Pages just subscribe to stores
and render.

### 2.4 Charts

**visx**, **Observable Plot**, or **uPlot**. Declarative, fast,
good mobile support. Plot c-factor time series, balance
histograms, gate pass rates.

### 2.5 Editors

**CodeMirror 6** for inline code editing (diffs, patches). **Tiptap**
or **Lexical** for rich text (PRDs, heuristic descriptions).

## 3. The core pages

Ship exactly these. Resist scope creep.

### 3.1 Home / Pulse

Landing page. Five tiles:

- **System pulse**: current cohort c-factor (big gauge).
- **Active tasks**: tasks currently running with live progress.
- **Recent episodes**: last ~10 with one-line summaries.
- **Cost meter**: session-to-date spend.
- **Alerts**: gate failures, circuit breakers, demurrage extremes.

All tiles are live-updating. This is the "is everything OK?"
glance. A mobile-friendly view of this tile collection is the
default.

### 3.2 Chat

The interactive surface. Full-duplex via WebSocket. Features:

- Streaming token rendering.
- Inline diffs for proposed changes with apply/copy/edit.
- Slash commands (same set as CLI, `28` §4).
- File drop for context.
- Agent switch in the header (`@researcher`, `@implementer`).
- Voice input button for accessibility.
- Markdown + syntax highlighting.
- Replay link on every episode cite.

This is the most important page. 60% of usage will be here.

### 3.3 Plans

Tree/DAG visualization of plans and tasks.

- **Plan list** on the left; selected plan's DAG on the right.
- Nodes colored by status (pending, running, passed, failed).
- Clicking a task opens its episode trail.
- Drag-to-reorder tasks that haven't started.
- "Execute" button on non-running plans.
- Breakpoints: mark a task as "pause here, require approval."

This page is where Roko's plan-driven differentiation shows up
visually. It should be *beautiful*.

### 3.4 Beliefs

The heuristic + worldview browser.

- **Heuristics table** with calibration CIs, last trial,
  provenance.
- **Worldviews** as clustered views with dominant heuristics.
- **Replication ledger** with paper claims and our-vs-their
  effects.
- Challenge / retire / edit buttons.

This page communicates Roko's distinctive commitment to
empirical, inspectable belief. It's the *aha* page for skeptics.

### 3.5 Settings

Minimal. Covers:

- Model & API key management (delegates to `roko secret` via API).
- Profile selection (coding / research / blockchain / etc.).
- Gate thresholds (with reset-to-adaptive button).
- Plugin management (list, enable/disable, install from registry).
- Cost budgets.

Nothing exotic. Each settings change is an API call to the
control plane; the StateHub picks up the change and everyone
sees it immediately.

## 4. Component library

A reusable set of components lives in `@roko/ui` (or similar):

- `<CFactorGauge>` — the signature widget.
- `<EpisodeCard>` — stylized episode summary with citation trail.
- `<TaskNode>` — plan-DAG node with status and controls.
- `<HeuristicRow>` — calibration histogram + provenance.
- `<GateBadge>` — colored rung indicator.
- `<CostMeter>` — budget vs spend with warning states.
- `<ReplayTrack>` — timeline scrubber for an episode.
- `<DiffView>` — per-hunk diff with accept/reject.
- `<AgentAvatar>` — shows role, status, current action.

These are the building blocks. Third-party dashboards can import
them via `@roko/ui` and build custom pages.

## 5. Theming

- **Light + dark** baseline.
- **High-contrast** mode for accessibility.
- **Printable** CSS for exporting a replication report, PRD, or
  incident summary.
- Users can override via CSS variables. No proprietary theming
  system.

## 6. Routing and URLs

Every page is deep-linkable. Hash fragments for in-page state:

- `/plans/<slug>` — plan view
- `/plans/<slug>/task/<id>` — task focus
- `/beliefs` — heuristic list
- `/beliefs/h/<id>` — heuristic detail
- `/chat/<session>` — chat at a specific session
- `/episodes/<hash>` — episode detail

Users can share URLs. Replay links work. Screenshots come with
context.

## 7. Authentication

Three modes:

- **No-auth** (`--allow-any`): local dev only.
- **Basic auth**: for small teams. HTTP Basic over HTTPS.
- **OIDC**: for real deployments. Google Workspace, Microsoft,
  Okta, Authentik.

Session stored in an HTTP-only cookie; CSRF protection on all
mutations. The frontend never sees an API key.

## 8. Offline and progressive

The shell app is a PWA. Service worker caches assets. On
reconnect, StateHub catches up with cursors. Short outages don't
break the experience.

## 9. Accessibility

- Semantic HTML first, ARIA second.
- Every button and link reachable by keyboard.
- Screen-reader tested with a checklist (NVDA, VoiceOver).
- Focus outlines visible.
- Reduced motion respected.
- Internationalization: English default; every user-facing string
  in a resource bundle so translation is possible.

## 10. Mobile viewing (not full mobile app)

- Responsive layouts on all pages.
- Home / Pulse is the primary mobile view.
- Chat works on mobile but is read-biased (typing long prompts on
  a phone is not the goal).
- Plans DAG scrollable on mobile; horizontal focus is fine.

## 11. Extension points

### 11.1 Custom tiles on Home

Plugins can contribute Home-page tiles by registering a component
and a projection:

```ts
registerTile({
  name: "my-custom-tile",
  projection: "projection:my_metric",
  component: MyTile,
  size: "medium",
});
```

The registry fetches the component bundle lazily. Tiles are
sandboxed (iframe or shadow DOM) to prevent CSS bleed.

### 11.2 Custom pages

A plugin can register a top-level route. Users opt-in from
Settings. Third-party pages share the component library but can't
reach cross-origin resources.

### 11.3 Custom visualizations in existing pages

A plugin can register an alternative visualization for a projection
(e.g., a Sankey for lineage). Users choose from a dropdown.

## 12. Performance

Targets:

- **Time to interactive** on Home: under 2 seconds on a laptop.
- **Bundle size**: under 300 KB for the shell; pages lazy-load.
- **Memory**: under 150 MB browser memory even after an hour of
  chat.
- **Render budget**: no single update should block the main
  thread more than 16 ms (60 fps).

Tight budgets. Achievable with SvelteKit + careful component
structure. React is harder but doable.

## 13. Development experience

- Run `npm run dev` next to `cargo run -- serve` for full live
  development.
- Storybook (or equivalent) for every component — inspect in
  isolation.
- Typed schema from StateHub (codegen) so client types match
  server.
- Playwright for end-to-end tests.
- Vitest for unit tests.

## 14. What a non-developer sees on day one

```
[Home]
  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
  │   c-Factor   │  │ Active Tasks│  │    Cost     │
  │    0.76 ↑    │  │      3      │  │   $0.42     │
  │ (24h trend)  │  │ 2 passing   │  │   / $5.00   │
  └──────────────┘  └──────────────┘  └──────────────┘

  [Recent episodes]
    14:22  fix failing test in tests/core.rs       ✓  1.2m
    13:50  implement rate limiter                   ✓  4.8m
    13:12  investigate flaky integration test      ?  in progress

  [Alerts]
    ⚠ gate 'clippy' pass rate dropped 14% in last hour
```

A person with no context understands that the agent is
working, how expensive it's being, and whether anything needs
attention. That understanding is the product win. Most agent
frameworks don't try to produce it; Roko's substrate makes it
trivial once the UI exists.

## 15. Why not "just use the CLI"

The CLI is great. But:

- Non-developers can't or won't use CLIs.
- Mobile users can't use CLIs.
- Demos and board presentations don't work in CLIs.
- Observation and operation are different workflows; a web UI
  optimizes for observation.

The web UI earns its keep in those cases even if developers live
in the CLI.

## 16. Shipping sequence

1. **SvelteKit shell + auth + realtime connection**. Two weeks.
2. **Home + Chat pages**. Two weeks.
3. **Plans page**. Two weeks.
4. **Beliefs page**. Two weeks.
5. **Settings page**. One week.
6. **Component library polish + docs**. Two weeks.
7. **Accessibility audit**. One week.

A quarter of focused work for a credible reference web UI.
Shippable milestones at each step — Home + Chat alone would be
a release.

## 17. The big idea

Don't overbuild. Five pages, done well, built on a clean
StateHub + realtime surface, exposing a component library for
teams that want to fork. That's a product *and* a platform move:
the reference UI is both directly useful and a demonstration of
what the external API can do.

## 18. Page-level state ownership

Clear ownership rules so pages don't race each other:

- Each page owns its **local view state** (scroll position, expanded
  rows, input buffer). Not persisted across sessions.
- StateHub projections own **shared application state**. Pages
  subscribe, never mutate directly.
- Mutations go through **explicit API calls** that emit Pulses; the
  Pulses eventually produce Deltas that update the projection.
- **Optimistic UI**: for high-latency actions, apply a speculative
  local delta, then reconcile when the real delta arrives. Label
  speculative state visually (dimmed, italic, or "..." suffix).

A rule that compounds: any piece of UI that needs to live across
page navigation belongs in a projection, not in a page-local store.

## 19. The five-page minimal feature surface

| Page | Subscribed projections | Possible mutations |
|---|---|---|
| Home / Pulse | `cohort_health`, `active_tasks`, `recent_episodes`, `cost_meter`, `alerts` | Acknowledge alert |
| Chat | `agent_trails`, `recent_episodes`, `heuristic_library` (for footnotes) | Send prompt, apply diff, annotate |
| Plans | `plans_list`, `plan_detail/<id>` | Create, pause, resume, execute, reorder tasks |
| Beliefs | `heuristic_library`, `worldview_clusters`, `replication_ledger` | Challenge, retire, edit, import, export |
| Settings | `config_current`, `plugins_list`, `secrets_status` | Set config keys, install/enable plugins, rotate secrets |

Every mutation is an explicit API call. The web UI never reaches
directly into Substrate or Bus — only through the realtime wire
protocol and StateHub projections.

## 20. Responsive / mobile specifics

A phone is not a shrunk desktop. Page-by-page mobile rules:

- **Home**: 5 tiles stack vertically, each full-width. Alerts
  show as a sticky footer if any are pending.
- **Chat**: full-screen; keyboard + voice-input button. Tool-call
  banners collapse by default; expand on tap.
- **Plans**: DAG becomes a vertical list of tasks with expand
  arrows. Drag-to-reorder disabled on mobile; long-press for move.
- **Beliefs**: heuristic cards stack; calibration CI renders as
  mini bar on each card. Worldview clusters scroll horizontally.
- **Settings**: read-only for most fields; explicit "Edit on
  desktop" hint for irreversible operations.

Touch target minimum 44pt (Apple HIG). Spacing generous. Font
at least 16px (browser won't zoom on iOS Safari).

## 21. Server-side rendering and first-paint

SvelteKit defaults to SSR for the first page load. Rules:

- **Home** SSRs the projection initial state; hydrates with live
  subscription client-side. First paint under 500 ms on a good
  connection.
- **Chat, Plans, Beliefs** SSR the shell + auth state; load
  projections after hydration. Keeps bundle size down for landing.
- **Settings** is a protected route; SSR loads auth only; form
  state loads on client.

Budget: Lighthouse Performance >= 90 on Home, >= 85 on others.

## 22. Deep-link semantics

Shareable URLs from §6 are load-bearing. Explicit cases:

- `/plans/<slug>?task=<id>` — pre-opens task focus on the plan page.
- `/chat/<session>?cursor=<c>` — scrolls chat to a specific message.
- `/episodes/<hash>` — direct link into an episode's detail view.
- `/beliefs/h/<id>?challenge=true` — opens challenge modal on the
  heuristic.
- `/replay/<episode>?t=0:45` — opens replay scrubber at a specific
  offset.

When a user shares a deep link:

- Public session links expire (default 24h; configurable).
- Private session links require auth.
- Shared replay links embed a snapshot cursor so stale state
  doesn't resolve differently from when the link was created.

## 23. Cross-references

- Projection layer this UI consumes: `26-statehub-rearchitecture.md`.
- Wire protocol behind subscriptions: `27-realtime-event-surface.md`.
- Component library primitives (diffs, footnotes, scrubbers) come
  from: `30-rich-ux-primitives.md`.
- CLI equivalent of each mutation: `28-cli-parity-familiar-workflows.md`.
- Permission model for mutating actions:
  `32-safety-sandbox-provenance.md` §4.
- Observability for the UI (RUM metrics, error tracking):
  `33-observability-telemetry.md` §7.
- Plugin-contributed custom pages and tiles:
  `17-plugin-extension-architecture.md` §2.
