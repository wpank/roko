# Master Implementation Plan

Roko is a Rust toolkit for agents that build themselves: 18 crates, ~177K LOC.
It reads PRDs, generates plans, executes tasks via LLM agents, validates with
gates, persists results, and learns from outcomes. This document is the top-level
plan for completing that vision.

Written 2026-04-29 against branch `wp-arch2`. Every claim grounded in source code
and the 71-document analysis corpus in `tmp/solutions/roko/`.

---

## 1. Executive Summary

### What Works

Roko's core self-hosting loop is functional. A user can capture a work item
(`prd idea`), draft a PRD (`prd draft`), generate an implementation plan
(`prd plan`), execute it (`plan run`), resume if interrupted (`--resume`),
and inspect results (`dashboard`, `status`, `learn`). The HTTP control plane
serves ~85 routes. The ACP protocol integrates with editors. The ratatui TUI
provides live monitoring. 31 models across 12 providers are configured. The
codebase compiles clean on stable Rust 1.91+.

### What's Broken

**The system cannot learn from its own runs.** The default execution path
(runner v2) records zero durable feedback -- no episodes, no routing
observations, no gate threshold updates, no section effectiveness data. The
most sophisticated learning features exist only in the deprecated
`orchestrate.rs` (22K lines, zero live callers). Every run is the first run.

Three execution engines (runner v2, WorkflowEngine, orchestrate.rs) serve
overlapping purposes with different feature sets. Config fields are loaded
then discarded -- `default_model` is ignored by most dispatch paths, `[[gate]]`
arrays from `roko init` are silently thrown away by the runtime parser. Three
P0 issues exist: a runtime panic (`config mcp`), share routes outside auth,
and cloud deploys without authentication.

### What Needs to Happen

1. **Fix what's wrong** -- kill panics, close security holes, make config work
2. **Make the system learn** -- wire feedback into the live execution paths
3. **Converge architectures** -- one dispatch path, one gate pipeline, retire dead code
4. **Build the UX workflow** -- Context Packs, smart decomposition, interactive steering
5. **Ship innovations** -- agent memory, multi-agent collaboration, self-improving gates

---

## 2. Priority Tiers

### P0: Stability and Correctness

Things that are demonstrably wrong. Panics, security holes, config that's
loaded and discarded, features that crash or silently fail.

| Issue | Impact | Source Doc |
|-------|--------|-----------|
| `roko config mcp` panics (unreachable) | CLI crash | 11-GROUND-TRUTH S2.1 |
| Share routes outside auth middleware | Security: unauthenticated transcript access | 11-GROUND-TRUTH S4 |
| Cloud deploy without auth provisioning | Security: public unauthenticated API | 11-GROUND-TRUTH S4 |
| CLI Gist path sends unscrubbed transcripts | Secret leakage | 11-GROUND-TRUTH S4 |
| `[[gate]]` arrays from init silently discarded | Config ignored | 05-GAPS AP-6 |
| Streaming events silently drained in chat | No streaming UX | 05-GAPS AP-7 |
| Model name shows "-" in TUI | Broken display | 11-GROUND-TRUTH S7.2 |
| Runner v2 records zero learning artifacts | System never learns | 01-LESSONS S7 |
| `default_model` ignored by most dispatch paths | Config not respected | 05-GAPS S9 |

### P1: Architecture Convergence

Structural problems that cause feature fragmentation and maintenance burden.
These don't crash, but they make every subsequent fix harder.

| Issue | Impact | Source Doc |
|-------|--------|-----------|
| Three execution engines with different features | Feature disparity | 01-LESSONS S7, 17-ORCH-AUDIT |
| 9+ model selection paths with inconsistent behavior | Wrong model selected | 05-GAPS S9, 19-DISPATCH-ISSUES |
| orchestrate.rs god object (22K lines) | Unmaintainable | 17-ORCH-ISSUES ISS-03 |
| Two gate config schemas | Init config invisible to runtime | 05-GAPS AP-6 |
| StateHub type duplication (#[path] includes) | Serve+CLI can't share state | 11-GROUND-TRUTH S7.2 |
| Two playbook mechanisms (bench vs plan runner) | Data never converges | 05-GAPS AP-4 |
| CascadeRouter has zero live callers | Router never learns | 19-DISPATCH-ISSUES ISS-01 |

### P2: Features and UX Workflow

New capabilities that make roko genuinely useful, not just functional.

| Feature | Value | Source Doc |
|---------|-------|-----------|
| Context Packs (aggregate-funnel-execute) | Full research-to-ship workflow | 09-UX-VISION |
| Parallel task execution in plan runner | 2-4x speedup for DAG plans | 11-GROUND-TRUTH S7.5 |
| Prompt assembly fixes (model-aware windowing) | Small models stop getting overloaded | 16-PROMPT-ISSUES ISS-01 |
| LLM judge gate (rung 6) | Semantic code review | 06-IMPL-PLANS 7.6 |
| Express gate mode (per-task complexity) | 10x faster gates for trivial tasks | 06-IMPL-PLANS 7.1 |
| Wave gates for multi-task plans | 3-5x faster total gate time | 06-IMPL-PLANS 7.3 |
| Replan-on-gate-failure loop | Self-healing execution | 06-IMPL-PLANS 1.5 |
| Agent memory and continuity | Agents learn across sessions | 10-INNOVATIONS S1 |
| API provider chat (send_turn_api) | Chat with all configured providers | 11-GROUND-TRUTH S7.3 |

### P3: Polish, GTM, and Ecosystem

Things that matter for shipping but don't affect core functionality.

| Feature | Value | Source Doc |
|---------|-------|-----------|
| A2A interop bridge | Cross-framework agent communication | 10-INNOVATIONS S10 |
| Marketplace and community gates | Shareable gate criteria | 14-GATE-VIZ-06 |
| HAL benchmark integration | Standardized agent evaluation | 13-PERF-HAL |
| Tracker integrations (Linear, GitHub, Jira) | Corpus-centric work intake | 15-UX-TRACKER |
| Performance optimization playbook (14 items) | Sub-500ms fast path | 13-PERF-PLAYBOOK |
| Dream cycle cron trigger | Automatic knowledge consolidation | 11-GROUND-TRUTH S7.7 |
| Cross-project learning | Transfer knowledge between repos | 10-INNOVATIONS S7 |

---

## 3. Phase Plan

### Phase 0: Critical Fixes (Week 1-2)

**Goal**: Nothing crashes, nothing leaks secrets, config is respected, the system
records feedback on every run.

**Tracks** (can run in parallel):

#### Track A: P0 Bug Fixes (1 day)

- Wire `ConfigCmd::Mcp` dispatch (kill the `unreachable!()` panic)
- Move share routes inside auth middleware
- Auto-provision auth key on cloud deploy
- Add secret scrubbing to CLI Gist path

Impl plan: `impl/01-STABILITY-AND-FIXES.md`

#### Track B: Learning Wiring (2-3 days)

Wire all learning hooks into runner v2 (the default execution path):
- Episode logging after each task attempt
- CascadeRouter observations after dispatch
- AdaptiveThreshold updates after gate execution
- Efficiency event emission after agent turns
- Section effectiveness recording on completion

Impl plan: `impl/07-LEARNING-AND-FEEDBACK.md`

#### Track C: Config Consistency (2-3 days)

- Unify model selection: all paths use `ServiceFactory::build()` for resolution
- Accept both `[[gate]]` and `[gates]` config formats
- Update `roko init` to write runtime-compatible format
- Remove hardcoded model strings (8 occurrences across codebase)

Impl plans: `impl/03-INFERENCE-DISPATCH.md`, `impl/16-CONFIG-AND-WIRING.md`

#### Track D: Streaming UX (1-2 days)

- Forward streaming events to TUI (replace silent drain)
- Fix model name display in runner v2 TUI
- Wire token count updates during generation

Impl plan: `impl/08-UX-AND-CLI.md`

**Phase 0 Exit Criteria**:
- `roko config mcp list` runs without crash
- `POST /api/runs/{id}/share` returns 401 without auth
- `roko plan run` on a 3-task plan produces entries in `episodes.jsonl`,
  `cascade-router.json`, `gate-thresholds.json`, `efficiency.jsonl`,
  and `section-effects.json`
- `default_model` in roko.toml is respected by `run`, `chat`, and `plan run`
- Chat TUI shows streaming text during generation
- TUI shows actual model name, not "-"

**Effort**: 6-10 person-days across 4 parallel tracks.

---

### Phase 1: Architecture Convergence (Week 3-4)

**Goal**: One dispatch path, one gate pipeline, orchestrate.rs decomposed.
The system has a single source of truth for execution and learning.

#### Track A: Runtime Convergence (3-4 days)

Port remaining orchestrate.rs features to runner v2 and WorkflowEngine:
- Playbook extraction and injection
- Knowledge store queries for system prompt enrichment
- C-factor computation at run end
- Crate familiarity tracking
- Daimon affect modulation
- Replan-on-gate-failure loop
- Enrichment pipeline (highest complexity -- extract from god object first)

Impl plan: `impl/02-ORCHESTRATION.md`

#### Track B: orchestrate.rs Decomposition (3-4 days)

Break the 22K-line god object into focused modules:
- Gate execution -> `roko-gate/src/gate_executor.rs`
- Model routing -> `roko-learn/src/routing_executor.rs`
- Worktree management -> already at `roko-orchestrator/src/worktree.rs`
- Context assembly -> `roko-compose/src/context_assembler.rs`
- Feedback recording -> `roko-learn/src/feedback_executor.rs`
- Keep only action dispatch and state management in the residual

Impl plan: `impl/02-ORCHESTRATION.md`, `impl/12-CODE-DEBT.md`

#### Track C: Dispatch Unification (2-3 days)

- Make all 9+ entry points use `ServiceFactory::build()` for model resolution
- Remove `auth_detect.rs` env-scanning when config is available
- Normalize model aliases at load time
- Eliminate `unsafe { std::env::set_var() }` for provider override
- Route plan runner dispatch through `ModelCallService`

Impl plan: `impl/03-INFERENCE-DISPATCH.md`

#### Track D: Gate Pipeline Convergence (2-3 days)

- Extend `GateConfig` with complexity and prior_failures fields
- Add feedback and failure classification to `GateReport`
- Wire GateService into all execution paths (run.rs, event_loop.rs, ACP runner)
- Accept both config formats with deprecation warning for `[[gate]]`

Impl plan: `impl/04-GATE-PIPELINE.md`

**Phase 1 Exit Criteria**:
- `grep -rn 'orchestrate' crates/roko-cli/src/ | wc -l` returns < 50
  (module extracted, not monolith)
- All entry points (`run`, `chat`, `plan run`, ACP, HTTP) produce identical
  model selection for the same config
- Gate feedback is generated by GateService, not per-caller code
- Learning persistence works identically across runner v2 and WorkflowEngine
- `cargo check --workspace` compiles cleanly with extracted modules

**Effort**: 10-14 person-days across 4 parallel tracks.

---

### Phase 2: UX Workflow and Prompt Assembly (Week 5-8)

**Goal**: The aggregate-funnel-execute workflow works end-to-end. Prompt
assembly is model-aware. The gate pipeline has semantic evaluation.

#### Track A: Context Packs (5-7 days)

The core UX innovation. Five-pass funnel:
1. `roko pack create` -- aggregate sources (dirs, files, URLs) into a pack
2. `roko pack synthesize` -- multi-pass compression with agent
3. `roko pack architect` -- turn synthesis into architecture decisions
4. `roko pack decompose` -- break architecture into tasks (tasks.toml)
5. `roko pack execute` -- run the generated plan

Each pass is agent-driven with configurable model and token budget per tier.
Human approval gates between passes (configurable: auto-approve for simple tasks).

Impl plan: `impl/08-UX-AND-CLI.md`
Design source: `09-UX-WORKFLOW-VISION.md`

#### Track B: Prompt Assembly Fixes (3-4 days)

- Wire `ContextTier` from model profile into prompt assembly
- Implement per-tier section budgets (small models get compressed prompts)
- Wire `BudgetPredictor` (built, never called) for token estimation
- Add progressive refinement: 5-stage pipeline from raw context to final prompt
- Make chat use `PromptAssemblyService` (currently bypassed)

Impl plan: `impl/06-PROMPT-ASSEMBLY.md`
Issue source: `16-PROMPT-ISSUES.md`

#### Track C: Gate Evolution (3-5 days)

- Replace `StubJudgeGate` with `LlmJudgeGate` (configurable model, real review)
- Express gate mode: trivial tasks get compile-only, complex tasks get full pipeline
- Wave gates for multi-task plans (gate at wave boundaries, not per-task)
- Failure classification with episode-based similarity search

Impl plan: `impl/05-GATE-EVOLUTION.md`
Design source: `14-GATE-VIZ-*` series

#### Track D: Parallel Execution (2-3 days)

- Expose `max_concurrent_tasks` from config (currently hardcoded to 1)
- Add `--parallel N` flag to `plan run`
- Auto-derive parallelism from DAG width when not specified
- Wire worktree isolation per concurrent task
- Cumulative context section: each agent knows what prior agents changed

Impl plan: `impl/02-ORCHESTRATION.md`

#### Track E: Repo-Grounded Plan Generation (2 days)

- Wire `build_repo_context()` into plan generate, regenerate, and prd plan
- Inject validation diagnostics into regeneration loop (retry up to 2x)
- Make grounding validation blocking (reject plans that duplicate existing crates)

Impl plan: `impl/08-UX-AND-CLI.md`

**Phase 2 Exit Criteria**:
- `roko pack create -> synthesize -> architect -> decompose -> execute` works
  end-to-end on a real feature
- Small model (Cerebras 8B) gets a compressed prompt under 4K tokens; large
  model (Claude Sonnet) gets full 16K prompt
- LLM judge gate runs real code review with configurable model
- 10-task plan with 3 waves: gates run 3 times, not 10
- Plan generation includes repo context and retries on validation failure

**Effort**: 15-21 person-days across 5 tracks.

---

### Phase 3: Innovations and Learning Loops (Week 9-12)

**Goal**: The system genuinely improves over time. Agent memory persists.
Multi-agent collaboration works. Self-improving gates evolve criteria.

#### Track A: Agent Memory and Continuity (5-7 days)

Three-tier memory system:
1. Working memory (per-session): task context, tool history, partial results
2. Episodic memory (per-project): what happened, what worked, what failed
3. Semantic memory (cross-project): durable knowledge, distilled patterns

Wire the existing KnowledgeStore, EpisodeLogger, and PlaybookStore into a
unified memory layer. Agents query memory at dispatch time and update it
on completion. Memory consolidation via dream cycle with automatic trigger.

Impl plan: `impl/11-INNOVATIONS.md`
Design source: `10-INNOVATIONS-AND-NEW-FEATURES.md` S1

#### Track B: Self-Improving Gates (3-5 days)

Closed feedback loop: gate outcomes inform gate criteria:
- Track pass/fail rates per criterion over time
- Auto-adjust thresholds based on EMA/CUSUM statistical control
- Feed gate failure patterns into agent prompts (anti-pattern inoculation)
- Adaptive skip: gates that pass 95%+ of the time can be skipped for trivial tasks

Impl plan: `impl/05-GATE-EVOLUTION.md`
Design source: `14-GATE-VIZ-05-Self-Improvement-Flywheel.md`

#### Track C: Multi-Agent Collaboration (4-6 days)

- Wire composition operators (pipeline, parallel, conditional, mixture)
- SkillSelector routing by task category, complexity, and quality profile
- Speculative execution: run parallel predictions with multiple models
- Cost-aware scheduling: route cheap tasks to fast models, complex to powerful
- Convergence detection: stop agents stuck in loops

Impl plan: `impl/11-INNOVATIONS.md`
Design source: `10-INNOVATIONS-AND-NEW-FEATURES.md` S2

#### Track D: Performance Optimization (3-4 days)

The 14 optimizations from the playbook, prioritized by impact:
1. Shared HTTP client (320-500ms savings, 2h effort)
2. Express gate mode for trivial tasks (500-2000ms, 4h)
3. Memoize efficiency signals, batch substrate writes (150-300ms, 4h)
4. Pre-spawned warm agent pool (200-500ms, 8h)
5. VCG auction for prompt assembly under tight budgets

Target: fast API model path from ~880ms to ~460ms.

Impl plan: `impl/10-PERFORMANCE.md`
Source: `13-PERF-OPTIMIZATION-PLAYBOOK.md`

#### Track E: Observability (2-3 days)

- Wire gateway events into cost dashboard
- Add episode-based regression detection to V2
- `roko run --dry-run` for debugging
- Persist WorkflowEngine checkpoint for resume
- Wire conductor observations into stuck detection

Impl plan: `impl/18-OBSERVABILITY.md`

**Phase 3 Exit Criteria**:
- Agent dispatched for task X in project Y has memory of past attempts on
  similar tasks
- Gate criteria auto-adjusted after 50+ observations
- Multi-agent plan with parallel tasks uses appropriate models per task
- `roko run` with fast model completes in <500ms (no gates)
- `roko run --dry-run` shows model, prompt sections, gates without dispatching

**Effort**: 17-25 person-days across 5 tracks.

---

### Phase 4: GTM, Integrations, and Ecosystem (Week 13+)

**Goal**: Roko is ready for external users. Integrations, benchmarks,
marketplace, and documentation are in place.

#### Track A: Integrations (5-7 days)

- Linear adapter: import/sync issues as roko tasks
- GitHub adapter: PR-driven workflow, issue ingestion
- Langfuse adapter: observability export
- Tracker abstraction layer (`CorpusSource` + `TrackerAdapter` traits)

Impl plan: `impl/13-GTM-AND-INTEGRATIONS.md`
Source: `15-UX-TRACKER-INTEGRATIONS.md`, `21-GTM-INTEGRATIONS.md`

#### Track B: Benchmarking (3-5 days)

- HAL benchmark integration (Princeton ICLR 2026)
- Multi-dimensional evaluation (correctness, efficiency, cost, safety)
- SWE-bench proxy with contamination detection
- CI integration for continuous benchmark tracking

Impl plan: `impl/15-TESTING-AND-VERIFICATION.md`
Source: `13-PERF-HAL-AND-AGENT-BENCHMARKS.md`

#### Track C: A2A Interop (3-4 days)

- Bridge between ACP and Google's A2A protocol
- Agent Card generation from roko agent manifests
- Task delegation across A2A-compatible frameworks
- MCP federation for tool sharing across agents

Impl plan: `impl/09-ACP-AND-MCP.md`
Source: `10-INNOVATIONS-AND-NEW-FEATURES.md` S10, `12-ACP-MCP-DEEP-DIVE.md`

#### Track D: Marketplace and Community (3-4 days)

- Community gate criteria (shareable criterion definitions)
- Trust and reputation system for shared components
- Plugin registry and quality scoring
- Recipe format for composable workflows (`recipe.toml`)

Impl plan: `impl/13-GTM-AND-INTEGRATIONS.md`
Source: `14-GATE-VIZ-06-Community-Marketplace.md`, `21-GTM-ECOSYSTEM-PATTERNS.md`

#### Track E: Safety and Security Hardening (2-3 days)

- Make safety contracts non-permissive by default
- Generate default contract YAML during `roko init`
- Remove always-true `dangerously_skip_permissions` in plan mode
- Budget enforcement across all dispatch paths
- Audit trail for all agent actions

Impl plan: `impl/17-SAFETY-AND-SECURITY.md`

**Phase 4 Exit Criteria**:
- Linear issues sync into roko task format
- HAL benchmark suite runs and reports scores
- A2A Agent Card published and discoverable
- `roko init` generates safety contracts
- Community gate criteria loadable from registry

**Effort**: 16-23 person-days across 5 tracks.

---

## 4. Dependency Graph

```
Phase 0 (Critical Fixes)
  |
  +-- Track A: P0 Bugs .............. no deps, do first
  +-- Track B: Learning Wiring ...... no deps
  +-- Track C: Config Consistency ... no deps
  +-- Track D: Streaming UX ........ no deps
  |
  | All Phase 0 tracks are independent and parallelizable.
  |
  v
Phase 1 (Architecture Convergence)
  |
  +-- Track A: Runtime Convergence ... depends on Phase 0 Track B (learning wired)
  +-- Track B: orchestrate.rs Decomp . depends on Phase 1 Track A (features ported)
  +-- Track C: Dispatch Unification .. depends on Phase 0 Track C (config consistent)
  +-- Track D: Gate Convergence ...... depends on Phase 0 Track C (config formats)
  |
  | Track A blocks Track B. Tracks C and D are independent of A/B.
  |
  v
Phase 2 (UX Workflow and Features)
  |
  +-- Track A: Context Packs ........ depends on Phase 1 Track C (unified dispatch)
  +-- Track B: Prompt Assembly ....... depends on Phase 1 Track C (model resolution)
  +-- Track C: Gate Evolution ........ depends on Phase 1 Track D (gate convergence)
  +-- Track D: Parallel Execution .... depends on Phase 1 Track A (runtime converged)
  +-- Track E: Repo-Grounded Plans ... no deps beyond Phase 0
  |
  | Track E can start during Phase 1. Others wait for Phase 1 completion.
  |
  v
Phase 3 (Innovations)
  |
  +-- Track A: Agent Memory ......... depends on Phase 1 Track A (learning wired)
  +-- Track B: Self-Improving Gates .. depends on Phase 2 Track C (gate evolution)
  +-- Track C: Multi-Agent ........... depends on Phase 1 Track C (dispatch unified)
  +-- Track D: Performance ........... depends on Phase 1 (architecture stable)
  +-- Track E: Observability ......... depends on Phase 0 Track B (learning exists)
  |
  | Track E can start during Phase 2. Others wait for Phase 2 completion.
  |
  v
Phase 4 (GTM and Ecosystem)
  |
  +-- All tracks depend on Phase 2 completion (stable product surface).
  +-- Tracks are independent and parallelizable.
```

### Critical Path

The longest sequential dependency chain:

```
Phase 0 Track B (Learning) -----> Phase 1 Track A (Runtime Convergence)
  2-3 days                          3-4 days

Phase 1 Track A ----------------> Phase 1 Track B (Decomposition)
                                    3-4 days

Phase 1 Track B ----------------> Phase 2 Track D (Parallel Execution)
                                    2-3 days

Total critical path: 10-14 days (Weeks 1-4)
```

Everything else can be parallelized around this chain.

---

## 5. Plan Index

Each `impl/` document covers one subsystem or cross-cutting concern. Plans are
independently actionable -- a developer can pick up any single plan and execute
it without reading the others.

| # | Document | Scope | Phase | Key Issues |
|---|----------|-------|-------|------------|
| 01 | `impl/01-STABILITY-AND-FIXES.md` | P0 bug fixes: panics, security holes, config crashes | Phase 0 | ConfigCmd::Mcp panic, share auth bypass, Gist scrubbing, cloud auth |
| 02 | `impl/02-ORCHESTRATION.md` | Runtime convergence, orchestrate.rs decomposition, parallel execution | Phase 1-2 | Three engines, god object, serial execution, feature parity gap |
| 03 | `impl/03-INFERENCE-DISPATCH.md` | Model selection unification, CascadeRouter wiring, provider consolidation | Phase 0-1 | 9+ dispatch paths, hardcoded models, auth_detect ignoring config |
| 04 | `impl/04-GATE-PIPELINE.md` | Gate dispatch convergence, config format unification, feedback integration | Phase 0-1 | 3 dispatch paths, stub verdicts, config schema split |
| 05 | `impl/05-GATE-EVOLUTION.md` | LLM judge, express gates, wave gates, self-improving criteria | Phase 2-3 | StubJudgeGate, per-task compilation overhead, static thresholds |
| 06 | `impl/06-PROMPT-ASSEMBLY.md` | Model-aware windowing, BudgetPredictor wiring, progressive refinement | Phase 2 | Small models overloaded, predictor never called, chat bypasses builder |
| 07 | `impl/07-LEARNING-AND-FEEDBACK.md` | Episode logging, routing observations, threshold updates, efficiency events | Phase 0 | Runner v2 records nothing, ACP partial, orchestrate.rs dead |
| 08 | `impl/08-UX-AND-CLI.md` | Context Packs, streaming fix, plan grounding, CLI polish | Phase 0-2 | No aggregation workflow, silent drain, ungrounded plans |
| 09 | `impl/09-ACP-AND-MCP.md` | ACP/MCP convergence, A2A bridge, MCP federation, protocol gaps | Phase 2-4 | Legacy env gating, non-CLI MCP, 11 protocol gaps |
| 10 | `impl/10-PERFORMANCE.md` | 14 optimizations, warm pool, express gates, VCG activation | Phase 3 | 880ms baseline, compilation dominates gate time |
| 11 | `impl/11-INNOVATIONS.md` | Agent memory, multi-agent, speculative execution, cross-project learning | Phase 3-4 | Stateless agents, no collaboration operators, no transfer learning |
| 12 | `impl/12-CODE-DEBT.md` | Dead code removal, duplicate elimination, module extraction | Phase 1-2 | 22K LOC dead orchestrate.rs, duplicate implementations, unused imports |
| 13 | `impl/13-GTM-AND-INTEGRATIONS.md` | Tracker adapters, marketplace, recipes, partnership integrations | Phase 4 | No external integrations, no community sharing |
| 14 | `impl/14-RUNNER-PATTERNS.md` | Worktree isolation, wave gating, cumulative context, result files | Phase 2-3 | Operational patterns from mega-parity runner |
| 15 | `impl/15-TESTING-AND-VERIFICATION.md` | HAL benchmarks, SWE-bench, CI integration, multi-dimensional eval | Phase 4 | No standardized benchmarks, no regression tracking |
| 16 | `impl/16-CONFIG-AND-WIRING.md` | Config schema unification, field load path tracing, init template fixes | Phase 0-1 | 10 config-runtime disconnects, 2 schema formats |
| 17 | `impl/17-SAFETY-AND-SECURITY.md` | Safety contracts, permission enforcement, budget guardrails, audit trails | Phase 4 | Always-permissive defaults, no contract enforcement |
| 18 | `impl/18-OBSERVABILITY.md` | Gateway events, regression detection, dry-run mode, checkpointing | Phase 3 | Write-only artifacts, no regression alerts, no dry-run |
| 19 | `impl/19-CROSS-CUTTING.md` | StateHub unification, event schema alignment, error handling, spec migration | Phase 1-2 | Dual StateHub types, event system overlap, 95 spec migration batches |

---

## 6. Success Criteria

### Phase 0: "Nothing Is Broken"

| Criterion | Verification |
|-----------|-------------|
| No CLI panics | `roko config mcp list` shows servers or "none configured" (no crash) |
| Security by default | `POST /api/runs/{id}/share` without auth returns 401 |
| Learning persists | After `roko plan run` on 3-task plan: `episodes.jsonl` has 3+ entries, `cascade-router.json` has observations > 0, `gate-thresholds.json` has per-rung stats |
| Config respected | Set `default_model = "cerebras-70b"` in roko.toml; `roko run "hello"` uses Cerebras |
| Streaming works | `roko chat` shows streaming text during generation, not spinner-only |
| Model displayed | TUI shows "claude-sonnet-4-20250514", not "-" |

### Phase 1: "One Way to Do Things"

| Criterion | Verification |
|-----------|-------------|
| Single dispatch path | All entry points use ServiceFactory for model resolution. `grep -rn '"claude-3-haiku\|"claude-sonnet' crates/roko-cli/src/ | grep -v test` returns 0 |
| Gate pipeline converged | GateService used by run.rs, event_loop.rs, and ACP runner. Feedback generated internally |
| God object decomposed | orchestrate.rs < 2K lines (state management only). Extracted modules have tests |
| Learning consistent | `roko run` and `roko plan run` produce identical learning artifact types |
| Config formats unified | Both `[[gate]]` and `[gates]` accepted. `roko init` generates runtime-compatible config |

### Phase 2: "Actually Useful"

| Criterion | Verification |
|-----------|-------------|
| Context Packs work | `roko pack create -> synthesize -> architect -> decompose -> execute` end-to-end |
| Model-aware prompts | Small model gets < 4K token prompt; large model gets full context |
| Judge gate functional | `enabled_gates = ["compile","clippy","test","judge"]` runs real LLM review with non-zero cost |
| Parallel execution | 10-task plan with 5 independent tasks runs 5 concurrently |
| Plans grounded | Generated plans reference existing crates, not greenfield duplicates |

### Phase 3: "Genuinely Intelligent"

| Criterion | Verification |
|-----------|-------------|
| Agent memory | Agent dispatched for task X recalls outcomes of similar past tasks |
| Self-improving gates | After 50 observations, gate thresholds auto-adjusted and skip rate > 0 for high-pass criteria |
| Multi-agent | Plan with mixed complexity tasks routes to appropriate models per task |
| Performance | Fast API path < 500ms (no gates). Express gate path < 700ms |
| Dry-run | `roko run --dry-run "hello"` shows model, prompt sections, gates without dispatching |

### Phase 4: "Ready for Users"

| Criterion | Verification |
|-----------|-------------|
| Integration works | Linear issues sync into roko tasks and execute |
| Benchmarks run | HAL suite produces multi-dimensional scores |
| A2A interop | Agent Card discoverable via A2A protocol |
| Safety enforced | `roko init` generates contracts; plan mode enforces them |
| Community sharing | Gate criteria loadable from published registry |

---

## 7. Risk Assessment

### R1: Decomposition Breaks Existing Functionality

**Risk**: Extracting modules from orchestrate.rs introduces regressions.
**Likelihood**: Medium. The 22K-line file has deep coupling.
**Mitigation**: Phase 0 wires learning into runner v2 first. Phase 1
decomposes orchestrate.rs only after runner v2 has feature parity. The
decomposition is extraction (moving code), not rewriting. Comprehensive
gate testing validates each extraction.
**Fallback**: Keep orchestrate.rs behind feature flag as reference while
extracted modules mature.

### R2: Learning Wiring Incomplete

**Risk**: Episode/routing/threshold recording is partially wired, producing
incomplete or incorrect learning data.
**Likelihood**: Medium. The types exist, but threading them through the
event loop has edge cases (retries, partial failures, gate timeouts).
**Mitigation**: Each learning component has explicit acceptance criteria with
concrete verification steps. Wire one component at a time, verify, then next.
**Fallback**: Partial learning (episodes only) is still far better than
zero learning. Ship incrementally.

### R3: Context Packs Scope Creep

**Risk**: The 5-pass funnel (synthesize -> architect -> decompose -> scope ->
execute) is complex. Each pass adds agent cost and failure surface.
**Likelihood**: Medium-High.
**Mitigation**: Phase 2 Track A builds the pack data model and 2 passes first
(create + synthesize). Additional passes ship in subsequent iterations. Each
pass is independently useful -- synthesize alone is valuable.
**Fallback**: Ship `roko pack create` and `roko pack synthesize` as the MVP.
Manual architecture and decomposition as escape hatch.

### R4: Parallel Execution Introduces Merge Conflicts

**Risk**: Concurrent agents modifying shared files produce conflicts.
**Likelihood**: High for plans with poor task boundaries.
**Mitigation**: Worktree isolation per concurrent task (already built in
`roko-orchestrator/src/worktree.rs`). Cumulative context sections tell
each agent what others changed. Wave gates catch conflicts at boundaries.
DAG dependencies prevent truly conflicting tasks from running concurrently.
**Fallback**: Default `max_concurrent_tasks = 1` (current behavior). Users
opt into parallelism with `--parallel`.

### R5: Model Provider API Changes

**Risk**: Provider APIs (Claude, OpenAI, Gemini) change and break dispatch.
**Likelihood**: Low-Medium. APIs are generally backward compatible but
streaming formats and tool calling conventions evolve.
**Mitigation**: `ModelCallService` is a single dispatch wrapper. Provider
changes require updating one adapter, not 9+ call sites. Integration tests
against real APIs run in CI.
**Fallback**: Claude CLI subprocess is the most stable path and always works.

### R6: Performance Targets Not Met

**Risk**: The ~460ms fast-path target requires multiple optimizations.
**Likelihood**: Low. Each optimization is independently measurable.
**Mitigation**: Benchmark before and after each optimization. Ship
optimizations individually. The biggest win (express gates) is architectural,
not micro-optimization.
**Fallback**: Current ~880ms path is usable. Performance is P3 priority.

### R7: Breaking Existing Dogfood Workflows

**Risk**: Changes to dispatch, gates, or config break the working self-hosting loop.
**Likelihood**: Medium. The dogfood workflow is the primary validation path.
**Mitigation**: Run the full self-hosting loop (`prd idea -> plan -> run`)
as integration test after each phase. The dogfood session from April 26
documented 6 critical fixes -- these are now regression tests.
**Fallback**: `--engine legacy` flag preserved as emergency rollback.

---

## 8. Resource Estimates

### Effort by Phase

| Phase | Scope | Effort | Calendar (1 dev) | Calendar (2 devs) |
|-------|-------|--------|------------------|--------------------|
| Phase 0 | Critical fixes | 6-10 days | 2 weeks | 1 week |
| Phase 1 | Architecture | 10-14 days | 3 weeks | 2 weeks |
| Phase 2 | UX + Features | 15-21 days | 5 weeks | 3 weeks |
| Phase 3 | Innovations | 17-25 days | 6 weeks | 3 weeks |
| Phase 4 | GTM | 16-23 days | 6 weeks | 3 weeks |
| **Total** | | **64-93 days** | **22 weeks** | **12 weeks** |

### Parallelization Opportunities

**Phase 0**: All 4 tracks are independent. With 4 developers (or 4 parallel
agent sessions), Phase 0 completes in 2-3 days instead of 6-10.

**Phase 1**: Tracks C (dispatch) and D (gates) are independent of A (runtime)
and B (decomposition). 2-way parallelism.

**Phase 2**: Track E (repo grounding) can start during Phase 1. Tracks A-D
are mostly independent. 4-way parallelism.

**Phase 3**: Track E (observability) can start during Phase 2. Tracks A-D
are independent. 4-way parallelism.

**Phase 4**: All 5 tracks are independent. 5-way parallelism.

### Self-Hosting Acceleration

Roko can accelerate its own development. Once Phase 0 is complete (learning
works), subsequent phases can use `roko plan run` to execute implementation
tasks with agent feedback. The meta-loop:

1. Write impl plan as tasks.toml
2. `roko plan run impl/phase-1/` -- agents execute tasks
3. Gates validate (compile, clippy, test)
4. Learning feeds back into next run
5. Repeat with Phase 2, etc.

Expected acceleration: 30-50% reduction in calendar time for Phases 2-4
once learning loop is functional.

### Quick Wins (Day 1)

These 5 fixes take 30 minutes to 2 hours each and have outsized impact:

| Fix | Time | Impact |
|-----|------|--------|
| Wire `ConfigCmd::Mcp` (kill crash) | 30 min | P0 panic eliminated |
| Move share routes inside auth | 30 min | Security hole closed |
| Fix model name "-" in TUI | 1 hour | UX clarity |
| Add scrubbing to CLI Gist | 1 hour | Secret leak closed |
| Forward streaming events to TUI | 2 hours | Streaming UX |

Total: ~5 hours of work. Fixes 3 P0 issues, 1 security issue, and the
most visible UX problem.

---

## 9. Source Document Cross-Reference

Every recommendation in this plan traces to a source document:

| Recommendation | Primary Source | Supporting Sources |
|----------------|---------------|-------------------|
| Wire learning into runner v2 | 06-IMPLEMENTATION-PLANS Plan 1 | 01-LESSONS S7, 18-LEARN-ISSUES |
| Unify model selection | 06-IMPLEMENTATION-PLANS Plan 2 | 05-GAPS S9, 19-DISPATCH-ISSUES |
| Fix P0 blockers | 06-IMPLEMENTATION-PLANS Plan 3 | 11-GROUND-TRUTH S2, S4 |
| Context Packs workflow | 09-UX-WORKFLOW-VISION | 15-UX-PLAN, 15-UX-GOALS |
| Gate evolution (judge, wave, express) | 06-IMPLEMENTATION-PLANS Plan 7 | 20-GATE-PLAN, 14-GATE-VIZ-* |
| orchestrate.rs decomposition | 17-ORCH-PLAN | 17-ORCH-ISSUES, 01-LESSONS S9.3 |
| Prompt assembly fixes | 16-PROMPT-PLAN | 16-PROMPT-ISSUES, 16-PROMPT-CONTEXT-WINDOWING |
| Performance optimization | 13-PERF-OPTIMIZATION-PLAYBOOK | 13-PERF-BOTTLENECK-ANALYSIS |
| Agent memory | 10-INNOVATIONS-AND-NEW-FEATURES S1 | 18-LEARN-AUDIT, Research synthesis |
| Safety contracts | 11-GROUND-TRUTH S7.4 | 17-SAFETY-AND-SECURITY |
| A2A interop | 10-INNOVATIONS-AND-NEW-FEATURES S10 | 12-ACP-MCP-DEEP-DIVE |
| Runner operational patterns | 22-RUNNER-LESSONS | 01-LESSONS S3 |

---

## 10. Guiding Principles

These are the lessons that cost real time and should inform every implementation decision:

1. **Wire, don't build.** The fix for interactive chat was 4 CLI flags, not a
   new architecture. Before building anything, `grep` for existing implementations.

2. **Diagnostic progression.** Read reference code before designing replacements.
   "Why does Mori work?" is faster than "How should we architect this?"

3. **Two outcomes, not one.** Process exit 0 is not artifact success. Only emit
   positive learning when both process and gates pass.

4. **Defer compilation to wave boundaries.** Per-task compilation is 10-100x
   more expensive than wave-level gating with equivalent error detection.

5. **Context handoff is the hard problem.** Telling agent B what agent A changed
   matters more than telling agent B what to do.

6. **Prompt placement determines compliance.** System prompt first line: 99%.
   Context file: 95%. Prompt middle: 85%.

7. **Never delete branches or worktrees.** Disk is cheap; lost work is expensive.

8. **Manual intervention is a feature.** Result files on disk, `--continue`,
   human-readable status. The system must be operable.

9. **The cheapest model that works.** gpt-5.4-mini at $0.01/batch vs gpt-5.4
   at $0.30/batch. For 195 batches: $2 vs $60. Route by complexity.

10. **Ship incrementally.** Each phase is independently valuable. Partial
    learning is better than no learning. One pass of Context Packs is better
    than waiting for all five.
