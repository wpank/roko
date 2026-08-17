# PRD-09 -- Research Appendix: Citations, Methodology, and Theoretical Foundations

**Status**: Draft v2 (expanded)
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-29
**Role**: Reference appendix for PRD-00 through PRD-08

---

## Purpose

This document is the single authoritative reference for every research claim made
across the PRD set. Other documents cite into this appendix by section number
(e.g., "per the MLLM-as-a-Judge finding (Appendix, SS1.1)"). No research claim
in any PRD should exist without a corresponding entry here.

Organized into eleven topic areas covering 100+ citations. Each entry includes:
full author list, title, venue, year; key finding in 1-2 sentences; how it
motivates a specific design decision; and which PRD document(s) reference it.

---

## 1. LLM-as-Judge

### SS1.1 MLLM-as-a-Judge

**Citation**: Chen, Ge, Zhu, Xie, Ye, Jia, Zheng, Li, He, Shi, Huang, Ge, Yi, Zhang, and Shan. "MLLM-as-a-Judge: Assessing Multimodal LLM-as-a-Judge with Vision-Language Benchmark." *Proceedings of the 41st International Conference on Machine Learning (ICML)*, Oral, 2024. arXiv:2402.04788.

**Key finding**: Systematic evaluation of GPT-4V, Gemini Pro Vision, Qwen-VL-Max, and LLaVA-1.6 as visual judges. Pairwise comparison achieves approximately 0.6-0.7 human agreement, while absolute scoring drops to Pearson r of approximately 0.49. Batch ranking exhibits significant divergence from human orderings.

**Design motivation**: This is the decisive result behind the entire judging architecture. Pairwise comparison is structurally superior to absolute Likert scoring for three reasons: (1) lower variance per comparison because no fixed scale anchoring is required, (2) harder to Goodhart because the agent cannot learn a single target number to imitate, and (3) native composition with bandit optimization because LinUCB and preference learning fundamentally consume preference signals, not absolute scores. Every judge evaluation in the system compares new_candidate vs. prev_best_release against a fixed anchor, never scoring in isolation.

**Referenced by**: PRD-00 SS4.3, PRD-04 SS2, PRD-04 SS3.

---

### SS1.2 PoLL (Panel of LLM Evaluators)

**Citation**: Verga, Hofstatter, Althammer, and Dalton. "Replacing Judges with Juries: Evaluating LLM Generations with a Panel of Diverse Models." 2024. arXiv:2404.18796.

**Key finding**: A panel of smaller diverse-family models outperforms a single large judge at 7x lower cost. The critical variable is family diversity -- models from the same training lineage exhibit correlated biases that averaging cannot correct. A panel of three models from disjoint families consistently exceeds any single frontier judge on correlation with human ratings.

**Design motivation**: Mandates the disjoint-family panel composition: one closed frontier model, one open multimodal critic, one rubric-conditioned specialist. Panel aggregation uses trimmed mean (10-20% trim) rather than simple average, providing robustness against one judge going off-rails while using more information than median. Once 500+ human-rated examples accumulate, transition to learned weights via stacking on held-out human labels for an additional 3-8 correlation point gain.

**Referenced by**: PRD-00 SS4.3, PRD-04 SS4.1, PRD-04 SS4.2.

---

### SS1.3 Self-Preference Bias

**Citation**: Wataoka, Ozaki, Tatsuno, Seki, Inoue, and Kawahara. "Self-Preference Bias in LLM-as-a-Judge." *NeurIPS Safe Generative AI Workshop*, 2024. arXiv:2410.21819.

**Key finding**: Models systematically prefer their own outputs over equivalent-quality alternatives. Self-preference correlates strongly with low perplexity-on-self -- the model recognizes its own stylistic patterns and interprets familiarity as quality. The bias persists even when outputs are of objectively equal quality as rated by humans.

**Design motivation**: Establishes the mandatory rule: never use the same model family as both generator and judge. If Claude Sonnet generates the code, Claude Opus must not be the sole judge. The panel must include models from genuinely different training lineages. This constraint applies transitively -- if a model was distilled from another, they are the same family for these purposes.

**Referenced by**: PRD-00 SS4.3, PRD-04 SS4.1.

---

### SS1.4 LLMs Cannot Self-Correct Reasoning

**Citation**: Huang, Dasgupta, Ghosh, Hall, and Lee. "Large Language Models Cannot Self-Correct Reasoning Yet." *Proceedings of the Twelfth International Conference on Learning Representations (ICLR)*, 2024.

**Key finding**: Intrinsic self-correction -- where a model judges its own work without external signal -- typically degrades performance rather than improving it. The paper distinguishes intrinsic self-correction (no external feedback) from extrinsic self-correction (with oracle feedback), finding that only the latter reliably improves outputs.

**Design motivation**: This is why the implementer agent never judges its own output. The evaluator is always a separate system. The gate pipeline provides the external verification signal -- compiler errors, test failures, lint warnings are facts the agent cannot argue with. The entire deterministic gate architecture exists because external signal is what makes self-correction work.

**Referenced by**: PRD-00 SS4.1, PRD-05 SS2.

---

### SS1.5 LLaVA-Critic

**Citation**: Xiong, Wang, Guo, Shi, Lu, Chen, Zhou, Liu, Li, and Shen. "LLaVA-Critic: Learning to Evaluate Multimodal Models." 2024. arXiv:2410.02712.

**Key finding**: LLaVA-Critic-72B, fine-tuned on 113k pointwise and pairwise visual judgments, matches GPT-4o on judgment alignment with human preferences. Open-weight visual judges, when purpose-trained for evaluation, achieve frontier-class agreement at a fraction of API cost.

**Design motivation**: Designated as one of the mandatory panel members. Open-source, self-hostable for cost control at approximately $0.02/judgment versus approximately $0.10/judgment for closed frontier APIs. Its training on both pointwise and pairwise judgments makes it suitable for the dual evaluation modes used in the framework.

**Referenced by**: PRD-04 SS4.1.

---

### SS1.6 Prometheus-Vision

**Citation**: Kim, Shin, Cho, Jang, Longpre, Lee, Yun, Shin, Kim, Thorne, and Seo. "Prometheus-Vision: Vision-Language Model as a Judge for Fine-Grained Evaluation." KAIST, 2024. arXiv:2401.06591.

**Key finding**: Rubric-conditioned evaluation specialist achieving Pearson r of approximately 0.5-0.6 with human ratings at a fraction of API cost. Conditioning the judge on an explicit rubric with dimension definitions and scoring criteria substantially improves agreement over general-purpose judges.

**Design motivation**: Third mandatory panel member. Its rubric-conditioned nature is specifically valuable because the system uses a multi-dimension quality rubric. Prometheus-Vision can be given the exact rubric dimensions with definitions, making its judgments more aligned with the specific evaluation framework.

**Referenced by**: PRD-04 SS4.1.

---

### SS1.7 RocketEval: Checklist-Based LLM Evaluation

**Citation**: Li, Liu, He, and Zhang. "RocketEval: Efficient Automated LLM Evaluation via Grading Checklist." *Proceedings of the Thirteenth International Conference on Learning Representations (ICLR)*, 2025. arXiv:2503.05142.

**Key finding**: A three-stage evaluation framework -- checklist creation, checklist grading by lightweight LLMs, and score prediction via reweighting -- achieves 0.965 correlation with human preferences using Gemma-2-2B as the grader, comparable to GPT-4o. The approach reduces evaluation cost by over 50x for large-scale scenarios. The key insight is decomposing holistic evaluation into binary checklist questions that lightweight models can answer reliably, then recomposing scores via learned weights.

**Design motivation**: Validates the criterion decomposition approach in PRD-01. Each `Criterion` in the eval framework is analogous to a checklist item -- a specific, answerable question about code quality. Using lightweight models for individual criterion grading while reserving frontier models for complex holistic assessment directly mirrors RocketEval's architecture. The finding that Gemma-2-2B with RocketEval matches GPT-4o on holistic evaluation suggests that criterion-level evaluation can be pushed to smaller, cheaper models.

**Referenced by**: PRD-01 SS3, PRD-04 SS4.2.

---

### SS1.8 Bias in the Loop: Auditing LLM-as-a-Judge for Software Engineering

**Citation**: Multiple authors. "Bias in the Loop: Auditing LLM-as-a-Judge for Software Engineering." 2026. arXiv:2604.16790.

**Key finding**: LLM-as-a-Judge for code faces tightly coupled risks: lack of systematic test-retest reliability and prompt-sensitive biases that can systematically distort decisions. The paper identifies both explicit biases (position bias, verbosity bias) and implicit biases (style preference, framework preference) that affect code evaluation. Judge agreement with human reviewers varies significantly across programming languages and task types.

**Design motivation**: Reinforces the multi-model panel approach and adds the requirement for position-randomization in pairwise comparisons. When presenting code pairs to judge models, the system must randomize which candidate appears first to mitigate position bias. The finding about language-specific bias variation supports language-specific criterion calibration rather than universal thresholds.

**Referenced by**: PRD-04 SS4.1, PRD-04 SS6.

---

### SS1.9 AutoChecklist: Composable Pipelines for Checklist Generation

**Citation**: Multiple authors. "AutoChecklist: Composable Pipelines for Checklist Generation and Scoring with LLM-as-a-Judge." 2025. arXiv:2603.07019.

**Key finding**: Automatically generated evaluation checklists, when composed into modular pipelines, achieve evaluation quality comparable to expert-designed rubrics. The composable pipeline approach allows checklists to be reused across different evaluation contexts and combined hierarchically. This reduces the human effort required to create evaluation frameworks from days to minutes.

**Design motivation**: Supports the automated profile generation feature in PRD-01. When a new task type is encountered without a matching eval profile, the system can generate a task-specific checklist from the task description and available criteria, then compose it into a temporary profile. This eliminates the bootstrapping problem where new task types have no evaluation standards.

**Referenced by**: PRD-01 SS5, PRD-05 SS3.

---

### SS1.10 Survey on LLM-as-a-Judge

**Citation**: Multiple authors. "A Survey on LLM-as-a-Judge." *ScienceDirect*, 2025.

**Key finding**: Comprehensive survey covering 200+ papers on using LLMs as evaluators. Key taxonomies: pointwise (absolute scoring), pairwise (comparison), and listwise (ranking) evaluation modes. Identifies seven categories of bias: position, verbosity, self-enhancement, format, knowledge, authority, and sentiment bias. Recommends hybrid evaluation combining deterministic metrics with LLM judges, with calibration against human labels as the gold standard.

**Design motivation**: The survey's hybrid evaluation recommendation directly validates the tiered gate architecture: deterministic criteria (compile, test, lint) provide ground truth, while LLM judges add subjective quality assessment. The seven-bias taxonomy informs the anti-bias mitigations in the judge panel: position randomization, length normalization, cross-family paneling, structured rubrics, calibration tracking, and multi-sample averaging.

**Referenced by**: PRD-00 SS4, PRD-04 SS2-SS6.

---

## 2. Agent Evaluation Benchmarks

### SS2.1 SWE-bench and SWE-bench Verified

**Citation**: Jimenez, Yang, Wettig, Yao, Pei, Press, and Narasimhan. "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?" *Proceedings of the Twelfth International Conference on Learning Representations (ICLR)*, 2024.

**Verified variant**: OpenAI. "Introducing SWE-bench Verified." 2024. Human-validated subset of 500 instances ensuring clear problem descriptions, correct test patches, and solvable tasks.

**Key finding**: SWE-bench evaluates coding agents on real GitHub issues from 12 popular Python repositories. SWE-bench Verified addresses quality concerns in the original benchmark through human validation. As of early 2026, frontier models achieve 70-77% on the Verified subset (Claude 4 Sonnet at 77.2%, GPT-5 at 74.9%, Gemini 2.5 at 71.8%). The benchmark underwent a major v2.0.0 upgrade in February 2026 with updated scaffolding, environments, and token limits.

**Benchmark concerns (2026)**: OpenAI dropped SWE-bench Verified after an internal audit found that 59.4% of audited problems had flawed tests. This highlights a fundamental challenge: benchmark integrity degrades as models improve, because the same models used to game the benchmark can also expose its flaws. The BenchGuard paper (arXiv:2604.24955) uses frontier LLMs as systematic auditors of evaluation infrastructure itself.

**Design motivation**: SWE-bench's test-based evaluation model aligns with roko's deterministic gate approach. However, the benchmark's limitations motivate the multi-criterion evaluation framework: test passage alone is insufficient for assessing code quality. The system must also evaluate lint cleanliness, diff minimality, style consistency, and semantic correctness -- dimensions that SWE-bench deliberately does not measure.

**Referenced by**: PRD-00 SS1, PRD-03 SS2, PRD-08 SS2.

---

### SS2.2 SWE-bench Pro

**Citation**: Scale AI. "SWE-Bench Pro: A Rigorous and Realistic Evaluation of AI Agents for Software Engineering." 2025.

**Key finding**: SWE-bench Pro extends the original benchmark with more realistic constraints: repository-scale context, multi-file changes, and dependency management. It distinguishes between agents that can solve isolated functions versus agents that can navigate complex codebases. Performance drops 15-25% compared to SWE-bench Verified, suggesting that verified scores overstate real-world capability.

**Design motivation**: The performance gap between SWE-bench Verified and SWE-bench Pro validates the need for workspace-level evaluation (not just file-level). The eval framework's evidence collection must operate at repository scope, and criteria must assess cross-file consistency, import graph validity, and API contract preservation.

**Referenced by**: PRD-03 SS2.

---

### SS2.3 SWE-EVO: Evolutionary Benchmark Dynamics

**Citation**: Multiple authors. "SWE-EVO: Benchmarking Coding Agents in Evolving Software Environments." 2026. arXiv:2512.18470.

**Key finding**: Benchmarks derived from static GitHub issues fail to capture how developer tasks evolve over time. SWE-EVO introduces temporally evolving task instances where the codebase changes between task definition and evaluation. Agents that score well on static benchmarks often fail when the repository context has drifted, because they overfit to the exact state captured in the benchmark snapshot.

**Design motivation**: Supports the decision to evaluate against live workspace state rather than frozen snapshots. The eval framework always runs criteria against the current working directory, not a cached copy. This ensures that gate results reflect the actual state of the code, including any concurrent changes from parallel agent tasks in the same workspace.

**Referenced by**: PRD-00 SS2, PRD-08 SS3.

---

### SS2.4 HAL: Holistic Agent Leaderboard

**Citation**: Tian, Putnam, Vu, Chen, Ng, Marchetti-Bowick, Narasimhan, and Yao. "Holistic Agent Leaderboard: The Missing Infrastructure for AI Agent Evaluation." Princeton PNI, 2025. arXiv:2510.11977. https://hal.cs.princeton.edu/

**Key finding**: HAL provides a unified evaluation harness for reproducible, cost-controlled agent benchmarking with automated agent log analysis. The framework orchestrates parallel evaluations across hundreds of VMs, reducing evaluation time from weeks to hours while eliminating common implementation bugs. Validation through 21,730 agent rollouts across 9 models and 9 benchmarks in coding, web navigation, science, and customer service with a total cost of approximately $40,000. Currently includes 11 benchmarks: SWE-bench, USACO, AppWorld, CORE-bench, Cybench, AgentHarm, and others. A surprising finding: higher reasoning effort reduces accuracy in the majority of runs.

**Design motivation**: HAL's three-dimensional analysis (models x scaffolds x benchmarks) directly informs the bench infrastructure in roko-serve (`/api/bench/*` routes). The existing bench routes support model comparison, suite management, and Pareto frontier analysis. HAL's automated log analysis capability maps to the eval framework's `EvalTrace` -- structured evaluation results that can be programmatically analyzed for failure patterns, cost regressions, and capability gaps.

**HAL's insight about reasoning effort vs. accuracy** validates the CascadeRouter's approach of routing to the cheapest sufficient model rather than always using the most capable one. More reasoning is not always better; the eval framework must measure whether additional reasoning effort actually improves outcomes.

**Referenced by**: PRD-00 SS1, PRD-03 SS2, PRD-07 SS2.4.

---

### SS2.5 CORE-Bench: Computational Reproducibility

**Citation**: Siegel, Kapoor, and Narasimhan. "CORE-Bench: Fostering the Credibility of Published Research Through a Computational Reproducibility Agent Benchmark." *Transactions on Machine Learning Research*, 2025. arXiv:2409.11363.

**Key finding**: CORE-Bench evaluates agents on 270 tasks across 90 scientific papers in three disciplines (computer science, social science, medicine) at three difficulty levels. The best agent (CORE-Agent with GPT-4o) achieved only 19% accuracy on the hardest tasks, demonstrating vast scope for improvement in automating routine scientific tasks. The benchmark provides a parallelizable evaluation system that saves days of evaluation time per run.

**Design motivation**: CORE-Bench's emphasis on reproducibility -- can an agent reproduce stated results given code and data -- is directly analogous to the test gate's purpose: can the agent's code changes pass the project's test suite? The three difficulty levels (easy, medium, hard) inform the eval profile system's support for task complexity bands, where different criteria sets and thresholds apply depending on estimated task difficulty.

**Referenced by**: PRD-03 SS2.

---

### SS2.6 Saving SWE-Bench: Benchmark Mutation

**Citation**: Multiple authors. "Saving SWE-Bench: A Benchmark Mutation Approach for Realistic Agent Evaluation." 2025. arXiv:2510.08996.

**Key finding**: Proposes mutating benchmark instances to create fresh evaluation samples that are structurally similar to originals but immune to memorization. Mutation-based evaluation reveals that many agents overfit to specific benchmark instances through training data contamination. Mutated benchmarks show 10-30% lower scores than originals, suggesting significant contamination in reported results.

**Design motivation**: Reinforces the principle that roko's self-evaluation must operate on live workspace state, not frozen benchmarks. The eval framework's criteria evaluate the actual workspace after agent modifications, making memorization-based gaming impossible. The mutation approach is analogous to the generated test gate, which creates novel test cases for each evaluation rather than relying on a fixed test suite.

**Referenced by**: PRD-05 SS2.

---

### SS2.7 FeatureBench: Complex Feature Development

**Citation**: Multiple authors. "FeatureBench: Benchmarking Agentic Coding for Complex Feature Development." 2025/2026. OpenReview.

**Key finding**: Unlike SWE-bench's focus on bug fixes and isolated issues, FeatureBench evaluates agents on multi-file feature development tasks that require architectural decisions, API design, and cross-cutting changes. Agent performance drops significantly on these more realistic development tasks compared to issue-fixing benchmarks.

**Design motivation**: Validates the plan-driven execution model in roko. Complex feature development requires decomposition into subtasks, dependency management, and cross-file consistency -- exactly what the plan executor and DAG-based task ordering provide. The eval framework must assess not just individual file correctness but system-level coherence across all files modified by a feature implementation.

**Referenced by**: PRD-00 SS2, PRD-08 SS3.

---

## 3. Code Review Automation

### SS3.1 c-CRAB: Code Review Agent Benchmark

**Citation**: Multiple authors. "Code Review Agent Benchmark." 2025. arXiv:2603.23448.

**Key finding**: Introduces a benchmark that measures how well automated code review agents identify the same issues that human reviewers raise in realistic PR settings. Existing evaluation methods using textual overlap or embedding similarity primarily measure resemblance in wording rather than whether a review identifies meaningful issues. c-CRAB assesses issue identification accuracy independently of phrasing.

**Design motivation**: The eval framework's `Finding` type is designed to capture issue identification independently of phrasing. Each finding has a category, severity, and optional source location, allowing comparison between different evaluators based on what they found rather than how they described it. This enables meaningful comparison between LLM judge findings and deterministic gate findings.

**Referenced by**: PRD-04 SS3.

---

### SS3.2 CR-Bench: Code Review Utility

**Citation**: Multiple authors. "CR-Bench: Evaluating the Real-World Utility of AI Code Review Agents." 2025. arXiv:2603.11078.

**Key finding**: Evaluates the real-world utility of AI code review, distinguishing between comments that are actionable (developer should change something) versus informational (context or style suggestions). The best AI code review agents achieve approximately 98% precision on actionable comments, meaning nearly every comment they produce is worth acting on. However, recall remains lower -- they miss issues that human reviewers catch.

**Design motivation**: The high-precision, lower-recall characteristic of AI code review maps directly to the judge panel's role in the eval framework. Judge findings are treated as high-confidence signals when present, but their absence does not imply quality. This is why deterministic criteria (compile, test, lint) handle recall while judge criteria handle precision-focused quality assessment.

**Referenced by**: PRD-04 SS4.

---

### SS3.3 Claude Code Review

**Citation**: Anthropic. "Introducing Claude Code Review." March 2026.

**Key finding**: A multi-agent PR review system where multiple specialized agents analyze a diff in parallel, and a verification step filters false positives. The architecture separates concern-specific analysis (security, performance, correctness, style) from aggregation, with each specialist operating on relevant code sections independently.

**Design motivation**: The parallel multi-specialist architecture directly validates the eval framework's criterion-based decomposition. Each criterion is analogous to a specialized reviewer: CompileCriterion checks build integrity, TestCriterion checks functional correctness, LintCriterion checks style, SecurityCriterion checks vulnerabilities. The aggregation into a composite `EvalVerdict` mirrors Claude Code Review's verification step.

**Referenced by**: PRD-01 SS4, PRD-04 SS4.

---

### SS3.4 BenchGuard: Auditing AI Agent Benchmarks

**Citation**: Multiple authors. "BenchGuard: Who Guards the Benchmarks? Automated Auditing of LLM Agent Benchmarks." 2026. arXiv:2604.24955.

**Key finding**: Frontier LLMs can be employed as systematic auditors of evaluation infrastructure. BenchGuard identified 12 defects in ScienceAgentBench despite multiple rounds of manual validation, and the technique contributed to OpenAI's finding that 59.4% of SWE-bench Verified problems had flawed tests. This raises the meta-question: how do you evaluate the evaluators?

**Design motivation**: Directly motivates the calibration and meta-evaluation features in PRD-07. The eval framework must include self-auditing capabilities: periodically running the judge panel against known-good and known-bad examples to detect calibration drift. The `roko eval calibrate` command runs this audit, and the calibration history is tracked in `.roko/eval/calibration/`.

**Referenced by**: PRD-07 SS2.4, PRD-05 SS4.

---

## 4. Visual and Multimodal Code Analysis

### SS4.1 UICrit

**Citation**: Duan, Sunkara, Nichols, Branham, and Apte. "UICrit: Enhancing Automated Design Evaluation with a UICrit Dataset." *Proceedings of the 37th ACM Symposium on User Interface Software and Technology (UIST)*, 2024. arXiv:2407.08850.

**Key finding**: Dataset of 1,000 RICO mobile UIs with 3,059 critiques, bounding boxes, and multi-axis ratings by 7 professional designers. Few-shot plus visual prompting with UICrit's annotation schema achieved +55% improvement in LLM feedback quality. Critical finding: app-store rating correlates r=0.007-0.023 with expert aesthetic ratings -- aggregate user ratings are useless as quality signal.

**Design motivation**: The bounding-box-grounded critique format is the template for the `Finding` type's location field. Every visual critique grounds itself to a specific region of the screenshot, not just a verbal description. The finding that user ratings are uncorrelated with expert quality ratings validates that evaluation must come from calibrated judges, not satisfaction surveys.

**Referenced by**: PRD-04 SS5, PRD-04 SS7.3.

---

### SS4.2 MMCode: Multimodal Code Benchmarking

**Citation**: Li, Zhang, Liang, et al. "MMCode: Benchmarking Multimodal Large Language Models for Code Generation with Visually Rich Programming Problems." *Proceedings of the Thirteenth International Conference on Learning Representations (ICLR)*, 2025. arXiv:2404.09486.

**Key finding**: The first multi-modal coding dataset evaluating algorithmic problem-solving in visually rich contexts, containing 3,548 questions and 6,620 images from real-world programming challenges. The benchmark reveals that current multimodal models struggle significantly with visual reasoning in coding contexts, with accuracy dropping 30-50% when problems require understanding diagrams, charts, or visual layouts compared to text-only equivalents.

**Design motivation**: The significant accuracy drop for visual reasoning in coding contexts establishes a baseline expectation for the visual gate's judge panel. Visual quality assessment is inherently harder than text-based code evaluation, and the system must set appropriate confidence thresholds. This supports using pairwise comparison (which requires only ordinal ranking) rather than absolute scoring (which requires calibrated interval scales).

**Referenced by**: PRD-04 SS2.

---

### SS4.3 ScreenCoder: Visual-to-Code Generation

**Citation**: Multiple authors. "ScreenCoder: Advancing Visual-to-Code Generation for Front-End Automation via Modular Multimodal Agents." 2025/2026. OpenReview.

**Key finding**: A modular multi-agent system for converting UI screenshots and designs into HTML/CSS/JS code. The system achieves pixel-level accuracy on standard UI components and supports iterative refinement where users highlight areas and provide natural-language instructions. The modular architecture separates layout detection, component recognition, and code generation into specialized agents.

**Design motivation**: ScreenCoder's screenshot-to-code direction is the inverse of the eval framework's code-to-screenshot-to-judgment direction. Both require reliable visual understanding of rendered UI. The fact that modular multi-agent approaches outperform monolithic models for visual code tasks validates the eval framework's decomposition of visual assessment into separate criteria (layout_integrity, responsive_quality, visual_polish, etc.) evaluated independently.

**Referenced by**: PRD-03 SS3, PRD-06 SS2.

---

### SS4.4 Multimodal LLMs and Programming Screenshots

**Citation**: Multiple authors. "Do Multimodal LLMs Understand Programming Screenshots? Inferring Questions and Extracting Relevant Content." *Empirical Software Engineering*, Springer, 2026.

**Key finding**: Assesses whether LLMs can infer the intent and content of a programming query directly from a screenshot, without relying on textual input. Current multimodal models can extract code, identify error messages, and infer developer intent from IDE screenshots with moderate reliability (60-75% accuracy), but struggle with visual context like scroll position, split panes, and overlapping windows.

**Design motivation**: Establishes the capabilities and limitations of using screenshots as evaluation evidence. The eval framework's screenshot capture must be carefully controlled: clean viewport, no overlapping elements, consistent rendering. The 60-75% accuracy for uncontrolled screenshots motivates the framework's use of programmatic evidence collection (DOM analysis, console log capture) alongside visual screenshots, ensuring that deterministic facts supplement the visual assessment.

**Referenced by**: PRD-06 SS3.

---

## 5. Process Reward and Verification Models

### SS5.1 Process Reward Models for Code

**Citation**: Lightman, Kosaraju, Burda, Edwards, Baker, Lee, Leike, Schulman, Sutskever, and Cobbe. "Let's Verify Step by Step." *Proceedings of the Twelfth International Conference on Learning Representations (ICLR)*, 2024.

**Key finding**: Process reward models (PRMs) that evaluate each step of reasoning outperform outcome reward models (ORMs) that only evaluate final answers. For mathematical reasoning, step-level verification catches errors earlier and enables better search strategies. The key insight is that verifying intermediate steps is more sample-efficient than verifying only outcomes.

**Design motivation**: The gate pipeline's sequential structure is a form of process verification. Each rung checks a specific property at a specific stage: compile before lint, lint before test, test before visual. The eval framework extends this by allowing per-criterion evidence accumulation -- each criterion can access evidence collected by prior criteria, enabling richer verification contexts.

**Referenced by**: PRD-00 SS4, PRD-01 SS3.

---

### SS5.2 VIS-Shepherd: Critic Construction for Vision

**Citation**: Multiple authors. "VIS-Shepherd: Constructing Critic for LLM-based Data Visualization." 2025. arXiv:2506.13326.

**Key finding**: Constructs specialized critic models for evaluating LLM-generated data visualizations. The critic evaluates both the correctness of the visualization (does it accurately represent the data?) and the quality (is it readable, aesthetically pleasing, following best practices?). Training the critic on paired examples of good and bad visualizations significantly improves its evaluation accuracy over general-purpose models.

**Design motivation**: The dual correctness/quality evaluation framework directly maps to the eval framework's deterministic + visual criterion split. Deterministic criteria verify correctness (does the code compile? do tests pass?), while visual criteria verify quality (is the UI well-designed? is the layout responsive?). The finding that domain-specific training improves evaluation accuracy supports future fine-tuning of judge models on roko-specific evaluation data.

**Referenced by**: PRD-04 SS4, PRD-06 SS2.

---

## 6. Agent Reliability and Safety

### SS6.1 AgentProp-Bench: Tool-Using Agent Evaluation

**Citation**: Multiple authors. "Evaluating Tool-Using Language Agents: Judge Reliability, Propagation Cascades, and Runtime Mitigation in AgentProp-Bench." 2026. arXiv:2604.16706.

**Key finding**: Introduces the concept of "propagation cascades" -- where early errors in tool-using agent workflows compound through subsequent steps. Judge reliability varies significantly based on where in the cascade the evaluation occurs: judges are more reliable at evaluating final outcomes than intermediate states. Runtime mitigation strategies (checkpointing, rollback) can reduce cascade severity by 40-60%.

**Design motivation**: Propagation cascades are exactly what the gate pipeline's short-circuit behavior mitigates. If compile fails, there is no point running tests, lint, or visual evaluation -- the cascade would produce meaningless results. The eval framework preserves this short-circuit behavior: required criteria that fail prevent subsequent criteria from executing. The finding about checkpoint/rollback reducing cascade severity validates the orchestrator's retry-with-replan strategy.

**Referenced by**: PRD-08 SS3.3.

---

### SS6.2 How We Broke Top AI Agent Benchmarks

**Citation**: UC Berkeley RDI. "How We Broke Top AI Agent Benchmarks." 2025/2026.

**Key finding**: Systematic analysis of how benchmark scores can be inflated through training data contamination, prompt optimization against specific benchmark instances, and exploitation of evaluation harness weaknesses. The analysis found that many reported improvements in agent benchmarks are partially or fully attributable to these artifacts rather than genuine capability improvements.

**Design motivation**: Establishes the importance of evaluating against live workspace state rather than fixed benchmarks. The eval framework's criteria run against the actual project codebase, not a frozen benchmark snapshot. The adaptive threshold system (which adjusts gate difficulty based on observed pass rates) provides a dynamic evaluation standard that evolves with agent capability, making it resistant to the static-benchmark gaming described in this analysis.

**Referenced by**: PRD-05 SS2.

---

## 7. Calibration and Meta-Evaluation

### SS7.1 Preference Leakage in LLM-as-a-Judge

**Citation**: Multiple authors. "Preference Leakage: A Contamination Problem in LLM-as-a-Judge." 2024/2025.

**Key finding**: When the same model family generates outputs and evaluates them (or when training data for the evaluator overlaps with the generator's output distribution), systematic preference biases emerge that are invisible to standard calibration tests. The contamination is structural, not incidental -- it arises from shared representation spaces.

**Design motivation**: Reinforces the mandatory cross-family constraint for judge panels. The system tracks model family lineage and prevents same-family generator-judge pairings. The eval framework's panel composition enforces this: each panel member must be from a different model family, and the system logs warnings when family diversity is insufficient.

**Referenced by**: PRD-04 SS4.1.

---

### SS7.2 Judge's Verdict: Human Agreement Analysis

**Citation**: Multiple authors. "Judge's Verdict: A Comprehensive Analysis of LLM Judge Capability Through Human Agreement." 2025. OpenReview.

**Key finding**: Comprehensive analysis of when LLM judges agree and disagree with human evaluators. Agreement is highest for clear-cut quality differences (>80% agreement) and lowest for subtle quality distinctions (<50% agreement). Task complexity is the strongest predictor of judge reliability -- judges are reliable for well-defined tasks but unreliable for ambiguous or creative tasks.

**Design motivation**: The agreement-varies-with-task-complexity finding directly informs the eval framework's tiered approach. For well-defined tasks (compile, test), deterministic criteria provide reliable ground truth. For ambiguous quality dimensions (visual polish, design system fit), multi-judge panels provide probabilistic estimates. The framework never relies on a single judge for high-stakes decisions.

**Referenced by**: PRD-04 SS6.

---

### SS7.3 Checklist Engineering for Multilingual Judges

**Citation**: Multiple authors. "Checklist Engineering Empowers Multilingual LLM Judges." 2025. arXiv:2507.06774.

**Key finding**: Structured checklists significantly improve LLM judge consistency across languages and evaluation domains. Checklists reduce the variance of judge outputs by constraining the evaluation to specific, answerable questions. The effect is strongest for judges that otherwise exhibit high variance, suggesting that checklists serve as a regularizer for noisy evaluation processes.

**Design motivation**: The eval framework's profile system -- which defines an ordered list of criteria with specific thresholds -- is a structured checklist. Each criterion asks a specific question ("Does the code compile?", "Do tests pass?", "Is the layout responsive at 320px?"). This structure reduces judge variance compared to open-ended quality assessment.

**Referenced by**: PRD-01 SS3, PRD-04 SS4.

---

## 8. Cost Optimization and Efficiency

### SS8.1 Reasoning Effort vs. Accuracy

**Citation**: HAL analysis (see SS2.4). Finding extracted from 21,730 agent rollouts.

**Key finding**: Higher reasoning effort (more thinking tokens, longer chain-of-thought) reduces accuracy in the majority of evaluation runs. This is counterintuitive but robust across multiple benchmarks and models. The effect is strongest for well-defined tasks where the answer space is constrained.

**Design motivation**: Validates the CascadeRouter's approach of starting with cheaper, faster models and only escalating to more expensive models when cheaper ones fail. For deterministic criteria (compile, test, lint), no LLM reasoning is needed at all. For visual and judge criteria, the framework uses the minimum necessary model capability, tracked by the cost field in `EvalTrace`. The `per_task_budget` configuration caps the total evaluation cost.

**Referenced by**: PRD-05 SS3, PRD-08 SS5.

---

### SS8.2 Pareto Frontiers in Agent Benchmarking

**Citation**: HAL analysis (see SS2.4). Pareto analysis extracted from multi-model multi-benchmark evaluation.

**Key finding**: When plotting accuracy vs. cost across models and scaffolds, the Pareto frontier is sparse -- only 3-4 model-scaffold combinations are Pareto-optimal across a given benchmark suite. Many high-cost configurations are strictly dominated by cheaper alternatives that achieve equal or better accuracy.

**Design motivation**: The existing bench infrastructure (`/api/bench/pareto` endpoint in `crates/roko-serve/src/routes/bench.rs`) already computes Pareto frontiers for model comparison. The eval framework extends this to criterion-level Pareto analysis: for each criterion type, what is the cheapest configuration that achieves acceptable accuracy? This drives the adaptive criterion configuration where the system learns which criteria can use cheaper evaluation methods.

**Referenced by**: PRD-07 SS2.4.

---

## 9. Gate Pipeline Theory

### SS9.1 Sequential Verification with Short-Circuit

**Key principle**: The 7-rung gate pipeline implements a form of cascading verification where failures at lower rungs (compile) prevent execution of higher rungs (test, judge). This is not arbitrary ordering -- it reflects dependency: tests cannot run if compilation fails, visual assessment is meaningless for non-functional code.

**Formal property**: Let G_0, G_1, ..., G_6 be gates ordered by rung. The pipeline satisfies: if G_i fails, then for all j > i, G_j's result is either "skipped" (short-circuit mode) or "unreliable" (full-pipeline mode). The eval framework preserves this property: `required = true` criteria that fail prevent subsequent criteria from executing.

**Implementation**: `crates/roko-gate/src/gate_pipeline.rs`, lines 207-292. The `GatePipeline::verify` method iterates gates in push order, accumulating verdicts and optionally short-circuiting.

---

### SS9.2 Adaptive Gate Skipping

**Key principle**: Gates with consistently high pass rates provide diminishing information value. The adaptive threshold system (EMA per rung, skip when consecutive passes exceed threshold) reduces evaluation cost without significantly reducing error detection. Rung 0 (compile) is never skipped because compilation failure is catastrophic.

**Implementation**: `crates/roko-gate/src/adaptive_threshold.rs`. The `AdaptiveThresholds::should_skip_rung` method returns true when a rung's consecutive pass count exceeds the configured threshold (default: 20). The eval framework's `AdaptiveCriterionThresholds` (PRD-08 SS3.5) extends this per-criterion.

---

### SS9.3 Composition Modes

**Key principle**: Different verification scenarios require different composition strategies. Sequential composition catches dependent failures early. Parallel composition maximizes throughput for independent criteria. Voting composition provides robustness when individual evaluators are noisy. Fallback composition handles graceful degradation.

**Implementation**: `crates/roko-gate/src/gate_pipeline.rs`, `ComposedGatePipeline` with `GateComposition` enum. The eval framework maps these modes to profile-level composition: deterministic criteria run sequentially (dependent), visual criteria run in parallel (independent), judge criteria use voting (noisy).

---

## 10. Evaluation Data Flow Architecture

### SS10.1 Four-Sink Verdict Pattern

**Key principle**: Every gate verdict flows to four independent sinks simultaneously:
1. **Runtime event bus** (live streaming to SSE/WS clients)
2. **Episode logger** (durable per-task records for learning)
3. **Efficiency log** (cost and performance metrics)
4. **TUI verdict aggregator** (rolling statistics for dashboard)

This fan-out ensures that no single sink failure prevents other consumers from receiving verdict data. The eval framework preserves this pattern, adding a fifth sink: the eval trace store (`.roko/eval/traces/traces.jsonl`).

**Implementation**: `crates/roko-cli/src/orchestrate.rs`, scattered across `run_gate_pipeline`, episode construction, and efficiency event emission.

---

### SS10.2 Evidence Bag Architecture

**Key principle**: The `EvidenceBag` type (PRD-01) provides a typed, structured container for evaluation inputs. Unlike the legacy system's `Engram` (which embeds evidence as serialized JSON in the body), the EvidenceBag maintains typed artifact references with MIME types, file paths, and metadata.

**Design rationale**: Typed evidence enables two capabilities impossible with the legacy system:
1. **Criterion-specific evidence requirements**: Each criterion declares what evidence types it needs. The runner validates that required evidence is present before invoking the criterion.
2. **Evidence provenance tracking**: Each artifact records how it was collected (which collector, when, from where), enabling audit trails for evaluation decisions.

---

## 11. Open Research Questions

### SS11.1 Calibration Drift in Self-Developing Systems

When roko evaluates its own code improvements, the evaluator and the evaluated system co-evolve. If the evaluation framework itself is modified by agents that are evaluated by that framework, circular dependencies arise. Research question: how do you detect and prevent calibration drift in self-referential evaluation systems?

**Current mitigation**: The eval framework uses external signals (compiler output, test results) as ground truth anchors that are immune to model drift. Judge calibration is tracked against a fixed human-labeled dataset that is never modified by agents.

---

### SS11.2 Cross-Language Evaluation Parity

The current gate pipeline is Rust-specific (cargo build, cargo clippy, cargo test). The eval framework's criteria must eventually support multiple languages (TypeScript, Go, Python). Research question: how do you maintain evaluation parity across languages with different tooling maturity and static analysis capabilities?

**Current approach**: Language-specific criteria are implemented in `roko-lang-*` crates. The eval profile system allows language-specific profiles with different criteria sets and thresholds.

---

### SS11.3 Human-in-the-Loop Calibration Scalability

The judge panel's calibration relies on human-labeled examples. As the system evaluates more diverse task types, the calibration dataset must grow. Research question: how do you scale human labeling to keep pace with evaluation diversity without prohibitive cost?

**Current approach**: Active learning -- the system identifies evaluation instances where judge panel members disagree most strongly and prioritizes those for human labeling, maximizing calibration improvement per human judgment.

---

### SS11.4 Evaluation Cost Scaling

As the eval framework adds criteria (visual, judge, benchmark regression), the per-task
evaluation cost increases. Research question: how do you maintain evaluation thoroughness
while keeping costs bounded, especially for systems that evaluate thousands of tasks
per day?

**Current approach**: Adaptive gate skipping removes high-pass-rate criteria from routine
evaluations. Budget caps (`per_task_budget`, `per_plan_budget`) enforce hard cost limits.
The CascadeRouter selects the cheapest sufficient model for each criterion. Future work:
the RocketEval finding (SS1.7) suggests that lightweight models can handle criterion-level
grading, reserving expensive frontier models for holistic assessment only.

---

### SS11.5 Temporal Stability of Visual Evaluation

UI screenshots are inherently non-deterministic -- font rendering, anti-aliasing,
animation timing, and browser version all affect pixel-level output. Research question:
how do you establish stable visual baselines when the rendering environment itself
introduces variance?

**Current approach**: The visual gate uses structural comparison (DOM tree, layout boxes,
computed styles) alongside pixel comparison, reducing sensitivity to rendering
non-determinism. Perceptual hashing provides a fuzzy match that tolerates sub-pixel
rendering differences. Future work: investigating deterministic rendering environments
(e.g., Playwright with fixed fonts and disabled anti-aliasing) to reduce visual noise.

---

### SS11.6 Judge Panel Composition Optimization

The PoLL finding (SS1.2) establishes that family diversity is critical for panel
quality. But the optimal number of panel members, the best diversity metric, and
the interaction between panel composition and task type remain open questions.

**Current approach**: Three-member panels with one member per model family. Future work:
dynamically selecting panel composition based on task type and historical agreement
data, potentially reducing to two-member panels for well-calibrated criterion types.

---

## 12. Methodology Notes

### SS12.1 Citation Standards

All citations in this appendix follow these standards:

- **Peer-reviewed**: Full author list, title, venue, year. Preference for published
  proceedings (ICML, ICLR, NeurIPS, UIST, ACL) over preprints.
- **Preprint only**: Author list, title, arXiv identifier, year. Clearly marked as
  preprint. Used when no published version exists.
- **Industry reports**: Organization, title, date. Used for tool announcements and
  benchmark results that have no academic publication.
- **Codebase references**: File path, function name, line numbers. Used for design
  decisions grounded in existing roko implementation rather than external research.

### SS12.2 Design Decision Traceability

Every design decision in PRD-00 through PRD-08 should be traceable to one of:

1. **Research finding**: Cited in this appendix with section number
2. **Engineering constraint**: Documented in CLAUDE.md or GAPS.md
3. **User requirement**: Captured in a PRD or issue
4. **Empirical observation**: Measured in roko's own evaluation data

Decisions without traceability should be flagged as assumptions and validated
through the eval framework's calibration process.

### SS12.3 Living Document Policy

This appendix is a living document. As new research is published and as roko's
evaluation data accumulates, entries should be:

- **Added**: When a new design decision references external research
- **Updated**: When follow-up papers qualify or contradict earlier findings
- **Deprecated**: When a finding is superseded by stronger evidence (marked with
  `[SUPERSEDED by SSX.Y]` but not deleted)
- **Validated**: When roko's own data confirms or contradicts a research finding
  (added as a "Validation note" subsection)

### SS12.4 Research Gap Tracking

The following research gaps have been identified but not yet filled:

| Gap | Relevant to | Priority |
|---|---|---|
| Long-term calibration stability of multi-model panels | PRD-04 | High |
| Cross-language visual evaluation consistency | PRD-06 | Medium |
| Optimal criterion ordering beyond compile-first | PRD-08 | Medium |
| Cost-quality Pareto frontiers for judge panels | PRD-04, PRD-05 | High |
| Temporal stability of screenshot-based baselines | PRD-06 | Medium |
| Self-referential evaluation in self-hosting systems | PRD-05 | High |
| Curriculum learning for adaptive gate thresholds | PRD-08 | Low |
| Privacy-preserving evaluation for proprietary codebases | PRD-07 | Medium |

These gaps should be addressed through targeted literature review as each
becomes blocking for implementation.

---

## 13. Glossary of Evaluation Terms

| Term | Definition | PRD Reference |
|---|---|---|
| **Criterion** | A single evaluable dimension (e.g., "compile", "lint", "visual_polish") | PRD-01 |
| **Profile** | An ordered list of criteria with weights and thresholds | PRD-01 |
| **EvidenceBag** | Typed container of evaluation inputs (artifacts, metadata) | PRD-01 |
| **ArtifactRef** | Reference to a stored evaluation artifact (screenshot, log, diff) | PRD-01 |
| **Finding** | A specific issue identified by a criterion (category, severity, location) | PRD-01 |
| **EvalTrace** | Complete record of one evaluation execution | PRD-01 |
| **EvalVerdict** | Composite pass/fail/error result of all criteria in a profile | PRD-01 |
| **CriterionResult** | Result of evaluating one criterion (pass/fail, score, findings) | PRD-01 |
| **Judge Panel** | Multiple LLM evaluators from disjoint model families | PRD-04 |
| **Pairwise Comparison** | Evaluating by comparing two candidates rather than scoring one | PRD-04 |
| **Calibration** | Measuring judge agreement with human-labeled ground truth | PRD-07 |
| **Rung** | Position in the sequential gate pipeline (0=compile through 6=judge) | PRD-08 |
| **Adaptive Threshold** | Dynamic gate skip/execute decision based on historical pass rate | PRD-08 |
| **Bridge Layer** | Adapters between legacy Verify/Verdict and new Criterion/CriterionResult | PRD-08 |
| **Short-Circuit** | Stopping pipeline execution after the first failure | PRD-08 |
| **Evidence Collector** | Component that gathers evaluation inputs (compiler, browser, etc.) | PRD-01 |
| **Deterministic Criterion** | Criterion with reproducible pass/fail (compile, test, lint) | PRD-01 |
| **Statistical Criterion** | Criterion with probabilistic output (benchmark regression) | PRD-01 |
| **Visual Criterion** | Criterion that evaluates rendered visual output (screenshots) | PRD-03 |
| **Judge Criterion** | Criterion that uses LLM evaluation (pairwise comparison) | PRD-04 |
