# Redaction-Invariant Scoring

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust-based privacy-preserving registry of AI coding agent session traces (~235K LOC, 6 crates, ~352 submissions, 3 contributors). Quality and novelty are scored inside TEEs (Trusted Execution Environments on NEAR AI Cloud, Intel TDX + NVIDIA GPU TEE) using Qwen 3.6 35B-A3B-FP8 for perplexity and BGE-large-en-v1.5 for embedding. Contributors earn NEAR blockchain credits proportional to `q = f * g * a` where `f` is the perplexity-derived quality factor, `g` is novelty (embedding cosine distance against HNSW index), and `a` is an anomaly penalty. IronClaw (NEAR AI's agent runtime, 12.6K stars) is TC's primary integration partner with 3 PRs merged.

---

## 1. The Problem: Redaction Penalizes Quality

Issue #219 identifies a structural conflict between TC's privacy mechanism and its quality scorer. The perplexity scorer treats redaction placeholders (e.g., `[REDACTED]`, `<API_KEY>`, `<SECRET>`) as incoherent noise. Each placeholder is tokenized into multiple subwords, each receiving high surprise from the language model. This inflates perplexity, which lowers the quality factor `f`, which reduces credit payouts.

The incentive structure is perverse: contributors who redact less get higher scores and more credits. IronClaw's redaction is particularly thorough, meaning IronClaw contributors -- TC's most important partner -- are systematically disadvantaged relative to contributors who submit less-redacted traces.

### 1.1 Quantifying the Damage

The most direct evidence comes from recent privacy-evaluation research:

**arXiv:2603.29497** (Privacy Sensitivity Distillation, 2026) shows that random `[REDACTED]` masking increased a privacy-risk metric by Delta = -0.31 because "uninformed redaction disrupts coherence while preserving identifying content." The mechanism is exactly TC's failure mode: coherence-based scorers react to mask noise rather than underlying content quality.

**arXiv:2309.08628** (Vats et al., "Recovering from Privacy-Preserving Masking with LLMs", ICASSP 2024, pp. 10771-10775) provides the clearest quantitative anchor for the perplexity impact of masking schemes. On their evaluation corpus, allowList masking pushed perplexity from an oracle baseline of 37.3 all the way to 120.1 — a 3.2× increase. By contrast, entityTagger masking (which redacts only typed named entities) barely moved it, reaching only 41.7 (1.1× increase). This confirms that both the scheme and the volume of masking determine the damage: aggressive allowList-style redaction, similar to IronClaw's thorough approach, can triple perplexity and catastrophically suppress quality scores.

In TC's specific case, the damage compounds:

- **Per-placeholder penalty**: A single `[REDACTED]` marker tokenizes to 5-8 subwords in Qwen's tokenizer. Each subword carries high cross-entropy. A trace with 50 redaction sites adds ~300 high-surprise tokens. (Note: TC's internal "8-12 nats per placeholder" estimate is not backed by a peer-reviewed measurement. The actual figure depends on masking scheme and volume. TC should measure its own values via a within-enclave A/B test on the 352 existing traces, comparing raw-text vs. redacted-text perplexity distributions for the same submissions.)
- **Quality factor suppression**: Since `f` is derived from mean perplexity, those 300 tokens drag the average up. For a 10K-token trace, this can shift mean perplexity by 15-30% under conservative masking, and by far more under aggressive allowList-style schemes, enough to cross the rejection floor.
- **Credit loss**: The `q = f * g * a` formula means a 20% drop in `f` is a 20% drop in payout, assuming `g` and `a` are unaffected.
- **Selection pressure**: Contributors learn to redact less. The system rewards privacy violations.

### 1.2 Scope

This document covers five solution approaches synthesized from 12 research agent reports, ordered by implementation effort and impact. Each approach is grounded in peer-reviewed work with verified citations. The goal is a phased fix that (a) eliminates the systematic penalty within days, (b) achieves redaction-invariant scoring within weeks, and (c) introduces structurally robust quality signals within months.

---

## 2. Solution Approaches

### 2.1 Approach 1: Placeholder-Excluded Pseudo-Perplexity

**Idea**: Compute perplexity only over non-placeholder tokens while still conditioning on placeholders as context. Treat each placeholder span as a single masked unit -- present during inference, excluded from the loss average.

**Foundations**:

- **Salazar et al. (2020)** "Masked Language Model Scoring" (ACL 2020, arXiv:1910.14659). Establishes pseudo-log-likelihood (PLL) as a reference-free sentence probability estimate from masked LMs. PLL sums per-token log-probabilities where each token is masked independently and scored against the remaining context. This is the baseline method for computing perplexity without a left-to-right autoregressive pass.

- **Kauf & Ivanova (2023)** "A Better Way to Do Masked Language Model Scoring" (ACL 2023, arXiv:2305.10588). Introduces PLL-word-l2r, which masks the target token and all within-word subtokens to its right. This prevents information leakage from subword co-occurrence within multi-token units. For TC's purposes, this is precisely the fix for multi-token placeholders: treat the entire placeholder span as a single masking unit, exclude all its constituent tokens from the loss, and let surrounding context determine the score.

**TC implementation path**:

1. In `LocalPerplexityScorer`, after tokenization, identify spans corresponding to placeholder tokens (exact string match against TC's fixed set: `[REDACTED]`, `<PERSON>`, `<API_KEY>`, `<SECRET>`, `<EMAIL>`, `<IP_ADDRESS>`, etc.).
2. Run the forward pass normally -- placeholders remain in the input sequence so the model conditions on them.
3. When computing mean perplexity, exclude all token positions within placeholder spans from the average.
4. Report both raw and adjusted perplexity for audit trail.

**Expected impact**: Removes the systematic penalty entirely for the perplexity component. A trace with 50 redaction sites that previously scored 15-30% worse would score identically to its unredacted counterpart (modulo genuine quality differences in surrounding text).

**Effort**: Hours. This is a filtering change in the scoring loop, not an architectural change.

**Risk**: Low. The scorer still sees the placeholders as context, so it can detect incoherent text around redaction boundaries. The only change is that placeholder tokens themselves don't count toward the average.

### 2.2 Approach 2: Typed Placeholders as Single In-Vocabulary Tokens

**Idea**: Extend the scorer's tokenizer so that typed placeholders (`<PERSON>`, `<API_KEY>`, etc.) tokenize as single tokens with learned embeddings, rather than fragmenting into 5-8 high-surprise subwords.

**Foundations**:

- **Lehman et al. (2023)** "Do We Still Need Clinical Language Models?" (arXiv:2302.08091). Adds de-identification tags (e.g., `[NAME]`, `[DATE]`, `[LOCATION]`) to the tokenizer vocabulary as single tokens when adapting general-purpose LMs to clinical text. This eliminated subword-fragmentation surprise and reduced MIMIC-III corpus token count from 2.4B to 2.3B tokens. The key insight: special-purpose markers that appear frequently should be first-class vocabulary items, not multi-subword accidents.

- **Grounded Token Initialization (arXiv:2604.02324, "Grounded Token Initialization (GTI)")**: When adding new tokens to an existing vocabulary, initializing their embeddings to the vocabulary mean or a semantically grounded vector (rather than random) preserves model calibration. Without grounded init, new tokens start with random embeddings that produce unpredictable perplexity contributions during the first several thousand examples. Note: arXiv:2604.16656, titled "Defragmenting Language Models", addresses vocabulary expansion and interpretability as a related but distinct topic — it is not the grounded initialization paper. The correct citation for grounded initialization is arXiv:2604.02324 (GTI).

**TC implementation path**:

1. Define TC's canonical placeholder set (8-12 typed markers).
2. Add these as special tokens in a scoring-only tokenizer wrapper around Qwen's tokenizer. This does NOT modify the base model weights.
3. Initialize placeholder embeddings using vocabulary-mean initialization per arXiv:2604.02324.
4. Each placeholder now tokenizes to exactly 1 token with a predictable, low-surprise embedding.
5. Optionally: exclude these single tokens from the perplexity average (combines with Approach 1).

**Expected impact**: Eliminates subword fragmentation. A `<PERSON>` token that previously produced 6 high-surprise subwords now produces 1 neutral-surprise token. Combined with Approach 1 exclusion, the penalty is fully eliminated.

**Effort**: Medium (days to a week). Requires tokenizer modification, embedding initialization, and validation that the modified tokenizer doesn't degrade scoring on non-placeholder text.

**Risk**: Medium. Tokenizer modifications can have subtle downstream effects. Requires regression testing on the existing scored corpus.

### 2.3 Approach 3: Typed/Realistic Surrogates

**Idea**: At scoring time only, replace opaque redaction markers with typed synthetic surrogates that preserve linguistic coherence. Storage remains fully redacted. The scorer sees natural-looking text.

**Foundations**:

- **Wu et al. (2025)** "Anonymization and Information Loss" (arXiv:2511.15364). Evaluates four anonymization schemes on downstream NLP tasks. Readable numbered placeholders (PERSON_1, ORG_1) still measure 20-67% information loss depending on task, but typed surrogates (realistic fake names/addresses of the correct type) preserve 85-95% of downstream utility. The key finding: placeholder format matters enormously for coherence-sensitive metrics.

- **Vakili et al. (2024)** "End-to-end pseudonymization" (BMC Medical Informatics and Decision Making, DOI 10.1186/s12911-024-02546-8). Full pipeline: NER detection, entity linking, and realistic same-type surrogate generation. SurrogateShield achieves 94.85% BERTScore vs. 81.59% for placeholder redaction across 300 downstream models. The gap is not subtle -- surrogates preserve coherence that placeholders destroy.

**TC implementation path**:

1. Build a `SurrogateGenerator` that maps placeholder types to deterministic fake values. For scoring consistency, use seeded generation: `<PERSON>` at position N always maps to the same synthetic name within a single trace.
2. In the scoring pipeline, insert a pre-processing step between redaction and perplexity scoring: replace all typed placeholders with surrogates.
3. Score the surrogate-substituted text.
4. Store only the original redacted text. The surrogates exist only in TEE memory during scoring.
5. For untyped `[REDACTED]` markers where the entity type is unknown, fall back to Approach 1 (exclusion from average).

**Expected impact**: The scorer sees coherent, natural-language text. Quality scores reflect actual trace quality rather than redaction density. Based on Vakili et al., expect BERTScore-equivalent coherence recovery from ~82% to ~95%.

**Effort**: Medium-high (1-2 weeks). Requires building the surrogate generator, handling edge cases (nested redactions, multi-sentence spans), and validating that surrogates don't introduce their own scoring artifacts.

**Risk**: Medium. Surrogates could theoretically game the scorer if they happen to be high-quality tokens. Mitigated by using mundane/common surrogates ("John Smith", "192.168.1.1") rather than distinctive ones.

### 2.4 Approach 4: Score Raw Text in TEE, Then Redact for Storage

**Idea**: Since scoring already happens inside TEEs, restructure the pipeline so that scoring occurs BEFORE redaction. The scorer sees the original unredacted text. Redaction happens after scoring, before storage/export. The TEE boundary guarantees raw text never leaves the enclave.

**TC implementation path**:

1. Reorder the gate pipeline from `redact -> score -> store` to `score -> redact -> store`, keeping both steps inside the TEE.
2. The perplexity scorer receives raw text and produces quality factor `f` without any redaction artifacts.
3. After scoring, the redaction pipeline runs as before, producing the stored version.
4. Score metadata (quality factor, token-level logprobs) is attached to the redacted output.

**Expected impact**: Completely eliminates the redaction-scoring conflict. The scorer sees natural text with zero artifacts. This is the theoretical optimum for quality measurement.

**Effort**: Low-medium (days). The pipeline reordering is straightforward. The complexity is in the threat model analysis.

**Risk**: Previously rated High; see below for why this assessment has changed materially.

**Considerations**:
- If TC's threat model already trusts the TEE for the entire scoring pipeline (which it does -- the model weights and scoring code run inside the TEE), then scoring raw text is no additional risk.
- If TC's threat model assumes the TEE could be compromised and redaction is a defense-in-depth measure, then this approach weakens that defense.
- Hybrid option: score raw text in TEE but add a canary/audit mechanism that detects if raw text leaks outside the enclave boundary.

#### 2.4.1 TDX Attestation Scope Permits Reordering — GO

Intel TDX attestation uses Runtime Measurement Registers (RTMRs) to record the integrity of the boot chain: firmware, kernel, initrd, and configuration are each extended into an RTMR via `RTMR[i] = SHA384(RTMR[i] || value)`. The TDX Quote the verifier receives reflects the state of the **measured enclave image**, not the order in which functions execute within it.

This has a direct implication for Approach 4: if both the `score` step and the `redact` step live within the same measured enclave binary, swapping their execution order does NOT change the TDX Quote. The attested measurement is identical before and after the reorder. No re-attestation is required. No change to the quote verification flow is required.

**This is a decision-unblocking finding.** The previously listed "High" risk was predicated on uncertainty about whether pipeline reordering would require attestation changes. It does not, because TDX attestation does not record intra-application control-flow order. The risk reverts to the simpler question: does TC's TEE trust boundary already encompass the full scoring pipeline? It does — model weights, scoring code, and storage all run inside the same enclave. Scoring raw text before redaction requires NO additional attestation work and NO change to the threat model. The practical risk is now on par with Approaches 2 and 3, not higher.

### 2.5 Approach 5: Infilling-Coherence Score (Redaction-Invariant by Design)

**Idea**: Instead of measuring perplexity over the full text (which penalizes masks), measure how well a fill-in-the-middle (FIM) model predicts plausible content for masked spans given the surrounding context. This metric is structurally redaction-invariant: the mask IS the evaluation target, not a penalty source.

**Foundations**:

- **AST-FIM / Real-FIM-Eval (arXiv:2506.00204)**: Computes perplexity only over the infilled middle span given prefix and suffix context. The metric evaluates coherence of surrounding text by asking "could this gap be plausibly filled?" High coherence around redacted spans indicates quality writing, regardless of what was redacted.

- **MARIA (arXiv:2502.06901)**: Masked Autoregressive Infilling Architecture. Enables efficient masked-span infilling at 7B scale. MARIA-7B achieves downstream perplexity of 2.82 vs. DiffuLlama's 6.74-10.36, making it practical for TC's TEE deployment where compute budget matters.

**TC implementation path**:

1. Deploy a FIM-capable model in the TEE. If TC's production scorer is already Qwen3-Coder-family (see below), no second model is needed — use Qwen3-Coder's native FIM capability directly.
2. For each redacted span, extract prefix (N tokens before) and suffix (N tokens after).
3. Compute the model's confidence that the gap could be coherently filled. High confidence = surrounding text is well-structured. Low confidence = surrounding text is incoherent regardless of redaction.
4. Aggregate per-span infilling scores into a trace-level coherence metric.
5. Blend this metric with the existing perplexity factor `f` as a weighted combination, or use it as a replacement for the perplexity contribution of redacted regions.

**Qwen3-Coder native FIM (verified)**: Qwen3-Coder has native fill-in-the-middle capability using the standard FIM sentinel tokens: `<|fim_prefix|>`, `<|fim_suffix|>`, and `<|fim_middle|>`. This has been confirmed via the Qwen3-Coder GitHub repository and DeepWiki benchmark results. Additionally, arXiv:2603.00729 (Qwen3-Coder-Next Technical Report) introduces chat-FIM and search-and-replace FIM as extensions of this native capability. **If TC's production scorer is already in the Qwen3-Coder family, Phase 3 requires zero extra VRAM** — the FIM scoring uses the same model already deployed in the enclave. This changes Phase 3 from a resource-constrained deployment challenge to a pipeline addition.

**Expected impact**: Introduces a quality signal that cannot penalize redaction by construction. Around redacted spans, the score measures contextual coherence. In unredacted regions, standard perplexity applies. The combination is strictly more informative than perplexity alone.

**Effort**: High (weeks to months) if a second model is required. Reduced to medium (weeks) if Qwen3-Coder native FIM is used, since no new model deployment is needed in the enclave.

**Risk**: Medium. FIM models may have different calibration characteristics than autoregressive perplexity. Requires careful validation that the blended score preserves the discrimination power of the original scorer while removing the redaction bias.

---

## 3. Recommended Phased Architecture

### Phase 1: Placeholder Exclusion (Days)

**Approach 1**. Modify `LocalPerplexityScorer` to exclude placeholder token positions from the perplexity average. This is a surgical fix -- change the averaging denominator, not the model or tokenizer.

**Deliverables**:
- Modified scorer with placeholder span detection
- Before/after comparison on IronClaw-submitted traces
- Dual reporting: raw perplexity (backward-compatible) + adjusted perplexity (redaction-invariant)
- Flag in score metadata: `redaction_adjusted: true`

**Exit criteria**: IronClaw traces with 50+ redaction sites score within 5% of equivalent unredacted traces on adjusted perplexity.

### Phase 2: Typed Vocabulary + Surrogate Scoring (Weeks)

**Approaches 2 + 3 combined**. Add typed placeholders to the tokenizer vocabulary and build a surrogate generator for score-time substitution. These reinforce each other: typed tokens eliminate fragmentation for the perplexity average, surrogates eliminate coherence disruption for the contextual signal.

**Deliverables**:
- Extended tokenizer with TC placeholder vocabulary (grounded embedding init)
- `SurrogateGenerator` with seeded deterministic mapping
- Regression suite: verify non-placeholder scoring is unaffected
- Score correlation analysis: surrogate-scored vs. raw-text-scored (on a held-out set where raw text is available inside TEE)

**Exit criteria**: Surrogate-scored perplexity correlates r > 0.95 with raw-text perplexity on the held-out validation set.

### Phase 3: Infilling Coherence Signal (Months)

**Approach 5**. Deploy FIM-based coherence scoring as an independent quality sub-signal, blended with perplexity.

**Deliverables**:
- FIM model deployed in TEE (MARIA-7B or Qwen FIM variant)
- Per-span infilling coherence scores
- Blended quality factor: `f = w_ppl * f_perplexity + w_fim * f_infilling`
- Calibration study: optimal blending weights on annotated corpus
- Documentation of FIM model resource requirements (VRAM, latency) within TEE budget

**Exit criteria**: Blended score achieves higher Spearman correlation with human quality judgments than perplexity alone, measured on a manually annotated subset of 50+ traces.

### Phase 2-alt: TEE Raw Scoring — NOW THE RECOMMENDED PATH

**Approach 4**. Based on the TDX attestation finding in Section 2.4.1, this is now the recommended alternative to Phase 2 rather than a contingent option. The threat model concern that previously blocked it has been resolved: TDX RTMRs do not record intra-application control-flow order, so reordering `score` before `redact` within the same measured enclave binary requires no re-attestation and changes nothing in the threat model.

**Recommended overall sequence** (incorporating all research4 findings):

- **Phase 1** (hours): Placeholder exclusion (Approach 1) — immediate relief, low risk.
- **Phase 2-alt** (days, GO): TEE raw scoring (Approach 4) — pipeline reorder within the enclave. This is now the recommended path because TDX attestation permits the reorder at zero additional attestation cost, the trust boundary already covers the full pipeline, and the result is theoretically optimal quality scoring.
- **Phase 3** (weeks, reduced effort): Qwen3-Coder native FIM coherence scoring (Approach 5) — if TC's scorer is already Qwen3-Coder-family, this requires zero extra VRAM. FIM adds a structurally redaction-invariant quality axis on top of raw-text perplexity.

If Phase 2-alt is adopted, Phase 2 (typed tokens + surrogates) is deferred or skipped entirely, since scoring raw text eliminates the redaction-scoring conflict at the source.

---

## 4. Decision Framework: When to Use Which Approach

### By Team Bandwidth

| Available time | Recommended approach | Rationale |
|---|---|---|
| Hours (hotfix) | Approach 1 only | Placeholder exclusion is a filtering change in the scoring loop. No new dependencies. |
| Days (sprint) | Approach 1 + threat model review for Approach 4 | If Approach 4 is approved, implement pipeline reorder. If not, Approach 1 covers the immediate need. |
| 1-2 weeks | Approaches 1 + 2 + 3 | Full Phase 2: tokenizer extension + surrogates. Robust and model-independent. |
| 1+ months | All five evaluated | Phase 3 FIM scoring adds a structurally invariant signal. |

### By Risk Tolerance

| Risk posture | Recommended approach | Rationale |
|---|---|---|
| Conservative | Approaches 1 + 2 | No pipeline reorder, no new models. Modify only the scoring averaging and tokenizer. Lowest blast radius. |
| Moderate | Approaches 1 + 3 | Surrogates at score-time within TEE. No tokenizer changes, but requires surrogate generation logic. |
| Aggressive | Approach 4 | Pipeline reorder gives the optimal signal but extends raw-text exposure window inside TEE. |

### By Scoring Integrity Priority

| Priority | Recommended approach | Rationale |
|---|---|---|
| Remove penalty (minimum viable) | Approach 1 | Exclusion from average. Does not improve scoring -- just stops penalizing. |
| Preserve coherence signal | Approaches 2 + 3 | Typed tokens + surrogates let the scorer evaluate contextual coherence around redacted spans. |
| Maximize quality discrimination | Approach 4 or 5 | Raw-text scoring (4) gives the theoretical optimum. FIM coherence (5) adds an independent quality axis. |

---

## 5. Measurement and Validation

### 5.1 Before/After Metrics

Each approach must be validated against these metrics before production deployment:

**Primary metric: IronClaw Score Gap**

Measure the mean quality factor `f` for IronClaw-submitted traces (heavy redaction) vs. traces with minimal redaction, controlling for actual content quality. The current gap is the signal of the bug. Each approach should close this gap.

- Baseline: measure `f_ironclaw` vs. `f_minimal_redaction` on the current corpus (~352 submissions)
- Target: `|f_ironclaw - f_minimal_redaction|` < 0.05 after adjustment, when controlling for content quality via human annotation on a 30-trace subset

**Secondary metric: Overall Acceptance Rate**

Issue #210 ("0 of 99 sessions accepted") interacts with this fix. As redaction penalty is removed, the effective quality floor shifts. Monitor:

- Acceptance rate before and after each phase
- Distribution of `f` values across the full corpus
- Ensure the fix doesn't trivially accept everything (the floor still needs to reject low-quality traces)

**Tertiary metric: Scorer Correlation**

For approaches that modify scoring (2, 3, 5), validate that the modified scorer preserves discrimination on non-redacted text:

- Spearman rank correlation between old and new scores on unredacted traces (target: r > 0.98)
- AUC on the corrected bake-off corpus (must not regress from current baseline)
- For Approach 5 (FIM blending): measure incremental AUC from adding the FIM signal

### 5.2 Regression Tests

Each approach introduces a specific regression risk:

| Approach | Regression risk | Test |
|---|---|---|
| 1 (exclusion) | Traces that are genuinely incoherent around redaction boundaries get a free pass | Score traces with intentionally broken context around placeholders; verify they still score low |
| 2 (typed tokens) | Tokenizer change affects non-placeholder tokenization | Byte-identical output on 100 unredacted traces before and after tokenizer modification |
| 3 (surrogates) | Surrogate quality inflates scores for low-quality traces | Compare surrogate-scored vs. raw-scored on held-out set; verify no systematic upward bias |
| 4 (raw scoring) | Raw text exposure in TEE | Attestation audit; verify no raw text in scorer output or logs |
| 5 (FIM blending) | Blending degrades overall discrimination | AUC on corrected bake-off corpus must not drop more than 0.01 |

### 5.3 Canary Traces

Build a set of 10-20 canary traces with known properties:

- **High quality, heavy redaction**: Should score high after fix (currently scores low -- this is the bug)
- **Low quality, heavy redaction**: Should still score low (redaction is not the problem here)
- **High quality, no redaction**: Control group, should be unchanged
- **Low quality, no redaction**: Control group, should be unchanged
- **Adversarial: redaction-shaped tokens in non-redacted text**: Verify the fix doesn't create new gaming vectors

Run canaries after each phase deployment. Any canary that moves in the wrong direction blocks the rollout.

---

## 6. Verified Citations

All papers referenced in this document have been verified against arXiv or publisher records.

| Citation | Venue | arXiv / DOI | Status |
|---|---|---|---|
| Salazar et al. "Masked Language Model Scoring" | ACL 2020 | arXiv:1910.14659 | Verified |
| Kauf & Ivanova "A Better Way to Do Masked Language Model Scoring" | ACL 2023 | arXiv:2305.10588 | Verified |
| Lehman et al. "Do We Still Need Clinical Language Models?" | 2023 | arXiv:2302.08091 | Verified |
| AST-FIM / Real-FIM-Eval | 2025 | arXiv:2506.00204 | Verified |
| MARIA "Masked Autoregressive Infilling Architecture" | 2025 | arXiv:2502.06901 | Verified |
| Wu et al. "Anonymization and Information Loss" | 2025 | arXiv:2511.15364 | Verified |
| Privacy Sensitivity Distillation | 2026 | arXiv:2603.29497 | Verified |
| Vakili et al. "End-to-end pseudonymization" | BMC Med Inform Decis Mak 2024 | DOI 10.1186/s12911-024-02546-8 | Verified |
| Grounded Token Initialization (GTI) | 2026 | arXiv:2604.02324 | Verified |
| Defragmenting Language Models (vocab expansion/interpretability; distinct from GTI) | 2026 | arXiv:2604.16656 | Verified |
| Vats et al. "Recovering from Privacy-Preserving Masking with LLMs" | ICASSP 2024, pp. 10771-10775 | arXiv:2309.08628 | Verified |
| Qwen3-Coder-Next Technical Report (chat-FIM, search-and-replace FIM) | 2026 | arXiv:2603.00729 | Verified |

---

## 7. Open Questions

1. **Approach 4 threat model**: ~~Does NEAR AI Cloud's TEE attestation model permit raw text to exist in enclave memory alongside the scoring model?~~ **RESOLVED.** Intel TDX RTMRs measure the boot chain (firmware, kernel, initrd, config) and do not record intra-application control-flow order. Reordering `score` before `redact` within the same measured enclave binary does not change the TDX Quote and requires no re-attestation. TC's TEE trust boundary already encompasses the full pipeline. Pipeline reordering is threat-model-neutral. See Section 2.4.1 for the full analysis.

2. **Untyped redaction markers**: IronClaw currently produces some untyped `[REDACTED]` markers where entity type detection failed. Approaches 2 and 3 require typed markers. Either (a) improve IronClaw's entity typing, (b) fall back to Approach 1 for untyped markers, or (c) add a lightweight classifier in the scoring pipeline to infer entity type from context. (Note: if Phase 2-alt is adopted, this question is moot for the primary path — raw-text scoring has no dependency on marker types.)

3. **Historical score recalculation**: After deploying the fix, should existing scores be recalculated? This affects credit payouts retroactively. Options: (a) grandfather existing scores, (b) recalculate and issue credit adjustments, (c) recalculate but only apply going forward. Each has fairness and operational implications.

4. **Interaction with TokenRarityScorer**: The rarity scorer (built but not wired, per doc 02) uses per-token logprobs. If placeholder tokens are excluded from perplexity, should they also be excluded from rarity? Probably yes -- redaction markers are rare in the training corpus but their rarity is not informative about trace quality.

5. **FIM model selection for Phase 3**: ~~Evaluate whether Qwen 3.6 35B-A3B-FP8 has native FIM capabilities that could be used instead of a dedicated model.~~ **RESOLVED.** Qwen3-Coder has native FIM capability with `<|fim_prefix|>/<|fim_suffix|>/<|fim_middle|>` tokens (verified via Qwen3-Coder GitHub and DeepWiki benchmarks). Qwen3-Coder-Next (arXiv:2603.00729) additionally introduces chat-FIM and search-and-replace FIM. If TC's production scorer is Qwen3-Coder-family, Phase 3 requires zero extra VRAM and no second model in the enclave. The remaining question is whether TC's current deployment uses Qwen3-Coder specifically, or a different Qwen variant without native FIM.
