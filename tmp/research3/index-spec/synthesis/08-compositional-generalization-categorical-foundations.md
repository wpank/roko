# Compositional Generalization and Categorical Foundations

## Why Current AI Systems Cannot Compose, and What Mathematics Guarantees That a Different Architecture Can

---

## 1. The Problem: Why LLMs Hit a Ceiling

Large language models are the most capable AI systems ever built for tasks that resemble their training distribution. They are also, provably, the wrong architecture for tasks that require composing learned primitives into novel combinations. This section presents the empirical evidence and the theoretical proof.

### 1.1 Faith and Fate: The Composition Depth Collapse

Dziri et al. published "Faith and Fate: Limits of Transformers on Compositionality and Reasoning" at NeurIPS 2023 (arXiv:2305.18654, now over 700 citations). The paper's central experiment is disarmingly simple: test GPT-4 on multi-digit multiplication at increasing digit counts. The results are:

- 2x2 digit multiplication: ~90% accuracy (zero-shot)
- 3x3 digit multiplication: ~50% accuracy
- 4x4 digit multiplication: ~4% accuracy
- 5x5 digit multiplication: ~0% accuracy

The decay is not gradual. It is exponential. Each additional layer of composition reduces accuracy by roughly an order of magnitude.

The explanation Dziri et al. provide is precise: transformers perform "linearized subgraph matching against memorized fragments." When the model encounters a problem, it does not execute a composition algorithm. Instead, it searches its training data for similar subgraph patterns and stitches together fragments of memorized solutions. This works when the problem closely resembles training examples. It fails catastrophically when the required composition depth exceeds what was memorized, because the model has no mechanism for chaining computation steps -- it can only recall and recombine previously seen patterns.

This is not a training data problem. It is an architectural one. No amount of additional training data for 4x4 multiplication would teach GPT-4 to compose 3x3 multiplication steps, because the attention mechanism does not implement step-by-step composition -- it implements pattern retrieval.

### 1.2 GSM-Symbolic: Perturbation Collapse

The GSM-Symbolic experiments (Mirzadeh et al., Apple, arXiv:2410.05229, 2024) demonstrated a complementary failure mode. When standard GSM8K math word problems are subjected to minor symbolic perturbations -- changing names, swapping numbers, altering irrelevant surface details while preserving mathematical structure -- LLM accuracy drops by 10-20 percentage points on average.

This result is devastating because it reveals that LLMs are not performing mathematical reasoning at all. They are performing pattern matching against the syntactic surface of problems. When the surface changes, the pattern match fails, even though the underlying mathematical structure is identical. A system that genuinely composed mathematical operations would be invariant to these surface-level perturbations.

### 1.3 ARC-AGI-2: The Frontier Benchmark

The Abstraction and Reasoning Corpus (ARC), created by Francois Chollet, is specifically designed to test compositional generalization -- the ability to learn abstract rules from a few examples and compose them to solve novel problems. ARC-AGI-2 is the current frontier version, deliberately harder than the original.

As of early 2026, the performance landscape on ARC-AGI-2 is:

- Best open-source systems: ~24%
- Claude Opus 4.5 Thinking: 37.6% (at $2.20/task)
- Gemini 3 Pro + Poetiq: 54% (at $30/task)
- Human baseline: ~75%

The ARC Prize 2025 survey (arXiv:2601.10904) catalogues 90 papers addressing ARC-style tasks, up from 47 the previous year. The growing attention reflects a field-wide recognition that compositional generalization is the bottleneck, not parameter count, not training data volume, not inference-time compute.

The cost numbers are revealing. Reaching 54% -- still 21 points below human baseline -- costs $30 per task using the best available system. This is not a viable path to general compositional reasoning. The cost scales with the brute-force search required to compensate for the absence of genuine composition.

### 1.4 The Kernel-Additivity Ceiling: A Mathematical Proof

Lippl and Stachenfeld (ICLR 2025, arXiv:2405.16391) proved a theorem that moves the composition failure from empirical observation to mathematical certainty. Their result concerns the Neural Tangent Kernel (NTK) regime -- the well-studied limit in which wide neural networks behave as kernel machines.

The theorem states: compositionally-structured kernel models are limited to "conjunction-wise additivity." In concrete terms, this means such models can only compute sums of values over training-seen combinations of features. They can perform f(a,b) = g(a) + h(b) + k(a,b) where (a,b) was seen in training. They cannot perform transitive generalization of equivalence relations -- the fundamental operation required for open-ended composition.

To make this concrete: if a kernel model learns that "A is equivalent to B" and "B is equivalent to C" from separate training examples, it cannot infer that "A is equivalent to C" unless that specific combination appeared in training. This is not a matter of insufficient data or compute. It is a structural limitation of kernel methods, which bound the behavior of transformers in the NTK regime.

The ceiling is architectural. More parameters, more data, more compute, more chain-of-thought scaffolding -- none of these can breach a mathematical barrier. What is needed is a different algebraic structure.

---

## 2. The Mathematical Solution: Category Theory as Architecture

Category theory is the branch of mathematics that studies composition itself. Where set theory asks "what are things?", category theory asks "how do things compose?" This makes it the natural foundation for an architecture designed to compose learned primitives.

The claim here is not speculative. It rests on peer-reviewed work published at top venues (ICML, ICLR, Cambridge University Press) and funded by serious institutions (Symbolica raised $31M USD on the categorical deep learning thesis).

### 2.1 Parametric Lenses: Differentiable Composition by Construction

Cruttwell and Gavranovic (arXiv:2404.00408, March 2024) formalize the concept of a "Block" -- a processing unit in a neural architecture -- as a parametric lens. A parametric lens consists of:

- A parameter space P
- A forward map f: P x A -> B (takes parameters and input, produces output)
- A backward map f*: P x A x B' -> A' x P' (takes parameters, input, and output gradient, produces input gradient and parameter gradient)

This is exactly the structure of a differentiable layer in a neural network. The forward map is the forward pass; the backward map is backpropagation.

The critical property is that composition of parametric lenses is associative by category law. If Block1 is a parametric lens and Block2 is a parametric lens, then Block1 composed with Block2 is a parametric lens, and the composition is guaranteed to satisfy (f . g) . h = f . (g . h). This is not a design goal or an aspiration. It is a mathematical theorem that follows from the category structure.

Furthermore, reverse-mode automatic differentiation (backpropagation) is itself a functor -- a structure-preserving map -- from the category Para(Smooth) to the category Para(Lens(Smooth)). This means that the gradient computation for any composed Block is automatically correct by functoriality. There is no need to manually derive gradients for composed structures; the category theory guarantees them.

Gavranovic, Lessard, and Velickovic published "Categorical Deep Learning: An Algebraic Theory of Architectures" (ICML 2024, arXiv:2402.15332), grounding every Block in the mathematics of Para(Lens(C)). This is not a metaphor or an analogy. The paper proves that standard deep learning architectures -- convolutional networks, recurrent networks, transformers, graph neural networks -- are all instances of the same categorical construction, differing only in the choice of category C. The practical implication is that composition correctness is guaranteed by the mathematics, not verified by testing.

### 2.2 Polynomial Functors: Type-Checked Interaction Protocols

Niu and Spivak's book "Polynomial Functors: A Mathematical Theory of Interaction" (Cambridge University Press, 2024) provides the established categorical framework for interaction protocols. A polynomial functor p(y) = sum_i y^{B_i} encodes a protocol where the system can be in one of several states (indexed by i), and in each state, the environment can make one of B_i possible responses.

For the purposes of a compositional architecture, this means that every Block's input/output protocol can be described as a polynomial functor, and the type-checking of Block composition happens at compose time, not at inference time. If two Blocks have incompatible protocols, their composition is undefined in the polynomial category -- it simply does not exist as a mathematical object. This eliminates an entire class of runtime errors (type mismatches, protocol violations, shape mismatches) by making them compile-time impossibilities.

Polynomial functors are closed under composition, product, and coproduct. This means that composed Block protocols are themselves polynomial functors, which can themselves be composed further. The type system is fractal: composition of typed things produces typed things of the same kind.

### 2.3 DPO Hypergraph Rewriting: Open-Ended Composition

Double-Pushout (DPO) rewriting is a well-studied formalism from algebraic graph theory for modifying graph structures while preserving invariants. A DPO rewrite takes a pattern (the left-hand side), a replacement (the right-hand side), and an interface (the shared boundary), and transforms a host graph by replacing an occurrence of the pattern with the replacement, gluing along the interface.

The key theorem is the pushout-complement property: if the interface is correctly specified, the rewrite is guaranteed to preserve type-correctness of the surrounding graph. Applied to a Graph-of-Blocks architecture, this means that any DPO rewrite -- adding a Block, removing a Block, rewiring connections, replacing a subgraph -- automatically preserves the type-correctness of the entire Graph.

This is the mechanism for open-ended composition. New Block combinations that were never seen during training can be constructed by DPO rewriting, and their type-correctness is guaranteed by the pushout-complement theorem. The architecture can grow and mutate while maintaining the invariants that make composition meaningful.

AlgebraicRewriting.jl provides a working implementation of DPO hypergraph rewriting, demonstrating that this is not merely theoretical but computationally tractable.

---

## 3. HDC Binding Breaks the Kernel Ceiling

Hyperdimensional Computing (HDC) operates on high-dimensional binary vectors (in this architecture, 10,240 bits). The critical operation is binding -- combining two vectors into a single vector that represents their conjunction.

### 3.1 Why Binding Is Structurally Different from Kernel Methods

HDC binding (implemented as component-wise XOR for binary vectors, or circular convolution for real-valued vectors) is a multiplicative operation. Given vectors A and B representing two concepts, the bound vector A * B represents their composition. This operation has three critical algebraic properties:

1. **Distributivity**: A * (B + C) = A*B + A*C (binding distributes over bundling/superposition)
2. **Invertibility**: Given A * B and A, you can recover B (via inverse binding)
3. **Dimensionality preservation**: A * B has the same dimensionality as A and B

These properties make HDC binding structurally different from the kernel inner products that underlie the kernel-additivity ceiling. Kernel methods compute dot products, which are additive operations. HDC binding is multiplicative. The Lippl-Stachenfeld theorem (Section 1.4) proves that conjunction-wise additivity cannot achieve transitive generalization. HDC binding, being multiplicative, is not subject to this theorem.

### 3.2 The 10,240-Bit Fabric Is an Algebraic Operation

In the architecture described in this document, every data element (called a Signal) carries a 10,240-bit HDC fingerprint. This is not a compressed representation or a hash. It is a vector in a 10,240-dimensional binary algebra where:

- Bundling (majority vote) creates superpositions: "A or B or C"
- Binding (XOR) creates compositions: "A composed with B"
- Permutation (bit rotation) creates sequences: "A then B then C"

When the system composes two Blocks, the HDC fingerprints of their inputs and outputs are bound together, producing a fingerprint for the composed operation. This fingerprint encodes the compositional structure -- not just that two things were combined, but how they were combined. The binding operation creates representations that kernel methods cannot express, because binding is multiplicative where kernels are additive.

### 3.3 DPO + HDC: Open-Ended Composition Beyond Kernel Limits

The combination of DPO rewriting (Section 2.3) and HDC binding creates a composition mechanism with two properties that kernel-additive models provably lack:

1. **Structural mutation**: DPO rewrites can create Block compositions never seen in training, with type-correctness guaranteed by the pushout-complement theorem.
2. **Compositional encoding**: HDC binding produces fingerprints for these novel compositions that preserve the algebraic structure (associativity, distributivity, invertibility) needed for transitive generalization.

Together, these mechanisms break the kernel-additivity ceiling not by incremental improvement but by operating in a fundamentally different algebraic regime.

### 3.4 Meta-Learning Compositionality: What DPO Fills

Lake and Baroni published "Human-like systematic generalization through a meta-learning neural network" (Nature 623, 2023). They demonstrated a meta-learning approach that achieves human-parity on few-shot pseudo-language compositional generalization tasks -- learning to compose novel word meanings from a handful of examples.

However, Bushnaq et al. (arXiv:2506.01820, June 2025) showed that meta-learning compositionality scales poorly to open primitive sets. When the set of primitives that need to be composed grows beyond the meta-training distribution, performance degrades. This is the gap that DPO rewriting fills: DPO rewrites can introduce new primitives and compose them with existing ones while preserving type-correctness, without requiring meta-training over the expanded primitive set.

---

## 4. Empirical Proof Points

The theoretical argument (Sections 2-3) predicts that architecturally compositional systems should dramatically outperform LLMs on compositional reasoning tasks, even at tiny parameter counts. Two recent results confirm this prediction.

### 4.1 TRM: Three Orders of Magnitude

Jolicoeur-Martineau et al. (Samsung SAIT, arXiv:2510.04871, October 2025) introduced the Transformation-based Recursive Machine (TRM), a 7-million-parameter recursive model. Its results on compositional reasoning benchmarks:

- ARC-AGI-1: 44.6% (vs. DeepSeek R1, o3-mini, Gemini 2.5 Pro -- all scoring lower with billions of parameters)
- ARC-AGI-2: 7.8%

The parameter count comparison is staggering. TRM has 7 million parameters. The LLMs it outperforms have billions -- a factor of roughly 1,000x. This inversion is not explicable by training data quality or inference-time tricks. It is the direct consequence of architectural compositionality: the model's structure mirrors the compositional structure of the task, allowing it to compose transformations rather than retrieve memorized patterns.

### 4.2 HRM: Two-Timescale Compositional Convergence

Wang et al. (arXiv:2506.21734, June 2025) introduced the Hierarchical Recursive Machine (HRM), a 27-million-parameter model with a two-timescale architecture: a high-level (H) module and a low-level (L) module that converge at different rates during training.

Results:

- ARC-AGI-1: 40.3% (with only 1,000 training examples and no pretraining)
- Also outperforms multi-billion-parameter LLMs on Sudoku-Extreme and Maze-Hard

The "no pretraining" detail is crucial. HRM achieves these results without the benefit of internet-scale pretraining that LLMs require. It learns compositional rules from 1,000 examples because its architecture is structured to compose, not to memorize.

### 4.3 The Target

These proof points establish the empirical validity of the theoretical argument. The target for the architecture described in this document is: 30%+ on ARC-AGI-2 at less than $1 per task with a sub-100-million-parameter Block-Graph stack. This would exceed the current open-source state of the art (24%) at dramatically lower cost than current frontier systems ($2.20-$30 per task), using a model small enough to run on commodity hardware.

The ARC Prize 2025 survey (arXiv:2601.10904) documents the growing momentum in this direction: 90 papers in 2025 versus 47 in 2024, reflecting a field-wide pivot toward architecturally compositional approaches.

---

## 5. Compositional Generalization as Theorem

The preceding sections build toward a specific claim: the compositional generalization of the Block-Graph stack is mathematically guaranteed, not architecturally hoped-for. This section makes the argument explicit.

### 5.1 The Five Guarantees

**Guarantee 1: Differentiability and composition are associative by category law.** Every Block is a parametric lens in Para(Lens(C)) (Cruttwell-Gavranovic, arXiv:2404.00408). Composition of parametric lenses inherits associativity from the category structure. Reverse-mode AD is a functor, so gradient computation for composed Blocks is automatically correct. This is not verified by testing; it is a consequence of functoriality.

**Guarantee 2: Polynomial types are closed under composition.** Block interaction protocols are polynomial functors (Niu-Spivak, Cambridge 2024). The category of polynomial functors is closed under composition, product, and coproduct. Therefore, any composition of type-correct Blocks is itself a type-correct Block with a well-defined polynomial protocol. Type errors in Block composition are compile-time errors, not runtime failures.

**Guarantee 3: DPO rewriting preserves type-correctness.** The pushout-complement theorem guarantees that DPO rewrites on a typed hypergraph preserve the typing of the surrounding graph. When the architecture mutates -- adding Blocks, removing Blocks, rewiring connections -- the type-correctness of the entire Graph is maintained. This is not a design invariant that must be manually enforced; it is a theorem of the rewriting system.

**Guarantee 4: HDC binding exceeds the kernel-additivity ceiling.** HDC binding is a multiplicative operation (XOR / circular convolution) that is structurally different from the additive inner products of kernel methods. The Lippl-Stachenfeld theorem (arXiv:2405.16391) proves that kernel-additive models cannot transitively generalize equivalence relations. HDC binding, being multiplicative, is not subject to this limitation. The 10,240-bit fingerprints encode compositional structure in an algebra that kernel methods cannot reach.

**Guarantee 5: The search over compositional space is systematic.** CycleQD (Quality-Diversity search with cyclic evaluation) combined with active inference (free energy minimization) provides a principled search over the space of Block compositions. CycleQD maintains a population of diverse Block-Graph configurations, evaluated on quality (task performance) and diversity (structural difference). Active inference biases the search toward configurations that minimize surprise -- the difference between predicted and observed outcomes. Together, these mechanisms ensure that the compositional space is explored systematically rather than randomly.

### 5.2 What "Mathematically Guaranteed" Means

The claim is not that the system will solve every compositional reasoning problem. The claim is that the system's composition mechanism does not have an inherent ceiling on composition depth. Unlike kernel-additive models, which provably cannot generalize beyond training-seen combinations (Lippl-Stachenfeld), the Block-Graph stack composes via operations (parametric lens composition, polynomial type composition, DPO rewriting, HDC binding) that are structurally capable of open-ended composition.

Whether the system achieves any particular accuracy on any particular benchmark depends on implementation quality, training data, compute budget, and many engineering factors. But the composition mechanism itself does not decay exponentially with depth (as in transformers, per Dziri et al.), does not collapse under symbolic perturbation (as in LLMs on GSM-Symbolic), and does not hit a kernel-additivity ceiling (as proven by Lippl-Stachenfeld). These specific failure modes are eliminated by mathematical structure, not ameliorated by engineering effort.

---

## 6. Agent-Native Programming Paradigms That Leverage This

The categorical Block-Graph architecture is not just a neural architecture. It is a programming paradigm for building agent systems that optimize their own structure. Several recent frameworks validate pieces of this paradigm.

### 6.1 GEPA: Gradient-Free Optimization of Agent Programs

GEPA (Gradient-Estimation-based Prompt-tuning with Advantages) was published by researchers at Stanford, Berkeley, Databricks, and MIT (arXiv:2507.19457, ICLR 2026 Oral). GEPA optimizes agent workflows (multi-step LLM programs with tools and retrieval) using policy-gradient methods rather than the brute-force rollouts required by GRPO (Group Relative Policy Optimization).

Results: GEPA outperforms GRPO with 35x fewer rollouts at +20% gain. On AIME-2025 (math competition problems), GEPA achieves +12 percentage points over MIPROv2 (the DSPy optimizer).

The relevance to the Block-Graph architecture is direct. GEPA treats agent programs as computation graphs and optimizes them with gradient estimation. In the Block-Graph stack, these computation graphs are explicitly represented as Graphs of Blocks with categorical composition guarantees. GEPA-style optimization becomes more effective when the graphs being optimized have formal composition properties, because the optimizer can exploit associativity and type-correctness to prune the search space.

### 6.2 AFlow: MCTS Over Code-Represented Workflows

AFlow (ICLR 2025 Oral, arXiv:2410.10762) applies Monte Carlo Tree Search (MCTS) over code-represented agent workflows. Instead of hand-designing agent pipelines, AFlow treats the space of possible workflows as a search tree and uses MCTS to find high-performing configurations.

Results: +5.7% average improvement over prior state of the art across multiple benchmarks.

AFlow uses ad-hoc code-level operators to define the search space (add a retrieval step, swap a prompt template, insert a verification stage). In the Block-Graph architecture, these operators are replaced by categorical DPO rewrites, which are type-preserving by theorem rather than by testing. This is a strict upgrade: every AFlow-style search is also a DPO rewrite search, but DPO rewrites additionally guarantee type-correctness of the result, eliminating the need for expensive runtime validation of candidate workflows.

### 6.3 Microsoft Trace + OptoPrime: Optimizer.step() Over Execution Graphs

Microsoft's Trace framework with OptoPrime optimizer (NeurIPS 2024) provides a PyTorch-style `optimizer.step()` interface for optimizing execution-trace DAGs. The developer writes a forward pass (an agent program), Trace records the execution as a DAG, and OptoPrime computes updates to improve performance.

This is structurally isomorphic to the Graph-of-Blocks paradigm. Trace's execution DAG is a Graph. OptoPrime's update step is gradient computation on a parametric lens. The Microsoft team arrived at essentially the same abstraction from the engineering side that the categorical deep learning community derived from the mathematical side.

The Block-Graph architecture formalizes what Trace/OptoPrime leaves implicit: the composition of execution DAGs is associative, the types are polynomial, and the mutations are DPO rewrites. This formalization enables optimizations (compile-time type checking, guaranteed gradient correctness, type-preserving mutations) that Trace/OptoPrime must verify empirically.

### 6.4 TextGrad: Textual Gradients as Parametric Optics

TextGrad (Stanford, published in Nature 2025) introduces textual gradients -- natural language feedback messages passed backward through an agent computation graph, analogous to numerical gradients in neural networks. Each node in the graph receives a text message describing how its output should change to improve the overall result.

This maps precisely onto parametric optics. The forward pass is the "view" (lens get). The textual gradient backward pass is the "update" (lens put). The TextGrad architecture is, mathematically, an instance of parametric lens composition over the category of natural language transformations rather than the category of smooth functions.

The Block-Graph architecture makes this mapping explicit and leverages it: textual gradients for LLM-based Blocks and numerical gradients for differentiable Blocks compose in the same parametric lens framework, because the framework is parametric over the category C. This means hybrid systems (part LLM, part neural, part symbolic) can be optimized end-to-end using the same categorical machinery.

### 6.5 TLA+ Formal Specification: Behavioral Verification

TLA+ (Leslie Lamport's Temporal Logic of Actions) provides formal specification and model checking for concurrent and distributed systems. Recent work has demonstrated that LLM-guided TLAPS (the TLA+ Proof System) makes proof generation tractable for real-world specifications.

In the Block-Graph architecture, TLA+ serves as the behavioral verification layer. The deterministic execution engine guarantees that every Block execution is reproducible. DPO rewrites are type-preserving by theorem. Together with TLA+ specifications of desired behavioral properties (liveness, safety, fairness), every mutation to the Block-Graph is statically verifiable:

1. The DPO rewrite preserves type-correctness (pushout-complement theorem)
2. The TLA+ model checker verifies that the rewritten Graph satisfies behavioral specifications
3. The deterministic engine ensures that verified behavior is reproduced at runtime

This triple guarantee -- type preservation, behavioral verification, deterministic replay -- is unique to this architecture.

---

## 7. Competitive Comparison

No competing agent framework provides the combination of guarantees described in this document. This section compares the Block-Graph stack against the major alternatives.

### 7.1 LangGraph

LangGraph (LangChain) represents agent workflows as state machines with runtime TypedDict typing. It provides:

- Runtime type checking via Python TypedDict (not compile-time, not categorical)
- No composition law (composing two LangGraph workflows does not produce a well-typed workflow by construction)
- No structural mutation safety (modifying a graph can introduce type errors detectable only at runtime)
- No formal verification (no TLA+ or equivalent)
- No HDC binding or equivalent compositional encoding

LangGraph is a practical tool for building agent workflows, but it provides no mathematical guarantees about composition. Every composition must be tested empirically.

### 7.2 CrewAI

CrewAI provides a role-based agent collaboration framework. It has:

- No type system for agent interactions
- No composition law
- No structural mutation mechanism
- No formal verification
- No compositional encoding

CrewAI is an orchestration layer, not a compositional architecture.

### 7.3 DSPy

DSPy (Stanford) provides Signature-based typing for LLM programs. Signatures partially specify input/output types, enabling some compile-time checking and optimization. DSPy is the closest existing framework to the Block-Graph approach, but:

- Signature typing is partial (it types inputs and outputs but not the interaction protocol between modules)
- Composition is not associative by construction (composing DSPy modules can produce type-incorrect programs)
- No DPO rewriting (mutations are not type-preserving by theorem)
- No HDC binding or equivalent
- No formal behavioral verification

DSPy's MIPROv2 optimizer is outperformed by GEPA (Section 6.1) by 12 percentage points on AIME-2025, suggesting that the partial typing limits optimization effectiveness.

### 7.4 Agint (NeurIPS 2025)

Agint provides nominal type floors for agent interactions -- each agent declares its input and output types, and the framework checks compatibility at dispatch time. This is a meaningful step beyond untyped frameworks, but:

- Types are nominal, not structural (two agents with identical behavior but different type names are incompatible)
- Effects (side effects, state mutations, resource consumption) are tracked informally, not via a type system
- No composition law guaranteeing associativity
- No type-preserving structural mutation
- No formal verification

### 7.5 What Only This Stack Provides

The Block-Graph architecture is unique in providing all five of:

1. **Polynomial-functor compile-time types**: Block protocols are polynomial functors, checked at compose time, closed under composition. Not runtime TypedDict (LangGraph), not partial Signatures (DSPy), not nominal floors (Agint).

2. **Parametric optic composition**: Block composition is associative by category law. Gradient computation for composed Blocks is correct by functoriality. Not empirically verified (LangGraph, DSPy), not absent (CrewAI).

3. **DPO type-preserving rewrites**: Structural mutations to the Block-Graph preserve type-correctness by the pushout-complement theorem. The architecture can evolve -- adding, removing, rewiring Blocks -- with mathematical guarantees that the result is well-typed. No competing framework provides this.

4. **TLA+ behavioral verification**: Combined with the deterministic execution engine, every mutation is not only type-correct but behaviorally verified against formal specifications. No competing framework integrates formal methods at the architectural level.

5. **Deterministic replay**: Every Block execution is reproducible, enabling debugging, auditing, and verification of production behavior. Combined with the four guarantees above, this creates a system where compositional generalization is verifiable, not just achievable.

The competitive moat is not any single capability. It is the combination. Any competitor can adopt polynomial types, or parametric optics, or DPO rewriting, or TLA+ verification, or deterministic replay individually. The moat is that these five capabilities compound: polynomial types make DPO rewrites type-preserving, parametric optics make gradient computation correct for composed Blocks, TLA+ verification leverages deterministic replay to check behavioral properties, and HDC binding provides the compositional encoding that breaks the kernel-additivity ceiling. Adopting one without the others provides marginal benefit. Adopting all five requires rebuilding the architecture from categorical foundations -- a multi-year effort that is the primary barrier to competitive replication.

---

## Summary of Citations

| Paper | Authors | Venue | arXiv / Reference |
|---|---|---|---|
| Faith and Fate | Dziri et al. | NeurIPS 2023 | arXiv:2305.18654 |
| GSM-Symbolic | Mirzadeh et al. (Apple) | 2024 | arXiv:2410.05229 |
| ARC Prize 2025 Survey | ARC Prize Foundation | 2025 | arXiv:2601.10904 |
| Kernel-Additivity Ceiling | Lippl, Stachenfeld | ICLR 2025 | arXiv:2405.16391 |
| Parametric Lenses | Cruttwell, Gavranovic | 2024 | arXiv:2404.00408 |
| Categorical Deep Learning | Gavranovic, Lessard, Velickovic | ICML 2024 | arXiv:2402.15332 |
| Polynomial Functors | Niu, Spivak | Cambridge University Press 2024 | ISBN 978-1009349987 |
| MLC Meta-Learning | Lake, Baroni | Nature 623, 2023 | DOI:10.1038/s41586-023-06668-3 |
| Meta-Learning Compositionality Limits | Bushnaq et al. | 2025 | arXiv:2506.01820 |
| TRM | Jolicoeur-Martineau et al. (Samsung SAIT) | 2025 | arXiv:2510.04871 |
| HRM | Wang et al. | 2025 | arXiv:2506.21734 |
| GEPA | Stanford/Berkeley/Databricks/MIT | ICLR 2026 Oral | arXiv:2507.19457 |
| AFlow | -- | ICLR 2025 Oral | arXiv:2410.10762 |
| Microsoft Trace + OptoPrime | Microsoft Research | NeurIPS 2024 | -- |
| TextGrad | Stanford | Nature 2025 | -- |
