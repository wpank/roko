# Process Reward Models

> Depth file for [07-GATES.md](../../07-GATES.md) sections 8-9.
> Source: `crates/roko-gate/src/process_reward.rs`

> **Citation:** Lightman et al. "Let's Verify Step by Step"
> (arXiv:2305.20050, 2023) -- PRM800K dataset; process supervision
> outperforms outcome supervision for mathematical reasoning.

---

## 1. Overview

Process reward models (PRMs) score intermediate reasoning steps, not just
final output. In agent-driven development: evaluating each tool call, each
file edit, each reasoning turn -- not just whether the final code passes
gates.

Standard verification is binary: the code either compiles or it does not.
Process rewards add granularity: *how much progress did the agent make
toward a working solution?* This matters because:

- An agent 90% of the way to a working solution is more promising than
  one that made no progress
- Intermediate progress signals enable early intervention
- Process rewards provide 10x richer training signal than final pass/fail

---

## 2. Two Dimensions: Promise and Progress

### 2.1 Promise

**Promise** estimates how likely the current execution is to eventually
succeed. It answers: "Is this approach heading somewhere good?"

High Promise indicators:

- Agent makes compile-passing edits (Rung 0 passes)
- Tool calls target the right files
- Edit patterns match successful historical executions

Low Promise indicators:

- Same edit repeated (loop detection)
- Tool calls target unrelated files
- Error count is increasing, not decreasing

**Intervention:** Low Promise -> early termination before the full retry
budget is consumed.

### 2.2 Progress

**Progress** measures whether the agent is advancing or stalling. It
answers: "Is this attempt doing better than the previous one?"

Positive Progress: higher rung reached, fewer errors, more tests passing.
Negative Progress: same or lower rung, same or more errors.

**Intervention:** Negative Progress across multiple attempts -> re-planning.

---

## 3. The Promise Score Function

```
Promise(attempt) = w_1 * rung_fraction
                 + w_2 * test_pass_rate
                 + w_3 * error_trend
                 + w_4 * tool_efficiency
```

Where:

- `rung_fraction = highest_rung_passed / total_rungs` (0.0 to 1.0)
- `test_pass_rate = tests_passed / total_tests` (0.0 to 1.0)
- `error_trend = 1.0` if errors decreasing, `0.5` if stable, `0.0` if
  increasing
- `tool_efficiency = useful_tool_calls / total_tool_calls`

Default weights: `w_1 = 0.4, w_2 = 0.3, w_3 = 0.2, w_4 = 0.1`.

### Promise Thresholds

| Promise | Action |
|---------|--------|
| > 0.8 | Continue, possibly reduce retries |
| 0.4 - 0.8 | Continue with standard retries |
| 0.2 - 0.4 | Consider early termination |
| < 0.2 | Terminate, try different approach |

---

## 4. The Progress Score Function

```
Progress(attempt_n) = Delta_rung + Delta_test_rate + Delta_error_count
```

Where:

- `Delta_rung = (current_rung - previous_rung) / total_rungs`
- `Delta_test_rate = current_pass_rate - previous_pass_rate`
- `Delta_error_count = (previous_errors - current_errors) /
  max(previous_errors, 1)`

### Progress Thresholds

| Progress | Action |
|----------|--------|
| > 0.1 | Advancing -- continue |
| -0.1 to 0.1 | Stalling -- escalate complexity, adjust prompt |
| < -0.1 | Regressing -- stop retrying, re-plan |

---

## 5. Three Feedback Timescales

1. **Process reward** (per-turn): Promise/Progress -> continue/terminate
2. **Retry loop** (per-attempt): Gate verdict -> retry with adjusted prompt
3. **Escalation** (across attempts): Repeated failure -> add rungs, re-plan

---

## 6. Continuous Progress Scores

> **Citation:** AgentPRM (arXiv:2511.08325, WWW 2026) -- step-level process rewards for agent tasks, trained from TD estimation with GAE (§3.3).

The core limitation of binary verdicts: they cannot distinguish "almost
passed" (9/10 tests green, one off-by-one error) from "completely failed"
(does not compile). Both return `Verdict::fail()`.

### 6.1 The Upgrade

Each gate produces a continuous progress score `p in [0.0, 1.0]`
alongside its binary verdict, computed using **Temporal Difference
estimation** combined with **Generalized Advantage Estimation** (GAE):

```
TD_error(t) = r(t) + gamma * V(s_{t+1}) - V(s_t)

A^GAE(t) = sum_{l=0}^{T-t} (gamma * lambda)^l * TD_error(t+l)
```

Parameters: `gamma = 0.99` (discount), `lambda = 0.95` (GAE smoothing).

### 6.2 Gate-Specific Progress Scores

| Gate | Progress score formula |
|------|----------------------|
| CompileGate | `1.0 - (error_count / max_errors).min(1.0)` |
| TestGate | `tests_passed / tests_total` |
| ClippyGate | `1.0 - (warning_count / max_warnings).min(1.0)` |
| SymbolGate | `symbols_found / symbols_expected` |
| DiffGate | `substantive_lines / expected_lines` (capped at 1.0) |
| LlmJudgeGate | Judge's continuous quality score |

### 6.3 What Continuous Scores Enable

1. **Partial-success replanning.** Instead of "try again," the system says
   "you are 80% there -- the failure is in module X."
2. **8x compute efficiency.** Per-step scoring identifies the exact failure
   point, reducing verification rollouts needed.
3. **Test-time compute scaling.** Near-passing attempts get focused
   verification; far-from-passing attempts get terminated early.

---

## 7. Self-Supervised PRM Training

Roko generates its own step-level training labels. The gate pipeline is
a deterministic oracle -- every intermediate artifact can be verified.

### 7.1 The Self-Supervision Loop

```
Agent execution trace:
  step_1: read file -> artifact_1 (no code change)
  step_2: edit file -> artifact_2 (code changed)
  step_3: edit file -> artifact_3 (code changed)
  step_5: fix test  -> artifact_5 (code changed)

Gate verification of each intermediate artifact:
  artifact_2: compile PASS, test FAIL -> partial credit (0.5)
  artifact_3: compile PASS, test PASS 8/10 -> more credit (0.7)
  artifact_5: compile PASS, test PASS 10/10 -> full credit (1.0)
```

### 7.2 Monte Carlo Step-Level Q-Values

For richer labels, estimate probability that continuing from a state leads
to eventual gate passage:

```
q_value = successes / num_rollouts
```

With 8 rollouts per step and ~5 code-modifying steps per task: 40
additional agent turns + 40 gate evaluations. At Haiku-tier costs, the
total labeling cost is ~$0.04 per task.

---

## 8. Potential-Based Reward Shaping

> **Citation:** Ng et al., "Policy Invariance Under Reward Transformations"
> (ICML 1999) -- potential-based reward shaping preserves optimal policy.

Raw gate verdicts are sparse. Reward shaping fills gaps with dense signals:

```
R'(s, a, s') = R(s, a, s') + gamma * Phi(s') - Phi(s)
```

Where `Phi` is the potential function:

```
Phi(state) = w_compile * compile_status
           + w_test    * test_pass_rate
           + w_lint    * lint_cleanliness
           + w_complete * (1 - stub_fraction)
```

Default weights: `w_compile = 0.4, w_test = 0.3, w_lint = 0.1,
w_complete = 0.2`.

### Shaping Signal Interpretation

```
Step: agent adds use statement (fixes compile error)
  Phi(prev) = 0.0 (doesn't compile)
  Phi(next) = 0.4 (compiles, no tests pass yet)
  Shaped reward: +0.396 (positive: progress)

Step: agent deletes a test (vacuous pass)
  Phi(prev) = 0.7 (compiles, 7/10 tests pass)
  Phi(next) = 0.6 (compiles, 7/7 pass but completeness drops)
  Shaped reward: -0.106 (negative: regression)

Step: agent reads a file (no code change)
  Phi(prev) = 0.5, Phi(next) = 0.5
  Shaped reward: -0.005 (near zero: encourages efficiency)
```

Potential-based shaping naturally penalizes vacuous changes and rewards
genuine progress, without hand-coded rules.

---

## 9. Implementation: StepVerdict and TurnSnapshot

```rust
pub struct ProcessRewardModel {
    // Weights for promise scoring
    // Weights for progress scoring
    // Historical turn data for delta computation
}

pub struct StepVerdict {
    pub step_index: usize,
    pub promise: f64,
    pub progress: f64,
    pub combined: f64,
}

pub struct TurnSnapshot {
    pub highest_rung: u32,
    pub test_pass_rate: f64,
    pub error_count: u32,
    pub tool_calls: u32,
    pub useful_tool_calls: u32,
}
```

---

## 10. Academic Foundations

### Lightman et al. (2023) -- PRM800K

Process supervision outperforms outcome supervision for mathematical
reasoning. Best-of-N selection with process rewards outperformed majority
voting by 8%.

### AgentPRM (Xi et al. 2026)

Extended process rewards to agent tool-use settings. Step rewards capture each action's promise and progress toward the goal (§3.2). Key insight: not all tool calls contribute
equally to the outcome.

### Self-Refine (Madaan et al. 2023)

LLMs improve outputs through iterative refinement with feedback. Process
rewards formalize the feedback signal.

### Reflexion (Shinn et al. 2023)

Verbal reinforcement for agents: "Your last 3 attempts reached Rung 1 but
failed at Rung 2. Test failures all in auth module. Focus on auth tests."

---

## 11. Integration with Gate Dispatch

The runner's `gate_dispatch.rs` emits `GateCompletion` events that include
verdict summaries. The process reward model consumes these to compute
Promise and Progress scores, which feed back into:

- Early termination decisions
- Retry budget adjustments
- Replan triggers
- Cascade router model selection

---

## Verification

```bash
cargo test -p roko-gate -- process_reward
```
