# Active Inference State Space: Factorized Discrete POMDP

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/11-active-inference-state-space.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS13.

---

## 1. Abstract

Active inference promises principled, zero-hyperparameter compute allocation, but
naive implementation is intractable. A real-world agent state space is effectively
infinite. The key insight from Koudahl et al. (2024, arXiv:2412.10425) and VERSES
AI's Genius platform: **do not model the world -- model the agent's epistemic
situation.**

Instead of tracking the full environment state, track three dimensions that fully
characterize what the agent needs for good decisions:
1. **Where am I in the task lifecycle?** (TaskPhase)
2. **How good is my current context?** (ContextQuality)
3. **How uncertain am I?** (Uncertainty)

This factorized state space has only 6 * 5 * 3 = **90 states** -- completely
tractable for standard active inference POMDP matrices.

---

## 2. The Factorized State Space

```
State = (TaskPhase, ContextQuality, Uncertainty)

TaskPhase in { Understanding, Planning, GatheringContext,
               Implementing, Verifying, Complete }         -- 6 states

ContextQuality in { None, Insufficient, Partial,
                    Adequate, Comprehensive }              -- 5 states

Uncertainty in { High, Medium, Low }                       -- 3 states

Total: 6 * 5 * 3 = 90 states
```

### 2.1 Why These Three Dimensions

**TaskPhase** determines what kind of work is appropriate. During `Understanding`,
invest in retrieval (T1). During `Implementing`, invest in deep reasoning (T2).
During `Verifying`, run gates (T0).

**ContextQuality** determines whether more retrieval is needed. If `None` or
`Insufficient`, gather more before acting. If `Comprehensive`, proceed.

**Uncertainty** determines the tier. `High` -> T2. `Low` -> T0 or T1. `Medium` -> T1.

---

## 3. The Four POMDP Matrices

Following the pymdp framework (Heins et al. 2022):

### 3.1 A Matrix (Likelihood): States -> Observations

Maps hidden states to observable signals. Observations are Pulses on the Bus,
joined by lineage:

```
observation = join(
  prediction.<operator>,
  outcome.<operator>,
  prediction.error.<operator>
)
```

Observable signals:
- `compilation_result` in {success, warning, failure, not_applicable}
- `test_pass_rate` in {high, medium, low, not_applicable}
- `embedding_similarity` in {high, medium, low}
- `prediction.error.<operator>` in {low, medium, high}
- `gate_verdict` in {pass, fail, not_applicable}

Example entries:
```
P(compilation_result=success | implementing, comprehensive, low) = 0.8
P(compilation_result=failure | implementing, insufficient, high) = 0.6
P(test_pass_rate=high | verifying, adequate, low) = 0.7
```

### 3.2 B Matrix (Transitions): Actions -> State Changes

```
Actions:
- retrieve_context     -- Query Neuro, run code intelligence
- implement            -- Write code, execute task steps
- run_tests            -- Run gate pipeline
- reflect              -- Theta-style step-back reflection
- escalate             -- Switch to stronger model (T1->T2)
- suppress             -- Stay at T0, no action
```

Example entries:
```
P(context=adequate | context=insufficient, retrieve_context) = 0.6
P(phase=implementing | phase=planning, implement) = 0.7
P(uncertainty=low | uncertainty=medium, run_tests) = 0.5
P(phase=complete | phase=verifying, run_tests, gate_pass=true) = 0.8
```

### 3.3 C Matrix (Preferences): Desired Observations

```
C (preferred observations):
- compilation_result=success:    +2.0
- test_pass_rate=high:           +2.0
- prediction.error.low:          +1.0
- gate_verdict=pass:             +3.0  (highest)
- compilation_result=failure:    -2.0
- gate_verdict=fail:             -3.0
```

### 3.4 D Matrix (Initial Beliefs): Prior State Distribution

```
D (initial belief):
- TaskPhase = Understanding:     P = 0.8
- ContextQuality = None:         P = 0.7
- Uncertainty = High:            P = 0.6
```

Updated across tasks: an experienced agent starts with `Uncertainty=Medium`.

---

## 4. EFE Computation

```python
def compute_efe(action, current_beliefs, A, B, C):
    predicted_state = B[action] @ current_beliefs
    predicted_obs = A @ predicted_state
    pragmatic = -KL_divergence(predicted_obs, softmax(C))
    epistemic = 0.0
    for state_idx in range(90):
        if predicted_state[state_idx] > 0.001:
            obs_given_state = A[:, state_idx]
            epistemic += predicted_state[state_idx] * entropy(obs_given_state)
    return -(pragmatic + epistemic)
```

With 90 states and 6 actions, this computation takes **microseconds**. All matrices
fit in a few kilobytes.

---

## 5. Mapping to Tier Selection

```rust
fn select_tier_efe(
    beliefs: &BeliefState,
    matrices: &POMDPMatrices,
) -> InferenceTier {
    let efe_suppress = compute_efe(Action::Suppress, beliefs, matrices);
    let efe_quick = compute_efe(Action::RetrieveContext, beliefs, matrices);
    let efe_deep = compute_efe(Action::Escalate, beliefs, matrices);

    let efes = [
        (InferenceTier::T0, efe_suppress),
        (InferenceTier::T1, efe_quick.min(compute_efe(Action::Implement, beliefs, matrices))),
        (InferenceTier::T2, efe_deep.min(compute_efe(Action::Reflect, beliefs, matrices))),
    ];

    efes.iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(tier, _)| *tier)
        .unwrap_or(InferenceTier::T1)
}
```

### 5.1 When EFE Beats Heuristic Threshold

1. **Exploration**: In `Understanding` with `None` context, heuristic might suppress
   (low prediction error) but EFE correctly identifies high epistemic value.
2. **Late-stage verification**: In `Verifying` with `Comprehensive` context, heuristic
   might escalate (test failures) but EFE correctly identifies that T0 is better.
3. **Budget optimization**: EFE naturally accounts for cost, eliminating separate
   throttling logic.

---

## 6. Learning the Matrices

### A Matrix Learning
After each gamma tick: `A[observation, state] += learning_rate * (observed_obs == predicted_obs)`

### B Matrix Learning
After each transition: `B[next_state, current_state, action] += learning_rate * (transition_occurred)`

### C Matrix Learning
Slowly from task outcomes. Higher C for observation-state pairs that correlate
with success.

### D Matrix Learning
Across tasks: update initial belief to match empirical starting states.

All learning is Bayesian: count-based updates with Dirichlet priors.

---

## 7. Comparison to Alternatives

| Approach | Hyperparameters | Exploration/Exploitation | Cost | State |
|---|---|---|---|---|
| Fixed threshold | 2 | None | Separate throttle | Scalar |
| Epsilon-greedy | 1 | Random | Not integrated | None |
| UCB1 | 1 | Bonus for under-explored | Not integrated | Counts |
| Thompson sampling | Prior dist. | Posterior sampling | Not integrated | Beta |
| **Active inference** | **0** | **Emergent from EFE** | **Integrated** | **90-state POMDP** |

---

## 8. References

- **Friston 2010** -- "The free-energy principle" (Nature Reviews Neuroscience 11(2)).
- **Friston et al. 2015** -- "Active inference and epistemic value" (Cognitive
  Neuroscience 6(4)).
- **Koudahl et al. 2024** -- "Factorized discrete POMDP" (arXiv:2412.10425).
- **Heins et al. 2022** -- "pymdp" (JOSS). Reference active inference implementation.
- **VERSES AI** -- Genius platform. Industrial active inference deployment.
- **Parr & Friston 2017** -- "Working memory, attention, and salience" (Scientific
  Reports 7).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/active-inference-compute-allocation.md` -- EFE theory
- `docs/v3/depth/29-heartbeat/dual-process-t0-t1-t2.md` -- Heuristic threshold
- `docs/v3/depth/29-heartbeat/16-t0-probes.md` -- Probe signals -> observations
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
