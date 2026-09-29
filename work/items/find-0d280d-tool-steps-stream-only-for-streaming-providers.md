+++
id = "find-0d280d"
kind = "finding"
title = "Tool steps stream only for providers that support streaming; others show only heartbeats until turn ends"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-agent/dispatch", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03c-backend-local-access#T10"
discovered_from = "plan:portal-programme/03c-backend-local-access#T10"
anchors = [
  "crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent::supports_streaming",
  "crates/roko-agent/src/claude_agent.rs::ClaudeAgent::supports_streaming",
  "crates/roko-agent/src/immune_boundary.rs::ImmuneBoundary::supports_streaming",
]
links = { depends_on = [], blocks = [], related = ["dec-578863"], supersedes = [], duplicate_of = "" }
+++

Live tool steps (03c T12–T20) are emitted only when the agent's `supports_streaming`
returns `true`, because they are projected from the streaming event channel that
the immune boundary opens in `drive_streaming_inner`. An agent that cannot stream
falls back to `drive_non_streaming_inner`, which blocks until the turn completes
and then publishes the screened transcript in one burst.

**Providers that stream tool steps live (supports_streaming = true):**
- `ClaudeCli` — `ClaudeCliAgent` reads Claude's `--output-format stream-json` events
- `CodexCli` — `CodexAgent`
- `CursorCli` / `CursorAcp` — `CursorAgent`
- `OpenClaw` (ACP variant) — `OpenClawAcpAgent`

**Providers that do NOT stream (supports_streaming = false); show only heartbeats:**
- `AnthropicApi` — `ClaudeAgent` (direct REST; batches the whole response)
- `OpenAiCompat`, `CerebrasApi`, `Hermes` — `OpenAiAgent`
- `PerplexityApi` — `OpenAiAgent`
- `GeminiApi` — `GeminiNativeAgent`
- `GeminiCli` — `GeminiCompatAgent` (delegates to inner; inner is non-streaming)
- `OpenClaw` (inference variant) — `OpenClawInferAgent`

**Practical impact:** the most common production provider is `ClaudeCli`, which does
stream. Workspaces using `AnthropicApi` directly (e.g. CI environments without the
Claude CLI installed) see no tool steps and must rely on `agent_heartbeat` for
liveness signals. Adding streaming to `ClaudeAgent` (direct Anthropic API SSE) is
the highest-value fix; it requires wiring the streaming tool-call delta events the
API already emits.
