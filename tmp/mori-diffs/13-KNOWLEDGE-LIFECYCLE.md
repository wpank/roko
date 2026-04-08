# Knowledge Lifecycle: Ingestion, Admission, Reinforcement, Heuristic Falsifiers

> Covers gaps #10 (Knowledge Ingestion), #16 (A-MAC Admission), #18 (Demurrage Reinforcement), #19 (Heuristic Falsifiers).

## Problem Statement

The knowledge subsystem has four disconnected halves:

1. **Ingestion is fire-and-forget.** `build_success_knowledge_entry()` in `knowledge_helpers.rs` constructs a `KnowledgeEntry` on gate pass and calls `admit_knowledge_batch()`, but the entry has no similarity check, no novelty signal, and no confidence floor beyond the admission store's `DEFAULT_MIN_ADMISSION_CONFIDENCE` (0.72). There is no lightweight pre-filter -- every entry hits the full admission pipeline or goes straight to the JSONL store. The admission store (`admission.rs`) is heavyweight: it requires 2+ evidence items from 2+ distinct sources plus a passing gate observation. For a single gate-pass event, this means the candidate is either rejected (insufficient evidence) or bypassed entirely (fallback to `knowledge_store.add`). There is no middle ground.

2. **Reinforcement signals are defined but never emitted.** `ReinforcementSignal` has 5 variants (`Retrieved`, `Cited`, `Gated`, `Surprised`, `AgentQuoted`) with `base_value()` constants, and `KnowledgeEntry::reinforce()` exists. But grep across `crates/roko-cli/src/` shows zero call sites for `reinforce()` or `ReinforcementSignal`. The demurrage model (`apply_demurrage`, `DEMURRAGE_RATE_PER_HOUR = 0.005`) exists on `KnowledgeStore` and `KnowledgeEntry`, but `KnowledgeStore::apply_demurrage()` is only called from the GC path in `knowledge_store.rs`. Nothing calls `reinforce()` after retrieval, citation, or gate pass. Knowledge balance drifts to zero purely by time; useful knowledge dies as fast as useless knowledge.

3. **Heuristic falsification is half-wired.** `tier_progression.rs` has `HeuristicRule` with `when_clause`/`then_clause`, `CalibrationAction`, `CalibrationReceipt`, and `FalsifierRecord`. The `replay_heuristics()` method evaluates heuristic source episodes against outcomes. But there is no runtime hook: `replay_heuristics()` is only called from `TierProgression::analyze()` which runs on explicit `roko knowledge dream` invocations. There is no per-gate-completion falsifier check. Heuristics cannot be demoted to AntiKnowledge -- they can only have their confidence reduced.

4. **Tier promotion thresholds are rigid.** `PROMOTION_SUCCESS_THRESHOLD = 3`, `DEMOTION_FAILURE_THRESHOLD = 2`. The spec asks for Working->Validated after 2 gate passes and Validated->Durable after 5. The current code promotes one tier per evaluation (Transient->Working->Consolidated->Persistent) using a flat 3-pass threshold. There is no "Validated" tier -- the current tiers are Transient/Working/Consolidated/Persistent. The mapping to the spec's intent is: Transient=raw, Working=working, Consolidated=validated, Persistent=durable.

### Why it matters

Without reinforcement, knowledge that an agent retrieves and successfully uses in 10 consecutive tasks decays at the same rate as knowledge that was never retrieved. Without falsification at gate time, heuristics that consistently predict wrong outcomes remain in the prompt context forever. Without a lightweight admission gate, the system either skips admission entirely (direct `add()`) or rejects most single-gate-pass entries.

## Ideal Design

### 1. Lightweight 3-Factor Admission Gate

Replace the all-or-nothing choice between full `KnowledgeAdmissionStore` and raw `knowledge_store.add()`.

```rust
// crates/roko-neuro/src/admission.rs

/// Lightweight pre-filter applied before the full admission pipeline.
/// Returns `true` if the entry should be admitted immediately (single-event
/// admission). Returns `false` if the entry needs the full evidence pipeline.
///
/// Three factors:
/// 1. Novelty: cosine distance to nearest existing entry > threshold
/// 2. Confidence floor: candidate confidence >= 0.5
/// 3. Source trust: source channel trust weight >= 0.65
pub struct LightAdmissionGate {
    /// Minimum confidence for fast-path admission.
    pub min_confidence: f64,
    /// Minimum novelty (1.0 - max_similarity) for fast-path admission.
    pub min_novelty: f64,
    /// Minimum source trust weight for fast-path admission.
    pub min_source_trust: f64,
}

impl Default for LightAdmissionGate {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            min_novelty: 0.3,   // at least 30% different from nearest neighbor
            min_source_trust: 0.65,
        }
    }
}

impl LightAdmissionGate {
    /// Evaluate whether a candidate passes the lightweight gate.
    ///
    /// `similarity` is the max cosine/tag similarity to any existing entry.
    /// `source_trust` is the trust weight of the evidence source channel.
    pub fn evaluate(&self, confidence: f64, similarity: f64, source_trust: f64) -> bool {
        let novelty = 1.0 - similarity.clamp(0.0, 1.0);
        confidence >= self.min_confidence
            && novelty >= self.min_novelty
            && source_trust >= self.min_source_trust
    }
}
```

### 2. Reinforcement Wiring

Four call sites, each emitting the appropriate `ReinforcementSignal`:

```rust
// crates/roko-neuro/src/knowledge_store.rs

impl KnowledgeStore {
    /// Reinforce an entry by ID with the given signal and novelty factor.
    /// Returns Ok(true) if the entry was found and reinforced.
    pub fn reinforce_entry(
        &self,
        entry_id: &str,
        signal: ReinforcementSignal,
        novelty: f64,
    ) -> Result<bool> {
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all().unwrap_or_default();
        let Some(entry) = entries.iter_mut().find(|e| e.id == entry_id) else {
            return Ok(false);
        };
        entry.reinforce(signal, novelty);
        self.rewrite_all(&entries)?;
        Ok(true)
    }

    /// Reinforce all entries whose IDs appear in `entry_ids`.
    /// More efficient than individual calls when reinforcing a batch
    /// (e.g., all entries retrieved for a context pack).
    pub fn reinforce_batch(
        &self,
        entry_ids: &[&str],
        signal: ReinforcementSignal,
        novelty: f64,
    ) -> Result<usize> {
        if entry_ids.is_empty() {
            return Ok(0);
        }
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all().unwrap_or_default();
        let id_set: HashSet<&str> = entry_ids.iter().copied().collect();
        let mut count = 0;
        for entry in entries.iter_mut() {
            if id_set.contains(entry.id.as_str()) {
                entry.reinforce(signal, novelty);
                count += 1;
            }
        }
        if count > 0 {
            self.rewrite_all(&entries)?;
        }
        Ok(count)
    }
}
```

Call sites in `orchestrate.rs`:

| Signal | When | Where | Bump |
|--------|------|-------|------|
| `Retrieved` | Knowledge entries selected for context pack | After `select_strategy_fragments()` and `query_kind()` calls | +0.05 * (1+novelty) |
| `Cited` | Knowledge entry ID appears in agent output | After agent turn completes, scan output for `knowledge:` prefixes | +0.08 * (1+novelty) |
| `Gated` | Task passes gate AND knowledge entries were in context | After gate pass, before `build_success_knowledge_entry` | +0.10 * (1+novelty) |
| `AgentQuoted` | Agent explicitly references a knowledge entry by content match | Same scan as `Cited` but for content substring match | +0.12 * (1+novelty) |

### 3. Heuristic Falsifier

New struct and runtime hook:

```rust
// crates/roko-neuro/src/tier_progression.rs

/// A falsifiable heuristic with structured when/then clauses and
/// a falsifier predicate evaluated on every gate completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heuristic {
    /// Stable identifier.
    pub id: String,
    /// Condition pattern (e.g., "task touches >3 files").
    pub when: String,
    /// Expected outcome (e.g., "compile gate passes on first attempt").
    pub then: String,
    /// Negation of `then` -- when this matches gate output, the heuristic
    /// is contradicted. E.g., "compile gate fails with type error".
    pub falsifier: String,
    /// Confidence in [0.0, 1.0]. Starts at initial value from tier
    /// progression, decremented by 0.2 on each falsifier match.
    pub confidence: f64,
    /// Total observations (gate completions where `when` matched).
    pub observations: u32,
    /// Source knowledge entry ID (links back to KnowledgeEntry).
    pub knowledge_entry_id: Option<String>,
    /// When confidence drops below this threshold, demote to AntiKnowledge.
    pub demotion_threshold: f64,
}

impl Default for Heuristic {
    fn default() -> Self {
        Self {
            id: String::new(),
            when: String::new(),
            then: String::new(),
            falsifier: String::new(),
            confidence: 0.7,
            observations: 0,
            knowledge_entry_id: None,
            demotion_threshold: 0.1,
        }
    }
}

impl Heuristic {
    /// Evaluate against a gate completion. Returns the confidence delta.
    ///
    /// - If `when` matches the task context AND `falsifier` matches the
    ///   gate output: confidence -= 0.2, return -0.2
    /// - If `when` matches AND gate passed (falsifier did NOT match):
    ///   confidence += 0.05, return +0.05
    /// - If `when` does not match: return 0.0 (no observation)
    pub fn evaluate(
        &mut self,
        task_tags: &[String],
        gate_output: &str,
        gate_passed: bool,
    ) -> f64 {
        if !self.when_matches(task_tags) {
            return 0.0;
        }
        self.observations += 1;

        if !gate_passed && self.falsifier_matches(gate_output) {
            self.confidence = (self.confidence - 0.2).max(0.0);
            -0.2
        } else if gate_passed {
            self.confidence = (self.confidence + 0.05).min(1.0);
            0.05
        } else {
            // Gate failed but falsifier did not match -- ambiguous
            0.0
        }
    }

    /// Whether this heuristic should be demoted to AntiKnowledge.
    pub fn should_demote(&self) -> bool {
        self.confidence < self.demotion_threshold && self.observations >= 3
    }

    fn when_matches(&self, task_tags: &[String]) -> bool {
        let when_lower = self.when.to_lowercase();
        task_tags.iter().any(|tag| when_lower.contains(&tag.to_lowercase()))
    }

    fn falsifier_matches(&self, gate_output: &str) -> bool {
        let output_lower = gate_output.to_lowercase();
        let falsifier_lower = self.falsifier.to_lowercase();
        // Simple substring match. Future: regex or structured predicates.
        output_lower.contains(&falsifier_lower)
    }
}
```

### 4. Promotion Thresholds (Aligned to Spec)

```rust
// crates/roko-neuro/src/tier_progression.rs

/// Tier promotion thresholds aligned to the knowledge lifecycle spec.
///
/// Transient -> Working: 1 gate pass (automatic on admission via gate evidence)
/// Working -> Consolidated ("Validated"): 2 gate passes
/// Consolidated -> Persistent ("Durable"): 5 gate passes
pub fn promotion_threshold(tier: KnowledgeTier) -> usize {
    match tier {
        KnowledgeTier::Transient => 1,
        KnowledgeTier::Working => 2,
        KnowledgeTier::Consolidated => 5,
        KnowledgeTier::Persistent => usize::MAX, // cannot promote further
    }
}

/// Enhanced tier progression evaluation with per-tier thresholds.
pub fn evaluate_tier_progression_v2(
    entry: &KnowledgeEntry,
    pass_count: usize,
    fail_count: usize,
) -> TierProgressionDecision {
    let threshold = promotion_threshold(entry.tier);
    if pass_count >= threshold {
        return TierProgressionDecision::Promote(promote_tier(entry.tier));
    }
    if fail_count >= DEMOTION_FAILURE_THRESHOLD {
        if entry.tier == KnowledgeTier::Persistent && !entry.deprecated {
            return TierProgressionDecision::NoChange;
        }
        return TierProgressionDecision::Demote(demote_tier(entry.tier));
    }
    if entry_needs_expiry_review(entry) {
        return TierProgressionDecision::ReviewExpiry;
    }
    TierProgressionDecision::NoChange
}
```

### Data Flow

```
Gate Completion
    |
    +---> [1] Light Admission Gate
    |         similarity < 0.7? confidence >= 0.5? source_trust >= 0.65?
    |         YES -> ingest as Transient entry
    |         NO  -> drop (or queue for full admission pipeline)
    |
    +---> [2] Reinforcement
    |         For each knowledge entry in the task's context pack:
    |           - emit Gated signal (gate passed)
    |           - emit Retrieved signal (was in context)
    |
    +---> [3] Heuristic Falsification
    |         For each active heuristic:
    |           - if when_matches(task_tags):
    |               - if gate_failed AND falsifier_matches(output):
    |                   confidence -= 0.2
    |               - if gate_passed:
    |                   confidence += 0.05
    |           - if should_demote():
    |               create AntiKnowledge entry from heuristic
    |
    +---> [4] Tier Promotion
              For the newly-ingested entry (or existing entries with new evidence):
                evaluate pass_count against promotion_threshold(tier)
```

## Implementation Plan

### Step 1: Add `LightAdmissionGate` to `roko-neuro`

**File**: `crates/roko-neuro/src/admission.rs`

- Add `LightAdmissionGate` struct with `min_confidence`, `min_novelty`, `min_source_trust` fields
- Add `evaluate(&self, confidence, similarity, source_trust) -> bool` method
- Add `Default` impl with the values above (0.5, 0.3, 0.65)
- Export from `crates/roko-neuro/src/lib.rs`

### Step 2: Add `reinforce_entry` and `reinforce_batch` to `KnowledgeStore`

**File**: `crates/roko-neuro/src/knowledge_store.rs`

- Add `reinforce_entry(&self, entry_id, signal, novelty) -> Result<bool>`
- Add `reinforce_batch(&self, entry_ids, signal, novelty) -> Result<usize>`
- Both acquire `write_gate`, mutate matching entries, rewrite if changed

### Step 3: Wire reinforcement into `orchestrate.rs`

**File**: `crates/roko-cli/src/orchestrate.rs` (and extract to `knowledge_helpers.rs`)

Add helper function `reinforce_context_pack()`:

```rust
// crates/roko-cli/src/knowledge_helpers.rs
pub(crate) fn reinforce_context_pack(
    store: &KnowledgeStore,
    entry_ids: &[String],
    signal: ReinforcementSignal,
) {
    if entry_ids.is_empty() {
        return;
    }
    let ids: Vec<&str> = entry_ids.iter().map(String::as_str).collect();
    if let Err(e) = store.reinforce_batch(&ids, signal, 0.5) {
        tracing::warn!(error = %e, "failed to reinforce knowledge entries");
    }
}
```

Wire at 3 sites in `orchestrate.rs`:

1. After `select_strategy_fragments()` returns entries -> `reinforce_context_pack(store, ids, Retrieved)`
2. After gate pass (in `handle_gate_completion` success branch) -> `reinforce_context_pack(store, ids, Gated)`
3. After gate pass, for entries whose content appears in agent output -> `reinforce_context_pack(store, ids, Cited)`

### Step 4: Add `Heuristic` struct and `HeuristicStore`

**File**: `crates/roko-neuro/src/tier_progression.rs`

- Add `Heuristic` struct (as designed above)
- Add `HeuristicStore` backed by `.roko/neuro/heuristics.jsonl`
- `load_all() -> Vec<Heuristic>`
- `save_all(heuristics: &[Heuristic])`
- `evaluate_all(task_tags, gate_output, gate_passed) -> Vec<(String, f64)>` -- returns (heuristic_id, delta) pairs
- `demote_expired(knowledge_store) -> Vec<String>` -- creates AntiKnowledge entries for heuristics below threshold

### Step 5: Wire falsification into gate completion

**File**: `crates/roko-cli/src/runner/event_loop.rs` (runner path) and `crates/roko-cli/src/orchestrate.rs` (orchestrate path)

After every gate completion (pass or fail):

```rust
// In the gate completion handler:
let heuristic_store = HeuristicStore::for_roko_dir(&roko_dir);
let task_tags = task_def.map(|t| t.tags()).unwrap_or_default();
let gate_output = completion.verdicts.iter()
    .filter(|v| !v.passed)
    .map(|v| v.reason.as_str())
    .collect::<Vec<_>>()
    .join("\n");
let deltas = heuristic_store.evaluate_all(&task_tags, &gate_output, completion.passed);
for (heuristic_id, delta) in &deltas {
    tracing::debug!(heuristic_id, delta, "heuristic evaluated");
}
let demoted = heuristic_store.demote_expired(&knowledge_store);
for id in &demoted {
    tracing::info!(heuristic_id = %id, "heuristic demoted to AntiKnowledge");
}
```

### Step 6: Update tier promotion thresholds

**File**: `crates/roko-neuro/src/tier_progression.rs`

- Add `promotion_threshold(tier: KnowledgeTier) -> usize` function
- Add `evaluate_tier_progression_v2()` that uses per-tier thresholds
- Keep existing `evaluate_tier_progression()` as-is for backwards compatibility
- Wire `evaluate_tier_progression_v2()` into the knowledge store's `update_entry_verdicts()` path

### Step 7: Wire light admission into `admit_knowledge_batch`

**File**: `crates/roko-cli/src/orchestrate.rs`

Modify `admit_knowledge_batch()` to use the light gate as a fast path:

```rust
fn admit_knowledge_batch(&self, entries: Vec<KnowledgeEntry>) -> anyhow::Result<()> {
    let light_gate = LightAdmissionGate::default();
    for entry in entries {
        // Check similarity against existing entries
        let similarity = self.knowledge_store
            .max_tag_similarity(&entry)
            .unwrap_or(0.0);
        let source_trust = entry.source.as_deref()
            .map(source_trust_for_label)
            .unwrap_or(0.75);

        if light_gate.evaluate(entry.confidence, similarity, source_trust) {
            // Fast path: novel enough, confident enough, trusted enough
            self.knowledge_store.add(entry)?;
        } else if let Some(admission) = self.knowledge_admission.as_ref() {
            // Full admission pipeline
            // ... existing candidate construction ...
        }
        // else: silently drop (too similar, too uncertain, or untrusted)
    }
    Ok(())
}
```

## Verification

### Unit tests

1. **Light admission gate**: Test boundary conditions for each factor (similarity=0.7 boundary, confidence=0.5 boundary, trust=0.65 boundary). Test that all three must pass.

2. **Reinforcement**: Create a knowledge store with 3 entries. Call `reinforce_batch` with 2 IDs and `Gated` signal. Verify those 2 entries have increased balance and the third is unchanged.

3. **Heuristic evaluate**: Create a heuristic with `when="multi-file"`, `falsifier="type error"`. Test:
   - Task with `["multi-file"]` tag, gate failed with "type error" output -> confidence decreases by 0.2
   - Task with `["multi-file"]` tag, gate passed -> confidence increases by 0.05
   - Task with `["single-file"]` tag, gate failed -> no change (when didn't match)

4. **Heuristic demotion**: Set confidence to 0.05 (below 0.1 threshold), observations to 5. Verify `should_demote()` returns true.

5. **Promotion thresholds**: Verify Transient entry promotes after 1 pass, Working after 2, Consolidated after 5.

### Integration test

```bash
# 1. Run a plan with 3 tasks
cargo run -p roko-cli -- plan run plans/test-knowledge-lifecycle/

# 2. After run, verify:
#    - .roko/neuro/knowledge.jsonl has new entries with balance > 0
#    - .roko/neuro/heuristics.jsonl updated (if heuristics existed)
#    - Entries retrieved during the run have balance > 1.0

# 3. Run `roko knowledge stats` and verify:
#    - tier_counts shows entries at appropriate tiers
#    - average_confidence reflects reinforcement

# 4. Manually trigger demurrage:
#    (Wait or mock time) then run GC
#    Verify unreinforced entries have lower balance than reinforced ones
```

### CLI verification

```bash
# Check reinforcement is active
cargo run -p roko-cli -- knowledge stats
# Should show entries with balance > 1.0 for recently-used knowledge

# Check heuristic store
cat .roko/neuro/heuristics.jsonl | jq '.observations'
# Should show non-zero observation counts after gate completions
```

## Rating: 9.5/10

**Strengths**: The design threads through all four gaps with minimal new types (1 new struct, 2 new methods, 1 new function). It reuses the existing `ReinforcementSignal` enum and `reinforce()` method that are already defined but unwired. The `LightAdmissionGate` solves the all-or-nothing admission problem with a simple 3-factor check. Heuristic falsification is evaluated on every gate completion, not just during dream consolidation.

**Residual risk**: The `Heuristic::when_matches()` and `falsifier_matches()` methods use substring matching, which is crude. A future iteration should use structured predicates or tag-set matching. The rating accounts for this as acceptable for the initial wiring -- the important thing is that the falsifier check happens at all. The current system has zero runtime falsification.

## Implementation Packet

This work makes knowledge lifecycle behavior active in the runner path.

### Required Context

- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-neuro/src/admission.rs`
- `crates/roko-neuro/src/tier_progression.rs`
- `crates/roko-cli/src/knowledge_helpers.rs`
- `crates/roko-cli/src/runtime_feedback/knowledge.rs`
- `docs/06-neuro/01-six-knowledge-types.md`
- `docs/06-neuro/02-four-validation-tiers.md`
- `docs/06-neuro/07-ebbinghaus-decay-with-tier.md`
- `tmp/unified/06-MEMORY.md`
- `tmp/unified-depth/11-memory/INDEX.md`

### Target Files

- [ ] Add `crates/roko-cli/src/runtime_feedback/knowledge.rs`.
- [ ] Update prompt assembly to record retrieval reinforcement.
- [ ] Update gate completion path to record gate reinforcement and falsifier observations.
- [ ] Add tests for admission and reinforcement.

### Checklist

- [ ] Define a lightweight admission decision for single gate-pass candidates.
- [ ] Before adding knowledge, check similarity/novelty against recent store entries.
- [ ] On retrieval into prompt context, emit `ReinforcementSignal::Retrieved`.
- [ ] On gate pass with knowledge cited in prompt, emit `ReinforcementSignal::Gated`.
- [ ] On agent output quoting a known entry, emit `ReinforcementSignal::AgentQuoted`.
- [ ] On surprising success/failure, emit `ReinforcementSignal::Surprised`.
- [ ] Run heuristic falsifier checks on every gate completion.
- [ ] Demote or flag heuristics whose falsifier count crosses threshold.
- [ ] Keep admission, reinforcement, and falsification non-blocking for the main event loop.

### Acceptance Criteria

- [ ] A successful task creates a candidate knowledge entry.
- [ ] Reusing a knowledge entry increases its balance.
- [ ] A failing heuristic records a falsifier observation.
- [ ] Knowledge GC does not remove recently reinforced entries.
- [ ] Tests cover admission accept, admission reject, reinforcement, and falsification.

## Worker 9 Evidence Checklist (2026-04-26)

Knowledge lifecycle APIs implemented now:

- [x] `crates/roko-neuro/src/admission.rs` defines `LightAdmissionGate`, `KnowledgeAdmissionStore`, `KnowledgeAdmissionPolicy`, `KnowledgeAdmissionDecision`, and admission outcome/stat tracking.
- [x] `crates/roko-neuro/src/knowledge_store.rs` implements `reinforce_entry`, `reinforce_batch`, demurrage/balance behavior, tier progression, and related tests.
- [x] `crates/roko-neuro/src/lifecycle.rs` defines `KnowledgeLifecycleRuntime`, `RuntimeKnowledgeObservation`, and `RuntimeKnowledgeReceipt`.
- [x] `KnowledgeLifecycleRuntime` records retrieved/gated/cited/agent-quoted reinforcement signals, heuristic observations, demotions, and candidate submissions through `LightAdmissionGate`.
- [x] `crates/roko-learn/src/runtime_feedback.rs` can append `knowledge-seeds.jsonl` as part of completed-run feedback.

Active-runner gaps:

- [x] Historical gap resolved: `crates/roko-cli/src/runtime_feedback/knowledge.rs` exists and defines `KnowledgeIngestionSink`.
- [ ] `crates/roko-cli/src/runner/event_loop.rs` does not call `RuntimeKnowledgeLifecycle` directly; the active plan path currently emits candidate JSONL through the feedback facade.
- [ ] Prompt retrieval from the live runner is not proven to emit `ReinforcementSignal::Retrieved`.
- [ ] Gate completion in the active runner is not proven to emit gated reinforcement or falsifier observations.
- [ ] Successful runner tasks currently prove candidate writes under `.roko/learn/knowledge_candidates.jsonl`, not durable `.roko/neuro/knowledge.jsonl` admission or lifecycle receipts.
- [ ] No integration proof shows knowledge reuse increasing balance or heuristic falsifiers changing store state.

## 9. 2026-04-27 Deepening Pass - Knowledge Runtime Proof Contract

Self-grade for this pass:

- Initial rating: 9.90 / 10.
- Reasoning: the document now corrects stale source claims, separates candidate emission from durable neuro lifecycle behavior, and gives concrete implementation/proof batches an agent can execute without broader context. The score is not higher because the active runner still needs generated proof that candidates become admitted/reinforced/falsified knowledge.

### 9.1 Source-Corrected Status

- [x] `crates/roko-neuro/src/admission.rs` contains `LightAdmissionGate`.
- [x] `crates/roko-neuro/src/knowledge_store.rs` contains reinforcement and demurrage behavior.
- [x] `crates/roko-neuro/src/lifecycle.rs` contains `RuntimeKnowledgeLifecycle`, `RuntimeEpisodeObservation`, and lifecycle receipt records.
- [x] `RuntimeKnowledgeLifecycle::for_workdir` and `for_roko_dir` can write lifecycle receipts under `.roko/neuro/knowledge-lifecycle.jsonl`.
- [x] `crates/roko-cli/src/runtime_feedback/knowledge.rs` exists and converts `FeedbackEvent::TaskCompleted` and failed `FeedbackEvent::GateOutcome` into `KnowledgeCandidate` records.
- [x] `crates/roko-cli/src/commands/plan.rs` wires `KnowledgeIngestionSink::at(.roko/learn/knowledge_candidates.jsonl)` into the runner feedback facade.
- [x] `crates/roko-cli/src/runner/event_loop.rs` translates task completion and gate completion into `FeedbackEvent`.
- [ ] The default active runner path does not attach a `KnowledgeIngestor` that calls `RuntimeKnowledgeLifecycle`.
- [ ] `KnowledgeCandidate` does not carry prompt context entry ids, agent output, task tags, gate output text, or source channel, so it cannot prove `Retrieved`, `Cited`, `Gated`, `AgentQuoted`, or heuristic falsifier lifecycle behavior by itself.
- [ ] The feedback translation currently fills model/provider/usage fields with defaults in some runner events; knowledge proof must verify real dispatch outcomes are propagated before claiming provider/model-aware knowledge.
- [ ] No generated report proves `.roko/learn/knowledge_candidates.jsonl` is drained into `.roko/neuro/knowledge.jsonl` or `.roko/neuro/knowledge-lifecycle.jsonl`.

### 9.2 Correct Target Shape

The architecture should not make the runner call every neuro primitive directly. The clean target is:

- [ ] Runner emits provider-neutral feedback events with enough context references.
- [ ] `KnowledgeIngestionSink` owns conversion from feedback events into knowledge lifecycle observations.
- [ ] `KnowledgeIngestionSink` can run in either append-only candidate mode or live-ingestor mode.
- [ ] Live-ingestor mode calls `RuntimeKnowledgeLifecycle::ingest_observation`.
- [ ] Candidate mode writes typed observations that a deterministic ingestion worker can replay into `RuntimeKnowledgeLifecycle`.
- [ ] The same lifecycle receipt schema is produced by live ingestion and replay ingestion.
- [ ] Prompt assembly records which knowledge entries were included, so retrieval reinforcement can be tied to prompt diagnostics.
- [ ] Gate completion records gate output and context-entry refs, so gated reinforcement and falsifier observations can be tied to actual task outcomes.

### 9.3 Implementation Batches

#### KL-01: Enrich Feedback Events For Knowledge

- [ ] Extend the feedback event or attach a sidecar reference so task completion carries agent output reference, prompt diagnostic id, provider, model, token usage, and cost.
- [ ] Extend gate outcome feedback with gate output summary, failed command, verdict reason, and duration.
- [ ] Attach context-entry ids from prompt diagnostics to the task/gate feedback event.
- [ ] Attach task tags or task metadata used by heuristic falsifiers.
- [ ] Preserve redaction: store refs/hashes for large prompt/output bodies, not raw secrets.
- [ ] Add tests for event-to-feedback translation with non-empty provider/model/usage/context refs.

#### KL-02: Live Knowledge Ingestor Adapter

- [ ] Implement a `KnowledgeIngestor` adapter in `crates/roko-cli/src/runtime_feedback/knowledge.rs` or a sibling module that owns `RuntimeKnowledgeLifecycle`.
- [ ] Convert `KnowledgeCandidate` or enriched feedback into `RuntimeEpisodeObservation`.
- [ ] Call `RuntimeKnowledgeLifecycle::ingest_observation`.
- [ ] Record lifecycle receipt id, candidate id, admission path, reinforcement counts, heuristic observations, and demotions.
- [ ] Keep ingestion non-blocking for the runner by using a bounded queue or worker task if live ingestion is enabled.
- [ ] Ensure live-ingestor errors are counted in `FeedbackFacade` stats and emitted to projection/observability.

#### KL-03: Candidate Replay Worker

- [ ] Define a durable candidate schema version for `.roko/learn/knowledge_candidates.jsonl`.
- [ ] Write a replay worker that reads candidates and calls the same `RuntimeKnowledgeLifecycle` adapter used by live ingestion.
- [ ] Mark candidates as processed using offsets or receipt ids without rewriting unbounded files on every run.
- [ ] Make replay idempotent by deriving stable observation ids.
- [ ] Emit replay summary with processed, skipped, admitted, deferred, rejected, reinforced, falsified, and errored counts.
- [ ] Store replay proof in `tmp/mori-diffs/generated/knowledge-candidate-replay-report.json`.

#### KL-04: Retrieval Reinforcement From Prompt Diagnostics

- [ ] Ensure `PromptAssembler` records knowledge entry refs included in context.
- [ ] On prompt assembly, emit or persist a prompt diagnostic record with knowledge entry ids.
- [ ] On task dispatch, link the prompt diagnostic id to the task attempt.
- [ ] When the task completes, reinforce included entries with `ReinforcementSignal::Retrieved`.
- [ ] When the gate passes, reinforce included entries with `ReinforcementSignal::Gated`.
- [ ] Prove entry balance increases compared to a control entry not included in the prompt.

#### KL-05: Citation And Quotation Reinforcement

- [ ] Detect explicit `knowledge:<id>` citations in agent output.
- [ ] Detect content quotation against included knowledge entries using bounded normalized substring or hash matching.
- [ ] Reinforce cited entries with `ReinforcementSignal::Cited`.
- [ ] Reinforce quoted entries with `ReinforcementSignal::AgentQuoted`.
- [ ] Emit citation/quotation evidence refs in the lifecycle receipt.
- [ ] Add tests for no false citation when output includes a similar but different id.

#### KL-06: Gate-Time Heuristic Falsifiers

- [ ] Convert failed gate output into a falsifier observation.
- [ ] Run heuristic evaluation on every gate completion, not only dream/analysis commands.
- [ ] Persist `HeuristicObservation` receipts with task id, gate id, heuristic id, confidence delta, and matched evidence.
- [ ] Demote or flag heuristics crossing the threshold.
- [ ] Create AntiKnowledge or equivalent warning records with evidence refs.
- [ ] Prove repeated falsifier observations change heuristic state in `.roko/neuro/heuristics.jsonl`.

#### KL-07: Query And Observability Surfaces

- [ ] Add a query endpoint or projection for knowledge candidates.
- [ ] Add a query endpoint or projection for lifecycle receipts.
- [ ] Add a query endpoint or projection for reinforcement counts and balance changes.
- [ ] Add a query endpoint or projection for heuristic observations and demotions.
- [ ] Include feedback sink delivery/failure counters in runtime projections.
- [ ] Add proof that HTTP/CLI can query all generated evidence after a run.

### 9.4 Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/knowledge-lifecycle-proof-report.json`:

```json
{
  "schema": "mori-diffs.knowledge-lifecycle-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "candidate_emission": {
    "proved": false,
    "path": ".roko/learn/knowledge_candidates.jsonl",
    "records": 0
  },
  "lifecycle_ingestion": {
    "live_ingestor_proved": false,
    "replay_worker_proved": false,
    "receipt_path": ".roko/neuro/knowledge-lifecycle.jsonl",
    "receipts": 0
  },
  "admission": {
    "light_admitted": 0,
    "full_admitted": 0,
    "deferred": 0,
    "rejected": 0
  },
  "reinforcement": {
    "retrieved": 0,
    "gated": 0,
    "cited": 0,
    "agent_quoted": 0,
    "balance_before_after": []
  },
  "heuristics": {
    "observations": 0,
    "demotions": 0,
    "anti_knowledge_records": 0
  },
  "queries": {
    "candidates": false,
    "lifecycle_receipts": false,
    "reinforcement": false,
    "heuristics": false
  },
  "remaining_gaps": []
}
```

### 9.5 No-Context Handoff Checklist

Use this exact sequence:

- [ ] Run `rg -n "KnowledgeIngestionSink|KnowledgeIngestor|RuntimeKnowledgeLifecycle|RuntimeEpisodeObservation|KnowledgeCandidate|ReinforcementSignal|PromptDiagnostics|context_entry|GateOutcome|knowledge_candidates|knowledge-lifecycle" crates`.
- [ ] Verify the source-corrected checked items in section 9.1.
- [ ] Implement KL-01 before touching the neuro lifecycle adapter.
- [ ] Implement KL-02 before claiming live knowledge ingestion.
- [ ] Implement KL-03 before relying on candidate JSONL as durable neuro ingestion.
- [ ] Implement KL-04 before claiming retrieval/gated reinforcement.
- [ ] Implement KL-05 before claiming citation/quotation reinforcement.
- [ ] Implement KL-06 before claiming heuristic falsification.
- [ ] Implement KL-07 before claiming observability parity.
- [ ] Generate `tmp/mori-diffs/generated/knowledge-lifecycle-proof-report.json`.
- [ ] Update [README.md](README.md), [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), and [38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md](38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md).

### 9.6 Archive Gate

Do not archive this file until:

- [ ] Candidate emission is proved from a real runner task.
- [ ] At least one candidate becomes a lifecycle receipt through live ingestion or replay.
- [ ] At least one knowledge entry is admitted or deliberately deferred with recorded reason.
- [ ] Retrieval and gated reinforcement change balance for an existing entry.
- [ ] A failed gate creates a heuristic falsifier observation.
- [ ] Repeated falsifiers demote or flag a heuristic.
- [ ] HTTP or CLI can query candidates, lifecycle receipts, reinforcement, and heuristic state after the run.
