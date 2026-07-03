# Roko Trustworthy Batches

The batches are ordered so Roko becomes safe enough to run future Roko-agent implementation waves. The first objective is not feature breadth. It is verified self-hosting.

| Batch | Group | Dependencies | Purpose |
| --- | --- | --- | --- |
| RT00 | gate | none | Define the minimum done gate, parity ledger contract, and fail-closed acceptance shape. |
| RT01 | kernel | RT00 | Parse review verdicts structurally and make missing/ambiguous review output fail closed. |
| RT02 | kernel | RT00 | Classify compile errors and run deterministic cargo-fix paths before spawning agents. |
| RT03 | kernel | RT01 RT02 | Share gate failure patterns across parallel agents and targeted retries. |
| RT04 | kernel | RT03 | Add post-gate reflection and promote repeated lessons into playbook rules. |
| RT05 | kernel | RT00 | Scope context injection and prevent unbounded prompt/context leakage. |
| RT06 | kernel | RT00 | Add warm agent spawning/reuse and stronger interruption-safe resume behavior. |
| RT07 | kernel | RT03 RT04 | Replan when gates fail, instead of blindly rerunning the same batch. |
| RT08 | kernel | RT00 | Persist provider/model pass-rate outcomes and reward telemetry. |
| RT09 | kernel | RT08 | Add A-MAC style knowledge admission and anti-knowledge handling. |
| RT10 | policy | RT00 | Remove live Mori/Bardo prompt path leakage from Roko roles. |
| RT11 | policy | RT10 | Introduce `RoleProfile` and `PromptPolicy` manifest contracts. |
| RT12 | policy | RT11 | Load built-in roles such as architect, implementer, and scribe from manifests. |
| RT13 | policy | RT11 | Add a `CognitiveWorkspace` audit object with provenance and budget accounting. |
| RT14 | policy | RT13 | Register context bidders with cold-start static policies. |
| RT15 | policy | RT13 | Persist prompt/context section metadata and gate outcomes. |
| RT16 | policy | RT14 RT15 | Wire `LearningBidder` posteriors into prompt/context composition. |
| RT17 | policy | RT08 RT16 | Use contextual bandits for model, routing, and context decisions. |
| RT18 | runtime | RT00 RT06 | Make runtime/control durable enough for long runs and observation. |
| RT19 | selfhost | RT01 RT02 RT03 RT04 RT05 RT06 RT07 RT08 RT10 RT11 RT13 RT15 | Finish the end-to-end self-hosting loop. |
| RT20 | core | RT19 | Create the architecture implementation queue that Roko can execute itself. |
| RT21 | parity | RT20 | Enforce docs parity only after the runtime can prove completion. |
| RT22 | dashboard | RT18 RT20 | Make dashboard/product surfaces consume stable Roko projections. |
| RT23 | advanced | RT20 RT21 | Defer chain/economy/advanced surfaces into gated future packets. |

## Why This Order

Acceptance gates come before implementation. Otherwise agents can mark tasks done without proof.

The execution kernel comes before configurable roles. Otherwise role/prompt refactors still run through brittle orchestration that cannot supervise failure.

Configurable roles, prompts, and context come before adaptive learning. Otherwise the system collects outcomes it cannot assign to meaningful policy choices.

Bandits come after telemetry. Bandits need observations, rewards, and stable action identifiers before they can improve anything.

Runtime durability comes before broad self-hosting. Long runs need restart, resume, cancellation, observability, and consistent configuration.

Dashboard and product surfaces come late. They should consume stable Roko projections, not force the backend architecture.

## Running by Phase

```bash
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --only RT00
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group kernel
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group policy
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group runtime
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group selfhost
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group core
```

Use `--max-batches` for shorter supervised windows:

```bash
bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group kernel --max-batches 2
```
