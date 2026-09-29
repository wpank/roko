# Evaluation Lifecycle -- 14 Loops, 5 Speed Tiers

> Depth file for [07-GATES.md](../../07-GATES.md) section 11.
> Source: `crates/roko-gate/`, `crates/roko-learn/`, `crates/roko-conductor/`

---

## 1. Overview

Evaluation in Roko is not a single step. It is a lifecycle that spans five
speed tiers, from sub-second machine checks to multi-day retrospective
analysis. The gate pipeline is the fastest tier. Gate verdicts compound,
combine with other signals, and drive progressive improvement across time.

The lifecycle has 14 feedback loops organized across 5 speed tiers. The
output of fast loops feeds into slower loops. Insights from slow loops
adjust parameters of fast loops.

---

## 2. The Five Speed Tiers

| Tier | Speed | Loops | What Runs |
|------|-------|-------|-----------|
| Machine | Sub-second to seconds | 5 | Confidence calibration, context attribution, cost-effectiveness, tool selection, adversarial awareness |
| Cognitive | Seconds to minutes | 3 | Gate pipeline, error diagnosis, retry logic |
| Consolidation | Minutes to hours | 3 | Skill extraction, pattern discovery, model calibration |
| Retrospective | Hours to days | 2 | Shadow testing, reasoning quality review |
| Meta | Days to weeks | 1 | Meta-learning evaluation |

---

## 3. Machine Speed (5 Loops)

These run within or immediately after a single agent turn.

### Loop 1: Confidence Calibration

The agent (or router) predicts success probability before the gate runs.
After the gate runs, the prediction is compared to the outcome. Residuals
accumulate, enabling calibration correction.

Metric: Expected Calibration Error (ECE) -- average gap between predicted
and actual pass rates across confidence bins.

### Loop 2: Context Attribution

Which parts of the prompt contributed to gate success? The section
effectiveness tracker correlates prompt sections with outcomes.

### Loop 3: Cost-Effectiveness

Did the agent's token spend produce proportionate verification results?
A 50,000-token turn that fails all gates is less cost-effective than a
10,000-token turn that passes all gates.

### Loop 4: Tool Selection

Are the agent's tool call patterns efficient? Redundant file reads,
unnecessary edits, and tool calls that do not advance the task are
identified.

### Loop 5: Adversarial Awareness

Does the agent detect adversarial inputs (prompt injections, malicious
test fixtures)? Monitors the agent's defensive behavior.

---

## 4. Cognitive Speed (3 Loops)

These run during a single task execution (across multiple turns).

### Loop 6: Gate Pipeline

The rung selector -> gate pipeline -> verdict cycle. The core verification
loop documented in the gate pipeline depth file.

### Loop 7: Error Diagnosis

Gate error output is parsed into structured feedback via
`feedback_for_agent()` and enriched with cheap-model diagnosis.

### Loop 8: Retry Logic

The orchestrator decides whether to retry, escalate, or re-plan based on
the verdict and process reward signals.

---

## 5. Consolidation Speed (3 Loops)

These run after a batch of tasks (e.g., after full plan execution).

### Loop 9: Skill Extraction

Successful episodes are analyzed to extract reusable tool-use patterns
(see the EvoSkills depth file).

### Loop 10: Pattern Discovery

Cross-task analysis identifies recurring success/failure patterns. Example:
"tasks that modify auth modules fail 3x more often than average."

### Loop 11: Model Calibration

Aggregate per-model performance data to calibrate the router's bandit
arms. Thompson Sampling parameters are updated based on gate outcomes.

---

## 6. Retrospective Speed (2 Loops)

These run on a longer cadence (nightly, weekly).

### Loop 12: Shadow Testing

Run the same tasks with different models/prompts in shadow mode and
compare outcomes. Discovers whether current routing is optimal.

### Loop 13: Reasoning Quality Review

Evaluate agent reasoning quality across completed tasks. Three signals:
alignment (did the agent follow the plan?), consistency (did reasoning
stay coherent?), annotations (useful comments left?).

---

## 7. Meta Speed (1 Loop)

### Loop 14: Meta-Learning Evaluation

Evaluate the evaluation system itself: are the 13 other loops improving
outcomes over time? The system's self-assessment, tracking whether its
learning is net positive.

---

## 8. Composition Diagram

```
Machine Speed                 Cognitive Speed
+-------------+              +-------------+
| Confidence  |--residuals-->| Gate        |
| Calibration |              | Pipeline    |
+-------------+              +------+------+
+-------------+                     |
| Context     |--lift-------+       | verdicts
| Attribution |             |       v
+-------------+             | +----------+
+-------------+             | | Retry    |
| Cost-       |--efficiency-| | Logic    |
| Effectiveness|            | +----------+
+-------------+             |
+-------------+             |  Consolidation Speed
| Tool        |--patterns---+  +--------------+
| Selection   |             +->| Skill        |
+-------------+             |  | Extraction   |
+-------------+             |  +--------------+
| Adversarial |--alerts-----+  +--------------+
| Awareness   |             +->| Pattern      |
+-------------+             |  | Discovery    |
                            |  +--------------+
                            |  +--------------+
                            +->| Model        |
                               | Calibration  |
                               +------+-------+
                                      |
               Retrospective Speed    |
               +--------------+       |
               | Shadow       |<------+
               | Testing      |
               +--------------+
               +--------------+
               | Reasoning    |
               | Quality      |
               +------+-------+
                      |
          Meta Speed  |
          +-----------+--+
          | Meta-Learning |
          | Evaluation    |
          +--------------+
```

---

## 9. The Karpathy Property

Every loop satisfies: **if the evaluation metric improves, the system's
end-to-end performance improves.** This is a design constraint:

- No metric uncorrelated with actual task success
- No metric that can be gamed without improving outcomes
- No metric that improves at the expense of another

For gate-based loops, this is straightforward: if the compile gate pass
rate improves, the system produces better code. For slower loops, the
property requires careful metric design.

---

## 10. The Four-Phase Lifecycle

Beyond speed tiers, evaluation goes through four phases:

### Phase 1: Trace Inspection

Examine individual agent turns. Raw data layer.

Data sources: `.roko/episodes.jsonl`, `.roko/learn/efficiency.jsonl`.

### Phase 2: Backtesting

Replay past executions with different parameters. Would a different model
have succeeded? Would more retries have been optimal?

Data sources: Artifact store, gate threshold history.

### Phase 3: Paper Trading

Run new configurations in shadow mode alongside production.

Data sources: Shadow execution results vs production results.

### Phase 4: Canary Deployment

Gradually roll out improvements to a fraction of tasks, monitoring for
regressions before full deployment.

Data sources: A/B experiment results (`.roko/learn/experiments.json`).

---

## 11. The Gauntlet

Benchmark suite that validates the evaluation lifecycle itself:

| Speed | Duration | Scope |
|-------|----------|-------|
| Smoke | 5 minutes | Core gate pipeline on known test cases |
| Nightly | 2-4 hours | Full rung ladder on real project tasks |
| Full | 24-48 hours | All 14 loops, cross-model comparison |

The "gate for the gates."

---

## 12. Gate Verdicts as Foundation

Every loop is ultimately grounded in gate verdicts. Even the slowest loop
(meta-learning) depends on aggregate gate outcomes.

Improving gate fidelity (adding rungs, reducing false negatives, increasing
coverage) has multiplicative effects across all 14 loops. This is the GVU
framework's insight: invest in verification quality.

---

## 13. Currently Wired Components

| Component | Status |
|-----------|--------|
| Gate pipeline (Loop 6) | Wired |
| Error feedback (Loop 7) | Wired |
| Adaptive thresholds | Wired |
| Efficiency events (Loops 1-5) | Wired |
| Episode logging | Wired |
| Model routing (Loop 11) | Wired |
| A/B experiments | Wired |
| Skill library (Loop 9) | Wired (E25) |
| Shadow testing (Loop 12) | Design |
| Meta-learning (Loop 14) | Design |

---

## Verification

```bash
cargo test -p roko-gate
cargo test -p roko-learn
```
