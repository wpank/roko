Status: draft · budget 550 words · owner spec-ce1484

# 6 Verification and measured trust

This section defines what counts as done, what Roko measures about itself and for which loop, and what it refuses
as acceptance.

## 6.1 What counts as done

A task is done when its checks pass: commands written at planning time, run by the harness in the attempt's worktree,
out of the agent's reach. Each attempt settles into exactly one verdict record (`roko.verdict/1`) carrying a learning
label: 1 for a pass, 0 when the agent's own work failed, and empty otherwise.[^6-label] An attempt with no checks, a
forced accept or an infrastructure failure never counts as a success, and a provider outage never counts against a
model. Every learner, from routing to the knowledge store, reads that one label.

## 6.2 What Roko measures

Each measure is defined on verified outcomes, per task resolution (one task after all its attempts).

**Table 6.1.** What Roko measures about itself, and the loop that reads each measure.

| Measure | Definition | Read by |
|---|---|---|
| Verified pass rate | Share of resolutions that pass the task's checks | Learned model router, per model and task kind; bounded controller |
| Cost per verified success | All attributable spend, planning and audits included, over verified successes | Learned model router; bounded controller |
| False-green rate | Share of passes that a stronger, independent check would fail, estimated by random deep audits (§5) | Verification depth and per-model trust; outcome forecaster; bounded controller |
| pass^k^ | Chance that all k independent runs of a task pass [@yao2024taubench] | Forecaster's comparison of routing policies |
| Calibration | Fit between forecast and observed verified success: Brier score [@brier1950], expected calibration error [@guo2017calibration] | Forecaster, which falls back to shadow mode when worse than the base rate |
| Exposure, influence, benefit | Per loop: whether its learned state reached a decision, changed it beyond rerun noise, and improved verified outcomes against a holdout | Loop audit, which keeps the loop or reverts it to its default |
| Latency | 90th-percentile wall time per resolution | Bounded controller |

The first three measures and latency are the bounded controller's essential variables. The false-green estimate
discounts the pass rate, and with it the denominator of cost per verified success. Cost counts every attempt, as agent
evaluations should [@kapoor2024agents], at API-equivalent prices; an unknown cost makes the figure unmeasurable, never
smaller. The instruments are checked too: a census test builds the production dispatcher with the wiring that plan
runs use, and fails whenever a learning component's connection differs from the census it records.

## 6.3 What is not acceptance

Three readily available signals stay out of every sensor.

**Agent self-reports.** A claim to be finished, a summary or a clean exit is not evidence: models struggle to correct
their own reasoning without external feedback [@huang2023large]. Watchers read an agent's output for signs of
trouble; learners read only the learning label.

**LLM judges.** A judge as the acceptance check is rejected because actor and sensor would share failure modes, and
judges show position, verbosity and self-enhancement biases [@zheng2023judging]. Roko's optional judge scores an
attempt only after its checks pass, so it can add a reason to fail but never passes work the checks reject.

**Visible checks alone.** Agents can satisfy a check they can see without doing the work, up to editing the tests
[@krakovna2020specification; @zhong2025impossiblebench], and the difference between visible and held-out results
grows with code size [@zhao2026specbench]. So the pre-verify screen rejects tampered or out-of-scope attempts before
tests run, and a visible pass rate is reported only beside the audited false-green rate.

[^6-label]: `learning_label_for` in `crates/roko-learn/src/telemetry/records.rs` (`1f860d408`).
