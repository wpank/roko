# Roko Subsystem Audit — 2026-05-01

Comprehensive audit of ACP, orchestration, dispatch, safety, learning, and serve subsystems.
Cross-referenced against 661 runner batches (arch, converge, converge-followup, mega-parity, post-parity).

## Documents

### Round 1: ACP & Integration

| File | Scope | Issues |
|------|-------|--------|
| [01-protocol-serialization.md](01-protocol-serialization.md) | SessionUpdate wire format, ContentBlock rename | 4 |
| [02-bridge-events.md](02-bridge-events.md) | Event mapping, dead code, panics, provider dispatch | 8 |
| [03-session-management.md](03-session-management.md) | Concurrency, busy flag race, history trimming | 6 |
| [04-terminal-serve.md](04-terminal-serve.md) | PTY sessions, WebSocket, session isolation | 9 |
| [05-cli-integration.md](05-cli-integration.md) | Chat session disconnect, provider routing | 5 |
| [06-demo-app.md](06-demo-app.md) | Frontend integration, workspace paths | 7 |
| [07-working-tree-review.md](07-working-tree-review.md) | Uncommitted changes on wp-arch2 | 5 |

### Round 2: Architecture & Runner Damage

| File | Scope | Issues |
|------|-------|--------|
| [08-orchestrate-rs-bloat.md](08-orchestrate-rs-bloat.md) | 22K-line god file, 2059-line function, duplication | 12 |
| [09-safety-bypass.md](09-safety-bypass.md) | 8 permission bypasses, permissive defaults, zero restarts | 6 |
| [10-learning-dead-code.md](10-learning-dead-code.md) | 800 LOC dead, broken feedback loops, write-only data | 9 |
| [11-serve-security.md](11-serve-security.md) | Path traversal, TOML injection, no timeouts, CORS | 11 |
| [12-runner-batch-damage.md](12-runner-batch-damage.md) | Systemic analysis of 661-batch runner impact | Assessment |

### Round 3: Post-Runner Regression Review

| File | Scope | Issues |
|------|-------|--------|
| [13-acp-provider-regression.md](13-acp-provider-regression.md) | ACP provider dispatch, Anthropic streaming, ContentBlock contract | 5 |
| [14-chat-session-model-regression.md](14-chat-session-model-regression.md) | Chat session model switching, auth detection, API dispatch | 5 |
| [15-config-safety-regression.md](15-config-safety-regression.md) | Root config, provider/model schema, permission bypass | 5 |
| [16-terminal-demo-adhoc.md](16-terminal-demo-adhoc.md) | PTY lifecycle, prompt scraping, demo workflow truth | 5 |
| [17-runner-review-gaps.md](17-runner-review-gaps.md) | Runner rule enforcement gaps and required fitness checks | 5 |

### Round 4: Redesign Targets

| File | Scope | Issues |
|------|-------|--------|
| [18-model-dispatch-redesign.md](18-model-dispatch-redesign.md) | One model/provider dispatch contract, streaming, raw dispatch removal | 6 |
| [19-workflow-result-state-redesign.md](19-workflow-result-state-redesign.md) | Workflow report truth, typed effect outcomes, commit/cancel/event durability | 6 |
| [20-learning-telemetry-redesign.md](20-learning-telemetry-redesign.md) | Unknown usage, contextual routing feedback, learning observation provenance | 5 |
| [21-gates-artifact-redesign.md](21-gates-artifact-redesign.md) | Gate status model, gate registry, artifact validity as outcome | 6 |
| [22-config-schema-redesign.md](22-config-schema-redesign.md) | Config schema ownership, provider identity, dangerous local overrides | 6 |

### Round 5: Implementation Redesign Plans

| File | Scope | Issues |
|------|-------|--------|
| [23-dispatch-streaming-migration-plan.md](23-dispatch-streaming-migration-plan.md) | DispatchPlan, shared streaming API, provider fallback/auth/capability migration | Plan |
| [24-runtime-gate-ledger-plan.md](24-runtime-gate-ledger-plan.md) | RunLedger, typed effect outcomes, gate registry, artifact/command truth | Plan |
| [25-config-safety-telemetry-plan.md](25-config-safety-telemetry-plan.md) | Validated config, safety defaults, UsageObservation, RoutingContext learning | Plan |
| [26-enforcement-and-runner-controls.md](26-enforcement-and-runner-controls.md) | CI fitness checks, runner prompt rules, merge/cherry-pick controls | Plan |
| [27-integrated-redesign-roadmap.md](27-integrated-redesign-roadmap.md) | Cross-track dependency order and first implementation slice | Roadmap |

### Round 6: Agent-Ready Mechanical Packets

| File | Scope | Issues |
|------|-------|--------|
| [28-agent-tasking-playbook.md](28-agent-tasking-playbook.md) | Packet format, prompt template, status vocabulary, global anti-patterns | Playbook |
| [29-wave0-guardrails-acp-wire-agent-packets.md](29-wave0-guardrails-acp-wire-agent-packets.md) | Guardrail scripts, ACP wire blockers, dangerous root config | Packets |
| [30-dispatch-streaming-agent-packets.md](30-dispatch-streaming-agent-packets.md) | DispatchPlan, stream event, resolver, usage parser, ACP stream adapter packets | Packets |
| [31-runtime-gate-agent-packets.md](31-runtime-gate-agent-packets.md) | CommitOutcome, RunLedger, GateStatus, registry, artifact, command event packets | Packets |
| [32-config-safety-telemetry-agent-packets.md](32-config-safety-telemetry-agent-packets.md) | Config provenance, provider identity, dangerous overrides, UsageObservation, routing packets | Packets |
| [33-agent-packet-verification-matrix.md](33-agent-packet-verification-matrix.md) | Verification commands, dependencies, runner report format, low-tier exclusions | Matrix |
| [34-agent-packet-execution-status.md](34-agent-packet-execution-status.md) | Multi-agent execution ledger, passed checks, remaining safe packets, known blockers | Status Ledger |
| [35-current-state-checklist.md](35-current-state-checklist.md) | Crossed-off completed packets, original priority state, remaining issue checklist, next agent packets | Checklist |

### Round 7: Deep Audit Follow-Up

| File | Scope | Issues |
|------|-------|--------|
| [36-deep-audit-acp-terminal-safety.md](36-deep-audit-acp-terminal-safety.md) | ACP 5-link failure chain, terminal security + lifecycle, safety fail-open defaults, orchestrate.rs decomposition, cross-cutting CORS/auth/bind issues | 19 prioritized issues with redesign direction |
| [37-learning-feedback-dead-code.md](37-learning-feedback-dead-code.md) | Write-only sinks, dead learn modules, broken feedback loops, facade/legacy duplication, empty AgentOutcome fields | 15 |
| [38-serve-routes-security.md](38-serve-routes-security.md) | Auth disabled by default, SSRF via agent registration, secret leakage, arbitrary terminal commands, no rate limiting | 17 |
| [39-config-schema-phantom-fields.md](39-config-schema-phantom-fields.md) | Phantom config sections, unvalidated config loading, duplicate model slugs, secret masking gaps, strict validator never called | 17 |
| [40-gate-pipeline-dispatch-audit.md](40-gate-pipeline-dispatch-audit.md) | Gate rungs 3-6 dead via catch-all, observe_pipeline/drain_spc_alerts never called, 2059-line dispatch function, gate caps hardcoded false | 9 |
| [41-consolidated-backlog.md](41-consolidated-backlog.md) | 42 items across 6 tiers with mechanical implementation details, acceptance criteria, anti-patterns, and do-not-do rules | Backlog |

## Issue Summary

**Total issues found: ~194 + systemic redesign assessments**

| Severity | Count | Key Examples |
|----------|-------|-------------|
| CRITICAL | 11 | ContentBlock type rename breaks Zed, root `dangerously_skip_permissions = true`, provider dispatch has no single execution contract, workflow reports are inferred from replayed events |
| HIGH | 32 | Raw Anthropic streaming in ACP, chat model state corruption, arbitrary provider/model fallback, noop commit as success, routing feedback without context |
| MEDIUM | 68 | CORS misconfiguration, prompt scraping as demo truth, unknown usage collapsed to zero, duplicate gate rung maps, runtime config synthesis |
| LOW | 25 | Test stubs, config poll intervals, dead learning modules, stale docs, ambiguous display-only diagnostics |

## Root Causes (from 12-runner-batch-damage.md)

1. **661 parallel batches optimized locally, nobody refactored globally** — each batch added 50-200 lines to god-file functions
2. **"Record data" implemented without "use data"** — feedback loops write JSONL but nothing reads it back
3. **"Wired" claims inflated** — components compile and unit-test but don't work end-to-end (CascadeRouter, safety, LLM judge, dreams)
4. **Safety defaults permissive for development convenience** — PE_02 "flip to secure" was planned but never executed
5. **Anti-pattern checks too narrow** — caught trait duplication and dead imports but not function bloat, parameter explosion, or cross-function duplication

## Priority Fixes

### Tier 1: Blocking ACP functionality
1. Revert ContentBlock `"text"` → `"content"` rename (types.rs:361)
2. Fix `send_session_update` double-nesting (bridge_events.rs:2955)
3. Wire conversation history into Anthropic API messages path
4. Remove ACP-local Anthropic streaming; route through provider/model-call layer
5. Stop mapping `ProviderKind::ClaudeCli` to Anthropic API

### Tier 2: Security
6. Execute PE_02: flip all `dangerously_skip_permissions` defaults/sites to false, including `roko.toml:1021`
7. Fix path traversal in agent creation (agents.rs:609)
8. Fix TOML injection in agent manifest (agents.rs:635)
9. Change safety fallback from permissive() to restricted()
10. Bind agent server to 127.0.0.1 not 0.0.0.0

### Tier 3: Stability
11. Add timeout to spawned plan execution
12. Make `/model` all-or-nothing; failed resolution must not mutate only `agent_session.model`
13. Make terminal spawn failures close with a typed WebSocket error
14. Set supervision default to max_restarts: 3
15. Close CascadeRouter feedback loop (populate RoutingContext)
16. Add episode/efficiency JSONL rotation

### Tier 4: Architecture
17. Extract `dispatch_agent_with` (2059 lines) into focused functions
18. Add CI fitness functions: raw provider HTTP, dangerous permissions, env-var access boundaries, max function length
19. Delete 800 LOC of unexported learning modules
20. Wire playbook store results into system prompt builder
21. Replace prompt scraping in demo automation with explicit command/result markers

### Tier 5: Redesign the failed abstractions
22. Introduce a single `DispatchPlan` contract and route ACP/chat/CLI/serve through it
23. Add streaming to the shared model-call/provider API; remove surface-level provider HTTP/SSE clients
24. Replace event-replay-derived workflow reports with a typed `RunLedger`
25. Split commit results into `Created`, `NoChanges`, `Rejected`, and `Failed`
26. Preserve unknown usage as optional telemetry through provider, runtime, feedback, and learning records
27. Require real `RoutingContext` for contextual router learning; label confidence-only updates separately
28. Replace gate pass/skipped booleans with a `GateStatus` enum and shared gate registry
29. Make artifact validity a workflow outcome, not a side field beside process success
30. Collapse CLI/core config into one validated versioned domain model
31. Move dangerous permission bypasses out of shared config and into explicit local-only overrides

## Implementation Sequence

The Round 5 plans order the redesign work as:

1. Add no-new-violations fitness checks and ACP wire compatibility fixes.
2. Land core types: validated config, `DispatchPlan`, `UsageObservation`, `RunLedger`, `GateStatus`, `ArtifactOutcome`, `CommitOutcome`.
3. Make config/safety/provenance and routing context feed dispatch before provider streaming cleanup.
4. Move provider streaming into `roko-agent` and migrate ACP/chat/serve to shared stream events.
5. Move workflow reports/gates/artifacts/terminal demo truth onto typed ledgers and outcomes.
6. Delete bypass paths and turn the fitness checks into blocking CI gates.

Round 6 breaks that sequence into low-tier, no-prior-context work packets. Start
with [28-agent-tasking-playbook.md](28-agent-tasking-playbook.md), then assign
individual packets from docs 29-32 and verify them using
[33-agent-packet-verification-matrix.md](33-agent-packet-verification-matrix.md).
Use [34-agent-packet-execution-status.md](34-agent-packet-execution-status.md)
to avoid redoing completed packets, then use
[35-current-state-checklist.md](35-current-state-checklist.md) to see what is
crossed off, what is partial, and which agent-sized packets remain.
