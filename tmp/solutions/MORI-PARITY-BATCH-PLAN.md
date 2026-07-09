# Mori Parity Batch Plan

Date: 2026-04-28

This is the implementation plan I would use after refining `solution-ACTUAL.md`. It is
ordered for shortest path to a working Mori-like product, not for completing every audit.

Update after E2E dogfood: add Batch -1 before safety/chat/PRD work. The run in
`/private/tmp/roko-e2e-rust-yW9yhe` showed that model selection, generated plan validity,
gate truth, telemetry, and resume paths must be made coherent first. See
`E2E-ROKO-DOGFOOD-AUDIT.md`.

## Batch -1: Execution Contract Repair

Estimate: 1-2 days for the narrow repair pass.

Why this is now first:

- `prd plan --model claude-haiku-4-5` used Opus.
- `plan regenerate --model claude-haiku-4-5` used Opus.
- `roko run --model glm-5-1` selected Anthropic API Sonnet and failed without
  `ANTHROPIC_API_KEY`.
- Positional one-shot prompts used GLM even when `--model gpt-4o` or `--model llama32` was
  requested.
- `config providers test ollama --model llama32` tested the provider default Gemma model.
- `config models route <anything>` selected Sonnet for every tested input.
- `prd plan` generated invalid tasks that `plan run` could not execute.
- `config migrate` repaired one `run` provider route, but was interactive, kept
  `shell:true`, and did not prevent an implementer/autofix loop on a no-edit prompt.

Do:

- Create one effective model/provider selection result shared by all agent-starting commands.
- Precedence should be: explicit CLI model, task model/model_hint, role config model, cascade
  router recommendation, project default.
- Make commands print and persist the selected model key, provider kind, provider source, backend
  slug, and selection reason.
- Thread CLI model/effort/workdir/repo/resume through `prd plan` and `plan regenerate`.
- Make provider tests honor `--model`, honor `--json`, and fail full-success mode on empty model
  content.
- Change `config models route <model>` to exact resolution or rename the current behavior to
  `recommend`.
- Run plan validation automatically after generation/regeneration and before run/dry-run.
- Normalize or reject model aliases like `sonnet` and `haiku`.
- Make `roko init` emit schema v2 provider/model tables, or make first run block on an
  explicit migration/preflight with a non-interactive confirmation option.
- Replace profile-generated and migrated no-op gates with real gates or mark them
  `skipped/not_wired`.
- Add `plan run --fresh` or an equivalent state reset for deliberate reruns after editing a plan.
- Make `roko resume` use `workspace_paths::plans_dir`, not hardcoded `./plans`.

Do not:

- Add a new gateway or routing crate.
- Keep adding path-specific model selection patches after this; the point is to remove divergent
  execution behavior.
- Treat provider connectivity as success when the model returns empty content.

Acceptance:

- The same `--model` produces the same selected model/provider in positional prompt, `run`,
  `prd plan`, `plan regenerate`, and `plan run`.
- A generated PRD plan either passes `plan validate` immediately or the command exits nonzero.
- `plan run --dry-run` refuses invalid plans.
- `roko resume` finds a PRD-generated plan under `.roko/plans`.
- Status, plan list, and run-state agree after a partial run.

## Batch 0: Decide The Safety Posture

Estimate: 2-4 hours if background serve stays enabled, less if disabled for M0.

Decision:

- If no-args `roko` keeps starting background serve, secure the server before calling M0 done.
- If the first goal is only local chat parity, make no-args chat not start background serve by
  default, then do server safety in M2.

Minimum safe behavior:

- Auth is enabled when binding publicly.
- Terminal routes are behind auth.
- Default CORS is localhost-only.
- `--share` uses private/scrubbed output.

Why this is first:

- `roko` currently starts background serve from `crates/roko-cli/src/unified.rs:45-64`.
- Server auth defaults to disabled at `crates/roko-core/src/config/serve.rs:54-57`.
- Terminal routes are merged at `crates/roko-serve/src/routes/mod.rs:140`.
- CORS falls back to permissive at `crates/roko-serve/src/routes/middleware.rs:428`.
- Gist sharing uses `--public` at `crates/roko-cli/src/share.rs:82-87`.

Acceptance:

- Local chat can run without exposing unsafe HTTP/PTY behavior.
- If `PORT` or public bind is used, auth is required.

## Batch 1: Add A Real Chat Session Object

Estimate: 4-6 hours.

Create a small session owner for no-args chat. It should not be a new gateway. It should be
the minimal state needed to call existing adapters correctly.

Suggested shape:

```rust
struct ChatAgentSession {
    auth: AuthMethod,
    workdir: PathBuf,
    model: String,
    effort: String,
    system_prompt: String,
    allowed_tools_csv: String,
    mcp_config: Option<PathBuf>,
    claude_session_id: Option<String>,
    history: Vec<ChatMessage>,
    http_client: reqwest::Client,
}
```

Responsibilities:

- Load config once at session start.
- Pick the effective model once, with `/model` mutating it.
- Build a workspace-rooted system prompt once, with `/system` replacing or appending to it.
- Resolve role/tool allowlist once, with a conservative default.
- Resolve MCP config once, and expose a refresh path if `/config` changes it.
- Store Claude `session_id` after each turn.
- Accumulate API-provider history.
- Own a shared HTTP client for raw HTTP only if a raw path still exists.

Implementation notes:

- Do not put this logic into `dispatch_direct.rs`.
- `chat_inline.rs` already has session fields, but they are local display state. Either extend
  that state with a real dispatch session or add one field that owns the dispatch session.
- `/system`, `/model`, `/effort`, and `/reset` should mutate this session and not only print
  confirmations.

Acceptance:

- A unit test can prove `/system` changes the next request.
- A unit test can prove a returned Claude session id is stored for the next turn.
- No chat dispatch function accepts only `(&AuthMethod, &str)` after this batch.

## Batch 1.5: Ground PRD/Plan Generation

Estimate: 6-10 hours for the first useful validator pass.

Why this moved up:

- The demo run at `/tmp/roko-demo-1777396797076` produced a greenfield plan for an existing Roko
  feature while apparently running from a temp workspace rather than the actual Roko source tree.
- Two nearby demo runs repeated the same pattern.
- The generated plans proposed `roko-prompt`, `roko-orchestrate`, and sometimes `roko-config`
  instead of using existing crates.

Do:

- Build a bounded repository context pack before `prd draft new`, `prd draft edit`, `prd plan`,
  and `plan generate`.
- Verify that the current workdir is the intended target repo; if a Roko-internal feature is
  requested from a non-Roko temp workspace, stop or ask for the target repo.
- Include workspace members from root `Cargo.toml`, matching source files from `rg`, related
  PRDs/plans, and explicit existing surfaces such as `roko-compose`, `roko-agent`, `roko-runtime`,
  and `roko-cli/src/runner`.
- Require generated PRDs to include a `Repository Grounding` section.
- Validate generated plans before accepting them:
  - referenced existing files exist
  - workspace crate names are real
  - task count metadata matches parsed tasks
  - generated plans do not say "greenfield" for an existing repo
  - new crates require explicit permission
- Separate subprocess success from artifact validation success.
- Withhold positive learning/knowledge seeds when artifact validation fails.
- Persist prompt, stream JSON, output, tool-call summary, file changes, and validation report as
  run sidecars.

Do not:

- Implement generated plans that create duplicate prompt/runtime crates.
- Accept a PRD or plan just because markdown/TOML parses.
- Treat zero token/cost metrics as real usage when Claude CLI did not provide parsed usage.

Acceptance:

- A regression fixture based on `/tmp/roko-demo-1777396797076` fails validation for greenfield
  duplicate-crate plans.
- A grounded plan for `system-prompt-wiring` names existing Roko files and does not create
  `roko-prompt` or `roko-orchestrate`.
- PRD/plan learning records distinguish `process_success=true` from
  `artifact_validation=failed`.
- The demo UI can show the validation report and transcript sidecar instead of only generic tool
  progress.

## Batch 2: Claude CLI Parity Through Existing Adapter Code

Estimate: 6-8 hours for non-streaming parity, 10-12 hours with event streaming started.

Use existing code as the implementation source:

- `crates/roko-agent/src/claude_cli_agent.rs:305-341` already handles the correct Claude CLI
  flags.
- It includes settings, model, effort, fallback model, system prompt, tools, MCP config, and
  resume.
- It already has timeout and subprocess behavior that is closer to the intended runtime than
  `dispatch_direct.rs`.

Do:

- Route Claude CLI chat turns through `ClaudeCliAgent` or a wrapper that calls the same command
  builder.
- Pass:
  - working directory
  - model
  - effort
  - system prompt
  - allowed tools
  - MCP config
  - previous session id
  - timeout
  - cancellation
- Store the new session id from the result event.
- Keep tool outputs visible in the chat UI.

Do not:

- Add a second complete Claude command builder to `dispatch_direct.rs`.
- Copy Mori's `--bare` unless the installed Claude CLI still supports it. Roko's current
  `ClaudeCliAgent` says it was removed.
- Add API tool-loop code in this batch.

Acceptance:

- A command-builder test asserts model, effort, system prompt, tools, MCP config, and resume are
  present when configured.
- A chat-session test proves turn 2 receives turn 1's session id.
- A manual run proves a repository file-read request uses Claude tools and renders tool output.

## Batch 3: Streaming To Inline UI

Estimate: 6-8 hours.

Goal:

- Users should see token/tool progress as it arrives, not after the subprocess exits.

Do:

- Convert Claude stream-json lines into chat stream events as they are read.
- Feed `StreamingState` rather than waiting for one final `DispatchResult`.
- Render assistant text deltas and tool output deltas separately.
- Capture final result metadata for session id, model, tokens, and cost.
- Add cancellation so Ctrl-C kills the active child and returns to input.

Existing assets:

- `crates/roko-cli/src/inline/primitives/streaming.rs`
- `crates/roko-cli/src/chat_inline.rs:1457-1488` current dispatch channel
- Mori parsing reference in `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:3150-3245`

Acceptance:

- Long responses stream before completion.
- Tool output appears before final answer when a tool runs.
- Ctrl-C during a turn cancels the child process.
- The final session id is still stored after streaming completes.

## Batch 4: API Provider Context Parity Without Rebuilding Tool Loops

Estimate: 6-10 hours for system/history/provider-adapter routing; more only if streaming is
included immediately.

Goal:

- API providers should stop being bare single-message calls.

Use existing provider adapters:

- Anthropic adapter already switches to tool loop when the model supports tools.
- OpenAI-compatible adapter already builds tool-loop agents and can discover MCP tools.
- `ModelCallService` already wraps provider execution, feedback, budget, cache, and events in
  partial form.

Do:

- Route API chat turns through `ModelCallService` or `create_agent_for_model`, not through
  handwritten request JSON in `dispatch_direct.rs`.
- Pass the session system prompt into `AgentOptions.system_prompt` or `ModelCallRequest.system`.
- Send conversation history, not only the latest prompt.
- Reuse a session-scoped HTTP client where raw HTTP remains.
- Keep a clear fallback if provider adapter creation fails.

Do not:

- Build a new Anthropic tool loop in `dispatch_direct.rs`.
- Build a new OpenAI-compatible tool loop in `dispatch_direct.rs`.
- Make API streaming block Claude CLI M0.

Acceptance:

- Anthropic API path sends a system prompt and prior messages.
- OpenAI-compatible path sends a system prompt and prior messages.
- Tool-capable provider tests use the existing provider tool-loop code.
- No new raw provider-specific tool loop exists in `roko-cli`.

## Batch 5: Slash Commands And One-Shot Coherence

Estimate: 3-5 hours.

Do:

- `/system` updates the live session prompt.
- `/effort` updates Claude effort and relevant provider settings.
- `/model` works for Claude CLI too, not only API providers.
- `/reset` clears local conversation and Claude resume id.
- `roko "prompt"` and no-args chat should share the same prompt/provider policy where possible.

Current source facts:

- `/system` stores text at `crates/roko-cli/src/chat_inline.rs:2134`.
- `/effort` currently confirms without storing behavior at
  `crates/roko-cli/src/chat_inline.rs:2245-2273`.
- One-shot already prefers `ModelCallService` at `crates/roko-cli/src/unified.rs:96-100`, but
  no-args chat still uses raw `dispatch_direct::dispatch_prompt` at
  `crates/roko-cli/src/chat_inline.rs:1486`.

Acceptance:

- Slash commands have observable effect in the next request.
- Commands that are still unsupported say unsupported, not success.
- The same model/prompt defaults appear in no-args chat and one-shot mode.

## Batch 6: Product Proof Suite

Estimate: 4-6 hours.

Add proof before widening scope.

Required proof scenarios:

- Claude CLI M0:
  - start `roko`
  - ask for repository file list
  - verify a tool call occurred and output rendered
  - ask a follow-up
  - verify resume id was used
- Slash command:
  - `/system` changes model behavior
  - `/effort high` changes invocation
  - `/reset` clears resume id
- Streaming:
  - long output appears before process exit
  - Ctrl-C cancels an active turn
- Safety:
  - no-args chat does not expose unauthenticated terminal routes
  - public bind requires auth if serve is started
- API minimal parity:
  - Anthropic/OpenAI-compatible requests include system/history through existing adapters

Only after these pass should work move to full runner/gateway convergence.

## Deferred Until After M0-M1

These are real, but they are not first:

- new `roko-gateway` crate
- cell/graph engine
- full `WorkflowEngine` retirement work
- deleting or shrinking `orchestrate.rs`
- full StateHub unification
- board/kanban/task CRUD UX
- dream/knowledge/router closed loop
- full API-provider streaming across every backend
- cross-plan DAG and merge-queue parity

## Recommended Time Box

Time box M0 to one focused iteration:

- Batch 0: 0.5 day
- Batch 1: 0.5-1 day
- Batch 2: 1 day
- Batch 3: 1 day
- Batch 5 plus proof: 1 day

That is roughly 4-5 focused engineering days for the experience users will actually judge
first. M1 API parity can follow immediately after if Claude CLI parity proves the session
model.
