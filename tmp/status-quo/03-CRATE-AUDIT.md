# 03 — Crate Audit (Exhaustive Per-Package Reference)

**Verification:** HEAD `5852c93c05`, branch `main`, 2026-07-08.
**Scope:** all 34 workspace packages — 31 crates under `crates/`, 3 apps under `apps/`
(plus the `tests/` integration member, noted at the end). Every package's `src/lib.rs`
and `Cargo.toml` were read for this pass. LOC = `wc -l` over `src/**/*.rs` (includes
inline tests + doc comments), so it overstates "product" code but is the honest tree size.

> **Supersedes the 2026-04 CLAUDE.md "31 crates, ~177K LOC" figure**, which is stale by
> ~4×. Real `src` tree is ~700K LOC. `roko-cli` alone is 179K LOC / 249 files.

Status tags used throughout:
- **Live** — reached by the default CLI runtime (`roko` binary / `roko-serve` / MCP servers) on a normal invocation. Call site cited.
- **Partial** — compiled and reachable, but the live path only exercises a subset (dry-run, feature-gated, or one entry point of several).
- **Unwired** — compiles, has no external caller on the live path; only its own tests / a dead module reach it.
- **Legacy** — reached only through `orchestrate.rs` (the pre-Runner-v2 executor, dead by default).
- **App** — standalone binary, not in `default-members`; "live" only when run directly.

---

## 0. The two facts that reframe this whole audit

1. **Canonical noun is `Engram`, not `Signal`.** `roko-core/src/engram.rs` defines `Engram`;
   `Signal`/`Store` are thin aliases (`roko-core/src/signal.rs`, `signal_kinds.rs`). Every crate
   still mixes both names. This is the #1 cosmetic-but-pervasive debt.

2. **The live engine is Runner v2 (`roko-cli/src/runner/`), not `orchestrate.rs`.**
   `crate::runner::run` (`runner/event_loop.rs`) is the executor called by every live entry point:
   - `roko-cli/src/commands/plan.rs:654` (`plan run`)
   - `roko-cli/src/commands/do_cmd.rs:616` (`do`)
   - `roko-cli/src/prd.rs:956` (`prd plan` execution)
   - `roko-cli/src/serve_runtime.rs:304` (server-triggered runs)
   - `roko-cli/src/worker/cloud.rs:462` (cloud worker)

   `orchestrate.rs` (22K+ LOC) still compiles and exports `PlanRunner` / `OrchestrationReport`
   via `roko-cli/src/lib.rs:156`, but **every** `PlanRunner::*` reference lives inside
   `orchestrate.rs` itself (mostly `#[cfg(test)]`, verified 2026-07-08). No command dispatches to
   it. Treat `orchestrate.rs` as **Legacy / dead-by-default**. It even contains a
   `run_with_v2_engine` (`orchestrate.rs:8369`) that delegates to `roko-graph`, but that path is
   also not the live one.

---

## Package Summary Table

LOC/files from `src` tree at HEAD. "Dep cycle?" flags dev-dependency back-edges (no runtime cycle).

| # | Package | LOC | files | Layer | Live status | Verdict |
|---|---------|-----|-------|-------|-------------|---------|
| 1 | roko-core | 51.8K | 115 | Kernel | **Live** | Keep (split) |
| 2 | roko-primitives | 4.7K | 10 | Kernel | **Partial** | Keep |
| 3 | roko-fs | 5.5K | 14 | Storage | **Live** | Keep |
| 4 | roko-runtime | 18.8K | 25 | Runtime | **Partial** | Keep (prune) |
| 5 | roko-std | 6.9K | 33 | Std impls | **Live** | Keep |
| 6 | roko-gate | 21.5K | 42 | Verify | **Live** | Keep |
| 7 | roko-compose | 26.5K | 53 | Compose | **Live** | Keep (prune) |
| 8 | roko-plugin | 1.7K | 2 | SPI | **Unwired** | Quarantine |
| 9 | roko-agent | 80.0K | 160 | Agent | **Live** | Keep (split) |
| 10 | roko-agent-server | 3.8K | 14 | Agent | **Live** | Keep/merge? |
| 11 | roko-learn | 58.6K | 66 | Learning | **Live** | Keep (prune) |
| 12 | roko-neuro | 16.6K | 10 | Knowledge | **Live** | Keep |
| 13 | roko-dreams | 13.7K | 26 | Knowledge | **Partial** | Keep (gate P2) |
| 14 | roko-daimon | 7.3K | 7 | Affect | **Live** | Keep (gate P2) |
| 15 | roko-conductor | 10.1K | 24 | Reactive | **Legacy** | Merge/rewire |
| 16 | roko-orchestrator | 20.8K | 30 | Runtime | **Legacy** | **Quarantine** |
| 17 | roko-graph | 4.4K | 18 | Runtime | **Partial** | Keep or fold |
| 18 | roko-chain | 23.4K | 40 | Chain | **Partial** | Keep (split) |
| 19 | roko-serve | 61.4K | 100 | App/HTTP | **Live** | Keep (prune) |
| 20 | roko-acp | 15.9K | 15 | App/editor | **Live** | Keep |
| 21 | roko-index | 4.6K | 7 | Intelligence | **Partial** | Keep |
| 22 | roko-lang-rust | 1.4K | 2 | Language | **Live** | Keep |
| 23 | roko-lang-typescript | 0.9K | 1 | Language | **Live** | Keep |
| 24 | roko-lang-go | 0.7K | 1 | Language | **Live** | Keep |
| 25 | roko-mcp-code | 1.9K | 2 | MCP | **Live** | Keep |
| 26 | roko-mcp-stdio | 0.3K | 1 | MCP | **Live** | Keep |
| 27 | roko-mcp-github | 3.2K | 1 | MCP | **App** | Keep |
| 28 | roko-mcp-slack | 1.1K | 1 | MCP | **App** | Keep/merge |
| 29 | roko-mcp-scripts | 0.8K | 1 | MCP | **App** | Keep/merge |
| 30 | roko-cli | 179.3K | 249 | App | **Live** | Keep (**shrink**) |
| 31 | roko-demo | 5.9K | 21 | Demo | **Partial** | Move under apps |
| 32 | mirage-rs (app) | 33.3K | 45 | App/chain | **App** | Keep (separate) |
| 33 | agent-relay (app) | 1.7K | 6 | App/relay | **App** | Keep/merge |
| 34 | roko-chain-watcher (app) | 2.9K | 7 | App/chain | **App** | Merge candidate |

Rough src total: **~700K LOC**. Three crates (`roko-cli`, `roko-agent`, `roko-serve`) are 44% of it.

---

# Tier 1 — Kernel

## 1. roko-core — 51.8K LOC / 115 files — **Live**
**Purpose.** The kernel: the `Engram` noun, the six verb traits (`Store`, `Score`, `Verify`, `Route`, `Compose`, `React`), config schema, error taxonomy, tool system, and a large grab-bag of shared domain types (affect, chain, conductor, job, phase, prediction…).

**Key public types / entry points.**
- `engram.rs:24` `Engram` (with `HdcFingerprint { vector: HdcVector }` at line 24, `bind`/`bundle` helpers 272–289).
- `traits.rs` — the six verb traits (`Score` re-exported).
- `signal.rs`, `signal_kinds.rs` — `Signal`/`Store` **aliases** over Engram.
- `config/` — `RokoConfig` schema + `loader::load_config_unified` (the canonical config resolver Runner v2 falls back to).
- `loop_tick.rs` — `loop_tick()` reference implementation of the universal loop.
- 60+ `pub mod`s including `cell`, `dispatch_plan`, `verdict`, `tool`, `job`, `conductor`, `affect`, `isfr_feed`, `foundation`, `obs`, `pulse`, `runtime_event`.

**Wired status.** Live — every crate depends on it. HDC is compiled **in** here: `roko-primitives` is a **non-optional** dep and `HdcVector` is used directly in `engram.rs` (42 refs across core). This nuances the "HDC compiled out" claim: it is compiled out only in the crates where `roko-primitives` is `optional` (see #2).

**Internal tech debt.**
- `loop_tick()` defined but never called on the live path (Runner v2 hand-rolls the loop).
- Dual naming: `Engram` vs `Signal`/`Store` aliases everywhere.
- Grab-bag: core carries runtime/state/bus/obs types (`state_hub.rs`, `pulse_bus.rs`, `obs/`) that duplicate or belong in `roko-runtime` (see boundary problems in doc 54).
- 115 files in a "kernel" is a smell — it has absorbed domain types from many layers.

**Boundary problems.** Core hosts `state_hub`/`pulse_bus`/`obs` runtime-ish code that overlaps `roko-runtime`'s canonical `StateHub`. Core also carries chain (`isfr_feed`, `attestation`) and affect types that arguably belong to their domain crates but are here to avoid cycles.

**Verdict: Keep, but split.** Extract the runtime/state/bus copies out; keep config schema + traits + Engram as the true kernel. Highest-value cleanup after the Engram rename.

## 2. roko-primitives — 4.7K LOC / 10 files — **Partial**
**Purpose.** Pure compute primitives, zero workspace deps: HDC vectors, tier routing, and a math shelf (`manifold`, `sheaf`, `tda`, `tropical`, `robust_stats`, `pad`, `codebook`).

**Key public types.** `hdc.rs` — `HdcVector`, `HDC_BITS` (10,240), `BundleAccumulator`, `ItemMemory`; `tier.rs` — `InferenceTier` (T0/T1/T2), `TierRouter`; plus `manifold`/`sheaf`/`tda`/`tropical` (exotic, mostly unused).

**Wired status.** **Partial.** Non-optional in `roko-core` and `roko-learn` (HDC fingerprints on episodes are real — `roko-learn/src/hdc_fingerprint.rs`). But it is `optional = true` in `roko-fs`, `roko-compose`, `roko-neuro`, `roko-serve` — so HDC-dependent code in those crates is **feature-gated and off by default** (this is the accurate reading of "HDC compiled out"). The `manifold/sheaf/tda/tropical` modules have essentially no live consumers.

**Debt.** Half the crate (topology/tropical algebra) is speculative math with no caller. Feature-gating in downstreams means two build configurations to reason about.

**Boundary.** Clean by design (no workspace deps). Good.

**Verdict: Keep.** Quarantine or delete the unused math modules; make the HDC feature decision explicit (on or gone) in fs/compose/neuro/serve.

## 3. roko-fs — 5.5K LOC / 14 files — **Live**
**Purpose.** Filesystem-backed `Store`: append-only JSONL substrate under `.roko/`, GC, layout helpers, tool-audit/metrics sinks, cold-substrate archival.

**Key public types.** `file_substrate.rs` `FileSubstrate`; `layout.rs` (canonical `.roko` path helpers); `gc.rs`; `cold_substrate.rs`; `tool_audit.rs`, `tool_metrics_sink.rs`, `trace_sink.rs`; `bandit.rs`, `archive.rs`.

**Wired status.** Live — `roko-cli` and `roko-serve` depend on it; the substrate is the persistence layer for signals/episodes. `cold_substrate.rs` is **built but not instantiated at runtime** (no cron/trigger — matches CLAUDE.md item #14).

**Debt.** `roko-primitives` optional dep gates pointer/HDC features. `cold_substrate` dormant. `bandit.rs` here overlaps bandit logic in `roko-learn`.

**Boundary.** Mild: `bandit`/`metrics` here vs `roko-learn` / `roko-runtime` equivalents.

**Verdict: Keep.** Enforce layout through helpers; wire or delete `cold_substrate`; dedupe bandit.

## 4. roko-runtime — 18.8K LOC / 25 files — **Partial**
**Purpose.** Shared async runtime primitives: event bus, process supervision, cancellation, metrics, `StateHub` projection, heartbeat, energy/theta/delta/demurrage consumers, and a `workflow_engine`.

**Key public types.** `state_hub.rs` `StateHub` (the canonical live projection hub); `process.rs` `ProcessSupervisor`; `event_bus.rs`; `pulse_bus.rs`; `cancel.rs`; `metrics.rs`; `workflow_engine.rs`; consumer modules (`theta_consumer`, `delta_consumer`, `demurrage_consumer`, `effect_driver`, `energy`, `heartbeat*`).

**Wired status.** **Partial.** `StateHub` + `ProcessSupervisor` are Live (Runner v2 + serve use them). `workflow_engine.rs` is **dead** (no live caller). Several consumers (`theta/delta/demurrage`, `effect_driver`) are dormant.

**Debt.** `workflow_engine` unused; consumer zoo mostly unwired; doc header still claims "no domain types" but the crate has drifted.

**Boundary problems (layer inversion).** `roko-runtime` depends on `roko-gate`, `roko-compose`, and `roko-learn` (verified in `Cargo.toml`). A "foundational runtime" crate must not depend on gate/compose/learn — concrete runners should be injected. This is the sharpest layer inversion in the tree.

**Verdict: Keep, prune, invert deps.** Make `StateHub`/event-envelope the single source of truth; delete `workflow_engine` and dead consumers; remove the gate/compose/learn edges.

## 5. roko-std — 6.9K LOC / 33 files — **Live**
**Purpose.** Kernel-adjacent trait impls: `MemorySubstrate`, `NoOp` impls of all six traits, composite scorers/routers, and the **builtin tool handlers**.

**Key public types.** `memory.rs` `MemorySubstrate`; `noop.rs`; `scorer.rs` (`SumScorer`/`MulScorer`/`ConstScorer`); `router.rs` (`First`/`HighestScore`/`RoundRobin`); `tool/` (builtin tool registry + handlers); `roles.rs`; `math.rs`; `trace_sink.rs`.

**Wired status.** Live — depended on by `roko-agent`, `roko-gate`, `roko-serve`, `roko-acp`, `roko-cli`. Builtin tools are dispatched through the agent tool loop.

**Debt.** Documented day-one stub/noop tool handlers that silently succeed instead of returning explicit "unsupported" errors. Builtin tool count has drifted across docs (16 std + chain + ISFR variants).

**Boundary.** `roko-std → roko-chain` (chain tool handlers live here). Splitting default/noop primitives from chain-coupled tool handlers would relieve layer pressure.

**Verdict: Keep.** Make stub tools error explicitly; register metrics/audit consistently; consider a `roko-tools` split from `roko-std` defaults.

---

# Tier 2 — Verify / Compose / SPI

## 6. roko-gate — 21.5K LOC / 42 files — **Live**
**Purpose.** The verification stack: concrete gates, the 7-rung pipeline + rung selector/dispatch, adaptive thresholds, and statistical process control (SPC/PELT/Hotelling/ratchet).

**Key public types.** Rung-dispatched (7): `CompileGate`, `ClippyGate`, `TestGate`, `SymbolGate`, `GeneratedTestGate`+`VerifyChainGate`, `PropertyTestGate`+`FactCheckGate`, `LlmJudgeGate`+`IntegrationGate`. Standalone (6): `DiffGate`, `CodeExecutionGate`, `ShellGate`, `BenchmarkRegressionGate`, `FormatCheckGate`, `SecurityScanGate`. Composition: `ParallelGate`, `VotingGate`, `FallbackGate`. Infra: `rung_dispatch.rs`, `rung_selector.rs`, `gate_pipeline.rs`, `gate_service.rs`, `adaptive_threshold.rs`, `registry.rs`.

**Wired status.** Live — Runner v2 dispatches gates per task via `roko-cli/src/runner/gate_dispatch.rs`. Adaptive thresholds persist to `.roko/learn/gate-thresholds.json`.

**Debt.** Some gates carry "stub-pass" risk (return Pass when the tool is absent). SPC modules (`pelt`, `hotelling`, `spc`, `ratchet`) are heavy and lightly exercised.

**Boundary problem.** `roko-gate → roko-agent` (LLM-judge/fact-check gates need a dispatcher). This makes the verify layer depend on the agent layer — acceptable but pins gate to agent's build. It is also *why* `roko-runtime → roko-gate` is a bad edge (runtime transitively pulls agent).

**Verdict: Keep.** Ensure Graph and Runner use the same gate contract; make absent-tool gates fail explicitly, not pass.

## 7. roko-compose — 26.5K LOC / 53 files — **Live**
**Purpose.** Prompt/context assembly: 9-layer `SystemPromptBuilder`, role templates, token budgeting, context mesh/foraging, and a **VCG auction** for context allocation.

**Key public types.** `system_prompt_builder.rs` `SystemPromptBuilder`; `prompt.rs` `PromptComposer`; `templates/` (role templates); `attention.rs` (`AttentionBidder` variants); `auction.rs` (`vcg_allocate` — built + exported); `budget.rs`, `budget_predictor.rs`, `token_counter.rs`; `context_assembler.rs`, `context_mesh.rs`, `foraging.rs`, `enrichment.rs`, `prompt_assembly_service.rs`.

**Wired status.** Live — `RoleSystemPromptSpec` in Runner v2 builds prompts through this. **VCG is Partial:** `vcg_allocate` is exported but the greedy attention path dominates at runtime (matches CLAUDE.md).

**Debt.** VCG dead-ish; large surface (`cognitive_workspace`, `symbol_resolver`, `compaction`, `strategy`) with uneven live coverage.

**Boundary.** `roko-compose → roko-agent, roko-learn, roko-neuro` (direct concrete deps). Longer-term these should be trait boundaries; the `roko-learn` back-edge is a **dev-dependency** (no runtime cycle).

**Verdict: Keep, prune.** Decide VCG (default-on or delete stale docs); narrow the agent/learn/neuro coupling toward traits.

## 8. roko-plugin — 1.7K LOC / 2 files — **Unwired**
**Purpose.** Plugin/extension SPI facade for event sources + feedback collectors.

**Key public types.** Only `pub mod manifest;` is exported — the crate is effectively a manifest schema plus deps (`cron`, `notify`, `globset`) that hint at an intended runtime that was never built.

**Wired status.** **Unwired.** `roko-cli` and `roko-serve` depend on it, but it exposes little beyond `manifest`. No live plugin loading loop.

**Debt.** A facade: deps present for a runtime that doesn't exist here. This is the clearest "3-file facade crate."

**Boundary.** Overlaps the extension-loading logic that actually lives in `roko-cli/src/runner/extension_loader.rs` and `roko-core/src/extension.rs`.

**Verdict: Quarantine** until the extension SPI is actually decided; do not grow an unused SPI. Possible delete if `roko-core::extension` + CLI loader remain the real path.

---

# Tier 3 — Agent / Learning / Knowledge / Affect

## 9. roko-agent — 80.0K LOC / 160 files — **Live** (2nd-largest package)
**Purpose.** All LLM backends, the tool loop, provider pools, MCP client, safety layer, and dispatcher — the execution heart.

**Key public types.** `dispatcher/mod.rs` (the wired dispatch entry); backends: `claude_cli_agent.rs`, `claude_agent.rs`, `codex_agent.rs`, `cursor_agent.rs`/`cursor_cli_agent.rs`, `openai_agent.rs`, `openai_compat_backend.rs`, `ollama.rs`/`ollama_backend`, `gemini.rs`, `perplexity.rs`, `exec.rs`, `mock.rs`; `tool_loop.rs`; `mcp/` (client + `--mcp-config` passthrough); `safety/` (role auth, pre/post checks — the **canonical** safety layer); `pool.rs`/`multi_pool.rs`; `composition.rs`; `metamorphosis.rs`, `hermes.rs`, `openclaw.rs` (exotic/experimental).

**Wired status.** Live — dispatch is called from Runner v2 (`runner/agent_stream.rs`, `agent_events.rs`) and from `roko-cli/src/dispatch*.rs`. Safety layer is integrated into the tool dispatcher.

**Debt.** 160 files, several experimental backends (`hermes`, `openclaw`, `metamorphosis`, `nl_to_format`) of unclear live status. `AgentContract` safety falls back to a permissive default when YAML is missing (CLAUDE.md "Partial").

**Boundary.** `roko-agent → roko-learn` is a **dev-dependency** (runtime edge is `roko-learn → roko-agent`), so no cycle. Real runtime deps: `roko-core`, `roko-fs`, `roko-std`.

**Verdict: Keep, split.** Candidate to split into `roko-agent-core` (trait + dispatcher + tool loop + safety) and `roko-agent-backends` (the 10 providers). Prove/delete experimental backends.

## 10. roko-agent-server — 3.8K LOC / 14 files — **Live**
**Purpose.** Per-agent HTTP sidecar with additive feature modules (`/message` real dispatch, `/stream` WS, `/predictions`, `/research`, `/tasks`).

**Key public types.** `state.rs`, `features/` (route modules), `registration.rs`, `auth.rs`. `roko-cli` and `roko-serve` both depend on it; started via `roko agent serve`.

**Wired status.** Live (as sidecar). Undertested for its complexity.

**Debt / boundary.** Overlaps `roko-serve` (aggregator) and `agent-relay` on "agent visibility." The relationship of these three surfaces is unresolved.

**Verdict: Keep, but clarify vs roko-serve/agent-relay** — merge candidate if the sidecar can become a `roko-serve` feature.

## 11. roko-learn — 58.6K LOC / 66 files — **Live** (3rd-largest)
**Purpose.** The learning loop: episodes, cascade router, experiments, efficiency, bandits, playbooks, cost/latency tracking, oracles, active inference.

**Key public types.** `episode_logger.rs` (`.roko/episodes.jsonl`); `cascade_router.rs` / `model_router.rs` (persists `.roko/learn/cascade-router.json`); `prompt_experiment.rs` / `model_experiment.rs` (A/B, `ExperimentStore`); `efficiency.rs` (`.roko/learn/efficiency.jsonl`); `bandits.rs`, `contextual_bandit.rs`; `playbook.rs` / `playbook_rules.rs`; `hdc_fingerprint.rs` / `hdc_clustering.rs`; `provider_health.rs`, `latency.rs`, `pareto.rs`, `cost_table.rs`, `oracles.rs`, `active_inference.rs`.

**Wired status.** Live — Runner v2 writes episodes/efficiency/cascade state. Playbook store is queried at dispatch time → system prompt.

**Debt.** Huge (66 modules); knowledge-informed routing (neuro → cascade) not yet implemented (CLAUDE.md #13). Multiple overlapping cost/reward stores (`cost_table`, `costs_db`, `costs_log`, `local_reward`).

**Boundary.** Runtime deps include `roko-agent`, `roko-neuro`, `roko-compose`, `roko-daimon`. The `roko-compose` back-edge is **dev-only**. `roko-learn → roko-agent` is real (needed for model-call feedback).

**Verdict: Keep, prune.** Collapse the duplicate cost/reward roots; wire neuro into cascade routing; preserve routing-source fidelity.

## 12. roko-neuro — 16.6K LOC / 10 files — **Live**
**Purpose.** Durable knowledge/memory store: admission, distillation, tier progression (working → short-term → long-term), temporal decay, context retrieval.

**Key public types.** `knowledge_store.rs` `KnowledgeStore` (used directly in `runner/event_loop.rs:51`); `admission.rs`, `distiller.rs`, `tier_progression.rs`, `temporal.rs`, `lifecycle.rs`, `context.rs`, `episode_completion.rs`.

**Wired status.** Live — `roko knowledge` commands + Runner v2 consult `KnowledgeStore`. `roko-primitives` is optional here → HDC retrieval path is feature-gated.

**Debt.** HDC/retrieval wiring undecided (optional primitives). Only 10 files but 16.6K LOC — dense modules.

**Verdict: Keep.** Decide the canonical knowledge-ingestion path and the HDC-retrieval feature.

## 13. roko-dreams — 13.7K LOC / 26 files — **Partial**
**Purpose.** Offline consolidation: dream cycle, hypnagogia, imagination, rehearsal, replay, routing-advice, plus Phase-2 stubs.

**Key public types.** `runner.rs` `DreamRunner` + `DreamLoopConfig`/`DreamAgentConfig` (instantiated in `runner/event_loop.rs:5491–5507`); `cycle.rs`, `hypnagogia.rs`, `imagination.rs`, `replay.rs`, `rehearsal.rs`, `routing_advice.rs`, `threat.rs`, `staging.rs`, `phase2.rs`.

**Wired status.** **Partial → Live-in-v2.** `DreamRunner` *is* constructed inside Runner v2's event loop, so it is reachable on the live path — but there is no cron/delta/BusPulse scheduler; it fires only when the runner path builds it. `routing_advice` is not consumed by the cascade router yet.

**Debt.** `phase2.rs` stubs ship in the default build (misleading). Routing advice produced but unconsumed.

**Verdict: Keep, feature-gate Phase 2.** Either consume dream routing advice in the router or document it as a non-goal.

## 14. roko-daimon — 7.3K LOC / 7 files — **Live**
**Purpose.** Affect engine: PAD state, somatic markers, dispatch modulation, plus Phase-2 goals/mortality/life-review stubs.

**Key public types.** `DaimonState` + `adjusted_thresholds` (used in `runner/event_loop.rs:20, 3028–4271` — `DaimonTaskHook`, `DaimonDispatchModulation`, `render_daimon_prompt_context`); `policy.rs`, `somatic_ta.rs`; `goals.rs`, `mortality.rs`, `life_review.rs` (Phase-2).

**Wired status.** **Live** — Runner v2 loads `DaimonState`, computes a per-task hook, modulates dispatch, and injects a "Daimon state" prompt section (`daimon_policy_for_hook` → `roko_core::DaimonPolicy`).

**Debt.** Duplicate affect paths (`roko-core/src/affect.rs` vs this crate). Phase-2 stubs (`goals`/`mortality`/`life_review`) in default build.

**Verdict: Keep, gate Phase 2.** Reconcile the two affect representations; keep proving live dispatch modulation.

## 15. roko-conductor — 10.1K LOC / 24 files — **Legacy / Unwired on live path**
**Purpose.** Reactive intelligence: 10 anomaly watchers (each a `React` impl), circuit breaker, diagnosis, self-healing, Yerkes-Dodson arousal, federation.

**Key public types.** `conductor.rs` `Conductor` (composite React); `watchers.rs` (10 watchers); `circuit_breaker.rs`; `diagnosis.rs`; `stuck_detection.rs`; `self_healing.rs`; `interventions.rs`; `yerkes_dodson.rs`; `threshold_learner.rs`.

**Wired status.** **Legacy.** `roko_conductor::` is referenced **only** in `roko-cli/src/orchestrate.rs` (the dead executor) and within the conductor crate's own modules/tests (verified 2026-07-08). Runner v2's `event_loop.rs` sets `conductor_load: 0.0` and does **not** call the conductor. So on the live path the reactive layer is **not running**.

**Debt.** Substantial built-but-unwired watcher fleet. Depends on `roko-learn`.

**Boundary.** `roko-conductor → roko-learn` coupling should be event-driven if the crate stays a library.

**Verdict: Merge or rewire.** Port the critical watchers (stuck-detection, circuit breaker) into Runner v2, or mark the crate explicitly legacy-only. Strong **merge/quarantine candidate**.

## 16. roko-orchestrator — 20.8K LOC / 30 files — **Legacy**
**Purpose.** The v1 DAG executor: plan discovery, task DAG, parallel executor, merge queue, worktree management, repair/replan, and a **full duplicate safety/ subsystem**.

**Key public types.** `executor/mod.rs` (v1 executor); `dag.rs`; `merge_queue.rs`; `worktree.rs`; `plan_discovery.rs`; `replan.rs`; `repair.rs`; `service_factory.rs`; `coordination.rs`; **`safety/`** (`loop_guard.rs`, `capability_tokens.rs`, `permit.rs`, `audit_chain.rs`, `sandboxing.rs`, `taint_propagation.rs`) — ~3.4K LOC that **duplicates `roko-agent/src/safety/`**.

**Wired status.** **Legacy.** Runtime deps are broad (`roko-agent`, `roko-compose`, `roko-conductor`, `roko-daimon`, `roko-gate`, `roko-learn`, `roko-neuro`, `roko-runtime`), and it is depended on by `roko-cli`, `roko-serve`, `roko-acp` — but the live `plan run` path is Runner v2, not this executor. Types may be referenced; the *executor* is not the default.

**Debt.** The duplicate `safety/` tree is the single largest type-duplication in the repo. Two DAG/merge-queue implementations (here vs `runner/task_dag.rs` + `runner/merge.rs`).

**Boundary.** Direct learning/runtime coupling; pulls the whole stack.

**Verdict: QUARANTINE.** Strongest quarantine candidate. Keep only as a service library if anything still imports its types; delete the duplicate `safety/` (canonical lives in `roko-agent`) and the dead executor. This is the #1 delete-for-LOC opportunity after `orchestrate.rs`.

## 17. roko-graph — 4.4K LOC / 18 files — **Partial**
**Purpose.** The v2 DAG cell engine: `Cell` trait, `Graph`/`Node`/`Edge` types, TOML loader, topo sort, registry, budget tracker, conditional edges, built-in cells (`AgentCell`, `ComposeCell`, `GraduationCell`).

**Key public types.** `cell.rs` `Cell`; `types.rs`; `engine.rs`; `loader.rs`; `topo.rs`; `registry.rs` `CellRegistry`; `cells/` (`AgentCell`, `ComposeCell`, `GraduationCell`); `budget.rs`, `condition.rs`, `convert.rs`, `hot.rs`.

**Wired status.** **Partial.** `roko-cli` depends on it and there is a `roko graph` command; `orchestrate.rs:8369 run_with_v2_engine` can delegate here. But task/gate cells **dry-run** rather than executing real dispatch, and it is not the default `plan run` engine.

**Debt.** Second DAG engine alongside Runner v2's `task_dag`; cells not wired to real dispatch/resume.

**Verdict: Keep or fold.** Decide the v2 timeline: either wire cells to real dispatch and make Graph the engine, or fold its concepts into Runner v2 and retire the standalone engine. Do not maintain three DAG implementations (orchestrator, runner, graph).

---

# Tier 4 — Chain / Apps / Editor / Intelligence

## 18. roko-chain — 23.4K LOC / 40 files — **Partial**
**Purpose.** On-chain client abstractions: `ChainClient` (reads), `ChainWallet` (signed writes), registries (agent/reputation/validation), ISFR oracle stack, markets (futures/identity-economy/x402/korai token), witness, chain gate, and a mock backend.

**Key public types.** `client.rs`, `wallet.rs`, `mock.rs` (test doubles); `alloy_impl.rs` (optional `alloy-backend` feature — real JSON-RPC); `gate.rs` (chain-verify gate); `tools.rs` (chain tool handlers, surfaced through `roko-std`); registries + `isfr*` + markets + `witness.rs` + `phase2.rs`.

**Wired status.** **Partial.** `roko-cli`/`roko-serve`/`roko-demo` enable `alloy-backend`. Mock/local paths dominate; live-chain authority and deployed contracts are **not** the default (Phase 2+, needs a blockchain backend for witness anchoring — CLAUDE.md #16).

**Debt.** Mock / local / live-chain authority are intermixed. Large speculative market/ISFR surface with thin tests.

**Boundary.** `roko-std → roko-chain` pulls chain into the std tool layer.

**Verdict: Keep, split.** Separate mock/local from live-chain authority behind clear features; wire chain tools into normal workflows or mark experimental.

## 19. roko-serve — 61.4K LOC / 100 files — **Live** (4th-largest)
**Purpose.** The HTTP control plane: ~85+ REST routes, SSE, WebSocket, StateHub projection, auth/JWKS, jobs, deploy, bench, feed-agents, relay, dreams/feedback surfaces, OpenAPI.

**Key public types.** `ServerBuilder` / `run_server` (lib entry); `routes/`; `state.rs` + `state_hub`; `auth`/`jwks`; `job_runner.rs`, `scheduler.rs`; `deploy/`, `bench/`; `feed_agents.rs`, `relay.rs`, `dreams.rs`, `feedback.rs`; `openapi.rs`, `parity.rs`, `truth_map.rs`, `projection_contract.rs`.

**Wired status.** Live — `roko serve` on :6677; `roko-cli` triggers runner via `serve_runtime.rs`.

**Debt.** Broad, partially in-memory state; route count includes stubs; auth matrix + persistence choices need an audit. `parity.rs`/`truth_map.rs`/`projection_contract.rs` hint at ongoing frontend-parity work.

**Boundary.** Overlaps `mirage-rs` (`/api/*`), `roko-agent-server`, and `agent-relay` on agent visibility; shares event/state concepts with core/runtime/cli.

**Verdict: Keep, prune.** Produce a route manifest + auth matrix; document serve vs mirage as separate surfaces; decide persistence.

## 20. roko-acp — 15.9K LOC / 15 files — **Live**
**Purpose.** Agent Client Protocol server (JSON-RPC 2.0 over stdio) so Roko acts as a coding agent inside editors (Zed, JetBrains, Neovim, VS Code).

**Key public types.** `acp_adapter.rs`, `handler.rs`, `session.rs`, `pipeline.rs`, `runner.rs`, `workflow.rs`, `transport.rs`, `builtin_tools.rs`, `event_forward.rs`, `config`/`config_watch.rs`.

**Wired status.** Live — `roko-cli` depends on it; exposed as an ACP stdio server. Pulls a wide runtime stack (`roko-runtime`, `roko-agent`, `roko-gate`, `roko-compose`, `roko-orchestrator`, `roko-learn`, `roko-dreams`, `roko-neuro`, `roko-std`).

**Debt.** Depends on `roko-orchestrator` (Legacy) — inherits that quarantine risk.

**Verdict: Keep.** Permission/capability/session parity + MCP env passthrough; drop the orchestrator edge once its executor is quarantined.

## 21. roko-index — 4.6K LOC / 7 files — **Partial**
**Purpose.** Code intelligence: parser (via `LanguageProvider`), symbol graph with PageRank, SQLite persistence, workspace scan, and **its own HDC fingerprints**.

**Key public types.** `parser.rs`, `symbol.rs`, `graph.rs` (PageRank), `sqlite.rs`, `workspace.rs`, **`hdc.rs`**.

**Wired status.** **Partial.** `roko index build/search` works and powers `roko-mcp-code`, but the index is **not auto-invoked during plan execution**.

**Debt / boundary — SURPRISE.** `roko-index` **re-implements HDC in its own `hdc.rs` and does NOT depend on `roko-primitives`** (deps are only `roko-core` + the three lang crates). So there are **two incompatible 10,240-bit HDC implementations** in the tree (`roko-primitives::hdc` vs `roko-index::hdc`) that cannot interoperate. Fingerprints produced by the indexer can't be compared with episode/engram fingerprints.

**Verdict: Keep, but unify HDC.** Make `roko-index` depend on `roko-primitives::hdc` and delete its private copy; integrate index freshness with serve/TUI.

## 22–24. roko-lang-rust / -typescript / -go — 1.4K / 0.9K / 0.7K LOC — **Live**
**Purpose.** `BuildSystem` + `LanguageProvider` impls per language (import parsing + symbol extraction; build/test/lint/fmt command descriptors).

**Key public types.** `CargoBuildSystem`/`RustLanguageProvider` (+ `tree_sitter_parser`), `GoBuildSystem`/`GoLanguageProvider`, TS equivalents.

**Wired status.** Live — consumed by `roko-index`. `roko-lang-rust` also drives the Rust gate commands.

**Debt.** TS/Go are thin single-file regex/text parsers (938 / 673 LOC); Rust has a tree-sitter path (2 files). Parser fidelity varies by language.

**Verdict: Keep.** Keep parser contracts aligned with `roko-index`; merge into `roko-index` only if language providers are not meant to be public extension points.

## 25. roko-mcp-code — 1.9K LOC / 2 files — **Live**
**Purpose.** MCP server exposing code-intelligence queries backed by `roko-index` (in `default-members`, so it ships).

**Wired status.** Live — one of the three default-build binaries; agents call it via `--mcp-config`.

**Debt / boundary.** Must enforce workspace-root + safety policy consistently.

**Verdict: Keep.**

## 26. roko-mcp-stdio — 0.3K LOC / 1 file — **Live**
**Purpose.** Shared line-delimited JSON-RPC 2.0 stdio transport used by all standalone MCP servers.

**Wired status.** Live — depended on by `roko-mcp-code/-github/-slack/-scripts`.

**Verdict: Keep.** The one genuinely shared MCP primitive; fold the MCP config story around it.

## 27. roko-mcp-github — 3.2K LOC / **1 file** — **App**
**Purpose.** GitHub MCP integration (in `default-members`).

**Wired status.** App/standalone MCP server; ships in default build. Single 3.2K-LOC file — thin but real.

**Debt.** Auth/env undocumented; should route through common secret handling.

**Verdict: Keep.**

## 28. roko-mcp-slack — 1.1K LOC / 1 file — **App**
Slack MCP server. Standalone (not in default-members). Thin. **Keep or merge** into a unified MCP crate; document auth/env.

## 29. roko-mcp-scripts — 0.8K LOC / 1 file — **App**
Script-runner MCP server (high-risk by nature — executes scripts). Standalone. **Keep** but maintain allowlist/timeouts and a safety proof; strong **merge** candidate into a shared MCP crate given its size.

## 30. roko-cli — 179.3K LOC / 249 files — **Live** (the giant)
**Purpose.** The `roko` binary: every subcommand, the TUI, Runner v2 (`runner/`), and Legacy `orchestrate.rs`.

**Key public entry points.** `runner/` (Live engine — `event_loop::run`, `plan_loader`, `task_dag`, `merge`, `gate_dispatch`, `resume`, `snapshot_writer`, `projection`, `tui_bridge`); `commands/plan.rs:654` + `commands/do_cmd.rs:616` (dispatch to runner); `prd.rs`, `research.rs`, `chat.rs`, `daemon.rs`, `serve_runtime.rs`, `worker/`; `orchestrate.rs` (**Legacy**, 22K+ LOC); `dispatch.rs`/`dispatch_v2.rs`; `tui/`.

**Wired status.** Live — the primary entry point. Depends on essentially everything (18 workspace crates including Legacy `orchestrator` + `conductor` and `roko-chain` with `alloy-backend`).

**Debt.** By far the biggest crate. `orchestrate.rs` is dead-by-default yet still compiled and 22K LOC. Runtime concepts (state hub, projections, snapshot) live here that belong in `roko-runtime`. `dispatch.rs` vs `dispatch_v2.rs` duplication.

**Verdict: Keep, but SHRINK aggressively.** Delete/quarantine `orchestrate.rs`; move reusable runtime contracts to `roko-runtime`/`roko-core`; unify dispatch. This is the top structural debt in the repo.

## 31. roko-demo — 5.9K LOC / 21 files — **Partial**
**Purpose.** Manifest-driven demo orchestrator: deploy contracts, seed fixtures, spawn a clade of agents per scenario, plus a demo TUI and WS server.

**Key public types.** `manifest.rs`, `scenarios.rs`, `deploy.rs`, `fixtures.rs`, `tournament.rs`, `autonomous.rs`, `benchmark.rs`, `chain_ctx.rs`, `tui.rs`, `ws_server.rs`, `verify.rs`, `bindings.rs`.

**Wired status.** Partial — a `roko-demo` binary exists; default provider is stub. Pulls `roko-chain` with `alloy-backend`.

**Verdict: Move under `apps/`** (or justify as a crate). Demo app/resources are the supported surface; a `crates/` demo blurs the layer map.

---

# Apps (standalone binaries, not in default-members)

## 32. mirage-rs — 33.3K LOC / 45 files — **App**
**Purpose.** In-process EVM fork simulator: fork state, JSON-RPC surface, precompiles, scenario/replay, persistence, and a `roko_bridge` to core traits.

**Key public modules.** `fork.rs`, `rpc.rs`/`chain_rpc.rs`, `http_api.rs`, `precompiles.rs`, `chain.rs`, `scenario.rs`, `replay.rs`, `cow.rs`, `resources.rs`, `persist.rs`, `roko_bridge.rs`, `integration.rs`. Features: base build = pure EVM; `chain` feature adds HDC/InsightEntry/pheromone; `roko` feature bridges to `Gate`/`Substrate`.

**Wired status.** App — standalone; drives chain scenarios and the `roko-chain-watcher`. Not part of the core CLI build.

**Boundary.** Exposes `/api/*` semantics that differ from `roko-serve`'s — must be documented as a separate surface.

**Verdict: Keep, separate.** Distinct product surface; keep its route semantics apart from `roko-serve`.

## 33. agent-relay — 1.7K LOC / 6 files — **App**
**Purpose.** Topic-based pub/sub bus for agents over WebSocket, with an optional chain-event watcher that polls `eth_blockNumber` and fans out `new_block` envelopes.

**Key public modules.** `bus.rs` `TopicBus`, `protocol.rs`, `state.rs`, `chain_watcher.rs`, `main.rs`.

**Wired status.** App — standalone binary. No workspace crate depends on it.

**Boundary.** Overlaps `roko-serve` (relay/feed-agents) and `roko-agent-server` on agent visibility.

**Verdict: Keep or merge.** Decide whether the serve proxy or a standalone relay is canonical; if serve wins, fold this in.

## 34. roko-chain-watcher — 2.9K LOC / 7 files — **App**
**Purpose.** Long-running agent that polls a `mirage-rs` (or real Ethereum) RPC, analyzes blocks (gas/base-fee/tx activity), and posts grounded insights/pheromones back via JSON-RPC.

**Key public modules.** `watcher.rs` `Watcher`, `block_observer.rs`, `rpc_client.rs`, `reactions.rs`, `known_addresses.rs`, `main.rs`.

**Wired status.** App — standalone; talks to `mirage-rs`. Has real (`block_observer`) + dry-run modes.

**Verdict: Merge candidate.** Could become a `roko-chain` observer or a `roko-serve` feed-agent rather than a standalone app. Add live-chain proof gates + config docs.

---

## Bonus member — `tests/` (integration package "tests")
Workspace member (not a `crates/` library). End-to-end integration tests. **Keep**, but expand: add default-path execution, route, state-migration, and frontend-contract coverage (the runner-v2 path is under-tested at the integration level).

> Note: the old audit listed `roko-benches` and `roko-test-utils`; neither is a current workspace member at HEAD `5852c93c05`. They have been dropped/absorbed.

---

## Consolidated Health Summary (34 packages)

```
Live (default runtime path):        18   roko-core, primitives*, fs, std, gate, compose,
                                         agent, agent-server, learn, neuro, daimon, serve,
                                         acp, lang-rust/ts/go, mcp-code, mcp-stdio, cli
Partial (reachable, subset live):    6   primitives (HDC feature), runtime (workflow dead),
                                         dreams (v2-only, no scheduler), graph (dry-run),
                                         chain (mock-default), index (not auto-invoked), demo
Legacy (only via orchestrate.rs):    2   roko-orchestrator, roko-conductor
Unwired (no live caller):            1   roko-plugin
Standalone MCP / apps:               7   mcp-github/slack/scripts, mirage-rs, agent-relay,
                                         chain-watcher, (roko-demo → move to apps)
```

**Top structural findings (this pass):**
1. `orchestrate.rs` (22K LOC) + `roko-orchestrator` (20K LOC) + `roko-conductor` (10K LOC) form a
   ~52K-LOC **Legacy island** reachable only through the dead executor. Largest quarantine/delete target.
2. `roko-orchestrator/src/safety/` (~3.4K LOC) **duplicates** `roko-agent/src/safety/` — delete the copy.
3. **Two incompatible HDC implementations**: `roko-primitives::hdc` (used by core/learn) vs
   `roko-index::hdc` (private, no primitives dep). Fingerprints can't interoperate.
4. **Three DAG engines**: `roko-orchestrator::executor` (Legacy), `roko-cli/runner::task_dag` (Live),
   `roko-graph` (Partial/dry-run). Converge on one.
5. `roko-runtime → roko-gate/compose/learn` is a real layer inversion.
6. `roko-plugin` is a 2-file facade; `roko-mcp-*` are thin single-file crates; `roko-demo` should
   live under `apps/`.

See `54-PER-CRATE-MIGRATION-CHECKLIST.md` for the per-crate zero-debt checklist.
