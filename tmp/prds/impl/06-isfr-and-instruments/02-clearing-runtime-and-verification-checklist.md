# Clearing, Runtime Integration, And Verification Checklist

## Scope

Use this file for clearing profiles, cooperative clearing, fallback ladders, and runtime integration.

## Implementation checklist

- [ ] Define `ClearingProfile` separately from market math.
  - profile risk tolerances;
  - allowed actions;
  - one-click or default flow support.
- [ ] Implement cooperative clearing in stages.
  - batch accumulation;
  - solver interface;
  - certificate or KKT verification;
  - fallback ladder when optimization fails.
- [ ] Emit structured outputs for downstream consumers.
  - `ClearingInsight`
  - benchmark updates
  - world/state graph hooks later
- [ ] Integrate with runtime only through event or state boundaries.
  - event fabric;
  - learning model inputs;
  - world-model consumers.
- [ ] Use large-agent clearing scenarios in mirage or deterministic simulations before claiming scale.

## Verification checklist

- [ ] A batch clearing test covers success and solver failure.
- [ ] Fallback ladder is deterministic and documented.
- [ ] Runtime subscribers can consume clearing outputs without importing market internals.
- [ ] End-to-end tests cover index update -> prediction -> clearing -> emitted insight.

## Acceptance criteria

- Clearing is robust to optimizer failure.
- Runtime integration uses explicit events or data contracts.
- Large-batch behavior is proven in simulation, not only described.
