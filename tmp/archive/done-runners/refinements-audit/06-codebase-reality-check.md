# Codebase Reality Check: Refinements vs. Actual Code

**Date**: 2026-04-17
**Auditor**: Automated code audit (Opus 4.6)
**Method**: Grep, read, and compile-check across all 32 workspace crates (~322K LOC, 3,761 tests)
**Build status**: `cargo check --workspace` passes cleanly (one unused-mut warning in mirage-rs)

---

## 1. Signal -> Engram Rename

**Refinement claim**: "877:5 ratio, essentially complete."

**Reality**: **Confirmed, mostly accurate.** The actual ratio today is ~926 Engram occurrences
across 134 files vs. 11 "Signal" occurrences across 6 files. The remaining 11 Signal references
break down as:

| File | Count | What it actually is |
|------|-------|---------------------|
| `roko-runtime/src/process.rs` | 5 | Unix `nix::Signal` (SIGTERM/SIGKILL) -- unrelated to the rename |
| `roko-neuro/src/context.rs` | 2 | Enum variant `SourceFamily::Signal` and match arm -- **straggler** |
| `roko-compose/src/system_prompt_builder.rs` | 1 | String literal `"Signal"` in a prompt template -- **straggler** |
| `roko-compose/src/context_provider.rs` | 1 | String literal `"Signal"` in a prompt template -- **straggler** |
| `roko-cli/src/plan_generate.rs` | 1 | Test assertion: `prompt.contains("Signal -> Engram")` -- meta-reference |
| `roko-cli/src/tui/views/dashboard_view.rs` | 1 | UI column header `"Signal"` -- **straggler** |

**Verdict**: 4 real stragglers out of 926+ usages. The rename is ~99.6% complete. The 4
stragglers are cosmetic (string literals in prompts and UI headers), not structural. No
`struct Signal` or `type Signal` remains anywhere in the codebase. The core type in
`roko-core/src/engram.rs` is `pub struct Engram` with BLAKE3 content-hashing, decay functions,
lineage DAG, multi-axis scoring, and attestation support (451 lines, 15 tests). This is a
real, fully-fleshed data type, not a stub.

---

## 2. Event Bus

**Refinement claim**: Bus should be promoted to a kernel trait as a "second fabric" alongside
Substrate. Refinement 03 proposes `trait Bus: Send + Sync` as a first-class kernel operator.

**Reality**: The event bus at `crates/roko-runtime/src/event_bus.rs` (428 lines, 9 tests) is:

- A **generic typed broadcast** using `tokio::sync::broadcast` + a bounded `VecDeque` replay ring
- Parameterized over `E: Clone + Send + Sync + 'static`
- Has `emit()`, `subscribe()`, `replay_from(seq)`, `sender()` (clonable producer handle)
- Has a concrete `RokoEvent` enum with 2 variants: `PlanRevision` and `PrdPublished`
- Has a `global_event_bus()` singleton via `OnceLock`

**What it actually does today**: It is a simple, well-built pub/sub with replay. It has exactly
two event types, both of which are wired:

1. `PlanRevision` -- emitted when gate failures exhaust retry budget (carries failing verdicts,
   log tail, reason)
2. `PrdPublished` -- emitted when a PRD is promoted to published state (carries slug, path,
   origin)

**Is it a "second fabric"?** No. Today it is a utility for two specific cross-cutting concerns:
plan revision events and PRD lifecycle hooks. It does not carry the volume, variety, or
real-time streaming characteristics that the refinements describe for Pulse traffic. The
refinements envision the Bus carrying agent turn events, gate verdicts, telemetry, heartbeats,
and live observability streams. Today it carries exactly 2 event types.

**Does it need to be a kernel trait?** The honest answer: not yet. What exists is a
well-designed building block (~260 lines of real code), but promoting it to a kernel trait
means every `roko-core` consumer would depend on the bus semantics. Today, `roko-runtime`
depends on `roko-core`, not the reverse. Making `Bus` a kernel trait inverts that
dependency, which is the right direction architecturally but requires moving the bus
implementation into `roko-core` or creating a new `roko-bus` crate that both `roko-core`
and `roko-runtime` depend on.

**Recommendation**: The bus is ready to grow. It is not ready to become a kernel trait in the
current crate graph without restructuring. The refinements' vision is sound but the path
requires either: (a) extracting `EventBus<E>` into a standalone `roko-bus` crate that
`roko-core` can depend on, or (b) moving the bus implementation directly into `roko-core`.
Option (a) is cleaner.

---

## 3. Layer Violations

**Refinement claim**: One confirmed dependency violation exists: `roko-conductor` (L3/L4) depends
on `roko-learn` (L2/cross-cut).

**Reality**: **Confirmed.** `roko-conductor/Cargo.toml` has `roko-learn = { path = "../roko-learn" }`
as a direct dependency. Grepping the conductor source shows the violation is narrow:

```
crates/roko-conductor/src/watchers/context_window_pressure.rs
    use roko_learn::efficiency::AgentEfficiencyEvent;
```

That's it. One import, in one watcher, for one struct. The conductor needs
`AgentEfficiencyEvent` to detect context-window pressure from efficiency telemetry. This is
exactly the kind of coupling the Bus proposal dissolves: the conductor should subscribe to
efficiency events through the bus rather than importing the type directly.

**Severity**: Low. The violation is a single struct import. But it demonstrates the pattern the
refinements correctly identify: without a shared event vocabulary on the bus, crates reach
across layers for type definitions.

The doc `docs/00-architecture/23-architectural-analysis-improvements.md` exists and contains a
thorough analysis (it is the doc that identified this violation). Its Section 2.2 proposes the
Bus-first fix for this exact coupling.

---

## 4. HDC Vectors

**Refinement claim**: 10,240-bit HDC fingerprints for episode similarity, code semantic search,
and associative memory.

**Reality**: The HDC implementation at `crates/roko-primitives/src/hdc.rs` (345 lines, 10 tests)
is **real and complete for its stated scope**:

| Feature | Status | Evidence |
|---------|--------|----------|
| 10,240-bit vector (160 x u64) | Built | `bits: [u64; 160]` |
| XOR bind (involution) | Built + tested | `bind()` + `hdc_bind_involution` test |
| Majority-vote bundle | Built + tested | `bundle()` + `hdc_bundle_tie_rule` test |
| Cyclic permutation | Built | `permute()` for sequence encoding |
| Hamming similarity [0,1] | Built + tested | `similarity()` |
| Deterministic seed expansion | Built + tested | `from_seed()` + FNV-1a hash |
| Serde roundtrip (JSON) | Built + tested | Custom `Serialize`/`Deserialize` impls |
| rkyv zero-copy similarity | Built (feature-gated) | `similarity_archived()` behind `#[cfg(feature = "rkyv")]` |
| `fingerprint(value)` | Built | Serializes any `serde::Serialize` -> HDC vector |
| `text_fingerprint(text)` | Built | Direct text -> HDC vector |

**How far from the refinements' vision?** The low-level math is done. What is NOT built:

1. **HDC-based substrate** (`HdcSubstrate` for nearest-neighbor search over a corpus): The trait
   implementation is mentioned in doc comments but no `HdcSubstrate` struct exists.
2. **HDC indexing for code intelligence**: `roko-index` exists (parser + graph + HDC indexing in
   the Cargo.toml description) but its actual HDC usage needs separate audit.
3. **Per-episode HDC fingerprinting**: **Wired.** `roko-learn/src/hdc_fingerprint.rs` provides
   `fingerprint_episode(prompt, outcome)` and it IS called from `orchestrate.rs` (line 2920).
   Episodes get HDC fingerprints attached before logging.
4. **HDC clustering**: `roko-learn/src/hdc_clustering.rs` exists as a module. Not audited
   deeply here but the file exists in the learning crate.

**Verdict**: The HDC primitives are real, tested, and already wired for episode fingerprinting.
The gap is in using HDC for semantic search/retrieval (the substrate side), not in the vector
math itself.

---

## 5. Learning Subsystem Reality

**Refinement claim**: Prediction errors, active inference, continuous calibration, and
cybernetic feedback loops.

**Reality**: The learning subsystem at `crates/roko-learn/src/` is **the most substantial
subsystem in the project** (35,847 lines across 42 modules). Here is what actually exists:

### What is real and wired

| Module | Lines | Wired? | What it does |
|--------|-------|--------|--------------|
| `cascade_router.rs` | 4,784 | Yes | 3-stage model router (static -> confidence -> UCB1 bandit) |
| `runtime_feedback.rs` | 2,720 | Yes | Post-task learning updates: updates router, playbooks, skills |
| `skill_library.rs` | 2,520 | Yes | Extracts, stores, and queries reusable skills from episodes |
| `model_router.rs` | 2,173 | Yes | LinUCB contextual bandit for model selection |
| `episode_logger.rs` | 1,979 | Yes | Append-only JSONL episode records with usage/verdicts |
| `cfactor.rs` | 1,847 | Yes | Collective intelligence factor: fleet-level quality metric |
| `bandits.rs` | 1,727 | Yes | UCB1 and Thompson sampling bandits |
| `playbook_rules.rs` | 1,355 | Yes | Rule-based playbook matching |
| `costs_db.rs` | 1,224 | Yes | Per-model cost tracking database |
| `efficiency.rs` | 1,176 | Yes | Per-turn efficiency events (tokens, latency, cost) |
| `provider_health.rs` | 1,117 | Yes | Circuit breaker for LLM providers |
| `pattern_discovery.rs` | 977 | Yes | Episode pattern mining |
| `playbook.rs` | 976 | Yes | Playbook storage and retrieval |
| `active_inference.rs` | ~200 | Yes | Bayesian belief state + expected free energy tier selection |
| `prediction.rs` | ~200 | Yes | Prediction records with residual tracking |
| `anomaly.rs` | exists | Yes | Runaway loop, cost spike, quality degradation detection |
| `budget.rs` | exists | Yes | Budget guardrails |
| `hdc_fingerprint.rs` | exists | Yes | Episode HDC fingerprint encoding |
| `hdc_clustering.rs` | exists | Partial | HDC-based episode clustering |
| `drift.rs` | exists | Partial | Distribution drift detection |
| `regression.rs` | exists | Partial | Regression detection |

### Active inference reality

The refinements propose "active inference with prediction errors and continuous model updating."
The actual `active_inference.rs` is ~200 lines implementing:

- A 90-state belief distribution (3 difficulty x 3 skill x 10 confidence levels)
- Bayesian belief update after each task observation (success, cost, latency)
- Expected free energy minimization for tier selection (Fast/Standard/Premium)
- Integration with the cascade router (`select_tier_with_belief`)

This is **real active inference**, not a placeholder. It is not Karl Friston's full Free Energy
Principle, but it is a working Bayesian tier selector that updates its beliefs from outcomes.
The code uses proper likelihood computation, normalization, and expected free energy ranking.

### Prediction error tracking

`prediction.rs` implements `PredictionRecord` with explicit fields for:
- `predicted_success_prob`, `predicted_cost_usd`, `predicted_duration_ms`
- `actual_success`, `actual_cost_usd`, `actual_duration_ms`
- `residual_success`, `residual_cost`, `residual_duration`

This is the prediction-error signal the refinements want. It exists. It is wired through the
routing log and calibration tracker.

### What the refinements propose that does NOT exist yet

1. **Automatic reweighting of scorer axes from prediction residuals** -- the residuals are
   computed but not fed back into scorer weights automatically
2. **Cross-agent prediction markets** -- not built
3. **Dreaming-phase automatic consolidation** -- the DreamRunner exists (5,964 lines) and IS
   wired from `orchestrate.rs` (auto-dream triggers after enough episodes), but the
   "imagination" and "hypnagogia" sub-phases are aspirational

**Verdict**: The learning subsystem is the most sophisticated part of the codebase. The gap
between refinement claims and reality is smaller here than anywhere else. Active inference is
real. Prediction tracking is real. The cascade router is real. The missing pieces are second-order
feedback loops (auto-reweighting, cross-agent prediction markets), not the core learning
machinery.

---

## 6. Safety Layer Reality

**Refinement claim (doc 32)**: Comprehensive safety with sandbox isolation, provenance chains,
formal contracts, and behavioral monitoring.

**Reality**: The safety layer at `crates/roko-agent/src/safety/` (4,465 lines across 10 files)
is **substantive and wired**:

### What actually exists

| Module | What it does | Wired? |
|--------|-------------|--------|
| `mod.rs` (SafetyLayer) | Aggregates all policies, `check_pre_execution()` chains them | Yes |
| `bash.rs` | Command allowlist/denylist for bash/run_tests tools | Yes |
| `git.rs` | Branch-protection policy (blocks force-push to main, etc.) | Yes |
| `network.rs` | Outbound destination allowlist (blocks private IPs, SSRF) | Yes |
| `path.rs` | Worktree-relative path canonicalization, escape prevention | Yes |
| `scrub.rs` | Secret scrubbing from outputs (API keys, tokens) | Yes |
| `rate_limit.rs` | Per-tool, per-role rate limiting | Yes |
| `capabilities.rs` | OCaps-style warrants (`AgentWarrant`, `Capability` enum) | Yes |
| `contract.rs` | Declarative behavioral contracts (invariants, governance, recovery) | Yes |
| `contracts/*.yaml` | Role-specific contracts (implementer, researcher, reviewer) | Yes |

The `SafetyLayer::check_pre_execution()` method chains 5 policy checks in order:
1. Role-local tool whitelist
2. Rate limit
3. OCaps warrant check
4. Bash/git command policy
5. Network/path policy

The `AgentWarrant` system implements proper capability tokens with:
- Unforgeable random IDs (`[u8; 32]`)
- Explicit capability grants (Tool, ReadPath, WritePath, Exec, Network)
- Issuer tracking
- Expiry timestamps
- Delegation depth limiting

The `AgentContract` system implements declarative behavioral contracts with:
- Invariants (MaxTokensPerTurn, RequireGateBeforeCommit)
- Governance rules (MaxToolCallsPerTurn, ForbiddenTools, RequireToolBeforeEdit)
- Recovery actions (Abort on contract violation)
- Role-specific contracts loaded from YAML assets

### What the refinements propose beyond current state

1. **Process-level sandboxing (containers, VMs)**: Not built. Safety is enforcement within the
   Rust process, not OS-level isolation.
2. **Formal verification of safety properties**: Not built. Current safety is runtime checks,
   not compile-time proofs.
3. **Provenance chain on-chain attestation**: The `Attestation` type exists in `roko-core`
   (Ed25519 signatures, `ChainAttestation` with chain_id/tx_hash/block_number) but is not
   wired to an actual chain submission path.
4. **Multi-party approval workflows**: Not built.

**Verdict**: The safety layer is real and enforced. It is not a stub. The OCaps warrant system
and declarative contracts are genuinely sophisticated. The gap is in OS-level sandboxing and
on-chain attestation, not in the application-level safety enforcement.

---

## 7. TUI / Serve Reality

### TUI (`crates/roko-cli/src/tui/`)

**Size**: 58,053 lines across 30+ files (plus subdirectories for views, widgets, pages, modals)

**Structure**: Full ratatui application with:
- `app.rs` -- main event loop
- `tabs.rs` -- F1-F7 tab navigation
- `views/` -- dashboard, verdicts, operations views
- `widgets/` -- reusable UI components
- `pages/` -- full-page views
- `modals/` -- popup dialogs
- `theme.rs` -- theming
- `postfx.rs`, `postfx_pipeline.rs` -- post-processing visual effects
- `atmosphere.rs` -- ambient visual effects
- `ws_client.rs` -- WebSocket client for live updates

This is NOT scaffolding. 58K lines of TUI code with WebSocket integration, post-processing
effects, and multiple view modes is a significant built artifact.

### HTTP Serve (`crates/roko-serve/src/`)

**Size**: 30,652 lines across 18 route modules

**Route count**: ~214 route registrations across 18 modules:
- `status.rs` -- 68 routes (health, metrics, signals, episodes, etc.)
- `aggregator.rs` -- 46 routes (fleet-wide aggregation)
- `webhooks.rs` -- 18 routes
- `learning.rs` -- 15 routes
- `prds.rs` -- 13 routes
- `agents.rs` -- 10 routes
- `providers.rs` -- 7 routes
- `deployments.rs` -- 7 routes
- `plans.rs` -- 6 routes
- `research.rs` -- 6 routes
- plus SSE, WebSocket, config, diagnosis, templates, subscriptions, run

The server uses axum with:
- CORS middleware
- API key authentication (configurable)
- Secret-scrubbing middleware layer
- SSE for live streaming
- WebSocket for bidirectional communication
- OpenAPI spec generation

**Verdict**: Both TUI and serve are substantive, not scaffolded. The "~85 routes" claim in the
CLAUDE.md understates the actual count (closer to 200+). Whether all 200+ routes return real
data vs. stubs would require individual route testing, but the infrastructure is undeniably
built.

---

## 8. Orchestration Loop

**Refinement claim (doc 05)**: The nine-step loop should become seven steps: SENSE, ASSESS,
COMPOSE, ACT, VERIFY, PERSIST/BROADCAST, REACT.

**Reality**: Two loops exist in the codebase:

### Loop 1: `roko-core/src/loop_tick.rs` (the pure kernel loop)

A clean 5-step function (158 lines total, 98 lines of implementation):

```
1. Query substrate for candidates
2. Router selects one
3. Composer builds composed signal
4. Gate verifies
5. If passed: persist + run policy reactions
```

This is the canonical `loop_tick()` function that takes all six traits as parameters. It is
used by `roko run <prompt>` via `run.rs`. It is **pure** -- no I/O decisions, no bus, no
learning feedback. This is genuinely elegant.

### Loop 2: `crates/roko-cli/src/orchestrate.rs` (the plan-driven runtime loop)

A **17,087-line** monster that implements the full plan execution lifecycle:

```
1. Discover plans -> build executor DAG
2. Tick executor -> get ExecutorActions
3. Dispatch actions (SpawnAgent, RunGate, etc.)
4. For SpawnAgent: compose system prompt, spawn agent process, collect output
5. For implementing: run gate pipeline (7-rung dispatch)
6. Record episodes, emit efficiency events, update learning
7. Auto-save state, check shutdown signals
8. Repeat until all plans terminal
```

This is NOT the refinements' seven-step loop. It is a plan-execution runtime that happens to
use the six traits internally. The actual flow is:

- `PlanRunner::run_all()` -- outer loop with tick/dispatch/autosave
- `dispatch_action()` -- big match on ExecutorAction variants
- `handle_implementing()` -- spawns agent, runs gate pipeline
- `run_gate_pipeline()` -- 7-rung gate dispatch with adaptive thresholds
- Episode logging, efficiency events, learning updates at each step
- Dream consolidation (auto-dream) triggered after enough episodes
- Daimon affect updates after each task outcome
- Conductor watcher loop running in parallel

The orchestrate.rs file is also where ALL the cross-crate wiring happens. It imports from
18+ crates and orchestrates their interaction. This is where the "WIRE, don't build"
principle manifests: the individual crates are clean, and orchestrate.rs is the integration
point.

**How does this compare to the refinements' seven-step loop?**

| Refinement Step | Current Implementation |
|-----------------|----------------------|
| 1. SENSE | `Substrate.query` in `loop_tick`; no Bus subscription in the kernel loop |
| 2. ASSESS | `Router.select` in `loop_tick`; cascade router in orchestrate.rs |
| 3. COMPOSE | `Composer.compose` in `loop_tick`; system prompt builder in orchestrate.rs |
| 4. ACT | Agent dispatch in orchestrate.rs (not in kernel loop) |
| 5. VERIFY | `Gate.verify` in `loop_tick`; 7-rung pipeline in orchestrate.rs |
| 6. PERSIST | `Substrate.put` in `loop_tick`; JSONL logging in orchestrate.rs |
| 6b. BROADCAST | Event bus emit in orchestrate.rs (2 event types only) |
| 7. REACT | `Policy.decide` in `loop_tick`; conductor watchers in orchestrate.rs |

**Verdict**: The pure kernel loop (`loop_tick`) is clean and could adopt the seven-step
reframing with minimal changes. The plan-execution loop in `orchestrate.rs` is a different
beast -- it is an integration harness, not a cognitive loop. The refinements should be clear
about which loop they are restructuring. The kernel loop is 98 lines and amendable. The
runtime harness is 17K lines and should NOT be restructured to match a conceptual model.

---

## 9. Crate Count and Dependencies

**Current state**: 32 crates + 3 apps + 1 test crate = 36 workspace members

**Workspace members by category**:
- Kernel: `roko-core`, `roko-primitives`, `roko-runtime` (3)
- Standard impls: `roko-std`, `roko-gate`, `roko-fs`, `roko-compose` (4)
- Agent: `roko-agent`, `roko-agent-server` (2)
- Orchestration: `roko-orchestrator`, `roko-conductor` (2)
- Learning: `roko-learn` (1)
- Knowledge: `roko-neuro` (1)
- Behavioral: `roko-daimon`, `roko-dreams` (2)
- CLI/Server: `roko-cli`, `roko-serve` (2)
- MCP: `roko-mcp-stdio`, `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-code` (5)
- Chain: `roko-chain` (1)
- Language: `roko-lang-rust`, `roko-lang-typescript`, `roko-lang-go` (3)
- Indexing: `roko-index` (1)
- Plugin: `roko-plugin` (1)
- Demo: `roko-demo` (1)
- Apps: `mirage-rs`, `agent-relay`, `roko-chain-watcher` (3)
- Tests: `tests` (1)

**Is adding 3 more kernel crates (roko-bus, roko-hdc, roko-spi) sane?**

Adding `roko-bus`: **Sane if extracted from roko-runtime.** The EventBus code is ~260 lines of
real implementation. Extracting it would fix the L3->L2 dependency violation and let `roko-core`
depend on bus semantics without importing the full runtime. Cost: one more crate. Benefit:
clean dependency direction.

Adding `roko-hdc`: **Probably unnecessary.** HDC is already in `roko-primitives` (345 lines).
The primitives crate is tiny (3 files: `lib.rs`, `hdc.rs`, `tier.rs`). Extracting HDC into its
own crate would create a crate with one file. The benefit is unclear unless other crates need
HDC without pulling in the tier router.

Adding `roko-spi` (Service Provider Interface): **Questionable without a clear consumer.** The
current plugin system is in `roko-plugin` (already exists). Adding an SPI crate implies a
stable plugin API, which the project is not yet ready to commit to.

**Verdict**: Extract `roko-bus` -- it solves a real dependency problem. Leave HDC in primitives.
Defer SPI until there are external plugin consumers. The project already has 36 workspace
members; adding crates should solve concrete problems, not satisfy taxonomic completeness.

---

## 10. State of the Art: What Works vs. What's Scaffolding

### Definitively works end-to-end (code exists AND is called from CLI)

| Capability | Evidence |
|------------|----------|
| `roko run "<prompt>"` | `run.rs`: compose -> agent -> gate -> persist -> episode |
| `roko plan run <dir>` | `orchestrate.rs`: discover plans -> DAG executor -> parallel dispatch |
| `roko prd idea/draft/plan` | PRD lifecycle with agent-driven drafting and plan generation |
| `roko research topic/enhance-*` | Research agent with citation tracking |
| `roko dashboard` | 58K-line ratatui TUI with F1-F7 tabs and WebSocket |
| `roko serve` | 200+ HTTP routes on :6677 with axum |
| `roko chat --agent <id>` | Per-agent sidecar communication |
| Plan DAG with parallel execution | `ParallelExecutor` in `roko-orchestrator` |
| 7-rung gate pipeline | `rung_dispatch.rs` with adaptive thresholds |
| 8+ LLM backends | Claude CLI, Claude API, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity |
| MCP config passthrough | `agent.mcp_config` in roko.toml -> `--mcp-config` flag |
| Episode logging with HDC fingerprints | JSONL append + per-episode HDC vector |
| Cascade model router | 3-stage (static -> confidence -> UCB1) with persistence |
| Safety enforcement | 6 policy families + OCaps warrants + declarative contracts |
| Prompt experiment A/B testing | `ExperimentStore` with static overrides |
| Adaptive gate thresholds | EMA per rung in `.roko/learn/gate-thresholds.json` |
| Conductor watchers | 10 watchers running in parallel during plan execution |
| Daimon affect modulation | PAD state, somatic markers, dispatch parameter tuning |
| Dream consolidation | Auto-triggered after N episodes; clusters episodes into knowledge |
| Active inference tier selection | Bayesian belief state + expected free energy routing |
| Prediction error tracking | Residual computation for success/cost/duration |
| C-Factor fleet metric | Collective intelligence score computed and persisted |

### Built but wiring quality varies

| Capability | Status |
|------------|--------|
| On-chain attestation | `Attestation` type with Ed25519 exists; no chain submission path |
| HDC substrate search | `HdcVector` math complete; no `HdcSubstrate` implementation |
| Code intelligence MCP | `roko-mcp-code` exists as a crate; wiring completeness unclear |
| Language providers | `roko-lang-{rust,typescript,go}` exist; usage from main flow unclear |
| Plugin SDK | `roko-plugin` exists with `EventSource`/`FeedbackCollector`; no known plugins |
| Cross-plan dependencies | Code exists in executor; no evidence of cross-plan dep execution |
| Merge queue / post-merge | `PostMergeRunner` exists and is called from orchestrate.rs |

### Phase 2+ (built structures, not wired to runtime)

| Capability | Status |
|------------|--------|
| `roko-chain` chain witness | Primitives exist; no production chain integration |
| `roko-demo` scenario orchestrator | Scaffolded for demo environments |
| `mirage-rs` EVM simulator | Standalone app, optionally bridges to roko-core |
| Full dream cycle (hypnagogia/imagination) | DreamRunner calls `consolidate_now()`; advanced phases are aspirational |

### The honest summary

The codebase is **far more real than typical architecture documents suggest**. The plan-execute-
gate-persist loop genuinely works. Agents actually spawn and produce output. Gates actually
compile, test, and clippy-check code. Episodes are actually logged with HDC fingerprints.
The cascade router actually routes between models using bandit algorithms. The conductor
actually watches for stuck patterns, cost overruns, and ghost turns. The daimon actually
modulates dispatch parameters based on PAD affect state.

The main risks the refinements should be aware of:

1. **orchestrate.rs is 17K lines.** This is the integration hairball. It works, but it is the
   single-most-fragile file in the project. The refinements' architectural improvements
   should focus on decomposing this file, not adding new abstractions.

2. **The kernel loop (`loop_tick`) and the runtime loop (orchestrate.rs) are different things.**
   The refinements sometimes conflate them. `loop_tick` is 98 lines of pure functional code.
   `orchestrate.rs` is 17K lines of imperative integration. Restructuring the kernel loop is
   easy. Restructuring the runtime loop is a multi-month project.

3. **322K LOC across 32 crates is already substantial.** Adding crates has a real cost in
   compile time, dependency management, and cognitive load. Each proposed new crate should
   justify itself against the alternative of adding a module to an existing crate.

4. **3,761 tests exist but test coverage is uneven.** The kernel, primitives, and learning crates
   are well-tested. The CLI/orchestrate/TUI/serve code has fewer tests relative to its size.

5. **The "built but not wired" pattern is mostly resolved.** The CLAUDE.md says "WIRE, don't
   build" because this was historically the main failure mode. Today, most major subsystems
   ARE wired. The remaining unwired pieces are genuinely Phase 2+ (chain integration, full
   dream cycle, external plugins).
