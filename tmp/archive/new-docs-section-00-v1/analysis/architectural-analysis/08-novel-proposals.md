---
title: "Finding: Three Novel Proposals"
section: analysis
subsection: architectural-analysis
id: aa-08
source: 23-architectural-analysis-improvements.md (§8)
tags: [novel-proposals, competitive-router, gradient-gate, hierarchical-pipeline, LIDA, active-inference, category-theory]
---

# Finding: Three Novel Proposals

> **Status**: Proposals — not yet implemented. Each is self-contained and preserves backward compatibility.

## Context

Source file 23 (§8) derives three novel architectural proposals directly from the analysis findings. Each proposal:
- follows from the theoretical framework established in findings AA-02 through AA-07
- is expressed as concrete Rust code
- plugs in via the existing trait system without breaking existing impls
- carries a cited theoretical basis

---

## Proposal 1: CompetitiveRouter (LIDA-Inspired)

### Problem

The current `Router` receives candidates scored by a **single** `Scorer` and selects one. Single-scorer selection is brittle — the scorer's blind spots become the router's blind spots.

### Proposal

Implement LIDA-style competitive attention where **multiple** `Scorer`s run concurrently, form coalitions around agreement, and the strongest coalition wins selection.

```rust
pub struct CompetitiveRouter {
    scorers: Vec<Box<dyn Scorer>>,
    coalition_threshold: f32,
    inner_router: Box<dyn Router>,
}

impl Router for CompetitiveRouter {
    fn select(&self, candidates: &[Signal], ctx: &Context) -> Option<Selection> {
        // 1. Each scorer independently scores all candidates
        let score_matrix: Vec<Vec<Score>> = self.scorers.iter()
            .map(|s| candidates.iter().map(|c| s.score(c, ctx)).collect())
            .collect();

        // 2. Form coalitions: scorers that agree on the top candidate
        let coalitions = form_coalitions(&score_matrix, self.coalition_threshold);

        // 3. Strongest coalition's top candidate wins
        let winning_coalition = coalitions.into_iter()
            .max_by_key(|c| c.members.len())?;

        // 4. Inner router selects from the winning coalition's candidates
        let coalition_candidates: Vec<Signal> = winning_coalition.top_candidates
            .iter()
            .filter_map(|&idx| candidates.get(idx).cloned())
            .collect();

        self.inner_router.select(&coalition_candidates, ctx)
    }
}
```

### Theoretical Basis

LIDA's attention codelets (Franklin et al. 2016) demonstrate that competitive attention produces more robust selection than single-scorer evaluation. The "consciousness spotlight" (Global Workspace Theory, Baars 1988) is the winning coalition that broadcasts its content to all subsystems.

### Integration Cost

`CompetitiveRouter` implements `Router`, so it plugs directly into `loop_tick` with zero changes to the pipeline structure. Estimated effort: Large (requires `form_coalitions` algorithm and testing framework). See finding [AA-10](./10-prioritized-improvements.md): Improvement #9, Low Priority.

### Cross-References
- [AA-04: Cognitive Speeds](./04-finding-cognitive-speeds.md) — Coalition-based selection is more appropriate at Gamma speed (fast decisions under time pressure)
- [AA-07: Category Theory](./07-finding-category-theory.md) — `CompetitiveRouter` is still a morphism `Vec<Signal> → Option<Selection>` in the Engram category; it does not break categorical structure
- Integration pair: [neuro-x-composition](../integration-map/neuro-x-composition.md) — knowledge-scored coalitions reinforce knowledge injection

---

## Proposal 2: Gradient Gate Feedback (Active Inference Enhancement)

### Problem

The current `Gate` returns binary pass/fail `Verdict`. Learning uses only the boolean. The continuous confidence score `verdict.score ∈ [0,1]` is discarded.

### Proposal

Use the Gate's confidence score as a **continuous** learning signal, and treat low-confidence verdicts as high prediction-error events that generate knowledge insights.

```rust
// After gate verification in loop_tick
let verdict = gate.verify(&composed, ctx).await;

// Continuous feedback to Router (not just success/failure)
let outcome = Outcome {
    selection: selection.clone(),
    success: verdict.passed,
    reward: verdict.score,  // Use continuous score, not binary
    cost: Some(inference_cost),
    latency_ms: Some(elapsed.as_millis() as u64),
};
router.feedback(&outcome);

// Active inference: high surprise → generate learning signal
if verdict.score < 0.3 {
    // High prediction error → create insight for Neuro
    let insight = Signal::builder()
        .kind(Kind::Insight)
        .body(Body::Json(json!({
            "gate": verdict.gate,
            "error": verdict.reason,
            "context": ctx.goal,
        })))
        .provenance(Provenance::trusted("gate-learner"))
        .build();
    substrate.put(insight).await?;
}
```

### Theoretical Basis

Active inference (Friston 2010) frames verification as free energy minimization. The Gate's confidence score is a direct measure of prediction error. Using it as a continuous learning signal (not binary) enables gradient-based model updating — the system updates its model of the world proportional to the magnitude of surprise, not just the binary presence of failure.

### Integration Cost

This enhancement modifies `loop_tick` behavior but preserves its signature. All existing trait implementations continue to work. Estimated effort: Medium. See finding [AA-10](./10-prioritized-improvements.md): Improvement #5, Medium Priority.

**This is the most impactful near-term improvement** identified in the full analysis. The conclusion of source file 23 states this explicitly.

### Cross-References
- [AA-07: Category Theory](./07-finding-category-theory.md) — Verdict is a filtered monoid; gradient feedback preserves this structure while enriching the learning signal
- [AA-06: Cross-Cut Isolation](./06-finding-crosscut-isolation.md) — The generated `Insight` Engram flows into `NeuroStore` — this is the Neuro→Learning feedback path
- Integration pairs: [learning-x-verification](../integration-map/learning-x-verification.md), [neuro-x-verification](../integration-map/neuro-x-verification.md)
- Missing integration M14 in [24-cross-section-integration-map.md] relates: knowledge-informed thresholds complement gradient feedback

---

## Proposal 3: Hierarchical Pipeline Composition

### Problem

Each cognitive speed runs the same `loop_tick` with different parameters. The relationship between speeds (how Theta "folds" Gamma outcomes, how Delta "folds" Theta outcomes) is implicit in scheduling logic.

### Proposal

Formalize the relationship between speeds as **monoid homomorphisms** — explicit fold operations that connect outputs of one speed to inputs of the next.

```rust
/// A pipeline is a configured loop_tick — a closure over trait implementations.
pub struct Pipeline {
    substrate: Arc<dyn Substrate>,
    scorer: Arc<dyn Scorer>,
    router: Arc<dyn Router>,
    composer: Arc<dyn Composer>,
    gate: Arc<dyn Gate>,
    policy: Arc<dyn Policy>,
}

impl Pipeline {
    /// Fold multiple pipeline outputs into a single pipeline input.
    /// This is the monoid homomorphism that connects cognitive speeds.
    pub fn fold_outcomes(outcomes: &[TickOutcome]) -> Query {
        // Theta folds Gamma outcomes; Delta folds Theta outcomes
        let hashes: Vec<ContentHash> = outcomes.iter()
            .flat_map(|o| o.written.iter().cloned())
            .collect();
        Query::by_lineage(hashes)
    }
}

/// Gamma → Theta → Delta as a composed pipeline
pub fn hierarchical_tick(
    gamma: &Pipeline,
    theta: &Pipeline,
    gamma_outcomes: &[TickOutcome],
) -> impl Future<Output = Result<TickOutcome>> {
    let query = Pipeline::fold_outcomes(gamma_outcomes);
    theta.tick(&query)
}
```

### Theoretical Basis

If `TickOutcome` forms a monoid (under concatenation of written hashes), then `fold_outcomes` is a monoid homomorphism. Category theory guarantees that this fold composes correctly, meaning Theta's processing of Gamma outcomes is well-defined regardless of how many Gamma ticks produced how many outcomes.

### Integration Cost

Estimated effort: Medium. Primarily a structural refactor of the scheduler; existing `loop_tick` implementations are unchanged. See finding [AA-10](./10-prioritized-improvements.md): Improvement #7, Medium Priority.

### Cross-References
- [AA-07: Category Theory](./07-finding-category-theory.md) — Score is a commutative monoid; Pipeline as monoid homomorphism extends this to the inter-speed relationship
- [AA-04: Cognitive Speeds](./04-finding-cognitive-speeds.md) — The three speeds are the three levels of the hierarchy; this proposal formalizes their connection
- Integration pairs: [learning-x-composition](../integration-map/learning-x-composition.md), [orchestration-x-learning](../integration-map/orchestration-x-learning.md)

---

## Summary Table

| Proposal | Theoretical Basis | Priority | Effort | Backward Compatible? |
|---|---|---|---|---|
| **CompetitiveRouter** | LIDA attention codelets, Global Workspace Theory | Low | Large | Yes — implements `Router` |
| **Gradient Gate Feedback** | Active Inference (Friston), FEP | Medium | Medium | Yes — extends `loop_tick` |
| **Hierarchical Pipeline** | Monoid homomorphisms, Category Theory | Medium | Medium | Yes — wraps existing `loop_tick` |

---

## Academic References

- Franklin, S. et al. (2016). "LIDA: A Systems-level Architecture for Cognition, Emotion, and Learning." IEEE Trans. AMD 6(1).
- Baars, B. J. (1988). "A Cognitive Theory of Consciousness." Cambridge University Press.
- Friston, K. (2010). "The free-energy principle: a unified brain theory?" Nature Reviews Neuroscience 11(2).
- Parr, T. et al. (2024). "Active Inference: The Free Energy Principle in Mind, Brain, and Behavior." arXiv:2402.14460.
- Milewski, B. (2014). "Category Theory for Programmers." bartoszmilewski.com.
