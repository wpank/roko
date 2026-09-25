# Causal Microstructure Discovery

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 8

---

## Structural Causal Model (SCM)

```rust
pub struct StructuralCausalModel {
    pub exogenous: Vec<Variable>,
    pub endogenous: Vec<Variable>,
    pub equations: HashMap<VariableId, StructuralEquation>,
    pub graph: CausalGraph,
}

pub struct Variable {
    pub id: VariableId,
    pub name: String,
    pub domain: VariableDomain,
}

pub enum VariableDomain {
    Continuous { min: f64, max: f64 },
    Discrete(Vec<String>),
    Binary,
}

pub struct StructuralEquation {
    pub target: VariableId,
    pub parents: Vec<VariableId>,
    pub function: Box<dyn Fn(&HashMap<VariableId, f64>) -> f64 + Send + Sync>,
    pub noise: NoiseDistribution,
}
```

## The do-Operator

```rust
pub fn do_intervention(
    scm: &StructuralCausalModel,
    variable: VariableId,
    value: f64,
) -> InterventionalDistribution {
    let mut modified = scm.clone();
    // Replace variable's equation with a constant
    modified.equations.insert(variable, StructuralEquation {
        target: variable,
        parents: vec![],
        function: Box::new(move |_| value),
        noise: NoiseDistribution::Constant(0.0),
    });
    // Remove incoming edges (sever causal mechanism)
    modified.graph.remove_incoming_edges(variable);
    modified.propagate()
}
```

## PC Algorithm

```rust
pub fn pc_algorithm(
    data: &DataFrame,
    alpha: f64,
    max_conditioning_set: usize,
) -> CausalGraph {
    let variables: Vec<VariableId> = data.columns().collect();
    let mut graph = CausalGraph::complete_undirected(&variables);

    // Phase I: Edge removal via conditional independence
    for conditioning_size in 0..=max_conditioning_set {
        for (x, y) in graph.edges() {
            let neighbors = graph.neighbors(x);
            for conditioning_set in neighbors.combinations(conditioning_size) {
                if conditional_independence_test(data, x, y, &conditioning_set, alpha) {
                    graph.remove_edge(x, y);
                    graph.add_separation_set(x, y, conditioning_set);
                    break;
                }
            }
        }
    }

    // Phase II: Orient v-structures
    for (x, z) in graph.undirected_edges() {
        for y in graph.common_neighbors(x, z) {
            if !graph.separation_set(x, z).contains(&y) {
                graph.orient(x, y);
                graph.orient(z, y);
            }
        }
    }

    // Phase III: Meek's orientation rules
    graph.apply_meek_rules();
    graph
}
```

## Granger Causality

```rust
pub struct GrangerCausalityTest {
    pub max_lag: usize,
    pub alpha: f64,
}
```

Extensions for Roko:
1. **Irregular-interval support**: for domains without fixed cadence
2. **Nonlinear extensions**: kernel methods for nonlinear relationships
3. **Confounding-robust variants**: conditioning on potential confounders
4. **Domain-specific lag calibration**: commit-level for code, sub-second for ops

## Interventional Experiments

For domains with simulation capability:

```
1. Identify candidate causal edge X -> Y
2. Fork the simulation state
3. Apply do(X = x)
4. Observe effect on Y in simulation
5. Compare with observational prediction (L1)
6. If they differ: genuine causal relationship
7. If they agree: may be confounded
```

## Counterfactual Reasoning via Dreams

During REM-phase dreaming:

```
1. Select an episode with a significant outcome
2. Identify the intervention point: "What if X had been different?"
3. Replay the episode with do(X = x') on the learned SCM
4. Compare actual outcome with counterfactual outcome
5. If they differ: X causally contributed to the outcome
6. Store as counterfactual knowledge entry in Neuro
```

## HDC Encoding of Causal Graphs

```rust
pub fn encode_causal_edge(
    cause_role: &HdcVector,
    cause_hv: &HdcVector,
    effect_role: &HdcVector,
    effect_hv: &HdcVector,
) -> HdcVector {
    let cause_binding = cause_role.xor(cause_hv);
    let effect_binding = effect_role.xor(effect_hv);
    HdcVector::bundle(&[cause_binding, effect_binding])
}
```

Enables nanosecond causal pattern matching across domains.

## Domain-Specific Causal Models

### Coding domain variables

| Variable | Type | What it captures |
|---|---|---|
| change_size | Continuous | Number of lines/files changed |
| complexity_delta | Continuous | Change in cyclomatic complexity |
| test_coverage | Continuous | Test coverage percentage |
| build_success | Binary | Did the build pass? |
| test_failures | Discrete | Number of failing tests |
| review_quality | Ordinal | Quality of code review |

### Research domain variables

| Variable | Type | What it captures |
|---|---|---|
| sample_size | Continuous | Study sample size |
| methodology_rigor | Ordinal | Preregistered, blinded, etc. |
| effect_size | Continuous | Reported effect magnitude |
| replication_status | Binary | Did the study replicate? |
| citation_count | Continuous | Number of citations |
| contradiction_count | Discrete | Contradicting studies |

## Causal Knowledge Persistence

Discovered causal relationships are stored as `CausalLink` entries in the
Neuro knowledge store:

```rust
pub struct CausalLink {
    pub cause: VariableId,
    pub effect: VariableId,
    pub strength: f64,          // estimated causal effect size
    pub confidence: f64,        // confidence in the causal claim
    pub discovery_method: CausalMethod,
    pub evidence_level: PearlLevel,  // L1, L2, or L3
    pub domain: OracleDomain,
}

pub enum CausalMethod {
    PcAlgorithm,
    GrangerCausality,
    Interventional,
    Counterfactual,
}
```

## Academic Foundations

- Pearl, J. (2009). *Causality*. 2nd ed. Cambridge University Press.
- Spirtes, P., Glymour, C., & Scheines, R. (2000). *Causation, Prediction, and Search*. MIT Press.
- Granger, C. W. J. (1969). "Investigating causal relations." *Econometrica*, 37(3), 424-438.
