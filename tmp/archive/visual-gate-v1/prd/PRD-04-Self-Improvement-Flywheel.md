# PRD-04: Self-Improvement Flywheel, Autonomous Eval Generation, and Benchmark Awareness

**Prerequisites**: PRD-00 through PRD-03.

---

## 1. Overview

This document specifies how UiGate turns every run into permanent capital. Seven flywheel steps, the autonomous eval generation architecture (five mechanisms from doc-17 adapted for UI), curriculum-from-failures synthesis, the RFT post-processor strategy, benchmark awareness and integration, and the timeline for building compounding assets.

The core thesis: **spend fine-tune budget on the post-fixer first, not the judge.** Repair tasks have crisper signal (broken→working is binary-ish), more abundant data (every error log is training), and savings compound on every subsequent generation. The judge stays frontier-API for 12+ months.

---

## 2. The Seven-Step Flywheel

Every UiGate run must emit labeled, structured, retrievable artifacts, or the system doesn't compound. These seven steps run after every gate execution.

### Step 1: Trace Capture (via TensorZero)

Store the complete execution trace:
- Prompt sent to generator agent
- Retrieved components/context used
- All rollout variants considered
- Bandit-arm chosen (model, retrieval-k, prompt template)
- AST of final generated output
- All screenshots captured
- Compile/render telemetry (build time, error count)
- User edits/keeps/discards (when human reviews)
- Time-to-accept (if applicable)
- Full computational metrics (all 15)
- Hard gate results
- Judge panel results with individual verdicts

**Format**: TensorZero-compatible trace JSON. Enables downstream preference mining and bandit optimization.

### Step 2: Auto-Grade

Grade using the disjoint-family panel (PRD-03) plus deterministic linters plus screenshot-diff vs nearest neighbor in the success bank.

- **Disagreement triggers human review.** If 2 of 3 panel judges disagree with the aggregate, flag for human inspection.
- **Pairwise vs previous-best, never absolute.** Every grading is a comparison against the fixed anchor.
- **Success bank**: Maintained collection of human-approved screenshots indexed by task type, viewport, and component category.

### Step 3: Pairwise Preference Mining

Every time a preference signal is emitted, log it:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceTriple {
    pub prompt: String,          // task description
    pub candidate_a: String,     // path to screenshot A
    pub candidate_b: String,     // path to screenshot B
    pub preferred: String,       // "A", "B", or "tie"
    pub source: String,          // "judge_panel", "user_edit", "user_select"
    pub timestamp: String,
    pub task_id: String,
    pub viewport: String,
    pub metadata: serde_json::Value,
}
```

Sources of preference signals:
- **Judge panel verdicts**: candidate vs anchor comparison → preference triple.
- **User edits**: post-edit version is positive, pre-edit version is near-positive negative. This is the strongest signal.
- **User variant selection**: if the system presents multiple variants (à la 21st.dev's 4-variant UI), the selection is a preference.

This collection IS your private WebDev Arena. **WebDev Arena's `lmarena-ai/webdev-arena-preference-10k`** (commercial-use licensed) is free pretraining corpus for bootstrapping before own data accumulates.

### Step 4: Pattern Extraction via AST Diffing (Nightly)

Run nightly batch job:

1. **Cluster successful components** by (Tailwind-class bag, DOM shape, prop signature). Emit cluster centroids as templated entries in a private pattern library.
2. **Diff successful-vs-rejected pairs** to extract anti-patterns. What CSS patterns appear in rejected but not approved screenshots?
3. **Feed both as positive/negative few-shots** through DSPy `BootstrapFewShotWithRandomSearch`.
4. **Build private component library** — the 21st.dev pattern. RAG over juried inventory that grows with approved submissions.

### Step 5: Curriculum-from-Failures (the WebRL Move)

**Source**: WebRL (Qi et al., arXiv 2411.02337).

1. **Cluster failed runs** by judge-rationale embedding. Each failure has textual rationale from the judge panel — embed these and cluster.
2. **Each cluster spawns synthetic training tasks.** E.g., "pricing card with 3 tiers and footnote disclaimer" (from a cluster of pricing page failures), "dashboard with 4 KPI cards + sparkline" (from dashboard failures).
3. **Schedule retries** on improved pipeline.
4. **Promote successes** into the regression eval set.

This ensures the system continuously generates new training tasks from its own weaknesses, dynamically adjusting difficulty to current skill level.

### Step 6: MIPROv2 Optimization (Weekly)

**Source**: MIPROv2 (Opsahl-Ong et al., EMNLP 2024).

Optimize over the right surface — NOT the base generator prompt:

| Optimize | Why |
|---|---|
| Retrieval queries for fetching components | Better context = better generation |
| Clarifying-question prompt | Subframe pattern: cheapest improvement |
| Judge rubrics | Keeps judges calibrated |
| AutoFix repair prompts | Better feedback = faster convergence |

**Do NOT optimize the base generator prompt directly.** Too much capacity, low signal-per-call.

Additionally, run **bandits over discrete pipeline arms**:
- Model choice (Opus vs Sonnet vs Haiku)
- Retrieval-k (how many reference components to include)
- With/without clarifying turn
- With/without post-fixer

### Step 7: Periodic RFT of Post-Processor (Month 7+)

**Source**: v0's `vercel-autofixer-01` (Fireworks RFT, replacing Gemini Flash 2.0).

Once ≥10k labeled `(broken-output, fixed-output)` pairs accumulate from steps 2 and 4:

1. RFT a Llama-3.1-8B or Qwen2.5-Coder-7B on Fireworks/Together as `uigate-autofixer-01`.
2. This model takes a failing screenshot + error evidence and produces a code patch.
3. **This is the single asset that actually compounds.** Every fix it learns reduces frontier API spend forever and survives model swaps.

**Why post-fixer before judge**: Repair tasks have crisper signal (broken→working is binary). More abundant data (every error log is training data). Savings compound on every subsequent generation.

**Judge stays frontier-API for 12+ months.** APIs already hit ~70–80% human agreement on pairwise UI judgments. Cost is real (~$0.05–0.20/judgment) but judging is parallelizable and infrequent compared to generation.

**One exception — train a tiny router classifier**: DistilBERT-class on ~1k UICrit + own-data pairs. Gates: "is this prompt likely to need clarification?" or "is this likely to need a chart library?" Cheap, high-signal, low-stakes.

---

## 3. Autonomous Eval Generation (Adapted from Doc-17)

Doc-17 describes five mechanisms for autonomous eval generation on the Agent Coordination Chain. These adapt to UiGate for UI verification.

### 3.1 Mechanism 1: Property-Based Eval Generation

**Source**: Echidna (Grieco et al., ISSTA 2020), ContractFuzzer (Jiang et al., ASE 2018), SynTest-Solidity (Tanida et al., ICSE 2022).

**Adaptation for UI**: Instead of smart contract ABIs, mine component interfaces and design system specifications.

- Extract component props/types from TypeScript definitions.
- Generate property assertions: "Button with variant='primary' must have background-color matching design token `color.action.primary`."
- Generate fuzz variants: random props, edge-case content (very long text, empty string, special characters), extreme viewport sizes.
- Use **Benchmark Self-Evolving** (Wang et al., COLING 2025) six reframing operations to expand each assertion into 6+ variants.

**Self-curation**: Properties that frequently distinguish good from bad implementations (high pass-rate variance across agents) gain weight. Properties that always pass or never compile are pruned.

### 3.2 Mechanism 2: Scenario Mining from Production

**Source**: SCONE-bench (Anthropic 2025), EVMbench (Wang et al. 2026).

**Adaptation for UI**: Instead of chain history, mine production UI deployments and design system updates.

- **Regression replay**: When a production bug is reported, capture the before/after state as a test scenario. "Can the agent avoid this layout bug?"
- **New component analysis**: When a new design system component is added, automatically generate UI test scenarios exercising it.
- **A/B test mining**: When A/B tests produce clear winners, the winning variant becomes a reference and the losing variant becomes a negative example.

Per **Re-Evaluating EVMBench** (Storhaug & Meling 2026): curated benchmarks overestimate real-world capability. Continuously mine NEW scenarios from real deployments.

### 3.3 Mechanism 3: Predictive Foraging as Evaluation

**Source**: Friston's Free Energy Principle (Nature Reviews Neuroscience, 2010), Millidge et al. "Whence the Expected Free Energy?" (Neural Computation, 2021).

**Adaptation for UI**: Before implementing, the agent predicts: "This modal will render at ~400px wide on mobile, fit within viewport, and the submit button will be visible without scrolling." After implementation, compare prediction to reality.

The prediction-outcome gap (the **residual**) IS an evaluation metric:
- **Gas cost equivalent**: predicted vs actual render time, bundle size.
- **Success/revert equivalent**: journey passes or fails.
- **State change equivalent**: computed styles match predictions.

Per Millidge et al.: expected free energy decomposes into pragmatic value (goal achievement) and epistemic value (information gain). The PF residual captures both "did the agent succeed?" and "did the agent learn?"

### 3.4 Mechanism 4: Red Team Agents

**Source**: AutoRedTeamer (Liu et al., 2025), HarmBench (Mazeika et al., ICML 2024), AGENTPOISON (NeurIPS 2024).

**Adaptation for UI**: Specialized agents that attempt to find UI failures the system currently misses.

1. **Select target**: A highly-rated component or layout pattern.
2. **Generate hypothesis**: "This component breaks with RTL text" or "This layout overflows with content longer than 200 characters."
3. **Construct test**: Build a concrete scenario exercising the hypothesis.
4. **Execute and evaluate**: Run the scenario through UiGate.
5. **If counterexample succeeds**: Red team agent earns reward; the scenario joins the eval suite; the original pattern is annotated.
6. **If counterexample fails**: The pattern is strengthened; red team agent pays the cost.

**AGENTPOISON relevance**: Poisoning <0.1% of knowledge base achieves 63% attack success. This means the playbook library, pattern library, and judge prompts are attack surfaces. Red team agents should specifically test these.

**WASP relevance**: Prompt injection against web agents is real (16–86% execution rate). If UiGate's runner browses external sites for design references, isolate browsing from code-writing and treat all external pages as untrusted.

### 3.5 Mechanism 5: Meta-Loop Optimization (Autoresearch Pattern)

**Source**: Karpathy autoresearch (March 2026), DSPy/MIPROv2, TensorZero, MASPOB.

The autoresearch pattern applied to UiGate's pipeline configuration:

```
prepare.py equivalent:
  - Eval suite from Mechanisms 1-4
  - 500+ property assertions
  - 1000+ historical scenarios
  - 200+ adversarial edge cases
  - Fixed. Never modified by the optimizer.

train.py equivalent:
  - UiGateConfig parameters (thresholds, weights, model selection)
  - The ONLY thing the optimizer changes

Loop:
  1. Propose config change (e.g., increase APCA threshold from 60→65)
  2. Run 100 eval scenarios with new config
  3. Measure: task_success_rate, false_positive_rate, false_negative_rate, cost
  4. Compare against baseline
  5. Keep if improved, discard if not
  6. Log experiment
  7. Repeat
```

**BetterTogether** (Soylu et al., EMNLP 2024) backing: alternating prompt optimization and weight fine-tuning achieves up to 60% gains over weight-only optimization. This validates the two-phase strategy: prompt optimization months 1–6, then weight fine-tuning (RFT post-processor) month 7+.

---

## 4. Anti-Goodhart Safeguards

Six operational tactics against the four Goodhart types (Manheim & Garrabrant 2018):

### 4.1 Disjoint-Family Panel with Trimmed-Mean

Inner-loop bandit reward and held-out validation never share a judge. The panel's diversity prevents any single model's biases from dominating.

### 4.2 Frozen Human-Rated Canary Set

200–500 prompts with UICrit-style ratings (Krippendorff α ≥ 0.8). Re-evaluated every release. If panel score rises but canary doesn't → Goodharting detected.

### 4.3 Eval Diversity with Quarterly Rubric Rotation

Goodhart relies on a stable target. Rotate rubric emphasis quarterly. Per **LiveBench** (White et al., ICLR 2025): monthly updates with fresh questions prevent contamination.

### 4.4 Adversarial Red-Team Eval

Run UIClip + LAION + "trivially gameable patterns" detector on the bandit's top-K outputs. Patterns to detect:
- Gradient bombs (excessive gradients to boost colorfulness)
- Oversaturated hero images
- Blur-everything (reduces perceived complexity)
- Copy-paste of approved screenshots with minor modifications

### 4.5 Reference-Conditioned BT Against Strong Baseline

Always compare pairwise against a strong baseline, not absolute scoring. BT against anchor is inherently harder to inflate.

### 4.6 Canary Correlation Monitor

Track Spearman correlation between cheap inner-loop judge and expensive canary. If ρ drops below ~0.6, the inner judge has drifted → retrain or replace.

### 4.7 Specific Countermeasures by Goodhart Type

| Type | Risk | Countermeasure |
|---|---|---|
| Regressional | Optimizing on AIM "balance" noise | Require improvements on ≥3 orthogonal metrics simultaneously |
| Extremal | Perfect symmetry = sterile UIs, max contrast everywhere = harsh | Clip rewards into bands rather than maximizing. AIM metrics validated in [normal, normal-bad] not pushed to perfect. |
| Causal | High WCAG ratio doesn't cause readability if font too thin | Use APCA (models font-weight) alongside WCAG, not instead |
| Adversarial | Agent imitates judge panel | Rotating eval suites, holdouts, meta-eval. Krakovna list as CI red-team prompt set. |

---

## 5. Benchmark Awareness and Integration

### 5.1 Three Benchmark Traditions

The field has bifurcated (per compass artifact):

1. **Reference-matching** (Pix2Code, WebSight, Design2Code, WebCode2M, DesignBench): "does the rendered HTML match a target screenshot?"
2. **Agentic/functional** (WebShop, Mind2Web, WebArena, VisualWebArena, WebLINX, ScreenSpot-V2, VisualWebBench, AITW, WebGen-Bench): "did the agent click the right button?"
3. **Aesthetic/preference** (WebDev Arena, Design Arena, UICrit): "do humans prefer this UI?"

UiGate cares about all three but especially the third, where rigorous benchmarks barely exist.

### 5.2 Benchmarks to Integrate

| Benchmark | How to Use |
|---|---|
| Design2Code-HARD (80 pages) | Held-out eval set. Compute Block-Match+Position+Text geometric mean as floor. |
| UICrit (1000 UIs, CC-BY 4.0) | Judge fine-tuning corpus. Annotation schema template. |
| WebGen-Bench (101 instructions, 647 GUI tests) | Closest to UiGate's use case. External validation. |
| WebDev Arena preference-10k | Free pretraining corpus for BT judge (commercial-use licensed). |
| VisualWebArena | Adversarial stressor — test if generated UIs are navigable by agents. |
| ScreenSpot-V2 | Grounding accuracy check — can agents find buttons in generated UIs? |
| 1D-Bench | Validates that rewards should be dense, localized, and per-failure-class. |
| FrontendBench | Interactive test scenario validation. |

### 5.3 Internal Benchmark Construction

Build a growing internal benchmark from:
- Every human-labeled UI gate run (accept/reject).
- Every fix that resolved a UI gate failure (before/after pairs).
- Synthetic tasks from curriculum-from-failures (Step 5 above).
- Red team counterexamples that succeeded.

Per **LiveCodeBench** (Jain et al., 2024): continuously collect new problems. Do not rely on a fixed set.

---

## 6. Products to Learn From

| Product | What to Steal |
|---|---|
| **v0** (Vercel) | Composite stack: frontier base + RAG + RFT post-fixer. The architectural template. |
| **21st.dev** | Curated community library (RAG over juried inventory that grows with submissions). Compounding-asset pattern. |
| **Subframe** | Clarifying-question step before generation. Cheapest MIPROv2-style improvement. |
| **Builder.io Mitosis** | IR layer: optimize once, emit many target frameworks. |
| **Builder.io component-mapping** | RAG over repo's actual components. Private design-system enforcement. Clone the pattern. |
| **Locofy** | Large Design Model: heuristics beat LLMs on deterministic parts. Validates deterministic-first design. |
| **WebSight** | Hard lesson: validation loss ≠ generation quality. Always rely on rendered-screenshot eval. |

**Ignore**: Lovable/Bolt "agent" framing — public reports of regression-loops suggest no compounding asset being created.

---

## 7. Timeline

### Months 1–2: Deterministic Verifier Core

- Browser runner (chromiumoxide or Playwright MVP)
- Eight-loop sensor harness emitting unified JSON
- Tiers 1–3 hard gates (axe + IBM + LHCI)
- odiff visual regression with reg-suit-compatible output
- Project Wallace + custom analyzer → W3C 2025.10 tokens
- Area-weighted token adherence score
- **No LLM judge yet.** Deterministic verifier core before any subjective scoring.

### Months 3–4: Judge Panel + Advanced Metrics

- Disjoint-family panel (Claude Opus + LLaVA-Critic + Prometheus-Vision)
- Pairwise BT aggregation
- Design2Code-style Block-Match/Position/Text floor metrics
- AIM layout-metric soft loop
- DeepGaze IIE + UMSI++ saliency loop
- Held-out human canary set with Krippendorff α ≥ 0.8 monitoring
- Tier 4–5 APCA + motion gates as warnings (promote to hard fail after 2 weeks)

### Months 5–6: Flywheel

- TensorZero traces feeding pairwise preference mining
- AST pattern extraction (nightly)
- MIPROv2 over retrieval queries, judge rubrics, clarifying prompts
- Bandit routing over pipeline arms
- Curriculum-from-failures synthesis

### Month 7+: Compounding Assets

- RFT `uigate-autofixer-01` post-processor (once ≥10k repair pairs)
- Custom judge training deferred until frontier-judge cost is binding constraint or domain quirk emerges
- DistilBERT router classifier for prompt routing

---

## 8. Learning Event Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiLearningEvent {
    pub event_type: String,
    pub timestamp: String,
    pub task_id: String,
    pub plan_id: String,
    pub attempt: u32,
    pub passed: bool,
    pub bt_score: Option<f64>,
    pub rubric_scores: Option<RubricScores>,
    pub computational_metrics_summary: Option<ComputationalMetricsSummary>,
    pub hard_failure_count: u32,
    pub soft_finding_count: u32,
    pub failure_classes: Vec<String>,
    pub duration_ms: u64,
    pub implementer_model: String,
    pub evaluator_models: Vec<String>,
    pub prompt_variant: String,
    pub pipeline_arm: String,       // bandit arm chosen
    pub cost_usd: f64,
    pub token_adherence: Option<f64>,
    pub visual_regression_diff: Option<f64>,
    pub viewport_count: u32,
    pub journey_count: u32,
    pub artifact_dir: String,
    pub preference_triples_emitted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationalMetricsSummary {
    pub metrics_passed: u32,
    pub metrics_total: u32,
    pub worst_metric: String,
    pub worst_value: f64,
    pub worst_threshold: f64,
}
```

Written to `.roko/learn/ui-events.jsonl`.
