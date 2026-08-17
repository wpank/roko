# 27 - Integrated Redesign Roadmap

Sources: `18-model-dispatch-redesign.md`, `19-workflow-result-state-redesign.md`,
`20-learning-telemetry-redesign.md`, `21-gates-artifact-redesign.md`,
`22-config-schema-redesign.md`, and implementation plans `23`-`26`.

Goal: turn the audit findings into an implementation sequence that fixes root
ownership boundaries first. The work should not proceed as isolated surface patches;
each wave below creates a typed owner, migrates product paths to that owner, and then
blocks recurrence with fitness checks.

## Dependency Order

1. **Stop drift before refactor**: add non-invasive fitness checks and local blockers
   so new raw provider HTTP, dangerous permissions, sentinel successes, and
   unknown-to-zero telemetry cannot grow while redesign work is underway.
2. **Make config and dispatch truth explicit**: build validated config,
   provider/model provenance, permission policy, `RoutingContextStatus`, and
   `DispatchPlan`. Provider streaming should consume this contract, not recreate
   config logic.
3. **Make runtime truth explicit**: add `RunLedger`, typed effect outcomes,
   `CommitOutcome`, `GateStatus`, and `ArtifactOutcome`. Reports/events/learning
   should be projections of typed outcomes, not sources of truth.
4. **Migrate product surfaces**: ACP, chat, one-shot CLI, runner, serve, terminal,
   and demo should become thin adapters over shared dispatch/runtime/gate/command
   owners.
5. **Delete or quarantine bypasses**: remove raw surface provider clients,
   `dispatch_direct` production fallback, duplicate gate/rung maps, prompt scraping,
   no-op success paths, and stale config synthesis.
6. **Require product-path proof**: status docs can only claim `Wired`,
   `LiveInAllProductPaths`, or `ProvenByE2E` when a live entry point exercises the
   path and a recurrence check blocks regression.

## Wave 0 - Compatibility and Guardrails

Purpose: unblock ACP verification and prevent new drift before deep refactors.

Work:

- Fix ACP `ContentBlock::Text` outbound serialization to the external `"text"`
  contract if protocol fixtures confirm it.
- Fix `send_session_update` flattening and add typed ACP dispatch failure events.
- Make `begin_prompt` busy-state mutation atomic before changing streaming internals.
- Add `scripts/roko-fitness-checks.sh` in non-blocking inventory mode with checks from
  `26-enforcement-and-runner-controls.md`.
- Add docs-status vocabulary checks so stale "resolved" claims do not spread.
- Remove or locally quarantine root `dangerously_skip_permissions = true`.

Exit criteria:

- ACP golden serialization tests exist.
- Fitness script reports baseline violations with owners/expiry, and CI can run it.
- No new raw provider HTTP, dangerous permission defaults, or sentinel successes can
  be introduced without appearing in the fitness report.

## Wave 1 - Core Types and Provenance

Purpose: define the contracts that later waves migrate toward.

Work:

- Add `ValidatedConfig`, `ResolvedRuntimeConfig`, `ConfigProvenance`,
  provider/model identity types, `PermissionPolicy`, and local-only dangerous override
  types in `roko-core`.
- Add `DispatchRequest`, `DispatchRequirement`, `DispatchPlan`, `TransportPlan`,
  `FallbackPolicy`, `DispatchAttempt`, and typed dispatch errors.
- Promote `UsageObservation` to core with optional fields and provenance.
- Add `RunLedger`, `EffectOutcome`, `AgentOutcome`, `CommitOutcome`, `GateStatus`,
  and `ArtifactOutcome`.
- Add compatibility adapters from old `TokenUsage`, `GateVerdict`, `PipelineInput`,
  and `WorkflowRunReport` shapes so behavior can migrate incrementally.

Exit criteria:

- New types compile and are unit-tested without changing product behavior.
- Existing reports can be generated through adapters from the new typed shapes.
- No new surface code needs to invent provider/model/gate/artifact status fields.

## Wave 2 - Config, Safety, and Routing Resolver

Purpose: make dispatch impossible without validated config, safety, and provenance.

Work:

- Make core `RokoConfig` the runtime domain model; turn CLI config into a legacy
  parser/overlay adapter.
- Add strict migration and `roko config doctor` showing provider/model provenance,
  safety status, aliases, and dispatch traces.
- Move provider/model synthesis to config migration/defaulting only; remove request-time
  synthesis from provider factory and `ModelCallService`.
- Validate provider id, kind, transport, auth, model alias, backend slug, and fallback
  priority.
- Implement `DispatchResolver::resolve(ResolvedRuntimeConfig, DispatchRequest)`.
- Add `RoutingContextBuilder` and split observations into contextual, confidence-only,
  and dashboard-only paths.
- Make production feedback, safety, permission policy, and budget policy mandatory.

Exit criteria:

- `DispatchPlan` is the only object that authorizes execution.
- Learned routing only runs with `RoutingContextStatus::Available`.
- A hard model override does not silently fallback unless policy explicitly says so.
- Claude CLI never silently becomes Anthropic API.

## Wave 3 - Provider Streaming and ModelCallService Ownership

Purpose: remove the abstraction gap that caused surface raw provider clients.

Work:

- Extend `ModelCaller` with `plan`, `call(plan, req)`, and `stream(plan, req)`.
- Add `ModelStreamEvent` covering content, reasoning, tool calls, usage, attempts,
  completion, failure, and cancellation.
- Implement streaming in provider adapters: Anthropic API, OpenAI-compatible,
  Cerebras/OpenAI-compatible, Claude CLI stream-json, Cursor ACP where supported.
- Normalize provider usage to `UsageObservation`; display zero-fill only in UI adapters.
- Emit requested, attempted, final model/provider, auth/capability, fallback, and usage
  provenance from every model call.

Exit criteria:

- Provider HTTP/SSE parsing exists only under provider/streaming ownership.
- ACP/chat/serve can consume one stream shape without knowing provider wire protocols.
- Dispatch failures emit typed failures, never normal completion with error text.

## Wave 4 - Workflow, Gate, Artifact, and Command Truth

Purpose: make workflow reports, gates, artifacts, and demo command state typed.

Work:

- Switch `WorkflowEngine` to build and return reports from `RunLedger`, not event replay.
- Replace commit string sentinel behavior with `CommitOutcome`.
- Add `roko-gate` registry owning gate ids, aliases, rung, executor, required inputs,
  resource policy, and config validation.
- Replace `passed/skipped` gate booleans with `GateStatus`.
- Convert PRD/plan generation validity into `ArtifactOutcome`.
- Add strict/best-effort JSONL replay modes and persistence health.
- Add typed serve command events for terminal/demo flows: started, output, exited,
  spawn failed, cancelled.

Exit criteria:

- Clean-tree commit is `NoChanges`, not a success hash.
- Required not-wired gates fail preflight config validation.
- Invalid required artifact cannot produce workflow success or positive learning.
- Demo correctness no longer depends on prompt or transcript scraping.

## Wave 5 - Surface Migration

Purpose: make entry points thin adapters over shared owners.

ACP:

- Replace local Anthropic/OpenAI streaming with `ModelCallService::stream`.
- Map `ModelStreamEvent` to ACP `SessionUpdate`.
- Return typed ACP failures for auth/capability/provider errors.

Chat and CLI:

- Replace chat API HTTP and streaming command construction with `DispatchPlan`.
- Make `/model` all-or-nothing.
- Remove production fallback to `dispatch_direct`.
- Treat CLI auth detection as diagnostic evidence, not dispatch authority.

Serve:

- Build shared resolver/service bundle in `AppState`.
- Route provider tests and agent/template dispatch through `DispatchPlan`.
- Map model stream events to SSE/WebSocket responses.

Runner:

- Use gate registry and typed outcomes.
- Require product-path proof manifests for any "wired" claim.

Exit criteria:

- Each product surface has a product-path proof manifest.
- Old raw or legacy path is deleted, blocked, or explicitly legacy-only.
- Product status docs use coverage vocabulary with proof links.

## Wave 6 - Deletion and CI Enforcement

Purpose: remove compatibility debt and make recurrence hard.

Delete or quarantine:

- ACP raw provider HTTP/SSE functions and direct provider env lookups.
- Chat raw API dispatch and hardcoded Claude fallback.
- Production imports/calls to `dispatch_direct`.
- Runtime config/provider/model synthesis after config resolution.
- Duplicate gate/rung maps outside `roko-gate` registry.
- `report_from_events` as a report source of truth.
- `"noop"` commit hash sentinel paths.
- Prompt/terminal scraping for scenario correctness.
- Lossy `UsageObservation -> Usage` conversions in runtime/learning paths.
- `RoutingContext::default()` override learning.

Enable CI blocking:

- raw provider HTTP outside provider adapters;
- dangerous shared permission bypass;
- provider API env reads outside config/auth/provider boundaries;
- unknown-to-zero telemetry in collection paths;
- path-based modules;
- sentinel success/noop outcomes;
- duplicate gate/provider/prompt dispatch owners;
- docs status claims without proof;
- touched function growth above ratcheted limits.

## Workstream Map

| Workstream | Primary Docs | First Owner | Blocks |
|---|---|---|---|
| Config/safety/provenance | 22, 25 | `roko-core`, `roko-cli/config` | Dispatch resolver, safe production runs |
| Dispatch/streaming | 18, 23 | `roko-core`, `roko-agent` | ACP/chat/serve provider cleanup |
| Runtime ledger | 19, 24 | `roko-runtime` | Accurate reports, learning, commit semantics |
| Gates/artifacts | 21, 24 | `roko-gate`, `roko-runtime`, `roko-cli/prd` | Valid workflow success and runner gates |
| Telemetry/learning | 20, 25 | `roko-agent`, `roko-learn` | Real routing feedback and cost/usage truth |
| Terminal/demo truth | 16, 24 | `roko-serve`, demo app | Demo correctness without scraping |
| Enforcement | 17, 26, anti-pattern docs | `scripts/`, CI | Preventing recurrence and bad runner output |

## First Implementation Slice

The most useful first slice should be small enough to land but structural enough
to avoid more patchwork:

1. Add fitness checks in inventory/no-new-violations mode.
2. Fix ACP wire blockers and typed failure event.
3. Add core `DispatchPlan`/`DispatchRequest` skeleton plus `DispatchResolver` stubs
   that wrap existing selection behavior without changing execution yet.
4. Add `UsageObservation` in core and stop one provider parser from converting
   missing usage to zero as a proof of pattern.
5. Add `CommitOutcome` and migrate clean-tree commit handling away from `"noop"`.

This slice gives immediate safety rails, unblocks ACP validation, and creates the
typed seams needed for the larger dispatch/runtime migration.

## Non-Goals

- Do not add another surface-local provider client while waiting for streaming.
- Do not make WorkflowEngine, GateService, or ModelCallService optional in production.
- Do not mark any old anti-pattern as resolved because a type exists; require live
  path migration and old-path retirement.
- Do not broaden runner parallelism until Wave 0 guardrails are at least in
  no-new-violations mode.
