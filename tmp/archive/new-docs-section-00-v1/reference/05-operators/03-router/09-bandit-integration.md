# Bandit Integration — UCB, LinUCB, Track-and-Stop

> How multi-armed bandit algorithms are integrated into the Router for exploration-driven
> action selection.

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

## TL;DR

The `UCBRouter` implements UCB1 (Upper Confidence Bound). `LinUCB` and Track-and-Stop are
planned extensions for context-dependent and best-arm identification routing respectively.
All three share the same `Router` trait and differ only in how they compute the arm selection
score.

---

## UCB1 Algorithm

For each `ActionKind` arm `a`, maintain:
- `n_a` — number of times arm `a` was pulled.
- `r_a` — sum of rewards received from arm `a`.
- `mean_a = r_a / n_a`.

Selection:

```
a* = argmax_a [ mean_a + C * sqrt(ln(N) / n_a) ]
```

where `N = sum of all n_a` and `C` is the exploration constant.

**Reward signal**: After each loop tick, the Policy operator (or the OBSERVE step) calls
`router.update_reward(action, reward: f32)` where `reward` is derived from the outcome
(e.g., user satisfaction, task completion, prediction error reduction).

---

## LinUCB (Planned)

LinUCB is a contextual bandit: the reward is a linear function of context features derived
from the `Engram`'s `Score` and fingerprint. This allows the router to learn that
`Action::ExecuteTask` is best when `score.confidence > 0.8 AND kind == Task`, rather than
globally.

LinUCB uses a ridge regression estimate per arm:

```
reward_estimate(a, x) = θ_a^T x  (linear in context x)
ucb(a, x) = θ_a^T x + alpha * sqrt(x^T A_a^{-1} x)
```

---

## Track-and-Stop (Planned)

Track-and-Stop is a best-arm identification algorithm: given a budget of N trials, find the
arm with the highest mean reward with high probability (delta-correct). This is used for
offline evaluation of routing strategies, not live agent operation.

---

## Reward Sources

| Reward source | Description |
|---|---|
| Explicit outcome (`outcome.quality`) | User rates the response explicitly |
| Implicit outcome (session continuation) | User continued the session → positive |
| Prediction error reduction | Policy reports lower error → positive |
| Gate rejection on next tick | The action led to a low-quality engram → negative |

---

## See Also

- [Semantics](./02-semantics.md)
- [Policy Overview](../05-policy/00-overview.md) — provides outcome signals

## Open Questions

- What is the right reward normalisation? Raw outcome scores range 0–1; rewards should too.
- Should `update_reward` be part of the `Router` trait or a separate `BanditFeedback` trait?
