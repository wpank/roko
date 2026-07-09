# Cybernetic Policy Primer

The target architecture is not a larger hardcoded prompt. It is a configurable feedback system where roles, prompts, context, routing, and gates are explicit policies with observations.

## RoleProfile

`RoleProfile` should describe a role independently from hardcoded orchestration:

- role id and version;
- objectives;
- responsibilities;
- default prompt policy;
- context policy;
- tool/capability requirements;
- output schema expectations;
- review/gate expectations;
- safety and escalation behavior.

## PromptPolicy

`PromptPolicy` should make prompt composition declarative:

- ordered sections;
- inclusion rules;
- token/cost budgets;
- source/provenance metadata;
- experiment identifiers;
- fallback behavior;
- output schema instructions.

## CognitiveWorkspace

`CognitiveWorkspace` is the audit object for one agent invocation:

- task contract;
- selected role/profile;
- prompt policy;
- context policy;
- included context sections;
- rejected context candidates with reasons;
- model/provider choice;
- tools/capabilities granted;
- output parse results;
- gate outcomes;
- reward observations.

## Context Bidders

Context should be assembled by bidders rather than hand-stuffed in orchestrator code.

Examples:

- task requirement bidder;
- docs/source map bidder;
- failure memory bidder;
- playbook bidder;
- code ownership bidder;
- recent diff bidder;
- learning bidder.

Cold-start policies can be static. Adaptive priors come later after telemetry exists.

## Bandits and Posteriors

Use bandits only after there are stable action identifiers and reward observations.

Initial useful decisions:

- model/provider routing;
- prompt section inclusion;
- context bidder budget share;
- retry strategy;
- reviewer role/model choice.

Suggested early algorithms:

- Beta/Thompson posteriors for binary pass/fail outcomes;
- contextual bandits for model/context decisions with features from task type, crate, gate history, and cost/latency;
- conservative promotion rules before writing winners back to manifests.

## Policy Updates

Learning should emit structured `PolicyUpdate` records. A policy update should identify:

- policy id and version;
- action changed;
- evidence window;
- reward summary;
- safety bounds;
- rollback path;
- whether the update is shadow, candidate, or active.

Do not let one successful run rewrite global policy without admission rules.
