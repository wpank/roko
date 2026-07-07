# roko-learn — Test Coverage

> 101 tests for the learning layer: episode logging, bandit algorithms, playbooks, and the feedback loop.

**Status**: Shipping
**Crate**: `roko-learn`
**Section**: 05 — Learning
**Last reviewed**: 2026-04-19

---

## Test Count: 101

Source: implementation status audit, 2026-04-17.

| Module | Approx. tests | Focus |
|---|---|---|
| `episode_logger` | ~20 | Episode creation, outcome recording, retrieval |
| `bandit_ucb1` | ~20 | UCB1 arm selection, reward update, convergence |
| `bandit_linucb` | ~20 | LinUCB contextual bandit, feature vectors |
| `bandit_track_stop` | ~10 | Track-and-Stop best-arm identification |
| `playbook` | ~15 | Rule extraction, pattern matching, application |
| `skill_library` | ~10 | Skill storage, retrieval, scoring |
| `c_factor` | ~6 | Collective intelligence metric computation |

---

## Key Test Focus Areas

### Episode Logger

- An episode records start time, task context, and all LLM calls made during the task.
- Outcome (success/failure/partial) is set atomically at episode close.
- Episodes are retrievable by task ID, time range, and outcome.
- An open episode (not yet closed) is not retrievable as a completed episode.

### Bandit Algorithms (UCB1, LinUCB, Track-and-Stop)

Tests verify the decision-theoretic correctness of each bandit:

**UCB1**:
- With all arms unexplored, each arm is selected once in the first N rounds.
- UCB1 converges to the best arm over 1000 rounds (high-reward arm selected > 70% of time).
- Reward update increments the arm's mean reward and decrements regret.

**LinUCB**:
- Arms with higher feature-reward correlation are preferred.
- The contextual feature vector influences arm selection.
- Convergence: LinUCB converges to best arm faster than UCB1 when features are predictive.

**Track-and-Stop**:
- Returns the estimated best arm with configurable confidence δ.
- Exploration stops when the confidence bound is satisfied.

Key property: [../by-property/bandit-score-monotonicity.md](../by-property/bandit-score-monotonicity.md).

### Playbook Rules

- A rule that matches on task context is retrieved on subsequent matching tasks.
- A rule that does not match on context is not retrieved.
- Rule scoring: more-specific rules score higher than general ones.
- Conflicting rules: the highest-scoring rule wins.

### C-Factor

- C-Factor metric is in range [0, 1].
- C-Factor increases when agent outputs are more diverse.
- C-Factor decreases when agent outputs are more uniform.

Key property: [../by-property/c-factor-bounds.md](../by-property/c-factor-bounds.md).

---

## Property Tests

| Property | Test name |
|---|---|
| Bandit score monotonicity | `bandit_best_arm_score_monotone` |
| Episode completeness | `all_llm_calls_recorded_in_episode` |
| C-Factor bounds | `c_factor_in_unit_interval` |
| Playbook rule determinism | `playbook_retrieval_deterministic` |

---

## Known Gaps

- `roko-learn` has 42 modules and 35,847 LOC but only 101 tests — a low ratio. Many modules (pattern miner, regression detector, efficiency events) are sparsely tested.
- No integration tests for the full 10-subsystem simultaneous update path.
- No adversarial tests for bandit algorithms under distribution shift.

## See also

- [../by-property/bandit-score-monotonicity.md](../by-property/bandit-score-monotonicity.md)
- [../by-property/c-factor-bounds.md](../by-property/c-factor-bounds.md)
- [../gaps-and-roadmap.md](../gaps-and-roadmap.md) — roko-learn is listed as a coverage gap
