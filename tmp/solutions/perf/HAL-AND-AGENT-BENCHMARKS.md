# HAL and Agent Benchmarks: Comprehensive Research

Research date: 2026-04-29
Status: Comprehensive survey of agent evaluation state-of-the-art (2025-2026)

---

## Table of Contents

1. [HAL: Holistic Agent Leaderboard](#1-hal-holistic-agent-leaderboard)
2. [Major Code and Agent Benchmarks](#2-major-code-and-agent-benchmarks)
3. [SWE-bench Ecosystem: Verified, Pro, and Contamination](#3-swe-bench-ecosystem)
4. [Novel Evaluation Techniques](#4-novel-evaluation-techniques)
5. [LLM-as-Judge for Software Engineering](#5-llm-as-judge-for-software-engineering)
6. [Agent Reliability Beyond Accuracy](#6-agent-reliability-beyond-accuracy)
7. [State of the Art Summary Table](#7-state-of-the-art-summary-table)
8. [Roko Benchmark Suite Design](#8-roko-benchmark-suite-design)

---

## 1. HAL: Holistic Agent Leaderboard

### What It Is

HAL (Holistic Agent Leaderboard) is a standardized infrastructure for systematic,
multidimensional evaluation of AI agent systems across real-world tasks. Built at
Princeton's PLI (Programming Languages and Intelligence) lab, HAL provides both
a reproducible evaluation harness and a public leaderboard.

Paper: "Holistic Agent Leaderboard: The Missing Infrastructure for AI Agent
Evaluation" (arXiv:2510.11977). Accepted to ICLR 2026.

Source: https://hal.cs.princeton.edu/
GitHub: https://github.com/princeton-pli/hal-harness

### Architecture

HAL's architecture has three layers:

**Harness layer.** A Python framework that accepts any agent exposing a minimal
API and orchestrates its evaluation across diverse benchmarks. The harness
integrates with:
- Weave for comprehensive logging (2.5B tokens of LLM calls shared publicly)
- LiteLLM for cross-model compatibility
- Multiple execution environments: local, Docker, and Azure VMs

**Infrastructure layer.** Automates provisioning, execution, and teardown of
hundreds of Azure VMs, shifting agent evaluations from weeks-long, error-prone
batches to hour-scale reproducible processes. The validated infrastructure ran
21,730 agent rollouts across 9 models and 9 benchmarks with a total cost of
approximately $40,000.

**Analysis layer.** Three-axis evaluation framework:
1. Accuracy -- the standard pass/fail on benchmark tasks
2. Cost -- dollar-denominated cost of LLM API calls per task
3. Reliability -- consistency, robustness, predictability, safety

### Benchmarks Included in HAL

HAL covers four major agent domains with nine specific benchmarks:

| Domain | Benchmark | What It Tests |
|---|---|---|
| Software Engineering | SWE-bench Verified | Real GitHub issue resolution |
| Software Engineering | USACO | Competitive programming |
| Scientific Research | CORE-Bench Hard | Computational reproducibility of papers |
| Scientific Research | SciCode | Scientific coding tasks |
| Scientific Research | ScienceAgentBench | End-to-end science workflows |
| Web Navigation | Online Mind2Web | Web navigation with live sites |
| General Assistance | GAIA | General AI assistant tasks |
| General Assistance | AssistantBench | Complex assistant workflows |
| Customer Service | TAU-bench Airline | Airline customer service scenarios |

### How HAL Works -- Evaluation Pipeline

1. **Agent registration.** Agent implementors provide a Python class conforming
   to HAL's agent protocol (initialization, step, and cleanup hooks).

2. **Benchmark selection.** Evaluators choose which benchmarks to run, configure
   batch sizes, and specify execution environments.

3. **VM orchestration.** For isolated execution, HAL provisions Azure VMs with
   per-instance environments. Each benchmark instance gets a clean environment.

4. **Parallel evaluation.** Rollouts run in parallel across VMs. Each agent-task
   pair produces a structured log of all LLM calls, tool invocations, and
   environment state changes.

5. **Scoring.** Per-benchmark scoring functions compute accuracy. HAL additionally
   computes cost (total LLM API spend) and reliability metrics.

6. **Aggregation.** Results flow into the public leaderboard with sortable
   columns for accuracy, cost, and cost-controlled accuracy.

### HAL Reliability Dashboard

In 2026, HAL shifted focus from raw accuracy leaderboards to measuring
reliability. The reliability dashboard (https://hal.cs.princeton.edu/reliability/)
decomposes agent reliability into four dimensions with twelve concrete metrics:

**Consistency.** Does the agent produce the same outcome when run multiple times
on the same task?
- Distribution consistency: do similar action distributions emerge across runs?
- Sequence consistency: do identical action orderings emerge across runs?
- Key finding: agents show substantially higher distribution consistency than
  sequence consistency. They reliably select similar action types but vary in
  execution order.

**Robustness.** Does the agent maintain performance under input perturbations?
- Epsilon-level perturbations to task descriptions
- Environmental noise (e.g., changed file paths, modified outputs)
- Infrastructure fault injection

**Predictability.** Can we predict which tasks the agent will solve?
- Calibration: alignment between predicted confidence and actual accuracy.
  Improved in recent frontier models.
- Discrimination: ability to distinguish solvable tasks from unsolvable ones.
  Shows divergent trends -- in some cases worsened despite accuracy gains.

**Safety.** When the agent fails, how severe is the failure?
- Bounded severity: does the agent avoid catastrophic actions on failure?
- Recovery: can the agent detect and recover from partial failures?

Critical finding: despite steady accuracy improvements over 18 months of model
releases, reliability shows only modest overall improvement. Accuracy and
reliability are not correlated.

---

## 2. Major Code and Agent Benchmarks

### HumanEval and MBPP (Baselines)

**HumanEval** (OpenAI, 2021). 164 hand-written Python programming problems with
function signatures and docstrings. The agent writes a function body; correctness
is verified by unit tests. Originally designed for standalone code generation,
not multi-file engineering.

- Measures: isolated function synthesis
- Limitation: no codebase navigation, no multi-file changes, no tool use
- Current SOTA: effectively saturated. Multiple models score 95%+ on EvalPlus.

**MBPP** (Google, 2021). 974 mostly basic Python programming problems. Simpler
than HumanEval. Useful as a lower-bound baseline.

- Measures: basic code synthesis
- Limitation: trivial tasks that do not reflect real engineering
- Current SOTA: saturated alongside HumanEval

### SWE-bench Family

See dedicated section below.

### FeatureBench (ICLR 2026)

FeatureBench evaluates agentic coding for complex, feature-oriented software
development -- not bug fixes but building entire new features.

Source: https://arxiv.org/abs/2602.10975
GitHub: https://github.com/LiberCoders/FeatureBench

**Design.** 200 challenging tasks derived from 24 open-source repositories with
3,825 executable environments. Uses an execution-based evaluation protocol and a
scalable test-driven method that automatically derives tasks from repositories
with minimal human effort.

**Key result.** Claude Opus 4.5, which achieves 74.4% on SWE-bench Verified,
succeeds on only 11.0% of FeatureBench tasks. This dramatic gap reveals that
bug-fix benchmarks vastly overestimate agent capability for greenfield feature
development.

**Why it matters.** Feature development requires architectural reasoning, API
design, integration with existing patterns, and multi-file coordination --
capabilities not tested by issue-resolution benchmarks.

### AgencyBench (ACL 2026)

AgencyBench evaluates autonomous agents on 1M-token real-world contexts across
six core agentic capabilities.

Source: https://arxiv.org/abs/2601.11044
GitHub: https://github.com/GAIR-NLP/AgencyBench

**Scale.** 32 distinct scenarios and 138 specific tasks:
- Game Development: 50 tasks (36.2%) -- continuous state, physics, complex logic
- Code: 29 tasks -- full-stack (15 frontend, 15 backend) + 10 MCP tool use tasks
- Research: multi-step literature analysis
- Front-end, Back-end, MCP categories explicitly balanced

**Demands.** Each scenario requires multi-million-token context retention (mean
approx. 1M tokens), extensive tool use (approx. 90 invocations per scenario), and
execution spanning several hours.

### CORE-Bench

CORE-Bench evaluates agent ability to computationally reproduce scientific
papers: 270 tasks from 90 papers across computer science, social science, and
medicine, written in Python or R.

**CORE-Bench Hard** (featured in HAL) gives the agent only the codebase of a
paper. The agent must install all libraries and dependencies, run the code, and
read output/figures to answer questions about the paper.

### Terminal-Bench 2.0

89 carefully curated tasks in computer terminal environments inspired by real
workflows, ranging from model training to system administration.

Source: https://www.tbench.ai/

**Current leaderboard (April 2026):**
- Claude Opus 4.7: 68.54%
- Gemini 3.1 Pro Preview: 67.42%
- GPT-5.3 Codex: 64.05%
- GPT-5.5: 62.92%

### Vibe Code Bench

First benchmark testing LLMs' ability to generate a complete working web
application from only a text specification.

Source: https://www.vals.ai/benchmarks/vibe-code

**Design.** 100 realistic application specifications (50 public, 50 test) paired
with 964 automated browser workflows (10,131 substeps). Models build applications
from scratch in a sandboxed environment with browser, terminal, and production
services.

**Current leaderboard:** Claude Opus 4.7 at 71.00%, GPT-5.5 at 69.85%.

### Agentic Code Security Benchmark (Endor Labs)

Extends the SusVibes framework from Carnegie Mellon. Evaluates 200 real-world
tasks from 108 open-source projects covering 77 CWE vulnerability classes.

Key finding: top-performing AI coding agents pass functional tests but still fail
security. Agents that score highest on SWE-bench do not necessarily produce the
most secure code.

---

## 3. SWE-bench Ecosystem

### SWE-bench Verified

2,294 real GitHub issues from 12 Python repositories. The "Verified" subset was
human-validated to ensure problem statements and test patches are correct.

**Contamination crisis (February 2026).** OpenAI's Frontier Evals team published
an audit showing:
- Every major frontier model (GPT-5.2, Claude Opus 4.5, Gemini 3 Flash) could
  reproduce verbatim gold patches for some Verified tasks
- The 500 Python tasks appeared in model training data before benchmark publication
- 59.4% of the hardest unsolved problems had flawed test cases
- 49 tests were too narrowly defined to accept functionally correct submissions
- 26 tests were "too wide" -- testing features never described in the problem

OpenAI stopped reporting Verified scores in early 2026 and recommends SWE-bench
Pro instead. Source: https://openai.com/index/why-we-no-longer-evaluate-swe-bench-verified/

### SWE-bench Pro

Scale AI's response to the contamination problem. Key differences from Verified:
- Uses GPL-style copyleft repositories and private proprietary codebases
- Excludes trivial edits; retains only problems requiring substantial multi-file
  modifications
- Reference solutions average 107.4 lines of code across 4.1 files
- Legal and access barriers reduce contamination likelihood

**Performance gap reveals contamination extent:**
- Claude Opus 4.5: 80.9% on Verified, drops to approximately 23% on Pro
- GPT-5: 23.3% on Pro (highest to date under unified scaffold)

This 3x+ gap between Verified and Pro scores is strong evidence that Verified
scores reflected memorization, not generalization.

### Benchmark Selection Guidance

Match benchmarks to agent task profile:
- Autocomplete: HumanEval/MBPP (saturated, but useful as baseline)
- PR generation / bug fixes: SWE-bench Pro (not Verified)
- Feature development: FeatureBench
- Terminal workflows: Terminal-Bench
- Full application generation: Vibe Code Bench
- Internal evaluation: 100-200 tasks from recent PRs and bug fixes in your own
  codebase (most predictive for deployment)

---

## 4. Novel Evaluation Techniques

### Multi-Step Reasoning Evaluation

**Process Reward Models (PRMs).** Rather than scoring only final answers, PRMs
evaluate each reasoning step independently. HAL's reliability framework implicitly
uses this through sequence consistency analysis.

**DyCodeEval.** Dynamically generates code reasoning tasks to avoid contamination.
Uses an adaptive reasoning graph (DARG) framework that escalates task complexity
based on model performance. Key finding: smaller models struggle with tool
orchestration (41.2% success for Qwen2.5-3B vs 94.2% for GPT-5).

**CoRe (Code Reasoning benchmark).** Tests LLMs' code reasoning capabilities
through multi-step tasks that require understanding control flow, data
dependencies, and API semantics.

### Tool Use Evaluation

**Tool Use Accuracy (TUA).** Verifies if the model:
1. Invoked the correct API
2. Processed responses reliably
3. Respected data permissions

**Reflexive Calibration Score (RCS).** Assesses whether the model's confidence
aligned with tool interaction success. Models that are overconfident in wrong tool
calls are penalized more heavily than models that express uncertainty.

**AgencyBench MCP tasks.** 10 dedicated tasks testing Model Context Protocol
tool use -- the first benchmark to explicitly evaluate MCP competence.

### Code Quality Metrics

Emerging metrics beyond correctness:

**CodeJudgeBench** (2025). A coding-specific LLM-as-a-Judge benchmark that pairs
good/bad responses for three tasks (code generation, code repair, unit test
generation). 5,352 samples from LiveCodeBench. Execution-free: evaluates code
quality without running the code.

**Multi-file reasoning metrics.** Teams increasingly measure project-wide
reasoning: models that understand architecture diagrams, directory structures, and
related files perform significantly better on end-to-end workflows.

**Security scoring.** Endor Labs' benchmark reveals a critical blind spot: agents
that pass functional tests frequently introduce security vulnerabilities. Code
quality evaluation must include security scans (CWE coverage) alongside
correctness.

### Dynamic and Anti-Contamination Techniques

**Temporal freshness.** Use tasks from after the model's training cutoff.
SWE-bench Pro uses private repositories to prevent pre-training leakage.

**Perturbation-based evaluation.** ReliabilityBench applies epsilon-level
perturbations to task descriptions and measures performance delta. A robust agent
should handle paraphrased problems identically to original phrasing.

**K-trial consistency.** Run the same task K times and measure pass-rate
distribution. A model scoring 60% on single runs may drop to 25% on 8-run
consistency (all 8 must pass), revealing fragile solutions.

---

## 5. LLM-as-Judge for Software Engineering

### The Core Approach

LLM-as-Judge uses a language model to evaluate the output of another language
model. For software engineering, this means scoring code diffs, implementations,
and architectural decisions using a judge LLM.

### Key Benchmarks and Methods

**CodeJudgeBench.** 5,352 samples testing execution-free code evaluation. Compares
26 LLM judges on accuracy using pre-verified good/bad pairs across:
- Code generation quality
- Code repair correctness
- Unit test generation quality

**JETTS (Judge Evaluation for Test-Time Scaling).** Evaluates judge performance
across three domains (math, code, instruction following) under three task
settings:
- Response reranking: select the best from N candidates
- Step-level beam search: evaluate reasoning steps for search
- Critique-based response refinement: improve outputs iteratively

**Bias in the Loop (arXiv:2604.16790).** Auditing LLM-as-a-Judge for software
engineering biases. Identified systematic biases:
- Position bias: judges favor first or last responses
- Verbosity bias: longer responses scored higher regardless of quality
- Self-preference bias: models score their own outputs higher

### Best Practices for LLM-as-Judge in Agents

1. **Use rubric-based scoring.** Structured rubrics with 7 primary dimensions,
   25 sub-dimensions, and 130 fine-grained criteria produce more consistent
   judgments than open-ended scoring prompts.

2. **Calibrate with human ground truth.** Maintain a calibration set of
   human-judged examples and periodically verify judge alignment.

3. **Multi-judge ensembling.** Use multiple judge models and aggregate scores.
   Voting across judges reduces individual bias.

4. **Separate dimensions.** Score correctness, readability, security, and
   maintainability independently rather than asking for a single holistic score.

5. **Evidence-anchored scoring.** Require judges to cite specific evidence from
   the code for each score, reducing hallucinated evaluations.

### The Publication Explosion

26 publications on LLM-as-Judge were cataloged by August 2025 alone, far
surpassing the entire 2024 total in just eight months. The field is rapidly
maturing from ad-hoc prompting to structured evaluation frameworks.

---

## 6. Agent Reliability Beyond Accuracy

### The Reliability Gap

Current benchmarks reveal a systematic gap: agent accuracy has improved steadily,
but reliability has not kept pace. An agent scoring 60% on a single run may drop
to 25% on 8-run consistency, meaning only 25% of tasks are reliably solved across
all attempts.

### ReliabilityBench

Source: arXiv:2601.06112

Evaluates three dimensions:
1. **Consistency under repeated execution** -- k-trial pass rates
2. **Robustness to task perturbations** -- epsilon-level perturbation tolerance
3. **Fault tolerance under infrastructure failures** -- lambda-level degradation

Key finding: "what but not when" pattern. Agents achieve substantially higher
distribution consistency than sequence consistency. They pick the right actions
but in varying order.

### CLEAR Framework

Cost, Latency, Efficacy, Assurance, Reliability -- a holistic evaluation
framework designed for enterprise deployment of AI agents.

Enterprise-focused metrics missing from academic benchmarks:
- Security compliance: does the agent follow data handling policies?
- Latency SLAs: does the agent meet response time requirements?
- Cost predictability: how variable is per-task cost?
- Policy compliance: does the agent stay within authorized actions?

### Beyond Accuracy: Multi-Dimensional Framework

From arXiv:2511.14136, a framework for enterprise agentic AI evaluation:

1. **Comprehensiveness** -- does the agent address all aspects of the task?
2. **Coherence** -- is the agent's output internally consistent?
3. **Adaptability** -- can the agent handle edge cases and variations?
4. **Efficiency** -- does the agent minimize unnecessary steps and cost?
5. **Safety** -- does the agent avoid harmful or unauthorized actions?
6. **Transparency** -- can the agent explain its reasoning?
7. **Alignment** -- does the agent follow instructions faithfully?

### Emerging Metrics (2026)

**Consistency Score.** Variance in output quality across repeated runs of the
same task. Low variance = high consistency.

**Calibration Error.** Gap between agent's self-reported confidence and actual
success rate. Well-calibrated agents know what they do not know.

**Cost-Performance Pareto Frontier.** Plot accuracy vs cost across model/routing
configurations. The Pareto-optimal set represents the best tradeoffs.

**Recovery Rate.** Percentage of initially-failed tasks that the agent
successfully completes on retry with feedback. Measures learning from failure.

**Tool Use Efficiency.** Ratio of necessary tool calls to total tool calls.
Agents that make unnecessary API calls waste budget and time.

---

## 7. State of the Art Summary Table

### Benchmark Landscape (April 2026)

| Benchmark | Domain | Tasks | Top Score | Top Model | Status |
|---|---|---|---|---|---|
| SWE-bench Verified | SE: bug fixes | 500 | 87.6% | Claude Opus 4.7 | Contaminated; deprecated |
| SWE-bench Pro | SE: multi-file | ~200 | 23.3% | GPT-5 | Active, recommended |
| FeatureBench | SE: features | 200 | 11.0% | Claude Opus 4.5 | Active (ICLR 2026) |
| Terminal-Bench 2.0 | Terminal ops | 89 | 68.54% | Claude Opus 4.7 | Active |
| Vibe Code Bench | Full-app gen | 100 | 71.00% | Claude Opus 4.7 | Active |
| AgencyBench | Multi-domain | 138 | varies | varies | Active (ACL 2026) |
| CORE-Bench Hard | Sci repro | 270 | varies | varies | Active (in HAL) |
| HumanEval+ | Function gen | 164 | 95%+ | multiple | Saturated |
| MBPP+ | Basic coding | 974 | 90%+ | multiple | Saturated |
| HAL (aggregate) | 4 domains | 9 benchmarks | varies | varies | Active, ICLR 2026 |

### Model Rankings Across Benchmarks (April 2026)

| Model | SWE-bench V | SWE-bench Pro | Terminal 2.0 | Vibe Code | FeatureBench |
|---|---|---|---|---|---|
| Claude Opus 4.7 | 87.6% | ~22% | 68.54% | 71.00% | -- |
| Claude Opus 4.5 | 80.9% | ~20% | 58.43% | -- | 11.0% |
| GPT-5.5 | -- | -- | 62.92% | 69.85% | -- |
| GPT-5.3 Codex | -- | ~23% | 64.05% | -- | -- |
| GPT-5 | -- | 23.3% | -- | -- | -- |
| Gemini 3.1 Pro | -- | -- | 67.42% | -- | -- |

### Key Trends

1. **Contamination is the biggest threat.** SWE-bench Verified's collapse shows
   that static benchmarks are unreliable for frontier evaluation.

2. **Feature development is much harder than bug fixes.** The 74% -> 11% gap
   between SWE-bench and FeatureBench is the biggest signal in 2026 benchmarking.

3. **Reliability matters more than accuracy.** HAL's shift to reliability
   measurement reflects real-world deployment needs.

4. **Internal benchmarks are most predictive.** Tasks from your own codebase
   (recent PRs, bug fixes) are the best predictor of agent value.

5. **Multi-dimensional evaluation is necessary.** Cost, latency, security, and
   consistency all matter alongside correctness.

---

## 8. Roko Benchmark Suite Design

Roko already has substantial infrastructure for agent evaluation through its
gate pipeline and learning subsystems. This section outlines how to build a
comprehensive benchmark suite using what exists.

### Existing Infrastructure

**Gate pipeline** (`crates/roko-gate/src/`):

Roko's 7-rung gate pipeline is already a multi-stage evaluation framework:

| Rung | Gates | Benchmark Analog |
|---|---|---|
| 0: Compile | `CompileGate` | Format validity (does it build?) |
| 1: Lint | `ClippyGate` | Static analysis quality |
| 2: Test | `TestGate` | Functional correctness |
| 3: Symbol | `SymbolGate` | Symbol resolution / API compliance |
| 4: Generated Test | `GeneratedTestGate` + `VerifyChainGate` | Test generation quality |
| 5: Property Test | `PropertyTestGate` + `FactCheckGate` | Invariant checking |
| 6: Integration | `LlmJudgeGate` + `IntegrationGate` | LLM-as-judge + E2E |

Additional standalone gates provide:
- `BenchmarkRegressionGate` -- performance regression detection (stub, needs baseline infra)
- `FormatCheckGate` -- code formatting compliance
- `SecurityScanGate` -- security vulnerability scanning
- `DiffGate` -- diff analysis and review

Composition wrappers enable HAL-style multi-gate evaluation:
- `ParallelGate` -- run multiple gates concurrently, collect all verdicts
- `VotingGate` -- majority-vote across inner gates (configurable threshold)
- `FallbackGate` -- try gates in order, use first non-error verdict
- `ComposedGatePipeline` -- configurable composition modes (Sequential, Parallel, Voting, Fallback)

**LLM-as-Judge gate** (`crates/roko-gate/src/llm_judge_gate.rs`):

Already implements the core LLM-as-judge pattern:
- `JudgeOracle` trait: any LLM backend can serve as judge
- `JudgePayload`: structured (task_description, diff) input
- Configurable threshold, blocking/non-blocking modes
- Diff truncation with UTF-8 boundary respect
- Score clamping to [0, 1] range
- Duration tracking for every verdict path

**Benchmark harness** (`crates/roko-cli/src/bench.rs`):

A native SWE-bench-style proxy harness already exists:
- `SweBenchOptions` -- configurable dataset, batch size, agent adapter
- `SweAgentMode` -- Gold (plumbing validation), Empty (negative control),
  PredictionFile (JSONL patches), Command (arbitrary agent)
- Proxy scoring pipeline: format validation -> `git apply --check` ->
  patch application -> test execution
- Learning integration: writes episodes, efficiency events, C-factor snapshots
- Knowledge store integration: writes benchmark insights to neuro store
- Built-in smoke dataset with two tiny tasks for CI

**Benchmark demo** (`crates/roko-cli/src/bench_demo.rs`):

Comparison framework for naive vs optimized dispatch:
- Side-by-side measurement of cost, tokens, cache hit rate, latency
- Realistic simulation mode + real dispatch mode
- Cost waterfall decomposition (caching, routing, knowledge, gate early-exit)
- Session summary with aggregate CostMeter

**Learning subsystem** (`crates/roko-learn/src/`):

Comprehensive learning infrastructure for evaluation feedback:
- `episode_logger` -- append-only JSONL record of every agent turn
- `cfactor` -- composite quality factor tracking over time
- `efficiency` -- per-turn efficiency event telemetry
- `cascade_router` -- model routing with bandit-based selection
- `playbook` -- reusable patterns extracted from successful episodes
- `quality_judge` -- structured quality assessment
- `pareto` -- cost-quality Pareto frontier computation
- `anomaly` -- runaway loop, cost spike, quality degradation detection
- `calibration_policy` -- predict-publish-correct loop for calibration
- `verdict_scorer` -- gate-verdict-aware scoring and routing history
- `drift` -- performance drift detection over time
- `regression` -- regression analysis on evaluation data

### Proposed Benchmark Suite Architecture

#### Level 1: Internal Task Replay (Most Predictive)

Replay recent completed tasks against the gate pipeline and compare outcomes.
This is the "internal eval" that benchmark practitioners say is most predictive.

```
Source: .roko/episodes.jsonl + plan execution history
Runner: Gate pipeline (existing 7-rung or custom composition)
Scorer: Binary pass/fail + LlmJudgeGate score + gate aggregate
Output: .roko/bench/internal-replay.jsonl
Learning: Feed back into cascade_router and playbook store
```

Implementation path:
1. Extract recent episode task descriptions and agent outputs from episodes.jsonl
2. Re-run the gate pipeline on stored outputs using the existing `GateService`
3. Compare gate verdicts against original verdicts (regression detection)
4. Compute consistency metrics: how often do re-runs match original verdicts?

This can be built by wiring:
- `roko_learn::episode_logger::Episode` for task extraction
- `roko_gate::gate_pipeline::GatePipeline` for re-evaluation
- `roko_gate::adaptive_threshold::AdaptiveThresholds` for threshold tracking
- `roko_learn::drift::DriftDetector` for temporal regression detection

#### Level 2: SWE-bench Proxy (Already Built)

The existing `bench.rs` harness provides SWE-bench-style evaluation. Gaps to
fill:

1. **Real agent dispatch.** The `SweAgentMode::Command` path exists but needs
   integration with roko's own agent dispatcher so benchmarks use the same
   stack as production.

2. **Larger dataset.** The built-in smoke set has 2 tasks. Create a 50-task
   internal dataset from roko's own codebase (recent PRs, known bug fixes).

3. **Multi-run consistency.** Run each task K times (K=5 or K=8) and compute
   HAL-style consistency metrics. The harness currently runs each task once.

4. **Cost tracking.** The `BenchResult` struct has `cost_usd` but `run_task_real`
   leaves it at 0.0. Wire the cost tracking from `roko_learn::costs_db`.

#### Level 3: Multi-Dimensional Evaluation (New)

Add reliability and quality dimensions to the existing correctness-only scoring.
Map to roko's existing infrastructure:

| Dimension | Implementation | Roko Component |
|---|---|---|
| Correctness | Gate pipeline pass/fail | `GatePipeline` |
| Quality | LLM judge score | `LlmJudgeGate` |
| Security | Security scan gate | `SecurityScanGate` |
| Cost | API cost tracking | `roko_learn::costs_db` |
| Latency | Wall-clock timing | `roko_learn::latency` |
| Consistency | K-trial variance | New: multi-run adapter on `bench.rs` |
| Regression | Baseline comparison | `BenchmarkRegressionGate` (needs baseline) |
| Format | Code style compliance | `FormatCheckGate` |

#### Level 4: Comparative Routing Benchmark (Unique to Roko)

Roko's CascadeRouter already supports A/B testing of model routing strategies.
Build a benchmark that measures the Pareto frontier of cost vs quality across
routing configurations:

```
For each benchmark task:
  For each routing strategy (opus-only, haiku-only, cascade, custom):
    Run task N times
    Record: cost, quality, pass/fail, latency, consistency
  Plot Pareto frontier
  Identify optimal routing configuration per task difficulty band
```

This directly uses:
- `roko_learn::cascade_router::CascadeRouter` for routing
- `roko_learn::pareto` for frontier computation
- `roko_learn::model_experiment::ModelExperiment` for A/B tracking
- `roko_learn::prompt_experiment::ExperimentStore` for experiment management
- `roko_cli::bench_demo::BenchResult` for result capture

### Implementation Plan

**Phase 1: Wire existing infrastructure (1-2 days)**

1. Fill in `BenchmarkRegressionGate` baseline infrastructure. Currently a stub
   that always passes. Needs: baseline capture, storage, comparison logic.
   File: `crates/roko-gate/src/benchmark_gate.rs`

2. Wire cost tracking into `run_task_real`. The `BenchResult.cost_usd` field
   exists but is always 0.0. Connect to `roko_learn::costs_db`.
   File: `crates/roko-cli/src/bench.rs` (line 610)

3. Add multi-run mode to `SweBenchOptions`. Add `trials: usize` field and
   compute consistency metrics across K runs of each instance.
   File: `crates/roko-cli/src/bench.rs`

**Phase 2: Internal task replay (2-3 days)**

1. Build episode-to-benchmark converter. Read `.roko/episodes.jsonl`, extract
   task descriptions and diffs, create benchmark instances.

2. Implement replay runner. Re-run gate pipeline on historical outputs.
   Compare verdicts to detect gate drift.

3. Wire replay results into learning feedback. Use
   `roko_learn::runtime_feedback::LearningRuntime` to record replay outcomes.

**Phase 3: Multi-dimensional scoring (3-5 days)**

1. Build `BenchmarkSuite` struct that orchestrates all four levels.
2. Add `roko bench suite` CLI command.
3. Compute and persist multi-dimensional scores per task.
4. Generate Pareto frontier reports for routing optimization.
5. Wire results into TUI dashboard (F-key tab for benchmarks).

**Phase 4: External benchmark compatibility (5+ days)**

1. Implement HAL harness agent adapter. Expose roko's agent as a Python class
   conforming to HAL's protocol, enabling roko to participate in the public
   leaderboard.

2. Build SWE-bench Pro dataset loader. Parse the official dataset format and
   translate to roko's `SweBenchInstance`.

3. Add FeatureBench support. Feature development tasks require different
   scaffolding than bug-fix tasks. Wire into roko's plan generator.

### Comparison to HAL's Approach

| Aspect | HAL | Roko (Proposed) |
|---|---|---|
| Execution | Azure VMs, Docker | Local process, `ProcessSupervisor` |
| Cost tracking | LLM API spend | `roko_learn::costs_db` + efficiency events |
| Judge | External LLM call | `LlmJudgeGate` with `JudgeOracle` trait |
| Consistency | K-trial reruns | Same, via multi-run `SweBenchOptions` |
| Reliability | 4 dimensions, 12 metrics | Map to existing gate + learning metrics |
| Learning | Static leaderboard | Feedback into cascade router + playbooks |
| Self-improvement | None | Benchmark results tune routing + thresholds |

The key differentiator for roko's benchmark suite is the **feedback loop**.
HAL produces leaderboard scores. Roko produces leaderboard scores AND feeds
them back into the learning subsystems to improve future agent performance.
Every benchmark run makes the next run better through:
- Cascade router learning from cost/quality outcomes
- Adaptive gate thresholds adjusting to observed pass rates
- Playbook store capturing successful patterns
- Knowledge store recording benchmark insights
- C-factor tracking composite quality over time

This is the "agent that benchmarks itself and improves from the results" loop --
the core value proposition of roko's self-hosting architecture.

---

## Sources

### HAL
- [HAL: Holistic Agent Leaderboard](https://hal.cs.princeton.edu/)
- [HAL Harness (GitHub)](https://github.com/princeton-pli/hal-harness)
- [HAL Paper (arXiv)](https://arxiv.org/abs/2510.11977)
- [HAL Reliability Dashboard](https://hal.cs.princeton.edu/reliability/)
- [HAL Reliability Methodology](https://hal.cs.princeton.edu/reliability/methodology/)

### Benchmarks
- [SWE-bench Pro Leaderboard (Scale)](https://labs.scale.com/leaderboard/swe_bench_pro_public)
- [FeatureBench (arXiv)](https://arxiv.org/abs/2602.10975)
- [FeatureBench (GitHub)](https://github.com/LiberCoders/FeatureBench)
- [AgencyBench (arXiv)](https://arxiv.org/abs/2601.11044)
- [AgencyBench (GitHub)](https://github.com/GAIR-NLP/AgencyBench)
- [CORE-Bench (GitHub)](https://github.com/siegelz/core-bench)
- [Terminal-Bench](https://www.tbench.ai/)
- [Terminal-Bench 2.0 (Vals)](https://www.vals.ai/benchmarks/terminal-bench-2)
- [Vibe Code Bench (Vals)](https://www.vals.ai/benchmarks/vibe-code)
- [Vibe Code Bench Paper (arXiv)](https://arxiv.org/pdf/2603.04601)

### SWE-bench Contamination
- [Why SWE-bench Verified No Longer Measures Frontier Capabilities (OpenAI)](https://openai.com/index/why-we-no-longer-evaluate-swe-bench-verified/)
- [SWE-bench Verified's Fall (WebProNews)](https://www.webpronews.com/swe-bench-verifieds-sudden-fall-how-openai-exposed-flaws-in-ai-codings-top-metric/)
- [SWE-bench Contamination Debate (CodeSOTA)](https://www.codesota.com/news/swe-bench-contamination-debate)
- [SWE-bench Pro: Why 46% Beats 81% (MorphLLM)](https://www.morphllm.com/swe-bench-pro)

### LLM-as-Judge
- [Bias in the Loop: Auditing LLM-as-Judge for SE (arXiv)](https://arxiv.org/html/2604.16790v1)
- [LLM-as-a-Judge Complete Guide (Evidently AI)](https://www.evidentlyai.com/llm-guide/llm-as-a-judge)
- [LLM-as-a-Judge 2026 Guide (Label Your Data)](https://labelyourdata.com/articles/llm-as-a-judge)
- [CodeJudgeBench (arXiv)](https://arxiv.org/pdf/2510.24367)
- [Survey on LLM-as-a-Judge (arXiv)](https://arxiv.org/html/2411.15594v6)

### Reliability
- [Towards a Science of AI Agent Reliability (arXiv)](https://arxiv.org/html/2602.16666v1)
- [ReliabilityBench (arXiv)](https://arxiv.org/pdf/2601.06112)
- [Beyond Accuracy: Multi-Dimensional Framework (arXiv)](https://arxiv.org/abs/2511.14136)
- [8 AI Agent Metrics Beyond Accuracy (Galileo)](https://galileo.ai/blog/ai-agent-reliability-metrics)
- [Agent Evaluation Framework 2026 (Galileo)](https://galileo.ai/blog/agent-evaluation-framework-metrics-rubrics-benchmarks)

### General
- [AI Benchmarks 2026 Guide (Kili Technology)](https://kili-technology.com/blog/ai-benchmarks-guide-the-top-evaluations-in-2026-and-why-theyre-not-enough)
- [Top 7 Benchmarks for Agentic Reasoning (MarkTechPost)](https://www.marktechpost.com/2026/04/26/top-7-benchmarks-that-actually-matter-for-agentic-reasoning-in-large-language-models/)
- [AI Agent Benchmark Compendium (GitHub)](https://github.com/philschmid/ai-agent-benchmark-compendium)
- [How We Broke Top AI Agent Benchmarks (Berkeley RDI)](https://rdi.berkeley.edu/blog/trustworthy-benchmarks-cont/)
- [Agentic Code Security Benchmark (Endor Labs)](https://www.prnewswire.com/news-releases/endor-labs-launches-agentic-code-security-benchmark-finds-top-performing-ai-coding-agents-pass-tests-but-still--fail-security-302742611.html)

---

## Roko Source Files Referenced

| File | What It Contains |
|---|---|
| `crates/roko-gate/src/lib.rs` | 7-rung gate pipeline exports, 11+ gate types, composition wrappers |
| `crates/roko-gate/src/gate_pipeline.rs` | GatePipeline, ComposedGatePipeline, GateComposition enum |
| `crates/roko-gate/src/llm_judge_gate.rs` | LlmJudgeGate, JudgeOracle trait, JudgePayload |
| `crates/roko-gate/src/benchmark_gate.rs` | BenchmarkRegressionGate (stub -- needs baseline infra) |
| `crates/roko-gate/src/adaptive_threshold.rs` | AdaptiveThresholds, EMA per rung |
| `crates/roko-gate/src/composition.rs` | ParallelGate, VotingGate, FallbackGate |
| `crates/roko-gate/src/security_scan_gate.rs` | Security vulnerability scanning gate |
| `crates/roko-gate/src/format_check_gate.rs` | Code formatting compliance gate |
| `crates/roko-gate/src/process_reward.rs` | ProcessRewardModel for step-level evaluation |
| `crates/roko-cli/src/bench.rs` | SWE-bench proxy harness with learning integration |
| `crates/roko-cli/src/bench_demo.rs` | Naive vs optimized benchmark comparison |
| `crates/roko-learn/src/lib.rs` | 60+ learning modules for evaluation feedback |
| `crates/roko-learn/src/episode_logger.rs` | Episode JSONL record of agent turns |
| `crates/roko-learn/src/cfactor.rs` | Composite quality factor tracking |
| `crates/roko-learn/src/efficiency.rs` | Per-turn efficiency event telemetry |
| `crates/roko-learn/src/cascade_router.rs` | Bandit-based model routing |
| `crates/roko-learn/src/pareto.rs` | Cost-quality Pareto frontier computation |
| `crates/roko-learn/src/quality_judge.rs` | Structured quality assessment |
| `crates/roko-learn/src/drift.rs` | Performance drift detection |
| `crates/roko-learn/src/calibration_policy.rs` | Predict-publish-correct calibration |
| `crates/roko-learn/src/anomaly.rs` | Runaway loop and degradation detection |
| `crates/roko-learn/src/playbook.rs` | Reusable patterns from successful episodes |
| `crates/roko-learn/src/runtime_feedback.rs` | LearningRuntime, CompletedRunInput |
| `crates/roko-learn/src/verdict_scorer.rs` | Gate-verdict-aware scoring and history |
| `crates/roko-serve/src/bench.rs` | BenchStrategy enum for dispatch modes |
