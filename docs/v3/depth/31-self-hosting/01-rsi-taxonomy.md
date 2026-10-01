# 31-01 -- RSI Taxonomy and Loop Closure Spectrum

> **Parent:** [31-SELF-HOSTING.md](../../31-SELF-HOSTING.md) section 2
> **Primary source:** RSI Survey (Chen et al. 2026, arXiv:2607.07663)
> **Additional sources:** Schmidhuber 2003 (Godel Machine), Nivel et al. 2013
> (AERA), Argyris & Schon 1978 (double/triple-loop learning)

---

## 1. What Is Recursive Self-Improvement?

Recursive self-improvement (RSI) is the capacity of a system to modify its own
processes in ways that increase its performance on future tasks, where the
improved system can then further improve itself. The "recursive" qualifier
distinguishes RSI from simple learning: a system that improves its weights via
gradient descent is learning; a system that improves its *learning algorithm*
is recursively self-improving.

The RSI Survey (arXiv:2607.07663) provides the first comprehensive taxonomy of
RSI in AI systems, identifying two orthogonal classification axes: **what is
improved** and **degree of loop closure**.

---

## 2. What Is Improved: Four Targets

The survey identifies four targets of self-improvement, ordered by increasing
scope, risk, and difficulty:

### 2.1 Deployment Behavior

Changes to how the system operates at inference time without modifying
underlying parameters or structure. Examples: prompt engineering, tool
selection, context assembly strategy, output formatting.

**Roko implementation:**
- Playbook rules inject learned lessons into agent prompts
- Cascade router selects models based on empirical pass rates
- Section effectiveness tracking adjusts prompt weights (Loop 3)
- Skill library provides reusable capabilities

This is the safest level because the system's structure is unchanged -- only
the inputs to the fixed pipeline vary. All of Roko's fully automated learning
loops operate at this level.

### 2.2 Training Policy

Changes to how the system learns from experience -- adjusting learning rates,
reward signals, data curation, or feedback dynamics.

**Roko implementation:**
- Adaptive gate thresholds: EMA-based adjustment of pass/fail boundaries
  within a configured floor (default 0.30)
- Confidence dynamics: asymmetric validation/contradiction rates (+0.05/-0.10)
  with 0.95 ceiling
- Efficiency feedback: per-section token cost tracking influences future
  context assembly budgets
- Hindsight adjustments: append-only relabeling of past episodes based on
  new evidence

This level is riskier because changes to the training policy affect *all*
subsequent learning. Roko bounds this risk through floors (threshold floor),
ceilings (confidence ceiling), and velocity limits (max changes per day).

### 2.3 Evaluator

Changes to the verification pipeline itself -- adding, removing, or modifying
gates, changing what counts as "correct."

**Roko implementation (bounded):**
- Adaptive gate thresholds adjust the numerical boundary for each rung via
  EMA feedback, but the gate pipeline structure (which gates exist, their
  ordering, the rung hierarchy) is immutable to the learning system.
- Constitutional constraint: `gates_immutable = true` prevents disabling or
  weakening gates below the threshold floor.
- Gate gaming detection monitors for the pathological case where the system
  learns to game the evaluator.

This is the highest level Roko permits with any automation. Full evaluator
modification (adding new gate types, changing the rung order) requires human
intervention.

### 2.4 Research Process

Changes to the method of searching for improvements -- modifying the
architecture search space, the evaluation methodology, or the meta-learning
strategy itself.

**Roko status:** Not automated. ADAS-style architecture search is specified
as target design but gated behind mandatory human approval (L4 structural
adaptation). R04 delivers bounded meta-agent lifecycle but explicitly does not
implement autonomous architecture search.

---

## 3. Degree of Loop Closure

The second axis measures how much human involvement the improvement loop
requires:

### 3.1 Fully Open (Manual)

Human performs all steps: identifies problem, designs solution, implements
change, evaluates result. This is the traditional software development
workflow, and it is how roko was originally built.

### 3.2 Partially Closed (Human Approves)

The system proposes changes; a human reviews and approves each one. This is
Roko's L4 structural adaptation loop: the system can propose new graph
topologies, cell registrations, or model introductions, but a human must
approve before deployment.

### 3.3 Mostly Closed (Human Monitors)

The system makes changes automatically within bounded parameters; a human
monitors for anomalies. This is Roko's L1-L3 operational loops:

- **L1 (per-tick):** Gate threshold EMA, experiment weights, model temperature
  adjustments within declared `ParamRange` bounds
- **L2 (per-task):** Cascade router selects among pre-approved model
  alternatives
- **L3 (per-session):** Dream consolidation compresses episodes into durable
  knowledge

The human monitors via `roko dashboard`, `roko show learning`, and the
C-Factor governance recommendations.

### 3.4 Fully Closed (No Human)

The system modifies itself without any human involvement. No production Roko
loop operates at this level. The DGM and ADAS architectures describe
fully-closed improvement loops; Roko deliberately does not implement them.

---

## 4. Where Roko Sits: Bounded Self-Refinement

The RSI Survey identifies several classes of self-improving systems. Roko's
operational class is **bounded self-refinement**:

| Property | Value |
|----------|-------|
| Primary improvement target | Deployment behavior (L1-L2) |
| Secondary improvement target | Training policy (L2-L3, bounded) |
| Loop closure | Mostly closed (L1-L3); partially closed (L4) |
| Structural boundary | Fixed graph topology, gate pipeline, cell registry |
| Safety mechanism | Constitutional constraints, velocity limits, confidence ceilings |
| Verification | Deterministic external gates (not LLM-as-judge) |

This class is distinguished from:

- **Open-ended RSI** (DGM, ADAS): no structural boundaries, fully closed loop
- **Static optimization** (DSPy): one-shot optimization, no online adaptation
- **Reward hacking** (naive RL): optimizes proxy metric without safety bounds

The bounded self-refinement class provides genuine improvement (measured by the
four key metrics) while maintaining the safety properties required for a
development tool modifying production codebases.

---

## 5. Comparison with Other Systems

| System | Improvement Target | Loop Closure | Verification |
|--------|-------------------|--------------|-------------|
| Roko (operational) | Deployment + Training policy | Mostly closed | Deterministic gates |
| Reflexion (Shinn 2023) | Deployment behavior | Mostly closed | Task-specific env |
| DSPy (Khattab 2024) | Deployment behavior | Static (one-shot) | Test set |
| ADAS (Hu ICLR 2025) | Research process | Fully closed | Benchmark suite |
| DGM (Lange 2025) | Research process | Fully closed | Archive comparison |
| AlphaCode (Li 2022) | Deployment behavior | Partially closed | Unit tests |
| SWE-agent (Yang 2024) | Deployment behavior | Partially closed | Test suite |

---

## References

- Chen, M., Wang, L. & Qu, B. "Recursive Self-Improvement in AI: From Bounded
  Self-Refinement to Autonomous Research Loops." arXiv:2607.07663, July 2026.
- Schmidhuber, J. "Godel Machines: Fully Self-Referential Optimal Universal
  Self-Improvers." 2003.
- Argyris, C. & Schon, D.A. "Organizational Learning: A Theory of Action
  Perspective." Addison-Wesley, 1978.
- Nivel, E. et al. "Bounded Recursive Self-Improvement." Technical Report
  RUTR-SCS13001, Reykjavik University, 2013.
