# UX Arc Audit: Refinements 22-30

Audit of the "UX" refinement arc (developer UX, user UX, deployment,
domain profiles, StateHub, realtime, CLI parity, web UI, rich
primitives). Cross-referenced against the actual codebase as of
2026-04-17.

---

## REF-22: Developer UX — Four-Layer Rust SDK

**Verdict: DEFER**

### What it proposes

A four-layer SDK (one-liner / builder / trait-impl / runtime-impl)
with `roko::run("...")`, `Agent::builder()`, proc macros (`#[tool]`,
`#[gate]`), a `cargo roko` plugin, 10 worked examples, full rustdoc
discipline, and a 6-week release train.

### What actually exists

- The 6 kernel traits exist in `crates/roko-core/src/traits.rs`:
  `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`.
  These are the "trait-impl" layer and they are real.
- No `roko::run()` one-liner exists anywhere. No `Agent::builder()`
  exists. `grep -rn 'roko::run\|Agent::builder\|roko::prelude'
  crates/` returns zero hits.
- The `examples/` directory has 10 files but they are all `.md`
  docs and `.toml` config samples, not runnable Cargo projects.
- Error types exist in `crates/roko-core/src/error/` (1,493 lines)
  with `RokoError` but no `#[non_exhaustive]`.
- No proc macros (`#[tool]`, `#[gate]`, `claim!`, `prompt!`) exist.
- No `cargo roko` plugin exists.
- 29 crates already exist. This is a complex workspace. The "one-liner
  hello agent" would require designing a facade crate that does not
  exist.

### Honest assessment

The aspiration is correct but it is designing for an audience that
does not exist yet. There are zero external Roko users. The SDK
surface matters when someone wants to import `roko` as a library; the
current consumption mode is `cargo run -p roko-cli`. Building a
polished SDK surface before the system self-hosts reliably is
premature.

The useful subset:
- Better error types with actionable messages: YES, do this.
- `#[warn(missing_docs)]` enforcement: YES, cheap.
- A worked `examples/` directory with one or two runnable demos: YES.
- `roko::run()` one-liner: only after `Agent::builder()` and
  `AgentBuilder` exist, which requires unifying the dispatcher and
  tool-loop into a coherent builder. That is a real refactor, not a
  doc exercise.

The scope-creep:
- `cargo roko` plugin: 6 subcommands for a tool nobody uses yet.
- `#[tool]` / `#[gate]` proc macros: nice but premature. Ship the
  trait impls; macros can come later.
- `claim!` macro: depends on a research-to-runtime system that is
  conceptual.
- 6-week release train with SemVer: you have 0 dependents.
- `BrowserRuntime`: WASM runtime impl for browsers. There is no WASM
  target in any Cargo.toml.

---

## REF-23: User UX — One Verb Set Across All Surfaces

**Verdict: SIMPLIFY**

### What it proposes

Nine canonical verbs (ask, plan, do, watch, inspect, replay, learn,
tune, connect) rendered identically in CLI, TUI, Chat, and Web.
Interactive `roko init`. TUI becomes a control surface. Multi-agent
chat. Slash commands in chat. Undo. Session export/replay. i18n.

### What actually exists

- CLI: ~40+ subcommands in `main.rs` (7,462 lines). `run`, `plan`,
  `prd`, `status`, `replay`, `config`, `chat`, `dashboard`, `serve`,
  `research`, etc. These are real and working.
- TUI: 22K lines in `crates/roko-cli/src/tui/`. Has pages, widgets,
  themes, modals, state management, fs/git watchers. Substantial.
- Chat: 131 lines in `chat.rs`. A bare REPL that posts to
  `roko-serve` and polls for completion. No streaming. No slash
  commands. No multi-agent. This is the weakest link.
- Web: `roko-serve` has 13K lines of routes but no first-party HTML
  is served. API-only.

### Honest assessment

The verb unification is a good idea but the proposal conflates
"clean up CLI flag inconsistencies" (a weekend job) with "build a
full multi-surface UX framework with i18n, undo, session export,
and accessibility" (months of work for features nobody has asked for).

The useful subset:
- Improve `roko init` to be interactive and detect models/MCP: YES.
  Low effort, high payoff.
- Standardize `--format`, `--quiet`, `--verbose` across subcommands:
  YES. clap makes this easy.
- Make `roko chat` not terrible: YES. Add streaming, basic slash
  commands. It is currently a 131-line polling REPL.
- Make the TUI more interactive (execute plans, adjust thresholds):
  YES, but incrementally. The TUI is already 22K lines.

The scope-creep:
- i18n ("internationalizable strings"): for a tool with 0 non-English
  users and no strings extracted.
- Session export/replay/sharing with URL generation: nobody is
  requesting this.
- `roko session share --expires 24h` uploading to a registry: what
  registry?
- "Heuristic commons opt-in dialog" during init: the heuristic
  commons does not exist.

---

## REF-24: Deployment UX — Five Shapes

**Verdict: SIMPLIFY (most already done)**

### What it proposes

Five deployment shapes (laptop, single-server, container, clustered,
edge). Deployment profiles in roko.toml. Secret management CLI.
State export/import. Observability (Prometheus, OpenTelemetry).
Docker image. Helm chart. WASM target. Multi-tenancy with OIDC.
Air-gap support.

### What actually exists

- **Laptop-local**: works. This is the default.
- **Docker**: `docker/roko.Dockerfile` exists and builds. Multi-stage
  with rust:1.91, distroless runtime base would improve it but the
  current image works. `docker/docker-compose.yml` exists with
  roko + mirage + prometheus + grafana.
- **Container/server**: `roko serve` works.
- **Secrets**: `roko config set`, `roko secrets` subcommands exist.
- **Clustered / WASM / edge**: nothing exists. No NATS, no Kafka,
  no WASM target in any Cargo.toml.

### Honest assessment

This is the most grounded of the UX docs because "laptop" and
"container" already work. The Dockerfile is real. docker-compose is
real. The proposals to improve them are concrete and achievable.

The useful subset:
- `profile` concept in roko.toml: YES, small config addition.
- `roko state export/import`: YES, useful for backups. Small scope.
- Improve the Docker image (distroless, healthcheck, musl): YES.
  The sketch in the doc is close to the existing Dockerfile.
- `roko secret set/get/rotate`: partially exists, finish it.
- Cost visibility (live spend counter): YES, pairs with the cascade
  router that already tracks costs.

The scope-creep:
- Helm chart: for zero Kubernetes users of this tool.
- WASM target (`roko-wasm` binary): zero evidence this is needed.
- Multi-tenancy with OIDC, JWT-to-tenant mapping, group rules:
  this is SaaS infrastructure for a single-user tool.
- Kafka / NATS bus backends: no code exists, no demand exists.
- Air-gap plugin registry mirror: for a plugin registry that does
  not exist.
- "Zero-downtime upgrades" with rolling restart: for something
  that restarts in under a second.

---

## REF-25: Domain-Specific Agents — Six Profiles + TypedContext

**Verdict: SKEPTICAL**

### What it proposes

Six domain profiles (coding, research, blockchain, data/ML, ops,
writing) as installable plugin bundles. TypedContext primitive in
roko-core. Custody record for audit trails. Domain-specific gates,
heuristics, and evaluation suites.

### What actually exists

- **Coding agent**: the entire system is oriented around this. Tools
  (fs, cargo, git), gates (compile, test, clippy, diff), roles
  (researcher, planner, implementer, reviewer) all exist and work.
- **Research agent**: `roko research` subcommands exist.
- **Blockchain**: `roko-chain` crate exists but is tagged "Phase 2+".
- **Data/ML, Ops, Writing**: nothing exists.
- **TypedContext**: zero hits in the codebase. Does not exist.
- **Custody**: zero hits in the codebase. Does not exist.
- **Plugin bundles / profiles**: `roko-plugin` exists (event sources
  and feedback collectors) but has no concept of "domain profiles"
  or "installable bundles."

### Honest assessment

This doc is designing a product ecosystem for a product that has one
user. The coding agent profile is implicit in how the system already
works. Formalizing it as an installable profile requires a plugin
registry, a bundle format, a profile composition system, and
conflict resolution rules -- all of which are new infrastructure for
a currently-internal tool.

TypedContext is actually interesting as a kernel primitive. Situations
are currently free-text, and typed matching would improve gate and
heuristic precision. But the doc wraps it in 2-3 months of profile
infrastructure that is premature.

The useful subset:
- TypedContext as a struct in roko-core: YES, small and useful.
  Gates and heuristics would benefit from structured situation data.
- Formalize the existing coding setup as a default "profile" config:
  YES, but as a config section, not an installable bundle.
- Custody record for audit trail: interesting for ops and blockchain
  contexts but premature as a core primitive.

The scope-creep:
- Six fully specified domain profiles with starter heuristic
  libraries: you don't have users in 5 of these 6 domains.
- Domain-specific evaluation suites with benchmark scores: for whom?
- Profile composition rules with priority ordering: for zero
  installed profiles.
- `roko plugin install @roko/coding-profile`: the plugin registry
  does not exist.
- Voice fingerprinting via HDC encoding for writing agents: pure
  fantasy at this stage.

---

## REF-26: StateHub Rearchitecture

**Verdict: SHIP IT (mostly already done)**

### What it proposes

Promote StateHub from a TUI helper to a kernel subsystem with typed
projections, subscription filters, multi-consumer delivery,
transport-agnostic wire format, and replayable cursors.

### What actually exists

StateHub already IS a kernel subsystem. It lives in
`crates/roko-core/src/state_hub.rs` (343 lines). It has:

- `publish()` and `publish_batch()` for events.
- `snapshot()` returning `watch::Receiver<DashboardSnapshot>` for
  the TUI.
- `subscribe_events()` returning a broadcast receiver for WS/SSE.
- `replay_from(seq)` for late-joiner catchup via ring buffer.
- `sender()` returning a clone-safe `StateHubSender`.
- `SharedStateHub` with `bootstrap_from_workdir()`.
- Full test coverage.

The `DashboardSnapshot` (2,350 lines) already materializes state
from events and is consumed by:
- The TUI (22K lines) via `watch::Receiver`.
- The WebSocket endpoint (`routes/ws.rs`) via broadcast.
- The SSE endpoint (`routes/sse.rs`) via broadcast.
- REST endpoints via `current_snapshot()`.

### Honest assessment

This is the best refinement doc because it proposes what largely
already exists. The current StateHub is already multi-consumer,
event-driven, replay-capable, and transport-aware. It already serves
TUI, WebSocket, SSE, and REST.

What the doc proposes beyond current state:
- Named typed projections with `Projection` trait: this is a
  generalization of the current monolithic `DashboardSnapshot`. It
  would let you subscribe to just `cohort_health` instead of the
  whole snapshot. This is a real architectural improvement but not
  urgent -- the monolithic snapshot works fine at current scale.
- Subscription filters: could be useful but the current "everything
  or nothing" delivery works because there is one user.
- New crate `roko-statehub`: the code already lives in `roko-core`.
  Moving it out is a pure refactor.

The useful subset:
- Keep doing what you're doing. StateHub is well-designed.
- If/when you need per-projection subscriptions, add them. Not now.
- The `Projection` trait is a good north star for the API shape.

The scope-creep:
- Ten canonical projections with typed State/Delta: over-specified
  for current needs.
- `roko-statehub` as a new crate: pure refactor overhead.
- Custom projections via plugin registry: premature.

---

## REF-27: Realtime Event Surface — WS / SSE / gRPC

**Verdict: SIMPLIFY (WS + SSE already done)**

### What it proposes

Three co-equal transports (WebSocket, SSE, gRPC) with a unified
subscription protocol, five channel types, back-pressure semantics,
auth, cursor resumption, and three first-party client libraries
(TypeScript, Python, Rust).

### What actually exists

- **WebSocket**: `crates/roko-serve/src/routes/ws.rs` (139 lines).
  Working. Has replay-from-ring, filter subscriptions, live event
  streaming.
- **SSE**: `crates/roko-serve/src/routes/sse.rs` (47 lines). Working.
  Has event IDs for reconnection and keep-alive.
- **gRPC**: zero. No tonic, no protobuf in any Cargo.toml.
- **Client libraries**: zero. No `@roko/client`, no `roko-client`
  Python package, no `roko-client-rs` crate.

### Honest assessment

WebSocket and SSE already work. They stream DashboardEvents. The
subscription protocol is simple JSON. This is adequate for the
current state.

The doc proposes formalizing what exists and adding layers on top.
Some of it is useful (cursor resumption, proper back-pressure) but
most of it is infrastructure for consumers that don't exist.

The useful subset:
- Harden the existing WS/SSE with proper cursor tracking: YES.
  The SSE endpoint already uses `envelope.seq` as event ID, which
  is most of the way there.
- Document the existing wire format: YES, cheap and useful.
- `roko-protocol` crate for shared types: maybe, if the types
  diverge between serve and cli.

The scope-creep:
- gRPC: zero use case, adds tonic + protobuf build deps.
- Three first-party client libraries: for zero external consumers.
- Five channel types (projection:, topic:, engram-stream:, agent:,
  session:): over-specified. The current "subscribe to DashboardEvents"
  works.
- GraphQL (even "maybe, carefully"): no.
- Wire format stability contract with frozen schema and corpus
  testing: for zero external consumers.
- Presence channel: "3 others viewing this plan" for a single-user
  tool.

---

## REF-28: CLI Parity — Claude Code / Aider Muscle Memory

**Verdict: SHIP IT**

### What it proposes

Make `roko` (bare, no subcommand) the interactive entry point with
intent detection. Slash commands. Diff-first output with per-hunk
control. Workspace detection. Budget display. Tab completion.
Transcript importers.

### What actually exists

- `main.rs` (7,462 lines) has 40+ subcommands. When run without
  a subcommand, it currently tries to enter a default mode.
- `repl.rs` (232 lines) exists as a basic REPL.
- `chat.rs` (131 lines) is a bare polling REPL.
- No slash commands exist (zero hits for `SlashCommand` in the
  codebase).
- No diff-first output with per-hunk control exists.
- No workspace detection banner.
- Tab completion: clap supports `clap_complete` but it's unclear
  if it's wired.

### Honest assessment

This is the most immediately useful refinement. The gap between
"roko has 40+ working subcommands" and "a new user can be productive
in 60 seconds" is exactly the gap this doc addresses. And the
proposals are concrete, scope-bounded, and achievable.

The key insight is correct: users coming from Claude Code and Aider
have specific expectations. Meeting those expectations costs a month
of focused work and pays off in every subsequent interaction.

The useful subset (all of it, basically):
- Interactive `roko` entry with intent detection: YES.
- Slash commands (`/edit`, `/run`, `/undo`, `/plan`, `/explain`):
  YES. The `SlashCommand` trait proposed is clean.
- Diff-first output with per-hunk accept/reject: YES.
- Workspace detection banner: YES.
- Budget display in prompt: YES.
- Tab completion via clap_complete: YES, nearly free.
- Piped/CI mode with semantic exit codes: YES.

The nice-to-have:
- Claude Code / Aider transcript importers: cool but not urgent.
- Natural-language shortcut routing ("show me last failure"): the
  LLM already does this; explicit routing is unnecessary.
- `--record session.jsonl` with `--assert` replay: advanced, defer.

---

## REF-29: Web UI Architecture — Five-Page SvelteKit App

**Verdict: DEFER**

### What it proposes

A five-page SvelteKit web UI (Home, Chat, Plans, Beliefs, Settings)
with Tailwind, shadcn-svelte, CodeMirror, reactive stores synced
to StateHub projections, PWA with service worker, accessibility
audit, mobile responsive design, plugin extensibility.

### What actually exists

- `roko serve` exposes ~85 REST API routes, plus SSE and WebSocket.
  This is a real backend.
- No first-party web frontend exists. Zero HTML, zero JS, zero
  SvelteKit.
- No `@roko/ui` component library exists.

### Honest assessment

Building a SvelteKit web UI is a real product effort. A quarter of
focused work, the doc says. That is optimistic -- more like 3-4
months for one person to build, test, and polish five pages plus a
component library plus PWA plus accessibility.

The API backend already exists and is solid. But the question is:
who is the user? The CLAUDE.md says "0 external users." The doc
lists target users as PMs, managers, executives, mobile viewers,
demo audiences. These are speculative audiences for a tool that
currently self-hosts its own development.

If you need a web UI, the cheapest path is:
1. Use the existing API with a generic dashboard tool (Grafana,
   Retool, or a simple React page).
2. Build a single-page status dashboard, not five pages.

The useful subset:
- A single Home/Status page showing c-factor, active tasks, recent
  episodes, cost: YES, if there is demand. Could be done in a few
  hundred lines of HTML + JS consuming the existing SSE endpoint.

The scope-creep:
- Full SvelteKit with SSR, hydration, PWA, service worker.
- Component library with 9 reusable components.
- Plugin-contributed custom tiles and pages.
- CodeMirror and Tiptap editors.
- Mobile-specific layouts with touch targets.
- Deep-link semantics with expiring shared URLs.
- Storybook for component development.
- Lighthouse Performance >= 90 targets.
- Voice input button.

---

## REF-30: Rich UX Primitives

**Verdict: SIMPLIFY (subset is useful, most is premature)**

### What it proposes

Ten UX primitives: reasoning streams, tool-call banners, gate badges,
heuristic footnotes, uncertainty bars, replay scrubber, alternative
renderings, confidence-weighted aggregation, progressive disclosure,
spatial memory. Plus annotations as Engrams, explainability panel,
voice I/O, collaborative presence, keyboard registry.

### What actually exists

- TUI widgets: 14 widget files including `diff_panel.rs`,
  `task_progress.rs`, `plan_tree.rs`, `phase_compact.rs`,
  `wave_progress.rs`, `token_sparkline.rs`, `status_bar.rs`,
  `header_bar.rs`. These are real rendering components.
- Tool-call banners: the TUI already shows task/agent activity.
- Gate results: already rendered in the TUI via DashboardSnapshot.
- No heuristic footnotes exist.
- No uncertainty bars exist.
- No replay scrubber exists.
- No annotation system exists.
- No explainability panel exists.
- No voice I/O exists.
- No collaborative presence exists.

### Honest assessment

Several of these primitives are actually good ideas that would
differentiate Roko's UI from generic chat boxes:

1. **Heuristic footnotes** -- showing which heuristics influenced
   a decision is genuinely novel and useful. But it requires the
   heuristic system to track provenance per-response, which is
   partially wired.
2. **Tool-call banners** -- already partially exist in the TUI.
   Improving them is incremental.
3. **Gate badges** -- already exist in the TUI. Polish, not new
   work.
4. **Progressive disclosure** -- good design principle, already
   partially implemented via the TUI's tab structure.

The rest is premature:
- Replay scrubber with time-travel: requires substrate snapshots
  at each point, which don't exist.
- Confidence-weighted multi-agent aggregation: requires multiple
  agents answering the same question, which isn't how the system
  works today.
- Collaborative presence with cursors and live edits: for a
  single-user tool.
- Voice I/O and ambient sound design: no.
- Annotation system as Engrams: interesting but premature.
- Keyboard shortcut registry shared across TUI and web: the web
  UI does not exist.

---

## Cross-Cutting Observations

### 1. The audience problem

Every doc writes for multiple audiences: Rust developers, end users,
non-developer stakeholders, mobile users, team operators, enterprise
admins, edge deployers. Roko currently has ONE user profile: Will,
using it to develop itself. The docs design for an imaginary customer
base that doesn't exist.

### 2. The dependency chain is backwards

Docs 29 (web UI) and 30 (rich primitives) depend on 26 (StateHub)
and 27 (realtime surface). Docs 26 and 27 depend on 03 (Bus) and 02
(Engram). The dependency chain means you can't ship the UX story
without shipping the kernel story first. And the kernel story
(engrams, bus, projections) is itself a refactoring proposal, not
existing code. The existing StateHub works without any of these
abstractions.

### 3. "Wire, don't build" violations

The CLAUDE.md explicitly says: "WIRE, don't build. Before building
anything new, check if existing code just needs to be called." At
least half of these docs propose building entirely new systems:

- New SvelteKit web app (29)
- New client libraries in 3 languages (27)
- New gRPC transport (27)
- New proc macros (22)
- New cargo plugin (22)
- New plugin bundle format (25)
- New TypedContext + Custody primitives (25)
- New `roko-statehub` crate (26)
- New annotation system (30)
- New voice I/O (30)

Meanwhile, existing code that needs wiring:
- Chat REPL (131 lines, no streaming)
- Slash commands (zero)
- Interactive init (not wired)
- Tab completion (not wired)
- Budget display (not wired to CLI)

### 4. Time estimates are fantasy

The docs estimate ~6 months total (a quarter each for web UI and
primitives, plus weeks for each other piece). This is for one person.
The actual codebase is 177K lines of Rust across 29 crates. Each
"two weeks" estimate becomes a month when you factor in the existing
complexity, testing, and the need to not break what works.

---

## If You Can Only Do 3 of These 9, Which 3 and Why?

### 1. REF-28: CLI Parity (SHIP IT)

**Why**: This is the only doc that directly improves the experience
of the one real user (you) and any near-term user (someone trying
roko for the first time). Interactive entry, slash commands,
diff-first output, workspace detection, and tab completion are all
concrete, bounded, testable, and immediately useful. Every proposed
feature is a missing wire in the existing CLI, not a new system.
Estimated real effort: 3-4 weeks.

### 2. REF-26: StateHub (SHIP IT, but just hardening)

**Why**: StateHub already works. It already serves TUI, WS, SSE,
and REST. The useful work here is not building a new
`roko-statehub` crate with typed projections -- it's hardening
what exists. Add proper cursor tracking to the SSE endpoint.
Add reconnect-with-replay to the WS endpoint. Document the wire
format. Maybe split DashboardSnapshot into smaller logical groups.
This is 1-2 weeks of incremental improvement, not a rearchitecture.

### 3. REF-23: User UX, but only the chat + init subset (SIMPLIFY)

**Why**: `roko chat` is 131 lines of polling. `roko init` is
non-interactive. These are the two weakest points in the user
experience and they gate adoption. Fix chat to stream responses
and support basic slash commands. Make init interactive with model
detection. That's 2-3 weeks. Skip session export, i18n,
accessibility audits, undo, and session sharing.

### What to skip entirely

- **REF-22 (SDK)**: No external users to serve. The trait impls are
  already in roko-core.
- **REF-24 (deployment)**: Docker already works. Skip Helm, WASM,
  multi-tenancy, OIDC.
- **REF-25 (domain profiles)**: TypedContext is interesting but the
  profile bundle infrastructure is premature.
- **REF-27 (realtime)**: WS and SSE already work. Skip gRPC, client
  libraries, GraphQL.
- **REF-29 (web UI)**: No one is asking for it. The API is there
  when someone does.
- **REF-30 (rich primitives)**: The TUI already has decent widgets.
  Heuristic footnotes and the explainability panel are interesting
  ideas to keep in mind but not to build now.

### The honest prioritization

The self-hosting loop (items 10-11 in the CLAUDE.md priority list)
is more important than any of these 9 docs. Automatic plan
generation from PRDs, and feedback loops from failed gates back to
the planner, are the two things that make roko actually self-hosting.
Every UX refinement in this arc is less important than closing that
loop. Ship 28 and the chat/init subset of 23 because they make the
self-hosting loop more pleasant to operate. Skip everything else
until there is a second user.
