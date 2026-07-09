# Refinement Audit Runner — Batch AUD05

Run id: run-20260417-214125
Attempt: 1
Model: gpt-5.4
Reasoning: high

## Shared Context Pack

### 00-AUDIT-RULES

# Audit Application Rules

You are applying refinement-audit critiques to Roko's documentation and tooling.
The audit found that the refinements were "directionally correct but 5-10x overscoped."

## Core Principles

1. **The diagnosis is correct, the prescription was overscoped.** Ship what matters.
2. **Split "exists" from "planned."** Never describe unbuilt features in present tense.
3. **Narrow, don't delete.** Move overscoped content to "future work" sections.
4. **Fix factual errors.** Update LOC counts, route counts, crate counts, status labels.
5. **Reduce jargon inflation.** If a concept has 0 lines of code, it's a research hypothesis.

## Verdicts to Apply

- `keep` → Polish wording. Strengthen evidence. Keep it.
- `narrow` → Reduce scope. Add "aspirational" or "target-state" caveats.
- `defer` → Move to explicit future-work section with a clear label.
- `rewrite` → Reframe per the audit's specific guidance. Don't just edit — rethink.

## Factual Corrections (from codebase reality check)

- Total Rust LOC: 322,088 (not 177K)
- Workspace members: 36 (not 18)
- roko-serve routes: 200+ (not ~85)
- TUI: 58K LOC (wired, not "text-mode only")
- roko-learn: 42 modules, 35,847 LOC
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Pulse/Datum/Demurrage/Worldview/Custody: 0 lines of code each

## 5 Aspirational Concepts with 0 Code

These MUST be labeled as "target-state" or "planned" in docs, never described as existing:
1. Pulse (ephemeral event type)
2. Datum (medium polymorphism enum)
3. Demurrage (knowledge decay economic model)
4. Worldview (heuristic cluster)
5. Custody (chain-of-custody record)

### 01-PRIORITY-QUEUE

# Priority Queue

From the audit master summary — this is the recommended priority order.

## Ship Now (1-2 weeks total)

1. Add HDC fingerprint field to Engram — `roko-core/src/engram.rs` — 1 day
2. Unify event enums into `RokoEvent` — across 4 crates — 1 week
3. Add generic `Bus<E>` trait to roko-core — ~100 lines — 2-3 days
4. Clean up stale "Signal" references — traits.rs, README, kind.rs — 1 hour
5. Fix architecture INDEX status — `docs/00-architecture/INDEX.md` — 30 min

## Ship Soon (next month)

6. CLI parity / muscle memory (REF28)
7. StateHub hardening (REF26)
8. Heuristic calibration struct (REF14)
9. Safety: extend Attestation + expand taint (REF32)
10. Threat model doc (REF32 §13)

## Defer

- Pulse type, Datum enum, Operator generalization
- Demurrage, Plugin SPI tiers 4-5, 3 new kernel crates
- All 5 rewrite candidates, SvelteKit web UI, gRPC
- 12-month roadmap timeline

## Wrong (needs correction in docs)

- Synergy matrix (7/10 primitives don't exist)
- REF32 ignores existing safety system
- Glossary marks EventBus as "retired" (it's the only live transport)
- "Moat" framing (2/10 components exist fully)
- Doc INDEX says serve/TUI "not wired" (both definitively wired)

### 02-DOCS-TREE-MAP

# Docs Tree Map

The canonical documentation lives at `docs/`. Here is the full structure:

```
docs/
├── 00-architecture/        # 33+ files; kernel + trait system + analysis + design principles
├── 01-orchestration/       # Plan DAG, execution, plan runner
├── 02-agents/              # Agent dispatch, backends, sidecar
├── 03-composition/         # Prompts, context assembly, templates, budgets
├── 04-verification/        # Gates, validation, 7-rung pipeline
├── 05-learning/            # Self-learning loops, episodes, playbooks, experiments
├── 06-neuro/               # HDC, knowledge store, distillation, tier progression
├── 07-conductor/           # Event watchers, circuit breaker, diagnosis
├── 08-chain/               # On-chain primitives, ChainBus (Phase 2+)
├── 09-daimon/              # Behavior primitives (Phase 2+)
├── 10-dreams/              # Sleep-time compute, consolidation (Phase 2+)
├── 11-safety/              # Role auth, provenance, attestation, taint
├── 12-interfaces/          # CLI, HTTP API, TUI, Web UI, chat
├── 13-coordination/        # Stigmergy, coordination theory, c-factor
├── 14-identity-economy/    # Identity, economic models
├── 15-code-intelligence/   # Parser, indexing, HDC graphs
├── 16-heartbeat/           # Reactive/reflective loops, timing, CoALA mapping
├── 17-lifecycle/           # Agent lifecycle, shutdown
├── 18-tools/               # Tool system, plugin SPI
├── 19-deployment/          # Containers, orchestration, observability
├── 20-technical-analysis/  # Architecture audit, moat analysis, innovations
├── 21-references/          # Bibliography, research papers
├── INDEX.md                # Top-level index
├── STATUS.md               # Current wiring status
├── BENCHMARKS.md           # Performance data
└── CLI-REFERENCE.md        # Command documentation
```

## Key files you'll likely need to edit

- `docs/00-architecture/INDEX.md` — master architecture index (stale status claims)
- `docs/00-architecture/01-naming-and-glossary.md` — canonical glossary
- `docs/00-architecture/15-crate-map.md` — crate dependency graph
- `docs/00-architecture/31-implementation-readiness-audit.md` — readiness status
- `docs/INDEX.md` — top-level doc index
- `docs/STATUS.md` — current wiring status table

## What the refinements-runner already changed

The first pass (`tmp/refinements-runner/`) landed 35 batches (REF01-REF35) that introduced
new concepts (Pulse, Bus, Datum, demurrage, etc.) into the docs. Many of these concepts
have ZERO lines of code. The audit found that the docs now describe aspirational
architecture as if it exists. Your job is to fix that.

### 03-WORKSPACE-TOPOLOGY

# Workspace Topology

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/`.

## Crate map (36 workspace members)

| Crate | Path | LOC | Status |
|---|---|---|---|
| roko-core | `crates/roko-core/` | kernel | Stable — Engram + 6 traits + config + tools |
| roko-agent | `crates/roko-agent/` | large | 8 LLM backends, pools, MCP, tool loop, safety |
| roko-agent-server | `crates/roko-agent-server/` | medium | Per-agent HTTP sidecar, real LLM dispatch |
| roko-serve | `crates/roko-serve/` | 30K | HTTP control plane, 200+ routes, SSE, WebSocket |
| roko-orchestrator | `crates/roko-orchestrator/` | medium | Plan DAG, parallel executor, merge queue |
| roko-gate | `crates/roko-gate/` | medium | 11 gates, 7-rung pipeline, adaptive thresholds |
| roko-compose | `crates/roko-compose/` | medium | Prompt assembly, 9 templates, enrichment |
| roko-conductor | `crates/roko-conductor/` | medium | 10 watchers, circuit breaker, diagnosis |
| roko-learn | `crates/roko-learn/` | 36K | 42 modules: episodes, playbooks, bandits, routing, experiments |
| roko-cli | `crates/roko-cli/` | 17K+ | CLI binary + ratatui TUI (58K LOC total) |
| roko-fs | `crates/roko-fs/` | small | FileSubstrate (JSONL), GC, layout |
| roko-std | `crates/roko-std/` | medium | Defaults, 19 builtin tools, mock dispatcher |
| roko-runtime | `crates/roko-runtime/` | medium | ProcessSupervisor, event bus, cancellation |
| roko-primitives | `crates/roko-primitives/` | small | HDC vectors (10,240-bit), tier routing |
| roko-neuro | `crates/roko-neuro/` | medium | Durable knowledge store, distillation, tiers |
| roko-mcp-code | `crates/roko-mcp-code/` | medium | Code-intelligence MCP server |
| roko-index | `crates/roko-index/` | medium | Parser + graph + HDC indexing |
| roko-lang-* | `crates/roko-lang-*/` | small | Language support (rust, typescript, go) |
| roko-dreams | `crates/roko-dreams/` | small | Offline consolidation (Phase 2+) |
| roko-daimon | `crates/roko-daimon/` | small | Behavior primitives (Phase 2+) |
| roko-chain | `crates/roko-chain/` | small | Chain witness primitives (Phase 2+) |

## Key numbers (from codebase audit)

- Total Rust LOC: 322,088
- Workspace members: 36
- Test functions: 3,761
- orchestrate.rs: 17,087 lines
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Signal→Engram rename: 99.6% complete

## Concepts with 0 lines of code

These exist ONLY in docs, not in any crate:
- Pulse, Datum, Demurrage, Worldview, Custody
- roko-bus, roko-hdc (as separate crate), roko-spi
- Bus trait (as a formalized kernel trait)

### 04-DELEGATION-GUIDANCE

# Delegation Guidance

You are explicitly authorized to use multiple subagents for this batch.
Use them where it helps, but keep the immediate blocking work local.

## Required delegation behavior

- Before editing, form a short plan and identify 2-4 concrete subtasks.
- Spawn explorers for targeted codebase/docs reads and workers for bounded edits.
- Give each worker a disjoint write scope — no two workers edit the same file.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally without failing.

## Reading files

Before editing any file, READ IT FIRST. You are working in a git worktree
that contains the full repository. Use your file-reading capabilities to
inspect the current state of any file before modifying it.

## Phase-specific guidance

### Phase 1 (AUD* batches) — Docs only
- Only edit files under `docs/`. Never touch `crates/`, `tmp/`, or `src/`.
- Read the target docs before editing to understand their current state.
- The refinements-runner already made changes — you are refining those changes.

### Phase 2 (PU* batches) — Parity content refresh
- Only edit files under `tmp/docs-parity/NN/`.
- Read the current `docs/` tree first to understand what the audit pass changed.
- Update context-pack/, BATCHES.md, 00-INDEX.md, and all batch detail .md files.
- Update the run-docs-parity.sh script if its batch descriptions or verify
  commands reference stale content.

### Phase 3 (PE* batches) — Code execution
- Edit files under `crates/` to implement what the parity docs describe.
- Read BATCHES.md and 00-INDEX.md from the parity section FIRST.
- Search before writing: `grep -rn 'Name' crates/ --include='*.rs' | grep -v target/`
- Wire existing code — do not reimplement what already exists.
- Run `cargo check` after changes to verify compilation.

## Audit Source Files

These are the critique/triage documents that drive your edits.
Read them carefully — they contain specific verdicts (keep/narrow/defer/rewrite)
and codebase reality checks.

--- BEGIN 04-ux-audit.md ---

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

--- END 04-ux-audit.md ---

--- BEGIN 03-extensions-and-surfaces.md ---

# Extensions, Domain Profiles, And User Surfaces

## Extensions and modularity

### What is right

The strongest extensibility idea is simple:
- prefer low-power extension tiers first;
- make discovery local and boring;
- treat manifests and profile bundles as real user-facing leverage;
- clean up the crate graph where obvious seams are mixed today.

This part is good.

### What is overstated

The five-tier story becomes weaker as it moves up the power curve. The redesign
benefit is obvious for:
- prompt packs;
- profile bundles;
- manifest tools;
- MCP adapters.

It becomes much less obviously necessary for:
- stable native ABI commitments;
- WASM host abstractions;
- registry/network-effect strategy.

### Best near-term modularity sequence

1. Build local Tier 1/2/3 loading first.
2. Add a minimal real plugin command surface.
3. Ship one or two concrete external examples.
4. Clean internal crate seams after those extension points are real.
5. Only revisit native ABI and WASM host ideas if real usage demands them.

### What to demote

- registry/network-effect claims;
- stable ABI rhetoric before actual extension pressure exists;
- moat language built on extension surfaces rather than user value.

## Developer UX

### Strong core

The "time to first working agent" goal is good. The layered SDK story is also
good in principle:
- one-liner;
- builder;
- trait-level customization;
- runtime implementation boundary.

### Main problem

The four-layer Rust SDK story is good as a design target, but it risks becoming
too baroque if every layer gets too much bespoke machinery too early.

Better framing:
- desired SDK shape;
- minimum stable path first;
- sharper rules about what must be ergonomic vs what can stay advanced.

### What to keep

- typed errors at the public API surface;
- docs/examples discipline;
- example-driven onboarding;
- cargo-native ergonomics as an aspirational design constraint.

### What to narrow

- runtime-impl rhetoric before the underlying runtime interfaces are stable;
- macros and cargo subcommands unless they remove real friction;
- type and API layering that adds ceremony without reducing cognitive load.

## Domain profiles

### Strong part

Domain profiles are a sensible packaging abstraction:
- they bundle tools, roles, gates, starter heuristics, and defaults;
- they give a practical unit of adoption;
- they work well with low-power extension tiers.

### Risk

The risk is over-formalizing domain abstraction too early:
- `TypedContext` is promising, but could become a premature universal
  substrate;
- `Custody` is valuable, but should stay tightly tied to safety and audit
  semantics instead of becoming a generic catch-all object.

### Better framing

Treat domain profiles as:
- curated bundles first;
- shared typed context and custody expectations second;
- full typed domain kernels later.

## User UX and CLI parity

### What is genuinely useful

- one verb set across surfaces is a good target;
- better first-run onboarding is high leverage;
- diff-first review and visible approvals are good ideas;
- resumption, transcripts, and undo/replay should be first-class.

### What is misleading today

The risk is promising symmetry before the shared interaction model is clear.
The redesign should not aim for parity as aesthetic tidiness. It should aim for
parity where the user mental model is genuinely the same across surfaces.

### Better near-term UX sequence

1. Make one good interactive session surface real.
2. Expose stable session/resume/transcript mechanics.
3. Unify action names only after actual flows exist behind them.
4. Add slash commands and per-hunk review where the data model supports them.
5. Treat the browser as a read-first ops console before a full five-page rich
   app.

## StateHub, realtime, and web UI

### Strong part

StateHub is one of the most promising refinement directions because it gives
the redesign a clean place to put:
- projections;
- subscriptions;
- replayable state transitions;
- interface-friendly read models.

### Main risk

The redesign can overreach here by trying to standardize all transport,
projection, query, auth, replay, and UI semantics in one move. Better to keep
StateHub small and composable:
- projection contract;
- subscription contract;
- snapshot/replay contract;
- auth and tenancy layered around those contracts.

### Rich UX primitives

These are strongest when they are treated as rendering consequences of upstream
contracts.

Prioritize first:
- tool banners;
- gate badges;
- replay milestones;
- heuristic footnotes.

Be careful with:
- raw reasoning streams as a stable product primitive;
- confidence bars without calibrated semantics;
- "everything everywhere" UX promises before projection semantics are settled.

## Recommended rewrite principles for this area

1. Keep plugin and profile docs target-state, but narrower and more local-first.
2. Pull platform rhetoric back toward local-first, low-power extensibility.
3. Treat domain profiles as bundles before treating them as typed domain
   runtimes.
4. Rewrite user UX docs around the ideal user journey and only then decide
   which surface parity is genuinely needed.
5. Recast StateHub as a small projection kernel, not an all-at-once interface
   platform.
6. Make the first web story read-only and ops-facing before promising a richer
   full product surface.

--- END 03-extensions-and-surfaces.md ---

## Master Summary (reference)

# Refinements Audit — Master Summary

> **Date**: 2026-04-17 | **Auditor**: Claude Opus 4.6 (7 parallel agents)
> **Scope**: All 35 refinement docs + runner infrastructure + landed doc updates + codebase reality check
> **Output**: 7 detailed audits in this directory (01-foundation through 07-doc-quality)

---

## Executive Verdict

**The diagnosis is correct. The prescription is 5-10x overscoped.**

The refinements correctly identify real problems in the codebase (event enum proliferation, a conductor/learn layer violation, stale "Signal" naming, Policy signature mismatch). But they propose a 6-12 month, 5-7 engineer refactoring program for a single-developer project, introducing ~15 types that don't exist yet (Pulse, Datum, Bus trait, TopicFilter, Demurrage, Custody, Worldview, Claim, Paper, TypedContext, etc.) to solve problems that could be fixed in ~1-2 weeks with targeted changes.

---

## The 5 Things to Ship Now

These emerged consistently across all 7 audit workstreams as high-value, low-risk:

| # | What | Where | Effort | Why |
|---|---|---|---|---|
| 1 | **Add HDC fingerprint field to Engram** | `roko-core/src/engram.rs` | 1 day | HdcVector exists (10,240-bit, tested). Episode fingerprinting already works. This is the single highest-value bridge between the learning and memory layers. |
| 2 | **Unify event enums into `RokoEvent`** | Across 4 crates | 1 week | Four incompatible event enums (2x `AgentEvent`, `RokoEvent`, `ServerEvent`) is the real problem. Unify them. |
| 3 | **Add generic `Bus<E>` trait to roko-core** | `roko-core/src/traits.rs` | 2-3 days | ~100 lines. Keep it generic (not Pulse-specific). Solves the layer violation. |
| 4 | **Clean up stale "Signal" references** | traits.rs, README, kind.rs, CLAUDE.md | 1 hour | 40+ stale occurrences across docs and code comments. |
| 5 | **Fix architecture INDEX status** | `docs/00-architecture/INDEX.md` | 30 min | Says "roko-serve: HTTP API not wired" and "TUI: Text-mode dashboard only" — both factually wrong per CLAUDE.md and code (30K LOC serve, 58K LOC TUI). |

---

## The 5 Things to Ship Soon (next month)

| # | What | Source | Effort |
|---|---|---|---|
| 6 | **CLI parity / muscle memory (REF28)** | UX audit | 1-2 weeks |
| 7 | **StateHub hardening (REF26)** | UX audit | 1 week |
| 8 | **Heuristic calibration struct** | Learning audit (REF14) | 3-5 days |
| 9 | **Safety: extend Attestation + expand taint** | Integrator audit (REF32) | 1 week |
| 10 | **Threat model doc** | Integrator audit (REF32 §13) | 2 days |

---

## The 10 Things to Defer

| What | Why defer |
|---|---|
| **Pulse type** (REF02) | Unified `RokoEvent` enum solves the same problem more simply |
| **Datum enum** (REF04) | Premature abstraction; doubles every trait's surface area |
| **Operator generalization** (REF04) | Only Policy actually needs a signature change |
| **Demurrage** (REF12) | Add `last_used + access_count` to Decay first; skip the full economic model |
| **Plugin SPI tiers 4-5** (REF17) | Zero plugin authors exist. WASM host is premature |
| **3 new kernel crates** (REF20) | roko-bus justified, roko-hdc unnecessary (345 LOC), roko-spi premature |
| **All 5 rewrite candidates** (REF21) | Existing code works. Build incrementally |
| **SvelteKit web UI** (REF29) | Zero frontend code exists. Build when someone asks |
| **gRPC wire protocol** (REF27) | No tonic dependency. WebSocket + SSE already work |
| **12-month roadmap timeline** (REF35) | Calibrated for 5-7 engineers, not 1 developer + AI |

---

## The 5 Things That Are Wrong

| What | Issue | Source |
|---|---|---|
| **Synergy matrix** (REF31) | 7 of 10 "load-bearing primitives" don't exist in code. Matrix is aspirational fiction. | Integrator audit |
| **REF32 ignores existing safety system** | The AgentContract/AgentWarrant/Capability system already exists and works. REF32 proposes replacing it without acknowledging it. | Integrator audit |
| **Glossary marks EventBus as "retired"** | `EventBus<E>` is the only live transport code. No Bus trait or Pulse exists. | Integrator audit |
| **"Moat" framing** (REF18) | Of 10 claimed moat components, 2 exist fully, 2 partially, 6 not at all. The moat is aspirational. | Moat audit |
| **Doc INDEX says serve/TUI "not wired"** | serve has 200+ routes (30K LOC), TUI has 58K LOC with WebSocket. Both are definitively wired. | Doc quality + reality check |

---

## Codebase Reality (Key Numbers)

From the reality-check audit:

| What | Reality |
|---|---|
| Total Rust LOC | 322,088 (not 177K as CLAUDE.md says) |
| Workspace members | 36 (not 18) |
| Test functions | 3,761 |
| orchestrate.rs | 17,087 lines (the integration hairball) |
| roko-serve routes | 200+ (not ~85) |
| TUI code | 58K LOC |
| roko-learn modules | 42 modules, 35,847 LOC |
| Signal→Engram rename | 99.6% complete (4 real stragglers) |
| Event bus event types | Exactly 2 (PlanRevision, PrdPublished) |
| Demurrage in code | 0 lines |
| Pulse in code | 0 lines |
| Worldview in code | 0 lines |

---

## Doc Quality Assessment

Overall: **3.8 / 5**

**Good**: No copy-paste artifacts. Glossary is excellent. Synergy map and safety spine read as unified docs. Cross-references resolve.

**Issues**:
1. "Signal" still used in ~40 places across 8+ pre-existing docs
2. Target crates (roko-bus, roko-hdc, roko-spi) described in present tense as if they exist
3. Architecture INDEX has stale status information contradicting CLAUDE.md

---

## Per-Arc Summary

### Foundation (01-09): PARTIALLY AGREE
The diagnosis is correct. The prescription (Pulse, Datum, generalized operators, 7-step TickConfig) is overcomplicated. Fix: unify events, add generic Bus trait, update docs. ~1 week instead of 6-7 weeks.

### Learning (10-16): SIMPLIFY
The docs undercount what already exists. roko-learn has 42 modules and 36K LOC. HDC fingerprint field on Engram is the highest-value change. Demurrage/worldviews/replication-ledger are premature.

### Moat (17-21): DEFER/SKEPTICAL
Zero plugin authors, zero external users. The moat is aspirational. Plugin tier 3 (tool manifests) is useful later. Everything else waits.

### UX (22-30): Pick 3 of 9
Ship REF28 (CLI parity), REF26 (StateHub), and the chat/init subset of REF23. Defer the four-layer SDK, six domain profiles, SvelteKit UI, gRPC, and rich UX primitives.

### Integrators (31-35): Integrate code, not plans
The synergy matrix, glossary, and roadmap are plans connecting to plans. Ship: threat model, glossary (split into "exists" vs "planned"), dependency ordering. Reject: quarterly timeline, synergy matrix of unbuilt features.

---

## Recommended Priority Queue

For a single developer + AI agents:

1. **Close the self-hosting loop** (CLAUDE.md items 10-11: auto plan generation + feedback loop)
2. Ship the 5 "now" items above
3. Ship the 5 "soon" items above
4. Address ux-followup P0 items (67 items in `tmp/ux-followup/`)
5. Decompose `orchestrate.rs` (17K lines is the real tech debt)
6. Everything else goes into "when the system needs it"

---

## Audit Files

| File | What |
|---|---|
| `01-foundation-audit.md` | REF01-09 vs codebase (28K chars) |
| `02-learning-audit.md` | REF10-16 vs codebase (30K chars) |
| `03-moat-audit.md` | REF17-21 vs codebase (25K chars) |
| `04-ux-audit.md` | REF22-30 vs codebase (25K chars) |
| `05-integrator-audit.md` | REF31-35 vs codebase (23K chars) |
| `06-codebase-reality-check.md` | 10 factual claims verified (27K chars) |
| `07-doc-quality-audit.md` | Landed doc updates quality (18K chars) |

## Refinement Matrix (per-REF verdicts)

# Refinement Matrix

Legend:
- `keep`
- `narrow`
- `defer`
- `rewrite`

| Ref | Title | Verdict | Audit note |
|---|---|---|---|
| REF01 | critique one noun | `keep` | The diagnosis is real: transport is under-modeled and the kernel story is too storage-centric. |
| REF02 | Engram vs Pulse | `keep` | `Pulse` is a good transport noun if used to clarify the redesign rather than force a total renaming campaign. |
| REF03 | Bus as first class | `keep` | This is the strongest foundational follow-up: unify and formalize transport. |
| REF04 | operators generalized | `narrow` | Good local idea, bad universal law. Medium polymorphism should be proven operator by operator. |
| REF05 | loop retold | `keep` | Useful as a reference architecture for the redesign, but should guide migration rather than dictate every interface immediately. |
| REF06 | refactoring plan | `keep` | A phased migration plan is appropriate; keep it honest and code-first. |
| REF07 | naming | `narrow` | Good cleanup instinct, but not every proposed term should become top-level canon immediately. |
| REF08 | code sketches | `narrow` | Helpful as exploratory sketches; should not be confused with settled API design. |
| REF09 | phase-2 implications | `narrow` | Good future map, but it should stay downstream of core runtime wins instead of shaping the first redesign pass. |
| REF10 | self-learning loops | `keep` | Strong direction if centered on calibration, contradiction, and adaptation rather than runtime-wide active-inference doctrine. |
| REF11 | HDC substrate | `narrow` | Keep HDC for retrieval/clustering; defer broader semantic-consensus rhetoric. |
| REF12 | knowledge demurrage | `defer` | Interesting hypothesis, but too early to present as the governing memory model. |
| REF13 | c-factor | `defer` | Worth exploring as coordination health, not yet worthy of strong canonical treatment. |
| REF14 | worldview validation | `narrow` | Keep typed heuristics and contradiction tracking; defer full worldview/dissonance stack. |
| REF15 | exponential scaling | `defer` | Too much product-theory confidence for the current maturity level. |
| REF16 | research-to-runtime | `narrow` | Claim registry and provenance-backed defaults are promising; the full paper economy is premature. |
| REF17 | plugin extension architecture | `keep` | Tiered extensibility is the right platform direction if it stays local-first and resists premature ecosystem ambition. |
| REF18 | competitive moat | `defer` | Too much architecture-theater and future-ecosystem assumption. |
| REF19 | net-new innovations | `rewrite` | The catalog format oversells speculative pieces; convert to research hypotheses or remove. |
| REF20 | modularity composability | `keep` | Crate-boundary cleanup and clearer seams are real needs. |
| REF21 | from-scratch redesigns | `narrow` | Useful as a pressure test and cleanup lens, but dangerous as the default implementation mindset. |
| REF22 | developer UX rust | `keep` | Strong redesign target if the SDK is kept crisp and optimized for time-to-first-agent rather than feature taxonomy. |
| REF23 | user UX running agents | `keep` | Strong target-state direction if parity follows a real shared session model instead of surface symmetry for its own sake. |
| REF24 | deployment UX | `keep` | Strong operator-centered direction; needs stricter sequencing and fewer assumptions bundled into the first wave. |
| REF25 | domain-specific agents | `keep` | Domain profiles are a strong packaging abstraction as long as bundles stay ahead of universal type formalism. |
| REF26 | StateHub rearchitecture | `keep` | One of the best proposals. Evolve the existing dashboard hub into real projections. |
| REF27 | realtime event surface | `keep` | Unification is the right target, but the contract should stay small: events, replay, filters, subscriptions. |
| REF28 | CLI parity familiar workflows | `keep` | Familiar-first is right if parity is earned from shared workflow semantics rather than copied command names. |
| REF29 | web UI architecture | `keep` | A web surface is a good redesign goal if it starts as an ops console and grows from projection contracts. |
| REF30 | rich UX primitives | `narrow` | Some primitives are valuable, but only when supported by real shared state and telemetry contracts. |
| REF31 | synergy integration map | `defer` | Fine as internal coherence tooling; too grand as canonical architecture backmatter. |
| REF32 | safety sandbox provenance | `keep` | Strong direction if safety remains a compact enforceable spine rather than an all-at-once governance superstructure. |
| REF33 | observability telemetry | `keep` | Strong direction if the signal set stays operator-useful and avoids speculative overmodeling. |
| REF34 | glossary | `rewrite` | Keep one glossary, but split current canon from target-state proposals. |
| REF35 | consolidated roadmap | `rewrite` | Keep sequencing discipline, but narrow the number of simultaneous deep bets and remove unearned quarter-level certainty. |

## Aggregated view

### Clear keeps

- REF01
- REF02
- REF03
- REF05
- REF06
- REF10
- REF17
- REF20
- REF22
- REF23
- REF24
- REF25
- REF26
- REF27
- REF28
- REF29
- REF32
- REF33

### Strong, but should be narrowed

- REF04
- REF07
- REF08
- REF09
- REF11
- REF14
- REF16
- REF21
- REF30

### Better deferred

- REF12
- REF13
- REF15
- REF18
- REF31

### Need substantive rewrite

- REF19
- REF34
- REF35

## Practical consequence

The refinement set should not be treated as a monolithic "land it all" bundle.
The right next pass is:

1. Preserve the `keep` items.
2. Rewrite the `narrow` items around smaller scope and less doctrinal force.
3. Move the `defer` items into explicit future-work or research-hypothesis sections.
4. Rebuild the `rewrite` items so they stop acting as authority multipliers for
   architecture that is still too speculative or too overloaded.

# Batch AUD05: Narrow UX docs (REF22-30) — keep REF26/28/23, defer the rest

**Audit refs**: 04-ux-audit.md (full file), 05-refinement-matrix.md (REF22-30 rows).
Applies the audit's "pick 3 of 9" verdict to `docs/12-interfaces/` and
`docs/19-deployment/`.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/04-ux-audit.md` (full file -- verdict per REF22-30)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF22-30 rows)
- `tmp/refinements-audit/00-MASTER-SUMMARY.md` ("The 5 Things to Ship Soon" section)
- `docs/12-interfaces/INDEX.md`
- `docs/12-interfaces/19-rust-sdk-developer-ux.md`
- `docs/12-interfaces/21-user-ux-running-agents.md`
- `docs/12-interfaces/22-statehub-projection-layer.md`
- `docs/12-interfaces/23-rich-ux-primitives.md`
- `docs/12-interfaces/13-web-portal.md`
- `docs/12-interfaces/00-cli-overview.md`
- `docs/12-interfaces/01-cli-command-reference.md`
- `docs/12-interfaces/06-websocket-streaming.md`
- `docs/19-deployment/INDEX.md`
- `docs/19-deployment/14-observability-and-telemetry.md`

## Task

The refinements-runner wrote a four-layer Rust SDK, nine canonical verbs across
four surfaces, SvelteKit web UI, gRPC wire protocol, rich UX primitives, six
domain profiles, and detailed deployment shapes into the interface and deployment
docs. The audit says: keep REF28 (CLI parity), REF26 (StateHub), and the
chat/init subset of REF23. Defer everything else. Mark accordingly.

## Current state (evidence)

The audit found these specific issues:

1. **Four-layer Rust SDK (REF22)**: No `roko::run()` one-liner, no
   `Agent::builder()`, no proc macros (`#[tool]`, `#[gate]`), no
   `cargo roko` plugin, no runnable examples. The 6 kernel traits are the real
   extension surface. Audit verdict: **DEFER** -- designing for an audience
   that does not exist.

2. **Nine canonical verbs / four surfaces (REF23)**: CLI has ~40+ real
   subcommands. TUI has 22K lines. Chat is 131 lines (bare REPL). Web has 0
   first-party HTML. The verb unification conflates "clean up CLI flags"
   (weekend job) with "build a full multi-surface UX framework with i18n,
   undo, session export" (months). Audit verdict: **SIMPLIFY** -- keep
   chat/init improvements, defer the universal verb set.

3. **StateHub (REF26)**: One of the best proposals. Evolve the existing
   `StateHub` (broadcast channel in `roko-core/src/state_hub.rs`) into real
   projections. Audit verdict: **KEEP**.

4. **CLI parity (REF28)**: Familiar-first is right if parity is earned from
   shared workflow semantics. The CLI already has extensive commands. Audit
   verdict: **KEEP**.

5. **SvelteKit web UI (REF29)**: Zero frontend code exists. No SvelteKit, no
   HTML templates, no browser build. Audit verdict: **DEFER** -- build when
   someone asks.

6. **gRPC wire protocol (REF27)**: No tonic dependency. WebSocket + SSE already
   work in roko-serve. Audit verdict: **DEFER**.

7. **Rich UX primitives (REF30)**: Reasoning streams, uncertainty bars, replay
   scrubbers -- these depend on real shared state and telemetry contracts that
   do not exist. Audit verdict: **NARROW**.

8. **`docs/12-interfaces/INDEX.md`**: The overview is a 1,500-character sentence
   citing 8 REFs. The accretive citation problem is the worst in the tree.

9. **Deployment docs**: Five deployment shapes (laptop, single-server,
   container, clustered, edge) are well-structured but only "laptop" is tested.
   The cross-references use vague labels without links.

## Implementation

### 1. Mark Rust SDK as deferred

In `docs/12-interfaces/19-rust-sdk-developer-ux.md`:
- Add an implementation-status callout:
  `> **Implementation status**: The 6 kernel traits in `roko-core/src/traits.rs`
  > are the current extension surface. No `roko::run()` one-liner,
  > `Agent::builder()`, proc macros, or `cargo roko` plugin exist.
  > This doc describes a **deferred** SDK surface for future external users.
  > Near-term useful: better error types, `#[warn(missing_docs)]`, worked
  > examples.`

### 2. Narrow user UX doc

In `docs/12-interfaces/21-user-ux-running-agents.md`:
- Add an implementation-status callout:
  `> **Implementation status**: CLI (~40+ subcommands) and TUI (22K LOC,
  > ratatui) are **Shipping**. Chat (131 lines, bare REPL) is **minimal**.
  > Web (API-only, no first-party HTML) is **not started**.
  > Near-term: improve chat with streaming and slash commands, add
  > interactive `roko init`. The nine-verb universal surface is
  > **target-state**.`

### 3. Keep StateHub doc, add status

In `docs/12-interfaces/22-statehub-projection-layer.md`:
- Add a status note acknowledging `StateHub` already exists in `roko-core`
  as a broadcast channel, and this doc describes the target evolution into
  typed projections

### 4. Mark web UI as deferred

In `docs/12-interfaces/13-web-portal.md`:
- Add an implementation-status callout:
  `> **Implementation status**: **Deferred**. Zero frontend code exists.
  > No SvelteKit, no HTML templates, no browser build target. roko-serve
  > provides an API-only HTTP surface (200+ routes). This doc describes
  > a target-state web surface.`

### 5. Mark rich UX primitives as target-state

In `docs/12-interfaces/23-rich-ux-primitives.md`:
- Add an implementation-status callout marking these as dependent on
  StateHub projections and shared telemetry that do not yet exist

### 6. Fix the interfaces INDEX

In `docs/12-interfaces/INDEX.md`:
- Break the 1,500-character overview sentence into a structured paragraph
  or bulleted list
- Add a brief status summary distinguishing:
  - **Shipping**: CLI (40+ commands), TUI (22K LOC ratatui), HTTP API (200+
    routes in roko-serve)
  - **Minimal**: Chat (131 lines, bare REPL)
  - **Target-state**: Web UI, universal verb set, rich UX primitives, Rust SDK

### 7. Qualify deployment shapes

In `docs/19-deployment/INDEX.md`:
- Add a note that only the "laptop" shape is currently tested
- Fix vague cross-references (replace "Agent Types documentation, section 8"
  with actual relative links)

In `docs/19-deployment/14-observability-and-telemetry.md`:
- If it references Prometheus or OpenTelemetry as current infrastructure,
  add a note that no Prometheus endpoint or OTLP exporter exists yet
- Acknowledge the existing observability baseline: JSONL episode log,
  efficiency events, StateHub, tracing-based structured logs

### 8. Mark gRPC as deferred in websocket doc

In `docs/12-interfaces/06-websocket-streaming.md`:
- If gRPC is described as a wire protocol option, add a note:
  `> gRPC (tonic) is **deferred**. No tonic dependency exists. WebSocket
  > and SSE are the current realtime transports.`

## Write scope

- `docs/12-interfaces/INDEX.md`
- `docs/12-interfaces/19-rust-sdk-developer-ux.md`
- `docs/12-interfaces/21-user-ux-running-agents.md`
- `docs/12-interfaces/22-statehub-projection-layer.md`
- `docs/12-interfaces/23-rich-ux-primitives.md`
- `docs/12-interfaces/13-web-portal.md`
- `docs/12-interfaces/00-cli-overview.md` (only if it overstates)
- `docs/12-interfaces/01-cli-command-reference.md` (only if it overstates)
- `docs/12-interfaces/06-websocket-streaming.md`
- `docs/19-deployment/INDEX.md`
- `docs/19-deployment/14-observability-and-telemetry.md`

## Rules

1. **Mark, do not delete.** Deferred designs are useful target specs.
2. **Keep REF26 (StateHub) and REF28 (CLI parity) intact.** These are the
   audit's recommended keeps. Add status callouts but do not weaken them.
3. **Fix the INDEX readability.** The 1,500-character sentence is a P1 issue
   from the doc quality audit. Break it up.
4. **Do not touch architecture docs** -- those are AUD02's scope.
5. **Do not touch learning/neuro docs** -- those are AUD03's scope.
6. **Do not touch safety docs** -- those are AUD06's scope.
7. **Do not fix Signal->Engram references** -- that is AUD07's scope.
8. **Use real numbers.** "131 lines" is more useful than "minimal." "22K LOC"
   is more useful than "substantial."

## Done when

- Rust SDK doc is marked as deferred with near-term useful subset identified
- Web portal doc is marked as deferred
- Rich UX primitives doc is marked as target-state
- gRPC is marked as deferred wherever it appears
- Interfaces INDEX is readable (no 1,500-char sentences)
- Deployment INDEX has specific links instead of vague references
- Observability doc acknowledges existing baseline before proposing Prometheus/OTLP
- StateHub and CLI parity docs are preserved with status context added
- Final message lists every doc edited and the status tier assigned
