# Crate map (productionizing-relevant subset)

This is the subset of the crate graph touched by H/F/D batches. Read it before guessing where a symbol lives.

## roko-core (foundation)

`crates/roko-core/src/`
- `config/schema.rs` — `RokoConfig`, `[providers.*]`, `[models.*]`, `[agent]`, `[serve]`, `[gc]` tables. **`is_provider_available`, `provider_available_for_model_key`, `available_provider_ids`, `available_model_keys_for_cascade`, `available_model_slugs_for_cascade`** all already live here (lines 440–514).
- `config/agent.rs` — `AgentConfig` (`default_model`, `effort`, etc.). Add `max_cost_per_{plan,task,session}_usd` here for D01.
- `config/provenance.rs` — config-change audit log; needed by D07.
- `agent.rs` — `resolve_model(config, model_key) → ResolvedModel`, `effective_context_window_tokens`. Already used in `orchestrate.rs:181`, `15469`.
- `error/mod.rs`, `error/rpc.rs` — `RokoError::BudgetExceeded`, `ErrorKind`. Reuse, don't redefine.
- `dispatch_plan.rs` — `BudgetExceeded` variant for plan-level events.

## roko-fs (filesystem primitives)

`crates/roko-fs/src/`
- `atomic.rs` — atomic file write helpers (`write_atomic`, `rename_atomic`).
- `gc.rs` — `GcEngine`, `GcPolicy`, `should_auto_gc()` (line 186). **Wire this into roko-serve for P11.**
- `lib.rs` — module declarations. Add `pub mod flock;` for P05.
- `flock.rs` — **does not exist**, must be created by P05 (libc-backed).

## roko-learn (the learning subsystem; the largest crate touched)

`crates/roko-learn/src/`
- `cascade_router.rs` — 3-stage UCB1 router. Has `route()`, `route_with_health()`, `filter_unhealthy()`, `save()`, `load_or_default()`. Already wired into `roko-serve` and `orchestrate.rs`.
- `costs_db.rs` — `CostsDb`, `CostRecord`, `query_by_plan()`, `total_cost()`. Cost summary aggregation already exists; D03 extends it; D06 adds `predict_task_cost`.
- `costs_log.rs` — append-only `costs.jsonl`. Used by `commands/util.rs`.
- `cost_table.rs` — per-model pricing tiers; `estimate_cost(prompt_tokens, completion_tokens, model_slug)`.
- `episode_logger.rs` — `EpisodeLogger::append()` (line 964). **Needs flock for P05.**
- `feedback_service.rs` — `FeedbackService`, `flush()`. **Needs flock for P05.**
- `provider_health.rs` — `ProviderHealthRegistry`, `ProviderHealthTracker`. Already used by `cascade_router::route_with_health`.
- `routing_log.rs` — append-only routing-decision log. **F03 adds sheaf inconsistency score here.**
- `hdc_fingerprint.rs` — wraps `roko_primitives::hdc` for episode/prompt fingerprinting. **D02 (semantic cache) and F05 (novelty search) reuse this.**
- `bandits.rs` — UCB1, Thompson sampling, contextual bandit primitives. **F05 uses UCB1 for novelty/exploitation balance.**
- `curriculum.rs` — `CurriculumScheduler`, `CurriculumMode { EasyFirst, HardFirst, Interleaved, Adaptive }`. **F01 adds `Adas`; F05 adjusts Adaptive.**
- `lib.rs` — module declarations. Add `pub mod adas;` (F01), `pub mod research_pipeline;` (F02), `pub mod novelty_search;` (F05), `pub mod bench_history;` (D04), `pub mod compliance_export;` (D07), `pub mod competitive_bench;` (D09).
- **NOTE:** `adas.rs` and `research_pipeline.rs` are referenced by the source plan as if they exist. Verify with `ls crates/roko-learn/src/{adas,research_pipeline}.rs`. Where the file is missing, F01/F02 must create it.

## roko-agent (LLM dispatch)

`crates/roko-agent/src/`
- `cache.rs` — `ResponseCache` (in-memory, blake3-keyed). **D02 adds `SemanticCache` alongside it; D05 adds atomic stats.**
- `model_call_service.rs` — `ModelCallService` orchestrates one model call (cache → predict → dispatch → record). Already has `cost_predict()`, `BudgetExceeded` returns, `with_cascade_router`, `with_config`. **D02 and D06 wire here; D01 extends budget enforcement.**
- `dispatch_resolver.rs` — `fallback_candidates()` (line 273). Already filters by `provider_available_for_model_key` (P03 done).
- `multi_pool.rs` — `MultiAgentPool`. Already used in `orchestrate.rs` for warm reuse. **F06 fans it out per-task.**
- `composition.rs` — `MergeStrategy { Concatenate, Aggregate, Vote, BestOfN }`. **F06 reuses these.**

## roko-cli

`crates/roko-cli/src/`
- `orchestrate.rs` — the legacy mega-file. Most P-batches touch it.
  - Pre-dispatch validation block: `15169–15188`
  - Hardcoded `claude-sonnet-4-6` fallbacks: `10354, 12941, 13745, 14232, 20377, 21473, 21509` (test fixtures at `21882, 22556, 22588, 22631` are exempt)
  - Mutex `expect()` to fix in P10: `1437, 1441, 17498`
  - 15 `eprintln!` to convert in P12: `6098, 6113, 6613, 6619, 6703, 6708, 7498, 7504, 7586, 7803, 7804, 11269, 11273, 13376, 13378`
- `dispatch/warm_pool.rs` — `WarmPool::insert/take/evict_expired/stats`. 4 `expect("poisoned")` at lines 109, 123, 140, 155.
- `gate_runner.rs` — `RecordingGate::verify` at line 187 has `expect("recorded gate sink poisoned")`.
- `commands/learn.rs` — host for `roko learn ...` subcommands. **F02 (research trial), D03 (costs), D07 (export), D09 (competitive baselines) attach here.**
- `commands/util.rs:514` — `costs_log` accessor reused by D03/D07.
- `main.rs` — tracing subscriber init lives here. **D08 adds the optional OTEL layer.**
- `telemetry.rs` — **may not exist**; D08 may need to create it.

## roko-serve

`crates/roko-serve/src/`
- `lib.rs` — `build_app_state` (line 780), `validate_bind_safety` (641–660), `log_provider_credential_status` (762). **P08 logs JWKS swallow; P11 wires `should_auto_gc` task here.**
- `routes/mod.rs:170–186` — top-level router. `RequestBodyLimitLayer` already wrapped here. **P07 adds `TimeoutLayer`.**
- `routes/status/health.rs` — `/api/health` handler. **P07 returns 503 when degraded; D05 surfaces cache stats.**
- `routes/mod.rs:193` — `top_level_health` (`/health`) — leave at 200.
- `jwks.rs:145` — silent `let _ = self.refresh_jwks().await;` (P08 fix).
- `embedded.rs:39, 69–87` — path canonicalization (P06 done).
- `terminal.rs` — PTY routes. P08 cleans 5+ silent terminal swallows.

## roko-chain

`crates/roko-chain/src/`
- `marketplace.rs` — agent-task assignment. **F07 hooks `CollusionDetector` here.**
- `collusion.rs` — `CollusionDetector`, `record_assignment`, `detect()`. Already implemented.
- `reputation_registry.rs` — has `CollisionFeedbackDilution` (note: spelling matches code). Reuse penalty values.

## roko-daimon

`crates/roko-daimon/src/`
- `phase2_stubs.rs` — `FatigueDetector`, `ErrorPatternTracker`. Already wired into `DaimonState` (F04 done).

## roko-primitives

`crates/roko-primitives/src/`
- `hdc.rs` — HDC vector type (HypervectorN). `bind`, `bundle`, `permute`, `cosine_similarity`, `hamming_similarity`. **D02, F05 reuse.**
- `tropical.rs` — `TropicalF64`, polynomial, attention. **F03 wires.**
- `sheaf.rs` — `CellularSheaf`, coboundary, Laplacian, inconsistency score. **F03 wires.**
