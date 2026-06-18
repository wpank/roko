# Already-built code (verify before re-implementing)

The plans in `tmp/productionizing/{11,12}-*.md` claim a lot of code "exists" already. Some of it does, some doesn't. Below is the snapshot from the audit run on **2026-05-01**. **Verify with a fresh `ls` / `grep` before assuming.**

## EXISTS in tree

| File | What | Why it matters here |
|---|---|---|
| `crates/roko-core/src/config/schema.rs:440–514` | `is_provider_available`, `provider_available_for_model_key`, `available_provider_ids`, `available_model_keys_for_cascade`, `available_model_slugs_for_cascade` | Use these, don't reinvent (P04, P09, D01 prompts) |
| `crates/roko-learn/src/cascade_router.rs:648–688` | `route_with_health`, `filter_unhealthy` | Already filtering by health; F03 wires sheaf inconsistency on top |
| `crates/roko-agent/src/dispatch_resolver.rs:273–286` | `fallback_candidates` filters by `provider_available_for_model_key` | Don't double-filter (P09) |
| `crates/roko-serve/src/lib.rs:641–660` | `validate_bind_safety` errors on public bind without auth | Don't weaken (informational) |
| `crates/roko-serve/src/lib.rs:762` | `log_provider_credential_status` startup banner | Don't duplicate (informational) |
| `crates/roko-serve/src/embedded.rs:39, 69–87` | path canonicalization | Don't redo (informational) |
| `crates/roko-fs/src/gc.rs:186` | `GcEngine::should_auto_gc()` | P11 wires it |
| `crates/roko-learn/src/episode_logger.rs:964` | `EpisodeLogger::append()` | P05 adds flock around the write |
| `crates/roko-learn/src/feedback_service.rs` | `FeedbackService::flush()` | P05 adds flock around the flush |
| `crates/roko-learn/src/cascade_router.rs` | `save()` | P05 adds flock |
| `crates/roko-learn/src/costs_db.rs` | `CostsDb`, `CostRecord`, `query_by_plan`, `total_cost`, daily aggregation | D03, D06 extend |
| `crates/roko-learn/src/costs_log.rs` | append-only `costs.jsonl` | D03, D07 read |
| `crates/roko-learn/src/cost_table.rs` | per-model pricing, `estimate_cost(...)` | D06 falls back to it when no history |
| `crates/roko-agent/src/cache.rs:16–128` | `ResponseCache`, `request_hash`, `shared_response_cache` | D02 adds `SemanticCache` alongside; D05 adds atomic stats |
| `crates/roko-agent/src/model_call_service.rs` | `ModelCallService`, `cost_predict`, `with_cascade_router`, `BudgetExceeded` returns | D01 extends; D02 wires semantic cache; D06 wires prediction |
| `crates/roko-agent/src/multi_pool.rs` | `MultiAgentPool` | F06 fans it out per-task |
| `crates/roko-agent/src/composition.rs` | `MergeStrategy { Concatenate, Aggregate, Vote, BestOfN }` | F06 reuses |
| `crates/roko-chain/src/collusion.rs:28–113` | `AssignmentEdge`, `CollusionConfig`, `CollusionRing`, `CollusionReport`, `CollusionDetector::{new, record_assignment, assignment_count, detect}` | F07 wires |
| `crates/roko-chain/src/reputation_registry.rs` | `CollisionFeedbackDilution` (note spelling) | F07 reuses penalty values |
| `crates/roko-daimon/src/phase2_stubs.rs` | `FatigueDetector`, `ErrorPatternTracker` | Already wired (F04 done) |
| `crates/roko-daimon/src/lib.rs:1999, 2030` | `DaimonState.fatigue_detector` | Already wired (F04 done) |
| `crates/roko-primitives/src/hdc.rs` | HDC vectors, hamming/cosine similarity | D02, F05 reuse |
| `crates/roko-primitives/src/tropical.rs` | `TropicalF64`, polynomials, attention | F03 wires |
| `crates/roko-primitives/src/sheaf.rs` | `CellularSheaf`, coboundary, Laplacian, inconsistency | F03 wires |
| `crates/roko-learn/src/hdc_fingerprint.rs` | `fingerprint_text`, `fingerprint_episode` | D02, F05 reuse |
| `crates/roko-learn/src/bandits.rs` | UCB1, Thompson, contextual bandit | F05 reuses |
| `crates/roko-core/src/error/mod.rs:62, 273, 297, 348` | `RokoError::BudgetExceeded`, `ErrorKind::BudgetExceeded` | D01 reuses, doesn't redefine |
| `crates/roko-runtime/src/run_ledger.rs:385` | `BudgetExceeded` plan event | D01 may emit |
| `crates/roko-cli/src/commands/learn.rs` | `roko learn ...` subcommand host | F02, D03, D07, D09 attach here |
| `crates/roko-cli/src/commands/util.rs:514` | `costs_log` accessor | D03 reuses |
| `crates/roko-serve/src/bench.rs` | `BenchSuite`, `BenchTask` | D04 wraps with `BenchHistory` |

## DOES NOT EXIST in tree (must be created)

| Missing file | Created by | Why the plan says "exists" |
|---|---|---|
| `crates/roko-fs/src/flock.rs` | **P05** | New file. The plan asked for it. |
| `crates/roko-learn/src/adas.rs` | **F01** | The frontier-plan claims `roko-learn/src/adas.rs` already exists with `AdasOptimizer`. As of 2026-05-01 it does NOT exist. F01 must create it. **Verify with `ls crates/roko-learn/src/adas.rs` before deciding.** |
| `crates/roko-learn/src/research_pipeline.rs` | **F02** | Same situation as adas.rs. Claimed but absent. Verify, then create. |
| `crates/roko-learn/src/novelty_search.rs` | **F05** | New file. |
| `crates/roko-learn/src/bench_history.rs` | **D04** | New file. |
| `crates/roko-learn/src/compliance_export.rs` | **D07** | New file. |
| `crates/roko-learn/src/competitive_bench.rs` | **D09** | New file. |
| `Dockerfile.optimized` | **P13** | New file. |
| `Dockerfile.runtime` | **P16** | New file. |
| `roko.production.toml` | **P15** | New file. |
| `deploy.sh` | **P17** | New file. |
| `crates/roko-cli/src/telemetry.rs` | **D08** | May or may not exist. Verify before creating. |

## Existing patterns to follow

**JSON state persistence** (atomic write + load_or_default):
```rust
// crates/roko-learn/src/cascade_router.rs#save / load_or_default
pub fn save(&self, path: &Path) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(self)?;
    roko_fs::atomic::write_atomic(path, &bytes)
}
pub fn load_or_default(path: &Path) -> Self {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
```
Use this for ADAS state (F01), novelty archive metadata (F05), bench history index (D04), competitive baselines snapshot (D09).

**JSONL append-only** (used for costs_log, episodes, efficiency):
```rust
// crates/roko-learn/src/costs_log.rs#append
let mut file = OpenOptions::new().create(true).append(true).open(&self.path).await?;
file.write_all(&serde_json::to_vec(&record)?).await?;
file.write_all(b"\n").await?;
file.flush().await?;
```
Use this for the research-trial ledger (F02), `bench-history.jsonl` (D04), and the audit trail JSON output (D07).

**HDC fingerprint generation**:
```rust
// crates/roko-learn/src/hdc_fingerprint.rs (existing API)
let fp: HypervectorN = roko_learn::hdc_fingerprint::fingerprint_text(prompt);
let sim = roko_primitives::hdc::hamming_similarity(&a, &b); // 0.0..=1.0
```
Use this for semantic cache (D02) and novelty archive (F05).

## Where to look for usage examples

- **CascadeRouter persistence**: `crates/roko-cli/src/orchestrate.rs` greps for `cascade_router.save` and `load_cascade_router`.
- **CostsDb queries**: `crates/roko-cli/src/commands/util.rs:514–525` shows the canonical access pattern.
- **Adding a `roko learn` subcommand**: copy the structure of an existing subcommand in `crates/roko-cli/src/commands/learn.rs`. Don't add a top-level command in `main.rs`.
- **Tracing subscriber init**: `crates/roko-cli/src/main.rs` — D08 hooks an OTEL layer here.
