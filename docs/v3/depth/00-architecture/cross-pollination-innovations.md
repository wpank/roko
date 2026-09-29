# Cross-Pollination Innovations

> **v3 depth file** -- `/docs/v3/depth/00-architecture/cross-pollination-innovations.md`
> Canonical source: v1 `docs/v1/00-architecture/30-cross-pollination-innovations.md`
> Status: **Mixed** -- Most compositions depend on primitives that are partially or fully
> target-state. The product advantage today comes from the working Rust orchestration stack,
> gates, HDC-enabled learning/neuro, feedback loops, and interfaces already in code.

---

## Abstract

Eight innovations that emerge from composing Roko's cognitive subsystems in novel ways.
Each connects two or more orthogonal systems -- Daimon, Neuro, Dreams, coordination,
code intelligence, learning, safety -- to produce capabilities no single subsystem
provides. These are structural compositions: the architecture's trait-based design means
each innovation can often be expressed as trait composition rather than a wholly separate
subsystem, even though several examples would still require substantial new code.

The moat is the composition. The advantage does not come from any isolated primitive.
Pulse, HDC fingerprinting, demurrage, the heuristic-with-falsifier pattern, replication
ledger, c-factor, worldview clusters, two-fabric operator generalization, and plugin
tiers each have prior art or close analogues. The net-new artifact is the reinforcing
weave across Substrate, Bus, shared traits, and shared evidence loops that makes those
primitives compound into a single system. A competitor can copy one primitive; copying
the aligned kernel decisions that make the whole architecture reinforce itself is a
much harder, slower rewrite.

### Net-New vs. Carefully Integrated

The right way to read this chapter is not "which primitive is unique by itself?" but
"which composition is net-new?" Most primitives are valuable because they are carefully
integrated with the rest of the runtime, not because they are novel in isolation.

| Primitive cluster | Composition role | Net-new claim |
|---|---|---|
| `Pulse` + two-fabric operator generalization | One operator algebra spans ephemeral and durable artifacts | Medium polymorphism becomes a first-class runtime property |
| HDC fingerprint + c-factor + worldview clusters | Similarity, collective signal, and emergent domains reinforce routing | Structural evidence replaces ad hoc metadata for comparison, organization, and steering |
| Demurrage + heuristic with falsifier + replication ledger | Ideas decay unless justified; claims stay auditable | Continuous correction by lived evidence once supporting runtime surfaces exist |
| Plugin tiers | Risk-aware extension boundaries | Extensibility is native to the kernel instead of bolted on |

---

## 1. HDC + Active Inference

**Beliefs as Vectors, Free Energy in Hamming Space**

### Motivation

Roko already has two powerful subsystems that operate in isolation:

- **Neuro** encodes knowledge as 10,240-bit Binary Spatter Code (BSC) vectors with
  XOR binding, majority-vote bundling, and cyclic permutation (Kanerva 2009; Kleyko
  et al. 2022).
- **The heartbeat's dual-process gating** computes prediction error to route between
  T0/T1/T2 tiers, an approximation of active inference's Expected Free Energy
  (Friston 2010).

The gap: prediction error is currently a scalar derived from probe anomaly counts and
regime drift. The agent has no structured belief representation that can be updated,
compared, or composed. Active inference requires a generative model -- a probability
distribution over world states that the agent updates via sensory prediction errors.
HDC vectors are that model.

### Research Basis

- **Bybee & Bhatt (2024)** "Modelling Neural Probabilistic Computation Using Vector
  Symbolic Architectures," *Frontiers in Computational Neuroscience* 18. Demonstrates
  that VSA operations natively compute marginalization, entropy, and mutual information
  over probability distributions. Belief updating with observations reduces to vector
  addition in HDC space -- no matrix inversion, no gradient descent.

- **Heddes et al. (2024)** "Hyperdimensional Computing: A Framework for Stochastic
  Computation and Symbolic AI," *Journal of Big Data*. Frames HDC as inherently
  stochastic computing where noise tolerance is a feature aligned with approximate
  Bayesian inference.

- **Renner et al. (2024)** "Brain-Inspired Computational Intelligence via Predictive
  Coding," arXiv:2308.07870v3. Formalizes predictive coding as a general-purpose
  learning algorithm implementable in distributed architectures -- precisely the
  structure HDC provides.

- **Friston (2010)** "The free-energy principle: a unified brain theory?" *Nature
  Reviews Neuroscience* 11(2). The foundational formulation: agents minimize
  variational free energy F = E_q[ln q(s) - ln p(o,s)] where q(s) is the approximate
  posterior (beliefs about states), p(o,s) is the generative model, and o are
  observations.

### Core Idea

Encode the agent's generative model as an HDC vector. Each belief about the world is a
role-filler binding in a 10,240-bit BSC vector. Prediction error becomes Hamming distance
between the predicted observation vector and the actual observation vector. Free energy
minimization becomes vector update operations -- no matrix algebra, O(160) word operations.

```
Generative model mu: HDC vector encoding current beliefs
Predicted observation o_hat: decode(mu) via unbinding
Actual observation o: encode current sensory state
Prediction error epsilon: hamming_distance(o_hat, o) / 10240
Free energy F ~ epsilon + complexity_penalty(mu)
Update: mu' = bundle([mu, weighted_bind(o, learning_rate)])
```

The elegance: free energy is a scalar derived from Hamming distance, which Roko already
computes in ~50ns via POPCNT. No new mathematical machinery needed.

### Algorithm: HDC Free Energy Minimization

```
Algorithm: HdcActiveInference

Input:
  mu in {0,1}^D          -- current belief vector (D = 10,240)
  o in {0,1}^D           -- observation vector (encoded from probes)
  R_role in {0,1}^D      -- role vectors for each state variable
  alpha in (0, 1)        -- learning rate (default 0.05)
  lambda in [0, 1]       -- complexity weight (default 0.01)
  mu_prior in {0,1}^D    -- prior belief vector (personality baseline)

Output:
  mu' in {0,1}^D         -- updated belief vector
  F in R                 -- free energy (scalar)
  tier in {T0, T1, T2}   -- selected inference tier

Steps:
  1. PREDICT:
     o_hat = unbind(mu, R_observation)          // Extract predicted observation
     // unbind(a, b) = XOR(a, b) in BSC

  2. COMPUTE PREDICTION ERROR:
     epsilon = hamming(o_hat, o) / D            // Normalized Hamming distance in [0, 1]
     // epsilon ~ 0.5 means random (maximum surprise)
     // epsilon ~ 0.0 means perfect prediction (zero surprise)

  3. COMPUTE COMPLEXITY:
     kappa = hamming(mu, mu_prior) / D          // Divergence from prior beliefs
     // Penalizes beliefs that deviate too far from baseline

  4. COMPUTE FREE ENERGY:
     F = epsilon + lambda * kappa               // Surprise + complexity penalty
     // F in [0, 1 + lambda], lower is better

  5. UPDATE BELIEFS (if epsilon > threshold):
     correction = bind(o, R_observation)        // Encode observation as belief update
     candidates = [mu repeated (1-alpha)*N times, correction repeated alpha*N times]
     mu' = majority_vote(candidates)            // Soft blend via stochastic bundling
     // N = bundle size parameter (default 100)

  6. SELECT TIER:
     if F < 0.10:  tier = T0                   // Beliefs accurate, no LLM needed
     if F < 0.25:  tier = T1                   // Moderate surprise, fast model
     else:         tier = T2                   // High surprise, full reasoning

  7. EMIT:
     Return (mu', F, tier)
```

**Complexity**: O(D/w) per step where w = 64 (word size). For D = 10,240: O(160) word
operations per belief update. At ~50ns per operation: ~8us total per tick.

### Rust Sketch

```rust
use roko_primitives::hdc::{HdcVector, hamming_distance, bind, majority_vote};

/// Belief state encoded as HDC vector with active inference dynamics.
///
/// The generative model mu is a 10,240-bit BSC vector where each role-filler
/// binding encodes a belief about a state variable. Free energy F is computed
/// as normalized Hamming distance (prediction error) plus complexity penalty.
pub struct HdcBeliefState {
    /// Current belief vector (generative model mu)
    pub mu: HdcVector,
    /// Prior belief vector (personality baseline from Daimon)
    pub mu_prior: HdcVector,
    /// Role vectors for state variables (deterministic from seeds)
    pub role_vectors: Vec<(String, HdcVector)>,
    /// Learning rate alpha for belief updates
    pub learning_rate: f64,
    /// Complexity weight lambda (KL penalty on deviation from prior)
    pub complexity_weight: f64,
    /// Historical free energy values (ring buffer for trend detection)
    pub free_energy_history: VecDeque<f64>,
    /// Maximum history length
    pub max_history: usize,
}

/// Result of one active inference step.
pub struct InferenceResult {
    /// Updated belief vector
    pub mu_prime: HdcVector,
    /// Scalar free energy F = epsilon + lambda*kappa
    pub free_energy: f64,
    /// Prediction error epsilon (normalized Hamming distance)
    pub prediction_error: f64,
    /// Complexity penalty kappa (divergence from prior)
    pub complexity: f64,
    /// Selected inference tier
    pub tier: InferenceTier,
    /// Whether beliefs were updated (epsilon exceeded threshold)
    pub beliefs_updated: bool,
}

impl HdcBeliefState {
    /// Default parameters calibrated for coding domain.
    pub const DEFAULT_LEARNING_RATE: f64 = 0.05;
    pub const DEFAULT_COMPLEXITY_WEIGHT: f64 = 0.01;
    pub const DEFAULT_HISTORY_SIZE: usize = 200;

    /// Tier thresholds (aligned with heartbeat gating).
    pub const T0_CEILING: f64 = 0.10;
    pub const T1_CEILING: f64 = 0.25;

    /// Create a new belief state from a prior (personality baseline).
    pub fn new(
        mu_prior: HdcVector,
        role_seeds: &[(String, u64)],
        learning_rate: f64,
        complexity_weight: f64,
    ) -> Self {
        let role_vectors: Vec<(String, HdcVector)> = role_seeds
            .iter()
            .map(|(name, seed)| (name.clone(), HdcVector::from_seed(*seed)))
            .collect();

        Self {
            mu: mu_prior.clone(),
            mu_prior,
            role_vectors,
            learning_rate,
            complexity_weight,
            free_energy_history: VecDeque::with_capacity(Self::DEFAULT_HISTORY_SIZE),
            max_history: Self::DEFAULT_HISTORY_SIZE,
        }
    }

    /// One step of HDC active inference.
    pub fn infer(&mut self, observation: &HdcVector) -> InferenceResult {
        // 1. PREDICT: extract predicted observation from belief vector
        let obs_role = self.role_for("observation");
        let predicted = bind(&self.mu, &obs_role);

        // 2. PREDICTION ERROR: normalized Hamming distance
        let epsilon = hamming_distance(&predicted, observation) as f64
            / HdcVector::TOTAL_BITS as f64;

        // 3. COMPLEXITY: divergence from prior beliefs
        let kappa = hamming_distance(&self.mu, &self.mu_prior) as f64
            / HdcVector::TOTAL_BITS as f64;

        // 4. FREE ENERGY: surprise + complexity penalty
        let free_energy = epsilon + self.complexity_weight * kappa;

        // 5. UPDATE BELIEFS (only if prediction error exceeds noise floor)
        let beliefs_updated = epsilon > 0.05;
        let mu_prime = if beliefs_updated {
            let correction = bind(observation, &obs_role);
            self.stochastic_blend(&self.mu, &correction, self.learning_rate)
        } else {
            self.mu.clone()
        };

        // 6. SELECT TIER based on free energy
        let tier = if free_energy < Self::T0_CEILING {
            InferenceTier::T0
        } else if free_energy < Self::T1_CEILING {
            InferenceTier::T1
        } else {
            InferenceTier::T2
        };

        // Record history for trend detection
        self.free_energy_history.push_back(free_energy);
        if self.free_energy_history.len() > self.max_history {
            self.free_energy_history.pop_front();
        }

        self.mu = mu_prime.clone();

        InferenceResult {
            mu_prime,
            free_energy,
            prediction_error: epsilon,
            complexity: kappa,
            tier,
            beliefs_updated,
        }
    }

    /// Stochastic blend: majority vote over weighted copies.
    fn stochastic_blend(
        &self,
        current: &HdcVector,
        correction: &HdcVector,
        alpha: f64,
    ) -> HdcVector {
        const BLEND_SIZE: usize = 100;
        let correction_count = (alpha * BLEND_SIZE as f64).round() as usize;
        let current_count = BLEND_SIZE - correction_count;

        let mut candidates = Vec::with_capacity(BLEND_SIZE);
        candidates.extend(std::iter::repeat(current).take(current_count));
        candidates.extend(std::iter::repeat(correction).take(correction_count));

        majority_vote(&candidates)
    }

    /// Free energy trend: positive = increasing surprise, negative = learning.
    pub fn free_energy_trend(&self, window: usize) -> f64 {
        if self.free_energy_history.len() < window * 2 {
            return 0.0;
        }
        let recent: f64 = self.free_energy_history
            .iter().rev().take(window).sum::<f64>() / window as f64;
        let earlier: f64 = self.free_energy_history
            .iter().rev().skip(window).take(window).sum::<f64>() / window as f64;
        recent - earlier
    }

    fn role_for(&self, name: &str) -> HdcVector {
        self.role_vectors
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| HdcVector::from_seed(fxhash::hash64(name.as_bytes())))
    }
}
```

### Integration Plan

| Step | What | Where | Connects To |
|---|---|---|---|
| 1 | Add `HdcBeliefState` to `roko-primitives` | `roko-primitives/src/belief.rs` | HDC vector ops already in crate |
| 2 | Initialize belief from Daimon personality | `roko-daimon/src/lib.rs` | PAD baseline -> mu_prior |
| 3 | Wire into heartbeat perception step | runner event loop | Replace scalar PE with HDC PE |
| 4 | Feed observation vectors from T0 probes | `roko-conductor/src/watchers/` | 16 probes -> observation bundle |
| 5 | Use free energy for tier gating | Heartbeat gating algorithm | F replaces anomaly-count formula |
| 6 | Persist belief state across sessions | `.roko/state/beliefs.bin` | Binary HDC vector (1,280 bytes) |
| 7 | Dream consolidation updates mu_prior | `roko-dreams` (NREM phase) | Slow prior update during Delta |

**Key insight**: The belief vector mu is the agent's target-state world model compressed to
1,280 bytes. It can be persisted, transmitted between agents, compared via Hamming
distance, and composed via bundling.

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_hamming_zero_when_beliefs_perfect` | hamming_distance(predicted, observed) = 0 when beliefs are perfect | Unit |
| `test_free_energy_decreases_monotonically` | Free energy decreases over repeated observations of same state | Unit |
| `test_tier_escalation_with_novelty` | T0 -> T1 -> T2 as observation novelty increases | Unit |
| `test_belief_update_idempotent` | Belief update is idempotent for identical observations | Unit |
| `test_mu_converges_to_prior` | mu converges to mu_prior when no observations arrive (complexity penalty) | Unit |
| `test_inference_cycle_under_10us` | Full inference cycle < 10us on M-series Apple Silicon | Benchmark |
| `test_belief_state_serializes_1280_bytes` | Belief state serializes to exactly 1,280 bytes | Unit |

---

## 2. Affect + Causal Discovery

**PAD Vectors as Interventional Variables in Causal Models**

### Motivation

Roko's Daimon tracks Pleasure-Arousal-Dominance (PAD) vectors that modulate behavior:
tier routing, exploration rate, context bidding, somatic marker lookup. But the
relationship between affect and outcomes is treated as correlational -- the OCC/Scherer
appraisal pipeline maps events to PAD deltas via fixed coefficients (gate pass -> P+=0.05,
task failure -> P-=0.20).

The problem: fixed coefficients cannot capture causal structure. When an agent is anxious
(high arousal, low dominance) and a task fails, is the anxiety *causing* the failure
(via conservative strategy selection), or is the failure *causing* the anxiety (via
appraisal)? Without causal models, the agent cannot distinguish these and cannot
intervene effectively.

### Research Basis

- **Qian et al. (2025)** "Teleology-Driven Affective Computing: A Causal Framework for
  Sustained Well-Being," arXiv:2502.17172. Proposes treating affect as a goal-directed
  adaptive process in a causal framework. Uses causal modeling to infer agents' unique
  affective concerns and provide tailored interventions.

- **Yang et al. (2024)** "Robust Emotion Recognition in Context Debiasing" (CLEF),
  CVPR 2024, arXiv:2403.05963. Formulates a generalized causal graph for emotion
  recognition that separates genuine emotional causes from confounding context.

- **SemEval-2024 Task 3** "Multimodal Emotion Cause Analysis in Conversations,"
  arXiv:2405.13049. Operationalizes emotion cause discovery as structured prediction
  over causal graphs.

- **Pearl (2009)** *Causality: Models, Reasoning, and Inference*. The three-level
  causal hierarchy: Association, Intervention, Counterfactual.

### Core Idea

Treat the PAD vector as a node in a Structural Causal Model (SCM) alongside task
outcomes, strategy choices, context variables, and environmental state. Use
interventional queries (do-calculus) to determine whether modifying affect would
change outcomes, and counterfactual queries to learn from hypothetical affect-strategy
pairings.

```
Causal Graph:

  Environment -> PAD -> Strategy -> Outcome
       |                             |
  Task_Difficulty -----------> Gate_Verdict
       ^                             ^
  Prior_Knowledge -> Model_Choice -> Quality

Interventions:
  do(PAD.arousal := 0.0)  -> Would outcome change?
  do(Strategy := Exploratory) -> Does affect still predict failure?

Counterfactuals:
  Given: anxious + failed
  Query: Had PAD.dominance been > 0.3, would task have passed?
```

This enables affect regulation as causal intervention: the agent can identify when its
emotional state is causally degrading performance and intervene on the PAD vector directly
(via contrarian retrieval, dream depotentiation, or forced behavioral state transition).

### Algorithm: Affective Causal Discovery

```
Algorithm: AffectCausalDiscovery

Input:
  episodes: Vec<Episode>              -- recent episodes with PAD + outcomes
  G_0: DAG                            -- initial causal graph (domain prior)
  significance_threshold: f64         -- p-value threshold (default 0.01)
  min_episodes: usize                 -- minimum sample size (default 50)

Output:
  G: DAG                              -- discovered causal graph
  interventions: Vec<Intervention>    -- recommended affect interventions

Variables (nodes in G):
  P, A, D                             -- PAD components (continuous [-1, 1])
  S in {Conservative, Balanced, Exploratory, Escalating, Proactive}
  T in {T0, T1, T2}                   -- tier selection
  O in {pass, fail}                   -- task outcome
  C in [0, 1]                         -- task complexity
  K in [0, 1]                         -- prior knowledge relevance

Steps:
  1. STRUCTURE LEARNING (PC algorithm, Spirtes et al. 2000):
     a. Start with complete undirected graph over {P, A, D, S, T, O, C, K}
     b. For each pair (X, Y), test conditional independence:
        X _|_ Y | Z  for all subsets Z of remaining variables
     c. Remove edge if independent (Fisher-Z test, threshold = significance)
     d. Orient edges via d-separation (colliders + acyclicity)

  2. PARAMETER ESTIMATION (linear SCM, MLE):
     For each edge X -> Y in G:
       beta_XY = cov(X, Y | pa(Y)\{X}) / var(X | pa(Y)\{X})

  3. INTERVENTIONAL QUERIES (do-calculus):
     For each PAD component P_i in {P, A, D}:
       E[O | do(P_i := v)] for v in {-0.5, 0.0, 0.5}
       If E[O | do(P_i := 0.5)] >> E[O | do(P_i := -0.5)]:
         P_i has strong positive causal effect on outcomes

  4. COUNTERFACTUAL ANALYSIS (Halpern-Pearl):
     For each failed episode e where |PAD_delta| > 0.15:
       Compute: O_cf = f(pa(O), do(PAD := PAD_counterfactual))
       If O_cf = pass: flag episode as "affect-caused failure"

  5. GENERATE INTERVENTIONS:
     For each PAD component with |causal_effect| > 0.10:
       If effect is negative (high arousal -> failure):
         Recommend: {trigger: ArousalAbove(0.4), action: ForceDepotentiation}
       If effect is positive (high dominance -> success):
         Recommend: {trigger: DominanceBelow(-0.2), action: InjectConfidence}

  6. UPDATE APPRAISAL COEFFICIENTS:
     Replace fixed OCC/Scherer deltas with learned causal effects:
       gate_pass: P += beta_gate->P, A += beta_gate->A, D += beta_gate->D
```

### Rust Sketch

```rust
/// A node in the affective causal graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AffectCausalVar {
    Pleasure, Arousal, Dominance,
    Strategy, Tier, Outcome,
    Complexity, KnowledgeRelevance,
}

/// Directed edge in the causal graph with estimated effect size.
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub from: AffectCausalVar,
    pub to: AffectCausalVar,
    pub beta: f64,           // Linear causal effect coefficient
    pub p_value: f64,        // Statistical significance
    pub confidence: f64,     // Bayesian posterior confidence
}

/// Result of a counterfactual query on a specific episode.
#[derive(Debug, Clone)]
pub struct CounterfactualResult {
    pub episode_id: String,
    pub actual_outcome: bool,
    pub counterfactual_pad: PadVector,
    pub counterfactual_outcome: bool,
    pub affect_caused: bool,
    pub causal_strength: f64,
}

/// Recommended intervention when affect causally degrades performance.
#[derive(Debug, Clone)]
pub struct AffectIntervention {
    pub trigger: AffectTrigger,
    pub action: AffectAction,
    pub expected_improvement: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum AffectTrigger {
    ArousalAbove(f64),
    DominanceBelow(f64),
    PleasureBelow(f64),
    FreeEnergyAbove(f64),
}

#[derive(Debug, Clone)]
pub enum AffectAction {
    ForceDepotentiation { target_arousal: f64 },
    InjectConfidence { dominance_boost: f64 },
    TriggerContrarianRetrieval,
    EscalateToDreamCycle,
    OverrideStrategy(DispatchStrategy),
}

/// The affective causal model: learns and queries causal structure
/// between PAD states and task outcomes.
pub struct AffectCausalModel {
    pub edges: Vec<CausalEdge>,
    pub learned_deltas: HashMap<AffectEvent, PadDelta>,
    pub interventions: Vec<AffectIntervention>,
    pub min_episodes: usize,
    pub significance: f64,
}

impl AffectCausalModel {
    /// Learn causal structure from episode history.
    pub fn learn_structure(&mut self, episodes: &[Episode]) -> Result<()> {
        if episodes.len() < self.min_episodes {
            return Ok(());
        }
        let data = self.extract_variables(episodes);
        let skeleton = self.pc_skeleton(&data)?;
        let dag = self.orient_edges(skeleton)?;
        self.edges = self.estimate_effects(&dag, &data)?;
        self.interventions = self.derive_interventions();
        self.update_appraisal_deltas();
        Ok(())
    }

    /// Counterfactual query: "Would this episode have succeeded
    /// with different affect?"
    pub fn counterfactual(
        &self,
        episode: &Episode,
        hypothetical_pad: &PadVector,
    ) -> CounterfactualResult {
        let noise = self.abduct(episode);
        let cf_outcome = self.propagate_intervention(
            hypothetical_pad, &noise, episode,
        );
        CounterfactualResult {
            episode_id: episode.id.clone(),
            actual_outcome: episode.success,
            counterfactual_pad: hypothetical_pad.clone(),
            counterfactual_outcome: cf_outcome,
            affect_caused: episode.success != cf_outcome,
            causal_strength: self.intervention_effect_size(episode, hypothetical_pad),
        }
    }

    /// Query: what is the optimal PAD state for this task context?
    pub fn optimal_pad(&self, complexity: f64, knowledge: f64) -> PadVector {
        let mut best_pad = PadVector::neutral();
        let mut best_expected = f64::NEG_INFINITY;

        for p in (-10..=10).map(|i| i as f64 * 0.1) {
            for a in (-10..=10).map(|i| i as f64 * 0.1) {
                for d in (-10..=10).map(|i| i as f64 * 0.1) {
                    let pad = PadVector { pleasure: p, arousal: a, dominance: d };
                    let expected = self.expected_outcome_under_do(&pad, complexity, knowledge);
                    if expected > best_expected {
                        best_expected = expected;
                        best_pad = pad;
                    }
                }
            }
        }
        best_pad
    }
}
```

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_pc_algorithm_recovers_structure` | PC algorithm recovers known causal structure from synthetic data | Unit |
| `test_causal_distinguishes_direction` | Distinguishes A->O from O->A (arousal causing failure vs. failure causing arousal) | Unit |
| `test_counterfactual_identifies_affect_failures` | Counterfactual queries identify at least 1 affect-caused failure per 50 episodes | Integration |
| `test_learned_deltas_converge` | Learned appraisal deltas converge within 200 episodes | Integration |
| `test_interventions_reduce_failure_rate` | Interventions reduce failure rate by >=5% when activated | Integration |
| `test_structure_learning_performance` | Structure learning completes in <100ms for 500 episodes | Benchmark |

---

## 3. Dreams + Formal Verification

**Verification Conditions from REM Imagination**

### Motivation

Roko's Dreams subsystem generates counterfactual scenarios during REM imagination:
"What if the agent had used a different strategy?" "What happens at the boundary of
this heuristic?" Separately, the safety system has formal verification capabilities
for domain-specific invariants.

The gap: no verification occurs during imagination. REM generates novel strategies
and hypotheses, but they are evaluated only by LLM reasoning. There is no formal
guarantee that imagined strategies satisfy invariants. A dream could produce a
"brilliant" optimization that violates a safety constraint.

### Research Basis

- **Hao, Guan et al. (2024)** "SafeDreamer: Safe Reinforcement Learning with World
  Models," ICLR 2024, arXiv:2307.07176. Integrates Lagrangian safety constraints into
  Dreamer world model imagination rollouts, verifying safety conditions during planning.

- **Lee et al. (2025)** "VeriPlan: Integrating Formal Verification and LLMs into
  End-User Planning," CHI 2025, arXiv:2502.17898. Applies model checking to
  LLM-generated plans using formal model checkers.

- **Hao, Chen, Zhang & Fan (2024)** "Large Language Models Can Solve Real-World
  Planning Rigorously with Formal Verification Tools," NAACL 2025, arXiv:2404.11891.

### Core Idea

During REM imagination, each counterfactual scenario generates verification conditions
(VCs) that must hold if the imagined strategy is correct. These VCs are checked against
the agent's invariant set before the hypothesis enters the staging buffer. Dreams that
violate invariants become AntiKnowledge entries with high confidence, preventing the
agent from ever attempting the unsafe strategy.

```
REM Imagination Phase (extended):

  1. Generate counterfactual scenario (existing)
  2. Extract strategy from scenario
  3. GENERATE VERIFICATION CONDITIONS:
     For each invariant I in agent's invariant set:
       VC_i = wp(strategy, I)           // Weakest precondition
     For each safety constraint C:
       VC_c = not(strategy AND not C)   // Contradiction check
  4. CHECK VCs:
     If all VCs satisfied: stage hypothesis (existing flow)
     If any VC violated:
       Create AntiKnowledge entry: "Strategy X violates invariant I"
       Tag with violation type and severity
       Skip staging, emit DreamVerificationFailure signal
  5. GRADE dream quality:
     quality = (verified_count / total_count) * novelty_score
```

### Algorithm: Dream Verification Pipeline

```
Algorithm: DreamVerificationPipeline

Input:
  scenario: CounterfactualScenario
  invariants: Vec<Invariant>
  constraints: Vec<SafetyConstraint>
  gate_history: Vec<GateVerdict>

Output:
  verdict: DreamVerdict
  vcs: Vec<VerificationCondition>
  anti_knowledge: Option<KnowledgeEntry>

Invariant Types:
  TypeInvariant     -- "function f always returns type T"
  RangeInvariant    -- "value v in [lo, hi] after operation"
  OrderInvariant    -- "operation A must precede operation B"
  MutexInvariant    -- "operations A and B never concurrent"
  MonotonInvariant  -- "metric m never decreases after operation"
  BudgetInvariant   -- "cumulative cost <= budget after sequence"

Steps:
  1. EXTRACT STRATEGY from scenario
  2. GENERATE VCs via weakest precondition calculus (Dijkstra 1976)
  3. CHECK TYPE INVARIANTS (compile-time analog)
  4. CHECK RANGE INVARIANTS (symbolic execution)
  5. CHECK ORDER INVARIANTS (temporal logic: AG(A -> AF B))
  6. CHECK BUDGET INVARIANTS (cumulative cost summation)
  7. AGGREGATE VERDICT:
     violated = vcs.iter().filter(|vc| !vc.holds).collect()
     if violated.is_empty(): verdict = Verified
     elif violated.iter().any(|v| v.invariant.severity == Critical):
       verdict = Violated; create AntiKnowledge
     else: verdict = Inconclusive
  8. EMIT SIGNALS
```

### Rust Sketch

```rust
/// An invariant that must hold before, during, or after a strategy.
#[derive(Debug, Clone)]
pub struct Invariant {
    pub id: String,
    pub name: String,
    pub kind: InvariantKind,
    pub severity: Severity,
    pub condition: InvariantCondition,
}

#[derive(Debug, Clone)]
pub enum InvariantKind {
    Type { expected: String },
    Range { variable: String, lo: f64, hi: f64 },
    Order { before: String, after: String },
    Mutex { ops: Vec<String> },
    Monotonic { metric: String },
    Budget { resource: String, limit: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity { Critical, Warning, Info }

/// A verification condition generated from a dream scenario.
#[derive(Debug, Clone)]
pub struct VerificationCondition {
    pub invariant: Invariant,
    pub precondition_met: bool,
    pub weakest_precondition: String,
    pub holds: bool,
    pub counterexample: Option<String>,
    pub check_duration_ms: u64,
}

/// Result of verifying a dream scenario against invariants.
#[derive(Debug, Clone)]
pub struct DreamVerdict {
    pub scenario_id: String,
    pub verdict: DreamVerdictKind,
    pub conditions: Vec<VerificationCondition>,
    pub anti_knowledge: Option<KnowledgeEntry>,
    pub quality_score: f64,
    pub total_check_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DreamVerdictKind {
    Verified,       // All invariants hold
    Violated,       // Critical invariant violated -> AntiKnowledge
    Inconclusive,   // Non-critical violations, may still stage
}
```

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_known_bad_strategy_violated` | Budget overflow strategy produces Violated verdict | Unit |
| `test_known_good_strategy_verified` | All-invariants-hold strategy produces Verified verdict | Unit |
| `test_anti_knowledge_prevents_reproposal` | AntiKnowledge from critical violation prevents re-proposal | Integration |
| `test_verification_overhead` | Verification adds <50ms per scenario for <20 invariants | Benchmark |
| `test_dream_quality_correlates` | Quality score correlates with staging-to-promotion rate (r > 0.5) | Integration |
| `test_invariants_from_gate_history` | Invariants learned from gate history match manual specs (>=80%) | Integration |

---

## 4. Morphogenesis + Knowledge

**Knowledge Concentration Gradients Drive Agent Specialization**

### Motivation

Roko's coordination system implements Turing reaction-diffusion for agent role
specialization (Gierer-Meinhardt dynamics, alpha=0.05, beta=0.15, mu=0.01). But
the activation and inhibition signals are based on task returns -- crude success/failure
metrics.

Meanwhile, Neuro maintains rich knowledge with six types, four validation tiers, HDC
vectors, and Ebbinghaus decay. This knowledge is not connected to the morphogenetic
process. Agents specialize based on whether they succeed at tasks, not based on what
they know.

The insight: knowledge concentration should drive specialization. An agent with deep
Consolidated knowledge about testing should specialize in testing. The knowledge
landscape *is* the morphogenetic field. The net-new object is the resulting worldview
cluster: a stable bundle of co-citing heuristics, claims, and behavioral priors that
can be routed and compared as a unit.

### Research Basis

- **Richardson et al. (2024)** "Learning Spatio-Temporal Patterns with Neural Cellular
  Automata," *PLOS Computational Biology*. Trains NCAs to learn Turing pattern dynamics.

- **Shimizu et al. (2025)** "An Algorithm Applying the Self-Organizing Capabilities of
  a Reaction-Diffusion Model to Control Active Swarm Robots," *Journal of Intelligent
  & Robotic Systems*.

- **Turing (1952)** "The Chemical Basis of Morphogenesis," *Philosophical Transactions
  of the Royal Society B*. The foundational insight: spatial pattern formation via local
  activation and long-range inhibition with diffusion asymmetry.

### Core Idea

Replace task-return-based morphogenetic activation with knowledge concentration vectors.
Each agent's Neuro store is projected into the 8-dimensional strategy space, producing a
"knowledge concentration gradient." High concentration in a dimension drives activation.
Knowledge shared via pheromones drives inhibition.

```
Knowledge Concentration Gradient:

  For each strategy dimension k in [0, 7]:
    concentration[k] = sum(entry.confidence * tier_multiplier * relevance_to_k)
                       for entry in neuro_store
                       where entry.tags intersect dimension_tags[k] != empty

  Morphogenetic update:
    activation[k] = alpha * concentration[k] * s[k]
    inhibition[k] = beta * (pheromone_knowledge[k] / collective_size) * s[k]
    s[k] += activation[k] - inhibition[k] - mu * (s[k] - baseline) + noise

  Key differences from task-return activation:
    - Knowledge concentration is PROACTIVE (what it knows)
    - Task returns are REACTIVE (what worked)
    - Knowledge gradients are STABLE (decay slowly via Ebbinghaus)
    - Task returns are VOLATILE (single failure can flip strategy)
```

### Algorithm: Knowledge-Driven Morphogenesis

```
Algorithm: KnowledgeMorphogenesis

Input:
  neuro: NeuroStore
  strategy: [f64; 8]
  pheromone_field: [f64; 8]
  collective_size: usize
  dimension_tags: [[String]; 8]

Parameters:
  alpha = 0.03       -- activation rate (slower than task-based 0.05)
  beta = 0.12        -- inhibition rate (beta > alpha for Turing instability)
  mu = 0.008         -- decay toward baseline
  sigma = 0.003      -- noise standard deviation
  baseline = 1/8
  tier_weights = {Transient: 0.1, Working: 0.5, Consolidated: 1.0, Persistent: 5.0}

Steps:
  1. COMPUTE KNOWLEDGE CONCENTRATION PER DIMENSION
  2. COMPUTE KNOWLEDGE PHEROMONE GRADIENT
  3. GIERER-MEINHARDT UPDATE (knowledge-driven)
  4. DEPOSIT KNOWLEDGE PHEROMONE (Wisdom kind, 24h half-life)
  5. DETECT NICHE COMPETITION (cosine similarity > 0.8)
```

### Rust Sketch

```rust
/// Knowledge concentration across strategy dimensions.
#[derive(Debug, Clone)]
pub struct KnowledgeConcentration {
    pub values: Vec<f64>,
    pub dimension_tags: Vec<Vec<String>>,
    pub tier_weights: TierWeights,
}

#[derive(Debug, Clone)]
pub struct TierWeights {
    pub transient: f64,     // 0.1
    pub working: f64,       // 0.5
    pub consolidated: f64,  // 1.0
    pub persistent: f64,    // 5.0
}

/// Morphogenetic specialization engine driven by knowledge gradients.
pub struct KnowledgeMorphogenesis {
    pub alpha: f64,
    pub beta: f64,
    pub mu: f64,
    pub sigma: f64,
    pub baseline: f64,
    pub concentration: KnowledgeConcentration,
    rng: SmallRng,
}

impl KnowledgeMorphogenesis {
    /// Coding domain dimension tags (default 8D).
    pub fn coding_dimension_tags() -> Vec<Vec<String>> {
        vec![
            vec!["refactoring", "cleanup", "restructure", "rename"],
            vec!["feature", "implementation", "api", "endpoint"],
            vec!["testing", "test", "assertion", "coverage", "mock"],
            vec!["documentation", "docs", "readme", "comment"],
            vec!["performance", "optimization", "benchmark", "latency"],
            vec!["security", "auth", "validation", "sanitize"],
            vec!["dependency", "upgrade", "version", "package"],
            vec!["architecture", "design", "pattern", "abstraction"],
        ]
    }

    /// One morphogenetic update step.
    pub fn update(
        &mut self,
        strategy: &mut [f64],
        local_concentration: &[f64],
        collective_pheromone: &[f64],
        collective_size: usize,
    ) {
        let d = strategy.len();
        let coll_avg_divisor = collective_size.max(1) as f64;

        for k in 0..d {
            let gradient = local_concentration[k]
                - collective_pheromone[k] / coll_avg_divisor;
            let activation = self.alpha * gradient.max(0.0) * strategy[k];
            let inhibition = self.beta * (-gradient).max(0.0) * strategy[k];
            let decay = self.mu * (strategy[k] - self.baseline);
            let noise = Normal::new(0.0, self.sigma)
                .expect("valid sigma").sample(&mut self.rng);
            strategy[k] += activation - inhibition - decay + noise;
            strategy[k] = strategy[k].max(0.001);
        }

        // Normalize to sum to 1.0
        let sum: f64 = strategy.iter().sum();
        if sum > 0.0 {
            for s in strategy.iter_mut() { *s /= sum; }
        }
    }
}
```

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_deep_knowledge_specializes` | Agent with deep testing knowledge specializes in testing (>3x baseline) | Unit |
| `test_identical_knowledge_differentiates` | Two agents with identical knowledge differentiate via inhibition within 100 ticks | Unit |
| `test_tier_multipliers_dominate` | Consolidated knowledge dominates Transient | Unit |
| `test_noise_breaks_symmetry` | Noise injection breaks initial symmetry within 50 ticks | Unit |
| `test_specialization_converges` | Specialization index monotonically increases for first 200 ticks | Integration |
| `test_niche_competition_detected` | Niche competition detected when cosine similarity > 0.8 | Unit |

---

## 5. Bandits + Pheromones

**Pheromone Trails as Bandit Arms**

### Motivation

Roko has two parallel selection mechanisms for explore/exploit:

- **Bandits** (UCB1, LinUCB, Thompson Sampling): Per-agent, internal. No inter-agent
  learning. Cold starts everywhere.
- **Pheromones** (stigmergy): Inter-agent, external. Rich spatial structure. But no
  formal optimality guarantees.

The synthesis: treat each pheromone trail as a bandit arm. Intensity encodes accumulated
reward. Exploration bonus comes from trail age. New agents inherit the collective's
exploration history via the pheromone field -- no cold start.

### Research Basis

- **Chari et al. (2025)** "Pheromone-based Learning of Optimal Reasoning Paths"
  (ACO-ToT), arXiv:2501.19278. Pheromone reinforcement outperforms standard
  chain-of-thought on GSM8K, ARC, MATH.

- **Li, Zhu et al. (2024)** "PooL: Pheromone-inspired Communication Framework for
  Large-Scale MARL," arXiv:2202.09722.

- **Dorigo & Stutzle (2004)** *Ant Colony Optimization*. The foundational framework.

- **Auer, Cesa-Bianchi & Fischer (2002)** "Finite-time Analysis of the Multiarmed
  Bandit Problem," *Machine Learning* 47(2-3). UCB1 achieves O(sqrt(KT ln T)) regret.

### Core Idea

Define a PheromoneArm that wraps a pheromone trail as a bandit arm:

```
Pheromone-Bandit Correspondence:

  Bandit Concept          Pheromone Analog
  ---------------         ----------------
  Arm                     Pheromone trail (kind + scope + location)
  Estimated reward        Trail intensity * depositor reputation
  Pull count              Number of reinforcements
  Exploration bonus       C * sqrt(ln(total_pulls) / trail_pulls)
  Reward update           Deposit (success) or anti-deposit (failure)
  Arm creation            New trail deposited by any agent
  Arm elimination         Trail intensity decays below threshold
```

### Rust Sketch

```rust
/// A pheromone trail wrapped as a bandit arm.
#[derive(Debug, Clone)]
pub struct PheromoneArm {
    pub trail_id: ContentHash,
    pub intensity: f64,
    pub reinforcement_count: u32,
    pub depositor_reputation: f64,
    pub kind: PheromoneKind,
    pub last_reinforced_ms: i64,
}

impl PheromoneArm {
    pub fn estimated_reward(&self, now_ms: i64) -> f64 {
        let age_hours = (now_ms - self.last_reinforced_ms) as f64 / 3_600_000.0;
        let age_penalty = 1.0 / (1.0 + age_hours * 0.1);
        self.intensity * self.depositor_reputation * age_penalty
    }
}

/// Stigmergic bandit: combines UCB1 exploration with pheromone-encoded rewards.
pub struct StigmergicBandit {
    pub local_rewards: HashMap<ContentHash, f64>,
    pub local_pulls: HashMap<ContentHash, u32>,
    pub exploration_constant: f64,  // default sqrt(2)
    pub local_weight: f64,          // default 0.7
    pub epsilon: f64,               // default 0.05
    pub total_pulls: u64,
}

impl StigmergicBandit {
    /// Select which pheromone trail to follow.
    pub fn select(&mut self, field: &PheromoneField, now_ms: i64) -> TrailSelection {
        let arms: Vec<PheromoneArm> = field.active_trails()
            .map(|trail| PheromoneArm { /* ... */ })
            .collect();

        if arms.is_empty() || self.should_explore() {
            return TrailSelection::Explore;
        }

        let best = arms.iter()
            .max_by(|a, b| self.ucb_score(a, now_ms)
                .partial_cmp(&self.ucb_score(b, now_ms))
                .unwrap_or(std::cmp::Ordering::Equal))
            .expect("arms non-empty");

        self.total_pulls += 1;
        TrailSelection::Follow(best.trail_id)
    }

    fn ucb_score(&self, arm: &PheromoneArm, now_ms: i64) -> f64 {
        let pheromone_reward = arm.estimated_reward(now_ms);
        let local_reward = self.local_rewards
            .get(&arm.trail_id).copied()
            .unwrap_or(pheromone_reward * 0.5);
        let blended = self.local_weight * local_reward
            + (1.0 - self.local_weight) * pheromone_reward;
        let local_count = self.local_pulls
            .get(&arm.trail_id).copied().unwrap_or(0) as f64;
        let exploration = self.exploration_constant
            * ((self.total_pulls as f64 + 1.0).ln() / (local_count + 1.0)).sqrt();
        blended + exploration
    }

    /// Initialize from pheromone field (cold start elimination).
    pub fn warm_start(&mut self, field: &PheromoneField, now_ms: i64) {
        for trail in field.active_trails() {
            self.local_rewards
                .entry(trail.content_hash())
                .or_insert(trail.current_intensity(now_ms) * 0.5);
        }
    }
}
```

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_cold_start_inheritance` | New agent inherits reward estimates from 10 existing trails within 1 tick | Unit |
| `test_ucb_exploration_decreases` | UCB exploration bonus decreases as trail is reinforced | Unit |
| `test_anti_pheromone_doubles_decay` | Anti-pheromone causes trail intensity to decay 2x faster | Unit |
| `test_lower_cumulative_regret` | Stigmergic bandit achieves >=20% lower regret than isolated UCB1 | Integration |
| `test_intensity_reward_correlation` | Trail intensity and bandit estimated reward correlate (r > 0.8) | Unit |
| `test_dead_trail_pruning` | Trails with intensity < 0.01 are pruned from arm set | Unit |

---

## 6. Witness DAG + Active Inference

**Agent History IS Its World Model**

### Motivation

Roko's Witness DAG records cryptographic commitments of every cognitive event:
Observations, Predictions, Decisions, Resolutions. It is currently used for forensic
analysis. But the DAG contains far more information than is being extracted.

The insight from active inference: an agent's generative model is precisely the structure
that predicts future observations from past actions. The Witness DAG already contains
this structure -- the edges from Observations to Predictions to Decisions to Resolutions
trace exactly the causal chains the agent believes in. The DAG does not just record
history; it *is* the world model.

### Research Basis

- **Richens & Everitt (2024)** "Robust Agents Learn Causal World Models," ICLR 2024
  (Oral), arXiv:2402.10877. Proves that any agent achieving bounded regret under
  distributional shifts must have learned an approximate causal model.

- **Gkountouras et al. (2024)** "Language Agents Meet Causality -- Bridging LLMs and
  Causal World Models," arXiv:2410.19923.

- **Deng et al. (2025)** "A Roadmap Towards Improving Multi-Agent RL with Causal
  Discovery and Inference," arXiv:2503.17803.

- **Conant & Ashby (1970)** "Every Good Regulator of a System Must Be a Model of
  That System." The Good Regulator Theorem.

### Core Idea

Extract a causal world model from the Witness DAG by analyzing statistical regularities
in the O->P->D->R chains. Prediction vertices that consistently predict correctly create
strong causal edges. Predictions that fail create weak edges or reveal confounders. The
resulting causal DAG becomes the agent's generative model for active inference.

```
Witness DAG -> Causal World Model extraction:

  1. Collect all Prediction vertices with their Resolution outcomes
  2. Group by prediction type
  3. For each prediction type:
     a. Extract parent Observations (what informed the prediction)
     b. Extract child Resolution (what actually happened)
     c. Compute: accuracy = correct_resolutions / total_predictions
     d. Create causal edge: Observation_type -> Outcome_type
        weight = accuracy * num_observations
  4. Prune edges with accuracy < 0.3 (likely spurious)
  5. Result: Causal DAG where edges represent learned causal beliefs

Active Inference using the model:
  Expected Free Energy of action a:
    G(a) = -sum_o P(o|a, model) * [ln P(o|a, model) - ln P(o|desired)]
```

### Rust Sketch

```rust
/// A causal edge in the world model, extracted from Witness DAG statistics.
#[derive(Debug, Clone)]
pub struct WorldModelEdge {
    pub from: String,
    pub to: String,
    pub weight: f64,
    pub accuracy: f64,
    pub sample_size: usize,
    pub updated_ms: i64,
}

/// Causal world model extracted from the Witness DAG.
pub struct CausalWorldModel {
    pub edges: HashMap<String, Vec<WorldModelEdge>>,
    pub calibration: HashMap<String, f64>,
    pub age_ticks: u64,
    pub rebuild_interval: u64,
}

impl CausalWorldModel {
    /// Extract causal world model from Witness DAG.
    pub fn from_witness_dag(
        dag: &WitnessDAG,
        min_observations: usize,
        accuracy_threshold: f64,
    ) -> Self { /* ... */ }

    /// Compute Expected Free Energy for a candidate action.
    pub fn expected_free_energy(
        &self,
        observations: &[String],
        action: &str,
    ) -> f64 {
        let mut outcome_probs: HashMap<String, f64> = HashMap::new();
        let mut total_weight = 0.0;

        for obs in observations {
            if let Some(edges) = self.edges.get(obs) {
                for edge in edges {
                    *outcome_probs.entry(edge.to.clone()).or_insert(0.0) += edge.weight;
                    total_weight += edge.weight;
                }
            }
        }

        if total_weight == 0.0 { return 0.0; }

        for prob in outcome_probs.values_mut() { *prob /= total_weight; }

        let pragmatic: f64 = outcome_probs.iter()
            .map(|(outcome, prob)| prob * self.utility_of(outcome)).sum();
        let epistemic: f64 = outcome_probs.values()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.ln()).sum();

        -(pragmatic + epistemic)
    }

    /// Detect when the world model is stale.
    pub fn is_stale(&self, recent_predictions: &[(String, bool)]) -> bool {
        for (pred_type, correct) in recent_predictions {
            if let Some(&historical_accuracy) = self.calibration.get(pred_type) {
                let recent_accuracy = if *correct { 1.0 } else { 0.0 };
                if (recent_accuracy - historical_accuracy).abs() > 0.2 {
                    return true;
                }
            }
        }
        false
    }
}
```

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_extracts_compilation_edge` | Correctly extracts "compilation error -> test failure" causal edge | Unit |
| `test_confounder_detection` | Removes spurious correlation when true cause is identified | Unit |
| `test_efe_selects_informative` | EFE selects informative actions when model is uncertain | Unit |
| `test_staleness_detection` | Staleness detection fires within 20 predictions when environment changes | Unit |
| `test_model_reconstruction_performance` | Model reconstruction < 50ms for DAG with 10,000 vertices | Benchmark |
| `test_calibration_convergence` | Calibration accuracy improves monotonically over 1,000 predictions | Integration |

---

## 7. Somatic Markers + Code Intelligence

**Code Smells Trigger Somatic Markers**

### Motivation

Roko's Daimon has somatic markers (Damasio 1994) -- fast pre-analytical gut feelings
stored in a k-d tree over the 8-dimensional strategy space. The code intelligence
system (roko-index) computes structural metrics: PageRank importance, HDC fingerprints,
dependency depth, cyclomatic complexity.

The gap: code metrics exist as cold numbers. Somatic markers exist as emotional
heuristics. They are not connected. When an agent encounters code with high cyclomatic
complexity, deep dependency chains, and low test coverage, it *should* feel uneasy --
a somatic marker should fire saying "this is dangerous territory, use Conservative
strategy." But currently the agent treats all code regions equally until a gate fails.

The insight from neuroscience: somatic markers are faster than analysis. Damasio showed
that patients with ventromedial prefrontal cortex damage can still reason about risks
analytically but cannot feel danger -- and they make catastrophically bad decisions
because analytical reasoning is too slow for real-time choice.

### Research Basis

- **Fakhoury et al. (2024)** "EEG as a Potential Ground Truth for Cognitive State in
  Software Development Activities," *PLOS ONE*. Validates that developers' neural
  signals during code comprehension reliably predict cognitive difficulty.

- **Pargaonkar et al. (2024)** "Quality Evaluation of Modern Code Reviews Through
  Intelligent Biometric Program Comprehension." AI predicts review quality from
  biomarkers with 87.77% accuracy.

- **Kaur et al. (2025)** "Towards Decoding Developer Cognition in the Age of AI
  Assistants," arXiv:2501.02684.

- **Damasio (1994)** *Descartes' Error: Emotion, Reason, and the Human Brain*.

### Core Idea

Map code intelligence metrics onto the Daimon's 8-dimensional strategy space and
create automatic somatic markers from historical gate results:

```
Code Region -> Strategy Space Mapping:

  Dimension 0 (Complexity):    cyclomatic_complexity / max_complexity
  Dimension 1 (Risk):          (1 - test_coverage) * reverse_dep_count / max_deps
  Dimension 2 (Novelty):       1 - max(hdc_similarity(region, known_patterns))
  Dimension 3 (Confidence):    agent's Daimon dominance (current PAD.D)
  Dimension 4 (Time Pressure): deadline_proximity * blocker_count
  Dimension 5 (Scope):         files_modified * lines_changed / max_scope
  Dimension 6 (Reversibility): is_additive ? 0.8 : 0.2
  Dimension 7 (Dep Depth):     transitive_dep_count / max_transitive

Somatic Query (before starting work on code region):
  coords = compute_strategy_coords(code_region_metrics)
  nearby = somatic_landscape.nearest(coords, k=5, radius=0.5)
  avg_valence = nearby.iter().map(|m| m.valence).mean()

  If avg_valence < -0.5: Bias toward Conservative + T2 (danger zone)
  If avg_valence > +0.5: Bias toward Exploratory + T0/T1 (safe territory)
```

### Rust Sketch

```rust
/// Code region metrics projected into the somatic marker strategy space.
#[derive(Debug, Clone)]
pub struct CodeRegionMetrics {
    pub complexity: f64,
    pub risk: f64,
    pub novelty: f64,
    pub confidence: f64,
    pub time_pressure: f64,
    pub scope: f64,
    pub reversibility: f64,
    pub dependency_depth: f64,
}

impl CodeRegionMetrics {
    pub fn to_coords(&self) -> [f64; 8] {
        [self.complexity, self.risk, self.novelty, self.confidence,
         self.time_pressure, self.scope, self.reversibility, self.dependency_depth]
    }
}

/// Somatic marker bias computed from code region metrics.
#[derive(Debug, Clone)]
pub struct CodeSomaticBias {
    pub strategy: DispatchStrategy,
    pub tier: InferenceTier,
    pub warnings: Vec<String>,
    pub avg_valence: f64,
    pub marker_count: usize,
    pub query_time_us: u64,
}

/// Computes somatic bias for a code region before the agent begins work.
pub struct CodeSomaticEngine {
    pub max_complexity: f64,      // 50.0
    pub max_rev_deps: f64,        // 50.0
    pub max_trans_deps: f64,      // 100.0
    pub max_scope: f64,           // 1000.0
    pub query_radius: f64,        // 0.5
    pub query_k: usize,           // 10
}

impl CodeSomaticEngine {
    pub fn query_bias(
        &self,
        metrics: &CodeRegionMetrics,
        landscape: &SomaticLandscape,
    ) -> CodeSomaticBias {
        let start = std::time::Instant::now();
        let coords = metrics.to_coords();
        let markers = landscape.nearest_within(&coords, self.query_radius, self.query_k);

        if markers.is_empty() {
            return CodeSomaticBias {
                strategy: DispatchStrategy::Balanced,
                tier: InferenceTier::T1,
                warnings: vec!["No prior experience with similar code metrics".into()],
                avg_valence: 0.0,
                marker_count: 0,
                query_time_us: start.elapsed().as_micros() as u64,
            };
        }

        let total_intensity: f64 = markers.iter().map(|m| m.intensity).sum();
        let avg_valence: f64 = markers.iter()
            .map(|m| m.valence * m.intensity).sum::<f64>() / total_intensity;

        let mut warnings = Vec::new();
        let (strategy, tier) = if avg_valence < -0.5 {
            warnings.push(format!("Somatic warning: {} similar regions averaged {:.2} valence",
                markers.len(), avg_valence));
            (DispatchStrategy::Conservative, InferenceTier::T2)
        } else if avg_valence < -0.2 {
            warnings.push("Somatic caution: mixed outcomes in similar regions".into());
            (DispatchStrategy::Balanced, InferenceTier::T1)
        } else if avg_valence > 0.5 {
            (DispatchStrategy::Exploratory, InferenceTier::T0)
        } else {
            (DispatchStrategy::Balanced, InferenceTier::T1)
        };

        if metrics.risk > 0.7 {
            warnings.push("High risk: many reverse dependencies with low coverage".into());
        }
        if metrics.novelty > 0.8 {
            warnings.push("Novel code pattern: no similar patterns in knowledge base".into());
        }

        CodeSomaticBias {
            strategy, tier, warnings, avg_valence,
            marker_count: markers.len(),
            query_time_us: start.elapsed().as_micros() as u64,
        }
    }
}
```

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_high_complexity_low_coverage_conservative` | Code with high complexity + low coverage triggers Conservative | Unit |
| `test_prior_success_triggers_exploratory` | Code with prior successful markers triggers Exploratory | Unit |
| `test_somatic_query_performance` | Somatic query < 100us for landscape with 10,000 markers | Benchmark |
| `test_gate_failure_negative_marker` | Gate failure produces negative valence marker | Unit |
| `test_novel_code_warning` | Novel code (no HDC similarity) produces warning | Unit |
| `test_pagerank_risk_increases_tier` | High-importance symbols get more scrutiny | Unit |

---

## 8. Token Economy + Dream Quality

**Pay for Dreams with Tokens, Better Dreams Earn More**

### Motivation

Roko's dream cycle consumes real resources: NREM replay uses Haiku (~$0.001/episode),
REM imagination uses Sonnet (~$0.01/counterfactual). Dream spending is currently
unregulated -- there is no feedback loop between dream quality and dream budget.

Dreams should be a market good. High-quality dreams (verified hypotheses that promote
to Consolidated knowledge) should earn more dream budget. Low-quality dreams should
cost budget without return. This creates a self-regulating system: agents that dream
well earn the right to dream more.

### Research Basis

- **Mantiuk, Becker & Wu (2025)** "From Curiosity to Competence: How World Models
  Interact with the Dynamics of Exploration," arXiv:2507.08210.

- **"INTUITOR" (2025)** "Learning to Reason without External Rewards,"
  arXiv:2505.19590. Uses model self-certainty as intrinsic reward.

- **Burda et al. / DreamerV3-XP (2025)** "Optimizing Exploration Through Uncertainty
  Estimation," arXiv:2510.21418.

- **Lin et al. (2025)** "Scaling LLM Test-Time Compute Optimally Can be More Effective
  than Scaling Model Parameters."

### Core Idea

Introduce a DreamBudget that starts at a base allocation and grows or shrinks based on
dream outcomes:

```
Dream Economy:

  DreamBudget(t+1) = DreamBudget(t) * (1 + ROI(t) - depreciation)

  ROI(t) = earned(t) / spent(t)

  earned(t) = sum(tier_value * validation_count)
              for each hypothesis promoted from dream cycle t

  spent(t) = sum(tokens * price_per_token)
             for all model calls in dream cycle t

  tier_value = {Transient: 0.01, Working: 0.10, Consolidated: 1.00, Persistent: 10.00}

  depreciation = 0.05 per cycle (prevents unbounded growth)

  Constraints:
    DreamBudget in [min_budget, max_budget]
    min_budget = $0.01 (always allow micro-consolidation)
    max_budget = $1.00 (prevent runaway dream spending)
```

### Algorithm: Dream Token Economy

```
Algorithm: DreamTokenEconomy

Input:
  budget: DreamBudget
  dream_history: Vec<DreamCycleReport>
  knowledge_promotions: Vec<Promotion>

Parameters:
  min_budget: f64 = 0.01
  max_budget: f64 = 1.00
  depreciation: f64 = 0.05
  tier_values = {Transient: 0.01, Working: 0.10, Consolidated: 1.00, Persistent: 10.00}
  quality_bonus_threshold: f64 = 0.7
  novelty_multiplier: f64 = 2.0

Steps:
  1. COMPUTE DREAM COST (spent)
  2. COMPUTE DREAM REVENUE (earned) from promoted knowledge
  3. COMPUTE ROI = earned / spent
  4. COMPUTE DREAM QUALITY METRICS:
     quality_score = 0.30 * verification_rate
                   + 0.30 * promotion_rate
                   + 0.20 * novelty_rate
                   + 0.20 * depotentiation_effect
     quality_grade = A (>= 0.75) / B (>= 0.50) / C (>= 0.25) / D
  5. UPDATE BUDGET: growth_factor = 1.0 + roi - depreciation
  6. ALLOCATE ACROSS PHASES:
     A/B: {nrem: 0.30, rem: 0.60, integration: 0.10}
     C:   {nrem: 0.50, rem: 0.40, integration: 0.10}
     D:   {nrem: 0.70, rem: 0.20, integration: 0.10}
  7. EMIT DreamEconomyEvent
```

### Rust Sketch

```rust
/// Dream budget with quality-driven growth dynamics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamBudget {
    pub current_usd: f64,
    pub min_usd: f64,
    pub max_usd: f64,
    pub depreciation: f64,
    pub roi_history: VecDeque<f64>,
    pub quality_history: VecDeque<char>,
    pub total_spent: f64,
    pub total_earned: f64,
    pub cycle_count: u64,
}

/// Value assigned to each knowledge tier for dream revenue calculation.
#[derive(Debug, Clone)]
pub struct TierValues {
    pub transient: f64,     // 0.01
    pub working: f64,       // 0.10
    pub consolidated: f64,  // 1.00
    pub persistent: f64,    // 10.00
}

/// How to allocate budget across dream phases.
#[derive(Debug, Clone)]
pub struct DreamAllocation {
    pub nrem: f64,
    pub rem: f64,
    pub integration: f64,
}

/// Dream quality metrics for a single cycle.
#[derive(Debug, Clone)]
pub struct DreamQualityMetrics {
    pub verification_rate: f64,
    pub promotion_rate: f64,
    pub novelty_rate: f64,
    pub depotentiation_effect: f64,
    pub quality_score: f64,
    pub quality_grade: char,
}

/// Economy event logged per dream cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamEconomyEvent {
    pub cycle: u64,
    pub spent_usd: f64,
    pub earned_usd: f64,
    pub roi: f64,
    pub quality_score: f64,
    pub quality_grade: char,
    pub budget_after: f64,
    pub allocation: DreamAllocation,
    pub timestamp_ms: i64,
}

impl DreamBudget {
    pub fn new(initial_usd: f64) -> Self {
        Self {
            current_usd: initial_usd,
            min_usd: 0.01,
            max_usd: 1.00,
            depreciation: 0.05,
            roi_history: VecDeque::with_capacity(100),
            quality_history: VecDeque::with_capacity(100),
            total_spent: 0.0,
            total_earned: 0.0,
            cycle_count: 0,
        }
    }

    /// Update budget after a dream cycle completes.
    pub fn update(
        &mut self,
        report: &DreamCycleReport,
        promotions: &[KnowledgePromotion],
        tier_values: &TierValues,
    ) -> DreamEconomyEvent {
        let spent = report.total_cost_usd();
        self.total_spent += spent;

        let earned: f64 = promotions.iter().map(|p| {
            let base = match p.tier {
                Tier::Transient => tier_values.transient,
                Tier::Working => tier_values.working,
                Tier::Consolidated => tier_values.consolidated,
                Tier::Persistent => tier_values.persistent,
            };
            let novelty = if p.is_novel { 2.0 } else { 1.0 };
            base * novelty + p.validation_count as f64 * 0.01
        }).sum();
        self.total_earned += earned;

        let roi = if spent > 0.0 { earned / spent } else { 0.0 };
        self.roi_history.push_back(roi);

        let quality = self.compute_quality(report);

        let mut growth = 1.0 + roi - self.depreciation;
        if quality.quality_score > 0.7 { growth += 0.10; }
        self.current_usd = (self.current_usd * growth).clamp(self.min_usd, self.max_usd);

        let allocation = self.compute_allocation(quality.quality_grade);
        self.cycle_count += 1;

        DreamEconomyEvent {
            cycle: self.cycle_count,
            spent_usd: spent,
            earned_usd: earned,
            roi,
            quality_score: quality.quality_score,
            quality_grade: quality.quality_grade,
            budget_after: self.current_usd,
            allocation,
            timestamp_ms: now_ms(),
        }
    }

    pub fn can_afford(&self, estimated_cost: f64) -> bool {
        estimated_cost <= self.current_usd
    }

    pub fn lifetime_roi(&self) -> f64 {
        if self.total_spent > 0.0 { self.total_earned / self.total_spent } else { 0.0 }
    }
}
```

### Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_budget_increases_on_consolidated` | Budget increases when dream produces Consolidated knowledge (ROI > 1.0) | Unit |
| `test_budget_decreases_no_promotions` | Budget decreases when dream produces no promotions | Unit |
| `test_budget_floor_enforced` | Budget never drops below min_usd ($0.01) | Unit |
| `test_budget_ceiling_enforced` | Budget never exceeds max_usd ($1.00) | Unit |
| `test_grade_a_allocates_rem` | Quality grade A allocates 60% to REM; grade D allocates 70% to NREM | Unit |
| `test_can_afford_prevents_expensive` | `can_afford()` prevents expensive cycles on depleted budget | Unit |
| `test_lifetime_roi_positive` | Lifetime ROI > 1.0 indicates dreams are net positive | Integration |
| `test_phase_allocation_sums_to_one` | Phase allocation sums to 1.0 for all quality grades | Unit |

---

## Cross-Innovation Interactions

These eight innovations form a reinforcing network:

```
   +-------------------------------------------------------------+
   |                                                               |
   |  [1] HDC Beliefs <---------> [6] Witness World Model         |
   |       |                             |                         |
   |       | beliefs encode              | DAG validates           |
   |       | world model                 | beliefs                 |
   |       v                             v                         |
   |  [2] Affect Causal <-------> [7] Code Somatic                |
   |       |                             |                         |
   |       | causal structure            | code metrics ->         |
   |       | of emotions                 | somatic markers         |
   |       v                             v                         |
   |  [3] Dream Verify <--------> [8] Dream Economy               |
   |       |                             |                         |
   |       | verification               | quality grades           |
   |       | conditions                  | budget allocation        |
   |       v                             v                         |
   |  [4] Knowledge Morph <------> [5] Stigmergic Bandits         |
   |       |                             |                         |
   |       | knowledge drives            | pheromones encode        |
   |       | specialization              | collective rewards       |
   |       +-----------------------------+                         |
   |                                                               |
   +-------------------------------------------------------------+
```

**Key feedback loops:**

1. **HDC Beliefs [1] <-> Witness World Model [6]**: HDC vectors provide a compact
   belief state; the Witness DAG provides the causal structure that validates and
   refines those beliefs. Use HDC for fast (~8us) per-tick inference, Witness model
   for deeper (~50ms) reflection.

2. **Affect Causal [2] <-> Code Somatic [7]**: Causal discovery learns which PAD
   states cause failures; code somatic markers encode these discoveries as fast-path
   heuristics. Causal model provides the *why*; somatic markers provide the *speed*.

3. **Dream Verify [3] <-> Dream Economy [8]**: Verification conditions determine
   dream quality; dream quality determines budget allocation. Verified dreams earn
   more budget for future REM imagination. This prevents low-quality dreaming from
   consuming resources.

4. **Knowledge Morphogenesis [4] <-> Stigmergic Bandits [5]**: Knowledge concentration
   drives agent specialization via reaction-diffusion; pheromone trails encode the
   collective's exploration history as bandit arms. Specialized agents deposit stronger
   pheromones in their niche, reinforcing the specialization gradient.

The full interaction set is what makes the moat durable: each subsystem improves the
others' signals, routing, persistence, and calibration. That architectural coherence
is expensive to copy because it depends on aligned kernel choices, not just feature
parity.

---

## Implementation Priority

| Priority | Innovation | Dependencies | Est. Effort |
|---|---|---|---|
| P0 | [1] HDC + Active Inference | `roko-primitives` HDC ops | Medium -- replace scalar PE with HDC PE |
| P0 | [7] Code Somatic Markers | `roko-index`, `roko-daimon` | Medium -- wire existing subsystems |
| P1 | [5] Stigmergic Bandits | `roko-learn` bandits, coordination pheromones | Medium -- compose existing |
| P1 | [8] Dream Economy | `roko-learn` costs, `roko-dreams` | Small -- accounting + gating |
| P2 | [4] Knowledge Morphogenesis | `roko-neuro`, coordination | Medium -- replace activation signal |
| P2 | [6] Witness World Model | `roko-core` Witness DAG | Large -- causal extraction engine |
| P3 | [2] Affect Causal Discovery | `roko-daimon`, `roko-learn` | Large -- PC algorithm + do-calculus |
| P3 | [3] Dream Verification | `roko-gate`, `roko-dreams` | Large -- invariant specification + checking |

---

## Cross-References

- [HDC fingerprinting and belief state](./vision-and-thesis.md) -- Kernel primitives
- [Cognitive immune system](./cognitive-immune-system.md) -- Safety integration for dream verification
- [Emergent goal structures](./emergent-goal-structures.md) -- GoalTree as goal emergence engine
- [Cognitive energy model](./cognitive-energy-model.md) -- Energy constraints on dream budgets
- [Attention as currency](./attention-as-currency.md) -- Token economics underlying dream economy
- [Synergy integration map](./synergy-integration-map.md) -- Ten load-bearing primitives
- [Temporal knowledge topology](./temporal-knowledge-topology.md) -- Allen algebra in causal chains
