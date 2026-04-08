# Architecture Reality Check

> Cross-repo audit of current `roko` against `docs/`, `tmp/unified*`, and the older `bardo` / `mori` failure modes.

## Executive Summary

`roko` is in a materially better place than old `mori`, but it still has a large **spec/runtime honesty gap**.

The good news:

- The workspace is real. Unlike the failed Mori refactor, the major crates are not empty shells.
- The new runner path is live. `roko plan run` now goes through `crates/roko-cli/src/runner/`, not the 21K-line legacy `orchestrate.rs`.
- The core extraction work mostly happened: `roko-agent`, `roko-compose`, `roko-orchestrator`, `roko-gate`, `roko-learn`, `roko-neuro`, `roko-daimon`, and `roko-serve` are all substantive crates.

The bad news:

- The runtime still does **not** match the unified architecture story.
- The most important seams are still owned by `roko-cli`, not by a shared engine/kernel boundary.
- Historical issue now partially corrected: the runner previously hardcoded a Claude-stream worldview. Current source has provider-neutral `AgentRuntimeEvent` and pushes Claude stream parsing below `roko-agent`, but provider-agnostic execution is still unproven across the full provider matrix.
- The docs often describe *target architecture* using *shipping language*, which makes the repo look more coherent on paper than it is in code.

The net assessment is:

**Roko is no longer suffering from dead-crate refactor theater; it is suffering from glue-layer centralization and status inflation.**

That is a much better problem to have, but it is still the main architectural blocker.

---

## 1. What Improved Versus Mori

The old Mori failure mode was: create attractive crates and aspirational abstractions, then keep production behavior in the monolith.

That is **not** what happened here.

### 1.1 The extraction is mostly real

Evidence:

- `roko-agent` is a substantive framework crate with real provider, tool-loop, MCP, safety, process, and session code.
- `roko-compose` is a substantive composition crate with prompt assembly, context selection, enrichment, templates, and attention/budget logic.
- `roko-orchestrator` owns a real executor/state-machine layer rather than being pure docware.
- `roko-gate`, `roko-learn`, `roko-neuro`, and `roko-conductor` all contain non-trivial implementation.

This is the opposite of the Mori audit, where five extracted crates were effectively disconnected.

### 1.2 The live CLI has already switched to the new runner

`crates/roko-cli/src/commands/plan.rs` calls `roko_cli::runner::run(...)` for `plan run`.

That matters because some docs still speak as if `orchestrate.rs` is the primary runtime harness. It is still important, but it is no longer the only or even main path for plan execution.

### 1.3 The repo's problem is not "fake modularity"

The repo's problem is now:

- too much policy and integration logic remains in the app layer;
- multiple architecture narratives coexist;
- the runtime path is narrower than the crate surface area implies.

That distinction is important. Fixing this repo does **not** require another grand extraction campaign. It requires making the existing boundaries honest and finishing a few critical migrations.

---

## 2. The Current Architecture Mismatch

The cleanest way to say it:

**The crate graph has improved faster than the execution model.**

### 2.1 Unified spec says "provider-agnostic Cells on Bus/Store"

Historical finding from the first pass:

- `runner/agent_stream.rs` spawns a CLI process directly.
- `runner/types.rs` defines `ClaudeStreamEvent`, `ClaudeAssistantEvent`, `ClaudeToolEvent`, `ClaudeResultEvent`.
- `AgentEvent` in the runner is derived from Claude's stream JSON, not from a provider-neutral agent event contract exported by `roko-agent`.

2026-04-27 source correction:

- [x] `crates/roko-cli/src/runner/types.rs` now aliases `AgentEvent` to `roko_agent::AgentRuntimeEvent`.
- [x] Claude protocol structs now live below `crates/roko-agent/src/provider/claude_cli/stream.rs`.
- [x] `crates/roko-cli/src/runner/agent_stream.rs` delegates parse normalization to `roko-agent`.
- [x] `crates/roko-cli/src/dispatch/mod.rs` uses `AgentRuntimeEvent` as the dispatch event surface.
- [ ] The current runner is still not proven provider-agnostic in the way `tmp/unified/05-AGENT.md` and `tmp/unified/27-ORCHESTRATOR.md` imply because there is no generated provider matrix proof across Anthropic, OpenAI, Moonshot, Z.AI, Perplexity, Claude CLI, and Codex CLI.

### 2.2 The unified spec says "one engine"

But the code still has at least three distinct orchestration shapes:

1. `roko-orchestrator` as the pure executor/state-machine layer.
2. `roko-cli/src/runner/` as the active event-loop runtime for `plan run`.
3. `roko-cli/src/orchestrate.rs` as the older rich integration harness that still contains large amounts of learning, knowledge, dashboard, retry, and dream wiring.

This is the central duplication risk in the repo right now.

The runner is small and cleaner, but it does not yet inherit the richer integrations from `orchestrate.rs`.

`orchestrate.rs` is still where many advanced features are actually wired:

- `CascadeRouter`
- `KnowledgeStore`
- `DreamRunner`
- richer `DashboardEvent` publishing
- more complete `create_agent_for_model()` usage
- broader gate and learning integrations

So the runtime split is currently:

- **runner** = cleaner execution loop
- **orchestrate** = richer system integration

That is a transitional state, not a stable architecture.

### 2.3 The docs say composition is "shipping"

That is only partly true.

`roko-compose` as a crate is substantial, but the live runner still uses:

- `build_task_prompt(...)`
- `build_minimal_system_prompt(...)`

from `runner/agent_stream.rs`, with an explicit TODO to replace that with `RoleSystemPromptSpec`.

2026-04-27 source correction:

- [x] `crates/roko-cli/src/dispatch/prompt_builder.rs` now owns a `PromptAssembler`.
- [x] `crates/roko-cli/src/runner/event_loop.rs` constructs `PromptAssembler::new()` for dispatch.
- [ ] The composition layer is still not proof-complete because production runs do not yet emit generated prompt diagnostics proving role policy, gate feedback, knowledge, playbooks, and code context were all assembled through the mandatory path.

### 2.4 The docs say agent abstractions are unified

That is also only partly true.

`roko-agent` already has the machinery needed for a better runtime:

- provider factory
- tool loop
- safety layer
- session reuse
- pools
- MCP

Historical issue: the runner path bypassed much of that by speaking directly to Claude CLI stream JSON.

Current source correction:

- [x] Stream normalization is now below `roko-agent` for Claude CLI.
- [x] The dispatch facade is present and uses provider-neutral runtime events.
- [ ] The remaining migration is no longer "add the adapter seam"; it is "prove every supported provider/model and retire or classify any remaining legacy helper that bypasses the dispatch facade."

This is not a missing crate boundary. It is a missing **adapter migration**.

---

## 3. Where the Repo Is Still Repeating Mori's Deeper Mistake

The superficial Mori mistake was dead crates.

The deeper Mori mistake was more important:

**architecture docs kept racing ahead of the actual controlling runtime path.**

Roko is still doing that.

### 3.1 Status inflation

`docs/STATUS.md` labels major sections as `Shipping`, while many of the runtime-critical paths are still split between:

- fully wired legacy-rich path,
- partially wired runner path,
- specified-only unified target.

That makes the docs useful as vision, but unreliable as implementation truth.

The biggest inflation points are:

- Orchestration
- Agents
- Composition
- Cross-cut integration between learning/neuro/dreams/conductor and the live runner

### 3.2 Multiple canonical stories

Right now there are three overlapping narratives:

1. `docs/` says the current architecture is broadly shipping.
2. `tmp/unified/` says the real target is Cells/Graphs/Bus/Store with a unified engine.
3. `tmp/mori-diffs/` documents the actual gaps in the live runtime.

All three are individually useful, but together they create ambiguity about what is actually true **today**.

### 3.3 The app layer is still the integration sink

The biggest code smell in current `roko` is not a single god file anymore. It is that `roko-cli` still acts as the integration sink for too many cross-cuts.

The moment a feature becomes "real", it tends to get wired in the CLI/runtime layer first:

- learning
- dashboard events
- knowledge injection
- dreams
- conductor interventions
- provider routing
- gate retry behavior

That is why the runtime remains difficult to reason about even after the crate split.

---

## 4. Concrete Structural Problems

### 4.1 Runner event model was backend-shaped, not system-shaped

The live event loop is architecturally cleaner than `orchestrate.rs`, but its event protocol is still derived from Claude's wire format.

This is backwards.

The runner should consume something like:

- `AgentLifecycleEvent`
- `AgentOutputDelta`
- `ToolInvocationStarted`
- `ToolInvocationFinished`
- `TurnUsageUpdated`
- `TurnCompleted`

exported from `roko-agent`, with provider-specific parsing hidden below that seam.

2026-04-27 source correction:

- [x] The normalized event seam now exists.
- [x] Provider-specific Claude parsing is hidden below `roko-agent`.
- [ ] Provider-agnostic dispatch is still not a proven runtime property until every supported provider/model is exercised through the same event/projection/query path with generated evidence.

### 4.2 `orchestrate.rs` still functions as the "feature-complete shadow runtime"

Even though `plan run` uses the new runner, many of the richer integrations remain stranded in the old path.

This is risky because:

- fixes land in one runtime and not the other;
- docs cite one path while CLI behavior comes from another;
- cross-cut systems become hard to migrate because their call sites are duplicated.

### 4.3 `roko-compose` and `roko-agent` are both still too umbrella-shaped

These crates are substantive, but they are also very broad:

- `roko-agent` exports a large mixed surface: providers, sessions, process management, safety, MCP, tool loop, pools, streaming, translation.
- `roko-compose` exports prompt assembly, enrichment, context bidding, templates, auctions, budget logic, foraging, compaction, role policies.

This is not yet the dead-crate problem, but it is the next likely scale problem.

The docs are already pointing at the right answer:

- `roko-compose` split into `roko-compose-core` + `roko-templates`
- `roko-std` split into defaults vs tools
- bus/kernel surfaces extracted explicitly

Those should be treated as **stabilization moves**, not speculative cleanup.

### 4.4 Bus/Store remains more conceptual than operational

The unified spec makes Bus/Store the universal fabric.

In current code, the runtime is event-driven in several places, but there is still no single operational Bus abstraction driving:

- runner events
- conductor signals
- learning feedback
- dashboard projection
- cross-agent coordination

Instead, there are multiple local mechanisms:

- Tokio channels
- runtime event bus pieces
- state hub sender
- persisted JSONL logs
- crate-local event structures

That is exactly the kind of hidden transport pluralism the unified spec is trying to eliminate.

---

## 5. The Most Important Strategic Conclusion

The repo does **not** need another broad "modularization" push.

It needs a **runtime convergence push**.

That convergence should be organized around this rule:

**Any capability that is considered shipping must be routed through the same live plan-execution path.**

If a feature only exists in `orchestrate.rs`, it is not shipping for the active runner.
If a feature only exists in `tmp/unified/`, it is not shipping at all.

That sounds obvious, but making it explicit prevents a repeat of the Mori pattern.

---

## 6. Recommended Priority Order

### Priority 1: Finish the runner migration and freeze `orchestrate.rs`

Do not let `orchestrate.rs` continue to accumulate first-class behavior.

Target state:

- `plan run` stays on `runner/`
- `orchestrate.rs` becomes compatibility/reference code only, then shrinks
- all new integration work lands in runner-compatible modules

This is the single most important anti-drift move.

### Priority 2: Move runner agent events behind `roko-agent`

The runner must stop knowing provider wire protocols directly.

Target seam:

- `roko-agent` owns provider-specific stream parsing
- runner consumes normalized agent runtime events

Current status:

- [x] Claude stream parsing moved below `roko-agent`.
- [x] Runner event type aliases the provider-neutral event.
- [ ] Provider matrix proof and legacy-helper retirement remain open.

### Priority 3: Make prompt assembly mandatory in the live runner

The runner should not keep `build_minimal_system_prompt()` as a normal path.

Target state:

- `RoleSystemPromptSpec` / `SystemPromptBuilder` or its successor is the default
- gate feedback, playbooks, knowledge hints, and anti-patterns are injected through one prompt assembly surface

Until then, "composition is shipping" is only conditionally true.

### Priority 4: Move cross-cut integrations out of `roko-cli` ad hoc wiring

Especially:

- learning feedback
- knowledge ingestion/query
- dreams consolidation triggers
- dashboard/state projection

These should be made reusable service boundaries, not CLI-only glue.

### Priority 5: Tighten docs to distinguish `shipping`, `wired in legacy path`, and `target`

The current status vocabulary is too optimistic for a repo in transition.

Add at least one more category in practice, even if not formally:

- `shipping`
- `active runner gap`
- `legacy-only wired`
- `built but not routed`
- `specified`

That would make the architecture docs dramatically more trustworthy.

---

## 7. Short Version

If Mori failed because it extracted crates without moving reality, Roko's current risk is different:

**Roko has moved a lot of reality into crates, but it has not yet converged that reality onto one honest runtime model.**

That is why the repo feels better than Mori and still more confusing than the docs promise.

The fix is not more theory. The fix is:

1. converge on the runner,
2. normalize agent events behind `roko-agent`,
3. force composition through the real runner path,
4. stop letting status docs describe target architecture as if it were already uniformly live.

## Implementation Packet

This file is the governance doc for the migration. It should be used when deciding whether a proposed implementation is fixing the split runtime or adding more drift.

### Required Context

- `docs/STATUS.md`
- `docs/00-architecture/15-crate-map.md`
- `docs/00-architecture/33-refactor-plan-phases.md`
- `tmp/unified/00-INDEX.md`
- `tmp/unified/27-ORCHESTRATOR.md`
- `tmp/unified-depth/00-index/architectural-thesis.md`
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/runner/`
- `crates/roko-cli/src/orchestrate.rs`

### Audit Checklist

- [ ] For every feature claimed as `Shipping`, identify the active runtime path that exercises it.
- [ ] Mark features as `legacy-only wired` when they exist in `orchestrate.rs` but not `runner/`.
- [ ] Mark features as `built but not routed` when crate code exists but no active runtime call site exists.
- [ ] Mark features as `target` when they only exist in `tmp/unified*`.
- [ ] Update `docs/STATUS.md` after each phase to avoid status inflation.
- [ ] Add a code search proof for each migration claim.
- [ ] Reject any implementation that increases unique business logic in `orchestrate.rs`.

### Decision Checklist For New Work

- [ ] Does this change run through `runner/`?
- [ ] Does this change move logic toward the owning crate?
- [ ] Does this change reduce provider-specific knowledge in the runner?
- [ ] Does this change add test coverage on the active path?
- [ ] Does this change leave a legacy-only behavior behind? If yes, record it in `21-FEATURE-PARITY-MATRIX.md`.

### Acceptance Criteria

- [ ] Status docs distinguish active runner, legacy-only, built-unrouted, and target states.
- [ ] All future runtime implementation docs cite the active code path they modify.
- [ ] `orchestrate.rs` has a documented freeze policy.
- [ ] `runner/` becomes the measured owner of plan execution.

## Worker 9 Evidence Checklist (2026-04-26)

Reality confirmed in the current tree:

- [x] `crates/roko-cli/src/runner/event_loop.rs` is the active `plan run` execution path and has no-mock one-task Codex/Claude proof.
- [x] `crates/roko-cli/src/dispatch/mod.rs` now exists as the planned dispatch facade; `dispatch_v2.rs` remains a legacy/transition surface that must be classified or retired.
- [x] `crates/roko-cli/src/runner/types.rs` aliases provider-neutral `AgentRuntimeEvent`; Claude protocol structs now live under `roko-agent`.
- [x] `roko-compose`, `roko-learn`, `roko-neuro`, `roko-dreams`, and `roko-conductor` contain substantial reusable engines, but several are built-unrouted from the active runner.
- [x] `crates/roko-cli/src/orchestrate.rs` still contains richer legacy-only integrations for learning, dreams, and knowledge wiring.

Architecture exit checks still open:

- [x] Add `crates/roko-agent/src/runtime_events.rs` and make runner consume normalized provider-neutral events.
- [x] Add a live feedback facade so learning, knowledge, conductor, and dreams have one runner fan-out seam.
- [x] Add a projection facade so UI/API/CLI output can share one event mapping surface.
- [ ] Prove the feedback facade is durable and changes cross-run routing/knowledge/prompt behavior.
- [ ] Prove projection parity across HTTP, TUI, and CLI using the same source events.
- [ ] Replace `orchestrate.rs` as a production-critical shadow runtime with audited donor/reference status only.
- [ ] Update public status docs after each migration phase with active-runner, legacy-only, built-unrouted, and target labels.

## 8. 2026-04-27 Deepening Pass - Source-Corrected Reality Check

Self-grade for this pass:

- Initial rating: 9.90 / 10.
- Reasoning: the file now separates historical gaps from current source-wired seams and gives an implementation-grade path to finish provider, prompt, feedback, projection, and status honesty. The score is not higher because final truth requires generated runtime proof, not just static source correction.

### 8.1 Current Truth Table

- [x] Active `plan run` is runner-based, not `orchestrate.rs`-based.
- [x] Provider-neutral event type exists.
- [x] Claude protocol parsing is below `roko-agent`.
- [x] Dispatch facade exists.
- [x] Prompt assembler facade exists and is constructed by the runner.
- [x] Feedback facade exists and is wired from `commands/plan.rs` into runner config.
- [x] Projection facade exists and runner events are mirrored into it.
- [x] HTTP projection routes exist.
- [ ] There is not yet a single runtime command service that every workflow entrypoint must use.
- [ ] There is not yet a single runtime query service that every HTTP/TUI/CLI read must use.
- [ ] There is not yet generated provider-matrix proof.
- [ ] There is not yet generated prompt-diagnostics proof.
- [ ] There is not yet generated feedback cross-run influence proof.
- [ ] There is not yet generated HTTP/TUI/CLI projection parity proof.
- [ ] There is not yet generated `orchestrate.rs` unique-behavior retirement proof.

### 8.2 Updated Status Vocabulary

Use these labels in every status doc and checklist:

- [ ] `proved_active`: source-wired through the active runner and backed by generated end-to-end evidence.
- [ ] `source_wired_unproven`: source-wired through the active runner, but missing generated proof.
- [ ] `legacy_only`: behavior exists only in `orchestrate.rs`, `dispatch_v2.rs`, old helpers, or abandoned one-off paths.
- [ ] `built_unrouted`: crate/module exists, but no active runtime call path proves it.
- [ ] `surface_owned`: behavior is owned by CLI/HTTP/TUI code when it should be in a service.
- [ ] `target_only`: described in `docs/` or `tmp/unified*`, but no implementation path exists.
- [ ] `retired`: removed or fenced as donor/test-only with grep proof.

### 8.3 Remaining Architecture Problems

#### AR-01: Runtime Command Service Missing

- [ ] Define the canonical service that starts, resumes, cancels, merges, and queries operations.
- [ ] Route `plan run` through it.
- [ ] Route PRD/research/task generation through it.
- [ ] Route HTTP start/cancel/resume actions through it.
- [ ] Route TUI actions through it.
- [ ] Reject or classify every direct runner caller that bypasses it.
- [ ] Produce `tmp/mori-diffs/generated/runtime-command-service-audit.json`.

#### AR-02: Runtime Query Service Missing

- [ ] Define query types for run, task, agent, provider, prompt, gate, retry, merge, resume, feedback, knowledge, and artifacts.
- [ ] Serve HTTP reads from the query service.
- [ ] Serve TUI reads from the query service.
- [ ] Serve CLI status/log reads from the query service.
- [ ] Prevent route handlers from reading runtime internals directly.
- [ ] Produce `tmp/mori-diffs/generated/runtime-query-service-audit.json`.

#### AR-03: Provider Proof Missing

- [ ] Run Anthropic API through dispatch and record status.
- [ ] Run OpenAI API through dispatch and record status.
- [ ] Run Moonshot API through dispatch and record status.
- [ ] Run Z.AI API through dispatch and record status.
- [ ] Run Perplexity API through dispatch and record status.
- [ ] Run Claude CLI through dispatch and record status.
- [ ] Run Codex CLI through dispatch and record status.
- [ ] Use only statuses `proved`, `missing_credentials`, `auth_failed`, `rate_limited`, `unsupported`, or `runtime_error`.
- [ ] Produce `tmp/mori-diffs/generated/provider-matrix-report.json`.

#### AR-04: Prompt Proof Missing

- [ ] Make `PromptAssembler` mandatory for production dispatch.
- [ ] Classify all `PromptAssembler::minimal()` call sites as test-only or migrate them.
- [ ] Emit prompt diagnostics with prompt hash, role policy id, knowledge refs, playbook refs, gate feedback refs, budget decisions, and redaction class.
- [ ] Add a proof run that verifies diagnostics for at least one task with gate feedback and knowledge context.
- [ ] Produce `tmp/mori-diffs/generated/prompt-diagnostics-report.json`.

#### AR-05: Feedback Proof Missing

- [ ] Subscribe feedback facade to task completion, gate failure, retry, provider lifecycle, prompt diagnostics, merge result, and resume events.
- [ ] Persist feedback outputs as durable events or typed records.
- [ ] Prove learning episode creation.
- [ ] Prove routing observation creation.
- [ ] Prove knowledge ingestion or reinforcement.
- [ ] Prove conductor observation creation.
- [ ] Prove dream trigger creation.
- [ ] Run the same project twice and prove first-run feedback changed the second run's prompt, routing, or policy decision.
- [ ] Produce `tmp/mori-diffs/generated/feedback-cross-run-report.json`.

#### AR-06: Projection Proof Missing

- [ ] Persist runtime events in a replayable stream.
- [ ] Build dashboard projection from the stream.
- [ ] Build HTTP projection from the stream.
- [ ] Build TUI projection from the stream.
- [ ] Build CLI status/log output from the stream.
- [ ] Replay events and verify projection digest stability.
- [ ] Produce `tmp/mori-diffs/generated/projection-parity-report.json`.

#### AR-07: Legacy Runtime Retirement Missing

- [ ] Generate an inventory of unique `orchestrate.rs` capabilities.
- [ ] For each capability, map active runner equivalent, planned migration, or explicit retirement decision.
- [ ] Fence `dispatch_v2.rs` as legacy/test-only or migrate remaining behavior into `dispatch/`.
- [ ] Add grep gates for new `orchestrate.rs` production call sites.
- [ ] Add grep gates for new direct provider protocol parsing outside provider adapters.
- [ ] Produce `tmp/mori-diffs/generated/legacy-runtime-retirement-report.json`.

### 8.4 No-Context Implementation Order

An agent should implement the remaining architecture work in this order:

- [ ] Run `rg -n "PlanRunner::from_plans_dir|orchestrate::|dispatch_v2|PromptAssembler::minimal|ClaudeStreamEvent|ClaudeAssistantEvent|tokio::spawn|Command::new|/tmp/|TODO|stub|mock" crates`.
- [ ] Classify every hit with the status vocabulary in section 8.2.
- [ ] Implement AR-01 before changing HTTP/TUI/PRD/research behavior.
- [ ] Implement AR-02 before adding new read endpoints or TUI panels.
- [ ] Implement AR-03 before claiming provider-agnostic runtime.
- [ ] Implement AR-04 before claiming composition parity.
- [ ] Implement AR-05 before claiming Mori-like feedback/knowledge/dream behavior.
- [ ] Implement AR-06 before claiming observability parity.
- [ ] Implement AR-07 only after each migrated capability has source and generated proof.
- [ ] Update [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [24-DEFINITIVE-GAP-LIST.md](24-DEFINITIVE-GAP-LIST.md), and [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md) with the new statuses.

### 8.5 Archive Gate

Do not archive this file until all of these are true:

- [ ] Every row in section 8.1 is `proved_active`, `retired`, or explicitly superseded by [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md).
- [ ] Generated provider, prompt, feedback, projection, command-service, query-service, and legacy-retirement reports exist.
- [ ] `docs/STATUS.md` uses the status vocabulary from section 8.2 or links to a file that does.
- [ ] A clean-clone proof script can regenerate the reports without relying on `/tmp` artifacts from prior sessions.
