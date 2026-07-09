# A — Knowledge Types, Tiers, Decay (Docs 00, 01, 02, 03, 07, 11)

Parity analysis of `docs/06-neuro/00-vision-and-grimoire-rename.md`,
`docs/06-neuro/01-six-knowledge-types.md`,
`docs/06-neuro/02-four-validation-tiers.md`,
`docs/06-neuro/03-type-half-lives.md`,
`docs/06-neuro/07-ebbinghaus-decay-with-tier.md`,
and `docs/06-neuro/11-antiknowledge-challenge.md`
vs the actual codebase.

---

## A.01 — Dissolution of `roko-golem` crate

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §"Dissolution of roko-golem" — the `roko-golem` umbrella crate has been dissolved. Grimoire placeholder deleted, Daimon moved to `roko-daimon`, Dreams expanded into `roko-dreams`, Chain Witness moved to `roko-chain`, Mortality deleted entirely. `GolemScaffold` and `ScaffoldEngine` deleted.
**Reality**: `ls crates/roko-golem/` returns `No such file or directory`; `ls crates/` shows `roko-daimon`, `roko-dreams`, `roko-neuro` and `roko-chain` but no `roko-golem`. Workspace `Cargo.toml:59-64` lists `crates/roko-neuro`, `crates/roko-dreams`, and `crates/roko-daimon` as workspace members; no `crates/roko-golem` member. `rg 'GrimoireEngine|MortalityEngine|GolemScaffold|ScaffoldEngine' crates/` returns **zero matches**. `rg 'golem::|roko_golem|roko-golem' crates/` returns **zero matches**. Dissolution is physically complete.
**Notes**: Project `CLAUDE.md:85` still lists `roko-golem` as a key crate ("Chain witness, daimon, dreams, grimoire | Phase 2+") and `bardo-primitives` at `:80` (actually `roko-primitives`). This is the stale CLAUDE.md noted in the prompt, not a code-side gap.

---

## A.02 — Grimoire → Neuro rename (type names)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 §"Naming Map" — `GrimoireEngine` → `NeuroStore`, `grimoire.rs` → `knowledge_store.rs`, `GrimoireEntry` → `KnowledgeEntry`, `GrimoireKind` → `KnowledgeKind`.
**Reality**: `crates/roko-neuro/src/lib.rs:349-364` defines `pub trait NeuroStore` with `init`, `query`, `ingest`, `decay`, `gc`. `crates/roko-neuro/src/knowledge_store.rs` (2,006 LOC) is the implementation file. `crates/roko-neuro/src/lib.rs:186-243` defines `pub struct KnowledgeEntry`. `crates/roko-neuro/src/lib.rs:42-59` defines `pub enum KnowledgeKind`. `rg 'GrimoireEntry|GrimoireKind|grimoire' crates/` returns zero hits in live source — all rename sites are clean.

---

## A.03 — `KnowledgeEntry` schema shape and fields

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 §"The KnowledgeEntry Struct" at `:274-317` lists 13 fields: `id`, `kind`, `source`, `content`, `confidence`, `confidence_weight`, `refuted_insight_id`, `refutation_evidence`, `source_episodes`, `tags`, `created_at`, `half_life_days`, `hdc_vector`.
**Reality**: `crates/roko-neuro/src/lib.rs:185-243` defines `KnowledgeEntry` with **18 fields**. Beyond the 13 documented, it also carries `source_model: Option<String>` at `:219-220`, `model_generality: f64` at `:223-224` (default 1.0 via `default_model_generality` at `:20-22`), `tier: KnowledgeTier` at `:232-233` (default `Transient`), `emotional_tag: Option<EmotionalTag>` at `:235-236`, and `emotional_provenance: Option<EmotionalProvenance>` at `:238-239`. Every field has `#[serde(default)]` or a named default function for forward-compat. The struct derive is `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` at `:185` — notably it is *not* `Default`, which the doc's "implicitly inherits from `Default`" line implies.
**Fix sketch**: Add the five extra fields (`source_model`, `model_generality`, `tier`, `emotional_tag`, `emotional_provenance`) to the doc 01 schema block at `:274-317`, or note that they exist but were omitted from the reference snippet for brevity. Drop the "implicitly inherits from `Default`" implication — there is no `Default` derive on `KnowledgeEntry` in current source.

---

## A.04 — `KnowledgeKind` six canonical variants

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §"The Six Types" + §"Reconciliation with the Six-Type Design" — the enum is PRD-native with six variants: `Insight`, `Heuristic`, `Warning`, `CausalLink`, `StrategyFragment`, `AntiKnowledge`. Legacy names (`Fact`, `Procedure`, `Playbook`, `Constraint`) survive only as serde aliases.
**Reality**: `crates/roko-neuro/src/lib.rs:39-59` defines `KnowledgeKind` (derive block at `:40`, `enum KnowledgeKind {` at `:42`, closing brace at `:59`) with exactly six variants: `Insight`, `Heuristic`, `AntiKnowledge`, `Warning`, `CausalLink`, `StrategyFragment`. Serde aliases present: `#[serde(alias = "fact", alias = "Fact")]` on `Insight` at `:44`, `alias = "procedure", alias = "Procedure"` on `Heuristic` at `:47`, `alias = "constraint", alias = "Constraint"` on `Warning` at `:52`, `alias = "playbook", alias = "Playbook"` on `StrategyFragment` at `:57`. `Default` is `Insight` (impl block at `:61-65`). `as_str()` at `:82-92` returns the snake_case labels. Unit test `legacy_knowledge_kind_names_deserialize_to_prd_variants` at `:469-490` verifies all eight legacy name/casing pairs deserialize to the PRD-native enum value.
**Notes**: Doc 01 §"Current Implementation" code block at `:234-255` is **stale** — it lists seven variants including `Fact`, `Procedure`, `Playbook`, `Constraint` as first-class variants (in that block the leading prose at `:234` also claims "seven variants"). The prose at `:259-270` correctly reconciles with code; the displayed code snippet in the doc is what's out of sync.

---

## A.05 — `refutation_warning()` method for AntiKnowledge

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §"The Challenge Mechanism" — `KnowledgeEntry::refutation_warning()` returns `Option<String>`, gated on `kind == AntiKnowledge` and non-empty `refuted_insight_id` + non-empty evidence (falling back to `content` if `refutation_evidence` is None). Format: `"Previous insight {id} was wrong because {evidence}."`
**Reality**: `crates/roko-neuro/src/lib.rs:245-271` matches byte-for-byte. Non-AntiKnowledge returns `None` at `:249-251`. Empty `refuted_insight_id` returns `None` at `:253-256`. Evidence falls back to `self.content` at `:260` and trims trailing punctuation at `:263`. Format string at `:268-270` matches doc claim exactly.

---

## A.06 — `KnowledgeTier` enum and tier multipliers

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §"The Four Tiers" + Doc 07 §"Tier multipliers" — four-variant enum `Transient/Working/Consolidated/Persistent` with multipliers `0.1×, 0.5×, 1.0×, 5.0×`. Default tier is Transient.
**Reality**: `crates/roko-neuro/src/lib.rs:95-108` defines `KnowledgeTier` with exactly four variants; `#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]` at `:96` and `#[default]` on `Transient` at `:100`. Note the return type is `f32`, not `f64`: `KnowledgeTier::multiplier(&self) -> f32` at `:110-121` returns `0.1, 0.5, 1.0, 5.0` for the four tiers. Unit test `knowledge_tier_multiplier_matches_spec` at `:393-399` asserts each value. Default tier test `missing_knowledge_tier_defaults_to_transient` at `:427-450` confirms a missing `tier` field deserializes to `Transient` and `12.0 × 0.1 = 1.2` days effective half-life.

---

## A.07 — `effective_half_life_days()` composition

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 §"Effective Half-Life Composition" — `effective_half_life = tier_multiplier × type_base_half_life`. Doc 02 `:277` still lists "Tier multiplier logic (`effective_half_life = tier_multiplier × type_base_half_life`)" as Missing.
**Reality**: `crates/roko-neuro/src/lib.rs:279-288` defines `pub fn effective_half_life_days(&self) -> f64`. Falls back to `kind.default_half_life_days()` at `:285` when `half_life_days` is non-finite or ≤ 0.0; otherwise uses the stored `half_life_days`. Multiplies by `self.tier.multiplier() as f64` at `:287` (explicit `f32 → f64` cast because `KnowledgeTier::multiplier()` returns `f32`). `recency_factor()` at `knowledge_store.rs:852-860` then calls back into this method via a thin wrapper `fn effective_half_life_days(entry)` at `:862-864`. Unit test `effective_half_life_applies_tier_multiplier` at `:401-425` confirms `20.0 days × 5.0 (Persistent) = 100.0 days`. Doc 02 `:277` "Missing" bullet is **stale**.

---

## A.08 — Half-life constants per knowledge kind

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §"Default Half-Life in Code" lists five constants with exact values:
- `INSIGHT_HALF_LIFE_DAYS = 30.0`
- `HEURISTIC_HALF_LIFE_DAYS = 90.0`
- `WARNING_HALF_LIFE_DAYS = 7.0`
- `CAUSAL_LINK_HALF_LIFE_DAYS = 60.0`
- `STRATEGY_FRAGMENT_HALF_LIFE_DAYS = 14.0`

**Reality**: `crates/roko-neuro/src/lib.rs:28-37` defines all five `pub const` values with exactly those numeric literals:
```rust
pub const INSIGHT_HALF_LIFE_DAYS: f64 = 30.0;
pub const HEURISTIC_HALF_LIFE_DAYS: f64 = 90.0;
pub const WARNING_HALF_LIFE_DAYS: f64 = 7.0;
pub const CAUSAL_LINK_HALF_LIFE_DAYS: f64 = 60.0;
pub const STRATEGY_FRAGMENT_HALF_LIFE_DAYS: f64 = 14.0;
```
`KnowledgeKind::default_half_life_days()` at `:67-79` (const fn) dispatches each variant to the matching constant. A private `const fn default_half_life_days() -> f64` at `:24-26` returns `30.0` and is used both for the struct field default (`#[serde(default = "default_half_life_days")]` at `:229`) and as the fall-through for `AntiKnowledge` at `:74`. Unit test `new_knowledge_kinds_have_expected_defaults` at `:452-466` asserts `Warning → 7.0`, `CausalLink → 60.0`, `StrategyFragment → 14.0`. Unit cast to days is implicit in the constant names and the `num_seconds / 86_400.0` conversion at `knowledge_store.rs:855-857`.
**Notes**: `AntiKnowledge` does not receive a dedicated constant — `KnowledgeKind::default_half_life_days()` at `:74` falls back to the private `default_half_life_days()` (30 days) rather than `f64::INFINITY`. The AntiKnowledge confidence floor (A.13) compensates; AntiKnowledge is exempt from GC via an unconditional skip regardless of decayed weight. Doc 11 `:47` code sample sets `half_life_days: f64::INFINITY` with a comment "never decays (confidence floor 0.3)" — that is idiomatic doc-intent but code never branches on `INFINITY`.

---

## A.09 — `FACT_HALF_LIFE_DAYS` — doc claims 365, code has no constant

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 03 §"Fact: 365 Days" shows a Rust snippet declaring `pub const FACT_HALF_LIFE_DAYS: f64 = 365.0;` at `:181`. Doc 03 §"Implemented" block at `:275-283` and Doc 07 §"Implemented" block at `:228` both claim `FACT_HALF_LIFE_DAYS` exists.
**Reality**: `rg 'FACT_HALF_LIFE_DAYS' crates/` returns **zero matches**. The `Fact` variant no longer exists on `KnowledgeKind` (doc 01 §"Reconciliation" confirms this — legacy `Fact` deserializes to `Insight`). No `Fact`-specific constant survives. The 365-day doc entries are dead references from a prior version of the enum.
**Fix sketch**: Remove `FACT_HALF_LIFE_DAYS` from the code snippet in Doc 03 `:181`. Drop `FACT_HALF_LIFE_DAYS = 365.0` from Doc 03 `:276` and Doc 07 `:229` "Implemented" bullets. Strip the Fact row from the half-life summary table at Doc 01 `:345` and Doc 02 `:43,65,87,112,210` (each tier-specific table lists Fact — all stale).

---

## A.10 — Ebbinghaus decay formula `2^(-age/half_life)`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 §"Base Decay Equation" — `weight(entry, t) = 2^(-age_days / effective_half_life)`, equivalent to `exp(-age_days × ln(2) / effective_half_life)`.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:852-860` defines `fn recency_factor(entry, now) -> f64` which computes `age_days = seconds_since(created_at) / 86_400.0` at `:853-857`, pulls `effective_half_life_days(entry)` at `:858`, and returns `0.5_f64.powf(age / half_life)` at `:859`. That is literally `2^(-age / half_life)` because `0.5^x = 2^-x`. Applied during `decay()` at `:381-399`: `entry.confidence = (entry.confidence.max(0.0) * factor).clamp(0.0, 1.0)` for non-AntiKnowledge. Unit test `decay_drops_below_half_after_two_half_lives` at `:1313-` confirms the math.

---

## A.11 — `DEFAULT_GC_MIN_CONFIDENCE = 0.05` threshold

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 §"GC Threshold" — `DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05`. Doc 02 `:181` repeats the same constant.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:24` declares `pub const DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05;` with doc comment "Default garbage-collection threshold for knowledge entries." Re-exported from `lib.rs:384`. Used at `knowledge_store.rs:1082` in tests via `store.gc(DEFAULT_GC_MIN_CONFIDENCE)`. `KnowledgeStore::gc()` at `:406-421` retains entries where `effective_confidence(entry) >= threshold`, with `threshold = min_confidence.max(0.0)` at `:408`.

---

## A.12 — `CONFIRMATION_BOOST = 1.5` applied to multi-source entries

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 02 `:166-173` and Doc 07 `:132-140` — `pub const CONFIRMATION_BOOST: f64 = 1.5;` applied multiplicatively to confidence, clamped to `[0.0, 1.0]`. Doc 02 `:173` says "An entry with confidence 0.4 that is confirmed once becomes `0.4 × 1.5 = 0.6`. A second confirmation raises it to `0.6 × 1.5 = 0.9`." Doc 07 `:138` says "Applied as: `entry.confidence *= CONFIRMATION_BOOST;` Clamped to [0.0, 1.0]".
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:27-28` declares `const CONFIRMATION_BOOST: f64 = 1.5;` (**private** module constant, not `pub`, contradicting doc 02 and doc 07 which both show it as `pub const`). `fn confirmation_boost(entry)` at `:879-885` returns `1.5` only when `entry.source_episodes.len() >= 2`, otherwise `1.0`. Applied inside `fn effective_confidence()` at `:866-868` as `bounded_confidence(entry) * confirmation_boost(entry) * entry.emotional_consolidation_boost()`. The persisted `entry.confidence` field is **not** mutated on confirmation — the boost is applied lazily at scoring/retrieval time only. This differs from the doc's `entry.confidence *= CONFIRMATION_BOOST` implication.
**Fix sketch**: Either change `CONFIRMATION_BOOST` to `pub const` at `knowledge_store.rs:28` (and re-export from `lib.rs` next to `DEFAULT_GC_MIN_CONFIDENCE` at `:384`), or update Doc 02 `:170`/`:173` and Doc 07 `:136`/`:138` to describe the actual retrieval-time multiplication and the `source_episodes.len() >= 2` gate. The doc's "second confirmation raises it to `0.9`" example cannot happen in current code because the boost is not idempotent-per-confirmation — it is a binary multi-source flag.
**Notes**: Gating on `source_episodes.len() >= 2` rather than on a promotion event adds an implicit requirement beyond what doc 02 §"Confirmation Boost on Confirmation" describes. The boost only applies once an entry has accumulated two or more episode sources. The `KnowledgeConfirmationRecord` at `knowledge_store.rs:42-52` separately records confirmation events (to a `knowledge-confirmations.jsonl` sibling file) that feed the `roko-learn` C-Factor metrics `knowledge_integration_rate` and `convergence_velocity`; this record is *not* the boost pathway.

---

## A.13 — AntiKnowledge confidence floor 0.3 (decay + GC exemption)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 §"Confidence Floor: 0.3" + §"Exemption from Garbage Collection" — AntiKnowledge confidence never drops below 0.3 during decay, and is exempt from GC (the 0.05 threshold does not apply).
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:26` declares `const ANTI_KNOWLEDGE_CONFIDENCE_FLOOR: f64 = 0.3;`. Enforced in `decay()` at `:390-394`:
```rust
entry.confidence = if entry.kind == KnowledgeKind::AntiKnowledge {
    decayed_confidence.max(ANTI_KNOWLEDGE_CONFIDENCE_FLOOR)
} else {
    decayed_confidence
};
```
and in `bounded_confidence()` at `:870-877`. `gc()` at `:406-421` uses filter predicate `entry.kind == AntiKnowledge || effective_confidence(entry) >= threshold` at `:413-416` — AntiKnowledge is retained unconditionally. Unit tests: `decay_preserves_antiknowledge_confidence_floor` at `:1216-` and `gc_preserves_antiknowledge_even_below_threshold` at `:1377-`.

---

## A.14 — Tier promotion/demotion logic (3 successes / 2 failures)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §"Tier Transition Mechanics" at `:119-160` — promotion flow `Transient→Working→Consolidated→Persistent` on success thresholds (1 / 3 / 10); demotion on negative outcomes. Doc 02 `:278` still claims "Tier promotion/demotion logic based on confirmation counts" is Missing.
**Reality**: `crates/roko-neuro/src/tier_progression.rs:219-258` defines `TierProgression::evaluate_promotion(entry, verdicts)` at `:230-236` (returns `Option<KnowledgeTier>`) and `TierProgression::evaluate_tier_progression(entry, verdicts)` at `:240-258` (returns rich `TierProgressionDecision`). Logic at `:244-257`: `successes = verdicts.iter().filter(|v| v.passed).count()`, then `successes >= 3 → Promote(promote_tier(entry.tier))`, `failures >= 2 → Demote(demote_tier(entry.tier))`, else `entry_needs_expiry_review(entry) ? ReviewExpiry : NoChange`. `promote_tier()` at `:855-861` steps `Transient → Working → Consolidated` (stops at Persistent — `Consolidated | Persistent => current`, i.e. no auto-promotion to Persistent). `demote_tier()` at `:863-869` steps `Persistent → Consolidated → Working → Transient` (and `Working | Transient → Transient`). `entry_needs_expiry_review` at `:871-875` flags entries older than `2 × effective_half_life_days`. Unit tests `evaluate_promotion_promotes_on_three_successes` at `:1218-1254` and `evaluate_promotion_demotes_on_two_failures` at `:1256-1291` confirm behavior.
**Notes**: Thresholds are `3 successes` and `2 failures` — simpler than the doc's three-level progression (1/3/10 successes). The doc's "1 successful use → Working" and "10+ uses → Persistent" thresholds are not realized by code; Persistent can only be reached via heuristic `inferred_retention_tier()` at `knowledge_store.rs:769-785` at ingest time (e.g. `StrategyFragment` with `source_count >= 3` infers `Persistent`), not via `evaluate_tier_progression`. Doc 02 `:278` "Missing: Tier promotion/demotion logic based on confirmation counts" is **stale**; the mechanism is implemented, just with different thresholds than the doc describes.

---

## A.15 — Half-speed on-chain demurrage for AntiKnowledge (0.5×)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 §"Half-Speed Demurrage: 0.5×" — on Korai chain, AntiKnowledge pays 0.5× the standard demurrage rate (0.5% annual vs 1%). Doc 11 `:271-295` provides a `compute_demurrage()` function specification.
**Reality**: `rg 'demurrage|compute_demurrage|DEMURRAGE' crates/roko-neuro/ crates/roko-chain/` returns **zero matches** against `demurrage`. The Korai chain demurrage precompile does not exist. The 0.5× multiplier for AntiKnowledge on-chain has no code path. Doc 03 `:287` already lists "Chain-level half-speed demurrage / richer AntiKnowledge longevity semantics" as missing; Doc 11 `:920` confirms: "Missing: Half-speed demurrage (designed above; Korai precompile not implemented)."
**Fix sketch**: Either wire `compute_demurrage()` into a future `roko-chain` precompile with the AntiKnowledge 0.5× branch, or move Doc 11 `:252-297` behind a "Design — not yet implemented" header so the function signature does not read as current reality.

---

## A.16 — `ValidationArc` + `EmotionalProvenance` (extra lifecycle model)

**Status**: DONE
**Severity**: —
**Doc claim**: None of Docs 00/01/02/03/07/11 mention `ValidationArc` or `EmotionalProvenance` as part of the Neuro lifecycle. Doc 13 Somatic Integration (not in this category) mentions the somatic landscape only. The item was listed as an additional check for "what the code carries that the doc does not describe."
**Reality**: `crates/roko-neuro/src/lib.rs:123-135` defines `pub enum ValidationArc` (`#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]`, `#[serde(rename_all = "snake_case")]`) with four variants: `Redemptive`, `Contaminating`, `Stable`, `Progressive`. `crates/roko-neuro/src/lib.rs:137-149` defines `pub struct EmotionalProvenance { average_pad: PadVector, discovery_emotion: String, validation_arc: Option<ValidationArc>, emotional_diversity: f64 }`. Constructor `EmotionalProvenance::from_tag()` at `:151-161` builds provenance from a single `EmotionalTag`; helper `coarse_emotion_label()` at `:164-181` discretizes PAD into `{positive|negative|neutral}_{high|mid|low}_arousal`. Both types are attached to `KnowledgeEntry.emotional_provenance: Option<EmotionalProvenance>` at `:238-239` and `KnowledgeEntry.emotional_tag: Option<EmotionalTag>` at `:235-236`. They feed into `emotional_consolidation_boost()` at `:304-323` (Redemptive → ×1.06, Progressive → ×1.04, Stable/None → ×1.0, Contaminating → ×0.94, plus `emotional_diversity × 0.15` and `intensity × 0.05` linear terms) and `emotional_retrieval_boost()` at `:330-339` (which adds an additional `intensity × 0.08` multiplier). Retrieval scoring uses this via `effective_confidence()` at `knowledge_store.rs:866-868`: `bounded_confidence × confirmation_boost × emotional_consolidation_boost`.
**Fix sketch**: No code change. Consider adding a short §"Emotional provenance and validation arcs" section to Doc 00 or 02 describing how the affect signal feeds tier progression, or leave it to Doc 13 §"Somatic Integration" per cross-references. Doc 01 `:317` should mention `emotional_tag` and `emotional_provenance` in the field table alongside `hdc_vector`.

---

## A.17 — Reactive AntiKnowledge checking against new candidates

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 §"Reactive Checking" at `:169-182` and §"Reactive checking against new knowledge candidates" at `:299-369` describe a mechanism where new candidate entries are compared by HDC similarity against every existing AntiKnowledge entry; if similarity > 0.526 the candidate is blocked/contested. Doc 11 `:317-366` provides a concrete `NeuroStore::reactive_anti_check(&candidate) -> ReactiveCheckResult` signature with `Contradiction { anti_entry_id, similarity, evidence }`. Doc 01 `:391` lists "Reactive AntiKnowledge checking (new candidates checked against existing AntiKnowledge)" as Missing.
**Reality**: `rg 'reactive_anti_check|ReactiveCheckResult|Contradiction' crates/` returns zero matches in `roko-neuro`. The only `entries_by_kind` hit is `roko-orchestrator/src/event_log.rs:295` (unrelated — event log, not knowledge store). `KnowledgeStore::ingest()` at `knowledge_store.rs:163-258` does have an AntiKnowledge branch at `:180-217` but it **only halves the confidence of the directly-named `refuted_insight_id` peer** (at `:210-211` via `original.confidence *= 0.5`); it does not perform HDC-similarity scan against the full store. Reactive similarity checking is unimplemented.
**Fix sketch**: Either implement `reactive_anti_check` as an HDC-index scan inside `KnowledgeStore::ingest()` (behind the `hdc` feature), or move Doc 11 `:299-369` behind a "Design — not yet implemented" header so the function signature does not read as current reality. Doc 11 `:920` already concedes "Reactive checking (designed above)" is missing; the body of §"Reactive checking" should match that status header-style.

---

## A.18 — Automatic AntiKnowledge generation from gate failures

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 `:920` concedes "Automatic AntiKnowledge generation from gate failures (designed above; needs wiring into gate pipeline)". Doc 11 §"The Challenge Mechanism" at `:27-51` shows the shape of a generated AntiKnowledge entry but the trigger is described as "typically a gate failure or a direct observation that the claimed pattern does not hold".
**Reality**: `rg 'AntiKnowledge' crates/roko-gate/` returns zero matches. The gate pipeline in `crates/roko-gate/src/` does not construct `KnowledgeEntry { kind: KnowledgeKind::AntiKnowledge, .. }` on failure. `crates/roko-cli/src/orchestrate.rs` pushes gate verdicts into the episode logger, but no automatic refutation flow synthesizes an AntiKnowledge record from a gate fail. AntiKnowledge entries today only arrive through manual `ingest()` calls with an AntiKnowledge `kind`.
**Fix sketch**: Wire a hook in the orchestrator gate-post-check path that, when a gate fails on a task whose context included a specific `KnowledgeEntry` (retrievable from the episode record), synthesizes an AntiKnowledge entry with `refuted_insight_id = <retrieved_entry.id>`, `refutation_evidence = <gate output summary>`, `source_episodes = [<episode_id>]`. Alternatively, leave the flow out of code and mark the doc 11 §"Challenge Mechanism" prose as design-only.

---

## A.19 — Epistemic parasite detection + Price equation diagnostics

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 §"Memetic Evolution Context" at `:100-140` describes the fitness function `W(E) = f × r × L` and the Price equation decomposition `Δmean_fitness = Cov(fitness, frequency) + E(Δfitness)` as knowledge-base-health diagnostics. Doc 11 §"Epistemic parasite detection" at `:371-410` provides a concrete `fn fitness(entry, stats) -> f64` and `fn decision_quality(entry, outcomes) -> f64` signature, and `:790-886` describes a `KnowledgeHealthAudit` struct emitting `AuditReport { health_score, parasites_detected, price_selection, price_transmission, cascade_risk_entries, recommendations }`.
**Reality**: `rg 'fitness|preservation_rate|decision_quality|epistemic_parasite|price_equation|KnowledgeHealthAudit|AuditReport' crates/roko-neuro/` returns zero matches. No fitness computation, no Price-equation diagnostic, no parasite detector. Doc 11 `:920` concedes "Epistemic parasite detection (designed above; needs outcome tracking). Price equation diagnostics (designed above)" — the present file had no item tracking this.
**Fix sketch**: Either keep these as a future-work doc section (cleanest) or implement a standalone `KnowledgeHealthAudit` in `roko-neuro` that reads the `knowledge-confirmations.jsonl` sibling file plus the roko-learn outcome log. At minimum, move Doc 11 `:371-410` and `:790-886` behind a clearly-labeled design header so the `fn` signatures do not read as current reality.

---

## A.20 — Neuro as "semantic wrapper around Substrate"

**Status**: NOT DONE
**Severity**: MED
**Doc claim**: Doc 00 §"Architectural Position: Semantic Wrapper Around Substrate" at `:80-100` claims Neuro "builds on top of" `Substrate` and "calls `Substrate.put()` and `Substrate.query()` underneath, but adds knowledge-specific logic on top". Abstract at `:27` similarly calls Neuro "a **semantic wrapper** around `Substrate` (the generic Engram storage trait from the Synapse Architecture)".
**Reality**: `rg 'Substrate' crates/roko-neuro/` returns **zero matches**. The `NeuroStore` trait at `crates/roko-neuro/src/lib.rs:349-364` is fully self-contained (`init`, `query`, `ingest`, `decay`, `gc`) and does not wrap or delegate to any `Substrate` implementation. `KnowledgeStore` at `crates/roko-neuro/src/knowledge_store.rs:71-75` uses a raw `PathBuf` + `parking_lot::Mutex` and writes JSONL directly; it does not consume `roko_core::Substrate` at all. The only `roko_core` types imported are `EmotionalTag`, `PadVector`, and (in `distiller.rs`/`context.rs`) `Engram`, `Body`, `Kind`, `Provenance` for constructing ambient context — not for substrate calls.
**Fix sketch**: Either wire `KnowledgeStore` through a `Substrate` adapter (preserving the Synapse-Architecture invariant) or rephrase Doc 00 `:23-30` and `:80-100` to describe the current reality — Neuro is a *peer* subsystem of Substrate, not a wrapper, and its JSONL log is its own substrate. Currently the doc's "Substrate semantic wrapper" framing is aspirational. Doc 00 `:16-17` lists the placeholder `crates/roko-golem/src/grimoire.rs` as a source; remove that stale path since the crate is dissolved (see A.01).

---

## A.21 — CausalLink HDC permute encoding

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 `:390` lists "`CausalLink` variant with 60-day half-life and HDC permute encoding" as Missing. Doc 01 `:267` says "CausalLink — Canonical variant with directional HDC encoding" as if implemented — the two sections contradict.
**Reality**: `crates/roko-neuro/src/hdc.rs:12-113` implements `CausalLinkParts` and a `CausalLink`-specific encoding path at `:60-80` using `role_hv("cause").permute(CAUSE_SHIFT).bind(&text_hv(&parts.cause))` and the parallel effect path — classic directional binding via permutation. The encoder is gated behind `#[cfg(feature = "hdc")]` at the crate level (`lib.rs:371-372`). Tests at `hdc.rs:285-330` construct a `KnowledgeEntry { kind: KnowledgeKind::CausalLink, .. }` and verify the permute encoding round-trips. The 60-day base half-life is `CAUSAL_LINK_HALF_LIFE_DAYS = 60.0` at `lib.rs:35` (see A.08). Both parts of the doc 01 `:390` "Missing" bullet are **stale**.
**Fix sketch**: Delete the "`CausalLink` variant with 60-day half-life and HDC permute encoding" bullet from Doc 01 `:390` and move it to the "Implemented" list at `:382-386`. The `:267` row is already correct.

---

## A.22 — Inferred retention tier on ingest

**Status**: DONE
**Severity**: —
**Doc claim**: None of Docs 00/01/02/03/07/11 mention an automatic tier inference at ingest time. Doc 02 `:267-274` "Implemented" and `:275-282` "Missing" both assume tiers flow only from `evaluate_tier_progression`.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:761-785` defines `fn inferred_retention_tier(entry)` and `fn normalize_entry_tier(entry)` which upgrade (never downgrade) the `entry.tier` at ingest time based on `entry.kind`, `source_episodes.len()`, and `confidence`. Rules at `:773-784`: `StrategyFragment` with ≥3 source episodes → `Persistent`; `Warning` with ≥2 sources or confidence ≥0.85 → `Consolidated`; `AntiKnowledge` always → `Working` (minimum); `Insight`/`Heuristic`/`CausalLink` with ≥4 sources or confidence ≥0.9 → `Consolidated`; with ≥2 sources or confidence ≥0.7 → `Working`; else `Transient`. This is the mechanism that allows `Persistent` tier to be reached without going through the 3-success gate loop (see A.14 Notes). Called from `normalize_entry_for_ingest` at `:749-759` on every `ingest`.
**Fix sketch**: No code change. Consider adding a short §"Automatic tier inference on ingest" note to Doc 02 describing the `source_count + confidence → tier` heuristic, since it materially affects whether a newly-distilled entry starts at `Working` vs `Transient`.

---

## A.23 — AntiKnowledge-triggered refuted-peer confidence halving

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 `:245-246` in the "Interaction with Confidence" section says "Negative outcome → confidence × 0.5 (halved)" as a general decay rule, implying negative gate outcomes halve confidence on the used entry.
**Reality**: `crates/roko-neuro/src/knowledge_store.rs:180-217` implements a **different** halving pathway. When an ingested batch contains an `AntiKnowledge` entry whose `refuted_insight_id` names an existing entry, `KnowledgeStore::ingest()` rewrites the store with `original.confidence *= 0.5` at `:210-211` applied to each refuted peer. Unit test `antiknowledge_halves_refuted_entry_confidence` at `:1415` verifies this behavior. There is no code path that halves confidence on a general negative gate outcome — that responsibility sits in `evaluate_tier_progression` (A.14), which demotes tier but does not multiply confidence.
**Fix sketch**: Either amend Doc 02 `:245-246` to specify "Negative outcome in an AntiKnowledge refutation → confidence × 0.5 on the refuted entry" (scoping the rule accurately), or add a general-purpose gate-outcome-→-confidence hook to match the doc claim. Current state is an unadvertised capability (halving on refutation) plus an over-promised doc line (halving on any negative outcome).

---

## Section Summary

| Status | Count | Items |
|--------|-------|-------|
| DONE | 15 | A.01, A.02, A.04, A.05, A.06, A.07, A.08, A.10, A.11, A.13, A.14, A.16, A.21, A.22, A.23 |
| PARTIAL | 3 | A.03 (schema extras undocumented), A.09 (stale `FACT_HALF_LIFE_DAYS`/`Fact` references), A.12 (`CONFIRMATION_BOOST` private + gated on multi-source) |
| NOT DONE | 5 | A.15 (on-chain demurrage), A.17 (reactive HDC checking), A.18 (auto-gen AntiKnowledge from gate fails), A.19 (parasite detection + Price equation), A.20 (Substrate wrapper claim) |
| SCAFFOLD | 0 | — |

The Neuro subsystem matches its documentation to a high degree on the core
mechanics but diverges on a sizeable set of design-only extensions.
`roko-golem` is physically dissolved — the crate directory is gone and no
legacy `Grimoire*`, `Mortality*`, or `Golem*` identifier survives in
`crates/`. Project `CLAUDE.md` is the only stale artifact still listing
`roko-golem` as Phase 2+. The six `KnowledgeKind` variants are PRD-native with
legacy aliases preserved only for deserialization; the four `KnowledgeTier`
variants carry exact `0.1/0.5/1.0/5.0` multipliers (as `f32`); the Ebbinghaus
formula is implemented as `0.5^(age_days/effective_half_life)`; the
AntiKnowledge confidence floor of 0.3 is enforced in both decay and GC;
CausalLink directional HDC permute encoding is wired in `hdc.rs`; tier
promotion (3 successes) and demotion (2 failures) are wired in
`tier_progression.rs` despite Doc 02 still listing them as missing; an
additional automatic tier inference on ingest gives StrategyFragment a path
to Persistent that bypasses the promotion gate.

The remaining gaps split into three categories:
1. **Pure doc drift**: Doc 03/07 still reference a `FACT_HALF_LIFE_DAYS`
   constant and a `Fact` variant that no longer exist. Doc 01's
   current-implementation code block lists seven variants when the enum has
   six. Doc 02 still lists tier promotion/multiplier as Missing. Doc 07's
   reference to `CONFIRMATION_BOOST` as `pub` is wrong (it is private). Doc
   01 omits `source_model`, `model_generality`, `tier`, `emotional_tag`,
   `emotional_provenance` from the `KnowledgeEntry` field table.
2. **Design-only extensions**: On-chain half-speed demurrage for
   AntiKnowledge (A.15), reactive HDC-similarity checking for new candidates
   (A.17), automatic AntiKnowledge generation on gate failure (A.18), and
   epistemic parasite detection with Price-equation diagnostics (A.19) are
   all present as `fn` signatures in the PRD docs but have **zero** backing
   code in `roko-chain` or `roko-neuro`.
3. **Architectural drift**: Doc 00 claims Neuro is a "semantic wrapper
   around Substrate" but `NeuroStore` and `KnowledgeStore` do not import or
   call `roko_core::Substrate` at all — Neuro is a peer subsystem, not a
   wrapper (A.20).

## Agent Execution Notes

### A.03 / A.09 / A.12 / A.20 — Prefer Contract Cleanup Over New Subsystems

Best use of this section in batch `06`:

1. fix schema and lifecycle docs so they match the real `KnowledgeEntry` and tier/decay contract,
2. correct the stale `Fact` and `FACT_HALF_LIFE_DAYS` story,
3. make the “Neuro wraps Substrate” claim honest.

Do not widen this into on-chain demurrage, reactive AntiKnowledge, or parasite-diagnostics implementation unless another batch explicitly owns it.

### A.15 / A.17-A.19 — Research / Chain Handoff

These items are real gaps, but they are poor unattended-batch targets for `06` unless a later chain or AntiKnowledge pass is explicitly created.

Acceptance criteria for this section:

- later agents can tell which lifecycle behavior is shipping,
- stale schema and tier docs no longer contradict code,
- research or chain-only extensions are clearly deferred.
