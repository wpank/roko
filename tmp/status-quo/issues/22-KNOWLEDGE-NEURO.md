# Knowledge/Neuro Store Issues

## Critical

### Cross-process write corruption
- `knowledge_store.rs:501-505`: `write_gate: Arc<Mutex<()>>` is process-local. When `roko-cli` and `roko serve` run simultaneously, two independent `KnowledgeStore` instances write same file.
- Two concurrent `rewrite_all` calls: second `rename` silently overwrites first's changes.

### Confirmation + append double-write fragility
- `knowledge_store.rs:560-621`: `rewrite_all` (line 598) followed by append (lines 601-613). Crash between them → confirmations incremented but new entries lost.

## High

### No automatic GC or decay in plan runner
- `decay()`, `gc()`, `prune_dead()` only called from manual `roko knowledge gc` CLI command.
- Demurrage only runs under `roko serve`. Plan-only runs → knowledge grows unboundedly.
- `knowledge-confirmations.jsonl`, `knowledge-lifecycle.jsonl`, `knowledge-candidates.jsonl` are append-only with no GC at all.

### O(n) full file scan on every query
- Every `query()`, `query_hits()`, etc. calls `read_all()` — opens and reads entire JSONL from disk.
- No in-memory index or cache. 4-6 full file scans per agent dispatch in the hot path.
- Plus tokenization and HashSet intersection on every call.

### Knowledge routing is functionally dead
- `build_knowledge_routing_advice()` IS called (`orchestrate.rs:15568`).
- But store is almost never written with model-specific routing entries.
- `has_signal = false` branch always taken → knowledge-informed routing has no data.

## Medium

### Silent deserialization failure drops data
- `knowledge_store.rs:1570-1572`: Malformed lines silently skipped. Subsequent `rewrite_all` permanently removes them.

### AntiKnowledge never GC'd
- `gc()` at line 1027-1037: AntiKnowledge unconditionally preserved even at 0.0 confidence. Confidence floor is 0.3 (never decays below). Entries are immortal.

### Inconsistent promotion logic (4 systems)
- `ingest()` (confirmation_count >= 2), `TierProgression::evaluate` (3 passing verdicts), `evaluate_v2` (per-tier thresholds), `maybe_adjust_tier()` (confidence >= 0.9). All can promote to different tiers.

### `created_at` defaults to current time on deser
- `lib.rs:377`: `#[serde(default = "Utc::now")]`. Old entries look brand-new, breaking decay calculations.

### `ingest()` swallows read failures
- `knowledge_store.rs:506`: `read_all().unwrap_or_default()`. Corrupt file → proceeds as empty → duplicates everything.

### Write-only auxiliary files
- `knowledge-confirmations.jsonl`, `knowledge-lifecycle.jsonl`, `heuristic-observations.jsonl`, `knowledge-admission-decisions.jsonl` — all append-only, never consumed at runtime.
