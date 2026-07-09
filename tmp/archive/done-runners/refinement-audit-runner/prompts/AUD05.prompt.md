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
