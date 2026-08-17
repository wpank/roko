# Native Harness, Cost Tracking, And Verification

## Scope

Use this file for native harness integration, per-tick cost accounting, tool-call safety checks, and end-to-end cognitive-pipeline tests.

## Implementation checklist

- [ ] Define the native harness boundary.
  - It should wrap the existing agent/tool execution path.
  - It must not bypass SafetyLayer or ToolDispatcher where those already exist.
- [ ] Add per-tick cost accounting.
  - record model cost when an LLM is called;
  - record zero-cost or fixed-cost ticks when T0/T1 work completes locally;
  - attribute cost by tier and by task/episode where possible.
- [ ] Wire cost state into the cognitive pipeline.
  - gate can use current burn rate;
  - TUI/dashboard can surface tier cost mix later;
  - cost output format must be stable enough for `roko-learn` consumers.
- [ ] Run a somatic or policy check before every tool call.
  - if the agent is in a stressed/escalating state, lower-risk tool choices should be preferred;
  - hard safety rules still win over cognitive preferences.
- [ ] Make the native harness the default only after the integration path is proven for one backend family.

## Tests required

- [ ] tool call path still reaches the existing safety layer;
- [ ] tier decision affects whether a model call happens;
- [ ] cost totals match the actual execution path in a deterministic test;
- [ ] a native-harness integration test covers at least one full task from input to persisted outcome.

## Build and test commands

- `cargo check -p roko-cli -p roko-learn -p roko-daimon -p roko-chain`
- `cargo test -p roko-learn`
- `cargo test -p roko-cli`
- `cargo test --workspace`

## Verification checklist

- [ ] Cost telemetry can be inspected after a test run.
- [ ] Tool-call safety checks still fire through the native harness path.
- [ ] Tier/cost decisions are observable in structured logs or persisted artifacts.

## Acceptance criteria

- Cost accounting is available per tick and per tier.
- The native harness does not regress safety coverage.
- Cognitive decisions are visible in logs, metrics, or persisted artifacts.
- At least one end-to-end test proves the full pipeline: observe -> gate -> execute -> account -> persist.
