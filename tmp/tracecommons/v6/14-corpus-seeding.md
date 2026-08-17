# Corpus Seeding from Open Datasets

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates) that scores agent session traces for quality and novelty inside TEEs (Trusted Execution Environments), compensating contributors with NEAR blockchain credits. HNSW-based novelty index, BGE-large-en-v1.5 embeddings, ~352 submissions, 3 contributors, 6 GitHub stars. TC has a cold-start problem: 352 submissions and 3 contributors mean the novelty index is sparse (cosine distances are unreliable with few reference points), calibration data is insufficient for conformal gate thresholds (doc 09 needs 200+ traces per stratum), and the corpus cannot demonstrate value to potential contributors or consumers. A trace registry that cannot show what "good" looks like cannot convince anyone to submit.

---

## 0. The Problem

TC's scoring pipeline depends on comparison. Novelty is cosine distance against an HNSW index. Quality calibration requires distributional statistics. Gate thresholds (doc 09) need quantile estimates. All of these degrade or fail with a small corpus.

At 352 submissions:

- **Novelty scores are meaningless.** With few reference vectors, almost everything appears "novel" -- the index lacks the density to distinguish genuine novelty from corpus sparsity.
- **Calibration is impossible.** Conformal prediction (doc 09) requires exchangeable calibration sets of 200+ examples per stratum. TC has 352 total, unevenly distributed.
- **No demonstration corpus exists.** A potential contributor or data consumer visiting TC sees a near-empty registry. There is nothing to browse, benchmark against, or learn from.
- **The quality formula is uncalibrated.** `q = f * g * a` requires distributional priors for f, g, and a. With 352 samples, the priors are noise.

Seeding the corpus from existing open datasets addresses all four problems simultaneously.

---

## 1. Verified Datasets

### 1.1 Open-SWE-Traces

**Source**: NVIDIA, hosted at huggingface.co/datasets/nvidia/Open-SWE-Traces
**Paper**: arXiv:2606.16038
**License**: Permissive (MIT/Apache/BSD, inherited from source repositories)

- **207,489 trajectories** across 9 programming languages: Python, Go, TypeScript, JavaScript, Rust, Java, PHP, C, C++
- Sourced from **20,000 real pull requests** via OpenHands and SWE-agent frameworks
- Synthesized using Minimax-M2.5 (thinking model) and Qwen3.5-122B (non-thinking model)
- PII-filtered at source
- Best fine-tuned model achieves **61.7% on SWE-bench Verified**
- Traces include tool calls, file edits, test execution, and reasoning steps

**TC relevance**: Directly usable for corpus seeding. Includes Rust traces (TC's own language). The 207K trajectory count dwarfs TC's 352 submissions by 589x. Format includes agent session structure with tool invocations -- compatible with TC's trace envelope after transformation.

### 1.2 Nebius SWE-rebench-openhands-trajectories

**Source**: Nebius, hosted on Hugging Face
**License**: Permissive

- **67,074 open trajectories** from OpenHands agent runs
- 3x more successful attempts than alternative trajectory datasets
- Complementary to Open-SWE-Traces (different agent configurations, different problem distributions)

**TC relevance**: Supplements Open-SWE-Traces with additional trajectory diversity. The 3x success-rate advantage means more examples of traces that achieve their goals -- useful for calibrating what "good" looks like.

### 1.3 TraceLab

**Source**: Zhu et al. (University of Washington, arXiv:2606.30560)
**License**: Research use

- **4,265 sessions**, 357K LLM steps
- Metadata only -- no trace content (tool calls, reasoning, file edits are absent)
- Session-level metadata (duration, agent type, workload characteristics)

**TC relevance**: Useful for calibration statistics (session length distributions, workload complexity profiles) but not for seeding the novelty index (no embeddings can be generated from metadata alone). Supplements rather than replaces Open-SWE-Traces. Note: this is a workload characterization study of LLM serving demands; it does not report session failure rates.

### 1.4 Additional Datasets

5+ additional trajectory datasets were identified during the research sweep but were not fully characterized within the research budget. These include agent benchmark traces from academic papers, competition submissions, and smaller curated collections. A follow-up survey (doc 07, query candidates) could catalog these systematically.

---

## 2. HNSW Seeding Pipeline

### Step 1: Download and Filter

Download Open-SWE-Traces (207K trajectories). Filter for TC-compatible format: traces must contain agent session structure with tool calls (file reads/writes, shell commands, search operations). Discard metadata-only entries and traces shorter than TC's minimum session length.

Expected yield: ~180K-200K traces after filtering (most Open-SWE-Traces include tool calls by construction).

### Step 2: Format Transformation

Transform filtered traces into TC's trace envelope format. Map OpenHands/SWE-agent tool call schemas to TC's canonical tool vocabulary. Preserve:

- Agent reasoning/thinking blocks
- Tool invocations with arguments and results
- File modifications with diffs
- Test execution outputs
- Session-level metadata (language, repository, outcome)

### Step 3: Redaction Pipeline

Run all traces through TC's own redaction pipeline, even though Open-SWE-Traces is PII-filtered at source. Rationale:

- TC's redaction markers must be consistent across seed and organic data
- Source PII filtering may miss patterns TC's pipeline catches (API keys, internal URLs, custom PII patterns)
- Consistency ensures the perplexity scorer (and its Issue #219 fix) treats seed and organic data identically

### Step 4: Embedding Generation

Generate embeddings via BGE-large-en-v1.5 (TC's existing embedder). Chunk traces using TC's existing chunking strategy. Each chunk produces one embedding vector.

Expected output: ~1M-2M vectors (multiple chunks per trace, depending on session length).

### Step 5: HNSW Index Insertion

Insert vectors into the HNSW index (usearch) with seed-data labels. Every entry carries:

```
{
  "source": "seed",
  "dataset": "open-swe-traces",
  "dataset_version": "2026-06",
  "original_id": "<huggingface-row-id>",
  "language": "rust",
  "inserted_at": "2026-08-XX"
}
```

### Step 6: Calibration Score Generation

Run the full gate pipeline on seeded traces to produce calibration distributions for:

- Perplexity score distribution (per-language)
- Novelty score distribution (cosine distances within seed corpus)
- Quality formula output distribution (`q = f * g * a`)
- Token rarity distribution (if TokenRarityScorer is wired per doc 02, A.1)

These distributions feed directly into conformal gate threshold tuning (doc 09).

### Step 7: Metadata Tagging

Mark all seeded entries with `source=seed` metadata. This label is permanent and immutable -- it cannot be removed or reclassified. The label enables:

- Organic-vs-seed novelty scoring (Section 3)
- Transparency to contributors (Section 3)
- Audit trail for data provenance
- Clean removal if seed data is later superseded

---

## 3. Bias Mitigation

This is the central risk of corpus seeding. Seeded data biases the novelty detector: a trace that is genuinely novel but structurally resembles a seeded trace will score as "not novel." Since novelty directly affects credit payouts (`g` in `q = f * g * a`), this bias has financial consequences for contributors.

### 3.1 Per-Workload Namespaces

Seed data occupies a separate namespace within the HNSW index. Novelty is computed twice:

- **Seed-corpus novelty**: cosine distance against seed vectors only
- **Organic-corpus novelty**: cosine distance against contributor-submitted vectors only

Only organic-corpus novelty affects credit calculation. Seed-corpus novelty is recorded for analytics (a trace that is novel relative to organic but not relative to seed tells TC the organic corpus has a gap in that region).

Implementation: HNSW supports filtered search via metadata predicates. The `source` label enables namespace filtering without maintaining separate indices.

### 3.2 Temporal Downweighting

Seed data's influence on novelty scores decays over time as the organic corpus grows. The decay function:

```
seed_weight(t) = max(0.0, 1.0 - organic_count / (organic_count + half_life_constant))
```

Where `half_life_constant` controls the transition speed. When `organic_count = half_life_constant`, seed weight is 0.5. When `organic_count = 4 * half_life_constant`, seed weight is 0.2.

The half-life parameter should be set per language/domain stratum (Section 3.4), since organic submissions will arrive unevenly across categories.

### 3.3 Seed-Label Transparency

Every seed-derived entry is permanently labeled. Contributors can see comparison context:

```
Your trace was compared against:
  - 847 seed traces (weight: 0.12)
  - 2,341 organic traces (weight: 0.88)
  Novelty score: 0.73 (organic-only: 0.81)
```

This transparency prevents the perception that TC is gaming scores against an opaque baseline. Contributors can see exactly how seed data affected their score and verify that organic-only novelty is the credit-relevant metric.

### 3.4 Language and Domain Stratification

Open-SWE-Traces is Python-heavy (Python dominates SWE-bench). Rust traces are a minority. Inserting the full dataset without stratification would over-represent Python in the index, making Python traces appear less novel relative to Rust traces purely due to reference density.

Mitigations:

- **Cap per-language seed count.** Set a maximum seed trace count per language (e.g., 10K per language). Downsample over-represented languages randomly.
- **Track per-language density.** Dashboard metric showing seed-vs-organic density per language. Alert when any language's seed-to-organic ratio exceeds 100:1 (the novelty signal for that language is dominated by seed data).
- **Prioritize Rust seeds.** TC is a Rust project attracting Rust developers. Ensure Rust traces are well-represented in the seed corpus despite their minority status in Open-SWE-Traces.

---

## 4. Three-Phase Transition Strategy

### Phase 1: Pure Seed (Weeks)

- Load Open-SWE-Traces into HNSW index (stratified per Section 3.4)
- Use seed corpus as the calibration set for conformal gate thresholds (doc 09)
- Gate thresholds set against seed distribution
- No organic-vs-seed distinction in scoring yet (too few organic traces for meaningful separation)
- All incoming organic traces scored against the full index (seed + organic combined)
- Primary goal: give the gate pipeline enough reference data to produce non-degenerate scores

**Exit criterion**: 500+ organic traces submitted (enough for initial organic-only statistics).

### Phase 2: Hybrid (1-3 Months)

- Organic corpus grows alongside seed corpus
- Novelty scored against both namespaces, weighted toward organic (Section 3.2)
- Seed influence begins temporal decay
- Calibration set updated: organic traces replace seed traces in calibration strata where organic count exceeds minimum (200 per stratum)
- Per-language dashboards show seed-vs-organic density
- Contributors see transparency breakdown (Section 3.3)

**Exit criterion**: Organic corpus exceeds 5,000 traces across at least 3 languages.

### Phase 3: Organic-Majority (3-6 Months)

- Organic corpus exceeds seed corpus in all active language categories
- Seed data used only for cold-start in new language/domain categories (e.g., if TC adds PHP support and has zero organic PHP traces)
- Full transition to organic-only calibration
- Seed data archived but retained for reproducibility and audit
- Seed weight decays below 0.05 for all established categories

**Exit criterion**: Seed weight below 0.05 in all categories with 1,000+ organic traces.

---

## 5. Privacy and Licensing

### 5.1 Source Data Licensing

Open-SWE-Traces inherits permissive licenses (MIT/Apache/BSD) from its source repositories. This is compatible with TC's dual-license (MIT/Apache-2.0). Nebius trajectories are similarly permissive. No copyleft contamination risk.

### 5.2 Redaction

Open-SWE-Traces is PII-filtered at source, but TC must run its own redaction pipeline regardless (Section 2, Step 3). This ensures:

- Consistency of redaction markers across the entire corpus
- Defense in depth against PII that source filtering missed
- Compliance with TC's own privacy guarantees (TEE-based scoring sees consistent input)

### 5.3 Attribution

Credit seed data sources in TC's corpus metadata. Every seed entry links back to its source dataset and original identifier. TC's public corpus browser (if built) displays seed provenance clearly.

### 5.4 No Credits for Seed Data

Seeded traces are not contributor submissions. No NEAR credits are minted, allocated, or paid for seed data. The credit formula applies only to organic submissions. This is enforced by the `source=seed` metadata label: the credit pipeline skips entries where `source != "organic"`.

---

## 6. Impact on Other v6 Documents

| Document | Impact |
|---|---|
| **Doc 02 (Scoring Pipeline)** | Seed data enables the bake-off corpus rebuild (A.3). The 207K traces provide stratified examples across languages, success/failure, and session length -- exactly what the fixed corpus needs. |
| **Doc 09 (Conformal Gates)** | Seed data provides the calibration set needed for quantile thresholds. 200+ traces per stratum becomes feasible immediately (Phase 1) instead of waiting months for organic growth. |
| **Doc 10 (Ground-Truth-Free Quality)** | Seed data enables the 200+ traces needed for judge-aware BTL ranking. Multiple traces per problem (Open-SWE-Traces has multiple agent attempts per PR) support pairwise comparison. |
| **Doc 12 (Trajectory RAG)** | Seed corpus bootstraps the retrieval index. Contributors querying "how did other agents handle X?" get results from day one instead of empty results. |

---

## 7. Implementation Effort

| Step | Effort | Dependencies |
|---|---|---|
| Download + filter Open-SWE-Traces | Hours | HuggingFace access, disk space (~50GB raw) |
| Format transformation script | 1-2 days | TC trace envelope schema documentation |
| Redaction pipeline run | Hours (batch) | TC redaction pipeline working (assumed) |
| Embedding generation | 1-2 days (GPU) | BGE-large-en-v1.5 model, GPU access |
| HNSW insertion + metadata | Hours | usearch index, `source` label support |
| Calibration score generation | Hours (batch) | Full gate pipeline functional (blocked by #210) |
| Namespace filtering in novelty scorer | 1-2 days | HNSW metadata filter support |
| Temporal downweighting | 1 day | Novelty scorer modification |
| Transparency display | 1 day | Contributor-facing API/UI |
| **Total** | **~1-2 weeks** | **Blocked by Issue #210 for calibration step** |

---

## 8. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Seed data dominates novelty scoring permanently | Medium | High (genuine novelty suppressed) | Per-workload namespaces + temporal decay (Section 3.1, 3.2) |
| Python over-representation skews cross-language scores | High | Medium | Language stratification caps (Section 3.4) |
| Open-SWE-Traces format incompatible with TC envelope | Low | Medium (delays seeding) | Format transformation step (Section 2, Step 2) |
| PII leakage from source data despite filtering | Low | High | TC's own redaction pipeline as defense in depth (Section 5.2) |
| Contributors perceive seed data as unfair baseline | Medium | Medium (churn) | Transparency display + organic-only credit scoring (Sections 3.3, 5.4) |
| Seed data becomes stale as agent capabilities evolve | High | Low (Phase 3 archives) | Temporal decay + eventual archival (Section 4, Phase 3) |

---

## 9. Open Questions

1. **What fraction of Open-SWE-Traces includes Rust?** The dataset covers 9 languages but SWE-bench is Python-dominated. Actual Rust trace count may be small. Needs empirical check before assuming Rust coverage is sufficient.

2. **Should seed data be publicly browsable?** Making seed traces visible in TC's corpus browser demonstrates corpus depth but may confuse contributors who see "207K traces" and wonder why they should contribute. Alternative: show seed count separately with a "bootstrapping" label.

3. **What is the right half-life constant?** Too fast and the seed data stops being useful before organic corpus is dense enough. Too slow and seed data dominates for months. Needs tuning against organic submission rate (currently ~13/week).

4. **Should Nebius trajectories be seeded alongside Open-SWE-Traces?** Adding 67K more trajectories increases density but also increases bias surface. Phased approach: seed Open-SWE-Traces first, evaluate, then add Nebius if density gaps remain.

5. **Can TraceLab metadata inform calibration even without content?** Session-level statistics (duration, agent type, workload complexity) from 4,265 sessions (Zhu et al., University of Washington, arXiv:2606.30560) could validate TC's calibration distributions without contributing to the novelty index.

---

## 10. Verification Ledger

| Item | Source | Status |
|---|---|---|
| Open-SWE-Traces (207,489 trajectories, 9 languages) | arXiv:2606.16038 | **Verified** -- paper confirms 207,489 trajectories, Minimax-M2.5 + Qwen3.5-122B synthesis, MIT/Apache/BSD licensing, PII filtering |
| Open-SWE-Traces best model 61.7% SWE-bench Verified | arXiv:2606.16038 | **Verified** |
| Open-SWE-Traces hosted on HuggingFace | huggingface.co/datasets/nvidia/Open-SWE-Traces | **Verified** |
| Nebius SWE-rebench-openhands-trajectories (67,074) | HuggingFace (Nebius) | **Verified** -- dataset listing confirms trajectory count |
| Nebius 3x success rate vs alternatives | HuggingFace dataset description | **Verified** |
| TraceLab (4,265 sessions, 357K LLM steps, metadata only) -- Zhu et al., University of Washington, arXiv:2606.30560 | arXiv:2606.30560 | **Verified** -- workload characterization study; metadata-only format confirmed; does NOT report a failure rate |
| 5+ additional trajectory datasets exist | Research sweep | **Partially verified** -- datasets identified but not fully characterized |
| BGE-large-en-v1.5 is TC's existing embedder | TC codebase | **Verified** -- confirmed in gate pipeline |
| usearch HNSW supports metadata filtering | usearch documentation | **Verified** |
| TC credit formula `q = f * g * a` | TC codebase + doc 02 | **Verified** |
