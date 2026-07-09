# Roko-First Roadmap

Bootstrap objective:

Make Roko a trustworthy self-hosting executor first.

Dashboard/product surfaces come after Roko can read docs, generate plans, dispatch agents, assemble context, verify outputs, recover from failures, and record learning signals.

## Phase 0: Define the Done Gate

Start with a narrow subset of `tmp/architecture-plans/08-end-to-end-acceptance.md`.

Minimum gates:

- `cargo check`, `cargo test`, and `cargo clippy` where relevant;
- no stub-pass gates for production paths;
- plan resume works after interruption;
- agent output is parsed structurally;
- failed gates produce retry/reflection/replan signals;
- every implemented doc requirement gets a parity ledger row where the ledger exists.

## Phase 1: Fix the Self-Hosting Execution Kernel

Priority source docs:

- `tmp/architecture-plans/arch-20-orchestrator-gaps.md`
- `tmp/architecture-plans/06-architecture-implementation.md`

Order:

1. Structured review verdict parsing.
2. Compile error classification and cargo-fix pre-agent path.
3. Error pattern sharing across parallel agents.
4. Post-gate reflection loop.
5. Context injection scoping.
6. Warm agent spawning and reuse.
7. Gate failure replanning.
8. Provider/model pass-rate feedback.
9. Reflection-derived playbook rules.
10. A-MAC and knowledge admission rules.

## Phase 2: Fix Roles, Prompts, and Context Before Scaling Agents

Priority source doc:

- `tmp/architecture-plans/generalized-cybernetic-agent-architecture-gaps.md`

Order:

1. Remove live Mori/Bardo prompt path leakage.
2. Add `RoleProfile`.
3. Add `PromptPolicy`.
4. Make architect, implementer, scribe, and similar built-ins load from manifests.
5. Add `CognitiveWorkspace`.
6. Move context injection out of orchestration internals.
7. Register context bidders with cold-start static policies.

## Phase 3: Add Cybernetic Learning

Telemetry comes before bandits.

Order:

1. Persist prompt/context section metadata per invocation.
2. Feed gate outcomes into section-effectiveness tracking.
3. Register `LearningBidder` posteriors into prompt composition.
4. Use contextual bandits for model routing.
5. Use Thompson/Beta posteriors for context bidder priors.
6. Promote winning prompt/model/context experiments into policy manifests.
7. Emit `PolicyUpdate` records.

## Phase 4: Make Runtime and Control Durable

Priority source docs:

- `tmp/architecture-plans/arch-02-agent-runtime.md`
- `tmp/architecture-plans/arch-03-extensions.md`
- `tmp/architecture-plans/arch-04-connectivity.md`
- `tmp/architecture-plans/arch-07-gateway.md`
- `tmp/architecture-plans/arch-16-config.md`
- `tmp/architecture-plans/arch-21-tui-and-operations.md`

Goal:

Agents can run longer, be observed, recover, use config correctly, and expose enough state for operators.

## Phase 5: Finish the Self-Hosting Loop

Priority source doc:

- `tmp/architecture-plans/05-self-hosting.md`

At this point Roko should be able to:

- generate architecture implementation plans;
- assign tasks to agents;
- review outputs;
- retry intelligently;
- update context/prompt policies from outcomes;
- resume after failure;
- prove completion with gates.

## Phase 6: Let Roko Implement Core Architecture

Run `tmp/architecture-plans/06-architecture-implementation.md` as a queue, not one giant wave.

Suggested order:

1. Config/schema and architecture contracts.
2. Agent runtime.
3. Extensions/connectors/feeds.
4. Gateway and model routing.
5. Knowledge/learning/neuro.
6. Gates/evals/arenas.
7. Chain/registries.
8. Groups/coordination.
9. Dashboard projections.
10. Visual composition and authoring.

## Phase 7: Docs Parity

Source:

- `tmp/architecture-plans/07-docs-parity-closure.md`

Do this after the self-hosting kernel. Docs parity is too large to lead with unless Roko already has strong context assembly, gates, retry loops, and ledgers.

## Phase 8: Dashboard and Product Surfaces

Late-phase source docs:

- `tmp/architecture-plans/01-dashboard-resilience.md`
- `tmp/architecture-plans/02-agent-creation.md`
- `tmp/architecture-plans/03-agent-streaming.md`
- `tmp/architecture-plans/04-plan-execution.md`
- `tmp/architecture-plans/dash-prd-*.md`

The dashboard should consume stable Roko projections. It should not force backend architecture before the self-hosting substrate is trustworthy.
