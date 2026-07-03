# Architecture Plan Map

Agents should read the docs relevant to their batch. This map keeps the source material stable across no-prior-context runs.

## Bootstrap and Acceptance

- `tmp/architecture-plans/00-INDEX.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/05-self-hosting.md`
- `tmp/architecture-plans/arch-20-orchestrator-gaps.md`

## Generalized Roles, Prompts, and Context

- `tmp/architecture-plans/generalized-cybernetic-agent-architecture-gaps.md`
- `tmp/architecture-plans/06-architecture-implementation.md`
- `tmp/architecture-plans/arch-16-config.md`

Expected abstractions:

- `RoleProfile`
- `AgentBlueprint`
- `PromptPolicy`
- `ContextPolicy`
- `CognitiveWorkspace`
- `ContextBidder`
- `LearningBidder`
- `PolicyUpdate`

## Runtime, Extensions, Connectivity, Gateway, Operations

- `tmp/architecture-plans/arch-02-agent-runtime.md`
- `tmp/architecture-plans/arch-03-extensions.md`
- `tmp/architecture-plans/arch-04-connectivity.md`
- `tmp/architecture-plans/arch-07-gateway.md`
- `tmp/architecture-plans/arch-21-tui-and-operations.md`

## Knowledge, Learning, Neuro, Bandits

- `tmp/architecture-plans/generalized-cybernetic-agent-architecture-gaps.md`
- `tmp/architecture-plans/arch-20-orchestrator-gaps.md`
- `tmp/architecture-plans/06-architecture-implementation.md`

Concepts to preserve:

- contextual bandits for model/context/routing choices;
- Thompson/Beta posteriors for binary or bounded success observations;
- reward observations tied to gate outcomes, latency, cost, and retry count;
- A-MAC style memory admission and anti-knowledge handling;
- reflection-derived playbooks promoted only after evidence.

## Docs Parity and Dashboard/Product Work

- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/01-dashboard-resilience.md`
- `tmp/architecture-plans/02-agent-creation.md`
- `tmp/architecture-plans/03-agent-streaming.md`
- `tmp/architecture-plans/04-plan-execution.md`
- `tmp/architecture-plans/dash-prd-*.md`
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd`

These are late consumers of the self-hosting substrate. Pull backend projection requirements forward only when needed by earlier runtime batches.
