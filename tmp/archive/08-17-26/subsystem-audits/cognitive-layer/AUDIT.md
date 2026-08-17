# Cognitive Layer Audit: Neuro, Dreams, Daimon, Pheromones

Knowledge store, dream consolidation, affect engine, stigmergic signals — the "brain" of roko. 60% good, 40% overengineered dead weight.

### Architecture Runner Status (2026-04-28)
**Foundation services provide cognitive integration points:**
- `FeedbackService` (P1C) enables all callers to feed episodes, routing, knowledge
- `PromptAssemblyService` (P1B) provides hook for neuro/episode/playbook context injection
- `RuntimeProjection` (P3C) enables cognitive state queries
- **Remaining**: daimon simplification (Phase 6), pheromone deletion (Phase 6), knowledge-informed CascadeRouter

## The Problem

The cognitive layer has 4 subsystems across 3 crates (~36,500 LOC). Two are genuinely useful (neuro knowledge store, dream consolidation). Two are overengineered and should be simplified or deleted (daimon PAD model, pheromones). The UNIFIED-IMPLEMENTATION-PLAN explicitly calls for deleting pheromones and replacing daimon with a simple failure tracker.

---

## 1. Status Summary

| Component | LOC | Status | Verdict |
|---|---|---|---|
| Neuro knowledge store | 4,047 | ACTIVE — queried at dispatch, written from episodes | KEEP |
| Episode distillation | 950 | ACTIVE — async LLM-driven extraction | KEEP, unify into ModelCallService |
| Tier progression | 2,322 | ACTIVE — entries promoted/demoted | KEEP |
| Dream consolidation | ~13,600 | ACTIVE — triggered after plans | KEEP, simplify |
| Daimon PAD model | 7,222 | SEMI-ACTIVE — PAD model, somatic markers, and fatigue tracking all wired and called at dispatch | SIMPLIFY → FailureTracker |
| Pheromones | ~2,000 | PARTIALLY LIVE — deposited on gate results and conductor health; injected into prompts; confirmation counter stuck at 0 | DELETE |
| Custody chain | ~326 | DEAD — in roko-agent/src/safety/provenance.rs, CLI inspection only | IGNORE |

---

## 2. Neuro Knowledge Store (KEEP)

**What it is:** Durable append-only JSONL store at `.roko/neuro/knowledge.jsonl` with multiplicative retrieval scoring:

```
total_score = keyword_score * effective_confidence * recency_factor * emotional_boost
            + hdc_similarity  (zero when hdc feature disabled or entry has no vector)
```

The `RetrievalWeights` additive formula (recency + importance + relevance + emotional) exists in `roko-daimon/src/lib.rs` as a spec-aligned type but is **not** used by the actual store query path; the store uses its own multiplicative formula above.

**4-tier system:**
| Tier | Lifetime multiplier | Promotion threshold |
|---|---|---|
| Transient | 0.1x | 2+ confirmations → Working |
| Working | 0.5x | 3+ distinct contexts → Consolidated |
| Consolidated | 1.0x (base half-life) | Long-term durable |
| Persistent | 5.0x | Highly durable; requires `deprecated=true` to demote |

`frozen=true` is a flag on any-tier entry that excludes it from hot queries. The "Death" stage is a pruning threshold: entries whose recency factor falls below 1% of initial weight are eligible for GC (constant `DEATH_THRESHOLD = 0.01`).

**How data gets in:**
1. Task success → `admit_knowledge_batch()` in orchestrate.rs:10498
2. Async distillation → `spawn_episode_distillation()` → `ClaudeAgent` (not raw API — but still reads `ANTHROPIC_API_KEY` directly from env, bypassing provider config) → parse JSON → `store.add()`
3. Dream consolidation → cluster insights → staged in `StagingBuffer` → promoted to store

**How data gets out:**
1. `build_knowledge_routing_advice()` → model routing bias (orchestrate.rs:14198)
2. `query_anti_knowledge_patterns()` → anti-pattern injection into system prompt (orchestrate.rs:14693)
3. `knowledge_routing_boost()` → score adjustment on CascadeRouter candidates
4. `apply_neuro_gate_hints()` → adaptive threshold bias (startup only, orchestrate.rs:7627)

**What works well:** The store design is solid — append-only JSONL, 4-tier progression with multipliers, multiplicative retrieval, confirmation-adjusted decay.

**What's missing:** Knowledge only consulted from orchestrate.rs (dead). Live paths never query the store.

---

## 3. Episode Distillation (KEEP, REFACTOR)

**Flow:**
```
Episode → .roko/episodes.jsonl
       → install_episode_distillation_hook() spawns async task
       → Distiller::distill(&[Episode]) → Claude API call
       → Parse JSON → Vec<KnowledgeEntry>
       → admit_knowledge_batch() → knowledge store
```

**Anti-Pattern #1 violation:** `ClaudeDistillationBackend` wraps `ClaudeAgent` (not a raw HTTP call), but `spawn_episode_distillation()` in `episode_completion.rs:25` still reads `ANTHROPIC_API_KEY` directly from the environment — bypassing the provider config system, with no cost tracking and no episode recording for the distillation call itself.

**Should use:** `ModelCallService::complete()` with `caller: ModelCallCaller::neuro_distillation`.

---

## 4. Dream Consolidation (KEEP, SIMPLIFY)

**How it triggers:**
1. After plan completion: `maybe_auto_dream()` checks episode count vs `min_episodes_for_dream`
2. On conductor critical patterns: `maybe_coordination_dream()`
3. Manual: `roko knowledge dream run`

**What it does:**
1. Load episodes since last dream
2. Cluster by pattern/theme
3. For each cluster: invoke review agent → generate `InsightRecord`
4. Stage insights in `.roko/dreams/`
5. Promote to knowledge store via `admit_knowledge_batch()`

**What works:** Consolidation generates useful insights and feeds them to the knowledge store.

**What doesn't:**
- No cron/scheduler — dreams only happen on manual trigger or plan completion
- Dream knowledge not back-fed to running tasks (only available to next plan)
- Dream agent created via `create_agent_for_model()` — should use ModelCallService

---

## 5. Daimon Affect Engine (REPLACE WITH FailureTracker)

**What it is:** A 3-layer temporal affect model (PAD: Pleasure-Arousal-Dominance):

```rust
AlmaLayers {
    emotion: PadVector,        // Fast layer, tau=0.1
    mood: PadVector,           // Medium, tau=0.5
    temperament: PadVector,    // Slow baseline, tau=0.9
}
effective_affect = 0.5*emotion + 0.3*mood + 0.2*temperament
```

**What's built (7,222 LOC in roko-daimon):**
- PAD emotional model with 3 temporal layers (`AlmaLayers`, `AffectState`, `DaimonState`)
- Somatic markers (KdTree-backed `SomaticLandscape` for experience retrieval)
- `FatigueDetector` / `CrateConfidence` for failure-streak detection
- Contagion functions (`contagion()`, `contagion_susceptibility()`) defined in roko-core
- Behavioral state thresholds that adjust gate strictness
- Depotentiation on dream consolidation (`apply_dream_depotentiation()`)

**What's actually used (verified in orchestrate.rs):**
- State loaded per plan: `DaimonState::load_or_new(daimon_state_path(workdir))`
- `daimon.appraise()` called on task outcomes, gate results, queue waits, blocks, time pressure, and dream outcomes
- `daimon.query_somatic()` called at dispatch time to get somatic signal for each task
- `daimon.modulate_with_strategy()` adjusts `DispatchParams` before each agent run
- `daimon.record_somatic_outcome()` records task outcomes to somatic landscape
- `daimon.record_crate_success/failure()` updates per-crate confidence
- `daimon.apply_dream_depotentiation()` called after plan completion
- Emotional tag stamped on conductor signals and episodes
- PAD metrics emitted (pleasure, arousal, dominance, behavioral_state)

**What's NOT used:**
- `RetrievalWeights` online learning (defined but not wired to any feedback loop)
- `contagion()` functions not called from orchestrate.rs (defined in roko-core but never invoked at plan runtime)

**7,222 LOC for a useful affect engine** — but less than the "40K+" claim. The UNIFIED-IMPLEMENTATION-PLAN says:
> "Drop: daimon tool filtering, PAD model. Replace with simple rule: if consecutive_failures >= 3, strip network/git tools."

Replace with:
```rust
struct FailureTracker {
    consecutive_failures: u32,
    last_error_pattern: Option<String>,
}
impl FailureTracker {
    fn should_restrict_tools(&self) -> bool {
        self.consecutive_failures >= 3
    }
}
```

---

## 6. Pheromones (DELETE)

**What they are:** Stigmergic signals with exponential decay:

```
7 kinds: Threat, Opportunity, Wisdom, Alpha, Pattern, Anomaly, Consensus
Decay: intensity * 0.5^(age / half_life)
Half-lives: per-kind defaults (e.g. Threat ~1hr, Wisdom ~48hrs)
```

**~2,000 LOC** across `roko-orchestrator/src/coordination.rs` (1,991 lines total; ~180 pheromone-specific lines) plus usage in orchestrate.rs, roko-compose, roko-std, and roko-runtime. The "68K LOC" claim was wrong.

**How they're "used":**
1. **Created on gate results** (orchestrate.rs:16155-16180): Gate failures deposit `Threat` pheromones; gate successes deposit `Opportunity` pheromones; 3+ consecutive failures on the same gate also deposit a `Pattern` pheromone
2. **Created on conductor health** (orchestrate.rs:6131-6145): `Degraded` health deposits `Anomaly` pheromone; `Critical` deposits `Threat`
3. **Collected** (orchestrate.rs:5405): `active_pheromone_chunks()` gathers non-evaporated pheromones
4. **Injected** (orchestrate.rs:14691): Added to prompt context via `pheromone_chunks`

**Why they're still problematic:**
- Confirmation counter stuck at 0 — subsequent tasks never confirm/deny pheromones
- Agents don't understand pheromone signals — no instructions tell them what to do with `[Threat] intensity=0.85`
- Overlaps with gate feedback — Threat/Anomaly pheromones duplicate information already in explicit error messages
- No observation loop — pheromones created but never read back meaningfully

**UNIFIED-IMPLEMENTATION-PLAN says:**
> "Remove pheromones entirely. Replace with `Vec<String>` of active warnings (gate failures, system issues, etc.)"

---

## 7. Live vs Dead Summary

"Live" = called from the currently-executing plan runner path. "Dead" = only called from orchestrate.rs which is itself live, but the downstream wiring into service abstractions is not done.

| Component | Called from orchestrate.rs | Notes | Action |
|---|---|---|---|
| Neuro store query (`query_hits`, `query_anti_knowledge_patterns`) | YES | Anti-patterns and routing advice wired | Wire into PromptAssemblyService abstraction |
| Neuro store write (`admit_knowledge_batch`) | YES | Written on task success | Wire into FeedbackService abstraction |
| Distillation (`spawn_episode_distillation`) | YES | Spawned async; bypasses provider config | Wire into ModelCallService |
| Dream trigger (`maybe_auto_dream`) | YES | Fires on plan completion | Add cron trigger for background runs |
| Daimon state (`appraise`, `query_somatic`, `modulate_with_strategy`) | YES | Extensively used at dispatch | Simplify to FailureTracker per unified plan |
| Pheromones (`pheromone_field.push`, `active_pheromone_chunks`) | YES | Deposited and injected; confirmation counter stuck at 0 | Delete entirely |
| Custody CLI (`roko knowledge custody`) | CLI only | Not wired to runtime | Low priority, ignore |

---

## 8. Anti-Patterns In This Subsystem

| Anti-Pattern | Where |
|---|---|
| **#1 Shell out to Claude** | Distillation reads `ANTHROPIC_API_KEY` directly from env (episode_completion.rs:25) — bypasses provider config |
| **#6 Feedback as afterthought** | Knowledge store only written from dead path (live wiring paths never active) |
| Overengineering | Daimon has extensive wiring but `RetrievalWeights` learning loop and contagion tracking are disconnected |

---

## 9. What The Unified Plan Preserves vs Deletes

**KEEP:**
- Neuro knowledge store (4-factor retrieval, tier system)
- Episode distillation (via ModelCallService)
- Dream consolidation (simplified trigger)
- Knowledge injection into prompts (via PromptAssemblyService)

**SIMPLIFY:**
- Daimon → FailureTracker (3 fields instead of 7,222 LOC; `FatigueDetector` in `phase2_stubs.rs` is already a step in this direction)
- 18 feedback hooks → 6 (episodes, efficiency, routing, knowledge, thresholds, playbooks)
- Section effectiveness → aggregate only (drop per-episode tracking)

**DELETE:**
- Pheromones (~2K LOC spread across orchestrator/compose/runtime)
- PAD emotional model
- Somatic markers (KdTree)
- Contagion tracking (defined in roko-core, not called at runtime)
- VCG payments in prompt budgeting
- HDC fingerprints per episode

**Net reduction:** ~22K LOC deleted (daimon + pheromones), ~5K LOC replacement code.

---

## Sources

Key source files verified for this audit:

| File | LOC | What was checked |
|---|---|---|
| `crates/roko-neuro/src/lib.rs` | 1,592 | `KnowledgeTier` enum (Transient/Working/Consolidated/Persistent), `KnowledgeKind`, `KnowledgeEntry` struct |
| `crates/roko-neuro/src/knowledge_store.rs` | 4,047 | Actual query scoring formula (`keyword * effective_confidence * recency * emotional_boost + hdc`), `score_entry_for_query()`, `DEATH_THRESHOLD` |
| `crates/roko-neuro/src/distiller.rs` | 950 | `ClaudeDistillationBackend` wraps `ClaudeAgent`; `DEFAULT_MODEL = "claude-haiku-3-5"` |
| `crates/roko-neuro/src/episode_completion.rs` | 51 | `spawn_episode_distillation()` reads `ANTHROPIC_API_KEY` directly from env |
| `crates/roko-neuro/src/tier_progression.rs` | 2,322 | `InsightRecord`, tier promotion constants |
| `crates/roko-daimon/src/lib.rs` | 3,759 | `AlmaLayers`, `AffectState`, `DaimonState`, `SomaticLandscape`, `RetrievalWeights`, `FatigueDetector` (in phase2_stubs.rs) |
| `crates/roko-daimon/src/phase2_stubs.rs` | 1,394 | `FatigueDetector`, `FatigueState`, `CrateConfidence` |
| `crates/roko-dreams/src/runner.rs` | 1,480 | `DreamRunner`, `maybe_auto_dream()`, uses `create_agent_for_model()` |
| `crates/roko-dreams/src/cycle.rs` | 3,489 | `DreamCycleReport`, `StagingBuffer` |
| `crates/roko-orchestrator/src/coordination.rs` | 1,991 | `Pheromone`, `PheromoneKind` (7 variants), pheromone decay formula |
| `crates/roko-cli/src/orchestrate.rs` | 21,577 | Verified: `admit_knowledge_batch()` at 10498, `apply_neuro_gate_hints()` at 7627, `knowledge_routing_boost()` at 14138, `build_knowledge_routing_advice()` at 14198, `query_anti_knowledge_patterns()` at 14693, `daimon.query_somatic()` at 14437, pheromone deposits at 6140 and 16155-16180, `DaimonState::load_or_new()` at 3952 |
| `crates/roko-agent/src/safety/provenance.rs` | 326 | `Custody` struct — custody chain is here, not in roko-neuro |
