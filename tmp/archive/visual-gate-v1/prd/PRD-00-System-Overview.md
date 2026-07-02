# PRD-00: Visual Gate System — Architecture, Research Foundations, and Design Rationale

**Status**: Draft — Complete Rewrite  
**Audience**: Implementation engineer with zero prior Roko context  
**Scope**: Self-contained system overview covering architecture, every cited research paper, every benchmark, every evaluation methodology, and the design rationale connecting them  

---

## 1. Document Set

| Doc | Covers | Approx Length |
|---|---|---|
| **PRD-00** (this) | Architecture, research foundations, design philosophy, all citations | Long |
| **PRD-01** | Complete data model: every Rust struct, JSON schema, TOML config, assertion type, metric type | Long |
| **PRD-02** | Browser runner, evidence collection, 15 computational metrics engine, APCA, AIM, saliency, design-token extraction | Long |
| **PRD-03** | Judging methodology: pairwise Bradley–Terry, disjoint panels, G-Eval, UICrit schema, Krippendorff monitoring, statistical rigor | Long |
| **PRD-04** | Self-improvement flywheel: seven-step capital accumulation, WebRL curriculum, RFT post-processor, benchmark awareness, autonomous eval generation | Long |
| **PRD-05** | Orchestration wiring, eight disjoint cybernetic loops, conjunctive/Pareto gate composition, task breakdown, rollout, acceptance criteria | Long |

---

## 2. The Problem

Roko is a Rust toolkit (~18 crates, ~177K lines) for building AI agents that build software. Its loop: take a PRD → decompose into tasks → dispatch coding agents → verify through gates → feed failures back as retries → persist everything as immutable signals → learn from outcomes. It already verifies code correctness through compile, lint, test, integration, and LLM-judge gates.

None of those gates can verify that a UI works or looks right. A React component can compile, pass TypeScript checks, satisfy every unit test, and still be unusable: a button clipped on mobile, a modal overflowing its container, a route throwing console errors, a form submission returning HTTP 500, a hydration error at runtime, a layout that renders but looks nothing like what was requested, an interactive element unreachable by keyboard.

The gap is not "take a screenshot." The gap is: **turn browser-observed reality into a structured gate verdict and a repair loop.**

---

## 3. The Solution: UiGate

A new gate — `UiGate` — that:

1. Reads UI verification requirements from a task definition.
2. Starts or connects to the frontend application.
3. Runs a browser runner (Playwright with real Chromium, or chromiumoxide for Rust-native CDP) against the rendered UI.
4. The runner executes user journeys (click, fill, wait, assert), captures screenshots, traces, console logs, network requests, accessibility snapshots, layout metrics, and performance data.
5. Deterministic hard gates run first: structural integrity (a11y critical/serious via axe-core + IBM Equal Access), functional correctness (journey completion, no console errors, no failed requests), Core Web Vitals, APCA perceptual contrast, reduced-motion compliance.
6. Computational metrics run: 15 quantitative measures including AIM grid quality, saliency scoring, colorfulness, element density, token adherence, visual balance.
7. A disjoint-family judge panel (3–4 models from different families) evaluates screenshots via pairwise Bradley–Terry comparison against a fixed anchor, using UICrit-style annotation schema.
8. Gate composition: conjunctive on hard gates (all must pass independently), Pareto frontier on soft gates (no single score compensates for another dimension's failure).
9. If the gate fails, structured retry feedback includes exact browser evidence, computational metric violations, judge findings, and 1–3 concrete fix suggestions.
10. Every run emits labeled, structured artifacts that feed the self-improvement flywheel: pairwise preference mining, AST pattern extraction, curriculum-from-failures synthesis, and periodic RFT of a post-processor model.

### 3.1 The Core Design Principle: Deterministic First, Visual Second

Five tiers of verification. Cheaper deterministic checks run first. Expensive subjective checks run last. Hard failure at any tier stops the pipeline.

```
Tier 1: Structural Integrity     → axe-core + IBM achecker critical/serious = 0, valid HTML5, no console errors
Tier 2: Full WCAG 2.2 AA         → full axe + ACT + IBM, tab-order graph, focus-visible contrast
Tier 3: Core Web Vitals          → LHCI median-of-5, LCP≤2500ms, CLS≤0.10, TBT≤200ms, scripted INP
Tier 4: APCA + Motion            → perceptual contrast per text element, reduced-motion compliance
Tier 5: Computational + Visual   → 15 metrics + judge panel with BT aggregation
```

Tiers 1–4 are fully deterministic. Tier 5 has deterministic components (computational metrics) and subjective components (judge panel). The panel uses pairwise comparison, not absolute scoring, because MLLM-as-a-Judge (Chen et al., ICML 2024) showed pairwise achieves ~0.6–0.7 human agreement while absolute scoring drops to Pearson ~0.49.

---

## 4. Roko Architecture Primer

### 4.1 The Universal Noun: Engram

Every event, data point, agent output, and gate verdict is an `Engram`:

```rust
pub struct Engram {
    pub id: ContentHash,                    // BLAKE3 hash identity
    pub fingerprint: Option<HdcFingerprint>, // HDC vector for similarity search
    pub kind: Kind,                         // GateVerdict, AgentOutput, Task, Episode, etc.
    pub body: Body,                         // Empty | Text | Json | Bytes
    pub created_at_ms: i64,
    pub decay: Decay,                       // HalfLife | Ttl | Ebbinghaus | None
    pub provenance: Provenance,             // author, trust, taint
    pub score: Score,                       // 7 axes: confidence, novelty, utility, reputation, precision, salience, coherence
    pub lineage: Vec<ContentHash>,          // DAG for auditing
    pub tags: BTreeMap<String, String>,
    pub attestation: Option<Attestation>,
    pub emotional_tag: Option<EmotionalTag>,
}
```

Score combination: `effective = confidence × (1 + novelty) × (1 + utility) × reputation × salience_factor × coherence_factor`.

### 4.2 The Gate Trait

```rust
#[async_trait::async_trait]
pub trait Gate: Send + Sync {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}

pub struct Verdict {
    pub passed: bool,
    pub reason: String,
    pub gate: String,
    pub score: f32,
    pub detail: Option<String>,
    pub error_digest: Option<String>,
    pub duration_ms: u64,
}
```

### 4.3 The Gate Pipeline

Sequential composition with short-circuit. Rungs 0–6 exist. UiGate is rung 7+.

```
Rung 0: CompileGate → Rung 1: ClippyGate → Rung 2: TestGate → Rung 3: SymbolGate
→ Rung 4: GeneratedTestGate → Rung 5: PropertyTestGate → Rung 6: IntegrationGate/LlmJudgeGate
→ Rung 7+: UiGate (NEW)
```

### 4.4 The Orchestration Loop

For each task: (1) 9-layer SystemPromptBuilder assembles prompt, (2) CascadeRouter selects model, (3) agent runs, (4) GatePipeline executes rungs in order, (5) if failed + replan-on-failure enabled, revised plan generated from failure feedback, (6) all results persisted as Engrams for learning.

---

## 5. Research Foundations: Complete Citation Index

Every research paper, benchmark, tool, and production system referenced in the source documents is catalogued below with its specific contribution and how it maps to UiGate's design.

### 5.1 Self-Improvement and Self-Correction

**Self-Refine** (Madaan et al., NeurIPS 2023, arXiv 2303.17651). LLMs iteratively refine outputs through self-generated feedback without additional training. Three-phase loop: generate → feedback → refine. ~20% average absolute improvement across 7 tasks. Up to 49.2% improvement on dialogue response generation with GPT-4. Maximum 4 iterations with diminishing returns. **UiGate mapping**: The attempt-level repair loop is Self-Refine with external verification. The critical difference: feedback comes from the browser (external oracle), not the LLM itself.

**Reflexion** (Shinn et al., NeurIPS 2023). Verbal reinforcement learning: agents maintain a running memory of natural-language self-reflections on past failures, used as context in future attempts. No weight updates — purely in-context learning. **UiGate mapping**: The playbook system. When "constrain modal height and add internal scrolling" fixes mobile clipping across multiple tasks, that pattern is stored and injected into future prompts.

**Voyager** (Wang et al., 2023). Growing skill library for game-playing agents. Successful behaviors extracted into reusable skills that compound over time. **UiGate mapping**: The assertion synthesis loop converts repeated visual failures into deterministic checks. "Mobile horizontal overflow in 11 tasks" becomes a generated assertion.

**Huang, Dasgupta, Ghosh, Hall, and Lee** — "Large Language Models Cannot Self-Correct Reasoning Yet" (ICLR 2024). Fundamental result: intrinsic self-correction — where the model judges its own work — typically *degrades* performance. LLMs cannot reliably evaluate their own outputs without external signal. **UiGate mapping**: This is why the implementer agent never judges its own UI. The evaluator is always separate. The browser provides the external verification signal.

**Song, Zhao, Lin et al.** — "Mind the Gap: Examining the Self-Improvement Capabilities" (ICLR 2025). Formalized the **generation-verification gap**: self-improvement works only when verification capability exceeds generation capability. **UiGate mapping**: The EVM/browser is the verifier that exceeds the agent's generation capability. Console errors, layout overflow, and failed requests are facts the agent cannot argue with.

**Pan, Misra et al.** — "Self-Refinement Spontaneously Reward-Hacks" (ICML 2024). Self-refinement systems learn to game their own evaluation rather than genuinely improve. **UiGate mapping**: This is why gate composition is conjunctive on hard gates (cannot be gamed) and why the judge panel uses disjoint model families (harder to simultaneously game multiple independent judges).

**Wu et al.** — "EvolveR: Self-Evolving LLM Agents through an Experience-Driven Lifecycle" (arXiv, October 2025). Experience-driven agent improvement lifecycle. **UiGate mapping**: The dream-cycle distillation loop (PRD-05) clusters episodes and promotes playbooks.

**Fang et al.** — "A Comprehensive Survey of Self-Evolving AI Agents" (arXiv, August 2025). Survey of self-evolving agent architectures. **UiGate mapping**: Contextual reference for the flywheel architecture.

### 5.2 Web Agent Training

**WebRL** (Qi et al., arXiv 2411.02337, ICLR 2025). Self-evolving online curriculum RL for web agents. Three innovations: (1) self-evolving curriculum generating new tasks from unsuccessful attempts, (2) robust outcome-supervised reward model (ORM) with binary success/failure signals, (3) KL-constrained policy updates preventing catastrophic forgetting. Results: Llama-3.1-8B from 4.8% to 42.4% on WebArena-Lite, surpassing GPT-4-Turbo (17.6%) by 160%+. **UiGate mapping**: The curriculum-from-failures pattern in the flywheel (PRD-04, step 5). Failed UI gate runs cluster by judge-rationale embedding; each cluster spawns synthetic training tasks. Promote successes into the regression eval set.

**WebGen-Agent** (Lu et al., arXiv 2509.22644, September 2025). Multi-level visual feedback for website generation: screenshot feedback (VLM scores visual quality at each step) + GUI-agent feedback (automated agent tests functional requirements) + backtracking mechanism + Step-GRPO training with dense step-level rewards. Results: Claude-3.5-Sonnet from 26.4% to 51.9% accuracy, 3.0 to 3.9 appearance. **UiGate mapping**: The closest published system to UiGate's architecture. Screenshot scoring maps to Tier 5. GUI-agent testing maps to Tiers 1–4. Backtracking maps to "reference previous best attempt." Step-GRPO maps to the RFT post-processor strategy.

**DigiRL** (arXiv 2406.11896). Lifts a 1.3B VLM from 17.7% to 67.2% on Android-in-the-Wild (AITW) via autonomous RL with VLM-evaluator. **UiGate mapping**: Methodological blueprint for self-improving visual loops. Demonstrates that even small models can dramatically improve with autonomous RL when the reward signal is grounded.

**Online-Mind2Web's WebJudge-7B**. Agent-as-judge reward model for web tasks. **UiGate mapping**: Pattern for training a small reward model from UiGate's accumulated preference data (deferred to month 7+).

### 5.3 Karpathy Autoresearch

**Karpathy, "autoresearch"** (GitHub, March 2026). AI agent autonomously runs ML experiments. Three files: `prepare.py` (immutable verifier), `train.py` (subject of optimization), `program.md` (search heuristic). Loop: propose change → train 5 min → evaluate against `bits_per_byte` → keep if improved → repeat. ~12 experiments/hour. First session: 700 experiments in 2 days, 20 optimizations found, 11% training speedup. Independently rediscovered RMSNorm and tied embeddings. **UiGate mapping**: The meta-loop optimization pattern (PRD-04). The ContextAssemblyConfig (or equivalent UiGate config) is `train.py`. The eval suite from all mechanisms is `prepare.py`. The optimization loop searches the config space automatically.

### 5.4 DSPy and Optimization Frameworks

**DSPy** (Khattab, Singhvi, Maheshwari et al., NeurIPS 2023; ICLR 2024 Spotlight). Programmatic framework for LLM pipeline optimization. Compiles declarative language model calls into self-improving pipelines. **UiGate mapping**: Framework for optimizing retrieval queries, clarifying-question prompts, judge rubrics, and AutoFix repair prompts.

**MIPROv2** (Opsahl-Ong et al., EMNLP 2024). Three-stage Bayesian optimization for multi-stage LM programs: (1) Bootstrap — collect top-scoring traces, (2) Generate candidates — LLM proposes modifications based on patterns, (3) Search — Tree-Parzen Estimators (TPE) as surrogate model. Improved accuracy by up to 13% with Llama-3-8B. **UiGate mapping**: The optimizer for prompt strategy experiments. MIPROv2 over retrieval queries, clarifying-question prompts, judge rubrics, and repair prompts. Do NOT optimize the base generator prompt directly (too much capacity, low signal-per-call).

**BetterTogether** (Soylu et al., EMNLP 2024). Alternates between prompt optimization and weight fine-tuning, achieving up to 60% gains over weight-only optimization. **UiGate mapping**: Theoretical backing for the two-phase strategy: optimize prompts first (months 1–6), then fine-tune a post-processor once sufficient data accumulates (month 7+).

### 5.5 Multi-Armed Bandits

**TensorZero** (2025). Track-and-Stop optimal bandits in an LLM gateway. Reduces time to identify best configuration by 37% vs standard A/B testing. Each configuration is an "arm"; each execution provides a reward signal. **UiGate mapping**: Production routing over pipeline arms (model choice, retrieval-k, with/without clarifying turn, with/without post-fixer). Every UiGate run emits TensorZero-compatible traces.

**MASPOB** (arXiv, March 2026). Multi-agent bandit optimization. Jointly optimizes agent prompts and system topology using UCB-based bandits with GNN-encoded topological priors. **UiGate mapping**: Future extension for jointly optimizing the generator agent, evaluator panel, and repair agent as a multi-agent system.

### 5.6 Evaluation and Judging Methodology

**MLLM-as-a-Judge** (Chen et al., ICML 2024 Oral, arXiv 2402.04788). Decisive finding: GPT-4V/Gemini/Qwen-VL/LLaVA-1.6 align with humans on pairwise comparison (~0.6–0.7+ agreement) but diverge significantly on absolute scoring (Pearson ~0.49) and batch ranking. **UiGate mapping**: This is why the judge uses pairwise comparison (new candidate vs previous best release against a fixed anchor), never absolute Likert scoring.

**PoLL** (Verga et al., arXiv 2404.18796). Panel of smaller diverse-family models outperforms a single large judge at 7× lower cost. **UiGate mapping**: The mandatory panel composition: one closed frontier (Claude Opus), one open multimodal critic (LLaVA-Critic-72B), one rubric-conditioned specialist (Prometheus-Vision). Trimmed-mean (10–20%) aggregation. Never use the same model family as both generator and judge.

**LLaVA-Critic-72B** (arXiv 2410.02712). Fine-tuned on 113k pointwise + pairwise judgments, matches GPT-4o on judgment alignment. **UiGate mapping**: One of three mandatory panel members. Open-source, can be self-hosted for cost control.

**Prometheus-Vision** (arXiv 2401.06591). Rubric-conditioned evaluation specialist. Pearson ~0.5–0.6 with humans at a fraction of API cost. **UiGate mapping**: Third panel member. Rubric-conditioned means it can be given UiGate's specific rubric dimensions.

**G-Eval**. Structured evaluation template. Modern frontier APIs no longer reliably expose token logprobs at the scale G-Eval needs, killing the probability-weighted variant. **UiGate mapping**: Fall back to N=5 samples per judgment at T=0 (captures ~80% of N=20's variance reduction), majority vote or median, force analyze-before-rate.

**UICrit** (Duan et al., UIST 2024, arXiv 2407.08850). 1,000 RICO mobile UIs with 3,059 critiques, bounding boxes, and ratings on aesthetics/learnability/usability/overall by 7 professional designers, three annotators per UI, CC-BY 4.0. Few-shot + visual prompting achieved +55% improvement in LLM feedback quality. Critical finding: app-store rating correlates r=0.007–0.023 with expert aesthetic ratings — aggregate user ratings are useless as quality signal. **UiGate mapping**: Use UICrit's annotation schema verbatim for judge prompts. Use the dataset as judge fine-tuning corpus. The bounding-box-grounded critique format is the template for UiGate's `VisualFinding` type.

**UIClip** (arXiv 2404.12500). 67.65% on synthetic UI quality but only 57.89% on real apps. **UiGate mapping**: Off-the-shelf MLLM judges underperform on real UIs — validates the need for ensembling and fine-tuning.

**AgentRewardBench** (2025). No single LLM judge performs best across all web-agent benchmarks. Rule-only evaluation underreports true success. **UiGate mapping**: Validates the hybrid approach: deterministic rules + LLM panel. Neither alone is sufficient.

**Wang et al. (ACL 2024, arXiv 2305.17926)**. Position bias is the single biggest reliability killer in LLM judging. Position consistency drops below 0.5 with 3–4 candidates when quality gaps are small. **UiGate mapping**: Always evaluate (A,B) and (B,A) and discard inconsistent verdicts. The 2× cost is non-negotiable.

**Davidson model** (arXiv 2412.18407). Extension of Bradley-Terry for ties. **UiGate mapping**: Use Davidson when judges return ties in pairwise comparison.

**Wataoka et al.** (NeurIPS Safe-GenAI WS 2024, arXiv 2410.21819). Self-preference is well-documented: models prefer their own outputs, correlating with low perplexity-on-self. **UiGate mapping**: Never use the same model family as both generator and judge.

### 5.7 Statistical Rigor

**Krippendorff's α** (Krippendorff 2004). Inter-rater reliability measure. Thresholds: ≥0.8 acceptable, ≥0.667 tentative, below 0.667 broken. Stricter than Landis-Koch κ which most LLM-judge papers cite to over-claim. **UiGate mapping**: α ≥ 0.8 on a held-out human canary set is the eval-quality gate. If panel score rises but canary doesn't, you're Goodharting.

**BCa bootstrap intervals** (Efron 1987). Second-order accurate, handles skewed aesthetic scores better than percentile bootstrap. **UiGate mapping**: Use BCa for computing confidence intervals on aesthetic scores.

**arXiv 2511.19794 release-gate protocol**. Paired-by-prompt design + BCa + sign-flip permutation. Claim significance only if BCa excludes 0 AND permutation p<0.05. **UiGate mapping**: Adopt for deciding whether a pipeline change is a real improvement.

**Benjamini-Hochberg FDR** for exploratory dimension-by-dimension analysis. **Bonferroni** for headline metric. **UiGate mapping**: Multi-dimensional rubric analysis uses BH for individual dimensions, Bonferroni for the overall score.

**Evan Miller — "Adding Error Bars to Evals"** (Anthropic, 2024). Clustered standard errors can be 3× larger than naive estimates. **UiGate mapping**: Use clustered standard errors when computing confidence intervals on eval metrics.

**Princeton — "AI Agents That Matter"** (Kapoor et al., 2025). Run each configuration at least 5 times, report mean ± standard deviation. **UiGate mapping**: Minimum 5 runs per configuration for optimization experiments. 3 runs even at T=0 for production judgments (API non-determinism produces 1–3% disagreement on identical inputs).

**τ-bench pass@k**. Agent scoring 60% on single run may drop to 25% when required to succeed 8 consecutive times. **UiGate mapping**: Small improvements in underlying capability produce large improvements in reliable deployment. Even modest eval-driven gains are practically significant.

**Chatbot Arena sample-size empirics** (Chiang et al., arXiv 2403.04132). ~400 votes per model gives ±10 Elo at typical dispersions. For 5-pt Likert at σ≈1.0 detecting Δ=0.2 at α=0.05/power=0.8, n≈393 per arm unpaired or n≈199 paired with ρ=0.5. **UiGate mapping**: Informs how many human-rated canary evaluations are needed before the system can claim calibration.

### 5.8 Goodhart's Law and Reward Hacking

**Skalse, Howe, Krasheninnikov, and Krueger** — "Defining and Characterizing Reward Hacking" (NeurIPS 2022; extended ICLR 2024). Geometric explanations for reward hacking in RL. Proposed provably safe early-stopping methods. **UiGate mapping**: Theoretical foundation for anti-Goodhart safeguards.

**Manheim & Garrabrant** (arXiv 1803.04585). Four types of Goodhart failure: (1) Regressional — optimizing on noise, (2) Extremal — pushing metrics into regimes where they're not validated, (3) Causal — confusing correlation with causation, (4) Adversarial — the optimizer actively games the metric. **UiGate mapping**: Specific countermeasures for each type. Regressional: require improvements on ≥3 orthogonal metrics. Extremal: clip rewards into bands rather than maximizing (perfect symmetry = sterile UIs). Causal: use APCA alongside WCAG contrast (high WCAG ratio doesn't cause readability if font is too thin). Adversarial: rotating eval suites, holdouts, meta-eval.

**Moskovitz et al.** — "Confronting Reward Model Overoptimization with Constrained RLHF" (ICLR 2024). Constrained RLHF with per-reward-model thresholds outperforms weighted-sum because correlated proxy reward models amplify Goodhart. **UiGate mapping**: Gate composition is conjunctive on hard gates and Pareto on soft, **never weighted-sum**. A strong saliency score must not compensate for an APCA failure.

**"The Leaderboard Illusion"** (Singh et al., NeurIPS 2025). Meta tested 27 private variants before public release on Chatbot Arena. Limited additional Arena data yielded up to 112% performance gains on Arena-Hard. **UiGate mapping**: Why the held-out canary set must be frozen and never exposed to the optimizer.

**Krakovna's specification gaming list**. Continuously-updated examples of agents gaming their reward functions. **UiGate mapping**: Maintain as a CI-integrated red-team prompt set. Every quarter, run agents adversarially against current eval and add new gaming patterns.

### 5.9 Harness Engineering

**Pan et al.** — "Natural-Language Agent Harnesses" (arXiv 2603.25723, March 2026). Framework for structuring agent evaluation harnesses. **UiGate mapping**: Informs the structure of UiGate's eval harness — how specs, runners, graders, and reporters compose.

**Lee et al.** — "Meta-Harness: End-to-End Optimization of Model Harnesses" (arXiv 2603.28052, March 2026). Stanford research showing 6× performance gap from scaffolding changes alone. **UiGate mapping**: Validates that the harness architecture (how context is assembled, how feedback is structured, how retries are sequenced) matters as much as model quality.

### 5.10 Context Engineering

**Liu et al.** — "Lost in the Middle: How Language Models Use Long Contexts" (TACL 2024). Models perform best when relevant information is at the beginning or end of the context, worst in the middle. **UiGate mapping**: Placement strategy for retry feedback — put the "what to fix" section at the beginning of the retry prompt, evidence in the middle, unchanged context at the end.

**Chroma** — "Context Rot: How Increasing Input Tokens Impacts LLM Performance" (July 2025). Performance degrades as context grows. **UiGate mapping**: Keep retry feedback concise (max 3 findings). Do not dump entire result.json into the prompt.

**LLMLingua** (Jiang et al., EMNLP 2023). Prompt compression for accelerated inference. **UiGate mapping**: Potential future optimization for compressing verbose browser evidence before including in retry prompts.

### 5.11 Smart Contract Testing (from doc-17)

**Echidna** (Grieco, Song, Feist, Groce, Gustafson — "Echidna: effective, usable, and fast fuzzing for smart contracts," ISSTA 2020, Trail of Bits). Property-based testing for smart contracts. Generates random call sequences from ABI, uses coverage-guided fuzzing, reports minimal violating sequences. Used in 10+ major paid security audits. **UiGate mapping**: Pattern for property-based eval generation — translate knowledge entries into property assertions automatically.

**ContractFuzzer** (Jiang, Liu, Chan — "ContractFuzzer: Fuzzing Smart Contracts for Vulnerability Detection," ASE 2018). Generates fuzzing inputs directly from ABI specifications. Defines test oracles for reentrancy, gasless send, exception disorder, timestamp dependency. Deployed against 6,991 contracts, detected 459 confirmed vulnerabilities. **UiGate mapping**: Pattern for generating test oracles from interface specifications.

**SynTest-Solidity** (Tanida, Volta, Panichella — ICSE 2022 Demo). DynaMOSA and NSGA-II genetic algorithms for automated test case generation. Evolves test suites maximizing branch coverage. **UiGate mapping**: Genetic algorithm approach to evolving eval suites for maximum discriminating power.

**EVMFuzz** (Fu, Ren, Ma, Jiang, Sun — "Differential Fuzz Testing of Ethereum Virtual Machine," 2019). Cross-implementation testing via deterministic replay. **UiGate mapping**: Pattern for cross-browser differential testing — run same journey on different backends, compare state diffs.

### 5.12 Agent Benchmarks

**SCONE-bench** (Anthropic, 2025). 405 historically exploited contracts across Ethereum, BSC, and Base, forked at pre-exploit blocks. Claude Opus 4.5 exploited 65% of problems. **UiGate mapping**: Exploit replay evaluation pattern — fork at known-bad state, test whether agent detects the problem.

**EVMbench** (Wang et al., OpenAI/Paradigm, March 2026). 117 curated vulnerabilities from Code4rena, deployed on fresh Anvil chains with Rust eval harness. Fully machine-graded via balance deltas and state transitions. **UiGate mapping**: Machine-grading via deterministic state transitions — analogous to UiGate's deterministic browser assertions.

**Re-Evaluating EVMBench** (Storhaug and Meling, March 2026). Agents achieved 61–72% on curated benchmarks but 0% on real-world incidents. **UiGate mapping**: Curated benchmarks dramatically overestimate real-world capability. Motivates continuous scenario mining from new events, not just curation of historical cases.

**SWE-bench** (Jimenez et al., ICLR 2024). Real-world GitHub issue resolution. **UiGate mapping**: Primary eval benchmark for code agent capability. UiGate should correlate with SWE-bench performance on frontend issues.

**AgentBench** (Liu et al., ICLR 2024). Evaluating LLMs as agents across diverse environments. **UiGate mapping**: Broad agent capability baseline.

**HAL** (Kapoor et al., Princeton, ICLR 2026). Holistic Agent Leaderboard. **UiGate mapping**: Reference leaderboard for comparing agent performance.

### 5.13 Web Agent Benchmarks

**MiniWoB++**. Synthetic short-horizon web interaction tasks. **UiGate mapping**: Useful for unit-testing UiGate's runner on simple interactions.

**WebArena** (Zhou et al., 2023). 812 tasks across 5 self-hosted websites. Current SOTA ~58% (Operator/CUA). **UiGate mapping**: Primary agentic web benchmark. UiGate-generated UIs should be testable by WebArena-class agents.

**VisualWebArena**. 910 vision-required tasks. Human 88.7%, current SOTA (tree-search GPT-4o) = 26.4%. **UiGate mapping**: Demonstrates the gap between human and agent visual understanding on web tasks.

**WorkArena**. Enterprise workflow tasks. **UiGate mapping**: Relevant for testing UiGate on enterprise UI patterns.

**Mind2Web / Online-Mind2Web**. 2,350 → 300 live tasks. Open-web generalization. **UiGate mapping**: Generalization benchmark.

**BrowserGym**. Unified evaluation ecosystem for web agents. **UiGate mapping**: Potential integration target for standardized evaluation.

**WebLINX**. Multi-turn web navigation. **UiGate mapping**: Multi-step journey evaluation pattern.

**ScreenSpot-V2 / ScreenSpot-Pro**. GUI grounding accuracy. Current SOTA (GUI-CURSOR) 93.9%. ScreenSpot-Pro highlights weak grounding on dense professional interfaces. **UiGate mapping**: If UiGate's generated UI is consumed by agents, grounding accuracy tells you whether the agent can find the buttons. Dense professional UIs remain challenging.

**VisualWebBench** (NAACL 2024, arXiv 2404.05955). 1,500 instances, 7 tasks. Decouples understanding from navigation. **UiGate mapping**: Useful for isolating visual understanding from action execution.

### 5.14 Frontend Code Generation Benchmarks

**Design2Code** (Si et al., Stanford, arXiv 2403.03163). 484 real C4 webpages + 80 HARD subset. Metric stack: Block-Match (visual block detection + Jonker-Volgenant assignment + area-overlap), text similarity per matched block, position similarity (1 − normalized centroid distance), color similarity (CIEDE2000), CLIP cosine. Gameability analysis: CLIP most gameable, Position most robust, Text robust if OCR-based. Human eval: GPT-4V rated better than original in 64% of cases. **UiGate mapping**: Use geometric mean of Block-Match + Position + Text as defensible auto-score floor. Use Design2Code-HARD as held-out eval set.

**DesignBench** (arXiv 2506.06251). Generation, edit, and repair across multiple frontend frameworks. MLLMs achieve only 0.27 accuracy on UI issue identification — frontier judges systematically miss real defects. **UiGate mapping**: Validates that deterministic tiers must run before and independently of visual judges.

**FrontendBench**. Interactive test scenarios rather than static outputs. **UiGate mapping**: Validates journey-based testing over screenshot-only evaluation.

**WebGen-Bench** (Lu et al., arXiv 2505.03733). 101 website-generation instructions + 647 GUI-agent test cases. Evaluates end-to-end repository-level website development. **UiGate mapping**: Closest benchmark to UiGate's use case.

**WebCoderBench**. Web development tasks with execution. **UiGate mapping**: Supplementary frontend eval.

**1D-Bench**. Iterative editing with execution feedback. Post-training and RL-style repair possible but unstable because rewards are sparse and file-level edits have high variance. **UiGate mapping**: Rewards should be dense, localized, and tied to explicit failure classes rather than a single terminal score.

**Figma2Code** (arXiv). Design mockups contain rich multimodal metadata lost when reduced to screenshots. Current models struggle with responsiveness and maintainability even when visual fidelity improves. **UiGate mapping**: If design metadata is available, ingest it — don't settle for screenshot-only references.

### 5.15 Self-Evolving Benchmarks

**Benchmark Self-Evolving** (Wang, Long, Fan, Huang, Wei — COLING 2025). Six reframing operations: paraphrase, semantic perturbation, format change, difficulty escalation, sub-ability probing, cross-context transfer. **UiGate mapping**: Applied to eval generation — a single UI failure spawns 6+ assertion variants.

**LiveBench** (White et al., ICLR 2025 Spotlight). Monthly updates with fresh questions, objective ground truth rather than LLM judges. **UiGate mapping**: Eval rotation pattern. Periodically retire old evals and generate fresh ones.

**LiveCodeBench** (Jain et al., 2024). Continuously collects new problems from competitive programming platforms. **UiGate mapping**: Continuously collect new UI eval scenarios from real deployments.

### 5.16 RAG and Context Quality

**RAGAS** (Shahul Es et al., EACL 2024). Automated evaluation of RAG systems. **UiGate mapping**: Eval framework for context assembly quality.

**ARES** (Saad-Falcon, Khattab et al., NAACL 2024). Automated RAG evaluation framework. **UiGate mapping**: Supplementary RAG eval methodology.

**Sufficient Context** (Joren et al., ICLR 2025). New lens on RAG system evaluation. **UiGate mapping**: Evaluating whether the assembled context pack contains sufficient information for the task.

**Databricks RAG study** (2024). Kendall's τ ≈ 0.7–0.8 correlation between retrieval quality and generation quality. **UiGate mapping**: Each context quality improvement compounds into downstream task improvements.

### 5.17 Multi-Agent Scaling

**MacNet** (Qian et al., ICLR 2025). Collaborative scaling law following logistic growth pattern in multi-agent systems. **UiGate mapping**: Informs scaling expectations for the multi-agent eval generation architecture.

**"Towards a Science of Scaling Agent Systems"** (Kim et al., Google Research, December 2025). +81% improvement on parallelizable tasks from multi-agent coordination. **UiGate mapping**: Validates parallel agent approaches for eval generation and red-teaming.

### 5.18 Adversarial and Security

**AGENTPOISON** (NeurIPS 2024). Poisoning less than 0.1% of an agent's knowledge base achieves 63% end-to-end attack success. **UiGate mapping**: The shared knowledge base is a high-value attack surface. Eval system must test resilience. Anomaly detection should quarantine poisoned entries within ~40 seconds.

**AutoRedTeamer** (Liu et al., 2025). 20% higher attack success rates than existing methods by maintaining memory of effective attack patterns. **UiGate mapping**: Red team golem architecture — specialized agents whose purpose is adversarial eval generation.

**HarmBench** (Mazeika et al., ICML 2024). Standardized automated red teaming with 18 attack methods. **UiGate mapping**: Standardized adversarial eval framework.

**WASP** (2025). Prompt injection against web agents: agents execute adversarial instructions 16–86% of the time. **UiGate mapping**: If the system browses external sites for design inspiration, isolate browsing from code-writing. Treat all external pages as untrusted.

### 5.19 Computational Aesthetics and Metrics

**AIM — Aalto Interface Metrics** (Oulasvirta et al., UIST 2018). Twenty-one computational metrics including Feature Congestion (Rosenholtz), Grid Quality, color-blindness simulation, JPEG-size complexity proxy. **UiGate mapping**: The load-bearing externals layer for layout health. Vendor the library. Compute Feature Congestion and Grid Quality on every render.

**Koch & Oulasvirta** (CHI 2016). Gestalt operationalization: proximity via DBSCAN silhouette on element centroids, similarity via cosine on style vectors per role-class, continuity via shared-axis alignment runs ≥3, closure via bounded-region detection, figure/ground via APCA on primary surfaces. **UiGate mapping**: Concrete algorithms for computing Gestalt principle adherence.

**DeepGaze IIE** (arXiv 2105.12441). Saliency prediction model. AUC 88.3. **UiGate mapping**: One of two models in the saliency ensemble.

**UMSI++** (UEyes, CHI 2023, Zenodo 8010312). UI-specific saliency SOTA. **UiGate mapping**: Second model in the saliency ensemble. UI-specific training makes it more relevant than general saliency models.

**Hasler-Süsstrunk colorfulness** metric. Optimal band M ∈ [15, 35] per Reinecke et al. CHI 2013 first-impression study. **UiGate mapping**: Soft metric — penalize outside the band.

**Miniukovich** (AVI 2014). Element density thresholds: ≤30 mobile, ≤50 desktop. **UiGate mapping**: Hard threshold for element count per viewport.

**APCA — Accessible Perceptual Contrast Algorithm** (`Myndex/apca-w3`, Apache-2.0). Per-text-element Lc thresholds by font size and weight: Lc 75 body preferred, Lc 60 minimum body, Lc 45 large, Lc 30 floor, Lc 15 non-text. Polarity-corrected on composited pixel colors. Stricter than WCAG AA, catches orange-button/thin-font failures that pass AA but are perceptually weak. **UiGate mapping**: Tier 4 hard gate, replacing simple WCAG contrast ratio.

**Design Tokens Community Group 2025.10 spec**. W3C technical specification for exchanging design tokens. `$type`/`$value` with composite types. Three-tier taxonomy: primitive, semantic, component. File extension `.tokens` or `.tokens.json`. **UiGate mapping**: Target format for extracted design tokens.

**Style Dictionary v4** (Apache-2.0). Canonical OSS sink for multi-platform token output: CSS variables, Tailwind config, Swift, Kotlin. **UiGate mapping**: Token output pipeline.

**Project Wallace** (`@projectwallace/css-analyzer` MIT, `@projectwallace/css-design-tokens` EUPL-1.2). Primary primitive-extractor: ingests raw CSS, emits near-DTCG JSON for color, fontSize, fontFamily, lineHeight, gradient, boxShadow, radius, duration, easing. **UiGate mapping**: Tier 2 extraction — what's literally written in CSS.

### 5.20 Production Systems to Learn From

**v0's composite stack** (Vercel). Frontier base model + RAG over curated component library + small RFT-trained post-processor (`vercel-autofixer-01`, Fireworks RFT, replacing Gemini Flash 2.0). **UiGate mapping**: The architectural template. The post-processor is the compounding asset.

**21st.dev's curated community library**. RAG over juried inventory that grows with submissions. **UiGate mapping**: Cleanest compounding-asset pattern. Pattern for the private component library that grows from approved UiGate outputs.

**Subframe's clarifying-question step**. Before generation, ask whether the prompt needs clarification. **UiGate mapping**: Cheapest MIPROv2-style improvement — a DSPy signature with conditional emission.

**Builder.io's Mitosis IR layer**. Optimize once, emit many target frameworks. **UiGate mapping**: Worth studying for framework-agnostic component generation.

**Builder.io's component-mapping feature**. RAG over the repo's actual components. **UiGate mapping**: Right architecture for private design-system enforcement — clone the pattern.

**Locofy's Large Design Model**. Heuristics beat LLMs on deterministic parts of the pipeline. **UiGate mapping**: Validates that deterministic checks should run first and independently.

### 5.21 Active Inference (from doc-17)

**Free Energy Principle** (Friston, "The free-energy principle: a unified brain theory?" Nature Reviews Neuroscience, 2010). Prediction residual IS the variational free energy that the system minimizes. **UiGate mapping**: The prediction-outcome gap (PF residual) is an automatic eval metric. Minimizing it is identical to self-improvement.

**Millidge, Tschantz, and Buckley** — "Whence the Expected Free Energy?" (Neural Computation, 2021). Expected free energy decomposes into pragmatic value (goal achievement) and epistemic value (information gain). **UiGate mapping**: PF residual automatically captures both "did the agent succeed?" and "did the agent learn?"

### 5.22 Accessibility Coverage Reality

**Rushi 2024**: Realistic WCAG 2.2 automation coverage is 29.5% fully automated, 10.3% partial, 60.2% manual of WCAG 2.2 success criteria. axe's "57%" marketing counts violation instances, not success criteria. **UiGate mapping**: Plan for the manual layer. Don't pretend automation is sufficient. Human calibration loop (PRD-05) covers the gap.

### 5.23 UX Measurement

**NN/g (Nielsen Norman Group)**. Task success rate, time on task, error rate, subjective satisfaction as basic usability measures. **UiGate mapping**: Foundational metrics for evaluating UI quality.

**Single Ease Question (SEQ)**. Post-task measure of task difficulty. **UiGate mapping**: Potential human calibration signal.

**System Usability Scale (SUS)**. 68 = roughly average, 80 = strong target. **UiGate mapping**: Benchmark interpretation for usability scoring.

**HEART framework**. Connects UX goals to observable signals and metrics. **UiGate mapping**: Framework for connecting UiGate metrics to product-level UX goals.

### 5.24 Visual Regression Tools

**odiff** (MIT, Zig+SIMD). ~6.6× pixelmatch performance. Emits `{match, reason, diffCount, diffPercentage}`. **UiGate mapping**: The differ for visual regression. Skip Applitools, Percy, Chromatic (SaaS-locked, ML-opaque).

**dssim** (Rust, AGPL-or-commercial). Structural similarity metric. **UiGate mapping**: Tiebreaker — if odiff says `diffPercentage > 0.1%` AND dssim > 0.05, fail. If pixel diff but dssim < 0.01, mark as anti-aliasing noise and pass.

**reg-suit**. OSS visual regression tool with JSON output shape. **UiGate mapping**: Emit reg-suit-compatible JSON `{failedItems, newItems, deletedItems, passedItems, diffItems}` so existing tooling consumes UiGate verdicts.

---

## 6. Goals

1. Add first-class UI verification with deterministic hard gates and calibrated visual judgment.
2. Use pairwise Bradley-Terry comparison with disjoint judge panels, not absolute Likert scoring.
3. Compute 15 quantitative metrics on every render before any LLM judge runs.
4. Gate composition: conjunctive on hard gates, Pareto frontier on soft, never weighted-sum.
5. Feed every run into a self-improvement flywheel that compounds permanently.
6. Defer custom UI judge training for 12+ months; spend fine-tune budget on a post-processor first.
7. Extract and enforce design tokens as measurable, deterministic constraints.
8. Maintain Goodhart resistance through held-out canaries, eval rotation, and Krippendorff monitoring.

## 7. Non-Goals

1. Pixel-perfect screenshot comparison (fragile, not how humans judge).
2. Replacing application-owned test suites.
3. Non-web UIs (mobile native, desktop native).
4. Building a browser engine.
5. Agent self-approval.
6. Custom UI judge training in year one (frontier APIs are sufficient).
7. Cross-browser testing beyond Chromium in MVP.
