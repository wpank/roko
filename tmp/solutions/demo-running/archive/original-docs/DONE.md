# Completed Work — DO NOT Re-Implement

These items are resolved and committed. Agents should NOT touch these.

## Resolved Blockers

- **B10**: OpenAI `max_tokens` → `max_completion_tokens` for newer models. Added `use_max_completion_tokens: bool` to `ModelProfile`, threaded through `OpenAiCompatLlmBackend` and `CodexAgent`.
- **B11**: Demo `showCmd` exit code capture via hidden `execCmd('(exit $?)')` + OSC 7777 sideband.

## Completed Batches (47 total)

- Provider/model synthesis removed from ModelCallService, ACP, core (batches 6, 10-14, 19-20)
- Direct agent-exec learning persistence wired across all dispatch paths (batches 8-9, 12, 15-18, 21)
- CLI model selection requires explicit config profiles (batch 22)
- Serve provider health requires explicit config (batch 23)
- Provider factory rejects missing providers (batch 24)
- ACP explicit config path, session revalidation, config options (batches 25-30)
- Demo workspace-explicit commands, PRD CWD guards (batches 31-34)
- Safety contract trust boundaries + asset embedding (batches 35, 38)
- Model max_output ceilings (batch 36)
- Health endpoint + graceful shutdown (batch 37)
- Request-timeout, retry-policy, circuit-breaker, tool-loop, vision-loop defaults centralized (batches 39-47)
- Cascade router persistence with stable model candidates (batch 5)
- Gateway provider health records provider IDs (batch 7)
- SystemPromptBuilder, EpisodeLogger, ProcessSupervisor, MCP passthrough, gate pipeline (all wired)

## Fixes Applied in wp-arch2 (Uncommitted — 2026-05-04)

### Provider Preflight (PIPELINE-RUN-AUDIT Issues A, B, G)
- **`plan.rs`**: `plan run` preflight switched from `preflight_providers()` → `preflight_provider_for_model()` (checks only default model's provider)
- **`agent.rs`**: `agent chat` preflight — same fix
- **`prd.rs`**: `prd draft new` and `prd plan` preflight — switched to targeted check using resolved model key
- **`util.rs`**: Added `preflight_provider_for_model()` function; `preflight_providers()` kept with `#[allow(dead_code)]`
- **`config.rs` (CLI)**: Added `apply_layer_value` support for `learning.*` keys (replan_on_gate_failure, replan_max_per_plan, replan_gate_attempts, auto_playbook_refresh, use_lookahead_router, lookahead_threshold)

### Config Propagation & Model Injection
- **`config.rs` (serve)**: `PUT /api/config` now writes updated config to all ephemeral workspace directories
- **`terminal-session.ts`**: Restored `--model` injection in `roko()` helper (belt-and-suspenders)

### Demo Terminal UX
- **`useTerminal.ts`**: ResizeObserver debounced (50ms), skips zero-size hidden tabs
- **`TerminalPaneWithHandle.tsx`**: Output activity detection via `outputBuffer` polling (replaces MutationObserver)
- **`Demo/index.tsx`**: Keyboard shortcuts don't intercept terminal/input focus
- **`terminal-session.ts`**: Simplified `roko()` — removed `LLM_COMMANDS` allowlist, `needsModel()`, shell quoting

### Orchestrator Performance
- **`orchestrate.rs`**: Playbook query + search enrichment run concurrently via `tokio::join!`
- **`orchestrate.rs`**: `--skip-validate` flag wired into `PlanRunner`
- **`main.rs`**: `--skip-validate` CLI arg added

## Infrastructure Already Wired

- `roko()` function in `terminal-session.ts` injects `--model` (when activeModel set)
- `showCmd()` captures exit codes via OSC 7777 sideband
- `resolveRoko()` uses unique dynamic markers to avoid echo contamination
- `fetchWorkflowSnapshot()` and `openWorkflowSubscriptions()` for live PRD/plan state
- InlineTerminal has Drop impl that calls `disable_raw_mode()`
- 66 command descriptions in `cmd-descriptions.ts`
- 12 scenarios registered in scenario-registry.ts
- Gate detection from terminal output (regex patterns for pass/fail)
