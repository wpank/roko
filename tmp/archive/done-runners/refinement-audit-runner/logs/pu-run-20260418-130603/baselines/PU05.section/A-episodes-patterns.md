# A — Episodes + Patterns (Docs 00, 05)

Parity analysis of `docs/05-learning/00-episode-logger.md` and
`docs/05-learning/05-pattern-discovery-trigram.md` vs the actual codebase.

---

## A.01 — `Episode` struct shape and fields

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 00 §"Episode Schema" lists 18 fields: `id`, `agent_id`, `task_id`, `plan_id`, `role`, `model`, `backend`, `success`, `iteration`, `input_tokens`, `output_tokens`, `cost_usd`, `duration_ms`, `gate_verdicts`, `timestamp`, `hdc_fingerprint: Option<HdcVector>`, `extra`.
**Reality**: `crates/roko-learn/src/episode_logger.rs:169-250` defines `Episode` with **25 fields**: `kind`, `id`, `timestamp`, `agent_id`, `task_id`, `input_signal_hash`, `output_signal_hash`, `episode_id`, `agent_template`, `model`, `trigger_kind`, `trigger_signal_hash`, `started_at`, `completed_at`, `duration_secs`, `gate_verdicts`, `usage: Usage`, `success`, `turns`, `tokens_used`, `external_actions`, `failure_reason`, `reasoning_summary`, `emotional_tag`, `headline`, `extra`. Every field carries `#[serde(default)]` for forward-compat.
**Fix sketch**: Update doc schema block to reflect actual 25-field layout. Replace `role` with `agent_template`, `backend` with `trigger_kind`, `iteration` with `turns`. Tokens/cost live inside nested `Usage` struct, not as top-level fields. Drop claimed top-level `hdc_fingerprint` field — fingerprints are stored in `extra` map (see A.06).

---

## A.02 — `GateVerdict` embedded in `Episode`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §"GateVerdict" — three fields: `gate: String`, `passed: bool`, `signature: Option<String>`.
**Reality**: `crates/roko-learn/src/episode_logger.rs:89-100` matches exactly. All three fields present with `#[serde(default)]`. `GateVerdict::new(gate, passed)` constructor at `:105-111` + `with_signature()` builder at `:115-118`. `Episode.gate_verdicts: Vec<GateVerdict>` at `:217`.

---

## A.03 — `EpisodeLogger::append` is crash-safe append-only JSONL

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §"Append Pipeline" — validate `extra` ≤ 16 KB, serialize + `\n`, acquire process-wide mutex, `O_APPEND | O_CREAT`, write, release. Uses `parking_lot::Mutex` not `tokio::Mutex`.
**Reality**: `crates/roko-learn/src/episode_logger.rs:842-868` implements the pipeline. `extra_size > MAX_EXTRA_BYTES` check at `:844` matches doc §"Append Pipeline" step 1. `OpenOptions::new().create(true).append(true)` at `:855-857` matches step 5. `sync_all()` at `:864` adds durability beyond what the doc describes. `MAX_EXTRA_BYTES = 16 * 1024` confirmed at `:47`.
**Notes**: Doc says `parking_lot::Mutex` guards the critical section. Code actually uses **both** — `parking_lot::Mutex` for the write counter at `:798`, `tokio::Mutex` (`AsyncMutex`) for the write gate at `:801`. The `write_gate` uses async locking because `append` is `async`. Substance matches but the explanation in doc §"Concurrency Model" is slightly misleading.

---

## A.04 — `DropPolicy` / bounded queue / backpressure

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: The doc does not explicitly mention `DropPolicy` or backpressure. The item was listed as a suggested check since Mori had such a thing. Doc §"Concurrency Model" describes unbounded serialization via mutex.
**Reality**: `rg 'DropPolicy|BoundedQueue|Backpressure' crates/roko-learn/` returns **zero matches**. There is no bounded queue or drop policy — every `append` call blocks on the write gate until its write completes. Concurrent callers are fully serialized, never dropped.
**Fix sketch**: No doc change needed. If a future bounded queue is added for high-throughput scenarios, add a §"Backpressure" section describing drop/block semantics.

---

## A.05 — `LoggerError` variants (Io, Serde, Parse, ExtraTooLarge)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §"Error Handling" lists four variants: `Io`, `Serde`, `ExtraTooLarge`, `Parse` (with 1-based line numbers).
**Reality**: `crates/roko-learn/src/episode_logger.rs:57-86` defines `LoggerError` as `enum` with exactly those four variants. `Parse { line, source }` matches doc claim of 1-based line indexing. `thiserror::Error` derived. Unit test at `:1374-1381` verifies `ExtraTooLarge { size, max }` with `max == MAX_EXTRA_BYTES`.

---

## A.06 — HDC fingerprinting (text + metadata) stored in `extra`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §"HDC Fingerprinting" — two fingerprints computed per episode: `text_fingerprint` and `metadata_fingerprint`. 10,240-bit vectors. Stored in `extra` under reserved keys, not as top-level fields. Uses `bardo_primitives::hdc::text_fingerprint`.
**Reality**: `crates/roko-learn/src/episode_logger.rs:317-343` implements `attach_text_fingerprint()` and `attach_metadata_fingerprint()`. Both insert into `episode.extra` under `TEXT_FINGERPRINT_KEY` / `METADATA_FINGERPRINT_KEY`. `use ... text_fingerprint` import confirms the `roko_primitives::HdcVector` dependency.
**Notes**: The doc says `bardo_primitives`; the crate has been renamed to `roko_primitives` (see `pattern_discovery.rs:41`). Naming drift only.

---

## A.07 — Tiered storage (Hot/Warm/Cold) + `EpisodeStorageConfig` + `CompressedEpisodeSummary`

**Status**: NOT DONE
**Severity**: HIGH
**Doc claim**: Doc 00 §"Episode Compression and Tiered Storage" — 3-tier architecture (hot JSONL, warm zstd, cold HDC-superposition summaries). Defines `EpisodeStorageConfig` and `CompressedEpisodeSummary` structs. Describes `compact_tier()` pseudocode with `read_recent(days)` and `similarity_search(query_hdc)` integration.
**Reality**: `rg 'TieredStorage|EpisodeStorageConfig|CompressedEpisodeSummary' crates/roko-learn/` returns **zero matches**. What exists instead: `EpisodeLogger::compact()` at `:964-1042` (single-tier age + size pruning using `RetentionPolicy`). No zstd compression. No HDC superposition tiering. No `read_recent`, no `similarity_search`. Headlines are pinned via `ep.headline` flag at `:245` but this is not multi-tier storage.
**Fix sketch**: Either (a) implement `EpisodeStorageConfig` + `compact_tier()` as described, or (b) mark doc §"Episode Compression and Tiered Storage" as "Design — not yet implemented" and keep the existing `compact()` + `RetentionPolicy` docs as the shipping behavior. Doc also wrongly claims "current implementation does not compact or rotate" (line 172) — compaction landed in `:964-1042`.

---

## A.08 — `EpisodeImportance` + `ImportanceComponents` scoring

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 00 §"Episode Importance Scoring" defines `EpisodeImportance { score, components }` and `ImportanceComponents { surprisal, novelty, difficulty_signal, information_gain, diversity }` with default weights `w_s=0.30, w_n=0.20, w_d=0.15, w_i=0.20, w_v=0.15`.
**Reality**: `crates/roko-learn/src/episode_logger.rs:384-410` defines `EpisodeImportanceComponents` (not `ImportanceComponents` + wrapper) with five fields: `surprisal`, `novelty`, `difficulty` (not `difficulty_signal`), `information_gain`, `diversity`. The composite `score()` method at `:402-409` uses weights `0.3, 0.25, 0.2, 0.15, 0.1` — different from the doc's `0.30, 0.20, 0.15, 0.20, 0.15`. Free functions `importance_score()` at `:491`, `importance_components()` at `:497-513`, `importance_tier()` at `:517-523`, and `prioritize_by_importance()` at `:528-540` all exist. Test coverage: `:1847-1871`.
**Fix sketch**: Align doc weights with code (`0.30, 0.25, 0.20, 0.15, 0.10`) or justify the discrepancy. Rename `difficulty_signal` → `difficulty` in doc. Replace the `EpisodeImportance { score, components }` wrapper in doc with the actual free-function API.

---

## A.09 — Auto-pattern discovery / episode clustering (DBSCAN)

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 00 §"Episode Clustering and Automatic Pattern Discovery" — incremental DBSCAN over HDC space. Defines `EpisodeClusterConfig { eps_similarity, min_points, max_batch_size, metric }` and `EpisodeCluster` + `ClusterEvolution` structs. Claims integration via `On new episode → find nearest cluster → update superposition`.
**Reality**: `rg 'auto_pattern_discovery|AutoPatternDiscovery|EpisodeCluster|incremental_dbscan|DBSCAN|eps_similarity' crates/roko-learn/` returns **zero matches**. Only k-medoids clustering exists (see A.13). No DBSCAN. No `EpisodeCluster`. No cluster evolution tracking.
**Fix sketch**: Mark doc §"Episode Clustering and Automatic Pattern Discovery" as "Design — not implemented". k-medoids in `hdc_clustering.rs` does not satisfy the DBSCAN spec (fixed k, no density-based outlier detection, no incremental updates).

---

## A.10 — `PatternMiner` trigram mining with support + confidence

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Trigram Mining Algorithm" — `PatternMiner { min_support, min_confidence }` with FNV-1a 64-bit signatures, `BTreeMap<u64, TrigramStats>` keyed by signature, `ingest_episode()` + `discover()` API. Default `min_support=2`, `min_confidence=0.5`.
**Reality**: `crates/roko-learn/src/pattern_discovery.rs:99-105` defines `PatternMiner { min_support, min_confidence, total_episodes, stats: BTreeMap<u64, TrigramStats>, clock_ms }`. `hash_trigram()` at `:639-653` implements FNV-1a with `FNV_OFFSET=0xcbf29ce484222325`, `FNV_PRIME=0x100000001b3`, plus `0x1f` delimiter byte. `ingest_episode` at `:150-181` clones the windowed slice and updates `TrigramStats`. Constructor uses explicit `(min_support, min_confidence)` args at `:114`; runtime default in `runtime_feedback.rs:376` picks `PatternMiner::new(3, 0.5)` — **doc claims default `min_support=2`, code uses `3`**.
**Notes**: The default-support discrepancy (doc 2, code 3) is minor drift. Algorithm, signature format, and API match the doc exactly.

---

## A.11 — `Pattern` struct + `EpisodeView` trait

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"EpisodeView Trait" — trait with `fn actions(&self) -> &[String]` + `fn succeeded(&self) -> bool`. Doc §"Step 2: Discover" — `Pattern { id, signature, description, support_count, confidence, first_seen_ms, last_seen_ms }`.
**Reality**: `crates/roko-learn/src/pattern_discovery.rs:53-58` defines the trait with exactly those two methods. `Pattern` struct at `:66-81` has all seven fields with types matching doc. `EpisodeActions` adapter at `runtime_feedback.rs:54-66` + `impl EpisodeView for EpisodeActions` at `:73-80` wraps an `Episode` by extracting gate names from `gate_verdicts`, matching doc's code snippet byte-for-byte.

---

## A.12 — `CrossEpisodeConsolidator` + meta-pattern discovery

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Cross-Episode Consolidation" — consolidator groups episodes via HDC fingerprints, produces cluster-level meta-patterns via `CrossEpisodeConsolidationReport`.
**Reality**: `crates/roko-learn/src/pattern_discovery.rs:291-296` defines `CrossEpisodeConsolidator { target_clusters, min_cluster_size, max_iterations, min_coherence }`. Default at `:298-307` picks `target_clusters=4, min_cluster_size=2, max_iterations=50, min_coherence=0.55`. `discover()` at `:353-407` runs k-medoids over `episode_vector(ep)` fingerprints and emits `CrossEpisodeMetaPattern` rows with medoid index, bundle superposition, and signature. `CrossEpisodeConsolidationReport` carries `meta_pattern_count`, `iterations`, `converged`, `meta_patterns`.

---

## A.13 — `k_medoids` clustering + `KMedoidsConfig`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"k-Medoids Algorithm" — PAM over 10,240-bit `HdcVector`, greedy farthest-first seeding, assign-update loop until `max_iterations` or stable medoids. Default `k=3, max_iterations=100`.
**Reality**: `crates/roko-learn/src/hdc_clustering.rs:38-52` defines `KMedoidsConfig { k, max_iterations }` with `Default` = `(3, 100)`. `k_medoids()` at `:81-` returns `ClusterResult { clusters, iterations, converged }`. Implementation follows doc: precompute distance matrix, greedy BUILD seeding, alternating assign/update. Distance metric is `1.0 - similarity` where similarity is HDC Hamming similarity (doc claim verified in §"HDC Distance Metric").
**Notes**: File is 498 LOC with 10 tests, consistent with the spec.

---

## A.14 — Pattern discovery runs every 20 episodes (slowest loop)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Operating Frequency" — pattern discovery runs every 20 episodes, the slowest learning loop. Full frequency table: router=1, gate_thresholds=5, pattern_discovery=20.
**Reality**: `crates/roko-learn/src/runtime_feedback.rs:205-215` defines `UpdateFrequency::default()` with `router_every_n_episodes: 1`, `gate_thresholds_every_n: 5`, `experiments_every_n: 1`, `skill_mining_every_n: 10`, `pattern_discovery_every_n: 20`, `distiller_every_n: 50`. `pattern_discovery_due()` at `:196-198` gates the call site at `:868-870` inside `record_completed_run` — exactly as doc §"Integration with LearningRuntime" describes.
**Notes**: Doc lists only three frequencies (router, gate_thresholds, pattern_discovery). Code also defines `experiments_every_n=1`, `skill_mining_every_n=10`, `distiller_every_n=50`. Doc frequency table is incomplete but not wrong.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 9 |
| PARTIAL | 2 (A.01 schema drift, A.08 weight drift) |
| NOT DONE | 3 (A.04 backpressure, A.07 tiered storage, A.09 DBSCAN) |
| SCAFFOLD | 0 |

The trigram miner, `EpisodeView` trait, `CrossEpisodeConsolidator`, and k-medoids
clustering all match their docs closely. The crash-safe append pipeline in
`EpisodeLogger` is real and wired through `LearningRuntime::record_completed_run`.
The main drift sits in doc 00's design sections: tiered storage
(`EpisodeStorageConfig`, `CompressedEpisodeSummary`) and automatic clustering
(`EpisodeCluster`, incremental DBSCAN) are specified but not implemented. The
`Episode` schema in the doc (18 fields, top-level `hdc_fingerprint`) has
drifted from the shipping 25-field struct that stores fingerprints in `extra`.
Importance scoring weights in doc (`0.30/0.20/0.15/0.20/0.15`) do not match
code (`0.30/0.25/0.20/0.15/0.10`).

## Agent Execution Notes

### A.01 / A.07 — Episode Contract Truth In Advertising

This is mostly a contract-clarification batch, not a storage-architecture batch.

Recommended slice:

1. align the docs with the real `Episode` schema,
2. make `EpisodeLogger::compact` the explicit current retention story,
3. do not imply tiered storage already exists if it does not.

Acceptance criteria:

- later agents can trust the episode schema and retention contract,
- docs stop implying Hot/Warm/Cold storage is live,
- no one is pushed toward implementing storage architecture accidentally inside batch `05`.

### A.09 — Clustering

Default action is to defer DBSCAN-style clustering. The shipped clustering story here is trigram mining plus k-medoids, not the doc's incremental DBSCAN design.
