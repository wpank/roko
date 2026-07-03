# Self-Hosting Gates

The first trustworthy milestone is a narrow, enforceable gate. It should be small enough to implement early and strict enough to prevent agents from falsely completing work.

## Minimum Gate Contract

Each future agent task should have:

- an explicit plan or task contract;
- structured output from the agent;
- a typed review verdict;
- compile/test/lint gates where relevant;
- failure classification;
- retry/reflection/replan behavior;
- a persisted outcome event;
- a parity ledger row for implemented doc requirements where the repo has parity tracking.

## Gate Outcomes

Use explicit outcome states. Avoid free-text-only status.

Suggested states:

- `passed`
- `failed`
- `blocked`
- `timed_out`
- `cancelled`
- `needs_replan`
- `needs_retry`
- `needs_human`

## Review Verdict Shape

A review verdict should carry enough structured information for the orchestrator to act:

- verdict id;
- batch/task id;
- reviewer role id;
- status;
- confidence;
- blocking findings;
- non-blocking findings;
- required next action;
- evidence links or file paths;
- parsed raw output reference;
- created timestamp.

Ambiguous, unparsable, or missing verdicts should fail closed.

## Failure Classification

Compile/test/lint failures should be classified before retry:

- deterministic syntax/import/type error;
- missing dependency or feature flag;
- flaky external dependency;
- test expectation drift;
- unsafe stub/pass behavior;
- prompt/context insufficiency;
- role/tool permission issue;
- architectural conflict requiring replan.

Deterministic cargo-fix paths should run before spawning an LLM retry where they are safe and scoped.

## Learning Signal

Every gate outcome should be able to feed:

- provider/model pass-rate statistics;
- prompt section effectiveness;
- context bidder posterior updates;
- playbook rule promotion;
- policy update records.
