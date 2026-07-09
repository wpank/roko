# Implementation Plan: Production Deployment Reality (from R4)

> Source: `docs/v2-depth/RESEARCH-PROMPT-4.md`
> Scope: Bridge the gap between roko's technical capabilities and production-readiness.
> Covers: cost management, caching, developer experience, measurement, compliance,
> scale testing, and competitive differentiation.

## What Already Exists (DO NOT REBUILD)

The codebase has extensive production infrastructure that's already built and wired.
Search before building:

| Area | Key Files | Status |
|------|-----------|--------|
| Cost tracking (30+ model SKUs) | `roko-learn/src/{costs_db,costs_log,cost_table}.rs` | **Wired** |
| Response caching (in-memory + file) | `roko-agent/src/{cache,file_cache}.rs` | **Wired** |
| Model routing (3-stage cascade) | `roko-learn/src/cascade_router.rs` | **Wired** |
| Efficiency metrics (per-turn) | `roko-learn/src/efficiency.rs` | **Wired** |
| Episode logging (full trace) | `roko-learn/src/episode_logger.rs` | **Wired** |
| Bench suite + SWE-bench | `roko-serve/src/{bench,routes/swe_bench}.rs` | **Wired** |
| OpenAPI (auto-generated) | `roko-serve/src/openapi.rs` | **Wired** |
| Config provenance tracking | `roko-core/src/config/provenance.rs` | **Wired** |
| ACP protocol (jsonrpc 2.0) | `roko-acp/` | **Wired** |
| Routing decision logging | `roko-learn/src/routing_log.rs` | **Wired** |
| C-factor (collective intelligence) | `roko-learn/src/cfactor.rs` | **Wired** |
| Latency tracking (per-model EMA) | `roko-learn/src/latency.rs` | **Wired** |
| Cost spike detection | `roko-learn/src/costs_log.rs` | **Wired** |

---

## Anti-Patterns (READ FIRST)

1. **DO NOT build a new cost tracking system.** `costs_db.rs` already tracks 13 dimensions per request with 30+ model pricing tiers. Add to it, don't replace it.
2. **DO NOT build a new caching layer.** `cache.rs` has TTL, dedup, content hashing. Extend it for semantic caching, don't create a parallel system.
3. **DO NOT add billing/payment integration yet.** Cost tracking is internal. Customer billing is a separate product decision.
4. **DO NOT publish to crates.io without review.** The API surface is not stable. Publishing prematurely locks you into backward compatibility.
5. **DO NOT add OpenTelemetry without config.** Tracing export should be opt-in, not default. Production systems don't want unexpected egress.
6. **DO NOT create SDK client libraries.** The OpenAPI spec auto-generates clients. If you need a client, use `openapi-generator`.
7. **DO NOT add auth bypass for convenience.** The server already has `serve.auth.enabled = false` as an anti-pattern. Don't make it worse.
8. **Wire, don't build.** Check `07-ANTI-PATTERNS.md` for the full list.

---

## Tasks

### D1: Add Budget Enforcement (Cost Limits per Plan/Task/Session)

**Priority**: Critical
**Scope**: ~200 LOC across 3 files
**Dependencies**: None (builds on existing `costs_db.rs`)
**Blocks**: D2

**Context**: Cost tracking exists (`costs_db.rs`, `costs_log.rs`, `cost_table.rs`) but there's no enforcement — an agent can run up unlimited costs. The `task_runner.rs` has a `budget` field but it's checked against token count, not dollar cost.

**What to do**:
1. Find existing budget tracking:
   ```bash
   grep -rn 'budget\|Budget\|cost_limit\|max_cost' crates/roko-agent/src/ crates/roko-learn/src/ --include='*.rs'
   ```
2. Find where costs are recorded per-task:
   ```bash
   grep -rn 'CostRecord\|record_cost\|costs_db' crates/roko-cli/src/orchestrate.rs
   ```
3. Add budget enforcement to `orchestrate.rs`:
   - Before each agent dispatch, check cumulative plan cost against `plan.max_cost_usd` (new config field).
   - Before each task, check cumulative task cost against `task.max_cost_usd`.
   - If budget exceeded: log error, skip task, mark as failed with reason `BudgetExceeded`.
4. Add config fields to `roko-core/src/config/`:
   ```toml
   [agent]
   max_cost_per_plan_usd = 10.0    # default: $10 per plan
   max_cost_per_task_usd = 2.0     # default: $2 per task
   max_cost_per_session_usd = 50.0 # default: $50 per session
   ```
5. Use `CostTable::estimate_cost()` for pre-dispatch cost estimation (already exists).
6. Use `CostsDb::query_by_plan()` for cumulative cost lookup (already exists).

**What NOT to do**:
- Don't create a new cost tracking system — use `costs_db.rs`.
- Don't make budget enforcement optional-per-model — it's a global limit.
- Don't add Stripe/billing code — budget enforcement is internal guardrails, not customer billing.
- Don't block the entire plan on a single task's budget failure — skip the task, continue the plan.
- Don't use `unwrap()` on cost lookups — use `unwrap_or(0.0)` for missing pricing data.

**Verification**:
```bash
cargo build --workspace
cargo test --workspace

# Verify config fields parse:
grep -rn 'max_cost_per_plan\|max_cost_per_task\|max_cost_per_session' crates/roko-core/src/config/ --include='*.rs'

# Verify enforcement is called before dispatch:
grep -rn 'BudgetExceeded\|budget_exceeded\|check_budget' crates/roko-cli/src/orchestrate.rs
```

---

### D2: Add Semantic Caching (Embedding-Based Cache Hits)

**Priority**: High
**Scope**: ~300 LOC in 2 files
**Dependencies**: None
**Blocks**: None

**Context**: `crates/roko-agent/src/cache.rs` has in-memory response caching with exact content hash matching (blake3). This misses near-duplicate prompts that would get the same answer. Semantic caching uses embedding similarity to find cache hits for semantically similar (but not identical) prompts.

**What to do**:
1. Read the existing cache:
   ```bash
   grep -rn 'pub fn\|pub struct\|pub async fn' crates/roko-agent/src/cache.rs
   ```
2. Add a `SemanticCache` struct alongside the existing `ResponseCache`:
   - Store (HDC fingerprint, response, timestamp, ttl) tuples.
   - On cache lookup: compute HDC fingerprint of the prompt (using existing `hdc_fingerprint.rs`), compare against stored fingerprints with Hamming similarity.
   - If similarity > 0.92 (configurable threshold), return cached response.
   - On cache miss: store the new entry after response is received.
3. Use existing `roko-primitives/src/hdc.rs` for fingerprinting — DO NOT add an embedding model dependency.
4. Wire into the agent dispatch path in `roko-agent/src/model_call_service.rs` or wherever the LLM call is made:
   ```rust
   if let Some(cached) = semantic_cache.lookup(&prompt_fingerprint) {
       tracing::info!(similarity = cached.similarity, "semantic cache hit");
       return Ok(cached.response);
   }
   ```
5. Add config:
   ```toml
   [agent.cache]
   semantic_enabled = true
   semantic_threshold = 0.92  # Hamming similarity threshold
   semantic_max_entries = 10000
   semantic_ttl_secs = 3600   # 1 hour
   ```

**What NOT to do**:
- Don't use an external embedding model (OpenAI embeddings, etc.) — HDC fingerprints are free and fast (~10ns).
- Don't replace the existing exact-match cache — semantic cache is supplementary (check exact first, then semantic).
- Don't cache tool-calling responses — only cache pure generation responses (tool calls are stateful).
- Don't cache responses for prompts shorter than 100 tokens — too short for meaningful semantic matching.
- Don't persist semantic cache to disk — in-memory only, rebuilt on restart (it fills quickly from natural traffic).

**Verification**:
```bash
cargo build -p roko-agent
cargo test -p roko-agent

# Unit test: two prompts with same meaning but different wording should match
# Unit test: unrelated prompts should NOT match (similarity < threshold)
# Unit test: cache entry expires after TTL
# Unit test: cache evicts oldest when max_entries exceeded
```

---

### D3: Add Cost-Per-Feature Metric

**Priority**: Medium
**Scope**: ~100 LOC in 1-2 files
**Dependencies**: None
**Blocks**: None

**Context**: The system tracks per-turn costs (efficiency events) and per-model costs (cost table), but there's no aggregation of "how much did it cost to implement feature X?" or "what's the cost per SLOC of generated code?". This is critical for proving the 10-30× cost reduction claim.

**What to do**:
1. Read existing cost aggregation:
   ```bash
   grep -rn 'CostSummary\|daily_breakdown\|aggregate' crates/roko-learn/src/costs_db.rs
   ```
2. Add a `plan_cost_summary()` function to `costs_db.rs` that aggregates:
   - Total USD spent on the plan
   - Total tokens (input + output + cached)
   - Cost per task
   - Cost per SLOC (if gate artifacts include diff line counts)
   - Cost per successful gate pass
3. Add a CLI command to display this: `roko learn costs --plan <plan-id>`
4. Add the summary to the plan completion log in `orchestrate.rs`:
   ```rust
   tracing::info!(
       plan_id = %plan_id,
       total_cost_usd = summary.total_usd,
       cost_per_task = summary.per_task_avg_usd,
       cost_per_sloc = summary.per_sloc_usd,
       tasks_completed = summary.tasks_completed,
       "plan completed"
   );
   ```

**What NOT to do**:
- Don't create a new aggregation system — extend `CostSummary` in `costs_db.rs`.
- Don't compute SLOC from raw code — use diff line counts from gate artifacts (already stored).
- Don't add a database dependency — use the existing JSONL + in-memory approach.

**Verification**:
```bash
cargo build --workspace
cargo test --workspace
cargo run -p roko-cli -- learn costs --help
```

---

### D4: Add Continuous Benchmark Regression Detection

**Priority**: Medium
**Scope**: ~200 LOC across 2 files
**Dependencies**: None
**Blocks**: None

**Context**: `roko-serve/src/bench.rs` has `BenchSuite` and `BenchTask` types. SWE-bench integration exists at `routes/swe_bench.rs`. But there's no regression detection — no comparison of current run vs previous runs, no alerting when performance drops.

**What to do**:
1. Read existing bench infrastructure:
   ```bash
   grep -rn 'pub fn\|pub struct' crates/roko-serve/src/bench.rs
   ```
2. Add a `BenchHistory` struct to `crates/roko-learn/src/`:
   - Stores timestamped benchmark results: `(timestamp, suite_name, task_results[])`.
   - Persists to `.roko/learn/bench-history.jsonl`.
   - On new result: compare against trailing 5-run average.
   - Flag regression if: pass_rate drops >10%, cost increases >20%, latency increases >30%.
3. Wire into the bench suite runner:
   - After each bench run, append to history and check for regressions.
   - If regression detected: `tracing::warn!("benchmark regression: ...")`.
4. Add a CLI command: `roko learn bench --history` to show trend.

**What NOT to do**:
- Don't add a database — use JSONL append (same pattern as costs_log.rs).
- Don't block bench runs on regression detection — detection is post-run, advisory.
- Don't compare against a single previous run — use trailing average (reduces noise).
- Don't send external alerts — just log and persist. External alerting is a separate integration.

**Verification**:
```bash
cargo build --workspace
cargo test --workspace
# Verify history file format:
grep -rn 'BenchHistory\|bench_history' crates/roko-learn/src/ --include='*.rs'
```

---

### D5: Add Cache Hit Rate Tracking

**Priority**: Medium
**Scope**: ~80 LOC in 1 file
**Dependencies**: None
**Blocks**: None

**Context**: The response cache (`cache.rs`) has no statistics. You can't tell if the cache is effective, what the hit rate is, or how much money it's saving.

**What to do**:
1. Read the cache:
   ```bash
   cat crates/roko-agent/src/cache.rs
   ```
2. Add a `CacheStats` struct:
   ```rust
   pub struct CacheStats {
       pub hits: AtomicU64,
       pub misses: AtomicU64,
       pub evictions: AtomicU64,
       pub total_entries: AtomicU64,
       pub estimated_savings_usd: AtomicU64, // cents, not dollars (avoid floating point atomics)
   }
   ```
3. Increment counters on each cache operation.
4. Add a method to `ResponseCache`: `pub fn stats(&self) -> CacheStats`.
5. Log stats periodically (every 100 requests) via `tracing::info!`.
6. Expose via the `/api/health` endpoint (add cache stats to health response).

**What NOT to do**:
- Don't use `Mutex` for stats — use `AtomicU64` for lock-free counters.
- Don't add a new endpoint — extend the existing health endpoint.
- Don't use floating point atomics for savings — store in cents as u64.

**Verification**:
```bash
cargo build -p roko-agent
cargo test -p roko-agent
# Verify stats appear in health:
grep -rn 'cache_stats\|CacheStats' crates/roko-serve/src/routes/ --include='*.rs'
```

---

### D6: Add Predictive Cost Estimation Before Dispatch

**Priority**: High
**Scope**: ~150 LOC across 2 files
**Dependencies**: D1 (budget enforcement)
**Blocks**: None

**Context**: The cost table has pricing data for 30+ models, and the efficiency logger tracks per-turn token usage. But before dispatching an agent, there's no estimate of "this task will cost approximately $X with model Y." This is needed for informed routing decisions and budget enforcement.

**What to do**:
1. Find the cost estimation function:
   ```bash
   grep -rn 'estimate\|predict.*cost' crates/roko-learn/src/cost_table.rs crates/roko-learn/src/costs_db.rs
   ```
2. Add a `predict_task_cost()` function to `costs_db.rs`:
   - Input: model_id, task_complexity (from curriculum scheduler), role
   - Output: `CostEstimate { min_usd, expected_usd, max_usd, confidence }`
   - Method: query historical cost records for similar (model, complexity, role) tuples. Use percentile-based estimation (p10 = min, p50 = expected, p90 = max).
   - If no history: use the cost table's per-token pricing × estimated token count (from prompt length + average output length per complexity band).
3. Wire into pre-dispatch in orchestrate.rs:
   ```rust
   let estimate = costs_db.predict_task_cost(&model, complexity, &role);
   tracing::info!(
       model = %model,
       estimated_cost = estimate.expected_usd,
       "pre-dispatch cost estimate"
   );
   if estimate.expected_usd > budget_remaining {
       tracing::warn!("estimated cost exceeds remaining budget, trying cheaper model");
       // trigger cascade router to select cheaper model
   }
   ```
4. Use existing `CostsDb::query()` for historical data — don't add new storage.

**What NOT to do**:
- Don't add ML-based prediction — use simple percentile statistics from historical data.
- Don't block dispatch on estimation failure — if no data, use cost table defaults.
- Don't add a new config section — reuse the existing `[agent]` section's budget fields.

**Verification**:
```bash
cargo build --workspace
cargo test --workspace
# Verify prediction is called before dispatch:
grep -rn 'predict_task_cost\|CostEstimate' crates/roko-cli/src/orchestrate.rs
```

---

### D7: Add Structured Compliance Export

**Priority**: Low
**Scope**: ~200 LOC in 1 new file within existing crate
**Dependencies**: None
**Blocks**: None

**Context**: The system logs episodes, costs, routing decisions, gate artifacts, and config provenance — but there's no way to export a structured compliance report. Enterprise buyers need SOC 2 audit trails and data retention evidence.

**What to do**:
1. Add `compliance_export.rs` to `crates/roko-learn/src/`:
   - `export_audit_trail(date_range: Range<DateTime>) -> AuditReport`
   - AuditReport contains:
     - All agent invocations with timestamps, models used, costs
     - All gate results (pass/fail) with artifacts
     - All config changes with provenance
     - All routing decisions with justification
     - Cost summary per model/provider
   - Export formats: JSON, CSV
2. Add CLI command: `roko learn export --from 2026-04-01 --to 2026-05-01 --format json > audit.json`
3. Use existing data sources:
   - Episodes from `.roko/episodes.jsonl`
   - Costs from `.roko/learn/costs.jsonl`
   - Routing from routing log
   - Config provenance from `provenance.rs`

**What NOT to do**:
- Don't add database dependencies — read from existing JSONL files.
- Don't add encryption — that's a separate task.
- Don't claim SOC 2 / HIPAA compliance — this is just the export. Certification is a business process.
- Don't include raw prompts/responses in the audit trail — they may contain sensitive data. Include metadata only (model, cost, tokens, pass/fail, timestamps).

**Verification**:
```bash
cargo build -p roko-learn
cargo test -p roko-learn
cargo run -p roko-cli -- learn export --help
```

---

### D8: Add OpenTelemetry Tracing Export (Opt-In)

**Priority**: Low
**Scope**: ~150 LOC across 2 files
**Dependencies**: None
**Blocks**: None

**Context**: All logging uses `tracing` (structured, leveled). But there's no export to external observability systems (Datadog, Grafana, New Relic). The `tracing` crate has an OpenTelemetry layer that can be added with minimal code.

**What to do**:
1. Find the tracing subscriber setup:
   ```bash
   grep -rn 'tracing_subscriber\|EnvFilter\|init_tracing\|subscriber' crates/roko-cli/src/main.rs crates/roko-serve/src/lib.rs
   ```
2. Add optional `tracing-opentelemetry` and `opentelemetry-otlp` dependencies to `roko-cli/Cargo.toml` (feature-gated):
   ```toml
   [features]
   otel = ["tracing-opentelemetry", "opentelemetry-otlp", "opentelemetry-sdk"]
   ```
3. In the tracing subscriber setup, conditionally add the OTEL layer:
   ```rust
   #[cfg(feature = "otel")]
   if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
       // Add OpenTelemetry layer
   }
   ```
4. Add config:
   ```toml
   [telemetry]
   otel_enabled = false
   otel_endpoint = "http://localhost:4317"
   otel_service_name = "roko"
   ```

**What NOT to do**:
- Don't make OTEL a required dependency — feature-gate it so it doesn't bloat the binary.
- Don't enable by default — only activate when `OTEL_EXPORTER_OTLP_ENDPOINT` is set.
- Don't add span instrumentation to every function — only instrument the orchestrator loop, agent dispatch, and gate pipeline.
- Don't add metrics export — start with traces only.

**Verification**:
```bash
# Build without OTEL (default):
cargo build -p roko-cli
# Build with OTEL:
cargo build -p roko-cli --features otel
# Verify it's opt-in:
cargo run -p roko-cli -- serve --help  # should work without OTEL env vars
```

---

### D9: Add Competitive Benchmark Comparison

**Priority**: Medium
**Scope**: ~100 LOC in 1 file
**Dependencies**: D4 (bench history)
**Blocks**: None

**Context**: There are no published benchmarks comparing roko vs LangGraph, CrewAI, AutoGen, or other agent frameworks. The 10-30× cost reduction claim has no comparative evidence.

**What to do**:
1. Add `competitive_bench.rs` to `crates/roko-learn/src/`:
   - Define baseline metrics for competing frameworks (manually researched):
     ```rust
     pub struct CompetitorBaseline {
         pub name: String,           // "LangGraph", "CrewAI", etc.
         pub avg_cost_per_task_usd: f64,
         pub avg_tokens_per_task: u64,
         pub avg_latency_ms: u64,
         pub source: String,         // citation for the numbers
         pub date: String,           // when measured
     }
     ```
   - `compare_against_baselines(our_summary: &CostSummary) -> ComparisonReport`
   - ComparisonReport shows: cost ratio, token ratio, latency ratio per competitor.
2. Populate baselines from public data:
   - LangGraph: typical token usage from docs/benchmarks
   - CrewAI: published case studies
   - Raw LLM calls (no framework): direct API cost for same prompts
3. Wire into `roko learn costs` CLI output:
   ```
   Plan cost: $4.50 (23 tasks, 145K tokens)
   vs LangGraph baseline: 3.2× cheaper
   vs raw API calls:      1.8× cheaper (caching saved $3.40)
   ```

**What NOT to do**:
- Don't fabricate competitor numbers — use only published/verifiable data.
- Don't claim superiority without data — if roko is more expensive, show that honestly.
- Don't add API calls to competitor services — baselines are static, manually maintained.
- Don't make this a marketing tool — it's an internal measurement system.

**Verification**:
```bash
cargo build -p roko-learn
cargo test -p roko-learn
# Verify baselines exist:
grep -rn 'CompetitorBaseline' crates/roko-learn/src/ --include='*.rs'
```

---

## Execution Order

```
CRITICAL (do first):
  D1 (Budget enforcement)

HIGH (do next):
  D2 (Semantic caching)
  D6 (Predictive cost estimation) — depends on D1

MEDIUM (can parallel):
  D3 (Cost per feature)
  D4 (Bench regression detection)
  D5 (Cache hit rate tracking)
  D9 (Competitive benchmarks) — depends on D4

LOW (before or after deploy):
  D7 (Compliance export)
  D8 (OpenTelemetry export)
```

## Dependency Graph

```
D1 (Budget enforcement)
 └─► D6 (Predictive cost estimation)

D4 (Bench regression detection)
 └─► D9 (Competitive benchmarks)

Independent: D2, D3, D5, D7, D8
```

## Checklist

- [ ] D1: Budget enforcement with per-plan/task/session limits
- [ ] D2: Semantic caching using HDC fingerprints
- [ ] D3: Cost-per-feature metric with CLI display
- [ ] D4: Continuous bench regression detection
- [ ] D5: Cache hit rate tracking with atomic counters
- [ ] D6: Predictive cost estimation before dispatch
- [ ] D7: Structured compliance export (JSON/CSV)
- [ ] D8: OpenTelemetry tracing (feature-gated, opt-in)
- [ ] D9: Competitive benchmark baselines
- [ ] All tasks pass: `cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace`

## Operations baseline

Cost and cache work assume correct routing keys and durable `.roko/` state — run **09-OPERATIONS-RUNBOOK.md** checks after enabling **D1** budgets so alerts match real HTTP paths (`/health` vs `/api/health`).
