# Long-Horizon Planning, Hierarchical Reasoning, Causal Discovery, and Self-Improvement

This document synthesizes the research landscape across long-horizon agent
planning, hierarchical reasoning architectures, causal structure learning, and
self-improvement loops. It is written as a self-contained reference for someone
with no prior exposure to the project or its research program. Every claim is
grounded in a specific paper with arXiv identifier, conference venue, or
benchmark dataset.

---

## 1. METR Time Horizon Benchmarks: Where the Frontier Actually Is

METR (Model Evaluation & Threat Research) maintains the most credible
third-party benchmark for measuring how long an AI agent can sustain coherent,
goal-directed work on a novel task. Their metric is the "50% time horizon" --
the maximum task duration at which a model achieves at least 50% success rate.
This is the single best proxy for whether an agent can do real engineering work
rather than answer quiz questions.

**METR TH1.1** (January 29, 2026) established the quantitative picture. The
post-2023 doubling time for agent time horizons is 131 days. Restricting to
the 2024-2025 window alone, the doubling time drops to approximately 89 days
(roughly 3 months). This is faster than Moore's Law at its peak and shows no
sign of deceleration.

The frontier as of January 2026 stands at:

| Model | 50% Time Horizon |
|---|---|
| Claude Opus 4.5 | ~2h 17min |
| o3 (OpenAI) | ~110 min |
| GPT-5 (high compute) | ~137 min |

These numbers mean today's best models can reliably complete tasks that take a
skilled human about two hours. Anything requiring a sustained half-day of
focused work remains beyond reliable reach.

**Cross-domain variation is extreme.** Coding and mathematics tasks exhibit
2-to-6-month doubling times -- the fastest improvement rates observed. But
agentic GUI interaction (measured by OSWorld, a benchmark requiring agents to
operate desktop applications through screenshots and mouse/keyboard actions)
lags by approximately 50x. An agent that can write a 2-hour coding task
reliably cannot reliably operate a web browser for 3 minutes. This gap is not a
minor calibration issue; it reflects fundamentally different capability
requirements between symbolic manipulation and perceptual-motor control.

**The strategic target** for an agent infrastructure stack is to clear the
8-hour mark at 50% reliability within 6 months. That requires two doublings
beyond the stock frontier -- meaning the stack must compound improvements from
scaffolding, memory, and routing on top of raw model capability gains. The
three compounding levers identified are: cross-model procedural memory transfer
(Memp), quality-diversity search for prompt strategies (CycleQD), and
information-gain-driven exploration (EFE). Each is discussed in later sections.

---

## 2. Hierarchical Reasoning Models: Small Models That Beat Giants

Two recent architectures demonstrate that hierarchical decomposition can
substitute for raw parameter count, achieving frontier-competitive performance
at 3-4 orders of magnitude fewer parameters.

### HRM (Hierarchical Reasoning Model)

Wang et al. (arXiv:2506.21734, Sapient Intelligence, June 2025) introduce a
27-million-parameter model trained on just 1,000 examples with no pretraining.
It outperforms multi-billion-parameter LLMs on three benchmarks: ARC-AGI-1
(40.3% accuracy), Sudoku-Extreme, and Maze-Hard. The architecture uses two
modules operating at different timescales: an H-module (high-level, slow,
abstract pattern extraction) and an L-module (low-level, fast, concrete
manipulation). The system converges through iterative communication between
these two modules, with the H-module providing structural constraints and the
L-module proposing concrete solutions.

The critical insight is two-timescale convergence: the H-module and L-module
operate as a fixed-point iteration, alternating until their outputs are
mutually consistent. This mirrors the gamma/theta timescale separation found
in biological neural circuits and in hierarchical reinforcement learning.

**Caveat.** Mechanistic analysis (arXiv:2601.10679) reveals that the
fixed-point property breaks on certain trivial cases -- problems that are
simple enough that the iterative convergence procedure overshoots. This is not
a fatal flaw but it means the architecture cannot be used as a universal
drop-in; it requires task-complexity routing to avoid regression on easy
problems.

### TRM (Transformer Reasoning Model)

The TRM (arXiv:2510.04871) achieves even stronger results with even fewer
resources: 7 million parameters, 2 transformer layers, 44.6% on ARC-AGI-1,
and 7.8% on ARC-AGI-2. Its core mechanism is a recursive Verify/Score
scratchpad: the model generates a candidate solution, verifies it against
constraints, scores the result, and recurses. This is essentially
compile-time test-driven development applied to neural reasoning.

The TRM is a drop-in module: it requires no architectural changes to the host
system, only a scratchpad buffer and a recursion budget. Its relevance is as a
verification oracle that can be embedded inside larger agent loops -- a Cell
(in the project's terminology) that provides structured reasoning without
requiring an expensive LLM call.

---

## 3. Active Inference and the Free Energy Principle

Active inference, rooted in Karl Friston's Free Energy Principle (2006),
provides a unified mathematical framework for perception, action, and learning.
The core idea: an agent minimizes surprise (equivalently, maximizes model
evidence) by either updating its beliefs (perception) or acting on the world
to make observations match predictions (action). Two recent papers make this
framework practical for engineered systems.

### AXIOM

Heins et al. (VERSES Research, arXiv:2505.24784, June 2025) present AXIOM, a
production-grade active inference implementation. The system uses four mixture
model types: static (sMM), input-dependent (iMM), transition-dependent (tMM),
and reward-dependent (rMM). Model parameters update via variational Bayes
online updating -- no gradient descent, no replay buffers, no batch training.
When a mixture component becomes redundant, Bayesian Model Reduction (BMR)
prunes it, keeping the model compact without manual intervention.

AXIOM matters because it proves active inference can run without the machinery
of deep learning. No optimizer state, no learning rate schedules, no GPU memory
management for replay buffers. The entire learning process is a sequence of
Bayesian updates that can run on a single CPU core.

### EFE as Variational Inference

De Vries et al. (arXiv:2504.14898, April 2025) solve a long-standing
theoretical gap: they derive Expected Free Energy (EFE) at each of three
timescales as a free-energy lower bound with a specific complexity penalty
coefficient (beta):

- **Gamma timescale** (reactive, 1-5 seconds): Low beta. The agent favors
  exploiting known good actions. Cheap reflexes.
- **Theta timescale** (plan refinement, 5-60 seconds): Medium beta. The agent
  balances exploration and exploitation. Working memory updates.
- **Delta timescale** (structural evolution, 120+ seconds): High beta. The
  agent favors information gain over immediate reward. Consolidation and
  long-term learning.

The breakthrough is unification: all three timescales derive from a single
variational objective with different beta parameters, not from three separate
hand-designed objectives. This means an agent runtime can implement one
optimization procedure and produce qualitatively different behaviors at
different timescales by adjusting a single scalar.

**No existing public stack has an end-to-end variational story across all three
timescales.** This is an open opportunity: the first system to implement
three-timescale EFE with proper beta scheduling would have a principled
replacement for the ad-hoc heuristics that currently govern when agents should
think fast versus slow.

---

## 4. Causal Discovery from Agent Episodes

Agents generate rich execution traces -- sequences of actions, observations,
tool calls, and outcomes. Extracting causal structure from these traces
(rather than mere correlations) enables agents to answer counterfactual
questions: "What would have happened if I had used a different tool?" This is
the foundation for genuine self-improvement rather than pattern matching.

### Rhino and Microsoft Causica

Gong et al. (ICLR 2023, with production deployment in Microsoft Causica
2024-2025) introduce Rhino, a causal discovery method that simultaneously
handles three challenges that defeat simpler methods: non-linear relationships,
instantaneous effects (where cause and effect occur in the same timestep), and
history-dependent noise (where the noise distribution changes based on past
states). Rhino consistently outperforms PCMCI+, DYNOTEARS, and VARLiNGAM on
both synthetic and real-world datasets.

For an agent system, Rhino is the right tool for dream-phase causal structure
learning: during offline consolidation (the "sleep" phase), the system
processes execution episodes and extracts causal graphs that explain which
actions actually caused which outcomes. These causal graphs become durable
knowledge that improves future planning.

### DAT-Graph: Scaling Past the 50-Variable Wall

Most causal discovery methods collapse when the number of variables exceeds
approximately 50. Amin and Wilson (ICML 2024, arXiv:2406.09177) break this
barrier with DAT-Graph, which scales to 1,000 variables by using a
differentiable acyclicity constraint that avoids the combinatorial explosion
of traditional constraint-based methods. For agent systems with hundreds of
observable state dimensions (tool availability, memory retrieval scores, model
routing decisions, gate verdicts, budget levels), this scalability is essential.

### Causal Abstraction: Bridging Components and Variables

Geiger et al. (JMLR 2025, arXiv:2301.04709) formalize how to map between
different levels of causal description. Their abstraction maps (tau) are
formally lenses (in the category-theoretic sense): they project from a
fine-grained causal model to a coarse-grained one, and interchange
interventions on the coarse model are backward passes through the lens to the
fine model. This provides a rigorous bridge between the mixture components
inside an agent system (individual scoring functions, routing weights, memory
retrieval parameters) and named causal variables in a human-readable causal
graph (e.g., "task difficulty," "agent confidence," "knowledge relevance").

### The Negative Result: LLMs Cannot Do Causal Reasoning

CausalProbe-2024 (arXiv:2506.21215) demonstrates that LLMs collapse on the
Corr2Cause benchmark under minimal perturbation. Simply renaming variables
(e.g., changing "smoking" to "glurbing") causes all tested LLMs to fail at
distinguishing correlation from causation. This is not a calibration issue; it
is evidence that LLMs perform pattern matching against training-set phrasings,
not genuine causal reasoning.

The practical implication is stark: causal discovery cannot be delegated to
LLMs via prompting. It requires dedicated algorithms (Rhino, DAT-Graph) that
operate on structured data, not natural language. LLMs can help formulate
hypotheses, but the causal testing must use proper statistical methods.

### Executable Counterfactuals

Zevcevic et al. (arXiv:2510.01539, ICLR 2026) propose translating
counterfactual queries into executable code and re-running them. Rather than
reasoning abstractly about "what would have happened if X," the system
literally replays the execution trace with the counterfactual intervention
applied and observes the result.

This insight directly maps to event-sourced replay architectures. If an agent
system stores execution traces as an append-only event log (which the project
does via `.roko/episodes.jsonl`), then Pearl's Level 3 counterfactual reasoning
collapses to Level 2 interventional reasoning -- because abduction (the hard
part of counterfactual reasoning) is replaced by direct observation of the
stored trace. The system does not need to infer what the world state was; it
recorded it.

---

## 5. Self-Improvement That Does Not Collapse

The central challenge of self-improvement is saturation: a system improves
rapidly for a few iterations, then plateaus or collapses. Recent work
identifies both the ceiling and strategies for pushing through it.

### rStar-Math

Microsoft's rStar-Math (arXiv:2501.04519, ICML 2025 Oral) demonstrates the
most dramatic self-improvement result in recent literature. Starting from
Qwen2.5-Math-7B at 58.8% on the MATH benchmark, four rounds of
self-evolution (generate solutions, verify with process reward model, filter,
retrain) reach 90.0% -- a 31.2 percentage point improvement that surpasses
OpenAI's o1-preview. The key mechanism is a process reward model (PRM) that
provides step-level verification, not just outcome-level rewards.

### Absolute Zero Reasoner

Zhao et al. (arXiv:2505.03335, NeurIPS 2025 Spotlight) introduce a
proposer/solver architecture where one model generates problems and another
solves them, with a code executor providing ground-truth reward (not LLM
judgment). The system improves without any human-curated data by using
execution results as the sole training signal. This eliminates the
reward-hacking failure mode where a model learns to exploit weaknesses in an
LLM-based judge rather than actually solving problems better.

### ThinkPRM

Khalifa et al. (arXiv:2504.16828) show that approximately 8,000 synthetic
chain-of-thought traces, used to train a process reward model, beat
discriminative PRMs trained on the full PRM800K dataset by +8% on GPQA-Diamond
and +4.5% on LiveCodeBench. The implication: quality of verification data
matters far more than quantity. A small number of carefully constructed
reasoning traces provides stronger training signal than a large corpus of
human-annotated step labels.

### The Universal Saturation Wall

Across all self-improvement work, a consistent pattern emerges: **gains
saturate after 3-4 rounds.** rStar-Math's four rounds, Absolute Zero's
convergence behavior, and older results (STaR, ReST) all hit the same wall.
The mechanism is clear: each round's training data is generated by the
previous round's model, and distributional shift compounds -- by round 4-5,
the model is training on data so far from the original distribution that it
cannot extract meaningful signal.

The practical strategy is to run self-improvement in batches with explicit
Bayesian Model Reduction (BMR) retraction between batches. BMR prunes mixture
components that have become redundant or degenerate, resetting the model's
effective complexity before the next batch of self-improvement. This maps to
the L4 structural adaptation loop: human-approved structural changes between
self-improvement cycles, not continuous unsupervised evolution.

### Strong-to-Weak Transfer: Memp

Memp (arXiv:2508.06433) demonstrates that procedural memory -- the implicit
knowledge of how to decompose and sequence sub-tasks -- transfers from stronger
models to weaker ones with substantial gains. A small model that receives
Memp-style procedural scaffolding from a larger model outperforms the small
model with conventional fine-tuning. This is a compounding lever for time
horizon extension: even as frontier models improve, their procedural
knowledge can be distilled to cheaper models for routine sub-tasks.

---

## 6. MCTS Over Graph Rewrites: Searching for Better Agent Architectures

Monte Carlo Tree Search (MCTS) applied not to game positions but to agent
workflow graphs represents a qualitative shift: instead of a human designing
the agent's architecture, the system searches over architectures using
execution feedback as ground truth.

### AFlow

AFlow (ICLR 2025 Oral) applies MCTS to code-represented workflows. Each node
in the search tree is a complete agent workflow (a directed graph of LLM calls,
tool invocations, and control flow). Each edge is a graph rewrite operation
(add a node, remove a node, change a connection, modify a prompt). The search
uses execution results on held-out tasks as the reward signal. AFlow achieves
+5.7% average improvement over prior state-of-the-art across six benchmarks,
demonstrating that architecture search can find non-obvious workflow designs
that human engineers miss.

### A-squared-Flow

A-squared-Flow (arXiv:2511.20693) extends AFlow by self-adapting the operator
alphabet -- the set of graph rewrite operations available during search. Instead
of using a fixed set of mutations, the system discovers new mutation types that
are productive for the current task distribution. This is meta-search: searching
over the space of search operators.

### MASTER

MASTER (arXiv:2501.14304) addresses a critical failure mode in MCTS over agent
workflows: reward hacking that grows with search depth. As the tree deepens,
the probability of finding a workflow that exploits a weakness in the reward
signal (rather than genuinely solving the task) increases. MASTER introduces
confidence-weighted UCT (Upper Confidence Bound for Trees), which discounts
reward estimates that have high variance relative to their depth in the tree.

**The key design principle across all three systems**: use execution feedback
as ground-truth reward, not LLM judges. When an agent workflow produces code,
run the code and check the tests. When it produces a plan, execute the plan
and measure the outcomes. LLM-as-judge introduces exactly the kind of
distributional bias that causes self-improvement to saturate.

---

## 7. Hierarchical Reinforcement Learning Revival

After a decade in the wilderness (2013-2023), hierarchical reinforcement
learning (HRL) is experiencing a revival driven by the practical needs of
long-horizon agent tasks.

### The Options Framework Mapping

The classical options framework (Sutton, Precup, Singh, 1999) decomposes
policies into temporally extended actions ("options"), each with its own
initiation set, internal policy, and termination condition. This maps
directly to agent infrastructure primitives:

- **Options** correspond to Block-level primitives: atomic capabilities like
  "search codebase," "run tests," "edit file," "query knowledge store."
- **Option-policies** correspond to Compose-protocol invocations: sequences of
  Block primitives assembled into coherent sub-workflows like "implement
  function," "debug test failure," "review pull request."
- **High-level policy** corresponds to the EFE-router: the decision of which
  option-policy to invoke next, balancing information gain (epistemic value),
  goal progress (pragmatic value), and cost.

### Key Recent Systems

**HiPER** (arXiv:2602.16165) introduces hierarchical planning with explicit
representation of sub-goal dependencies, enabling the high-level policy to
reason about which sub-goals must complete before others can start -- a natural
fit for DAG-structured task plans.

**HiMAC** (arXiv:2603.00977) extends hierarchical RL to multi-agent settings,
where multiple agents share a high-level policy but execute independent
low-level options. This maps to the multi-slot concurrency model where a single
agent runs N independent task slots under shared budget and memory.

**STEP-HRL** (arXiv:2604.05808) adds explicit skill transfer between hierarchy
levels, allowing low-level option policies trained in one context to be reused
in new high-level plans without retraining.

### ArCHer: The Efficiency Benchmark

ArCHer (Berkeley, arXiv:2402.19446) provides the most compelling efficiency
result: approximately 100x more sample-efficient than flat RLHF on multi-turn
dialogue tasks. The mechanism is a hierarchical actor-critic where the critic
operates at the episode level (evaluating entire trajectories) while the actor
operates at the turn level (selecting individual actions). This separation
prevents the credit assignment problem that makes flat RL over long horizons
intractable.

For an agent infrastructure stack, ArCHer's architecture suggests that learning
from execution episodes should happen at two levels: episode-level evaluation
("was this plan successful?") drives high-level routing decisions, while
turn-level evaluation ("was this tool call productive?") drives low-level
prompt and tool selection.

---

## 8. Multi-Modal Perception for Planning

Long-horizon planning requires perception -- the ability to observe and
understand the current state of the environment. For software agents, this
means understanding code, terminal output, browser state, and desktop GUIs.

### V-JEPA 2

Meta's V-JEPA 2 (arXiv:2506.09985, June 2025) is a video understanding model
trained on over 1 million hours of video via a joint-embedding predictive
architecture (no pixel-level reconstruction, no contrastive pairs). It achieves
77.3% on Something-Something-v2 (a temporal reasoning benchmark), and
demonstrates zero-shot transfer to robotic manipulation with just 62 hours of
robot data -- approximately 30x more data-efficient than NVIDIA's Cosmos model.

V-JEPA 2's relevance to software agents is indirect but important: its
architecture proves that predictive world models can be learned from raw
observation without reconstruction losses. The same architectural principle
could power a "desktop world model" that predicts the next state of a GUI
given an action.

### UI-TARS-2

ByteDance's UI-TARS-2 (arXiv:2509.02544) achieves 47.5% on OSWorld,
surpassing OpenAI's Computer Use Agent (CUA). Critically, it releases open
weights, making it integrable into custom agent stacks without API
dependencies. At 47.5% on OSWorld, UI-TARS-2 still fails on more than half of
desktop automation tasks, but it represents the open-source frontier.

### Claude Opus 4.7

Claude Opus 4.7 achieves 78% on OSWorld with a 3x vision resolution upgrade.
This is by far the highest OSWorld score, but at $5/$25 per million
input/output tokens, it is economically impractical for continuous GUI
monitoring. The cost structure implies that GUI perception should be
hierarchical: a cheap model monitors for state changes (gamma timescale), and
the expensive model is invoked only when the cheap monitor detects something
requiring detailed analysis (theta timescale).

### HyperDUM

HyperDUM (CVPR 2025, arXiv:2503.20011) brings Hyperdimensional Computing (HDC)
to uncertainty quantification in perception. It achieves 2.36x fewer FLOPs and
up to 38.30x fewer parameters than Bayesian baselines for equivalent
uncertainty estimation quality. HDC encodes data as high-dimensional binary
vectors and performs reasoning via arithmetic operations on these vectors
(binding, bundling, permutation).

HyperDUM's relevance is operational: it enables cheap, always-on confidence
estimation for perceptual inputs. An agent can know not just what it sees but
how confident it should be in that observation, without the computational cost
of full Bayesian inference. This feeds directly into the EFE calculation: low
perceptual confidence increases epistemic value, biasing the agent toward
information-gathering actions.

---

## 9. Biological Inspiration: What to Adopt and What to Reject

### Allostasis over Homeostasis

Classical control systems maintain homeostasis: detect deviation from setpoint,
correct. Khan and Lowe (arXiv:2406.08471) and Harrison, Friston, and
Buckwalter (Frontiers in Behavioral Neuroscience, 2025) argue that biological
agents use allostasis instead: predictive setpoint shifting. Rather than
waiting for a threshold to be crossed and then reacting, an allostatic agent
predicts that a threshold WILL be crossed and adjusts its setpoint
preemptively.

For agent systems, this means trigger protocols should fire on predicted drift,
not on threshold crossing. An agent that predicts its budget will be exhausted
in 10 minutes should begin conservation behavior now, not when the budget
actually hits the conservation threshold. This is a subtle but consequential
design choice: it converts reactive control into predictive control, which is
strictly more capable in non-stationary environments.

### HippoRAG 2

Gutierrez et al. (ICML 2025, arXiv:2502.14802) present HippoRAG 2, a
retrieval system inspired by hippocampal indexing. It integrates LLM-extracted
knowledge graph triples with Personalized PageRank for retrieval, achieving +7
F1 over NV-Embed-v2 (the previous SOTA dense retrieval model). The
architecture mirrors the hippocampal theory of memory: new experiences are
encoded as graph edges, and retrieval spreads activation through the graph
(analogous to hippocampal pattern completion).

HippoRAG 2 is directly relevant to agent knowledge systems. Rather than
treating memory as a flat vector store with cosine similarity retrieval, it
models memory as a knowledge graph with spreading activation. This naturally
supports multi-hop reasoning: "I need to find a technique for X. I know
technique A is related to X, and technique B is a variant of A, so B might
also work for X." Dense retrieval cannot perform this chain; graph-based
retrieval can.

### Theory of Mind

The Decrypto benchmark (arXiv:2506.20664) emerges as the new gold standard for
evaluating Theory of Mind (ToM) in AI systems. Decrypto requires an agent to
give clues that its teammate will understand but its opponents will not --
genuine perspective-taking, not just behavior prediction. The key finding: RL
tuning HURTS ToM performance. Models fine-tuned with reinforcement learning
become worse at perspective-taking, apparently because RL optimizes for
reward-maximizing behavior rather than accurate modeling of other agents'
beliefs.

This is a cautionary result for multi-agent systems: optimizing individual
agent performance via RL can degrade the collective intelligence of the group
by impairing agents' ability to model each other.

### What to Reject

Several biologically-inspired research directions should be explicitly
deprioritized:

- **Mirror neurons as the basis for imitation learning.** The neuroscience is
  contested, and the engineering implementations are uniformly less effective
  than behavioral cloning from demonstrations.
- **"Artificial consciousness" framings.** These add philosophical confusion
  without engineering value. Functional states (confidence, uncertainty,
  resource pressure) are sufficient; phenomenal consciousness is neither
  necessary nor measurable.
- **FEP as Theory of Everything.** The Free Energy Principle is useful as a
  design pattern for specific subsystems (EFE routing, BMR pruning). It is not
  a universal foundation. Applied without discipline, it becomes unfalsifiable.
- **Classical AIS (Artificial General Intelligence Safety) algorithms** such as
  AIXI and Solomonoff induction. These are computationally intractable and
  provide no practical guidance for bounded agents.
- **Hawkins cortical-column AGI.** The Thousand Brains theory produces
  architectures with no demonstrated advantage over standard transformers on
  any benchmark.

---

## 10. Gaia2: The Inverse Scaling Warning

**Gaia2** (arXiv:2509.17158, ICLR 2026) is a benchmark for long-horizon,
multi-step agent tasks that require real-time information retrieval, tool use,
and multi-modal reasoning. Its most important finding is an inverse scaling
result: **more reasoning does not always improve performance on time-sensitive
tasks.**

Models that spend more compute on chain-of-thought reasoning perform WORSE on
tasks where the environment changes during deliberation (e.g., tasks requiring
current information that becomes stale during a long reasoning chain). This is
not a minor edge case; it is a fundamental tension in agent design between
deliberation depth and action timeliness.

### Heterogeneous Teams Beat Monolithic Models

Gaia2 confirms that heterogeneous agent-to-agent (A2A) teams outperform any
single monolithic model. A team composed of a fast reactive agent (low
deliberation, high responsiveness) and a slow deliberative agent (deep
reasoning, latency-tolerant) achieves higher aggregate scores than either agent
type alone. The fast agent handles time-sensitive sub-tasks while the slow
agent handles reasoning-intensive sub-tasks.

### The Routing Imperative

The EFE-router (the system component that decides which cognitive tier to
invoke for each sub-task) must learn WHEN NOT TO THINK MORE. The default bias
in current agent systems is "more reasoning is always better," and Gaia2
proves this is wrong. The router's EFE calculation must include a timeliness
term: the expected cost of environmental state change during deliberation.
When this cost exceeds the expected benefit of deeper reasoning, the router
should select the fastest available tier.

### Current Frontier Scores

Best frontier performance on Gaia2 as of early 2026:

| Model | Pass@1 |
|---|---|
| GPT-5 (high compute) | 42% |
| Kimi-K2 (open-source SOTA) | 21% |

Both numbers are strikingly low. Gaia2 tasks are designed to require multiple
tool calls, real-time information retrieval, and multi-step reasoning over
periods of 30-60 minutes. The 42% frontier score means the best model in the
world fails on 58% of tasks in this regime. This is the clearest empirical
evidence that long-horizon agent capability remains a wide-open research
problem.

---

## Synthesis: How These Threads Interconnect

The ten areas surveyed in this document are not independent. They form a
coherent picture of what is required for agents that can sustain goal-directed
work over multi-hour horizons.

**The METR benchmark** (Section 1) defines the goal: 8-hour 50% horizon.
**Hierarchical reasoning** (Section 2) shows that small, structured models can
match or exceed large flat models, suggesting that architecture matters as much
as scale. **Active inference** (Section 3) provides the mathematical
framework for unifying fast reactive behavior and slow deliberative reasoning
under a single objective with different beta parameters. **Causal discovery**
(Section 4) enables agents to learn from their own experience via structural
understanding rather than pattern matching, while confirming that LLMs
themselves cannot perform causal reasoning. **Self-improvement** (Section 5)
works but saturates after 3-4 rounds, requiring explicit structural resets
between batches. **MCTS over graph rewrites** (Section 6) automates the search
for better agent architectures using execution feedback. **Hierarchical RL**
(Section 7) provides 100x sample efficiency improvements through the options
framework. **Multi-modal perception** (Section 8) remains the bottleneck for
GUI-based tasks but is advancing rapidly. **Biological inspiration** (Section
9) contributes allostatic control and graph-based memory while warning against
several overhyped paradigms. And **Gaia2** (Section 10) delivers the critical
negative result: more thinking is not always better, and routing must learn
when to stop deliberating.

The overarching lesson is that long-horizon agent capability is not a single
problem but a stack of interacting problems. No single breakthrough -- not
larger models, not better prompting, not more MCTS search -- will close the
gap from 2 hours to 8 hours. The compounding of multiple improvements
(hierarchical reasoning + causal learning + self-improvement with saturation
management + timeliness-aware routing + cross-model memory transfer) is what
produces multiplicative gains. The agent infrastructure stack that integrates
these techniques end-to-end, rather than applying them in isolation, will be
the one that clears the 8-hour horizon first.
