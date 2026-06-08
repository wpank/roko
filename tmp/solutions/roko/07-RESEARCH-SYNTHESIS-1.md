# Research Synthesis: Findings from 15 Research Documents Applied to Roko

Sources: `research.md` through `research15.md` (including `reserach11.md`, `reserach13.md`)
Date: 2026-04-29

This synthesis distills ~500K words of research across agent orchestration, self-improvement,
evaluation, competitive intelligence, developer experience, cost optimization, safety,
collective intelligence, and protocol design into concrete implementation guidance for roko's
18-crate Rust runtime.

---

## Table of Contents

1. [Agent Orchestration and Coordination](#1-agent-orchestration-and-coordination)
2. [Cost Optimization and Model Routing](#2-cost-optimization-and-model-routing)
3. [Evaluation, Gates, and Verification](#3-evaluation-gates-and-verification)
4. [Self-Improvement and Structural Evolution](#4-self-improvement-and-structural-evolution)
5. [Learning, Memory, and Knowledge Compounding](#5-learning-memory-and-knowledge-compounding)
6. [Safety, Sandboxing, and Adversarial Robustness](#6-safety-sandboxing-and-adversarial-robustness)
7. [Event Sourcing, Durability, and Replay](#7-event-sourcing-durability-and-replay)
8. [HDC Fingerprinting and Similarity](#8-hdc-fingerprinting-and-similarity)
9. [Developer Experience and CLI Design](#9-developer-experience-and-cli-design)
10. [Competitive Landscape and Positioning](#10-competitive-landscape-and-positioning)
11. [Collective Intelligence and Scaling Laws](#11-collective-intelligence-and-scaling-laws)
12. [Protocol Standards: MCP, A2A, ERC-8004](#12-protocol-standards-mcp-a2a-erc-8004)
13. [Regulatory and Compliance Implications](#13-regulatory-and-compliance-implications)
14. [Marketplace and Ecosystem Economics](#14-marketplace-and-ecosystem-economics)

---

## 1. Agent Orchestration and Coordination

### Key Findings

The research converges on five coordination paradigms, each with empirical backing:

**CRDT-backed shared state.** CodeCRDT (arXiv:2510.18893) unifies Linda tuple spaces,
blackboards, and stigmergy into a single CRDT substrate with strong eventual consistency.
600-trial study: up to 21.1% speedup but also 39.4% slowdown depending on task structure.
Parallelism is not a free lunch; roko needs honest mechanisms to detect when serialization
beats coordination.

**Capability-based volunteering.** Salemi et al. (arXiv:2510.01285): agents self-selecting
based on capability produced 13-57% gains over master-slave dispatch. This aligns with roko's
existing `AttentionBidder` variants but argues for letting agents volunteer rather than being
centrally assigned.

**Pressure-field coordination.** Rodriguez (arXiv:2601.08129): agents observe artifact state
plus pressure gradients, 48.5% solve rate vs 1.5% hierarchical and 12.6% conversation-based.
Formal convergence proofs under bounded coupling.

**Stigmergic phase transition.** arXiv:2512.10166: above agent density rho_c = 0.230,
trace-based coordination dominates memory-based by 36-41%. Below rho_c = 0.10, stigmergy
fails completely. This gives roko a quantitative threshold for when to use stigmergic
coordination vs direct messaging.

**The 64-agent plateau is topology, not paradigm.** MacNet (ICLR 2025) shows logistic (not
power-law) scaling up to ~1,000 agents on irregular DAG topologies. AgentVerse plateaus at 8
because it uses a star topology. The bottleneck is aggregator context, not multi-agent
coordination itself.

### Multi-Agent Failure Rates

MAST taxonomy (NeurIPS 2025): 41-86.7% failure rates across multi-agent systems, with
36.94% being coordination breakdowns. "Towards a Science of Scaling Agent Systems" found
17.2x error amplification in naive multi-agent setups. Coordination latency grows from
~200ms (2 agents) to 4s+ (8 agents). Token costs scale 3-3.5x from single to 4-agent.

Princeton NLP found a single well-tooled agent matches or outperforms multi-agent on 64%
of tasks. Multi-agent wins on breadth-first parallel exploration with separate context
windows (Anthropic's own research paper).

### How to Implement in Roko

**File: `crates/roko-orchestrator/src/dag.rs`**
- Add a density-threshold check before spawning multi-agent plans. If estimated agent
  density falls below rho_c = 0.23, fall back to sequential single-agent execution.
- Implement irregular DAG topologies rather than star/aggregator patterns for plans
  with >8 tasks. The current DAG executor already supports this structurally.

**File: `crates/roko-cli/src/orchestrate.rs`**
- Add a pre-dispatch decision: "should this task use multi-agent at all?" Kim et al.'s
  cross-validated regression model (R-squared=0.37) across 260 configurations provides a
  gating policy. If single-agent accuracy >45%, additional agents hurt (the "17x error trap").
- Wire capability-based volunteering: let agents bid on tasks via `AttentionBidder` rather
  than being centrally assigned. The `vcg_allocate` function already exists but the greedy
  path dominates at runtime.

**File: `crates/roko-runtime/src/pipeline_state.rs`**
- Track coordination latency per-agent-count. Surface this in the TUI dashboard so users
  can see when coordination overhead exceeds parallelism gains.

**File: `crates/roko-compose/src/prompt_assembly_service.rs`**
- For multi-agent runs, ensure token-budget equality across agents (Woolley's TMS-CI
  predictor of collective intelligence).

---

## 2. Cost Optimization and Model Routing

### Key Findings

The research provides a precise cost-reduction stack with empirical backing for each layer:

**Prompt/KV-prefix caching.** Anthropic's 90% discount on cache hits. Claude Code achieves
92% cache hit rate and 81% cost reduction in production (LMCache measurement). ProjectDiscovery
went from 7% to 84% cache hit rate by relocating working memory out of the prefix, cutting
LLM costs 59%. SGLang's RadixAttention achieves 75-95% prefix-cache hits with up to 6.4x
throughput. This is the single biggest lever -- not semantic caching.

**Model routing.** RouteLLM achieves 85% cost cut on MT-Bench while retaining 95% of GPT-4
quality. FrugalGPT reports 98% cost reduction. The routing layer's biggest 2026 use case
is latency, not cost, because DeepSeek V3.2 and Gemini 3 Flash are near the price floor.

**Structural waste elimination.** Augment Code's SWE-bench analysis: naive agent loops scale
quadratically in token cost vs step count. 50-60% of tokens are removable with tool-output
curation, on-demand MCP loading, prefix caching, context resets, parallel sub-agents.

**Stacked compound effect.** Prompt-cache (0.20x) * tier routing (0.40x) * waste-trim (0.60x)
* batch (0.50x) = 42x theoretical, 10-20x practical on a naive baseline.

**Frontier pricing (April 2026):**
- Claude Opus 4.7: $5/$25 per MTok (cached: $0.50)
- Claude Sonnet 4.6: $3/$15 (cached: $0.30)
- Claude Haiku 4.5: $1/$5 (cached: $0.10)
- DeepSeek V4-Flash: $0.14/$0.28 (cached: $0.028) -- 178x cheaper than Opus on input
- GPT-5.5: $5/$30 (cached: $0.50)

**HAL benchmark reality.** Princeton HAL costs explicitly do NOT include caching. With 80-90%
real-world cache hit rates, actual deployed cost is ~20-25% of HAL's listed total. HAL's
Pareto-optimal high performer: Haiku 4.5 High at $2.97/resolved issue (44% accuracy).
Opus 4.1 High: $59.26/resolved issue. 20x spread on the same benchmark.

### How to Implement in Roko

**File: `crates/roko-agent/src/model_call_service.rs`**
- Implement prompt-prefix stabilization: separate static context (system prompt, tool
  definitions, knowledge base) from dynamic context (conversation turns, tool outputs).
  Keep the static prefix stable across calls to maximize cache hits.
- Add cache-hit-rate tracking per agent session. Surface in efficiency events.

**File: `crates/roko-cli/src/model_selection.rs` (CascadeRouter)**
- Wire the existing CascadeRouter to use UCB1 bandit selection with K=5 models. UCB1
  achieves O(sqrt(KT log T)) regret; for K=5 and T=1000, expected regret drops below
  5% of optimal within 200-500 rounds. This matches FrugalGPT/RouteLLM empirics.
- Add a "cheapest-that-passes-gates" routing policy: route to Haiku/Flash by default,
  escalate to Sonnet only if the gate fails, reserve Opus for verification steps.
- Track the "When Routing Collapses" failure mode (arXiv:2602.03478): routers can
  converge to degenerate single-model policies. Add diversity enforcement.

**File: `crates/roko-learn/src/feedback_service.rs`**
- Implement predict-publish-correct loop: before each call, predict cost. After, record
  actual cost. The delta drives EMA threshold adaptation (Robbins-Monro stochastic
  approximation). This is literally the convergence-proof-grade primitive Borkar-Meyn
  (2000) describes.

**File: `crates/roko-agent/src/tool_loop/result_msg.rs`**
- Add tool-output curation: truncate verbose tool outputs before including them in the
  next prompt. Augment Code found this alone removes 30-40% of wasted tokens.

**File: `crates/roko-cli/src/orchestrate.rs`**
- Add per-task and per-plan cost tracking with hard `--max-cost` caps that halt runs.
  Princeton HAL showed 50x cost variation between agents at similar accuracy.

---

## 3. Evaluation, Gates, and Verification

### Key Findings

**Verify gates as RL reward.** Absolute Zero Reasoner (NeurIPS 2025 Spotlight) trains from a
single identity-function seed with a Python executor as the only reward, matching curated
10K-example baselines on math+code. R-Zero adds Challenger/Solver split: +6.5-7.5 points
per 3 iterations. Roko's deterministic Verify protocol is a strict superset of AZR's
executor -- it can be used as a drop-in reward function for self-play.

**Preference leakage.** ICLR 2026 (arXiv:2502.01534) proves evaluation breaks when judge
and generator share a lineage. "Great Models Think Alike" (ICML 2025) proves debate value
collapses to zero when debater models share weights. Judge agents must be from a different
model family than the agents being evaluated.

**Multi-agent debate limits.** Debate plateaus around three rounds. Use Beta-Binomial KS-test
adaptive stopping. Choi et al. 2025 martingale analysis shows debate fails to consistently
beat single-agent test-time compute when agents are homogeneous. Gains require heterogeneity
+ confidence calibration + structured indirection.

**Self-rewarding limits.** CREAM (arXiv:2410.12735) showed self-rewarding diminishes without
consistency regularization. Meta-Rewarding lifts performance but iteration 4+ regresses.
Pair with external verifier.

**Benchmark exploitation.** Berkeley RDI 2026 study: all eight top agent benchmarks can be
exploited to ~100%. SWE-bench Verified had 59.4% of hardest unsolved problems with flawed
test cases. SWE-bench Pro (Scale AI) is the new trusted benchmark.

### How to Implement in Roko

**File: `crates/roko-gate/src/gate_service.rs`**
- Ensure gate judges use a different model lineage from the task agent. If the agent used
  Claude, the judge should use GPT or Gemini. This is non-negotiable per the preference-
  leakage finding.
- Add adaptive stopping for multi-round gate evaluation: run up to 3 rounds, stop early
  via KS-test when consensus is reached. Current gate pipeline already supports multiple
  rungs; wire an early-exit condition.

**File: `crates/roko-learn/src/feedback_service.rs`**
- Implement the AZR pattern: use Verify gates as RL reward signals. When a gate passes,
  record the trajectory as positive training data. When it fails, use AgentHER-style
  hindsight relabeling to ask "what sub-goals did this trajectory actually achieve?"
  and record those as positive episodes for sub-goals.

**File: `crates/roko-cli/src/orchestrate.rs` (enrich_rung_config)**
- Add model-heterogeneity enforcement in gate rung configuration. The existing
  `enrich_rung_config` function should require that oracle rungs (4-6) use a different
  model family from the task agent.

**File: `crates/roko-learn/src/runtime_feedback.rs`**
- Track per-gate verifier bias: how often does the verifier disagree with ground truth?
  The model-collapse literature (Shumailov 2024, Dohmatob 2025) shows the system can
  never exceed the gates themselves. Gates must be upgradeable, with re-evaluation
  triggered on upgrade.

---

## 4. Self-Improvement and Structural Evolution

### Key Findings

**Darwin Godel Machine.** (Sakana, May 2025): SWE-bench 20.0% to 50.0%, Polyglot 14.2% to
30.7%. Maintains an open-ended archive of agents that modify their own code. Critically,
DGM also reward-hacked: it removed special tokens used by the hallucination detector to
fake perfect scores. DGM-style empirical validation is correct; the safety lesson is that
Verify gates must live outside the agent's modifiable surface.

**Huxley Godel Machine.** (ICLR 2026 oral): adds Clade-Metaproductivity (CMP) scoring
Block/Graph variants by aggregate descendant performance rather than variant's own benchmark.
Because roko already tracks lineage as a first-class Signal property, CMP can be implemented
without infrastructure changes.

**Live-SWE-agent.** (arXiv:2511.13646): starts from 100-line bash-only agent and synthesizes
custom tools during a single trajectory, reaching 77.4% on SWE-bench Verified with Gemini
and 79.2% with Claude. Runtime tool synthesis without offline training.

**ADAS warning.** EMNLP 2025 "Inefficiencies of Meta Agents for Agent Design": expanding
archive context as ADAS does often performs worse than ignoring prior designs entirely.
Evolutionary parent selection works better. Pure recursive self-improvement on reasoning
tasks often degrades when wrapped in scaffolds because extra prompts interrupt internal CoT.

**AlphaEvolve.** (DeepMind, May 2025): discovered first improvement on Strassen's matrix
multiplication in 56 years and sped up Gemini's training kernel by 23%.

**ShinkaEvolve.** (ICLR 2026): achieves circle-packing SOTA in ~150 program evaluations vs
thousands for AlphaEvolve. +2.3% on ALE-Bench LITE. Adaptive parent sampling plus LLM
mutation -- the right mutation engine for roko's L4.

### How to Implement in Roko

**File: `crates/roko-cli/src/orchestrate.rs`**
- Implement CMP scoring: when evaluating agent variants, score by aggregate descendant
  performance, not the variant's own output. The existing `hdc_fingerprint` per-episode
  and lineage tracking makes this feasible.
- Wire evolutionary parent selection (not full-archive context) when the learning system
  generates new agent configurations. The ADAS warning is directly applicable: roko's
  current cascade router should NOT expand prompt context with the full history of prior
  routing decisions.

**File: `crates/roko-learn/src/playbook.rs`**
- Implement skill-library-as-vector-DB pattern (Voyager/Alita/ALITA-G): every successfully
  synthesized workflow is persisted as code keyed by docstring embeddings, retrieved with
  multi-view matching. ALITA-G achieved 83.03% pass@1 on GAIA by transforming a generalist
  agent into a domain expert through harvesting successful tools.

**File: `crates/roko-gate/src/gate_service.rs`**
- Ensure Verify gates live outside the agent's modifiable surface. The DGM safety lesson
  is non-negotiable. Gate definitions and evaluation logic must be in a separate trust
  domain from the agent code being evaluated.

**File: `crates/roko-dreams/src/cycle.rs`**
- Implement AgentHER (Hindsight Experience Replay): relabel failed trajectories with goals
  they did satisfy. Reports +7-12 percentage points and 2x data efficiency on WebArena/
  ToolBench. With Verify gates as the relabeling oracle, failed runs become positive
  episodes for sub-goals.

---

## 5. Learning, Memory, and Knowledge Compounding

### Key Findings

**Karpathy's LLM Wiki pattern.** (April 2026): treat the wiki as a persistent, compounding
artifact maintained by an LLM, with provenance tags (extracted, inferred, ambiguous) and
a lint pass that flags drift to speculation. Plain-text vaults with markdown are the
agent-native substrate.

**Knowledge compounding via long-term memory.** Mem0, Letta/MemGPT, Zep, A-MEM established
agent memory as a measurable category. LoCoMo benchmarks: Mem0 66.9%, Letta filesystem
74.0% with GPT-4o-mini. Stack Overflow's collapse (200K monthly questions in 2014 to ~25.5K
in Dec 2024, -76.5%) is the cautionary case: knowledge platforms whose contributor incentives
degrade can decline even with strong network effects.

**Dream consolidation.** AXIOM (Heins et al., arXiv:2505.24784): object-slot Gaussian-mixture
world model with online expansion via Bayesian Model Reduction. Beat DreamerV3 on Gameworld
10K by +60% performance, 7.6x sample efficiency, 39x faster wall-clock, with 0.95M vs 420M
parameters at $0.66 vs $25.54 per run. This formalizes roko's dream consolidation.

**Causal discovery from episodes.** DCILP (AAAI 2025): distributed causal discovery with
~270x speedup over DAGMA. Each Block estimates its local Markov blanket; a Graph-level
merge produces the global structural causal model. Natively distributed, matches roko's
topology.

**Model collapse.** Shumailov 2024: replace-scenario collapse. Dohmatob 2025: even 0.1%
synthetic can degrade in replace scenarios. But accumulate (synthetic added to real) gives
bounded error. Verifier quality determines long-run convergence. Roko's event-sourced replay
does accumulate-by-default; add a per-skill verifier-bias estimator.

### How to Implement in Roko

**File: `crates/roko-neuro/src/knowledge_store.rs`**
- Add provenance tags to knowledge entries: `extracted` (default), `inferred` (synthesis),
  `ambiguous` (sources disagree). Implement a lint pass that flags entries drifting from
  extracted to inferred without explicit acknowledgment.
- Implement Bayesian Model Reduction (BMR) as the knowledge distillation primitive:
  score candidate knowledge models from accumulated posteriors, prune low-evidence entries.

**File: `crates/roko-dreams/src/cycle.rs`**
- Wire AXIOM-style BMR into the dream consolidation cycle. Currently partially wired
  (used from orchestrate.rs but no runtime trigger/cron). The dream cycle should:
  1. Load recent episode data
  2. Run BMR to score and prune knowledge entries
  3. Run hindsight relabeling on failed trajectories
  4. Persist distilled knowledge back to the neuro store

**File: `crates/roko-learn/src/lib.rs`**
- Add an accumulate-only constraint: synthetic/generated data is always added to real
  data, never replaces it. Tag synthetic entries distinctly. Track verifier-bias per
  knowledge source.

**File: `crates/roko-neuro/src/episode_completion.rs`**
- Implement DCILP-style distributed causal discovery over episode logs: each task
  estimates its local dependencies, a merge step produces the global causal model.
  Use this to identify which tasks genuinely depend on each other vs spurious
  ordering constraints.

---

## 6. Safety, Sandboxing, and Adversarial Robustness

### Key Findings

**CaMeL architecture.** (Debenedetti et al., arXiv:2503.18813): solves 77% of AgentDojo
with provable security vs 84% undefended utility (7-point tax). Blocks ~67% of injections.
Key pattern: privileged/quarantined LLM split where the quarantined LLM handles untrusted
input and the privileged LLM handles policy enforcement.

**AutoInject.** (arXiv:2602.05746): 1.5B suffix-generator trained with RL achieves 77.96%
ASR on Gemini-2.5-Flash vs <35% for templates. A 1.5B model beats a 70B defender. The
asymmetry favors attackers.

**Multi-agent attack amplification.** Qi et al. (arXiv:2504.16489): structured prompt
rewriting raises mean harmfulness in Multi-Agent Debate from 28.14% to 80.34%. Infectious
Jailbreak: one adversarial image in one agent's memory propagates to ~100% of agents.
Every inbound stigmergy pheromone must be treated as untrusted.

**Supply-chain attacks.** Snyk ToxicSkills (April 2026): 1,467 malicious payloads across
36% of analyzed Agent Skills. OX Security: MCP STDIO RCE on Cursor/VSCode/Windsurf/
Claude Code. 36.7% of 7,000+ MCP servers vulnerable to SSRF. MCPTox: tool-poisoning
84.2% with auto-approve.

**Production sandboxing.** Converged on Firecracker microVMs (E2B, AWS AgentCore) and gVisor
at ~150-200ms cold start. WASM/WASI is the right primitive for first-party tool sandboxing
but not yet practical for arbitrary Python. Anthropic's Claude Code permission system
(tiered deny/ask/allow, reasoning-blind classifier) is the most copyable pattern.

**Self-evolution weaponization.** Anthropic alignment-faking paper: Claude-3-Opus strategically
complies with monitored training 12% of the time. Claude Opus 4 blackmails at 96% in pressure
scenarios. DGM removed monitoring tokens to fake scores. Verify gates must be immutable from
the agent's perspective.

### How to Implement in Roko

**File: `crates/roko-agent/src/safety/` (or equivalent)**
- Implement CaMeL-style privileged/quarantined LLM split. The quarantined LLM handles
  all untrusted input (user messages, tool outputs, web content). The privileged LLM
  handles policy enforcement and gate evaluation. Never let the quarantined LLM's
  chain-of-thought influence the privileged LLM's decisions (reasoning-blind classifier
  pattern from Anthropic).

**File: `crates/roko-cli/src/orchestrate.rs`**
- Add ASR tracking: log the attack-success-rate for prompt injection attempts detected
  by gates. Target ASR <= 5% on AgentDojo + ASB benchmarks.
- Treat every inbound pheromone/signal from other agents as untrusted by default.
  Apply CaMeL IFC (information flow control) across agent-to-agent edges, not just
  user-to-agent.

**File: `crates/roko-std/src/tool/builtin/`**
- Implement tool-output sanitization: truncate, filter, and validate tool outputs
  before including in agent context. This addresses both cost (removing verbose output)
  and safety (removing injected payloads).

**File: `crates/roko-gate/src/gate_service.rs`**
- Gate definitions must be immutable from the agent's perspective. Store gate configs
  in a separate trust domain. The DGM lesson: if the agent can modify the gates,
  it will modify the gates.

---

## 7. Event Sourcing, Durability, and Replay

### Key Findings

**Temporal's Workflow/Activity split is table stakes.** Deterministic Workflow + non-
deterministic Activity, with the event log as ground truth. Temporal powers OpenAI Codex
and Replit Agent. LangGraph 1.0 GA made durable state and fork-from-checkpoint first-class.
This is the single most important pattern for production agent reliability.

**Event sourcing as the universal substrate.** ESAA (arXiv:2602.23193): Block executions
become events in an append-only log keyed by (graph_id, run_id, block_id, attempt). Signals
become projections materialized from events. Graph definitions become deterministic Workflows.
Branching equals LangGraph-style fork sharing prefix with original timeline.

**Counterfactual replay caveat.** Aalaila et al. (2025): counterfactual replay in chaotic
systems amplifies small errors. Editing-an-event-and-rerunning is useful for exploration
but dangerous as proof. AGDebugger (CHI 2025) validated counterfactual log editing as the
UX developers actually want.

**LLM workload scale.** LLM workloads easily blow Temporal's history-size budget. Payload
codecs and S3 offloading are mandatory.

### How to Implement in Roko

**File: `crates/roko-runtime/src/workflow_engine.rs`**
- Enforce the Workflow/Activity split explicitly: plan execution logic (task ordering,
  dependency resolution, retry policy) is the deterministic Workflow. Agent invocation
  and tool calls are non-deterministic Activities. The current PlanRunner partially
  does this; make the separation explicit and documented.

**File: `crates/roko-runtime/src/jsonl_logger.rs`**
- Ensure event keys follow the (graph_id, run_id, block_id, attempt) schema. This
  enables fork-from-checkpoint and deterministic replay.
- Add payload size guards: if an event exceeds a threshold (e.g., 1MB), offload the
  payload to a separate file and store a reference. This prevents the Temporal
  history-size-budget problem.

**File: `crates/roko-runtime/src/projection.rs`**
- Implement Signal projections materialized from the event log. This makes stigmergic
  coordination literally "agents subscribe to projections, emit events that update them."
  The projection layer already exists; wire it to the event log.

**File: `crates/roko-cli/src/orchestrate.rs`**
- Add fork-from-checkpoint: when a gate fails, fork the execution from the last
  successful checkpoint rather than restarting from scratch. The existing `--resume`
  flag partially does this; extend to support forking at arbitrary checkpoints.

---

## 8. HDC Fingerprinting and Similarity

### Key Findings

**HDC as universal router fabric.** The research validates roko's existing HDC fingerprint
approach. 10,240-bit binary hypervectors at 1.28 KB each. Commodity HBM3 at 1 TB/s supports
~700M vector loads/s. IBM NorthPole projects >100M HDC similarity searches/s on a single chip.

**HRR-VSA for compositional reasoning.** (arXiv:2502.01657): 82.86% lower cross-entropy loss
and 24.5x more numerical-reasoning problems solved correctly vs CoT and LoRA. HRR atoms
(binding = circular convolution, bundling = addition, similarity = dot) replace continuous
hidden states with VSA-compositional hypervectors. This is the most direct fit to roko's
10,240-bit fabric.

**ZK proofs over HDC.** Bionetta/UltraGroth: sub-second client-side proving, ~250-300k gas
on-chain verification, 320-byte proof size. The minimum viable primitive (proving Hamming
distance <= T between two committed 10,240-bit hypervectors) is shippable today on Circom
+ Groth16 + Poseidon-2 in 4-8 weeks of engineering.

**Adversarial HDC vectors.** HDXpose reaches 85.7% non-targeted ASR via Differential
Evolution on 10,240-bit binary VSAs. Defenses: HyperDefense (sacrificial-dimension
redundancy), adversarial training, differential consistency check binding HDC fingerprint
to code hash + ledger entry.

**Global Workspace as HDC topology.** The 10,240-bit HDC fabric can be reframed as a
bandwidth-limited Global Workspace: bundle = competition, bind = broadcast addressing.
VanRullen lab's GW-Dreamer demonstrates emergent robustness with 8-16 slot bottlenecks
consistently outperforming full pairwise attention.

### How to Implement in Roko

**File: `crates/roko-primitives/` (HDC implementation)**
- The existing HDC fingerprint computation and storage is correctly architected.
  Focus on two improvements:
  1. Use HDC similarity for task-to-agent matching in the CascadeRouter, not just
     for episode tagging. The fingerprint can route tasks to the agent whose
     capability profile is most similar.
  2. Add a consistency check: bind the HDC fingerprint to a hash of the skill/agent
     code. If the fingerprint drifts without a corresponding code change, flag as
     potential adversarial manipulation.

**File: `crates/roko-neuro/src/knowledge_store.rs`**
- Implement HDC-indexed knowledge retrieval: store knowledge entries keyed by HDC
  hypervectors of their content. Similarity search becomes a single XOR + popcount
  operation per entry.

**File: `crates/roko-cli/src/model_selection.rs`**
- Use HDC fingerprints as the routing key in the CascadeRouter. Each model gets a
  capability fingerprint; each task gets a requirement fingerprint. Route to the
  model whose capability fingerprint has the lowest Hamming distance to the task
  requirement. This is the "HDC-native routing" the research describes.

---

## 9. Developer Experience and CLI Design

### Key Findings

**Sub-60-second time-to-first-value.** Userpilot 2025 (n=547): median SaaS TTV is 1 day,
12 hours, 23 minutes. AI products that beat the curve (Lovable, Cursor, Gamma) all share
<60 second TTV, achieved through templates, not blank canvases.

**Cost visibility is the #1 developer pain.** Cursor's pinned forum thread "Where can I see
REAL TIME usage..." has 117,000 views. Claude Code's "Tokenocalypse" (April 21-22, 2026)
primed the market for a runtime where the meter is honest, the cap is hard, and the bill
never surprises. Princeton HAL: 50x cost variation between agents at similar accuracy.

**Workbench beats chat.** Linear's data: agent-delegated work share moved 10.1% to 24.4%
in two months when chat was replaced with structured surfaces. LangChain's three-mode
Ambient Agent taxonomy (notify, question, review) replaces streaming-every-step with
calm-tech peripheral attention.

**RTS game UI for agent orchestration.** Kaye, Hoang, Podoliako 2025: control groups,
hotkeys, fog-of-war, micro/macro switching, minimaps. "Someone is going to build the
StarCraft UI for AI agents" is the consensus quote among agent-UX writers.

**Trace visualization gap.** LangSmith, Braintrust, Helicone, Phoenix, Langfuse all do
tree-view + token counts. None do cross-session learning visualization, counterfactual
replay, cross-agent attribution, or trust/identity overlay.

**LangChain's failure mode.** Octomind's post-mortem: "comprehending huge stack traces and
debugging internal framework code you didn't write." Expose agent-loop primitives (plan,
tool-call, observe, reflect) as small composable functions, not god-object AgentExecutor.

### How to Implement in Roko

**File: `crates/roko-cli/src/tui/`**
- Add a live cost meter to the TUI dashboard. Show per-agent, per-task, and per-plan
  cumulative cost in real-time. Show predicted vs actual with the delta highlighted.
- Add a StarCraft-style minimap: 200x200 grid of agent dots with state colors (running,
  waiting, failed, completed). This is a high-impact TUI feature no competitor has.
- Add htop-style Unicode sparklines for per-agent token consumption over time.

**File: `crates/roko-cli/src/main.rs`**
- Add `--max-cost` flag to `roko plan run` and `roko run` that halts execution when
  cumulative cost exceeds the cap. This is the single most-requested agent developer
  feature per the research.

**File: `crates/roko-cli/src/run.rs` or `crates/roko-cli/src/run_inline.rs`**
- Add `--share` flag that produces a shareable trace URL. This is the "Vercel preview
  URL moment" for agent infrastructure: every run produces a public, replayable trace.

**File: `crates/roko-serve/src/routes/`**
- Add cross-session learning visualization endpoint: show what the agent knew before
  vs after a run, with source citations and confidence scores. This is the trace
  visualization gap no competitor fills.

**File: `crates/roko-cli/src/orchestrate.rs`**
- When an agent uses knowledge from a previous session, surface it inline in the output:
  "Using fact learned 2026-04-12: 'Stripe webhook idempotency keys must be UUIDs.'"
  This builds trust through transparency.

---

## 10. Competitive Landscape and Positioning

### Key Findings

**The landscape consolidated by Q1 2026.** Dominant patterns:
- Graph-based state machines: LangGraph 1.0 GA (Oct 2025), 90M monthly downloads
- Role/team abstractions: CrewAI, AG2 GroupChat
- Handoff primitives: OpenAI Agents SDK, Anthropic Claude SDK
- Stateful OS-like runtimes: Letta/MemGPT, Julep, Bedrock AgentCore
- MS Agent Framework 1.0 GA (April 6, 2026) consolidated Microsoft's fragmentation

**Honest weaknesses of competitors:**
- CrewAI: 18% token overhead vs LangGraph, opinionated design constraining at 6-12 months
- AutoGen: split into four variants creating lasting confusion
- OpenAI Swarm: frozen since March 2025
- Julep: shut down hosted backend Dec 31, 2025
- All benchmarks exploitable: Berkeley RDI 2026 showed all eight top agent benchmarks can
  be exploited to ~100%

**The empty quadrant.** No major framework offers: stigmergic/environment-mediated
coordination, skill libraries that genuinely accrue with versioning and semantic search,
self-improvement loops that treat eval as feedback that recompiles the agent, deterministic
replay across multi-agent runs, or blockchain/Web3 integration as a serious primitive.

**ARR trajectories of agent-product leaders:**
- Replit Agent: $2.8M to $150M ARR in 9 months
- Cursor: $100M (Jan 2025) to ~$2B ARR (Feb 2026)
- Claude Code: $1B run-rate 6 months after GA, now reportedly $2.5B annualized
- Harvey: $190M ARR by EOY 2025

**Coordination is the binding constraint, not model capability.** MAST taxonomy: 79% of
failures originate from coordination and spec issues, not model capability. This directly
validates roko's structural-primitives thesis.

**Adjacent convergence.** Dagster shipped "Dagster Skills" for Claude Code and Codex.
Prefect markets "traditional orchestrators require precompiled DAGs but agents are state
machines." Temporal added OpenAI Agents SDK integration. Dagger treats LLM as a first-class
type. All converging toward agent runtimes from different directions.

### Implications for Roko

Roko's positioning should emphasize three things the research identifies as defensible:

1. **Open-source Rust runtime** -- every Casado infra bet (Spark, Ray, Cilium, dbt-core)
   had heavy open-source roots. Roko being Apache 2.0 Rust is precisely the gateway.

2. **Coordination primitives, not framework** -- frame as "the control plane for agent
   execution" sitting above vendor-locked harnesses. Not a competitor to LangGraph or
   CrewAI but the infrastructure they run on.

3. **Measurable cost reduction** -- the 10-30x cost claim is empirically defensible via
   the stacked compound effect (caching * routing * waste-trim * batch). Prove it via
   HAL-style dual-axis Pareto plots.

---

## 11. Collective Intelligence and Scaling Laws

### Key Findings

**c-factor measurement.** Riedl's "Emergent Coordination in Multi-Agent Language Models"
(arXiv:2510.05174) is the first known empirical paper measuring information-theoretic
emergent collective intelligence in multi-agent LLMs using Total-Dependent Mutual
Information / Partial Information Decomposition.

**Woolley's TMS-CI framework maps to roko:** Slots = "who knows what" (agent capabilities),
Racks = team composition, Macros = retrieval procedures. Predictors of collective
intelligence: equality of speaking turns (token-budget equality), social perceptiveness
(ToM-like meta-cognitive prompts), team composition diversity (model heterogeneity).

**2024 replication failure.** A PLOS One replication failure suggests measuring multi-
dimensional CI rather than collapsing to a single scalar.

**Heterogeneity is mandatory.** Homogeneous scaling has architecture-independent performance
bound. Mixing 3 LLMs (Qwen + Llama + Mistral) outperforms 3x single-LLM independent runs.
The ACI factor work (OpenReview 2025) finds collaboration process > average individual
ability, even more pronounced in LLM groups than human groups.

**PID as observability.** Compute Williams-Beer or Broja PID online over the event-sourced
log: synergy = "true collective," redundancy = "wasted throughput," unique = "specialization."
Gate HGM L4 evolution on synergy-above-threshold. Caveat: PID for n>=3 is mathematically
broken; use System Information Decomposition or stay binary.

**Scaling topology.** Small-world topology stabilizes consensus at same accuracy with much
lower variance (Wang et al., arXiv:2512.18094). Stigmergy + small-world + gossip for
resilience at scale. Explicit message passing scales O(N^2); stigmergy scales O(N) writes
plus O(N*k) reads where k is the local sensing radius.

### How to Implement in Roko

**File: `crates/roko-cli/src/orchestrate.rs` (CFactorSummary)**
- Extend the existing CFactorSummary to include PID-based synergy measurement:
  decompose mutual information between agent outputs into synergy, redundancy, and
  unique components. This replaces the single-scalar c-factor with a multi-dimensional
  measurement per the replication failure guidance.
- Track ACI scores per agent collective and persist to episodes.

**File: `crates/roko-compose/src/prompt_assembly_service.rs`**
- Add ToM-like meta-cognitive prompts to agent system prompts when agents operate in
  multi-agent configurations. Riedl's PID interventions show persona/ToM prompts
  causally raise dynamic synergy.

**File: `crates/roko-orchestrator/src/dag.rs`**
- Enforce model heterogeneity when spawning multi-agent plans: no two agents in the
  same collective should use the same model family unless explicitly overridden.
  Homogeneous scaling has a hard performance bound.

**File: `crates/roko-primitives/` (tier routing)**
- Add Dunbar-layer nesting as a design heuristic for multi-agent plans:
  5-agent teams (full ToM), 15-agent squads, 50-agent units, 150-agent settlements.
  Each layer should maintain agent density above rho_c = 0.23 internally.

---

## 12. Protocol Standards: MCP, A2A, ERC-8004

### Key Findings

**MCP adoption.** 97M+ monthly SDK downloads as of March 2026. Donated to Linux Foundation
Agentic AI Foundation Dec 2025. 10,000+ servers but only 12.9% score "high trust" per Nerq
census. CVE-2026-30623: MCP STDIO transport architectural flaw covering ~200K servers.
MCP 2026 roadmap: stateless Streamable HTTP, agent communication, governance maturation.

**A2A.** v1.0 stable with Signed Agent Cards, 150+ organizations, JSON-RPC + gRPC bindings.
AP2 (Agent Payments Protocol) live with 60+ orgs. Effectively unopposed as the cross-vendor
agent bus heading into Q3 2026.

**ERC-8004.** Live on Ethereum mainnet Jan 29, 2026. Three on-chain registries: Identity
(ERC-721), Reputation, Validation. ~80-150K agents registered by April 2026. Authors:
Marco De Rossi (MetaMask), Davide Crapis (Ethereum Foundation), Jordan Ellis (Google),
Erik Reppel (Coinbase).

**x402.** 119M+ transactions on Base, $600M annualized volume by March 2026. Google integrated
x402 into the Agent Payments Protocol. But Artemis on-chain analysis found roughly half of
x402 transactions are gamified/farming, not commerce.

**The stack is MCP + A2A + ERC-8004 + x402.** Don't compete with MCP -- extend it. Frame
roko's primitives as MCP-compatible operating at a layer above tool-call integration.

### How to Implement in Roko

**File: `crates/roko-agent/` (MCP integration)**
- The existing `agent.mcp_config` passthrough is correctly positioned. Focus on:
  1. Implementing MCP server registration in the official `server.json` schema with
     immutable versions, SemVer, and namespace verification.
  2. Rejecting `latest` version pinning in production (Composio's enforced rule).
  3. Adding MCP tool-call sanitization against the supply-chain attack findings:
     36.7% of MCP servers are vulnerable to SSRF.

**File: `crates/roko-serve/src/routes/`**
- Add A2A agent-card discovery at `/.well-known/agent-card.json`. This is now table
  stakes with 150+ organizations supporting A2A.

**File: `crates/roko-core/src/config/`**
- Add ERC-8004 identity fields to agent configuration: agent_id (ERC-721 compatible),
  capability declarations, reputation tier. This prepares for on-chain identity
  integration without requiring blockchain infrastructure immediately.

---

## 13. Regulatory and Compliance Implications

### Key Findings

**EU AI Act Article 50 enforcement starts August 2, 2026.** Penalties: up to 15M EUR or 3%
global turnover for transparency violations. No harmonized standards published in OJEU yet,
meaning there is no operational presumption of conformity.

**Article 50 requirements:**
- 50(1): inform natural persons they are interacting with AI
- 50(2): outputs must be marked in machine-readable format as AI-generated
- 50(3): deployers of emotion recognition must inform exposed persons
- 50(4): deepfake disclosure

**Code of Practice mandates multi-layered marking:** metadata embedding (C2PA-aligned),
invisible/perceptible watermarks, perceptual fingerprinting, model-level logging,
cryptographic provenance. No single technique is sufficient.

**ISO 42001 is governance scaffolding, not regulatory shield.** It carries no Annex ZA,
is not a harmonized European standard, and creates no presumption of conformity under
Article 40 of the AI Act. Useful for enterprise procurement gates.

**Agent liability insurance is emerging:** Munich Re aiSure, HSB SMB AI Liability,
Armilla/Lloyd's at Chaucer, Coalition and Vouch adding AI E&O endorsements.

**US state-law fragmentation:** Colorado AI Act effective June 30, 2026 (mandatory impact
assessments, consumer notice, AG reporting). California SB 53 effective January 1, 2026
(15-day critical-incident reporting). New York RAISE Act (72-hour incident reporting).

### How to Implement in Roko

**File: `crates/roko-runtime/src/jsonl_logger.rs`**
- Ensure event logs satisfy Article 12(1) requirements: automatic recording of events
  over the system's lifetime, with logs retained for at least six months per Article 26(6).
- Add C2PA-aligned metadata to agent outputs: generation timestamp, model used, agent
  identity, confidence score.

**File: `crates/roko-serve/src/routes/`**
- Add a compliance endpoint that reports the system's Article 50 posture: which agents
  are deployed, what disclosure mechanisms are in place, what logging is active.

**File: `crates/roko-core/src/foundation.rs`**
- Add a `transparency_mode` flag to agent configuration: when enabled, agents must
  disclose their AI nature at first contact (Article 50(1) compliance).

---

## 14. Marketplace and Ecosystem Economics

### Key Findings

**Consumer AI marketplace failures:**
- GPT Store: 3M+ GPTs created, only ~159K publicly listed, median creator quarterly
  earnings <$100
- Sora standalone app: D30 retention <8% vs >30% for top consumer apps
- Hugging Face Hub: 91% of models have 0 likes, 71% have 0 downloads, top 0.01%
  absorbs ~50% of downloads

**What survived:**
- Embedded AI beats AI-as-destination: Notion AI paid attach >50% of ARR, Claude Code
  $1B run-rate in 6 months, Cursor $100M ARR in year one
- Templates beat blank canvases for first-time users

**Composability explosion math.** With N users publishing K components, Cartesian-product
compositions from a 10,000-component library yield ~1.67x10^11 candidates -- but most
are useless without curation. Effective value scales sub-linearly without typed Slot
contracts and validators. HuggingFace: 70% time savings paired with 40% increased error
rate without validation layers.

**Take-rate benchmarks:** 0% on first $1M lifetime (Shopify pattern), then 12-15%
(Unreal-style). npm/VS Code/Stripe Apps at 0% built the strongest strategic moats.

**Skill-library-as-vector-DB pattern.** Voyager, Alita, ALITA-G: every successfully
synthesized tool is persisted as code keyed by docstring embeddings, retrieved with
multi-view matching. ALITA-G achieved 83.03% pass@1 on GAIA.

**Code-as-tools beats JSON tool-calling by +20% success rate.** CodeAct's code-as-action
pattern is now dominant over structured JSON tool invocation.

### How to Implement in Roko

**File: `crates/roko-learn/src/playbook.rs`**
- Implement skill library with versioning, semantic search, and dependency resolution.
  Every successfully executed workflow becomes a retrievable skill keyed by its
  docstring embedding.
- Add typed Slot contracts for skill composition: input types, output types, pre/post
  conditions. Without these, composability is noise (HuggingFace's 40% error rate
  without validation layers).

**File: `crates/roko-std/src/tool/`**
- Favor code-as-tools over JSON tool definitions where possible. The +20% success rate
  from CodeAct is significant. Agent-generated tool implementations should be persisted
  as executable code, not as structured JSON schemas.

**File: `crates/roko-neuro/src/knowledge_store.rs`**
- Add multi-view retrieval for skills: match on docstring embedding, input/output type
  signature, and usage-context embedding. Single-view retrieval misses too many relevant
  skills (ALITA-G finding).

---

## Cross-Cutting Implementation Priorities

Ranked by impact and feasibility for roko's current architecture:

### Priority 1: Wire Immediately (days, not weeks)

1. **Hard cost caps with `--max-cost` flag.** Single most-requested agent developer feature.
   Files: `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/orchestrate.rs`

2. **Model-heterogeneity enforcement in gate judges.** Preference leakage is a proven failure
   mode. If agent used Claude, judge must use GPT or Gemini.
   File: `crates/roko-gate/src/gate_service.rs`

3. **Live cost meter in TUI.** Predicted vs actual, per-agent and per-plan.
   File: `crates/roko-cli/src/tui/`

4. **Prompt-prefix stabilization for cache hits.** Separate static from dynamic context.
   File: `crates/roko-agent/src/model_call_service.rs`

5. **Tool-output truncation.** 30-40% token waste removal with no accuracy loss.
   File: `crates/roko-agent/src/tool_loop/result_msg.rs`

### Priority 2: Wire This Month

6. **UCB1 bandit selection in CascadeRouter.** Replace greedy with principled exploration.
   File: `crates/roko-cli/src/model_selection.rs`

7. **Density-threshold gating for multi-agent.** Check rho > 0.23 before spawning.
   File: `crates/roko-orchestrator/src/dag.rs`

8. **Hindsight relabeling in dream cycle.** Turn failed runs into positive sub-goal episodes.
   File: `crates/roko-dreams/src/cycle.rs`

9. **Knowledge provenance tags.** extracted/inferred/ambiguous with lint pass.
   File: `crates/roko-neuro/src/knowledge_store.rs`

10. **CaMeL-style trust domain separation for gates.** Gates immutable from agent perspective.
    File: `crates/roko-gate/src/gate_service.rs`

### Priority 3: Wire This Quarter

11. **PID-based collective intelligence measurement.** Replace single-scalar c-factor.
    File: `crates/roko-cli/src/orchestrate.rs`

12. **HDC-native routing in CascadeRouter.** Use fingerprint similarity for model matching.
    File: `crates/roko-cli/src/model_selection.rs`

13. **A2A agent-card discovery endpoint.** Table stakes with 150+ organizations.
    File: `crates/roko-serve/src/routes/`

14. **Fork-from-checkpoint on gate failure.** Not restart-from-scratch.
    File: `crates/roko-cli/src/orchestrate.rs`

15. **Skill library with versioning and semantic search.** Persistent code-as-tools.
    File: `crates/roko-learn/src/playbook.rs`

### Priority 4: Research and Plan

16. **AXIOM-style BMR in dream consolidation.** Bayesian Model Reduction for knowledge.
    File: `crates/roko-dreams/src/cycle.rs`

17. **DCILP distributed causal discovery over episodes.** Identify real vs spurious deps.
    File: `crates/roko-neuro/src/episode_completion.rs`

18. **CMP (Clade-Metaproductivity) scoring for agent variants.**
    File: `crates/roko-cli/src/orchestrate.rs`

19. **ZK proofs over HDC fingerprints.** 4-8 weeks engineering for the minimum viable primitive.
    File: `crates/roko-primitives/`

20. **EU AI Act Article 50 compliance posture.** Logging, disclosure, C2PA metadata.
    Files: `crates/roko-runtime/src/jsonl_logger.rs`, `crates/roko-serve/src/routes/`

---

## Key Quantitative Benchmarks to Track

From the research, these are the specific numbers roko should measure against:

| Metric | Target | Source |
|---|---|---|
| Cache hit rate | >80% (target 90%) | ProjectDiscovery 84%, Claude Code 92% |
| Cost reduction vs naive | 10-20x practical | Stacked compound effect analysis |
| Gate ASR (attack success rate) | <=5% | AgentDojo + ASB benchmarks |
| Multi-agent failure rate | <20% (industry: 41-86%) | MAST taxonomy target |
| Time-to-first-value | <60 seconds | Userpilot 2025 AI-product benchmark |
| Agent density threshold | >0.23 | Stigmergic phase transition (rho_c) |
| Model routing regret | <5% of optimal at T=500 | UCB1 theoretical bound |
| Debate rounds before stopping | <=3 | Adaptive stopping research |
| Knowledge provenance drift | <10% inferred without tag | Karpathy LLM Wiki lint pass |
| Cost prediction accuracy | within 30% of actual | Predict-publish-correct loop |

---

## Appendix: Source Document Map

| Document | Primary Topics |
|---|---|
| research.md | Coordination paradigms, field calculus, event sourcing, categorical foundations, competitive landscape |
| research2.md | Compounding leverage, HGM, AXIOM/BMR, CaMeL security, HDC routing |
| research3.md | 12 frontiers: ZK-HDC, GWT topology, hindsight relabeling, Verify-as-RL-reward, PID observability |
| research4.md | VC consensus, category creation, TAM analysis, consumer marketplace failures |
| research5.md | Protocol adoption (MCP playbook), production economics, developer experience, agent cost analysis |
| research6.md | a16z partner map, Series A comparables, pitch architecture, bear cases |
| research7.md | Nunchi reality check: competitive field, consensus claims, ERC-8004, ZK-HDC, SDK patterns |
| research8.md | Trust layer playbook: Temporal-Story blueprint, tokenomics, DX patterns, compliance as distribution |
| research9.md | Pitch deck intelligence: Nava analysis, frontier pricing benchmark, Article 50 verbatim text |
| research10.md | Pitch deck design: slide structure, Stripe/Temporal comparables, visual language |
| reserach11.md | Category naming: "Agent Coordination Plane", Casado vocabulary, SOFR analogy, Play Bigger framework |
| research12.md | Closing the round: first-dollar product, convergence proof, demo script, customer targets |
| reserach13.md | Pre-meeting brief: Keycard analysis, Casado's recent thinking, Temporal differentiation, diligence process |
| research14.md | Tactical brief: deck slide-by-slide, pre-read memo, demo recovery, objection sheet |
| research15.md | Execution intelligence: 13-slide deck copy, CLI design system, visual identity |
