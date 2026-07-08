# Prompt: 09-daimon

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/`. Covers the Daimon affect engine — PAD vector, ALMA three-layer model, OCC/Scherer appraisal, somatic markers (Damasio, k-d tree over 8D strategy space), 6 behavioral states (NO mortality), integration with tier routing / VCG bidding / SystemPromptBuilder / CascadeRouter, collective emotional contagion.

**CRITICAL**: This topic MUST preserve all affect citations but MUST NOT propagate death/mortality framing. The Daimon in the new architecture tracks cognitive performance, NOT mortality anxiety.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` §2 Daimon — all sections (PAD vector, 6 behavioral states, somatic markers, coding integration, integration points)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §III Somatic Landscape (k-d tree, 15% contrarian retrieval, concrete struct), §XIX.F 8D Somatic Strategy Space (default coding dims, chain dims), §XIX.E VCG Bid Computation (affect weight formula)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` §**INCOMPATIBLE: Emotion Mapped to Mortality** — read carefully
4. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 2D/2E (Daimon PAD tracking, behavioral modulation)

## Step 3 — SOURCE-INDEX entry `## 09-daimon.md`

Read every file. Key legacy:
- `bardo-backup/prd/03-daimon/00-overview.md`, `01-appraisal.md`, `02-emotion-memory.md`, `03-behavior.md`, `06-dream-daimon.md`, `07-runtime-daimon.md`, `08-infrastructure.md`, `09-evaluation.md`

**SKIP ENTIRELY**: `bardo-backup/prd/03-daimon/04-mortality-daimon.md` (extract somatic marker + ALMA citations only) and `bardo-backup/prd/03-daimon/05-death-daimon.md` (skip completely).

## Step 4 — implementation-plans

- `12a-cognitive-layer.md` §F Daimon (F1–F9: PadVector, 8 octant states, appraisal triggers, decay, behavior modulation table, affect signatures on episodes, affect → SystemPromptBuilder, affect → CascadeRouter, persistence)
- `12a-cognitive-layer.md` §R2 roko-daimon crate creation plan

## Step 5 — active code

- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/lib.rs` (scaffold or full)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/daimon.rs` (972 lines — the existing implementation to move). This contains the actual PAD logic already.

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/09-daimon
```

Write **14 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-and-mortality-incompatibility.md` | Daimon as cognitive performance affect engine. **Explicit statement that the new architecture REMOVES the mortality framing.** What Daimon is (PAD tracking). What it's NOT (mortality anxiety). The reframe rationale. Refactoring-prd/08-translation-guide.md §INCOMPATIBLE reference. |
| 01 | `01-pad-vector.md` | Mehrabian 1996 PAD (Pleasure-Arousal-Dominance). Each dimension in [-1, 1]. 8 octant states. What each dimension captures for agents: Pleasure (outcome quality trajectory), Arousal (cognitive load & urgency), Dominance (confidence in approach). Full Rust struct. |
| 02 | `02-alma-three-layer-temporal.md` | ALMA three-layer: Emotion (seconds, reactive to immediate events) → Mood (hours, accumulated emotional trajectory) → Personality (lifetime, stable traits). How the three layers interact. Update rules per layer. |
| 03 | `03-occ-scherer-appraisal.md` | OCC and Scherer 2001 appraisal theory. Appraisal triggers: gate pass/fail, task outcome, blockers, time pressure, prediction accuracy. Event → appraisal → PAD update. Full appraisal rule set. |
| 04 | `04-six-behavioral-states.md` | (NOT mortality phases). Engaged (balanced) / Struggling (low P, high A → force T2, escalate, re-plan, help) / Coasting (high P, low A → T0/T1, more tasks, cheap models) / Exploring (low D → T2 research, T1 breadth) / Focused (high D, high P → T0/T1 exploit known patterns) / Resting (low A, low D → T1 Dreams, offline learning). Cyclical, never terminal. Tier bias table. |
| 05 | `05-behavioral-state-to-tier-routing.md` | Concrete mechanism: behavioral state modulates tier router's prediction error threshold. Struggling agents have lower T2 trigger (route to deep reasoning sooner). Coasting agents have higher trigger (stay cheap longer). **This is the compute-allocation control mechanism** — affect directly controls compute investment. |
| 06 | `06-somatic-markers-damasio.md` | Damasio 1994 "Descartes' Error" somatic marker hypothesis. Emotions provide fast heuristics for decisions. Implemented as **k-d tree over 8D strategy space**. Before acting, agent queries nearest neighbors for emotional valence of similar past strategies. Sub-1ms latency. System 1 fast-path before analytical reasoning. Full `SomaticLandscape` and `SomaticMarker` structs from 09-innovations.md §III. |
| 07 | `07-15-percent-contrarian-retrieval.md` | Bower 1981 — mood-congruent memory retrieval causes emotional echo chambers. Solution: mandatory 15% contrarian retrieval. System always retrieves at least 15% from markers with opposite valence. Ensures consideration of counterarguments, prevents confirmation bias. |
| 08 | `08-8-dimensional-strategy-space.md` | Domain-configurable. Default for **coding agents**: Complexity / Risk / Novelty / Confidence / Time pressure / Scope / Reversibility / Dependency depth. **Chain agents**: volatility / exposure / liquidity / correlation / leverage / time_horizon / slippage_risk / counterparty_risk. Each dimension [0,1]. k-d tree stores markers at these coordinates. |
| 09 | `09-mood-congruent-memory.md` | Current emotional state biases which knowledge is surfaced. Integration with Neuro. High arousal → recent + action-oriented. Low pleasure → include past failure context. Bower 1981, Blaney 1986. |
| 10 | `10-integration-points.md` | PAD drives 4 integration points simultaneously: (1) Behavioral state selection → self-model and TUI, (2) Tier routing bias → prediction error threshold, (3) VCG auction bidding → urgency × affect_weight (full formula from 09-innovations.md §XIX.E: `bid = expected_value × (1 + arousal × 0.5) × (1 + 0.3 × abs(pleasure - 0.5))`), (4) Somatic landscape querying. Different projections of same state. |
| 11 | `11-coding-agent-integration.md` | Per-crate confidence. "I'm confident with roko-core but uncertain about roko-daimon." Error pattern sensitivity — "I've seen this borrow checker error before, I know what to do." Fatigue detection — repeated failures on same task → high arousal + low pleasure → trigger re-plan or model escalation. NOT just for chain agents — every agent benefits from cognitive state tracking. |
| 12 | `12-collective-emotional-contagion.md` | Exponential decay of affect across mesh. Somatic field shared across agent collective. Coordination via affect sharing. Integration with C-Factor. |
| 13 | `13-current-status-and-gaps.md` | roko-daimon crate scaffold. Existing roko-golem/daimon.rs (972 lines) to move (per R2 dissolution plan). Wiring gaps from 12a §F (F1–F9). Persistence to `.roko/daimon/affect.json`. Integration points not yet wired. Which PRD docs are SKIPPED (04-mortality-daimon, 05-death-daimon). |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥3500 total. Preserve ALL affect citations: Mehrabian 1996 (Current Psychology 14(4)), Damasio 1994 (Descartes' Error), Bechara et al. 1994-1999, Bower 1981 (mood congruence), Blaney 1986, Plutchik 1980, Russell-Mehrabian 1977, Scherer 2001 (appraisal), Walker & van der Helm 2009 (emotional depotentiation), Zhang et al. SIGDIAL, OCC (Ortony Clore Collins 1988).

Cross-reference topics 00-architecture (universal loop META-COGNIZE step), 02-agents (behavioral state → tier routing), 03-composition (affect-modulated retrieval, VCG auction bid), 05-learning (Daimon feedback from gate outcomes), 06-neuro (mood-congruent retrieval), 10-dreams (dream-daimon reframed).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL AFFECT CITATIONS.
- **NO MORTALITY**. Daimon tracks cognitive performance, not mortality anxiety. Pleasure = task success, Arousal = urgency/load, Dominance = confidence. No death/dying/mortality framing allowed.
- SKIP `04-mortality-daimon.md` and `05-death-daimon.md` as SOURCES (do not base content on them; extract only citations from 04 if needed).
- Behavioral states are **cyclical** — Engaged ↔ Struggling ↔ Coasting ↔ Exploring ↔ Focused ↔ Resting. **NO Terminal state.**
- 15% contrarian retrieval is mandatory (Bower 1981).
- Apply naming map: golem → agent; clade → collective/mesh.
- Use Write tool. Don't ask questions. Continue.
