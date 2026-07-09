# 06 — Wiring Status: The Built-But-Unwired Census

**Exhaustive inventory of code that compiles (and often has tests) but is never reached
from the default runtime.** Second, deeper pass — every row re-verified by caller search.

## Verification

- **HEAD**: `5852c93c05` on `main`
- **Date**: 2026-07-08
- **Method**: For each symbol, searched all of `crates/` for non-test callers, excluding
  `/tests/` dirs and `#[cfg(test)]` blocks. "LEGACY-ONLY" = the sole prod caller is
  `crates/roko-cli/src/orchestrate.rs`, which sits behind the **default-off**
  `legacy-orchestrate` feature (`crates/roko-cli/Cargo.toml:16`, `lib.rs:94-95`).
- **Status tags**: ✅ WIRED · ⚠️ PARTIAL/DEAD-BRANCH · 🕯️ LEGACY-ONLY · ❌ NOT-WIRED · 👻 ORPHAN (on disk, not in any `mod` tree — not even compiled)

> **This pass corrected the previous (shallow) version of this doc.** Items previously
> listed as unwired that are in fact WIRED now: `roko-graph` (now the **default** `plan run`
> engine), `WorkflowEngine`, `RunLedger`, `EffectDriver`, `event_bus`, runtime `state_hub`,
> `demurrage_consumer`, `event_subscriber`, cold-substrate archival, and per-task Daimon.
> See "Corrections" at the bottom.

---

## The census

### roko-runtime — the heartbeat/economics consumers (self-labelled `//! STATUS: NOT WIRED`)

| Module / type | file:line | What it does | Why it was built | Prod caller | Tag | Verdict |
|---|---|---|---|---|---|---|
| `DeltaConsumer` | `delta_consumer.rs:168` (mod `lib.rs:44`) | Consumes "delta" pulses; hosts dream-cycle stubs (`:299-333`) | V2 pulse-driven metabolism | NONE | ❌ | Wire behind PulseBus or delete |
| `ThetaConsumer` | `theta_consumer.rs:140` (mod `lib.rs:66`) | Theta-rhythm consumer | V2 rhythm scheduler | NONE | ❌ | Delete or gate `phase2` |
| `CognitiveMetabolism` / `EnergyPool` | `energy.rs:82,163` (mod `lib.rs:47`) | Energy budget accounting | V2 metabolic model | NONE | ❌ | Delete or gate |
| `run_attention_auction` / `ContextGovernor` | `heartbeat_attention.rs:387,532` (mod `lib.rs:51`) | Attention auction over context budget | V2 attention economy | NONE | ❌ | Overlaps VCG; pick one |
| `HeartbeatProbeRegistry` | `heartbeat_probes.rs:601` (mod `lib.rs:52`) | Liveness probes | Observability | Re-export only (`roko-core/src/obs/mod.rs:25`); no constructor | ❌ | Wire into serve health or delete |
| `TaskScheduler` | `task_scheduler.rs` (mod `lib.rs:65`) | Priority task scheduler. Doc says "Used by WorkflowEngine" — **false** | Alt executor | NONE (stale doc) | ❌ | Delete; WorkflowEngine ignores it |
| `PulseBus` | `pulse_bus.rs:29` (mod `lib.rs:60`) | Pulse pub/sub kernel over `EventBus<Pulse>` | V2 "Pulse drives Bus" kernel | NONE (`PulseBus::new` never called) | ❌ | Core of V2 that never shipped |

**Corrected to WIRED** (were suspected dead): `RunLedger` (`run_ledger.rs`, header comment "not wired yet"
is stale — `workflow_engine.rs:159` + `runner/event_loop.rs:579` construct it); `WorkflowEngine`
(`workflow_engine.rs:114` → `run.rs:756/3391`, `roko-acp/runner.rs:489`, `roko-serve/routes/shared_runs.rs:487`);
`EffectDriver`/`EffectServices` (`service_factory.rs:93`, `run.rs:3380`); `EventBus` (dozens of prod
publishers in serve/cli/acp/agent); runtime `StateHub` (`roko-serve/state.rs:842` + cli re-export);
`DemurrageConsumer` (`roko-serve/lib.rs:1992` `start_demurrage_timer`).

### roko-core — orphans + the unused universal loop

| Module / type | file:line | What | Prod caller | Tag | Verdict |
|---|---|---|---|---|---|
| `roko-core/src/state_hub.rs` | `state_hub.rs:71` | Duplicate `StateHub`/`SharedStateHub`/`shared_state_hub` of the runtime one | **No `mod state_hub` in `roko-core/lib.rs`** — not compiled. Only broken doc-links (`dashboard_snapshot.rs:5,755`) | 👻 | **Delete** — dead duplicate of `roko-runtime::state_hub` |
| `roko-core/src/pulse_bus.rs` | `pulse_bus.rs:29` | `PulseBus` wrapping `EventBus<Pulse>` | **No `mod pulse_bus` in `roko-core/lib.rs`**. Broken `crate::PulseBus` doc-links at `bus_backends.rs:3`, `traits.rs:384` | 👻 | **Delete** — orphan, breaks doc links |
| `loop_tick()` | mod `lib.rs:114` | The V1 universal loop (query→score→route→compose→act→verify→write→react) | No prod caller (only `roko-std/tests/universal_loop.rs`) | ❌ | The spec's central abstraction is unused; runtime reimplements it ad-hoc |

### roko-conductor — 10 watchers + breaker + diagnosis (all LEGACY-ONLY)

The entire crate's only prod consumer is legacy `orchestrate.rs` (imports at `:46-51,2479,2698,6934,8324`).
The default runner (`crates/roko-cli/src/runner/`) has **zero** `roko_conductor` references.

| Symbol | file:line | Prod caller | Tag |
|---|---|---|---|
| `Conductor::new` | `conductor.rs` (called `orchestrate.rs:4637,4750,4977`) | legacy only | 🕯️ |
| 10 watchers (`TimeOverrun`, `CostOverrun`, `StuckPattern`, `SpecDrift`, `TestFailureBudget`, `IterationLoop`, `CompileFailRepeat`, `GhostTurn`, `ContextWindowPressure`, `ReviewLoop`) | `watchers/*.rs`, registered `conductor.rs:95-107` | legacy only | 🕯️ |
| `CircuitBreakerState`, `DiagnosisEngine`, `HealthMonitor`, `StuckDetector` | used `orchestrate.rs:790-811,6275,6395,10933` | legacy only | 🕯️ |
| `Conductor::from_config` + `configured_watchers()` | `conductor.rs:204,110` | **ZERO callers** → the entire `[conductor.watchers.*]` TOML block is dead config | ❌ |
| runner-v2 `conductor_load` | `runner/event_loop.rs:4258` | **hardcoded `0.0`** (legacy `orchestrate.rs:3016,15420` computes it live) | ⚠️ |
| `roko-orchestrator` → `roko-conductor` dep | `roko-orchestrator/Cargo.toml:17` | **dead dependency** — no `roko_conductor` symbol used in the crate | ❌ |

**Verdict**: the conductor's supervision layer (watchers, breaker, diagnosis) is invisible to the
default runtime. Either port a subset into `runner/event_loop.rs` or accept the runner is unsupervised.

### roko-learn — the "floating" learning islands (self-labelled `//! STATUS: NOT WIRED`)

| Module / type | file:line | Prod caller | Tag |
|---|---|---|---|
| `VerdictAwareScorer` (`ScoreFn`) | `verdict_scorer.rs:46` | NONE | ❌ |
| `VerdictHistory`/`VerdictRecord` | `verdict_scorer.rs:205,183` | **written** by wired `event_subscriber.rs:88,261`, but every read method (`reward_penalty`, `model_failure_rate`, `compile_failure_streak`, `recent_verdicts`) has **zero callers** | ❌ (data written to a void) |
| `enrich_error_digest` | `error_enrichment.rs:24` | NONE | ❌ |
| `oracles` (chain/coding/research/selector/witness) | `oracles/mod.rs:9-13` | NONE — fully isolated island | ❌ |
| `judge_quality` | `quality_judge.rs:19` | NONE | ❌ |
| `BayesianConfidenceUpdater` | `bayesian_confidence.rs:25` | NONE | ❌ |
| `BeliefState`/`select_tier` (active inference) | `active_inference.rs:19,85` | only `cascade_router.rs:436 select_tier_with_active_inference`, **which itself has zero callers** | ❌ (dead-calls-dead behind live `CascadeRouter`) |

**Corrected to WIRED**: `run_learning_subscriber` (`event_subscriber.rs:72`) is spawned at
`runner/event_loop.rs:766` (`tokio::spawn`, non-legacy) — header claim verified.

### roko-compose — VCG auction (dead branch) + duplicate PromptAssembler

| Symbol | file:line | Reality | Tag |
|---|---|---|---|
| `update_bidders` / `update_bidders_with_cost` | `prompt.rs:692,701` | **zero callers** | ❌ |
| `register_bidder` / `with_learning_bidders` | `prompt.rs:674,680` | zero prod callers → `learning_bidders` is **always empty** | ❌ |
| `vcg_allocate` / `VcgAllocation` | `auction.rs:380,357` | reachable only via `select_vcg_candidates` (`prompt.rs:1189`), gated on `strategy == Vcg` | ⚠️ dead branch |
| Strategy selection | `prompt.rs:864`, `strategy.rs:48` | `Auto` → picks `Vcg` only if min bidder-observations ≥ warmup(10); observations read empty `learning_bidders` → `min=0` → **`DensityGreedy` every time** | — |
| `learned_multiplier` | `prompt.rs:835` | always resolves to `1.0` (same empty-bidders cause) | ⚠️ |
| `AttentionBidder` (enum, section tags) | `prompt.rs:83` | ubiquitous prod use | ✅ |
| `templates::assembly::PromptAssembler` (struct) | `templates/assembly.rs:42` | instantiated only by its own unit tests | ❌ dead path |

**The winning prompt path** is `RoleSystemPromptSpec` → `SystemPromptBuilder`
(`prompting.rs:45,71` ← `dispatch_helpers.rs:133`, `run.rs:1517`). `PromptAssemblyService`
(`prompt_assembly_service.rs:356`) is a **second wired path used only in the v2 WorkflowEngine**
(`service_factory.rs:210` → `do_cmd.rs:143`, serve, acp) and delegates to `SystemPromptBuilder` too.
So there are **three** things named PromptAssembler; only the `templates::assembly` struct is dead.

### roko-orchestrator/safety — ~3.4K LOC dead duplicate

Distinct from the WIRED `roko-agent/src/safety/` (10.5K LOC, real dispatch enforcement). None of these
are referenced by any external crate.

| File | LOC | Prod caller | Tag |
|---|---|---|---|
| `capability_tokens.rs` | 860 | none | ❌ |
| `sandboxing.rs` | 651 | none | ❌ |
| `audit_chain.rs` | 565 | only `executor/mod.rs:427 with_audit_chain`, whose sole callers are `tests/lifecycle.rs` → field defaults `None` in prod | ❌ |
| `taint_propagation.rs` | 483 | none (doc-mention only `roko-core/provenance.rs:128`) | ❌ |
| `permit.rs` | 452 | none | ❌ |
| `loop_guard.rs` | 364 | none | ❌ |

**Verdict**: delete the crate's `safety/` module, or wire `AuditChain` into the ParallelExecutor
prod constructors (`event_loop.rs:3564,3663,3727` never call `.with_audit_chain`).

### Misc husks & gates

| Symbol | file:line | Reality | Tag | Verdict |
|---|---|---|---|---|
| `SubstrateMigrator` | `roko-fs/src/cold_substrate.rs:305` (re-export `lib.rs:49`) | constructed only in own tests; prod hot→cold path uses `archive_batch` directly | ❌ | Delete husk |
| ACP `request_permission` | `roko-acp/src/bridge_events.rs:768` | only `#[cfg(test)]` callers; agent side (`roko-agent/src/openclaw/acp_agent.rs:271`) merely **logs "auto-approved"**, never blocks | ❌ | The permission gate does nothing at runtime |
| `AgentPool` | `roko-agent/src/pool.rs:162` | `AgentPool::new` callers all `#[cfg(test)]` | ❌ | Test-only |
| `MultiAgentPool` | `roko-agent/src/multi_pool.rs:64` | prod callers `orchestrate.rs:4708,4943,5171` only | 🕯️ | Legacy-only; Graph engine + runner-v2 spawn fresh per task |
| Dream coordination-pattern trigger (`maybe_coordination_dream`, `DreamTrigger::CoordinationPattern`) | `orchestrate.rs:8324,8351` | legacy path only | 🕯️ | Consolidation IS wired elsewhere (see corrections) |
| delta-consumer dream hooks | `delta_consumer.rs:299-333` | TODO stubs "connected when dream runner is wired" | ❌ | Stub |

---

## Corrections to prior beliefs (now verified WIRED)

| Claim (old) | Reality (HEAD 5852c93c05) |
|---|---|
| roko-graph totally unwired | ✅ **Default `plan run` engine** — `commands/plan.rs:257,1567`, `GraphEngine::new` `:1645`; CLI `default_value="graph"` (`main.rs:1361`). Runner-v2 is the opt-in `--engine runner-v2` path |
| WorkflowEngine never instantiated | ✅ Built in `run.rs:756/3391`, acp, serve |
| Cold archival never triggered | ✅ Hourly serve timer `start_cold_archival_timer` (`roko-serve/lib.rs:2096`, invoked `:344,:800`); `[cold_storage] enabled=true` default (`config/schema.rs:1593`); `roko knowledge archive` |
| Daimon only in legacy orchestrate | ✅ Per-task in runner-v2: `event_loop.rs:3059,3105,4247,4344,4388` (hook, modulation, prompt ctx). **CLAUDE.md is stale** |
| Dream triggers not ported | ✅ Consolidation auto-fires: runner post-completion `event_loop.rs:5473` (`learning.dream_on_completion` default true) + serve daemon `daemon.rs:368`. Only the *coordination-pattern* trigger stays legacy |
| AgentContract fails open (permissive) | ✅ **Fails CLOSED** — missing YAML → `AgentContract::restricted(role)` deny-all (`safety/mod.rs:929-940`, `contract.rs:256-264`) |
| `error_recovery.rs` never called | Symbol **does not exist** anywhere in `crates/` |

---

## Prioritized roadmap

1. **Delete now (safe, zero prod callers, orphan/dupe)**: `roko-core/src/state_hub.rs`,
   `roko-core/src/pulse_bus.rs` (👻 fixes broken doc-links), `roko-orchestrator/src/safety/*`
   (3.4K LOC), `SubstrateMigrator`, `AgentPool`, `roko-orchestrator`→`roko-conductor` dead dep.
2. **Decide V2 fate**: `PulseBus`/`loop_tick`/`DeltaConsumer`/`ThetaConsumer`/`energy`/
   `heartbeat_attention` are the unshipped V2 kernel. Either gate them `phase2` or delete.
3. **Wire or cut supervision**: port a watcher subset + live `conductor_load` into
   `runner/event_loop.rs`, or delete the conductor's legacy-only breaker/diagnosis.
4. **Wire or cut the learning islands**: `verdict_scorer` reads, `quality_judge`,
   `bayesian_confidence`, `active_inference`, `oracles`, `error_enrichment` — connect to
   `event_subscriber`/`CascadeRouter` or delete. Note `VerdictHistory` is *written but never read*.
5. **Fix VCG**: either populate `learning_bidders` (call `update_bidders`) so the auction can
   ever win, or delete `vcg_allocate`/`auction.rs` and keep greedy.
6. **Fix the permission gate**: make ACP `request_permission` actually block, or delete it.

## Checklist

- [ ] Delete 2 roko-core orphan files (fixes broken intra-doc links)
- [ ] Delete/gate roko-orchestrator/safety (3.4K LOC)
- [ ] Delete SubstrateMigrator + AgentPool husks
- [ ] Remove dead roko-orchestrator→roko-conductor dependency
- [ ] Gate V2 kernel (`PulseBus`, `loop_tick`, delta/theta/energy/attention) under `phase2`
- [ ] Wire `conductor_load` in runner-v2 (currently `0.0`) or drop conductor
- [ ] Connect or delete 6 roko-learn floating modules
- [ ] Resolve VCG dead-branch (populate bidders or delete)
- [ ] Make ACP permission gate enforce or delete it
- [ ] Update CLAUDE.md: Daimon/dream/cold-archival wiring claims are stale

See also **104-DEAD-CODE-AND-FACADE-CENSUS.md** for the feature-façade and `#[allow(dead_code)]` catalog.
