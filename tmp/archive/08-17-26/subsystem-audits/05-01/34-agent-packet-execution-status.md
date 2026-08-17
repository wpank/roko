# 34 - Agent Packet Execution Status Ledger

Purpose: give future agents a no-chat-context ledger for packets defined in
`28-agent-tasking-playbook.md` through `33-agent-packet-verification-matrix.md`.

Scope note: this ledger records packet-sized implementation and verification status
from the multi-agent execution wave. `Changed` means the mechanical packet landed.
It does not mean the broader subsystem migration is live in every product path.
Skeletons, compatibility adapters, and one-path proofs are called out explicitly.

## Summary

| Track | Packet status | Important boundary |
|---|---|---|
| A0 guardrails and ACP wire | A0-1 through A0-6 changed and verified with focused checks. | Guard scripts are inventory-only, not CI-blocking. |
| Dispatch and streaming | D1 through D9 changed; D4/D5/D6/D7/D8/D9 reverified after resolver/chat/serve fixes. | Dispatch is not fully unified; legacy direct dispatch and some serve/compatibility paths remain. |
| Runtime, gates, artifacts, terminal DTOs | R1 through R9 changed or verified; one terminal lifecycle now constructs typed `CommandEvent`s. | Ledger/gate/artifact types are not complete product-path truth yet. |
| Config, safety, telemetry, learning | C1 through C9 changed and focused checks passed; two OpenAI-compatible production consumers now preserve `UsageObservation`. | Validators/types exist, but broad config loading and all provider parsers are not migrated. |

## Verification Observed Passing

| Command | Packet evidence |
|---|---|
| `bash -n scripts/roko-fitness-checks.sh && bash scripts/roko-fitness-checks.sh` | A0-1 inventory guardrail script runs. |
| `bash -n scripts/docs-status-check.sh && bash scripts/docs-status-check.sh` | A0-2 docs status inventory script runs. |
| `cargo test -p roko-acp content_block` | A0-3 content block wire tests pass. |
| `cargo test -p roko-acp send_session_update` | A0-4 session update payload test passes. |
| `cargo test -p roko-acp failure` | A0-5 typed failure tests pass. |
| `cargo check -p roko-acp` | ACP compiles after A0 and D6 changes. |
| `rg 'dangerously_skip_permissions\s*=\s*true' roko.toml` | A0-6 root config no-match check passes. |
| `cargo test -p roko-core dispatch_plan` | D1 core dispatch plan tests pass. |
| `cargo check -p roko-cli` | D2 runner-local rename compiled during packet verification. |
| `cargo test -p roko-agent dispatch_resolver` | D3 resolver wrapper tests pass. |
| `cargo check -p roko-agent` | D3/D4/D5/C8 checks pass after resolver fix. |
| `cargo test -p roko-core model_stream` | D4 core stream event adapter tests pass. |
| `cargo test -p roko-agent model_stream` | D4 agent stream mapping tests pass. |
| `cargo test -p roko-agent request_prompt --lib` | D8 shared model-call prompt rendering preserves user/assistant role boundaries for multi-turn history. |
| `cargo test -p roko-agent parse_usage` | D5/C8 unknown-vs-zero usage parser tests pass. |
| `cargo test -p roko-agent usage` | C5/D5/C8 usage-related tests pass. |
| `cargo test -p roko-acp model_stream_` | D6 ACP stream mapping tests pass. |
| `cargo test -p roko-acp --test telemetry_integration` | D6 ACP OpenAI-compatible provider stream migration tests pass. |
| `cargo test -p roko-cli dispatch --lib` | D7 chat inline fallback block tests pass. |
| `cargo check -p roko-cli --lib` | D8 chat API dispatch migration compiles through the shared model-call service. |
| `rg 'POST /v1/messages|POST /chat/completions|send_turn_api: Anthropic|send_turn_api: OpenAI|x-api-key|bearer_auth|anthropic-version' crates/roko-cli/src/chat_session.rs` | D8 raw provider HTTP/request construction no longer exists in `ChatAgentSession::send_turn_api`. |
| `cargo test -p roko-serve test_provider --lib` | D9 serve provider-test route uses `ModelCallService::call` and still records provider health/latency/usage. |
| `cargo check -p roko-serve` | Serve compiles after D9 provider-test migration. |
| `cargo check -p roko-cli --features legacy-orchestrate --lib` | Legacy-orchestrate parse blocker is fixed; feature path checks. |
| `cargo test -p roko-runtime commit_outcome` | R1 commit outcome tests pass. |
| `cargo test -p roko-runtime commit_no_changes` | R2 clean tree no-op replacement tests pass. |
| `cargo test -p roko-runtime run_ledger` | R3 ledger adapter tests pass. |
| `cargo test -p roko-runtime workflow_report` | R4 workflow report ledger switch tests pass. |
| `cargo test -p roko-runtime workflow_report` | R4 cancellation ledger record path is covered by focused workflow-report tests. |
| `cargo test -p roko-gate gate_status` | R5 gate status tests pass. |
| `cargo test -p roko-gate gate_registry` | R6 gate registry tests pass. |
| `cargo test -p roko-runtime gate_rung` | R7 runtime rung lookup uses registry. |
| `cargo test -p roko-serve command_event` | R9 command event DTO tests pass. |
| `cargo test -p roko-serve terminal_command_event_lifecycle` | R9 terminal session path constructs typed command lifecycle DTOs. |
| `cargo test -p roko-core config_provenance` | C1 config provenance tests pass. |
| `cargo test -p roko-core provider_identity` | C2 provider identity tests pass. |
| `cargo test -p roko-core dangerously_skip_permissions` | C3 strict config validator tests pass. |
| `cargo test -p roko-core dangerous_permission_override` | C4 local override validation tests pass. |
| `cargo test -p roko-core usage_observation_optional_fields` | C5 core usage telemetry tests pass. |
| `cargo test -p roko-learn record_confidence_outcome` | C6 confidence-only API tests pass. |
| `cargo test -p roko-cli --lib runtime_feedback::routing` | C6 CLI runtime-feedback caller uses confidence-only API directly. |
| `cargo test -p roko-learn override` | C7 no fake routing context override path tests pass. |
| `cargo test -p roko-agent perplexity_chat_agent --lib` | C8 Perplexity usage consumer preserves unknown usage as `UsageObservation`. |
| `cargo run -p roko-cli --bin roko -- config doctor` | C9 doctor command prints schema/provider/dangerous-permission status without mutation. |
| `git diff --check` | Final whitespace/conflict marker check passed. |

## Changed Packets

| Packet | Current result | Boundary: not claimed |
|---|---|---|
| A0-1 | Added `scripts/roko-fitness-checks.sh` and `scripts/fitness/allowlist.toml` inventory guardrails. | Not wired into CI and not blocking. |
| A0-2 | Added `scripts/docs-status-check.sh` status-claim inventory. | Does not rewrite stale docs. |
| A0-3 | `ContentBlock::Text` outbound JSON is canonical `"type": "text"`; inbound `"content"` alias remains tolerated. | No ACP message-flow redesign. |
| A0-4 | `send_session_update` emits a flat `session/update` payload. | No unrelated event mapping changes. |
| A0-5 | ACP stream failures can emit typed failed tool-call updates instead of normal completion. | Older slash/command paths still have unrelated best-effort sends. |
| A0-6 | Root `roko.toml` no longer enables `dangerously_skip_permissions = true`. | Other hardcoded bypass sites still need owner-reviewed packets. |
| D1 | Core `DispatchPlan` skeleton exists and is exported by `roko-core`. | No runtime behavior changed. |
| D2 | CLI runner-local `DispatchPlan` was renamed to `RunnerDispatchPlan`. | Runner execution does not yet use the shared plan. |
| D3 | `DispatchResolver::resolve_existing` projects current selection into shared `DispatchPlan`. | Auth/capability diagnostics are explicitly `Unvalidated`; no execution switch. |
| D4 | `ModelStreamEvent`, `BoxModelStream`, and call-to-stream/failure adapters exist. | Provider-native SSE is not complete for every provider. |
| D5 | OpenAI-compatible parser preserves absent usage as unknown and explicit zero as provider-reported zero. | Legacy `Usage` compatibility conversion still zero-fills when requested. |
| D6 | ACP Anthropic/Claude and OpenAI-compatible provider paths now route through `ModelCallService::stream` and map `ModelStreamEvent` to ACP events. | This migrates ACP provider streaming, not every chat/serve/CLI surface. |
| D7 | `unified.rs` and `chat_inline.rs` no longer silently fall back to `dispatch_direct` on managed session failure. | `dispatch_direct` itself is not deleted. |
| D8 | `ChatAgentSession::send_turn_api` now builds a `ModelCallRequest`, synthesizes only the missing provider/model config needed for the selected model, consumes `ModelCallService::stream` events, and no longer owns a `reqwest::Client`. `ModelCallService` prompt rendering now preserves `User:`/`Assistant:` role boundaries for multi-turn history. | Provider-native structured chat history is not complete for every adapter; some adapters still receive rendered prompt text. API key preflight remains in chat to preserve missing-key history semantics. |
| D9 | `POST /api/providers/{id}/test` in serve now builds a minimal `ModelCallRequest` and calls `ModelCallService::call` instead of constructing an agent directly. | Other serve dispatch surfaces still need migration; this is provider-test only, not template dispatch or agent messaging. |
| R1 | `CommitOutcome::{Created, NoChanges, Rejected, Failed}` exists with legacy adapters. | Legacy `PipelineInput` variants remain for compatibility. |
| R2 | Clean-tree commit no longer produces runtime `"noop"` hash as success; it records `CommitOutcome::NoChanges`. | Compatibility handling for old `CommitDone { hash }` inputs remains. |
| R3 | `RunLedger` skeleton and `to_report_compat` adapter exist. | Ledger is not complete workflow truth yet. |
| R4 | `run_with_cancel` creates a `RunLedger`, returns report compatibility output from it, and records typed cancellation requests. | Resume/checkpoint paths and full gate/artifact details still need separate packets. |
| R5 | `GateStatus` type and compatibility conversion exist. | Gate runtime/report migration is not complete. |
| R6 | `GateRegistry` alias/rung map exists. | Duplicate maps outside the migrated runtime lookup may remain. |
| R7 | Runtime rung lookup delegates to `GateRegistry`. | Only one duplicate map was replaced. |
| R8 | `ArtifactOutcome` adapter exists for PRD generation outcomes. | PRD generation/output behavior was not rewritten. |
| R9 | Serializable `CommandEvent` DTOs exist in `roko-serve`, and one terminal command/session lifecycle constructs typed Started/Output/Exited/SpawnFailed/Cancelled events. | Demo UI and every terminal consumer are not fully migrated. |
| C1 | Config provenance and resolved/validated config structs exist. | CLI config loading is not migrated to provenance ownership. |
| C2 | Provider/model identity and transport/auth definition types exist. | Existing provider config parsing/dispatch is not migrated. |
| C3 | Strict raw-TOML validator rejects shared config with dangerous root skip. | Validator is not wired into every normal config load path. |
| C4 | `DangerousPermissionOverride` type validates reason, scope, expiry, source, and acknowledgement env name. | No production path accepts overrides yet. |
| C5 | `UsageObservation` moved to `roko-core` and is re-exported from `roko-agent`. | Compatibility re-export remains. |
| C6 | Router API has `record_confidence_outcome`; CLI runtime-feedback and serve dispatch confidence-only callers use it directly. | The deprecated wrapper remains for compatibility and non-router `record_outcome` methods in other learning domains are unrelated. |
| C7 | Override learning without real context records confidence-only outcomes. | Real contextual routing still needs dispatcher context wiring. |
| C8 | OpenAI provider parser proof covers unknown-vs-zero usage; `openai_agent` and Perplexity chat consumers preserve canonical `UsageObservation`. | Other provider parsers/consumers are not migrated. |
| C9 | `roko config doctor` skeleton prints config/schema/provider/dangerous-permission status without mutation. | It does not migrate or enforce config. |

## Explicit Not Done Yet

| Area | Not done yet |
|---|---|
| CI enforcement | Fitness and docs-status scripts are inventory-only and not wired as blocking CI gates. |
| Full dispatch unification | ACP Anthropic/OpenAI-compatible streaming, CLI chat API dispatch, and serve provider-test now use the shared model-call contract, but legacy direct dispatch and other serve/compatibility paths still contain raw clients or local dispatch logic. |
| Provider auth/capability validation | D3 emits `Unvalidated` diagnostics; it does not enforce auth or capability support. |
| `dispatch_direct` deletion | Silent production fallback is blocked, but the legacy module/function remains under feature/deprecated ownership. |
| Runtime truth | `RunLedger` is now used in one main workflow path and records cancellation requests, but full gate verdicts, artifacts, event persistence, and resume still need ledger-first packets. |
| Gate migration | Gate status/registry types exist, but all gate report/event paths are not migrated to typed status. |
| Telemetry migration | OpenAI-compatible parser plus OpenAI and Perplexity consumers preserve unknowns; legacy `Usage` adapters and other provider parsers can still collapse unknowns. |
| Config enforcement | C3/C4 add types and validators, but shared config loading is not fully fail-closed on dangerous overrides. |
| Terminal events | One terminal session lifecycle constructs typed DTOs, but demo UI and every terminal consumer still need follow-up wiring. |
| Product proof | Most packets have focused unit/check proof, not end-to-end product-path manifests. |

## Current Blockers And Known Constraints

| Blocker | Impact |
|---|---|
| Workspace formatting may still be noisy because many unrelated dirty files are present. | Touched Rust files were formatted directly with `rustfmt --edition 2024`; final `git diff --check` passed. |
| Legacy-orchestrate parse blocker was fixed in `crates/roko-cli/src/orchestrate.rs`. | `cargo check -p roko-cli --features legacy-orchestrate --lib` now reaches type checking and passes in the focused verification run. |
| Static grep still finds `PipelineInput::CommitDone { hash }` compatibility handling. | This is not the R2 `"noop"` regression, but old commit inputs remain reachable. |

## Remaining Safe Next Packets

| Packet | Why it is safe next | Guardrails for the agent |
|---|---|---|
| Serve template/agent migration | D4/D6/D8/D9 shape is proven for ACP, CLI chat, and serve provider-test paths. | One serve endpoint at a time; do not add surface-local HTTP/SSE. |
| Provider-native structured history | `ModelCallRequest.messages` exists and shared rendering preserves role boundaries, but not every adapter receives provider-native message arrays yet. | Fix in provider/model-call ownership, not by restoring surface-local HTTP in chat or ACP. |
| C6 compatibility cleanup | Deprecated `record_outcome` wrapper still exists for compatibility. | Delete only after all real callers are proven migrated; do not change reward math. |
| C8 parser repetition | OpenAI-compatible parser pattern is proven. | One provider parser per packet; preserve unknown as `None`, not zero. |
| R4 ledger detail packets | Main run path creates a ledger. | Split by source: gates, artifacts, cancellation, event persistence, resume. |
| R9 terminal/demo wiring | Terminal DTO lifecycle is proven in one path. | No prompt scraping; wire explicit command events only. |
| A0 CI promotion | Inventory scripts exist and can be baselined. | Add allowlist review first; do not hide violations with broad grep exclusions. |

## Anti-Patterns Observed During Execution

| Anti-pattern | Where observed | Consequence |
|---|---|---|
| Skeletons described like migrations | D1, D3, D4, R3, R5, C1, C2, C5. | Do not claim product-path unification from type/adaptor packets alone. |
| Surface-local provider execution | Raw provider clients remain outside provider ownership even after ACP and CLI chat migration. | Dispatch bugs recur unless each surface is migrated behind the shared contract. |
| Compatibility adapters preserving old paths | `CommitDone`, `dispatch_direct`, `record_outcome`, and legacy `Usage` conversions remain. | Old behavior can stay reachable unless explicitly blocked or deleted later. |
| Unknown-to-zero compatibility | `UsageObservation -> Usage` still maps missing tokens/cost to zero for legacy consumers. | Telemetry unknowns can still be misread as real zero in legacy paths. |
| Product proof overclaim risk | Most packets prove focused contracts, not full CLI/ACP/serve product flows. | Status should be `Changed`/`Built`, not `LiveInAllProductPaths`. |
| Shared target contention | Concurrent cargo jobs blocked some default target-dir checks. | Use isolated `CARGO_TARGET_DIR` for agent verification. |
