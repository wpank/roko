# 35 - Current State Checklist

Purpose: give future agents a no-chat-context checklist for what is done, what
is only partially done, and what is still an issue after the `05-01` audit,
the agent-ready packet docs, and the follow-up dispatch/chat/serve work.

Last updated: 2026-05-01.

## Status Rules

- `[x]` means the item landed in the current worktree and has focused
  verification recorded in `34-agent-packet-execution-status.md`.
- `[ ] Partial` means useful groundwork landed, but the original design problem
  is still not fixed end to end.
- `[ ] Open` means there is no verified fix in the current worktree.
- Do not mark a design item complete just because a type, adapter, or one route
  exists. Product paths and deletion of old paths must be verified separately.

## Current Snapshot

| Area | Current state | Main remaining risk |
|---|---|---|
| ACP wire blockers | Mostly fixed. Content blocks, session/update payloads, typed stream failures, and ACP provider streaming have focused tests. | ACP product-path proof is still narrower than the audit surface. |
| Dispatch/model-call | Partial. Core plan/stream types exist; ACP, CLI chat API dispatch, and serve provider-test now use shared model-call paths. | Legacy `dispatch_direct`, serve routes, compatibility adapters, and provider auth/capability validation remain. |
| Config/safety | Partial. Root dangerous skip was removed; strict validators and local override types exist. | Validators are not wired into every load path; permissive safety fallbacks and bind defaults remain. |
| Runtime/gates/artifacts | Partial. `CommitOutcome`, `RunLedger`, `GateStatus`, `GateRegistry`, and `ArtifactOutcome` adapters exist. | Reports, gates, artifacts, resume, and event persistence are not fully ledger-first. |
| Telemetry/learning | Partial. `UsageObservation` exists and OpenAI-compatible parsing preserves unknown-vs-zero in some consumers. | Legacy `Usage` conversions and many provider/learning paths still collapse or synthesize zeros. |
| Terminal/demo | Partial. A typed `CommandEvent` DTO and one terminal lifecycle path exist. | Demo automation still needs explicit command/result truth instead of scraping and polling. |
| Enforcement | Open. Inventory scripts exist. | They are not blocking CI gates and allowlists are not reviewed as policy. |

## Crossed-Off Completed Packet Checklist

These are the packet-sized changes that are done and verified. The text after
each item is the boundary that must not be overclaimed.

### A0 - Guardrails And ACP Wire

- [x] ~~A0-1 inventory fitness script.~~ `scripts/roko-fitness-checks.sh` runs; it is inventory-only, not CI-blocking.
- [x] ~~A0-2 docs status script.~~ `scripts/docs-status-check.sh` runs; it inventories stale claims but does not rewrite docs.
- [x] ~~A0-3 ACP ContentBlock wire compatibility.~~ Outbound JSON is canonical `"type": "text"` and inbound `"content"` remains tolerated.
- [x] ~~A0-4 ACP `session/update` payload shape.~~ `send_session_update` emits a flat payload.
- [x] ~~A0-5 ACP stream failures become typed failures.~~ Model stream failures can surface failed tool-call updates instead of normal completion.
- [x] ~~A0-6 root shared config no longer enables dangerous permission skip.~~ `roko.toml` no longer has `dangerously_skip_permissions = true`.

### D - Dispatch And Streaming

- [x] ~~D1 shared `DispatchPlan` skeleton.~~ Type exists in `roko-core`; runtime behavior is not unified by this alone.
- [x] ~~D2 CLI runner-local dispatch plan renamed.~~ Local `DispatchPlan` collision removed by `RunnerDispatchPlan`.
- [x] ~~D3 resolver wrapper.~~ `DispatchResolver::resolve_existing` can project current selection into a shared plan with `Unvalidated` diagnostics.
- [x] ~~D4 shared model stream event contract.~~ `ModelStreamEvent`, boxed stream, and call-to-stream/failure adapters exist.
- [x] ~~D5 unknown-vs-zero OpenAI-compatible usage parsing.~~ Absent usage is distinct from provider-reported zero in the proven parser.
- [x] ~~D6 ACP provider streaming through `ModelCallService::stream`.~~ ACP Anthropic/Claude and OpenAI-compatible paths route through shared stream events; wrapper function names remain.
- [x] ~~D7 block silent `dispatch_direct` fallback from chat init failures.~~ The fallback is refused; `dispatch_direct` still exists.
- [x] ~~D8 CLI chat API dispatch through `ModelCallService::stream`.~~ `ChatAgentSession::send_turn_api` no longer owns raw provider HTTP and preserves role boundaries in shared prompt rendering.
- [x] ~~D9 serve provider-test route through `ModelCallService::call`.~~ `POST /api/providers/{id}/test` uses the shared model-call service; other serve dispatch surfaces remain.

### R - Runtime, Gates, Artifacts, Terminal

- [x] ~~R1 typed `CommitOutcome`.~~ `Created`, `NoChanges`, `Rejected`, and `Failed` exist with compatibility adapters.
- [x] ~~R2 clean-tree commit no longer reports fake runtime `"noop"` hash success.~~ It records `CommitOutcome::NoChanges`.
- [x] ~~R3 `RunLedger` skeleton.~~ Ledger and compatibility report adapter exist.
- [x] ~~R4 main run report can be derived from ledger.~~ `run_with_cancel` creates a ledger and records typed cancellation requests.
- [x] ~~R5 typed `GateStatus`.~~ Status enum and compatibility conversion exist.
- [x] ~~R6 `GateRegistry` alias/rung map.~~ Shared registry exists.
- [x] ~~R7 one runtime rung lookup uses `GateRegistry`.~~ One duplicate map was replaced.
- [x] ~~R8 `ArtifactOutcome` adapter.~~ Artifact outcome adapter exists for PRD generation.
- [x] ~~R9 `CommandEvent` DTOs plus one terminal lifecycle path.~~ Started/output/exited/spawn-failed/cancelled events are typed in one serve terminal path.

### C - Config, Safety, Telemetry, Learning

- [x] ~~C1 config provenance structs.~~ Resolved/validated config ownership types exist.
- [x] ~~C2 provider/model identity structs.~~ Transport/auth/model identity types exist.
- [x] ~~C3 strict raw-TOML validator for dangerous root skip.~~ Shared config with root skip is rejected by the validator.
- [x] ~~C4 local dangerous permission override type.~~ Reason, scope, expiry, source, and acknowledgement env are validated.
- [x] ~~C5 `UsageObservation` in `roko-core`.~~ Canonical optional usage telemetry exists and is re-exported.
- [x] ~~C6 confidence-only router API.~~ Confidence-only updates use `record_confidence_outcome` in migrated callers.
- [x] ~~C7 override learning without real context records confidence-only outcomes.~~ Fake contextual routing is avoided for that path.
- [x] ~~C8 OpenAI and Perplexity usage consumers preserve canonical `UsageObservation`.~~ This proves the parser/consumer pattern, not every provider.
- [x] ~~C9 `roko config doctor` skeleton.~~ Doctor prints status without mutating config.

## Original Priority Fixes: Current State

This section restates the 31 priority fixes from `00-ACP-AUDIT-INDEX.md` with
current state. A checked item here means the priority itself is complete enough
to stop assigning agents to it as a standalone issue. Partial items remain
unchecked.

### Tier 1 - Blocking ACP Functionality

- [x] ~~1. ACP ContentBlock wire compatibility.~~ Done by A0-3; verify with `cargo test -p roko-acp content_block`.
- [x] ~~2. ACP `send_session_update` double-nesting.~~ Done by A0-4; verify with `cargo test -p roko-acp send_session_update`.
- [ ] Partial 3. Conversation history into provider-native messages. Shared `ModelCallRequest.messages` exists and prompt rendering preserves user/assistant role boundaries, but provider-native arrays are not complete for every adapter.
- [x] ~~4. Remove ACP-local raw Anthropic/OpenAI-compatible streaming.~~ Done for ACP provider streaming by D6; wrapper functions remain but route through `ModelCallService::stream`.
- [ ] Partial 5. Stop `ProviderKind::ClaudeCli` from silently becoming Anthropic API. ACP uses shared model-call config now, but provider auth/capability validation is still `Unvalidated` and must fail clearly instead of synthesizing an API route.

### Tier 2 - Security

- [ ] Partial 6. Execute PE_02 and remove dangerous permission defaults/sites. Root `roko.toml` was fixed and validators exist; hardcoded bypass and permissive production sites remain.
- [ ] Open 7. Fix path traversal in agent creation. Needs a path canonicalization and workspace-boundary packet in serve agent routes.
- [ ] Open 8. Fix TOML injection in agent manifest generation. Needs structured TOML serialization, not string interpolation.
- [ ] Open 9. Change safety fallback from permissive to restricted. Multiple `permissive()` fallback sites still exist in safety and serve middleware paths.
- [ ] Open 10. Bind agent server to `127.0.0.1` by default. `0.0.0.0` defaults still appear in serve/CLI initialization paths.

### Tier 3 - Stability

- [ ] Open 11. Add timeout to spawned plan execution. No verified timeout wrapper exists for the audited plan execution path.
- [ ] Open 12. Make `/model` all-or-nothing. Chat dispatch migration landed, but model switch state mutation still needs an atomic transaction packet.
- [ ] Partial 13. Terminal spawn failures close with typed WebSocket errors. Typed command events exist for one lifecycle; all terminal/WebSocket consumers and demo UI are not migrated.
- [ ] Open 14. Set supervision default to `max_restarts: 3`. `max_restarts: 0` remains in process defaults.
- [ ] Partial 15. Close CascadeRouter feedback loop. Confidence-only APIs prevent fake context in some paths, but real `RoutingContext` is not threaded through dispatch.
- [ ] Open 16. Add episode/efficiency JSONL rotation. No verified rotation packet is recorded.

### Tier 4 - Architecture

- [ ] Open 17. Extract `dispatch_agent_with`. The large orchestrate path still needs mechanical extraction into focused functions.
- [ ] Partial 18. Add CI fitness functions. Inventory scripts exist, but they are not blocking CI and do not enforce no-new-violations yet.
- [ ] Open 19. Delete unexported/dead learning modules. The audit identified dead modules; no deletion packet is recorded.
- [ ] Open 20. Wire playbook store results into system prompt builder. No verified prompt-builder consumption packet is recorded.
- [ ] Open 21. Replace demo prompt scraping with explicit command/result markers. Command DTO groundwork exists, but demo automation still needs migration.

### Tier 5 - Redesign Failed Abstractions

- [ ] Partial 22. Single `DispatchPlan` contract across ACP/chat/CLI/serve. Skeleton and projections exist; execution is not fully routed through it.
- [ ] Partial 23. Shared streaming API and no surface-level provider HTTP/SSE. ACP and CLI chat API dispatch are migrated; serve and legacy/direct paths still contain surface-local clients.
- [ ] Partial 24. Replace event-replay-derived workflow reports with `RunLedger`. One main workflow path uses a ledger; full gate/artifact/event/resume truth is not migrated.
- [x] ~~25. Split commit results into typed outcomes.~~ Done by R1/R2; compatibility adapters remain for legacy inputs.
- [ ] Partial 26. Preserve unknown usage as optional telemetry everywhere. Core type and some providers are migrated; legacy `Usage` and many consumers still zero-fill.
- [ ] Partial 27. Require real `RoutingContext` for contextual router learning. Confidence-only paths are labeled, but real context is not fully threaded.
- [ ] Partial 28. Replace gate booleans/maps with `GateStatus` and `GateRegistry`. Types and one lookup are migrated; all gate/report/event paths are not.
- [ ] Partial 29. Make artifact validity a workflow outcome. Adapter exists; workflow behavior is not fully artifact-outcome-driven.
- [ ] Partial 30. Collapse CLI/core config into one validated versioned model. Provenance/identity types exist; broad config loading is not migrated.
- [ ] Partial 31. Move dangerous permission bypasses into explicit local-only overrides. Override type exists; production paths do not require it yet.

## Still-Issue Checklist By Subsystem

Use these as the backlog. Each item should become a small mechanical agent
packet with one owner, a narrow write set, a static recurrence check, and a
focused verification command.

### Enforcement And CI

- [ ] Open: Promote `scripts/roko-fitness-checks.sh` from inventory to no-new-violations CI. Acceptance: CI job fails on a new raw provider HTTP site, dangerous permission default, or oversized function outside an allowlist.
- [ ] Open: Promote `scripts/docs-status-check.sh` into a stale-claim guard. Acceptance: docs that claim resolved/completed without matching current status fail until updated or allowlisted.
- [ ] Open: Add reviewed allowlist ownership. Acceptance: every allowlist entry has owner, reason, expiry/review date, and a linked migration packet.
- [ ] Open: Add product-path proof manifests for ACP/chat/serve. Acceptance: each migrated route has a runnable command or integration test, not just unit tests.

### Dispatch And Provider Execution

- [ ] Open: Add provider auth/capability validation to `DispatchResolver`. Acceptance: unsupported provider/model/auth combinations fail before mutation or network dispatch with typed diagnostics, not fallback.
- [ ] Open: Remove or hard-quarantine `dispatch_direct`. Acceptance: production chat/unified paths cannot call it; static check allows only tests/deprecation module.
- [ ] Open: Migrate remaining serve dispatch/template/agent surfaces to `ModelCallService`. Acceptance: no route-local raw provider request construction for model execution.
- [ ] Open: Provider-native structured history for all adapters. Acceptance: adapters receive role-preserving messages where supported; prompt rendering remains a compatibility fallback only.
- [ ] Open: Thread per-request generation settings into `ModelCallService`. Acceptance: max tokens, temperature, and timeout come from model/profile/request config instead of hardcoded defaults.
- [ ] Open: Replace model/provider synthesis fallbacks with explicit resolution errors. Acceptance: missing providers/models produce typed errors and do not invent provider configs.

### ACP And Chat State

- [ ] Open: Make chat `/model` switch atomic. Acceptance: failed model resolution/auth does not mutate session model, config, adapter, history, or display state.
- [ ] Open: Add ACP end-to-end transcript proof. Acceptance: `session/update`, text deltas, typed failures, and final completion are asserted against a mocked model stream.
- [ ] Open: Verify Claude CLI vs Anthropic API identity. Acceptance: selecting CLI-backed Claude does not require `ANTHROPIC_API_KEY` unless the resolved provider is actually Anthropic API.
- [ ] Open: Remove stale/raw dispatch wrappers or rename them to shared-stream wrappers. Acceptance: static checks no longer flag misleading `run_anthropic_cognitive_task` as local HTTP/SSE.

### Config And Safety

- [ ] Open: Wire strict config validation into normal config load paths. Acceptance: shared config with dangerous root skip fails before runtime, not only in doctor/tests.
- [ ] Open: Require explicit local-only dangerous permission overrides. Acceptance: bypass requires reason, scope, expiry, source, acknowledgement env, and local config path.
- [ ] Open: Replace permissive safety fallback with restricted failure. Acceptance: missing/invalid safety config cannot grant broader permissions than default.
- [ ] Open: Fix path traversal in serve agent creation. Acceptance: paths canonicalize inside workspace/config roots and reject escape attempts with tests.
- [ ] Open: Fix TOML injection in generated agent manifests. Acceptance: structured serializer tests cover quotes, newlines, and table injection attempts.
- [ ] Open: Default local server bind to loopback except explicit deploy mode. Acceptance: init/config/serve defaults are `127.0.0.1`; deploy mode requires explicit opt-in.
- [ ] Open: Make CORS restrictive by default. Acceptance: wildcard/permissive CORS is only allowed in explicit local-dev mode.

### Runtime, Ledger, Gates, Artifacts

- [ ] Open: Move gate verdicts into `RunLedger`. Acceptance: workflow reports read gate status from ledger entries, not replayed string events.
- [ ] Open: Move artifact validity into `ArtifactOutcome` and block success when required artifacts are invalid. Acceptance: invalid required artifact cannot coexist with successful workflow outcome.
- [ ] Open: Persist event/ledger writes fail-closed where correctness depends on them. Acceptance: write failures are surfaced in outcome/status instead of logged and ignored.
- [ ] Open: Add resume/checkpoint ledger ownership. Acceptance: resume reconstructs typed state from ledger/checkpoint, not partial global events.
- [ ] Open: Add timeout and cancellation result semantics for spawned plan execution. Acceptance: timeout records a typed cancelled/failed outcome and cleans up child work.
- [ ] Open: Finish gate registry migration. Acceptance: duplicate gate/rung maps are removed or proven compatibility-only with static checks.

### Telemetry And Learning

- [ ] Open: Migrate all provider parsers/consumers to `UsageObservation`. Acceptance: absent usage remains `None`; explicit provider zero remains zero; no parser invents zero cost/tokens.
- [ ] Open: Stop legacy learning/runtime feedback from treating unknown usage as zero. Acceptance: unknown telemetry is recorded as unknown through episode, cost, and routing feedback records.
- [ ] Open: Thread real `RoutingContext` from dispatch into learning. Acceptance: contextual router updates require model, provider, task/rung/source, and selection reason.
- [ ] Open: Delete or quarantine dead learning modules. Acceptance: unexported/write-only modules are removed or marked experimental behind an explicit feature.
- [ ] Open: Wire playbook store results into prompt construction. Acceptance: retrieved playbook entries affect the system prompt or are not recorded as a live learning loop.
- [ ] Open: Add JSONL rotation/compaction for learning data. Acceptance: episode/efficiency files rotate or compact with tests for size boundaries.

### Terminal, Demo, Serve UI

- [ ] Open: Migrate demo automation off prompt scraping. Acceptance: scenario runners consume typed command/result events, not regex prompt detection.
- [ ] Open: Ensure terminal spawn failure closes with typed WebSocket error in every terminal path. Acceptance: client sees a typed error and no leaked session remains.
- [ ] Open: Clean up temporary terminal shell state. Acceptance: temp `ZDOTDIR` or equivalent resources are deleted on normal exit, failure, and cancellation.
- [ ] Open: Replace generation-counter reset hacks with typed lifecycle state. Acceptance: reconnect/refresh cannot mix old and new terminal output.
- [ ] Open: Add terminal/session auth isolation and rate limits. Acceptance: arbitrary session IDs cannot attach across users/tokens; abusive create/resize/send loops are bounded.

### Orchestrate/God-File Cleanup

- [ ] Open: Extract `dispatch_agent_with` in mechanical slices. Acceptance: each slice moves one cohesive block behind a typed helper with tests and no behavior change.
- [ ] Open: Replace string backend/model dispatch with typed enums/contracts. Acceptance: new branches cannot depend on string prefix checks for provider identity.
- [ ] Open: Remove parameter explosion via request structs. Acceptance: high-arity orchestration helpers accept typed request/context structs and tests cover conversion.
- [ ] Open: Replace broad unwrap/panic paths with typed errors. Acceptance: audited dispatch/session/terminal paths return errors that can be reported to users.

## Agent-Ready Next Packets

The next packets are intentionally mechanical. Give each one to a low-tier
agent with the acceptance criteria and "do not do" text included.

### Packet P1 - CI Promotion For Fitness Inventory

- Scope: `scripts/roko-fitness-checks.sh`, `scripts/fitness/allowlist.toml`, CI workflow/config only.
- Mechanical change: add a no-new-violations mode that compares current findings to the allowlist and exits nonzero for new findings.
- Acceptance: adding a fake raw provider HTTP site in a temp fixture makes the script fail; current repo passes with reviewed allowlist entries.
- Do not do: do not hide findings with broad regex exclusions; do not fix product code in this packet.

### Packet P2 - Dispatch Resolver Validation

- Scope: `crates/roko-agent/src/dispatch_resolver.rs` and focused tests.
- Mechanical change: replace `Unvalidated` diagnostics with typed auth/capability checks using existing provider/model identity structs.
- Acceptance: missing API key, wrong provider kind, unsupported stream capability, and unknown model produce typed errors before dispatch.
- Do not do: do not add raw HTTP; do not synthesize fallback provider configs.

### Packet P3 - Chat `/model` Atomic Switch

- Scope: `crates/roko-cli/src/chat_session.rs` and tests around model switching.
- Mechanical change: resolve, validate, and build the next model-call config in a temporary value; commit all session fields only after success.
- Acceptance: failed switch leaves previous model/config/history/display state unchanged.
- Do not do: do not weaken auth checks; do not mutate then rollback by best effort.

### Packet P4 - Serve Model Dispatch Surface Migration

- Scope: one serve endpoint at a time, starting with the smallest non-provider-test route that executes a model.
- Mechanical change: replace local model/agent construction with `ModelCallService::call` or `stream`.
- Acceptance: endpoint still returns the same user-facing result and static check shows no new route-local provider HTTP for that endpoint.
- Do not do: do not refactor all serve routes at once; do not alter provider health semantics outside the endpoint.

### Packet P5 - Strict Config Load Wiring

- Scope: normal config load entry points plus tests.
- Mechanical change: call the strict validator during shared config load, before runtime construction.
- Acceptance: `runner.dangerously_skip_permissions = true` in shared config fails in CLI and serve load paths; local override type remains the only bypass route.
- Do not do: do not silently downgrade invalid config to defaults; do not mutate config files.

### Packet P6 - Safety Restricted Fallback

- Scope: safety fallback constructors and focused call sites.
- Mechanical change: replace production permissive fallback with restricted/default-deny fallback and typed diagnostics.
- Acceptance: missing safety contract cannot grant more tools/network/filesystem access than the default.
- Do not do: do not change test-only permissive fixtures except to mark them test-only.

### Packet P7 - Provider Usage Parser Sweep

- Scope: one provider parser and its direct consumers per packet.
- Mechanical change: return/preserve `UsageObservation` with optional input/output/total/cost.
- Acceptance: tests distinguish absent usage from explicit zero for that provider and its consumer record.
- Do not do: do not change pricing math or route selection in the same packet.

### Packet P8 - Gate Ledger Detail Migration

- Scope: one gate result source and `RunLedger` adapter.
- Mechanical change: write typed `GateStatus` entries into the ledger and read report output from those entries.
- Acceptance: pass/fail/skipped/error statuses round-trip through ledger and report compatibility output.
- Do not do: do not rewrite all gates at once; do not use string status sentinels for new state.

### Packet P9 - Demo Explicit Command Events

- Scope: demo terminal/scenario runner event consumption.
- Mechanical change: consume `CommandEvent` lifecycle events instead of regex prompt scraping for one scenario.
- Acceptance: scenario success/failure comes from typed command exit/result, not visible prompt text.
- Do not do: do not add more polling or prompt regexes.

### Packet P10 - Agent Manifest Structured TOML

- Scope: serve agent manifest creation tests and serializer.
- Mechanical change: replace string interpolation with structured TOML serialization.
- Acceptance: names/prompts containing quotes, newlines, brackets, and table headers serialize safely and cannot inject new tables.
- Do not do: do not loosen allowed path/name validation.

## Do-Not-Do List For Future Agents

- Do not add a fourth provider dispatch path. Improve `ModelCallService`,
  provider adapters, or `DispatchResolver` instead.
- Do not mark a skeleton type as a completed migration.
- Do not convert unknown usage, cost, routing context, or gate status to zero,
  pass, skipped, or success for convenience.
- Do not silently fall back from managed dispatch to `dispatch_direct`.
- Do not make production safety permissive when config/auth/validation fails.
- Do not build TOML, JSON, SSE, or protocol payloads with ad hoc strings when a
  serializer or typed DTO exists.
- Do not use regex prompt scraping as workflow truth.
- Do not claim an endpoint is migrated without a static check proving no local
  raw provider HTTP was added for that endpoint.
- Do not broaden a low-tier packet after it starts; split follow-ups instead.

## Verification Ledger To Reuse

Use `34-agent-packet-execution-status.md` as the source of observed passing
commands. Minimum recheck before changing this checklist:

```sh
bash -n scripts/roko-fitness-checks.sh && bash scripts/roko-fitness-checks.sh
bash -n scripts/docs-status-check.sh && bash scripts/docs-status-check.sh
cargo test -p roko-acp content_block
cargo test -p roko-acp send_session_update
cargo test -p roko-acp failure
cargo test -p roko-agent request_prompt --lib
cargo check -p roko-cli --lib
cargo test -p roko-serve test_provider --lib
cargo check -p roko-serve
git diff --check
```

If an agent cannot run the full set, it must record exactly which commands ran,
which did not run, and why.
