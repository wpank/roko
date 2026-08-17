# Productionizing Issue Tracker

Open issues for the productionizing effort. Each issue has a batch ID that maps
1:1 to a runner prompt (`prompts/{ID}.prompt.md`) and a `batches.toml` entry.
Mark items complete only when the corresponding batch has merged AND the
verification block in the prompt passes.

**Verification truth source**: code in tree, NOT this file. Re-grep before
ticking a box.

> Source plans:
> - `tmp/productionizing/10-IMPLEMENTATION-PLAN.md` (P-prefix)
> - `tmp/productionizing/11-FRONTIER-CAPABILITIES-PLAN.md` (F-prefix)
> - `tmp/productionizing/12-PRODUCTION-DEPLOYMENT-PLAN.md` (D-prefix)

---

## Already done in tree (skipped by this runner)

These were marked complete when the audit ran on 2026-05-01. Do **not** re-add
to the runner. If a regression is found, file a new batch.

- [x] **P01** — `RokoConfig::is_provider_available`, `provider_available_for_model_key`, `available_provider_ids` in `crates/roko-core/src/config/schema.rs:440–482`
- [x] **P02** — `available_model_keys_for_cascade`, `available_model_slugs_for_cascade` (`schema.rs:484–514`)
- [x] **P03** — `CascadeRouter::route_with_health` + `filter_unhealthy` (`crates/roko-learn/src/cascade_router.rs:648–688`); `dispatch_resolver::fallback_candidates` filters by `provider_available_for_model_key` (`crates/roko-agent/src/dispatch_resolver.rs:273–286`)
- [x] **P06** — `validate_bind_safety` errors on public bind without auth (`crates/roko-serve/src/lib.rs:641–660`); path canonicalization in `crates/roko-serve/src/embedded.rs:39, 69–87`
- [x] **P14** — `railway.toml` exists with `healthcheckPath`, `healthcheckTimeout`, `restartPolicyType`, `restartPolicyMaxRetries`. (Note: `/health` is mounted at the top level and always returns 200 — see also P07 for the more discriminating `/api/health`.)
- [x] **F04** — `FatigueDetector` is a field on `DaimonState` (`crates/roko-daimon/src/lib.rs:1999, 2030`); `behavioral_state` is threaded through `crates/roko-cli/src/orchestrate.rs` (lines 2868, 2873, 3626, 9262, 15588). Contagion tracking remains stubbed but the wire is live.

---

## Group H — Hardening (production blockers)

**Wave 1** — independent. Run together.

- [ ] **P04** — Remove 11 hardcoded `"claude-sonnet-4-6"` fallbacks in `orchestrate.rs` (lines 10354, 12941, 13745, 14232 array, 20377, 21473, 21509). Replace with `config.agent.default_model.clone()`. Test fixtures (lines 21882, 22556, 22588, 22631) stay as-is. → `prompts/P04.prompt.md`
- [ ] **P05** — Cross-process `flock` for `.roko/episodes.jsonl`, `efficiency.jsonl`, `cascade-router.json`, `executor.json`. New `crates/roko-fs/src/flock.rs` + 4 call-sites. → `prompts/P05.prompt.md`
- [ ] **P07** — Add `tower_http::timeout::TimeoutLayer(30s)` to router; make `/api/health` return `503 Service Unavailable` when status is `down`. (`/health` stays 200 always — load balancers can pick.) → `prompts/P07.prompt.md`
- [ ] **P08** — `crates/roko-serve/src/jwks.rs:145` still has `let _ = self.refresh_jwks().await;` (security-critical). Plus 4 other documented swallows in `lib.rs` and `terminal.rs`. → `prompts/P08.prompt.md`
- [ ] **P10** — `crates/roko-cli/src/dispatch/warm_pool.rs:109,123,140,155` still call `.expect("poisoned")`. Same with `orchestrate.rs:1437,1441,17498` and `gate_runner.rs:187`. Recover via `.unwrap_or_else(|p| p.into_inner())` + `tracing::warn!`. → `prompts/P10.prompt.md`
- [ ] **P11** — `roko_fs::gc::GcEngine::should_auto_gc()` is implemented (`crates/roko-fs/src/gc.rs:186`) but no caller. Wire a 60-second tokio interval task in `roko-serve` that triggers compaction. → `prompts/P11.prompt.md`
- [ ] **P12** — 15 `eprintln!` remaining in `crates/roko-cli/src/orchestrate.rs` (lines 6098, 6113, 6613, 6619, 6703, 6708, 7498, 7504, 7586, 7803, 7804, 11269, 11273, 13376, 13378). Convert each to `tracing::{info,warn,error}!` with structured fields. → `prompts/P12.prompt.md`
- [ ] **P13** — Create `Dockerfile.optimized` using cargo-chef + sccache (Tier 2 build, 15–30s incremental). Spec in `tmp/productionizing/08-FAST-BUILD-DEPLOY.md`. → `prompts/P13.prompt.md`
- [ ] **P15** — Create `roko.production.toml` containing only providers with API keys (anthropic, openai, perplexity, gemini, ollama). Strip cerebras, zhipu, openrouter, moonshot, zai. Set `serve.auth.enabled = true`. → `prompts/P15.prompt.md`
- [ ] **P16** — Create `Dockerfile.runtime` (Tier 1 build): copies pre-built binary from `target/x86_64-unknown-linux-gnu/release/roko`, no Rust toolchain in image. → `prompts/P16.prompt.md`
- [ ] **P18** — Add `deploy-railway` job to `.github/workflows/docker-publish.yml` that runs `railway up --image ghcr.io/nunchi-trade/roko:latest` on push-to-main. → `prompts/P18.prompt.md`

**Wave 2** — depends on a Wave-1 task.

- [ ] **P09** *(deps: P04)* — Pre-dispatch token estimate (`prompt.len()/4`) vs `effective_context_window_tokens`. If >85% of window, switch to next-larger model in `available_models()`. Log warning. → `prompts/P09.prompt.md`
- [ ] **P17** *(deps: P16)* — Add `deploy.sh` at repo root. Builds frontend, runs `cargo zigbuild --release --target x86_64-unknown-linux-gnu`, `docker buildx`, pushes, deploys to railway/fly/local. → `prompts/P17.prompt.md`

---

## Group F — Frontier wiring

**Wave 1** — independent. Run together.

- [ ] **F01** — Create `crates/roko-learn/src/adas.rs` (Autocatalytic Design optimizer) with `AdasOptimizer::new`, `step`, `save`, `load`. Add `CurriculumMode::Adas` variant in `curriculum.rs`. Persist to `.roko/learn/adas.json`. **Note:** the source doc claims `adas.rs` already exists — verify and create only if missing. → `prompts/F01.prompt.md`
- [ ] **F02** — Create `crates/roko-learn/src/research_pipeline.rs` (Paper → Claim → Trial → Ledger). Add `roko learn research trial` subcommand. Persist ledger to `.roko/learn/research-ledger.jsonl`. → `prompts/F02.prompt.md`
- [ ] **F03** — Wire `roko_primitives::tropical` into `cascade_router::route_*` (max-plus attention) and `roko_primitives::sheaf` into `routing_log` (inconsistency score across signals). Best-effort, fall through on error. → `prompts/F03.prompt.md`
- [ ] **F06** — Add `parallel_subtasks: u8` to `TaskDef` in `roko-core`. When `>1`, fan out via `MultiAgentPool` with `MergeStrategy` from `composition.rs` (BestOfN for code, Concatenate for research, Vote for review). Cap at `agent.max_parallel`. → `prompts/F06.prompt.md`
- [ ] **F07** — Wire `CollusionDetector::record_assignment` + `detect()` into `marketplace.rs` post-assignment. On detected ring, apply `CollisionFeedbackDilution` from `reputation_registry.rs`. Async + advisory only. → `prompts/F07.prompt.md`

**Wave 2** — depends on a Wave-1 task.

- [ ] **F05** *(deps: F01)* — Create `crates/roko-learn/src/novelty_search.rs` with `NoveltyArchive` (HDC fingerprint store). `is_novel(fp) → 1.0 - max_similarity`. Wire into Adaptive curriculum mode (UCB1-balanced). Persist to `.roko/learn/novelty-archive.bin`. Opt-in via `learning.novelty_search = true`. → `prompts/F05.prompt.md`

---

## Group D — Production economics

**Wave 1** — independent. Run together.

- [ ] **D01** — Add `max_cost_per_plan_usd`, `max_cost_per_task_usd`, `max_cost_per_session_usd` to `[agent]` config (defaults: 10/2/50). Enforce in `orchestrate.rs` pre-dispatch using `costs_db.query_by_plan()`. On exceed: log + skip task with reason `BudgetExceeded` (already-defined error). → `prompts/D01.prompt.md`
- [ ] **D02** — Add `SemanticCache` struct in `cache.rs` alongside `ResponseCache`. Hamming similarity over HDC fingerprints (threshold 0.92). Memory-only, max 10k entries, 1h TTL, skip prompts <100 tokens, skip tool-calling. Wire into `model_call_service.rs` after exact-match miss. → `prompts/D02.prompt.md`
- [ ] **D03** — Add `CostsDb::plan_cost_summary(plan_id) → CostSummary` (USD, tokens, per-task, per-SLOC, per-passing-gate). Add `roko learn costs --plan <id>` subcommand. Log summary on plan completion in `orchestrate.rs`. → `prompts/D03.prompt.md`
- [ ] **D04** — Create `crates/roko-learn/src/bench_history.rs`. Append-only JSONL at `.roko/learn/bench-history.jsonl`. After each run, compare to trailing-5 average. Flag regression on >10% pass-rate drop / >20% cost / >30% latency. → `prompts/D04.prompt.md`
- [ ] **D05** — Add `CacheStats { hits, misses, evictions, total_entries, estimated_savings_cents }` (all `AtomicU64`) to `ResponseCache`. Expose at `/api/health`. → `prompts/D05.prompt.md`
- [ ] **D07** — Create `crates/roko-learn/src/compliance_export.rs` with `export_audit_trail(date_range)`. Read existing JSONL files (no DB). Export JSON or CSV. Add `roko learn export --from --to --format` subcommand. Strip raw prompts/responses. → `prompts/D07.prompt.md`
- [ ] **D08** — Add feature-gated OTEL: `[features] otel = ["tracing-opentelemetry", "opentelemetry-otlp", "opentelemetry-sdk"]`. Activate only when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. Default off. → `prompts/D08.prompt.md`

**Wave 2** — depends on a Wave-1 task.

- [ ] **D06** *(deps: D01)* — Add `CostsDb::predict_task_cost(model, complexity, role) → CostEstimate { min, expected, max, confidence }` using p10/p50/p90 of historical records. Log estimate before dispatch in `orchestrate.rs`. Fall back to `CostTable::estimate_cost()` when no history. → `prompts/D06.prompt.md`
- [ ] **D09** *(deps: D04)* — Create `crates/roko-learn/src/competitive_bench.rs` with `CompetitorBaseline` records (LangGraph, CrewAI, raw API). Static, manually maintained. Display ratios in `roko learn costs` output. → `prompts/D09.prompt.md`

---

## How to use this tracker

1. Pick a wave, kick the runner: `bash tmp/runners/productionizing/run.sh --group H`.
2. After a batch lands on `wp-arch2`, re-grep the codebase to confirm the symbol/file/line referenced in this tracker actually changed.
3. Tick the box only when (a) batch merged green, (b) re-grep passes, (c) verification block in the prompt passes.
4. Do not move an item to "done above" without a one-line line-number citation. The audit will fail otherwise.

## Out of scope (deliberately not in this runner)

- C-prefix critical fixes from `06-AUDIT-FINDINGS.md` other than the ones P01–P18 already cover (M2 schema migration, M4 SSE secret scrubbing, M8 `todo!()`/`unimplemented!()` in `bench.rs`).
- L1–L4 nice-to-haves (stale temp file cleanup, explicit SIGTERM handler, WS idle timeout, true concurrency).
- Anything touching `docker/mirage-demo.Dockerfile` (separate demo stack).

If you want any of these in scope, file a new batch with prefix `M_` (medium-not-blocking) or `L_` (low-priority).
