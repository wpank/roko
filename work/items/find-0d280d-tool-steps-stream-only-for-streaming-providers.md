+++
id = "find-0d280d"
kind = "finding"
title = "Tool steps stream only for providers that support streaming; others show only heartbeats until turn ends"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-agent/dispatch", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/03c-backend-local-access#T10"
discovered_from = "plan:portal-programme/03c-backend-local-access#T10"
anchors = ["crates/roko-agent/src/immune_boundary.rs::ImmuneBoundary::supports_streaming", "crates/roko-agent/src/tool_loop/agent_wrapper.rs::ToolLoopAgent::supports_streaming", "crates/roko-agent/src/gemini/native.rs::GeminiNativeAgent::supports_streaming", "crates/roko-agent/src/openclaw/infer_agent.rs::OpenClawInferAgent::supports_streaming", "crates/roko-agent/src/provider/hermes.rs"]
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

Re-checked 2026-09-29: the provider list needs correcting. Tool-enabled AnthropicApi, OpenAiCompat and CerebrasApi models run through ToolLoopAgent (tool_loop/agent_wrapper.rs:418, supports_streaming = true), which forwards backend stream events, including ToolCallEnd, to the immune boundary. They therefore do show live tool steps, at least once per model turn. ClaudeAgent and OpenAiAgent are only used for models without tools. The tool-capable agents that still fall back to non-streaming are HermesHttpAgent (Hermes provider, default supports_streaming = false), GeminiNativeAgent (gemini/native.rs:543), OpenClawInferAgent (openclaw/infer_agent.rs:252) and GeminiCompatAgent when its inner agent does not stream. Adding SSE streaming to ClaudeAgent is therefore not the highest-value fix.
