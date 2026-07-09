# Cognitive Layer Audit: Neuro, Dreams, Daimon, Pheromones

Knowledge store, dream consolidation, affect engine, stigmergic signals — the "brain" of roko. 60% good, 40% overengineered dead weight.

## The Problem

The cognitive layer has 4 subsystems across 3 crates (~15,700 LOC). Two are genuinely useful (neuro knowledge store, dream consolidation). Two are overengineered and should be simplified or deleted (daimon PAD model, pheromones). The UNIFIED-IMPLEMENTATION-PLAN explicitly calls for deleting pheromones and replacing daimon with a simple failure tracker.

---

## 1. Status Summary

| Component | LOC | Status | Verdict |
|---|---|---|---|
| Neuro knowledge store | 4,047 | ACTIVE — queried at dispatch, written from episodes | KEEP |
| Episode distillation | 950 | ACTIVE — async LLM-driven extraction | KEEP, unify into ModelCallService |
| Tier progression | 2,322 | ACTIVE — entries promoted/demoted | KEEP |
| Dream consolidation | ~2K | ACTIVE — triggered after plans | KEEP, simplify |
| Daimon PAD model | 40K+ | SEMI-ACTIVE — loaded but PAD model unused | SIMPLIFY → FailureTracker |
| Pheromones | 68K | DEAD — created but never observed | DELETE |
| Custody chain | ~200 | DEAD — CLI inspection only | IGNORE |

---

## 2. Neuro Knowledge Store (KEEP)

**What it is:** Durable append-only JSONL store at `.roko/neuro/knowledge.jsonl` with 4-factor retrieval:

```
score = w_recency * Ebbinghaus_decay
      + w_importance * Reflexion_quality
      + w_relevance * cosine(query, entry)
      + w_emotional * PAD_cosine(mood, entry_affect)
```

**5-tier system:**
| Tier | Half-life | Threshold |
|---|---|---|
| Transient | 1 day | Confidence < 0.5 |
| Validated | 7 days | 0.5 ≤ confidence < 0.9 |
| Durable | 30 days | Confidence > 0.9, 3+ validations |
| Frozen | ∞ | User-archived |
| Death | — | Below 1% of initial weight (prunable) |

**How data gets in:**
1. Task success → `admit_knowledge_entry()` in orchestrate.rs:10241
2. Async distillation → `spawn_episode_distillation()` → Claude API call → parse JSON → admit batch
3. Dream consolidation → cluster insights → staged → promoted to store

**How data gets out:**
1. `build_knowledge_routing_advice()` → model routing bias (orchestrate.rs:14198)
2. `query_anti_knowledge_patterns()` → anti-pattern injection into Layer 7 (orchestrate.rs:14689)
3. `knowledge_routing_boost()` → ±30% confidence adjustment on CascadeRouter scores
4. `apply_neuro_gate_hints()` → adaptive threshold bias (startup only)

**What works well:** The store design is solid — append-only JSONL, tier progression, 4-factor retrieval, confidence decay.

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

**Anti-Pattern #1 violation:** Distiller calls Claude API directly via `ClaudeDistillationBackend::new(api_key, model)` — reads `ANTHROPIC_API_KEY` from env, bypasses provider system, no cost tracking, no episode recording for the distillation call itself.

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

**What's built (40K+ LOC in roko-daimon):**
- PAD emotional model with 3 temporal layers
- Somatic markers (KdTree for experience retrieval)
- Retrieval weight learning (4-factor weights)
- Contagion tracking (emotional spread between agents)
- Behavioral state thresholds (adjust gate strictness)
- Depotentiation on dream consolidation

**What's actually used:**
- State loaded per plan: `daimon_state_path()`
- Confidence queried: affects gate thresholds slightly
- Somatic hash stamped on knowledge entries

**What's NOT used:**
- PAD model not driving any decisions (just flags)
- Somatic markers (KdTree) never queried at runtime
- Retrieval weight learning disconnected
- Contagion tracking unused
- Behavioral modulation unused

**40K LOC for a confidence flag.** The UNIFIED-IMPLEMENTATION-PLAN says:
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
Decay: intensity * 2^(-(elapsed / half_life))
Half-lives: 60min to 48hrs depending on kind
```

**68K LOC** in `roko-orchestrator/src/coordination.rs`.

**How they're "used":**
1. **Created** (orchestrate.rs:6140-6143): Gate failures deposit Threat/Anomaly pheromones
2. **Collected** (orchestrate.rs:5405): `active_pheromone_chunks()` gathers non-evaporated pheromones
3. **Injected** (orchestrate.rs:14691): Added to prompt context as Layer 3c

**Why they're dead:**
- Confirmation counter stuck at 0 — subsequent tasks never confirm/deny pheromones
- Agents don't understand pheromone signals — no instructions tell them what to do with `[Threat] intensity=0.85`
- Overlaps with gate feedback — Threat/Anomaly pheromones are just noise vs explicit error messages
- No observation loop — pheromones created but never read back meaningfully

**UNIFIED-IMPLEMENTATION-PLAN says:**
> "Remove pheromones entirely. Replace with `Vec<String>` of active warnings (gate failures, system issues, etc.)"

---

## 7. Live vs Dead Summary

| Component | Live Callers | Dead Callers | Action |
|---|---|---|---|
| Neuro store query | 0 | orchestrate.rs | Wire into PromptAssemblyService |
| Neuro store write | 0 | orchestrate.rs | Wire into FeedbackService |
| Distillation | 0 | orchestrate.rs | Wire into FeedbackService + ModelCallService |
| Dream trigger | 0 | orchestrate.rs | Wire into WorkflowEngine post-plan hook |
| Daimon state | 0 | orchestrate.rs | Delete, replace with FailureTracker |
| Pheromones | 0 | orchestrate.rs | Delete entirely |
| Custody CLI | CLI only | — | Low priority, ignore |

---

## 8. Anti-Patterns In This Subsystem

| Anti-Pattern | Where |
|---|---|
| **#1 Shell out to Claude** | Distillation reads ANTHROPIC_API_KEY directly |
| **#10 God file** | `coordination.rs` 68K LOC, `roko-daimon/lib.rs` 40K LOC |
| **#6 Feedback as afterthought** | Knowledge store only written from dead path |
| Overengineering | 108K LOC (daimon + pheromones) for effectively zero runtime impact |

---

## 9. What The Unified Plan Preserves vs Deletes

**KEEP:**
- Neuro knowledge store (4-factor retrieval, tier system)
- Episode distillation (via ModelCallService)
- Dream consolidation (simplified trigger)
- Knowledge injection into prompts (via PromptAssemblyService)

**SIMPLIFY:**
- Daimon → FailureTracker (3 fields instead of 40K LOC)
- 18 feedback hooks → 6 (episodes, efficiency, routing, knowledge, thresholds, playbooks)
- Section effectiveness → aggregate only (drop per-episode tracking)

**DELETE:**
- Pheromones (68K LOC)
- PAD emotional model
- Somatic markers (KdTree)
- Contagion tracking
- VCG payments in prompt budgeting
- HDC fingerprints per episode

**Net reduction:** ~110K LOC deleted, ~5K LOC replacement code.
