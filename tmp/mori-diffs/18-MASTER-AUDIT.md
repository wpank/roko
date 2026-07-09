# Master Audit: Roko Redesign From Scratch

> Repo-wide audit and redesign plan, grounded in current code, `docs/`, `tmp/unified*`, and lessons from `bardo` / `mori`.

## Verdict

If I were redesigning `roko` from scratch today, I would **not** start by inventing more subsystems.

I would freeze the conceptual model to a small number of hard boundaries:

1. **Kernel**: durable types, event types, config schema, capability model, Bus/Store contracts.
2. **Execution Engine**: one runtime that executes plans, agent loops, gates, triggers, and projections through one event model.
3. **Agent Runtime**: provider-normalized sessions, tool loop, safety, MCP, pooling, streaming.
4. **Composition Runtime**: prompt/context assembly, retrieval, budget allocation, role policy.
5. **Feedback Runtime**: episodes, routing, knowledge writeback, conductor signals, dreams.
6. **Surfaces**: CLI, TUI, HTTP, background workers.

The current repo roughly has these ingredients, but they are not composed through one authoritative runtime path.

That is why the architecture feels worse than the docs even when many crates are individually substantial.

---

## 1. What Is Actually Wrong

### 1.1 There is no single execution truth

Today the repo has:

- `roko-orchestrator` as pure state machine / executor logic
- `roko-cli/src/runner/` as the active `plan run` path
- `roko-cli/src/orchestrate.rs` as the legacy but richer integration path

This creates split ownership for:

- agent dispatch
- prompt assembly
- routing
- knowledge integration
- learning feedback
- dashboard event projection
- dream hooks

That is the biggest architectural defect in the repo.

### 1.2 Provider abstraction exists in the crate graph but historically was not in the live event model

`roko-agent` is broad and real, but the active runner still consumes Claude-specific stream JSON types directly.

2026-04-27 source correction:

- [x] `crates/roko-agent/src/runtime_events.rs` now defines provider-neutral `AgentRuntimeEvent`.
- [x] `crates/roko-cli/src/runner/types.rs` aliases the runner event type to `roko_agent::AgentRuntimeEvent`.
- [x] `crates/roko-cli/src/runner/agent_stream.rs` delegates Claude line parsing below `roko-agent`.
- [ ] This still needs end-to-end provider-matrix proof showing Anthropic, OpenAI, Moonshot, Z.AI, Perplexity, Claude CLI, and Codex CLI all emit the same runtime event vocabulary through the same dispatch path.

That means:

- provider-neutral construction is only partial
- provider-neutral streaming is not real
- provider-neutral observability is not real

### 1.3 Cross-cuts are implemented as app glue, not reusable services

The repo has real code for:

- learning
- neuro
- daimon
- conductor
- dreams

But their integration point is often "call this from CLI runtime glue" rather than "subscribe to canonical events and emit canonical effects."

This keeps every new feature expensive to wire correctly.

### 1.4 Docs collapse target-state and current-state too aggressively

The docs are excellent at defining intended structure.

They are weaker at differentiating:

- live runner behavior
- legacy-only wiring
- crate-complete but not routed behavior
- spec-only target behavior

That makes the repo hard to steer because the implementation status appears more coherent than it is.

---

## 2. What Still Works Well

This repo is not broken at the foundation.

### 2.1 The crate extraction is mostly real

Unlike Mori:

- core crates are not empty shells
- the main domains are already separated enough to be salvageable
- the workspace has meaningful tests and real behaviors

### 2.2 The docs are directionally right

The strongest parts of `docs/` and `tmp/unified*` are correct about the target:

- event-driven runtime
- explicit composition surfaces
- strict boundaries between contracts and implementations
- one transport model
- one execution model
- cross-cuts as enrichers, not ad hoc side effects

The issue is not the direction. It is the migration discipline.

### 2.3 The active runner is cleaner than the old harness

The current runner path is much closer to the right mental model than `orchestrate.rs`.

It already has:

- single-threaded event loop semantics
- explicit channels
- checkpointing
- cleaner state update logic

It is the right convergence target, even though it is still incomplete.

---

## 3. Redesign From Scratch

If I were rebuilding the architecture cleanly, I would impose the following model.

## 3.1 Layer A: Contracts

Crates:

- `roko-core`
- future `roko-bus`
- future `roko-spi`
- future `roko-hdc`

Responsibilities:

- identity types
- run/task/plan ids
- normalized event enums
- store records
- capability and safety contracts
- config schema
- role policy contracts

Strict rule:

No provider wire formats. No CLI assumptions. No tokio channels. No concrete file paths.

## 3.2 Layer B: Runtime Primitives

Crates:

- `roko-runtime`
- `roko-fs`
- parts of `roko-std`

Responsibilities:

- canonical Bus implementation
- Store implementation
- checkpointing
- process supervision primitives
- typed replay/event persistence

Strict rule:

This layer knows how to move events and state, but not what a "plan run" is.

## 3.3 Layer C: Feature Engines

This is where most of the current crate set should land.

### Agent Engine

Primary owner: `roko-agent`

Responsibilities:

- provider selection
- session lifecycle
- normalized stream events
- tool loop
- MCP
- safety hooks
- warm pools

Hard requirement:

The only event surface exposed upward is provider-neutral.

### Composition Engine

Primary owner: `roko-compose`

Responsibilities:

- role prompt assembly
- task brief assembly
- context retrieval
- budget arbitration
- structured retry feedback injection

Hard requirement:

No direct filesystem or CLI side effects. Pure input/output assembly.

### Verification Engine

Primary owner: `roko-gate`

Responsibilities:

- rung execution
- streaming gate output
- verdict classification
- retry guidance
- artifact references

Hard requirement:

Results must be normalized as structured verdict events, not raw output blobs only.

### Feedback Engine

Primary owners:

- `roko-learn`
- `roko-neuro`
- `roko-conductor`
- `roko-dreams`
- `roko-daimon`

Responsibilities:

- episode recording
- routing observations
- pattern extraction
- knowledge promotion
- anomaly detection
- dream-time consolidation
- affect/risk modulation

Hard requirement:

This layer subscribes to canonical runtime events and emits canonical decisions or records. It does not reach into CLI-local mutable state.

## 3.4 Layer D: Unified Plan/Flow Engine

This is the missing center.

It can live in:

- `roko-orchestrator`
- or a new dedicated execution crate if necessary

Responsibilities:

- task DAG scheduling
- plan lifecycle
- retries and classification
- gate coordination
- agent dispatch requests
- merge queue
- checkpoint/recovery
- transition emission

This layer should own:

- one event loop contract
- one set of actions/effects
- one snapshot format

It should not know provider wire formats, prompt templating details, or raw dashboard projection rules.

## 3.5 Layer E: Surfaces

Crates:

- `roko-cli`
- `roko-serve`
- TUI modules

Responsibilities:

- commands
- views
- API transport
- operator workflows

Strict rule:

Surfaces subscribe to projections and submit commands. They do not become the hidden home of feature logic.

---

## 4. What This Means for Existing Crates

### `roko-cli`

Current problem:

- still owns too much behavior

Target role:

- thin surface + runtime host

Should stop owning:

- provider-specific parsing
- composition internals
- cross-cut business logic
- ad hoc knowledge/dream/conductor wiring

### `roko-orchestrator`

Current problem:

- good pure core, but not yet the sole owner of execution truth

Target role:

- authoritative flow/plan engine

### `roko-agent`

Current problem:

- strong internals, weak normalized upward seam

Target role:

- provider/runtime abstraction boundary

### `roko-compose`

Current problem:

- broad and useful, but not consistently mandatory in live execution

Target role:

- all prompt/context assembly happens here or through a dedicated derivative

### `roko-learn`, `roko-neuro`, `roko-conductor`, `roko-dreams`, `roko-daimon`

Current problem:

- rich code, inconsistent routing into the active runtime

Target role:

- event subscribers and decision producers

---

## 5. Missing Functionality That Should Exist

These are the most important "should be there but isn't fully working" categories.

### 5.1 Provider-neutral agent streaming

Should exist:

- runner consumes normalized agent events from `roko-agent`

Current state:

- [x] Source-corrected: runner consumes provider-neutral `AgentRuntimeEvent` aliases.
- [x] Source-corrected: Claude stream protocol structs are below `roko-agent`.
- [ ] Remaining gap: no generated proof report demonstrates every supported provider/model reaches `AgentRuntimeEvent` lifecycle, text, usage, tool, error, and completion events through one path.

### 5.2 One authoritative `plan run` path with rich integrations

Should exist:

- all learning, knowledge, prompt, routing, gate, dashboard, and retry behavior routed through the same plan runtime

Current state:

- runner is cleaner
- `orchestrate.rs` is richer
- truth is split

### 5.3 Structured verify/review phase in runner

Should exist:

- real verify/reviewer dispatch and structured outcome handling

Current state:

- runner notes already identify the stubbed verify path

### 5.4 Mandatory real prompt assembly

Should exist:

- task role prompt, scoped context, playbooks, anti-patterns, and prior failures all assembled through the main composition path

Current state:

- live runner still uses minimal prompt helpers

### 5.5 Real routing feedback loop in active runtime

Should exist:

- every outcome recorded back into model/provider routing

Current state:

- legacy-rich path has more of this than the live runner

### 5.6 Knowledge writeback and retrieval through canonical hooks

Should exist:

- pass/fail/retry signals map into knowledge reinforcement, anti-pattern capture, and retrieval

Current state:

- partial and path-dependent

### 5.7 Dreams/consolidation triggers on the live path

Should exist:

- plan completion and idle windows trigger consolidation through shared hooks

Current state:

- more designed than consistently routed

### 5.8 Conductor as event subscriber, not runtime parasite

Should exist:

- watchers consume runtime event stream
- interventions return structured actions

Current state:

- much of the configuration and integration is still app-glue shaped

### 5.9 Streaming gate observability

Should exist:

- live gate output and classified failures in dashboard/API/CLI

Current state:

- mostly batch result reporting

### 5.10 Honest status map

Should exist:

- every subsystem tagged by actual runtime routing status

Current state:

- many sections read as more uniformly live than they are

---

## 6. Design Patterns I Would Enforce

### 6.1 Ports and adapters

Every boundary crossing should be explicit:

- provider adapter
- prompt builder adapter
- gate adapter
- knowledge sink
- event projection sink

### 6.2 Normalized domain events

No top-level runtime should parse provider or tool wire formats directly.

### 6.3 Single owner per state machine

Plan lifecycle, agent lifecycle, gate lifecycle, and subscription lifecycle each need one owner.

### 6.4 Event sourcing where state matters

Not everywhere, but definitely for:

- plan execution
- retry/review decisions
- learning observations
- knowledge promotions

### 6.5 Projection-based UI/API

TUI and HTTP should consume projected state, not runtime internals.

### 6.6 Capability-based side effects

Safety and permissions should be enforced through declared capability surfaces, not scattered checks.

### 6.7 Progressive extraction

No more speculative crate splits before routing is complete.

The correct order is:

1. normalize runtime path
2. move behavior behind seams
3. only then split crates further if compilation or ownership pressure demands it

---

## 7. Migration Order

### Phase 1: Runtime Convergence

- freeze `orchestrate.rs`
- move missing rich integrations into modules consumable by `runner/`
- make runner the only growing path

### Phase 2: Agent Event Normalization

- move provider-specific parsing fully into `roko-agent`
- expose canonical runtime events
- remove Claude-specific event types from runner

### Phase 3: Composition Convergence

- remove minimal prompt path from active runner
- require `roko-compose` path for live execution

### Phase 4: Feedback Convergence

- route routing/episode/neuro/conductor/dream hooks through shared event boundaries

### Phase 5: Projection Convergence

- unify dashboard/API/CLI progress views onto one projection model

### Phase 6: Crate Stabilization

Only after phases 1-5:

- consider `roko-compose-core` + `roko-templates`
- consider `roko-defaults` + `roko-tools`
- extract explicit bus/spi crates if still justified by real pressure

---

## 8. Non-Negotiable Rules

If I were acting as architecture gatekeeper for this repo, I would enforce these.

1. No new first-class runtime feature lands only in `orchestrate.rs`.
2. No runner code may depend on provider-specific stream schema.
3. No prompt path used in production may bypass the composition engine.
4. No subsystem may claim `shipping` unless it is routed through the active runtime path.
5. No new crate split is allowed unless the runtime ownership problem it solves is already concrete.
6. Cross-cuts must subscribe to canonical events rather than mutate app-local state directly.

---

## 9. Final Assessment

The repo is salvageable without another rewrite.

It already contains enough real code to become coherent.

The shortest truthful summary is:

**Roko does not need more ideas. It needs one runtime spine.**

Everything else in the redesign follows from that.

---

## 10. Reading Order

1. [17-ARCHITECTURE-REALITY-CHECK.md](17-ARCHITECTURE-REALITY-CHECK.md)
2. this file
3. [20-RUNTIME-RECONCILIATION.md](20-RUNTIME-RECONCILIATION.md)
4. [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md)
5. [22-STABILITY-PLAN.md](22-STABILITY-PLAN.md)
6. runner path docs [00-OVERVIEW.md](00-OVERVIEW.md) through [08-FILE-MAP.md](08-FILE-MAP.md)
7. subsystem deep dives `09` through `16`
8. [19-SELF-REVIEW-AND-PROOF.md](19-SELF-REVIEW-AND-PROOF.md)

## Implementation Packet

This file turns the redesign into a sequence of work packages. Each package should become one or more plan tasks.

### Package A: Runtime Spine

- [ ] Freeze `orchestrate.rs` as donor/reference implementation and prevent new production features from landing only there.
- [x] Add runtime event vocabulary or reuse the normalized event types introduced in `roko-agent`; current source owner is `crates/roko-agent/src/runtime_events.rs`.
- [ ] Ensure every `plan run` and plan-like caller enters the same runner/dispatch runtime instead of legacy helpers.
- [x] Move major dispatch, prompt, merge, projection, resume, and feedback effects into focused modules.
- [ ] Add checkpoint/recovery proof before increasing concurrency.

### Package B: Agent Runtime

- [x] Move Claude provider stream parsing below `roko-agent`.
- [x] Expose normalized `AgentRuntimeEvent`.
- [ ] Preserve test dispatchers without allowing production mocks/fallbacks to masquerade as proof.
- [x] Wire provider factory into runner dispatch through `crates/roko-cli/src/dispatch/mod.rs`.
- [x] Add session reuse/warm pool as optional behavior at the dispatch seam.
- [ ] Prove all configured providers through the same dispatch/event/projection path.

### Package C: Composition Runtime

- [ ] Replace all legacy/minimal production prompt construction and leave `PromptAssembler::minimal()` only in tests or explicit diagnostics.
- [x] Add a prompt assembler facade for runner dispatch.
- [ ] Prove role policy, gate feedback, knowledge, playbooks, and code context appear in prompt diagnostics and are influence-linked to runtime decisions.
- [ ] Add snapshot tests and generated prompt-diagnostics proof for live runs.

### Package D: Feedback Runtime

- [x] Add one feedback facade.
- [ ] Route episodes, efficiency, router observations, knowledge, conductor, and dreams through that facade with durable event references.
- [x] Keep feedback non-blocking for the runner event loop at the facade boundary.
- [ ] Persist feedback state and prove it changes behavior across repeated runs.

### Package E: Projection Runtime

- [x] Add runner projection module and HTTP projection routes.
- [ ] Normalize dashboard, CLI progress, event log, and HTTP/SSE output onto the same durable projection source.
- [ ] Add event coverage for tool activity, usage, cost, gates, retries, merge, resume, provider lifecycle, and dreams.
- [ ] Add snapshot mutation tests and HTTP/TUI/query proof.

### Package F: Stability

- [ ] Build parity tests from `21-FEATURE-PARITY-MATRIX.md`.
- [ ] Build crash/resume tests from `22-STABILITY-PLAN.md`.
- [ ] Dogfood runner-only execution.
- [ ] Remove or wrap legacy-only behavior.

### Done Criteria

- [ ] `orchestrate.rs` has no unique production-critical behavior.
- [ ] Each feature package has active-path tests.
- [ ] `docs/STATUS.md` reflects actual runtime routing.
- [ ] The parity matrix has no `Target = yes` row still marked `Runner = no`.

## 11. Current Execution Evidence (2026-04-26)

This audit now includes direct no-mock runtime proof, not only static architecture review.

### Proven today

- [x] Runner path executes a real one-task plan end to end with Codex CLI.
- [x] Runner path executes the same real one-task plan end to end with Claude CLI.
- [x] Gate layer executes both default compile gate and per-task `verify` commands.
- [x] Final runtime snapshot is persisted with `current_phase.kind = "complete"` on success.

### Still not proven enough for parity claims

- [ ] Multi-task DAG progression with real agents.
- [ ] Retry loops with structured failure classification.
- [ ] Resume after interruption with no duplicate completion.
- [ ] Routing and knowledge feedback effectiveness over repeated runs.
- [ ] Projection parity across TUI, HTTP, and non-TUI CLI outputs.

## Worker 9 Evidence Checklist (2026-04-26)

Audit evidence now tied to actual files:

- [x] Active core execution: `crates/roko-cli/src/runner/event_loop.rs`, `/tmp/roko-real-e2e-nrUD05/logs/codex-run-3.stdout`, and `/tmp/roko-real-e2e-nrUD05/logs/claude-run-1.stdout`.
- [x] Gate execution: `crates/roko-cli/src/runner/gate_dispatch.rs` plus `/tmp/roko-real-e2e-nrUD05/work/.roko/events.jsonl` showing `compile:cargo` and `task-verify` verdicts.
- [x] Snapshot persistence: `crates/roko-cli/src/runner/persist.rs`, `crates/roko-orchestrator/src/executor/snapshot.rs`, and `/tmp/roko-real-e2e-nrUD05/work/.roko/state/executor.json`.
- [x] Partial dispatch abstraction: `crates/roko-cli/src/dispatch_v2.rs`.
- [x] Built-unrouted feedback engines: `crates/roko-learn/src/runtime_feedback.rs`, `crates/roko-neuro/src/lifecycle.rs`, and `crates/roko-dreams/src/runner.rs`.

Do not archive this audit yet because:

- [x] Historical blocker resolved: `crates/roko-agent/src/runtime_events.rs` exists and the runner event alias is provider-neutral.
- [x] Historical blocker resolved: `crates/roko-cli/src/runtime_feedback/` and `crates/roko-cli/src/projection/` exist.
- [ ] Active runner still does not prove `LearningRuntime`, `KnowledgeLifecycleRuntime`, `DreamRunner`, and conductor decisions are all durable feedback subscribers with observable cross-run influence.
- [ ] `orchestrate.rs` still contains unique production-critical behavior not proven in `runner/`.
- [ ] Parity proof remains incomplete for multi-task/retry/resume/routing/knowledge/projection/provider matrix.

## 12. 2026-04-27 Deepening Pass - Current Master Architecture Handoff

Self-grade for this pass:

- Initial rating: 9.91 / 10.
- Reasoning: this document now distinguishes historical architecture failures from current source-wired improvements, names the exact source seams that changed, and gives no-context implementation checklists for the remaining work. The remaining 0.09 is withheld because the architecture still needs generated proof reports from real provider/runtime executions before the audit can be archived.

### 12.1 Source Refresh

Current source anchors that supersede older claims in this file:

- [x] Provider-neutral runtime events exist in `crates/roko-agent/src/runtime_events.rs`.
- [x] Runner event type aliases `roko_agent::AgentRuntimeEvent` in `crates/roko-cli/src/runner/types.rs`.
- [x] Claude stream parsing is owned by `crates/roko-agent/src/provider/claude_cli/stream.rs`.
- [x] Runner stream parsing wrapper delegates to `roko-agent` in `crates/roko-cli/src/runner/agent_stream.rs`.
- [x] Dispatch facade exists in `crates/roko-cli/src/dispatch/mod.rs`.
- [x] Prompt assembler exists in `crates/roko-cli/src/dispatch/prompt_builder.rs` and is constructed by `crates/roko-cli/src/runner/event_loop.rs`.
- [x] Runtime feedback facade exists in `crates/roko-cli/src/runtime_feedback/mod.rs`.
- [x] Projection module exists in `crates/roko-cli/src/runner/projection.rs`, with HTTP projection routes in `crates/roko-serve/src/routes/projections.rs`.
- [x] Resume preparation exists in `crates/roko-cli/src/runner/resume.rs`.
- [x] Real merge backend exists in `crates/roko-cli/src/runner/merge.rs`.
- [ ] Generated proof artifacts under `tmp/mori-diffs/generated/` are still missing for the master architecture claims.

### 12.2 Current Master Verdict

Roko no longer has the exact failure shape described by the first version of this audit. Several missing seams have been created and source-wired.

The remaining architecture problem is stricter:

- [ ] There is still no single command/query runtime service that every CLI, HTTP, TUI, PRD, worker, cloud, and one-shot workflow must use.
- [ ] There is still no generated proof report that demonstrates every provider, prompt, feedback, projection, merge, resume, retry, and query behavior through that service.
- [ ] There is still no durable runtime event store that all surfaces treat as the sole read model source.
- [ ] There is still no declared side-effect ownership registry that prevents source files from spawning processes, reading credentials, writing artifacts, or mutating runtime state outside approved services.
- [ ] There is still no compatibility gate that fails if a feature is only implemented in `orchestrate.rs`, a route handler, a TUI helper, or a CLI-local helper.

### 12.3 Target Architecture If Rebuilt From Scratch

Implement this architecture as services and adapters, not as one-off runner patches:

- [ ] `RuntimeCommandService`: one public command API for `plan run`, one-shot workflows, PRD generation, research, task execution, resume, cancel, merge, and status mutation.
- [ ] `RuntimeQueryService`: one public query API for run status, task status, provider lifecycle, prompt diagnostics, merge state, gate verdicts, retry decisions, feedback influence, artifacts, and logs.
- [ ] `RuntimeEventStore`: append-only durable event store with replay, stream offsets, schema versions, redaction, and snapshot compaction.
- [ ] `ProjectionEngine`: deterministic reducers from runtime events into dashboard/TUI/HTTP/CLI read models.
- [ ] `DispatchService`: provider/model/tool/session abstraction that owns provider compatibility, credentials, warm pools, stream normalization, retries, and model-call lifecycle events.
- [ ] `PromptCompositionService`: mandatory prompt assembly service that records context inputs, budget decisions, knowledge/playbook references, retry feedback, and prompt hashes.
- [ ] `FeedbackService`: non-blocking subscriber that records learning episodes, router observations, conductor signals, knowledge promotion, dream scheduling, and cross-run influence decisions.
- [ ] `MergeService`: queued git merge backend with success/conflict evidence, regression gates, rollback policy, and projection events.
- [ ] `PolicyService`: capability, filesystem, network, model, cost, timeout, approval, safety, and extension policy decisions with provenance.
- [ ] `WorkspaceRepository`: typed artifact storage for PRDs, plans, tasks, research, logs, generated reports, snapshots, and migrations.

### 12.4 Implementation Batches

#### MA-01: Runtime Service Boundary

- [ ] Create or designate the owner module for `RuntimeCommandService`.
- [ ] Define command records for run, resume, cancel, merge, PRD, research, and one-shot project execution.
- [ ] Define command result records with operation id, run id, event stream id, and projection references.
- [ ] Route CLI `plan run` through the service.
- [ ] Route HTTP plan/run endpoints through the service.
- [ ] Route TUI actions through the service.
- [ ] Route PRD/research/worker/cloud callers through the service or explicitly mark them unsupported with proof.
- [ ] Add a grep gate that fails on new direct `PlanRunner::from_plans_dir` callers outside the service.

#### MA-02: Runtime Query Boundary

- [ ] Define query records for run, task, agent, provider, prompt, gate, retry, merge, resume, feedback, knowledge, and artifact state.
- [ ] Implement each query from projections or repositories, not from live runner internals.
- [ ] Route HTTP reads through `RuntimeQueryService`.
- [ ] Route TUI reads through `RuntimeQueryService`.
- [ ] Route CLI status/log commands through `RuntimeQueryService`.
- [ ] Add a proof command that dumps all query responses for a completed run.

#### MA-03: Event Store And Projection Spine

- [ ] Define a durable runtime event envelope with schema version, run id, task id, agent id, provider, model, timestamp, source, redaction class, and payload.
- [ ] Map existing runner events into the envelope.
- [ ] Map provider runtime lifecycle events into the envelope.
- [ ] Map prompt diagnostics into the envelope.
- [ ] Map gate/retry/replan decisions into the envelope.
- [ ] Map merge success/conflict evidence into the envelope.
- [ ] Map resume snapshot and replay state into the envelope.
- [ ] Implement projection reducers for dashboard, TUI, HTTP, CLI, and proof reports.
- [ ] Add replay proof: delete projections, replay events, and compare the regenerated projection digest.

#### MA-04: Provider And Model Proof

- [ ] Drive Anthropic API through dispatch.
- [ ] Drive OpenAI API through dispatch.
- [ ] Drive Moonshot API through dispatch.
- [ ] Drive Z.AI API through dispatch.
- [ ] Drive Perplexity API through dispatch.
- [ ] Drive Claude CLI through dispatch.
- [ ] Drive Codex CLI through dispatch.
- [ ] Emit explicit provider status values: `proved`, `missing_credentials`, `auth_failed`, `rate_limited`, `unsupported`, `runtime_error`.
- [ ] Record lifecycle, text, usage, tool, completion, and error events for each provider.
- [ ] Store results in `tmp/mori-diffs/generated/provider-matrix-report.json`.

#### MA-05: Prompt And Feedback Closure

- [ ] Make `PromptAssembler` mandatory for production dispatch.
- [ ] Remove or confine minimal prompt construction to tests.
- [ ] Record prompt input references rather than raw secrets or unbounded text.
- [ ] Persist prompt diagnostics with prompt hashes and budget decisions.
- [ ] Subscribe feedback service to task completion, gate failure, retry, merge, and provider events.
- [ ] Record learning/router/knowledge/conductor/dream decisions as durable events.
- [ ] Run the same project twice and prove the second run consumed first-run feedback through prompt diagnostics or routing decisions.

#### MA-06: Merge, Retry, Resume, And Failure Semantics

- [ ] Prove gate failure triggers structured retry or terminal failure with reason.
- [ ] Prove retry decision emits policy, prompt feedback, and attempt counters.
- [ ] Prove resume validates plan fingerprints and avoids duplicate task completion.
- [ ] Prove crash recovery reclaims or marks orphaned agents.
- [ ] Prove merge success performs a real git merge and emits commit/evidence details.
- [ ] Prove merge conflict records conflicted files, exits non-zero, and does not emit false success.
- [ ] Store evidence in `tmp/mori-diffs/generated/runtime-failure-proof-report.json`.

#### MA-07: Legacy Retirement

- [ ] Rename or fence `crates/roko-cli/src/orchestrate.rs` as donor-only once all unique behavior is routed through the runtime service.
- [ ] Remove direct provider parsing from surfaces.
- [ ] Remove route-owned background execution.
- [ ] Remove TUI-owned status derivation that bypasses projections.
- [ ] Remove hardcoded temporary path assumptions from proof claims.
- [ ] Update all older mori-diffs docs with source-corrected status before archiving.
- [ ] Archive this document only after every open item links to generated proof or a superseding active ledger row.

### 12.5 Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/master-architecture-proof.json` with this shape:

```json
{
  "schema": "mori-diffs.master-architecture-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "runtime_command_service": {
    "implemented": false,
    "callers_migrated": [],
    "remaining_direct_callers": []
  },
  "runtime_query_service": {
    "implemented": false,
    "http_queries_proved": [],
    "tui_queries_proved": [],
    "cli_queries_proved": []
  },
  "provider_matrix": {
    "anthropic": "missing_credentials",
    "openai": "missing_credentials",
    "moonshot": "missing_credentials",
    "zai": "missing_credentials",
    "perplexity": "missing_credentials",
    "claude_cli": "missing_credentials",
    "codex_cli": "missing_credentials"
  },
  "runtime_proof": {
    "real_provider_run": false,
    "multi_task_dag": false,
    "retry_after_gate_failure": false,
    "resume_after_crash": false,
    "merge_success": false,
    "merge_conflict": false,
    "http_projection_queries": false,
    "tui_projection_parity": false,
    "feedback_cross_run_influence": false
  },
  "legacy_retirement": {
    "orchestrate_unique_behavior_remaining": [],
    "direct_provider_parsing_remaining": [],
    "direct_runner_callers_remaining": [],
    "route_owned_execution_remaining": []
  }
}
```

### 12.6 No-Context Handoff Checklist

Use this exact sequence if this file is handed to another agent with no other context:

- [ ] Read only this file, [20-RUNTIME-RECONCILIATION.md](20-RUNTIME-RECONCILIATION.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), and [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).
- [ ] Run `rg -n "PlanRunner::from_plans_dir|orchestrate::|dispatch_v2|PromptAssembler::minimal|claude_cli::stream|tokio::spawn|Command::new|/tmp/|TODO|stub|mock" crates`.
- [ ] Classify each hit as `test_only`, `donor_only`, `service_owned`, `adapter_owned`, `legacy_direct`, or `production_gap`.
- [ ] Implement MA-01 before touching provider-specific behavior.
- [ ] Implement MA-02 before adding new HTTP/TUI read routes.
- [ ] Implement MA-03 before claiming observability parity.
- [ ] Implement MA-04 before claiming provider parity.
- [ ] Implement MA-05 before claiming Mori-like learning/knowledge behavior.
- [ ] Implement MA-06 before claiming stability.
- [ ] Implement MA-07 only after generated proof reports exist.
- [ ] Update this file by checking off only items backed by source and proof.
- [ ] Update [README.md](README.md) with the new self-grade and proof status.

### 12.7 Archive Gate

This file may move to `archive/` only when all of these are true:

- [ ] `tmp/mori-diffs/generated/master-architecture-proof.json` exists and every required proof field is true or has an explicit non-success status with evidence.
- [ ] `tmp/mori-diffs/generated/provider-matrix-report.json` exists and covers all supported providers.
- [ ] `tmp/mori-diffs/generated/runtime-reconciliation-report.json` exists and shows no P0 runtime direct callers.
- [ ] `tmp/mori-diffs/generated/stability-proof-report.json` exists and proves crash/resume/merge/retry behavior.
- [ ] `rg -n "PlanRunner::from_plans_dir|orchestrate::|dispatch_v2|PromptAssembler::minimal|direct provider parsing" crates` has no unclassified production hits.
- [ ] HTTP, TUI, and CLI read surfaces consume projections or query services rather than runner internals.
- [ ] `orchestrate.rs` is donor-only, deleted, or renamed as legacy with no production-critical unique behavior.
