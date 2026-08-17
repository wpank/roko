# Prediction, Gating, And Triage Checklist

## Scope

Use this file for prediction-error computation, habituation, cognitive gating, somatic escalation, and the T0 chain triage path.

## Implementation checklist

- [ ] Define the minimum canonical types.
  - `Observation`
  - prediction-error result type
  - gate decision type
  - triage decision type
- [ ] Identify the real inputs already available in the codebase.
  - recent task outcomes from `roko-learn`
  - provider/latency/cost signals from `roko-learn`
  - affect and somatic state from `roko-daimon`
  - chain novelty/risk signals from `roko-chain/src/triage.rs`
- [ ] Implement prediction-error computation as a composable policy.
  - novelty component
  - uncertainty component
  - failure-history component
  - domain override component
- [ ] Add habituation.
  - repeated low-value or already-understood patterns should decay their escalation score;
  - habituation must be domain-aware so chain anomalies are not muted like repetitive coding logs.
- [ ] Wire somatic escalation.
  - if Daimon reports a strong marker match or high caution/exploration state, the gate can promote the tier;
  - document how much influence somatic state has vs hard safety signals.
- [ ] Define T0 triage for chain work.
  - use `crates/roko-chain/src/triage.rs` as the starting point;
  - make T0 decisions explicit and fast;
  - name which events bypass the LLM completely and which escalate immediately.
- [ ] Put thresholds behind config or policy, not magic constants buried in CLI code.

## Concrete file touchpoints

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-learn/src/active_inference.rs`
- `crates/roko-learn/src/runtime_feedback.rs`
- `crates/roko-learn/src/cost_table.rs`
- `crates/roko-daimon/src/lib.rs`
- `crates/roko-chain/src/triage.rs`

## Verification checklist

- [ ] Unit tests cover low novelty, high novelty, and habituated repeats.
- [ ] Somatic escalation is testable without a live model call.
- [ ] Chain T0 triage can classify at least one benign and one urgent event.
- [ ] Routing decisions explain themselves in logs or structured telemetry.

## Acceptance criteria

- The gate has explicit, inspectable inputs.
- Habituation reduces unnecessary escalation without masking urgent events.
- Daimon can promote or modulate routing in a bounded, documented way.
- T0 chain triage is fast enough to run on every relevant event without LLM involvement.
