# IMPL-04: Knowledge and stigmergy

Implements PRD-05 (Knowledge layer and privacy-preserving publication).

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates and ~177K LOC. It builds agents that build themselves: read PRDs, generate plans, execute tasks via Claude, validate with gates, persist results.

The knowledge subsystem has most pieces built but not connected. Episode clustering, resonance detection, somatic markers, the durable knowledge store, dream cycles, and HDC fingerprinting all exist as library code. The gap is wiring: none of these subsystems feed their outputs into the runtime loop or into each other.

This plan closes that gap across six phases, from wiring existing code through to on-chain knowledge publication.

---

## Crate map

| Crate | Path | Role in this plan |
|---|---|---|
| roko-primitives | `crates/roko-primitives/` | 10,240-bit `HdcVector` with bind/bundle/permute/similarity |
| roko-learn | `crates/roko-learn/` | Episode logger, HDC clustering (k-medoids), HDC fingerprinting, resonant patterns, playbook store |
| roko-neuro | `crates/roko-neuro/` | `KnowledgeStore` (JSONL-backed), `KnowledgeEntry`, tier progression, context assembly |
| roko-daimon | `crates/roko-daimon/` | `SomaticLandscape`, `SomaticOracleContext`, somatic TA integration |
| roko-dreams | `crates/roko-dreams/` | Dream cycle orchestration, hypnagogia, imagination, staging buffer |
| roko-chain | `crates/roko-chain/` | Chain client, witness engine, marketplace, ISFR |
| roko-cli | `crates/roko-cli/` | Orchestrator (`orchestrate.rs`) -- the runtime harness |
| roko-runtime | `crates/roko-runtime/` | Heartbeat attention auction, process supervisor |
| roko-compose | `crates/roko-compose/` | Prompt composition with `AttentionBidder` variants |

---

## Phase 1: Wire existing HDC subsystems

Status: built, not connected. Every struct and function below compiles and has unit tests. The work is calling them from the orchestration loop.

### Task 1.1: Wire episode clustering after batch threshold

**Read first:**
- `crates/roko-learn/src/hdc_clustering.rs` -- `k_medoids()` function, `KMedoidsConfig`, `ClusterResult`, `HdcCluster`
- `crates/roko-learn/src/hdc_fingerprint.rs` -- `fingerprint_episode()`, `encode()`, `decode()`
- `crates/roko-learn/src/episode_logger.rs` -- `Episode` struct, `EpisodeLogger::all_episodes()`
- `crates/roko-cli/src/orchestrate.rs` -- lines 100-120 (imports), find where episodes are logged after gate verdicts

**What to do:**
1. In `crates/roko-cli/src/orchestrate.rs`, locate the code path that calls `episode_logger.append()` after a task completes (search for `append_episode` or `log_episode`).
2. After the episode append, increment a counter (add a field `episode_count_since_cluster: usize` to the orchestration state struct, or use `AtomicUsize`).
3. When the counter reaches 50, trigger clustering:
   ```rust
   use roko_learn::hdc_clustering::{k_medoids, KMedoidsConfig};
   use roko_learn::hdc_fingerprint::fingerprint_episode;

   // Collect fingerprints from recent episodes
   let episodes = episode_logger.all_episodes()?;
   let vectors: Vec<HdcVector> = episodes.iter()
       .map(|ep| fingerprint_episode(&ep.prompt_summary, &ep.outcome_summary))
       .collect();

   let config = KMedoidsConfig { k: (vectors.len() / 10).max(2).min(8), max_iterations: 100 };
   let result = k_medoids(&vectors, &config);
   ```
4. Write the cluster result to `.roko/learn/clusters.json` using `serde_json`. Create the file path from `RokoLayout`.
5. Reset the counter to 0.
6. Log a tracing event: `tracing::info!(clusters = result.clusters.len(), iterations = result.iterations, "episode clustering complete")`.

**File to modify:** `crates/roko-cli/src/orchestrate.rs`
**New file (optional):** `crates/roko-cli/src/clustering_ext.rs` if the logic exceeds 80 lines

**Test:**
- Unit test: create 30 mock episodes with 3 distinct prompt patterns. Call clustering. Assert 3 clusters recovered.
- Integration: run `cargo run -p roko-cli -- plan run` on a plan with 50+ tasks. After completion, verify `.roko/learn/clusters.json` exists and contains non-empty cluster data.

**Acceptance:**
- [ ] Episode counter increments after each episode append
- [ ] Clustering triggers at threshold 50
- [ ] Cluster result persisted to `.roko/learn/clusters.json`
- [ ] Tracing event emitted

---

### Task 1.2: Wire resonance detection after clustering

**Read first:**
- `crates/roko-learn/src/resonant_patterns.rs` -- `ResonantPattern`, `lotka_volterra_step()`, `price_equation()`, `fitness_variance()`, `cull_extinct()`
- The cluster result from Task 1.1

**What to do:**
1. After clustering completes (Task 1.1), convert each cluster medoid into a `ResonantPattern`:
   ```rust
   use roko_learn::resonant_patterns::{ResonantPattern, lotka_volterra_step, cull_extinct};

   let mut patterns: Vec<ResonantPattern> = result.clusters.iter().enumerate()
       .map(|(i, cluster)| {
           let fitness = compute_cluster_fitness(cluster, &episodes);
           ResonantPattern::new(i as u64, cluster.medoid, fitness)
               .with_population(cluster.members.len() as f64, 100.0, 0.1)
       })
       .collect();
   ```
2. Implement `compute_cluster_fitness()`: average gate pass rate across member episodes.
3. Run one Lotka-Volterra step to update populations.
4. Cull extinct patterns.
5. Persist surviving patterns to `.roko/learn/resonant-patterns.json`.
6. On subsequent clustering runs, load existing patterns and merge: match new clusters to existing patterns by genome similarity (> 0.7), update fitness, add new patterns for unmatched clusters.

**File to modify:** same as Task 1.1 or the new `clustering_ext.rs`

**Test:**
- Unit test: create 5 patterns with varied fitness. Run 10 Lotka-Volterra steps. Assert high-fitness patterns grew, low-fitness patterns shrank or went extinct.
- Integration: after two clustering rounds, verify `.roko/learn/resonant-patterns.json` contains merged pattern history.

**Acceptance:**
- [ ] Cluster medoids converted to resonant patterns with correct fitness scores
- [ ] Lotka-Volterra dynamics applied per clustering round
- [ ] Extinct patterns culled
- [ ] Pattern history persists across clustering rounds

---

### Task 1.3: Wire somatic markers from episode outcomes

**Read first:**
- `crates/roko-daimon/src/somatic_ta.rs` -- `SomaticOracleContext`, `somatic_confidence_bias()`, `apply_somatic_confidence_bias()`, `SomaticRetrieval`
- `crates/roko-daimon/src/lib.rs` -- `SomaticLandscape`, `SomaticSignal`, `StrategyCoordinates`, `record_outcome()`
- `crates/roko-cli/src/orchestrate.rs` -- find the `DaimonState` usage (search for `daimon` or `affect_engine`)

**What to do:**
1. Locate where the orchestrator holds the `DaimonState` (or constructs one). The somatic landscape lives inside it.
2. After each episode is logged with a gate verdict, record the outcome in the somatic landscape:
   ```rust
   let coords = StrategyCoordinates::from_task_domain(&task.domain, &task.tier);
   let valence = if verdict.passed { 0.8 } else { -0.6 };
   let intensity = verdict.confidence.unwrap_or(0.5);
   landscape.record_outcome(coords, valence, intensity, episode_hash, now);
   ```
3. If `StrategyCoordinates::from_task_domain` does not exist, implement it: hash the domain + tier into the 3D coordinate space using `HdcVector::from_seed` projected onto 3 floats.
4. Persist the somatic landscape periodically (every 20 episodes) to `.roko/state/somatic-landscape.json`.

**File to modify:** `crates/roko-cli/src/orchestrate.rs`, possibly `crates/roko-daimon/src/lib.rs`

**Test:**
- Unit test: record 5 positive and 5 negative outcomes at different coordinates. Query back. Assert positive regions return positive valence, negative regions return negative valence.
- Integration: after plan execution, verify `.roko/state/somatic-landscape.json` contains entries.

**Acceptance:**
- [ ] Gate verdicts feed somatic landscape with correct valence/intensity
- [ ] Strategy coordinates derived from task domain + complexity tier
- [ ] Landscape persisted to disk
- [ ] Querying the landscape at a previously-recorded region returns the expected signal polarity

---

### Task 1.4: Wire neuro store queries into context assembly

**Read first:**
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeStore`, `query()` method, `QueryResult`
- `crates/roko-neuro/src/context.rs` -- `ContextSource::KnowledgeEntry`, `TaskInput`
- `crates/roko-runtime/src/heartbeat_attention.rs` -- `SubsystemId::Neuro`, the neuro bidder test at line ~1857
- `crates/roko-compose/src/` -- `AttentionBidder` trait, `PromptSection`, `PromptComposer`
- `crates/roko-cli/src/orchestrate.rs` -- search for `AttentionBidder` or `context_bidder` or `NeuroAttention`

**What to do:**
1. Check if a `NeuroContextBidder` (or equivalent) already exists in the compose or runtime crates. The heartbeat_attention module has a `SubsystemId::Neuro` and test stubs at line 1857.
2. If the bidder exists but is not called, wire it into the context assembly pipeline in orchestrate.rs. The bidder should:
   - Query the `KnowledgeStore` with the current task's title + description as the search string
   - Return matching entries as `PromptSection` candidates with `SectionPriority::Knowledge`
   - Participate in the VCG attention auction alongside other bidders
3. If the bidder does not exist, create `NeuroContextBidder` implementing `AttentionBidder`:
   ```rust
   struct NeuroContextBidder {
       store: KnowledgeStore,
   }
   impl AttentionBidder for NeuroContextBidder {
       fn bid(&self, task: &TaskContext) -> Vec<PromptSection> {
           let results = self.store.query(&task.title, 5).unwrap_or_default();
           results.into_iter().map(|entry| {
               PromptSection::new(entry.content.clone())
                   .with_source(format!("neuro:{}", entry.id))
                   .with_priority(SectionPriority::Knowledge)
           }).collect()
       }
   }
   ```
4. Register the bidder in the orchestrator's context assembly phase (search for where bidders are collected into a Vec or registered with the composer).

**File to modify:** `crates/roko-cli/src/orchestrate.rs`, possibly `crates/roko-compose/src/` or `crates/roko-runtime/src/heartbeat_attention.rs`

**Test:**
- Unit test: populate knowledge store with 3 entries. Create a task with a matching title. Assert the bidder returns relevant sections.
- Integration: run a plan where knowledge store has entries from a prior run. Verify the agent prompt includes neuro-sourced context sections.

**Acceptance:**
- [ ] Knowledge store queried at dispatch time with task title/description
- [ ] Matching entries appear in the assembled prompt as knowledge sections
- [ ] Bidder participates in VCG auction (bid value reflects entry confidence)
- [ ] Missing or empty knowledge store produces no sections (no crash)

---

## Phase 2: Episode fingerprinting enhancement

Status: basic fingerprinting works. The current implementation hashes prompt + outcome. This phase adds task description and tool-call sequence encoding for richer similarity.

### Task 2.1: Verify existing fingerprint coverage

**Read first:**
- `crates/roko-learn/src/hdc_fingerprint.rs` -- `fingerprint_episode()`, `EpisodeFingerprintInput`
- `crates/roko-cli/src/orchestrate.rs` -- search for `hdc_fingerprint` or `fingerprint_episode` or `encode_hdc_fingerprint`

**What to do:**
1. Trace the code path from episode creation to fingerprint storage. Confirm every episode gets an HDC fingerprint.
2. If any code path skips fingerprinting (error branch, early return), add the fingerprint computation there.
3. Verify the fingerprint is stored in the `Episode` struct's `hdc_fingerprint` field.
4. If the field exists but is `Option<String>` and sometimes `None`, make the fingerprint computation mandatory.

**File to modify:** `crates/roko-cli/src/orchestrate.rs`

**Test:**
- After running a plan with 10 tasks, load `.roko/episodes.jsonl` and assert every episode has a non-empty `hdc_fingerprint` field.

**Acceptance:**
- [ ] 100% of episodes have a non-None `hdc_fingerprint` after plan execution

---

### Task 2.2: Add task description encoding to fingerprint

**Read first:**
- `crates/roko-learn/src/hdc_fingerprint.rs` -- `EpisodeFingerprintInput` struct
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::from_seed()`, `bind()`, `bundle()`

**What to do:**
1. Extend `EpisodeFingerprintInput` to include a `task_description` field:
   ```rust
   #[derive(Debug, Serialize)]
   struct EpisodeFingerprintInput<'a> {
       prompt: &'a str,
       outcome: &'a str,
       task_description: &'a str,
   }
   ```
2. Update `fingerprint_episode()` to accept the task description:
   ```rust
   pub fn fingerprint_episode(prompt: &str, outcome: &str, task_description: &str) -> HdcVector {
       fingerprint(&EpisodeFingerprintInput { prompt, outcome, task_description })
   }
   ```
3. Update all call sites (search `fingerprint_episode` across the workspace). Pass the task description where available, or `""` where not.

**Files to modify:**
- `crates/roko-learn/src/hdc_fingerprint.rs`
- All call sites (use `grep -rn 'fingerprint_episode' crates/ --include='*.rs'`)

**Test:**
- Two episodes with the same prompt/outcome but different task descriptions produce fingerprints with similarity < 0.95.
- Two episodes with different prompts but the same task description produce fingerprints with similarity > 0.5 (task description contributes partial overlap).

**Acceptance:**
- [ ] `fingerprint_episode` accepts task description parameter
- [ ] All call sites updated
- [ ] Tests pass with `cargo test -p roko-learn`

---

### Task 2.3: Add tool-call sequence encoding

**Read first:**
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::permute()` for sequence encoding, `BundleAccumulator`
- `crates/roko-learn/src/hdc_fingerprint.rs`

**What to do:**
1. Add a function that encodes a sequence of tool names into an HDC vector using permuted binding:
   ```rust
   pub fn encode_tool_sequence(tools: &[&str]) -> HdcVector {
       if tools.is_empty() {
           return HdcVector::zeros();
       }
       let mut acc = BundleAccumulator::new();
       for (i, tool) in tools.iter().enumerate() {
           let tool_vec = HdcVector::from_seed(tool.as_bytes());
           // Permute by position to encode order
           acc.add(&tool_vec.permute(i));
       }
       acc.finish()
   }
   ```
2. Integrate this into the episode fingerprint by bundling the tool-sequence vector with the existing fingerprint:
   ```rust
   pub fn fingerprint_episode_full(
       prompt: &str, outcome: &str, task_description: &str, tools: &[&str]
   ) -> HdcVector {
       let base = fingerprint_episode(prompt, outcome, task_description);
       let tool_vec = encode_tool_sequence(tools);
       HdcVector::bundle(&[&base, &base, &tool_vec]) // base dominates 2:1
   }
   ```
3. Wire into orchestrate.rs where the episode is constructed (the tool list is available from the agent result).

**Files to modify:**
- `crates/roko-learn/src/hdc_fingerprint.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Test:**
- Two episodes that used tools `[read, write, test]` vs `[read, test, write]` produce different fingerprints (order matters via permute).
- Two episodes with the same tools in the same order produce identical tool-sequence vectors.
- The full fingerprint is more similar between episodes that share both content and tool patterns than episodes sharing only one.

**Acceptance:**
- [ ] `encode_tool_sequence` encodes tool order via HDC permutation
- [ ] Full fingerprint includes tool-call contribution
- [ ] Same tools, different order -> different fingerprints (Hamming distance > 0.05)

---

### Task 2.4: Validate fingerprint similarity thresholds

**Read first:**
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::similarity()`
- Tasks 2.1-2.3

**What to do:**
1. Write a benchmark test that generates 100 episodes in 5 task categories (20 per category).
2. Compute pairwise similarities within and across categories.
3. Assert:
   - Intra-category mean similarity > 0.65
   - Inter-category mean similarity < 0.55
   - The gap between intra and inter is > 0.1
4. If the gap is too small, adjust the bundling weights (e.g., give task_description more weight in the bundle).

**File to create:** `crates/roko-learn/tests/fingerprint_quality.rs`

**Test:**
- The benchmark test itself is the acceptance test. Run with `cargo test -p roko-learn -- fingerprint_quality`.

**Acceptance:**
- [ ] Intra-category similarity > 0.65
- [ ] Inter-category similarity < 0.55
- [ ] Gap between intra and inter > 0.1

---

## Phase 3: Geometric privacy (PP-HDC)

Status: not yet built. This phase implements privacy-preserving HDC encoding for knowledge that will eventually be published to the Korai chain.

### Task 3.1: Implement PP-HDC hash-encoding function

**Read first:**
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector`, `HDC_BITS` (10,240), `HDC_BYTES` (1,280)

**What to do:**
1. Create a new file `crates/roko-primitives/src/pp_hdc.rs`.
2. Implement `pp_encode()`:
   ```rust
   use crate::hdc::{HdcVector, HDC_BITS};
   use std::collections::hash_map::DefaultHasher;
   use std::hash::{Hash, Hasher};

   /// Block size for PP-HDC encoding (bits).
   const PP_BLOCK_SIZE: usize = 64;
   const PP_BLOCK_COUNT: usize = HDC_BITS / PP_BLOCK_SIZE; // 160

   /// Privacy-preserving HDC encoding.
   ///
   /// Splits the vector into 160 blocks of 64 bits each. Each block is
   /// hashed through a keyed function, producing a non-invertible projection
   /// that preserves approximate Hamming distance.
   pub fn pp_encode(vector: &HdcVector, key: &[u8]) -> HdcVector {
       let bytes = vector.to_bytes();
       let mut output_bits = [0u64; 160];

       for block_idx in 0..PP_BLOCK_COUNT {
           let block_start = block_idx * 8; // 64 bits = 8 bytes
           let mut hasher = DefaultHasher::new();
           key.hash(&mut hasher);
           block_idx.hash(&mut hasher);
           bytes[block_start..block_start + 8].hash(&mut hasher);
           output_bits[block_idx] = hasher.finish();
       }

       HdcVector::from_raw_bits(output_bits)
   }
   ```
3. Add `from_raw_bits(bits: [u64; 160]) -> Self` constructor to `HdcVector` (currently the `bits` field is private).
4. Export `pp_hdc` from `crates/roko-primitives/src/lib.rs`.

**Files to modify:**
- `crates/roko-primitives/src/pp_hdc.rs` (new)
- `crates/roko-primitives/src/hdc.rs` (add `from_raw_bits`)
- `crates/roko-primitives/src/lib.rs` (add `pub mod pp_hdc`)

**Test:**
- `pp_encode(v, key) != v` (output differs from input)
- `pp_encode(v, key1) != pp_encode(v, key2)` when keys differ
- `pp_encode(v, key)` is deterministic (same key -> same output)
- Two similar input vectors (Hamming similarity > 0.8) produce PP-encoded vectors with similarity > 0.79 (< 1% distance loss)

**Acceptance:**
- [ ] Non-invertible: no function to recover input from output
- [ ] Deterministic: same input + key -> same output
- [ ] Distance-preserving: similarity loss < 1% for high-similarity pairs

---

### Task 3.2: Implement role unbinding

**Read first:**
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::bind()` (XOR is its own inverse)
- Task 3.1

**What to do:**
1. In `crates/roko-primitives/src/pp_hdc.rs`, add:
   ```rust
   /// Remove a bound role from a vector via XOR unbinding.
   ///
   /// Since bind is XOR (involution), unbinding is the same operation.
   /// This strips agent-identity or sensitive-role information before
   /// publishing.
   pub fn unbind_role(vector: &HdcVector, role_vector: &HdcVector) -> HdcVector {
       vector.bind(role_vector) // XOR is self-inverse
   }
   ```
2. Define standard role vectors as constants or lazy-initialized seeds:
   ```rust
   pub fn agent_identity_role(agent_id: &str) -> HdcVector {
       HdcVector::from_seed(format!("role:agent:{agent_id}").as_bytes())
   }
   ```

**File to modify:** `crates/roko-primitives/src/pp_hdc.rs`

**Test:**
- Bind an agent role into a vector, then unbind it. The result matches the original vector (involution property).
- After unbinding, the vector no longer correlates with the agent-specific role vector.

**Acceptance:**
- [ ] Unbinding removes bound role information
- [ ] Involution property verified: `unbind(bind(v, role), role) == v`

---

### Task 3.3: Implement quality gate and embargo check

**Read first:**
- `crates/roko-neuro/src/lib.rs` -- `KnowledgeTier` enum (Transient, Working, Established, Canonical)
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeEntry` struct, `confidence` field

**What to do:**
1. In `crates/roko-primitives/src/pp_hdc.rs` (or a new module `crates/roko-neuro/src/privacy_gate.rs`), add:
   ```rust
   /// Check whether a knowledge entry passes the quality gate for publication.
   ///
   /// Requirements:
   /// - confidence >= 0.75
   /// - tier >= Working (not Transient)
   pub fn passes_quality_gate(entry: &KnowledgeEntry) -> bool {
       entry.confidence >= 0.75
           && matches!(entry.tier, KnowledgeTier::Working | KnowledgeTier::Established | KnowledgeTier::Canonical)
   }

   /// Check whether a knowledge entry is past its embargo period.
   ///
   /// Time-sensitive knowledge (e.g., chain state, pricing) is delayed
   /// by `embargo_hours` before publication.
   pub fn passes_embargo(entry: &KnowledgeEntry, now: DateTime<Utc>, embargo_hours: u64) -> bool {
       let embargo_duration = chrono::Duration::hours(embargo_hours as i64);
       now - entry.created_at >= embargo_duration
   }
   ```
2. Make embargo duration configurable via `roko.toml` under a `[knowledge.publishing]` section.

**Files to modify:**
- `crates/roko-neuro/src/privacy_gate.rs` (new) or `crates/roko-primitives/src/pp_hdc.rs`
- `crates/roko-core/src/config/schema.rs` -- add publishing config

**Test:**
- Entry with confidence 0.6 and tier Transient fails quality gate.
- Entry with confidence 0.8 and tier Working passes quality gate.
- Entry created 1 hour ago with 24-hour embargo fails embargo check.
- Entry created 25 hours ago with 24-hour embargo passes embargo check.

**Acceptance:**
- [ ] Quality gate enforces confidence >= 0.75 and tier >= Working
- [ ] Embargo check enforces time delay
- [ ] Both are configurable

---

### Task 3.4: Validate PP-HDC distance preservation

**Read first:**
- Tasks 3.1 through 3.3
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::similarity()`

**What to do:**
1. Write a statistical test in `crates/roko-primitives/tests/pp_hdc_quality.rs`:
   ```rust
   // Generate 200 random vector pairs with known similarities
   // Apply PP-HDC encoding to both
   // Compare similarity before and after encoding
   // Assert: mean absolute similarity difference < 0.01
   ```
2. Also verify non-invertibility: given `pp_encode(v, key)` and `key`, there is no function that recovers `v`. (This is a design assertion, not a computational test -- verify by code review that the hash function is one-way.)

**File to create:** `crates/roko-primitives/tests/pp_hdc_quality.rs`

**Test:**
- Mean absolute similarity difference < 0.01 across 200 pairs
- Max absolute similarity difference < 0.05

**Acceptance:**
- [ ] PP-HDC preserves > 99% of similarity accuracy
- [ ] Statistical test passes with n=200 pairs

---

## Phase 4: Knowledge publishing pipeline

Status: not yet built. This phase creates the pipeline that takes quality-gated, privacy-encoded knowledge entries and publishes them.

### Task 4.1: Define KnowledgePublisher

**Read first:**
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeStore`, `KnowledgeEntry`
- `crates/roko-primitives/src/pp_hdc.rs` (from Phase 3)
- `crates/roko-chain/src/witness.rs` -- `ChainWitnessEngine`, `witness_on_chain()`

**What to do:**
1. Create `crates/roko-neuro/src/publisher.rs`.
2. Define the 7-step pipeline:
   ```rust
   pub struct KnowledgePublisher {
       store: KnowledgeStore,
       chain_client: Option<Box<dyn ChainClient>>,
       pp_key: Vec<u8>,
       embargo_hours: u64,
   }

   impl KnowledgePublisher {
       pub fn publish_entry(&self, entry: &KnowledgeEntry) -> Result<PublishResult> {
           // Step 1: Quality gate
           if !passes_quality_gate(entry) { return Ok(PublishResult::Skipped("quality")); }
           // Step 2: Embargo check
           if !passes_embargo(entry, Utc::now(), self.embargo_hours) { return Ok(PublishResult::Embargoed); }
           // Step 3: Compute HDC fingerprint from entry content
           let fp = text_fingerprint(&entry.content);
           // Step 4: Unbind agent role
           let unbound = unbind_role(&fp, &agent_identity_role(&entry.source_agent()));
           // Step 5: PP-HDC encode
           let encoded = pp_encode(&unbound, &self.pp_key);
           // Step 6: Serialize publication record
           let record = PublicationRecord { encoded_fingerprint: encoded, content_hash: entry.content_hash(), ... };
           // Step 7: Submit to chain (or local store if no chain client)
           self.submit(record)
       }
   }
   ```
3. Define `PublishResult` enum: `Published { tx_hash }`, `Skipped { reason }`, `Embargoed`, `Error { msg }`.
4. Define `PublicationRecord` struct.

**File to create:** `crates/roko-neuro/src/publisher.rs`
**File to modify:** `crates/roko-neuro/src/lib.rs` (add `pub mod publisher`)

**Test:**
- Unit test with mock chain client: entry passes all gates -> `Published`.
- Entry with low confidence -> `Skipped`.
- Entry within embargo window -> `Embargoed`.

**Acceptance:**
- [ ] 7-step pipeline implemented
- [ ] Each step can reject or transform the entry
- [ ] Works with both real chain client and mock/None fallback

---

### Task 4.2: Add publishing triggers

**Read first:**
- Task 4.1
- `crates/roko-cli/src/orchestrate.rs` -- task completion handler
- `crates/roko-dreams/src/cycle.rs` -- `DreamCycle::run()` completion

**What to do:**
1. After a successful task (gate passed), scan recent knowledge entries and call `publisher.publish_eligible()`:
   ```rust
   impl KnowledgePublisher {
       pub fn publish_eligible(&self) -> Result<Vec<PublishResult>> {
           let entries = self.store.query_by_tier(KnowledgeTier::Established)?;
           entries.iter().map(|e| self.publish_entry(e)).collect()
       }
   }
   ```
2. After a dream cycle completes (in `cycle.rs`), call `publish_eligible()` for any entries promoted during the cycle.
3. After resonance detection (Task 1.2), call `publish_eligible()` for resonant pattern entries.
4. Make the trigger configurable: `[knowledge.publishing] auto_publish = true` in `roko.toml`.

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-dreams/src/cycle.rs`
- `crates/roko-core/src/config/schema.rs`

**Test:**
- With `auto_publish = true`, verify publishing runs after task completion.
- With `auto_publish = false`, verify no publishing occurs.

**Acceptance:**
- [ ] Publishing triggers after successful task, after dream cycle, after resonance detection
- [ ] Configurable via `roko.toml`
- [ ] Can be disabled

---

## Phase 5: Dream integration

Status: the dream cycle crate (`roko-dreams`) is fully built with cycle orchestration, hypnagogia, imagination, staging, and replay. The gap is triggering dreams from the runtime and feeding outputs back into the knowledge system.

### Task 5.1: Wire sleep pressure accumulation

**Read first:**
- `crates/roko-dreams/src/runner.rs` -- `DreamRunner`, `DreamTrigger`, `DreamSchedulePolicy`
- `crates/roko-dreams/src/phase2/sleep_time.rs` -- `DreamBudgetTracker`, `DreamComputeBudget`
- `crates/roko-cli/src/orchestrate.rs` -- search for `DreamRunner` or `dream`

**What to do:**
1. Add a `sleep_pressure: f64` field to the orchestration state (or to the `DreamRunner` if it does not already have one).
2. After each task dispatch, increment sleep pressure:
   ```rust
   self.sleep_pressure += 1.0;
   ```
3. After each dream cycle completes, reset:
   ```rust
   self.sleep_pressure = 0.0;
   ```
4. Persist sleep pressure in `.roko/state/executor.json` so it survives `--resume`.

**File to modify:** `crates/roko-cli/src/orchestrate.rs`

**Test:**
- After 50 tasks without a dream, sleep pressure == 50.0.
- After a dream cycle, sleep pressure == 0.0.
- Sleep pressure survives a `--resume`.

**Acceptance:**
- [ ] Sleep pressure increments per task dispatch
- [ ] Resets after dream cycle
- [ ] Persisted in executor state

---

### Task 5.2: Wire dream trigger

**Read first:**
- Task 5.1
- `crates/roko-dreams/src/runner.rs` -- `DreamRunner::run_cycle()`, `DreamConfig`

**What to do:**
1. After incrementing sleep pressure, check threshold:
   ```rust
   let threshold = config.dreams.sleep_pressure_threshold.unwrap_or(50.0);
   if self.sleep_pressure >= threshold {
       tracing::info!(pressure = self.sleep_pressure, "sleep pressure exceeded threshold, triggering dream cycle");
       self.run_dream_cycle().await?;
   }
   ```
2. Make the threshold configurable: `[dreams] sleep_pressure_threshold = 50` in `roko.toml`.
3. Add a `roko dream run` CLI command that manually triggers a dream cycle (for debugging).

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-cli/src/main.rs` or equivalent CLI entry point

**Test:**
- After 50 tasks, the dream cycle triggers automatically.
- After 49 tasks, no dream cycle.
- `cargo run -p roko-cli -- dream run` triggers a dream cycle regardless of pressure.

**Acceptance:**
- [ ] Dream cycle triggers when sleep pressure exceeds configurable threshold
- [ ] Manual trigger via CLI command
- [ ] Threshold defaults to 50 if not configured

---

### Task 5.3: Wire dream outputs to neuro store

**Read first:**
- `crates/roko-dreams/src/cycle.rs` -- `DreamCycleReport`, `knowledge_entries_written`, `strategy_hypotheses`
- `crates/roko-neuro/src/tier_progression.rs` -- `TierProgression`, `InsightRecord`
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeStore::ingest()`

**What to do:**
1. After a dream cycle completes, iterate over the report's knowledge entries:
   ```rust
   let report = self.dream_runner.run_cycle().await?;
   for entry in &report.strategy_hypotheses {
       self.knowledge_store.ingest(entry.clone())?;
   }
   ```
2. Promote entries that appeared in multiple dream cycles:
   - Load the dream history from `.roko/dreams/history.jsonl`.
   - If an entry's content_hash appeared in >= 3 dream cycles, promote its tier from Working to Established.
3. Feed dream-produced insights back into the tier progression pipeline.

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-dreams/src/cycle.rs` (if the report does not already include content hashes)

**Test:**
- After a dream cycle, verify new entries in the knowledge store match the dream report's `strategy_hypotheses`.
- After 3 dream cycles with overlapping hypotheses, verify tier promotion occurred.

**Acceptance:**
- [ ] Dream-produced knowledge entries ingested into the durable store
- [ ] Entries promoted after repeated dream confirmation
- [ ] Dream history persisted for cross-cycle analysis

---

### Task 5.4: Wire dream outputs to somatic markers

**Read first:**
- `crates/roko-dreams/src/cycle.rs` -- `regressions_detected` field in `DreamCycleReport`
- `crates/roko-daimon/src/somatic_ta.rs` -- `SomaticOracleContext`
- Task 1.3

**What to do:**
1. After a dream cycle, for each regression detected:
   ```rust
   for regression in &report.regressions_detected {
       let coords = StrategyCoordinates::from_knowledge_entry(regression);
       landscape.record_outcome(coords, -0.7, 0.8, regression_hash, now);
   }
   ```
2. For each successful strategy hypothesis confirmed:
   ```rust
   for hypothesis in &report.strategy_hypotheses {
       if hypothesis.confidence >= 0.8 {
           let coords = StrategyCoordinates::from_knowledge_entry(hypothesis);
           landscape.record_outcome(coords, 0.6, hypothesis.confidence, hyp_hash, now);
       }
   }
   ```

**Files to modify:** `crates/roko-cli/src/orchestrate.rs`

**Test:**
- After a dream cycle with regressions, query the somatic landscape at those coordinates. Assert negative valence.
- After a dream cycle with confirmed hypotheses, assert positive valence at those coordinates.

**Acceptance:**
- [ ] Dream regressions produce negative somatic markers
- [ ] Confirmed hypotheses produce positive somatic markers
- [ ] Markers persist across dream cycles

---

## Phase 6: InsightStore query path (chain integration)

Status: the chain crate has all the primitives (client, witness, marketplace) but no specific InsightStore query endpoint. This phase creates the query path.

### Task 6.1: Implement InsightStore RPC client

**Read first:**
- `crates/roko-chain/src/client.rs` -- `ChainClient` trait
- `crates/roko-chain/src/witness.rs` -- how on-chain queries work
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::similarity()`

**What to do:**
1. Create `crates/roko-chain/src/insight_store.rs`.
2. Define the `InsightStore` trait:
   ```rust
   #[async_trait]
   pub trait InsightStore: Send + Sync {
       /// Query the chain for knowledge entries similar to the given fingerprint.
       async fn query_similar(&self, fingerprint: &HdcVector, top_k: usize) -> Result<Vec<InsightEntry>>;
   }
   ```
3. Define `InsightEntry`:
   ```rust
   pub struct InsightEntry {
       pub content_hash: [u8; 32],
       pub encoded_fingerprint: HdcVector,
       pub similarity: f32,
       pub source_chain_id: String,
       pub block_number: u64,
   }
   ```
4. Implement a mock version for testing and an HTTP/RPC version that calls a Korai chain endpoint.

**Files to create:**
- `crates/roko-chain/src/insight_store.rs`
**Files to modify:**
- `crates/roko-chain/src/lib.rs` (add module + re-export)

**Test:**
- Mock: insert 5 entries, query with a similar fingerprint. Assert top-1 result has highest similarity.
- Integration: requires a running Korai chain node (skip in CI with `#[cfg(feature = "chain-integration")]`).

**Acceptance:**
- [ ] `InsightStore` trait defined with `query_similar`
- [ ] Mock implementation passes unit tests
- [ ] `InsightEntry` struct carries chain provenance metadata

---

### Task 6.2: Implement TTL-based local cache

**Read first:**
- Task 6.1

**What to do:**
1. Wrap the `InsightStore` with a caching layer:
   ```rust
   pub struct CachedInsightStore<S: InsightStore> {
       inner: S,
       cache: Mutex<HashMap<[u8; 32], (Vec<InsightEntry>, Instant)>>,
       ttl: Duration,
   }
   ```
2. Cache key: hash of the query fingerprint (first 32 bytes).
3. On cache hit within TTL, return cached results.
4. On cache miss or expiry, query the inner store and update cache.
5. Add a `cache_size_limit` (default 1000 entries). Evict oldest on overflow.

**File to modify:** `crates/roko-chain/src/insight_store.rs`

**Test:**
- Query twice with same fingerprint within TTL. Assert inner store called once.
- Query after TTL expires. Assert inner store called again.
- Insert 1001 entries. Assert cache size <= 1000.

**Acceptance:**
- [ ] Cache hit avoids RPC call
- [ ] TTL expiry triggers re-fetch
- [ ] Cache size bounded

---

### Task 6.3: Wire InsightStore into context bidding

**Read first:**
- Task 1.4 (NeuroContextBidder)
- Task 6.1 and 6.2

**What to do:**
1. Extend the `NeuroContextBidder` (from Task 1.4) to also query the `InsightStore`:
   ```rust
   // In the bidder:
   let local_results = self.store.query(&task.title, 3)?;
   let chain_results = self.insight_store.query_similar(&task_fingerprint, 3).await?;
   // Merge, deduplicate by content_hash, sort by similarity
   ```
2. Chain-sourced entries get a `ContextSource::InsightStore { chain_id, block }` attribution.
3. The bidder should degrade gracefully: if the InsightStore is unavailable (no chain client configured), skip chain queries and use local results only.

**Files to modify:**
- The bidder module from Task 1.4
- `crates/roko-neuro/src/context.rs` -- add `ContextSource::InsightStore` variant if needed

**Test:**
- With mock InsightStore: assert chain entries appear in prompt sections with correct attribution.
- Without InsightStore (None): assert local-only results returned, no error.

**Acceptance:**
- [ ] Bidder queries both local knowledge store and chain InsightStore
- [ ] Results merged and deduplicated
- [ ] Graceful degradation when chain is unavailable

---

## Phase 7: 7-Layer publishing defense

Status: the publishing pipeline (Phase 4) handles quality gates and embargo checks. This phase adds the full 7-layer defense stack for production-grade knowledge publication.

### Task 7.1: Implement Layer 1 -- Content classifier (PII/secret detection)

**File to create:** `crates/roko-neuro/src/content_classifier.rs` (new file)

**Read first:**
- `crates/roko-neuro/src/publisher.rs` (from Phase 4)
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeEntry` content field

**What to do:**

1. Create `crates/roko-neuro/src/content_classifier.rs`.
2. Define the classifier:

```rust
/// Presidio-style named entity recognition for sensitive content.
/// Scans knowledge entry content for API keys, file paths, PII,
/// and other secrets before publication.
pub struct ContentClassifier {
    patterns: Vec<SensitivePattern>,
}

pub struct SensitivePattern {
    pub name: String,
    pub regex: Regex,
    pub severity: Severity,
    pub action: ClassifierAction,
}

pub enum Severity { Low, Medium, High, Critical }

pub enum ClassifierAction {
    /// Redact the match and continue.
    Redact,
    /// Block publication entirely.
    Block,
    /// Replace with a placeholder.
    Replace(String),
}
```

3. Implement default patterns:
   - API keys: `(?i)(sk-[a-zA-Z0-9]{20,}|AKIA[0-9A-Z]{16}|ghp_[a-zA-Z0-9]{36})` -> `Block`
   - File paths: `(/Users/[a-zA-Z]+/|/home/[a-zA-Z]+/|C:\\Users\\)` -> `Redact`
   - Email addresses: `[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}` -> `Redact`
   - IP addresses: `\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b` -> `Redact`
   - Private keys: `(0x[a-fA-F0-9]{64}|-----BEGIN.*PRIVATE KEY-----)` -> `Block`

4. Implement `ContentClassifier::scan(content: &str) -> Vec<Finding>`.
5. Implement `ContentClassifier::clean(content: &str) -> CleanResult`:
   - Apply all actions (redact, block, replace) and return cleaned content or a block signal.
6. Register in `lib.rs`.

**Test:**
- Content with an AWS access key -> `Block`.
- Content with a home directory path -> redacted to `[PATH]`.
- Content with an email -> redacted to `[EMAIL]`.
- Content with no sensitive data -> unchanged.

- [ ] Pattern-based NER for API keys, paths, PII, private keys
- [ ] `scan()` returns all findings with severity
- [ ] `clean()` applies redaction/block/replace actions
- [ ] Content with secrets is blocked from publication

---

### Task 7.2: Implement Layer 2 -- Knowledge distillation (L0-L3 abstraction ladder)

**File to modify:** `crates/roko-neuro/src/publisher.rs`

**Read first:**
- Task 7.1 output
- `crates/roko-neuro/src/tier_progression.rs` -- `TierProgression`, `KnowledgeTier`

**What to do:**

1. Define the abstraction ladder:

```rust
pub enum AbstractionLevel {
    /// L0: Raw observation (full detail, never published).
    Raw,
    /// L1: Factual summary (key facts, no raw data).
    Factual,
    /// L2: Pattern (generalized insight, no specific instances).
    Pattern,
    /// L3: Principle (universal rule, no context).
    Principle,
}
```

2. Implement `distill(entry: &KnowledgeEntry, target: AbstractionLevel) -> KnowledgeEntry`:
   - L0 -> L1: strip raw data, keep key facts and outcomes.
   - L1 -> L2: generalize across instances, replace specifics with categories.
   - L2 -> L3: extract the universal rule, remove all context.
3. The publisher should distill to at least L1 before publication. L2 is preferred for chain publication.
4. Wire into the publishing pipeline between Layer 1 (content classifier) and Layer 3 (IFC labels).

**Test:**
- L0 entry with raw code diff -> L1 entry with summary of changes.
- L1 entry with specific file paths -> L2 entry with generalized pattern.
- L2 entry with domain context -> L3 entry with universal principle.
- Higher abstraction levels have shorter content.

- [ ] `AbstractionLevel` enum with 4 levels
- [ ] `distill()` transforms entries up the ladder
- [ ] Publisher distills to L1+ before publication
- [ ] Each level removes more specific detail

---

### Task 7.3: Implement Layer 3 -- IFC labels (confidentiality/integrity tags)

**File to create:** `crates/roko-neuro/src/ifc_labels.rs` (new file)

**Read first:**
- FIDO/FIDES data classification model
- Task 7.2 output

**What to do:**

1. Define information flow control labels:

```rust
pub struct IFCLabel {
    pub confidentiality: ConfidentialityLevel,
    pub integrity: IntegrityLevel,
    pub compartments: Vec<String>,
}

pub enum ConfidentialityLevel {
    Public,       // safe for chain publication
    Internal,     // visible to same-organization agents only
    Restricted,   // visible to same-agent-passport only
    Secret,       // never published
}

pub enum IntegrityLevel {
    Verified,     // confirmed by multiple sources
    Attested,     // confirmed by one trusted source
    Unverified,   // no confirmation
    Disputed,     // contradicted by other evidence
}
```

2. Implement `label_entry(entry: &KnowledgeEntry) -> IFCLabel`:
   - Default: `Internal` confidentiality, `Unverified` integrity.
   - If confidence >= 0.9 and confirmation_count >= 3: `Verified`.
   - If content classifier found `Redact`-level findings: `Restricted`.
   - If content classifier found `Block`-level findings: `Secret`.
3. Only `Public` + `Verified|Attested` entries pass to Layer 4.
4. Register in `lib.rs`.

**Test:**
- High-confidence, multi-confirmed entry -> `Public` + `Verified`.
- Entry with redacted PII -> `Restricted` (blocked from publication).
- New entry with no confirmations -> `Internal` + `Unverified` (blocked).

- [ ] `IFCLabel` with confidentiality and integrity dimensions
- [ ] Automatic labeling based on confidence, confirmations, and content scan
- [ ] Only `Public` + `Verified|Attested` entries proceed

---

### Task 7.4: Implement Layer 4 -- Quality gate (confidence >0.75, tier >=Working)

**File to modify:** `crates/roko-neuro/src/publisher.rs`

**Read first:**
- Phase 3 Task 3.3 (existing quality gate)
- Task 7.3 output

**What to do:**

1. This layer already exists from Phase 3 Task 3.3 (`passes_quality_gate`). Verify it enforces:
   - `confidence >= 0.75`
   - `tier >= Working` (not Transient)
2. Add IFC label check: entry must be `Public` confidentiality and `Verified|Attested` integrity.
3. Log entries that fail the quality gate with the reason (which condition failed).

**Test:**
- Entry with confidence 0.8, tier Working, Public/Verified -> passes.
- Entry with confidence 0.6 -> fails (confidence too low).
- Entry with tier Transient -> fails (tier too low).
- Entry with Restricted confidentiality -> fails (not public).

- [ ] Existing quality gate verified and extended with IFC check
- [ ] Failed entries logged with specific failure reason

---

### Task 7.5: Implement Layer 5 -- Temporal embargo

**File to modify:** `crates/roko-neuro/src/publisher.rs`

**Read first:**
- Phase 3 Task 3.3 (existing embargo check)
- `crates/roko-core/src/config/schema.rs` -- configuration

**What to do:**

1. Extend the existing embargo check with domain-specific durations:

```rust
pub struct EmbargoConfig {
    /// Default embargo duration.
    pub default_hours: u64,
    /// Domain-specific overrides.
    pub overrides: HashMap<String, u64>,
}

// Default domain embargoes:
// "trading" -> 24 hours
// "mev" -> 1 hour
// "security" -> 72 hours
// "coding" -> 4 hours
// "research" -> 12 hours
```

2. Implement `embargo_duration(entry: &KnowledgeEntry, config: &EmbargoConfig) -> Duration`:
   - Look up domain in overrides, fall back to default.
3. Wire into the publisher: entries within their embargo window are held back.
4. Make configurable via `roko.toml`:

```toml
[knowledge.publishing.embargo]
default_hours = 24
trading = 24
mev = 1
security = 72
```

**Test:**
- Trading entry created 1 hour ago with 24-hour embargo -> held.
- Trading entry created 25 hours ago -> released.
- Security entry created 48 hours ago with 72-hour embargo -> held.
- Security entry created 73 hours ago -> released.

- [ ] Domain-specific embargo durations
- [ ] Configurable via `roko.toml`
- [ ] Entries within embargo window held back

---

### Task 7.6: Implement Layer 6 -- PP-HDC encoding

**File to modify:** `crates/roko-neuro/src/publisher.rs`

**Read first:**
- Phase 3 Tasks 3.1-3.4 (PP-HDC implementation)

**What to do:**

1. Wire the PP-HDC encoding from Phase 3 into the publishing pipeline:
   - Compute HDC fingerprint of the distilled content
   - Unbind agent identity role (remove agent-specific information)
   - Apply PP-HDC encoding with the configured key
2. Store the encoded fingerprint in the `PublicationRecord`.
3. Verify distance preservation: similarity between the encoded fingerprint and the original should differ by less than 1%.

**Test:**
- Publish an entry. Verify the `PublicationRecord` contains a non-zero encoded fingerprint.
- The encoded fingerprint differs from the raw fingerprint.
- Two similar entries produce encoded fingerprints with similarity >0.99 of their raw similarity.

- [ ] PP-HDC encoding wired into publishing pipeline
- [ ] Agent identity role unbound before encoding
- [ ] Distance preservation verified (<1% loss)

---

### Task 7.7: Implement Layer 7 -- Selective sharing (novelty check)

**File to modify:** `crates/roko-neuro/src/publisher.rs`

**Read first:**
- Tasks 7.1-7.6
- `crates/roko-chain/src/insight_store.rs` -- `InsightStore::query_similar()`

**What to do:**

1. Before publishing, check if the InsightStore already contains a similar entry:

```rust
pub async fn novelty_check(
    store: &dyn InsightStore,
    fingerprint: &HdcVector,
    threshold: f64,  // default 0.95
) -> bool {
    let existing = store.query_similar(fingerprint, 1).await
        .unwrap_or_default();
    if let Some(top) = existing.first() {
        top.similarity < threshold as f32  // novel if below threshold
    } else {
        true  // no existing entries -> novel
    }
}
```

2. If the entry is not novel (similarity > 0.95 with an existing InsightStore entry), skip publication. Log: "entry skipped: duplicate detected (similarity {similarity})".
3. Wire as the final step before chain submission.

**Test:**
- New entry with no similar entries in InsightStore -> published.
- Entry with >0.95 similarity to existing -> skipped.
- InsightStore unavailable -> publish anyway (fail-open for novelty check only).

- [ ] Novelty check queries InsightStore before publishing
- [ ] Duplicate entries (>0.95 similarity) are skipped
- [ ] Fail-open when InsightStore is unavailable

---

### Task 7.8: End-to-end publishing defense test

**File to create:** `crates/roko-neuro/tests/publishing_defense.rs` (new file)

**Read first:**
- Tasks 7.1 through 7.7

**Do:**

1. Create a knowledge entry with:
   - Content containing an email address and a file path
   - Confidence 0.85, tier Working
   - Domain "coding", created 5 hours ago
2. Run through the full 7-layer pipeline:
   - Layer 1: email redacted, path redacted
   - Layer 2: distilled to L1 (factual summary)
   - Layer 3: labeled Public + Attested
   - Layer 4: passes quality gate (confidence 0.85, tier Working)
   - Layer 5: passes embargo (coding = 4 hours, entry is 5 hours old)
   - Layer 6: PP-HDC encoded
   - Layer 7: novelty check passes (no existing similar entries)
3. Assert: final `PublicationRecord` contains cleaned content, PP-HDC fingerprint, correct metadata.
4. Assert: raw email and file path are not present in the published content.

**Test:** `cargo test -p roko-neuro --test publishing_defense`

- [ ] Full 7-layer pipeline processes an entry end-to-end
- [ ] PII stripped, content distilled, labeled, gated, embargoed, encoded, novelty-checked
- [ ] Published content contains no sensitive information
- [ ] All 7 layers verify in sequence

---

## Phase 8: Geometric sharing

### Task 8.1: Implement role unbinding (XOR to remove project/client/date roles)

**File to modify:** `crates/roko-primitives/src/pp_hdc.rs`

**Read first:**
- Phase 3 Task 3.2 (existing `unbind_role` for agent identity)
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::bind()` (XOR involution)

**What to do:**

1. Extend role unbinding to cover 4 role types:

```rust
/// Standard role vectors that may be bound into knowledge fingerprints.
/// All must be unbound before publication.
pub fn agent_identity_role(agent_id: &str) -> HdcVector {
    HdcVector::from_seed(format!("role:agent:{agent_id}").as_bytes())
}

pub fn project_role(project_name: &str) -> HdcVector {
    HdcVector::from_seed(format!("role:project:{project_name}").as_bytes())
}

pub fn client_role(client_id: &str) -> HdcVector {
    HdcVector::from_seed(format!("role:client:{client_id}").as_bytes())
}

pub fn temporal_role(date: &str) -> HdcVector {
    HdcVector::from_seed(format!("role:date:{date}").as_bytes())
}

/// Unbind all standard roles from a vector.
pub fn unbind_all_roles(
    vector: &HdcVector,
    agent_id: &str,
    project: Option<&str>,
    client: Option<&str>,
    date: Option<&str>,
) -> HdcVector {
    let mut result = unbind_role(vector, &agent_identity_role(agent_id));
    if let Some(p) = project {
        result = unbind_role(&result, &project_role(p));
    }
    if let Some(c) = client {
        result = unbind_role(&result, &client_role(c));
    }
    if let Some(d) = date {
        result = unbind_role(&result, &temporal_role(d));
    }
    result
}
```

2. Wire into the publishing pipeline (Layer 6): unbind all available roles before PP-HDC encoding.

**Test:**
- Bind agent + project + date roles into a vector. Unbind all. Result matches the original pre-binding vector (involution property).
- After unbinding agent role, the vector no longer correlates with the agent-specific role vector (similarity < 0.55).
- Unbinding a role that was not bound does not corrupt the vector (XOR with an unrelated vector changes it, but a second XOR with the same vector restores it).

- [ ] 4 role types: agent, project, client, temporal
- [ ] `unbind_all_roles()` strips all bound roles
- [ ] Involution property verified for each role type
- [ ] Wired into publishing Layer 6

---

### Task 8.2: Implement 3-tier on-chain search (bloom -> approximate -> exact)

**File to create:** `crates/roko-chain/src/insight_search.rs` (new file)

**Read first:**
- `crates/roko-chain/src/insight_store.rs` -- `InsightStore` trait
- `crates/roko-chain/src/precompiles/htc.rs` -- HTC precompile
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::similarity()`

**What to do:**

1. Define the 3-tier search:

```rust
pub struct TieredInsightSearch {
    /// Tier 1: Bloom filter for fast negative answers.
    bloom: BloomFilter,
    /// Tier 2: Approximate search via locality-sensitive hashing.
    lsh: LocalitySensitiveHash,
    /// Tier 3: Exact cosine similarity via HTC precompile.
    htc: Box<dyn InsightStore>,
}
```

2. Implement `TieredInsightSearch::search(query: &HdcVector, top_k: usize) -> Vec<InsightEntry>`:
   - Tier 1 (bloom): check if any entries in the query's neighborhood exist. If bloom says no -> return empty (fast negative).
   - Tier 2 (LSH): hash the query vector with 8 hash functions. Find candidate entries sharing >=4 hash buckets. This reduces the search space by ~90%.
   - Tier 3 (exact): compute exact cosine similarity for the candidates via HTC precompile. Return top-k.

3. Implement `BloomFilter` using a simple bit array with k=7 hash functions.
4. Implement `LocalitySensitiveHash` with random hyperplane projections.

**Test:**
- Insert 1000 entries. Search with a vector similar to entry #500. Assert entry #500 in top-5 results.
- Search with a vector that matches no entries. Assert bloom filter returns empty in <1ms.
- Verify LSH reduces candidate set to <10% of total entries.

- [ ] 3-tier search: bloom -> LSH -> exact
- [ ] Bloom filter provides fast negative answers
- [ ] LSH reduces candidate set by ~90%
- [ ] Exact search via HTC precompile for final ranking

---

### Task 8.3: Validate PP-HDC similarity preservation

**File to create:** `crates/roko-primitives/tests/pp_hdc_geometric.rs` (new file)

**Read first:**
- Phase 3 Task 3.4 (existing PP-HDC quality test)
- Tasks 8.1, 8.2

**What to do:**

1. Generate 500 random vector pairs with known similarities.
2. Apply role unbinding (all 4 roles) followed by PP-HDC encoding.
3. Compute similarity before and after encoding.
4. Assert: mean absolute similarity difference < 0.01 (< 1% loss).
5. Assert: maximum absolute similarity difference < 0.05.
6. Assert: rank order preserved -- if `sim(A,B) > sim(A,C)` before encoding, then `sim(encode(A), encode(B)) > sim(encode(A), encode(C))` after encoding.

**Test:** `cargo test -p roko-primitives --test pp_hdc_geometric`

- [ ] Similarity preserved >99% after role unbinding + PP-HDC encoding
- [ ] Rank order preserved for 500 pairs
- [ ] Max deviation < 5%

---

## Verification checklist

Run these commands to verify each phase:

```bash
# Phase 1: Compile and test learn + neuro + daimon changes
cargo test -p roko-learn -- hdc_clustering
cargo test -p roko-learn -- resonant_patterns
cargo test -p roko-daimon -- somatic
cargo test -p roko-neuro -- knowledge_store

# Phase 2: Fingerprint quality
cargo test -p roko-learn -- fingerprint
cargo test -p roko-learn -- fingerprint_quality  # integration test

# Phase 3: PP-HDC
cargo test -p roko-primitives -- pp_hdc
cargo test -p roko-primitives -- pp_hdc_quality  # integration test

# Phase 4: Publishing
cargo test -p roko-neuro -- publisher

# Phase 5: Dreams
cargo test -p roko-dreams -- cycle
cargo test -p roko-cli -- dream  # CLI subcommand test

# Phase 6: InsightStore
cargo test -p roko-chain -- insight_store

# Full workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

## Dependency graph

```
Phase 1 (wire existing) ─┬─> Phase 2 (fingerprint enhance) ─> Phase 3 (PP-HDC)
                          │                                          │
                          │                                          v
                          ├─> Phase 5 (dream integration) ──> Phase 4 (publishing) ──> Phase 6 (InsightStore)
                          │
                          └─> (can run in parallel with Phase 2)
```

Phases 1 and 2 can run in parallel. Phase 3 depends on Phase 2. Phase 4 depends on Phase 3. Phase 5 depends on Phase 1. Phase 6 depends on Phases 4 and 1.4.

## Acceptance criteria (plan-level)

- [ ] Episode clustering runs automatically after 50 episodes
- [ ] Resonance detection finds cross-domain analogies and updates pattern populations
- [ ] Somatic markers populated from failure episodes, queried at gate time
- [ ] Enhanced fingerprints include task description and tool-call sequence
- [ ] PP-HDC preserves > 99% similarity accuracy while being non-invertible
- [ ] Knowledge publishing pipeline works end-to-end (quality gate -> embargo -> encode -> submit)
- [ ] Dream cycle triggers from sleep pressure, promotes knowledge, feeds somatic markers
- [ ] InsightStore queries return relevant chain-sourced entries, cached with TTL
- [ ] All phases pass `cargo clippy --workspace --no-deps -- -D warnings`
- [ ] All phases pass `cargo test --workspace`
