# D — Memory Integration and Integration Points (Docs 09, 10, 11)

Parity of the three "wiring" chapters: mood-congruent memory (Bower
1981), the four integration points (behavioral state, tier routing, VCG
bidding, somatic landscape), and coding-agent integration
(per-crate confidence, error-pattern familiarity, fatigue detection).

The emotional-tag plumbing + SystemPromptBuilder + CascadeRouter + VCG
auction integration are all partially to fully wired. The
coding-specific signals (per-crate confidence, fatigue, error-pattern
tracker) are the biggest remaining gap.

Generated 2026-04-16.

---

## D.01 — `EmotionalTag` struct ships in `roko-core` and is consumed across the stack (Doc 09 §"EmotionalTag on Engrams")

**Status**: DONE
**Severity**: —
**Doc claim**: `EmotionalTag { pad: PadVector, intensity: f32, trigger: String, mood_snapshot: PadVector }` attached to Engrams. Clamped on construction. Consumed by the retrieval layer for emotional congruence scoring.
**Reality**: `EmotionalTag` at `roko-core/src/affect.rs:162-189` matches doc exactly — `new()` clamps PAD via `.clamped()`, `intensity` via `.clamp(0.0, 1.0)`, and `mood_snapshot` via `.clamped()`. Test `emotional_tag_clamps_inputs` at `:231-242` pins the clamping. Consumers (grep-verified): `roko-learn/src/episode_logger.rs`, `roko-neuro/src/knowledge_store.rs`, `roko-neuro/src/distiller.rs`, `roko-neuro/src/context.rs`, `roko-dreams/src/{threat, imagination, hypnagogia, cycle}.rs`, `roko-compose/src/{system_prompt_builder, role_prompts, prompt, context_provider}.rs`. `AffectState::emotional_tag(trigger)` at `roko-daimon/src/lib.rs:107-110` produces the tag from the current state (normalized intensity = `PAD.magnitude() / √3`).

One doc-drift caveat matters for later agents: `docs/09-daimon/09-*`
still shows an `emotion: String` / `discovery_emotion` field and
Plutchik-backed schema examples. Those are **not** part of the
shipping `EmotionalTag` contract.

---

## D.02 — Emotional provenance transfers into knowledge consolidation (Doc 09 §"Emotional Provenance", §"Four-Factor Retrieval")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Four-factor scoring = `recency × importance × relevance × emotional_congruence`. Emotional tags persist from Engrams → distilled knowledge entries → retrieved context. Mood-congruent retrieval weighted by PAD cosine similarity.
**Reality**: Doc 13 §"Unimplemented Features — Emotional Memory Integration" confirms shipping vs gap:
- `ContextAssembler` in `roko-compose/src/context_assembler.rs` now uses `EmotionalTag` at retrieval time (partial)
- Neuro (`roko-neuro/src/knowledge_store.rs`, `context.rs`, `distiller.rs`) preserves emotional tags and derived emotional provenance metadata during consolidation (Done per Doc 13)
- PAD cosine similarity is used for retrieval congruence scoring (Done per Doc 13)
- Four-factor scoring is "partially implemented locally in Neuro; the richer cross-subsystem variant is still missing" (per Doc 13)

So: emotional provenance transfer = Done; PAD cosine similarity in retrieval = Done; full four-factor cross-subsystem scoring = Partial.
**Fix sketch**: Doc 09 §"Four-Factor Retrieval" should note that the shipping implementation is scoped to Neuro internally; the cross-subsystem variant (where compose, serve, and cli all apply emotional congruence in their own retrieval paths) is frontier.

---

## D.03 — Mood-congruent memory retrieval is locally live (Doc 09 §"Mood-Congruent Retrieval")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Retrieval scores are biased by `PadVector::cosine_similarity(query_mood, entry_mood)`. Bower 1981 associative-network theory predicts 5-30% accuracy boost (Faul & LaBar 2022).
**Reality**: `cosine_similarity` at `roko-core/src/affect.rs:70-81` (see A.06) is the primitive. It is called from Neuro knowledge-store retrieval (Doc 13 "PAD cosine similarity is now used for retrieval congruence scoring"). But per-subsystem coverage is uneven — only Neuro's internal retrieval path uses it today; the broader compose / serve retrieval paths are not yet on the same signal. Same caveat as D.02.

---

## D.04 — Emotional diversity + validation arcs persist on consolidated knowledge (Doc 09 §"Narrative Identity", §"Validation Arcs")

**Status**: DONE
**Severity**: —
**Doc claim**: McAdams 2001 narrative-identity concept: consolidated knowledge entries carry validation arcs (Redemptive / Contaminating / etc.). Emotional diversity signals retrieval reliability.
**Reality**: Per Doc 13: "Emotional diversity and heuristic validation arcs now persist on `KnowledgeEntry.emotional_provenance` and contribute a small retrieval reliability boost." `roko-neuro/src/knowledge_store.rs` has `emotional_provenance` hits (grep-verified). Matches the doc claim.

---

## D.05 — Four integration points: 3 of 4 are wired (Doc 10 §"Four Integration Points")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Daimon integrates at four points:
1. Behavioral state selection (consumed by dispatch)
2. Tier routing bias (consumed by CascadeRouter)
3. VCG auction bidding (consumed by prompt composer)
4. Somatic landscape querying (consumed by dispatch / retrieval)

**Reality**:
- Point 1 (Behavioral state → dispatch): **Done** — `DispatchParams` + `modulate()` (see B.05).
- Point 2 (Tier routing bias): **Done** — `DaimonPolicy` consumed by `roko-learn/src/cascade_router.rs` (see B.06).
- Point 3 (VCG auction bidding): **Partial** — per Doc 13: "orchestration now passes live PAD state into `PromptComposer`, and the shared prompt auction applies those multipliers plus per-bidder PAD modulation before selecting winners. Diagnostic externality payments are now emitted on the composed prompt as an approximate VCG accounting layer." Not the full urgency + affect-weight formula, but real.
- Point 4 (Somatic landscape querying): **Partial** — per Doc 13: "`query_somatic()` and `modulate_with_strategy()` blend nearby and contrarian markers to bias dispatch before task execution; strong matches now emit explicit runtime events" — functional but only in dispatch path, not cross-subsystem.

So 2 of 4 Done, 2 of 4 Partial. All 4 have wiring today.
**Fix sketch**: Doc 10 should clarify per-integration-point status alongside the integration map. Points 1-2 are production-ready; points 3-4 are live but have feature-depth gaps.

---

## D.06 — VCG auction bidding formula is partial (Doc 10 §"VCG Auction Bidding (Full Formula)")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 10 §"VCG Auction Bidding" specifies full formula:
- `urgency = 1 + arousal × 0.5`
- `affect_weight = 1 + 0.3 × |pleasure - 0.5|`
- Bid per context candidate = `relevance × affect_weight × urgency`
- Pay = second-highest bid (truthful bidding, Vickrey 1961)

**Reality**: Per Doc 13 §"Unimplemented Features — VCG Auction Integration":
- "Urgency multiplier from arousal" — partial
- "Affect weight from pleasure extremes" — partial
- "orchestration now passes live PAD state into `PromptComposer`, and the shared prompt auction applies those multipliers plus per-bidder PAD modulation before selecting winners" — real
- "Diagnostic externality payments are now emitted on the composed prompt as an approximate VCG accounting layer" — real

So the multipliers and auction-select logic ship; the exact payment formula with full externality accounting is an approximation. The shipping code computes what Vickrey prices "would have been", not a production payment ledger.
**Fix sketch**: Doc 10 §"VCG Auction Bidding" should note the shipping approximation scope — multipliers + auction selection live, full externality payment ledger frontier.

---

## D.07 — Event emission and integration map are wired (Doc 10 §"Event Emission", §"Integration Map")

**Status**: DONE
**Severity**: —
**Doc claim**: Daimon emits explicit runtime events on state transitions and on strong somatic matches, visible in the episode log / TUI.
**Reality**: `SomaticSignal::should_emit_event()` at `roko-daimon/src/lib.rs:1022-1025` gates emission. Episode logger at `roko-learn/src/episode_logger.rs` consumes `PadVector` / `EmotionalTag` (grep-verified). CLAUDE.md confirms episode recording to `.roko/episodes.jsonl` includes daimon events.

---

## D.08 — Per-crate confidence aggregation is unimplemented (Doc 11 §"Per-Crate Confidence")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 §"Per-Crate Confidence" describes aggregating success / failure counts per crate path (e.g., `crates/roko-chain` has 90% success, `crates/roko-agent` has 60%). Used to bias future task dispatch and to adjust strategy-coordinate `familiarity`.
**Reality**: `Grep 'per_crate_confidence\|crate_success_rate\|CrateFamiliarity' crates/ --include=*.rs` returns zero matches in daimon + learn. The shipping `TaskContext.existing_code_familiarity` at `roko-daimon/src/lib.rs:396-397` is **per-task**, not aggregated per-crate. No crate-level confidence ledger exists. Doc 13 §"Unimplemented Features — Coding Agent Integration" lists "Per-crate confidence aggregation" as not done.
**Fix sketch**: Doc 11 stays `Design — Phase 2+ Tier 2H/2D+`.

---

## D.09 — Error-pattern tracker with familiarity scoring is unimplemented (Doc 11 §"Error Pattern Sensitivity with Familiarity Scaling")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: An error-pattern tracker identifies recurring error signatures; familiarity with a pattern (seen-before count) scales emotional intensity downward (familiar errors are less arousing than novel ones).
**Reality**: `Grep 'ErrorPattern\|error_pattern_tracker\|PatternFamiliarity' crates/roko-daimon crates/roko-agent crates/roko-conductor --include=*.rs` returns zero matches for the daimon-side pattern tracker. `roko-conductor/src/diagnosis.rs` has an `ErrorCategory` / `ErrorPattern` pair (34 patterns per batch 07 section A.06), but that is the conductor's diagnosis table, not the daimon's familiarity-scaled emotional impact. No coupling from conductor `ErrorPattern` → daimon arousal scaling exists today.
**Fix sketch**: Doc 11 stays `Design — Phase 2+`. If the coupling ever lands, it should read conductor's diagnosis output and scale the appraise() arousal delta by `1 / (1 + seen_count)`.

---

## D.10 — Fatigue detection (consecutive failures) is unimplemented (Doc 11 §"Fatigue Detection")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Detects sustained consecutive-failure runs → triggers Struggling state + strategy change + possibly forced break (maintenance mode).
**Reality**: `Grep 'fatigue\|Fatigue\|consecutive_failures\|ConsecutiveFailures' crates/roko-daimon crates/roko-learn --include=*.rs` returns zero matches in daimon. The closest shipping surface is `crates/roko-conductor/src/circuit_breaker.rs` (per batch 07 — `MAX_PLAN_FAILURES = 2` tripping behavior), but that is plan-level circuit breaking, not per-agent fatigue tracking. Doc 13 confirms: "Fatigue detection (consecutive failure monitoring) — not done". No Seligman learned-helplessness trigger (Doc 07 §"Three Loop-Breaking Mechanisms" cross-ref, Seligman 1967).
**Fix sketch**: Doc 11 stays frontier. A minimal implementation would add `consecutive_failures: u32` to `AffectState`, increment on `TaskOutcome { succeeded: false }`, reset on success.

---

## D.11 — `SystemPromptBuilder` integration is wired via live Daimon PAD (Doc 11 §"SystemPromptBuilder Integration")

**Status**: DONE
**Severity**: —
**Doc claim**: The current PAD + behavioral state flows into the SystemPromptBuilder, where it modulates tone / retry framing / risk-aversion wording in the composed system prompt.
**Reality**: `roko-compose/src/system_prompt_builder.rs` consumes `PadVector` / `EmotionalTag` (grep-verified). Doc 13: "F7 — Affect → SystemPromptBuilder — Done; live Daimon PAD now feeds affect guidance in the system prompt." `roko-compose/src/role_prompts.rs` and `prompt.rs` also consume daimon types. The 6-layer system prompt (CLAUDE.md "SystemPromptBuilder (6-layer prompts) — Wired") includes an affect layer.

---

## D.12 — Conversational tone mapping / TUI visualization is partial (Doc 10 §"Conversational Tone Mapping")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Behavioral state maps to conversational tone (curious / confident / cautious / etc.) surfaced in CLI / TUI / Spectre visualization.
**Reality**: `roko-cli/src/tui/` consumes behavioral state (via `DaimonPolicy` per grep) and the TUI dashboard displays affect state (CLAUDE.md "Interactive TUI — Wired; F1–F7 tabs"). The full Spectre visualization + tone-per-state mapping is not all in place (no Spectre in current crate list); the direct TUI display is.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 4 (D.01 EmotionalTag cross-crate consumption, D.04 emotional diversity + arcs on KnowledgeEntry, D.07 event emission, D.11 SystemPromptBuilder) |
| PARTIAL | 5 (D.02 four-factor cross-subsystem, D.03 mood-congruent retrieval, D.05 4 integration points, D.06 VCG formula approximation, D.12 conversational tone / TUI) |
| NOT DONE | 3 (D.08 per-crate confidence, D.09 error-pattern tracker, D.10 fatigue detection) |

Section D has the most "honest partial" entries — a lot of integration
points are live but at reduced feature depth. The emotional-tag /
SystemPromptBuilder / cascade-router wiring is real; the four-factor
cross-subsystem retrieval, VCG externality payment ledger, and
coding-specific tracking (per-crate confidence, error-pattern
familiarity, fatigue) are the natural next surfaces.

## Agent Execution Notes

### D.01 / D.04 / D.07 / D.11 — Trust the shipping wiring

Mass consumer list for `EmotionalTag` / `PadVector` spans 15+ files.
This is the most cross-cutting daimon surface today.

### D.02 / D.03 / D.05 / D.06 / D.12 — Incremental deepening

Each PARTIAL entry is a well-scoped deepening pass: widen Neuro's
four-factor scoring to compose and serve; flesh out VCG externality
accounting; add full tone mapping to CLI output. None require new
Daimon primitives.

One ownership note should be explicit in the docs: the live
`ContextAssembler` implementation is in `roko-neuro`, while
`roko-compose` re-exports it. If a doc names Compose as the owner, it
should also point at the underlying Neuro implementation.

### D.08 / D.09 / D.10 — Coding integration frontier

Per-crate confidence, error-pattern familiarity scaling, and fatigue
detection are all in Doc 13's "not started" list. They are cheap
additions to `AffectState` when eventually wanted.

Acceptance criteria for this section:

- Doc 10 §"Four Integration Points" has per-point status,
- Doc 11 frontier items are banner-tagged,
- Doc 09 clarifies that four-factor retrieval is Neuro-internal today,
- Doc 09 does not imply a stored Plutchik field on `EmotionalTag`.
