# My Take: Shortest Path To Mori Parity

Date: 2026-04-28

Scope: refinement of `solution-ACTUAL.md` after reading `tmp/solutions`,
`tmp/binary-issues`, `tmp/mori-diffs`, `tmp/subsystem-audits`, the Mori reference code, and
the current Roko source.

## Verdict

`solution-ACTUAL.md` is directionally right for the first milestone. The shortest path to
user-visible Mori behavior is not a new gateway, not a cell graph, not another broad
convergence runner. It is to make the default interactive path a real Claude-style agent
session with prompt, tools, MCP, resume, streaming, and proper subprocess lifecycle.

But I would not implement `solution-ACTUAL.md` literally as "add a few flags to
`dispatch_direct.rs`." That file is already a thin pipe and should not become another
semi-orchestrator. The current repo already has better pieces:

- `crates/roko-agent/src/claude_cli_agent.rs:305-341` already builds a Claude CLI command
  with model, effort, settings, fallback, system prompt, tools, MCP config, and resume.
- `crates/roko-agent/src/provider/anthropic_api.rs:52-64` already routes tool-capable
  Anthropic models into the provider tool loop.
- `crates/roko-agent/src/provider/openai_compat.rs:360-381` already creates OpenAI-compatible
  tool-loop agents with system prompt and tools.
- `crates/roko-cli/src/dispatch_v2.rs:203-229` already has a provider-neutral CLI invocation
  seam for system prompt, MCP, and resume, although its Claude CLI invocation does not pass
  `--tools` today.
- `crates/roko-cli/src/unified.rs:96-100` already prefers `ModelCallService` for one-shot
  prompts, with raw direct dispatch only as fallback.

So the best solution is:

1. Keep `solution-ACTUAL`'s product priority: make interactive chat work like Mori first.
2. Change its implementation strategy: create the smallest session wrapper that delegates to
   existing adapters, rather than teaching `dispatch_direct.rs` every feature again.
3. Treat API/gateway/runner convergence as later phases unless they block Mori-style local use.

## Demo Run Update

After auditing `/tmp/roko-demo-1777396797076` and two nearby demo runs, I would expand the first
milestone slightly. The interactive chat recommendation is still right, but it is not enough if the
demo UI's PRD/plan workflow is part of the product experience.

The demo generated plans for `system-prompt-wiring` that repeatedly proposed greenfield crates
like `roko-prompt`, `roko-orchestrate`, and sometimes `roko-config`. The demo appears to have run
from a temp workspace rather than the real Roko source tree, so the precise failure is a
context-root mismatch plus missing artifact validation. If the feature was meant for Roko, it
needed the Roko repo context; if it was meant for a blank demo project, the plan should have said
that clearly.

So the revised shortest path is:

1. Make no-args interactive `roko` a real Mori-style agent session.
2. Make PRD/plan generation verify the intended repo, then validate artifacts before plans are
   accepted or learned from.

This is still a narrow product-first solution. It does not require a new gateway or a broad
runtime rewrite. It adds a repository context pack, artifact validators, honest telemetry, and run
transcript sidecars around the existing PRD/plan command paths.

See `DEMO-RUN-AUDIT.md` and `REVISED-BEST-SOLUTION-AFTER-DEMO.md` for the detailed evidence.

## What `solution-ACTUAL` Got Right

It correctly identifies the highest leverage issue: no-args `roko` chat is not an agent
session. The current chat path records session state locally but sends bare prompts to the
model:

- `crates/roko-cli/src/chat_inline.rs:740-764` has `conversation` and `system_message`.
- `crates/roko-cli/src/chat_inline.rs:1457-1488` dispatches only the current prompt text.
- `crates/roko-cli/src/chat_inline.rs:2120-2137` stores `/system`, but dispatch ignores it.
- `crates/roko-cli/src/dispatch_direct.rs:140-143` spawns Claude as only
  `claude --print --output-format stream-json`.
- `crates/roko-cli/src/dispatch_direct.rs:205-207` extracts `session_id`, but the chat
  session never reuses it on the next turn.

That is the user-facing breakage. Fixing this gets a much larger product improvement than
more runtime scaffolding.

## What Needs Refinement

### 1. The delta is not only four flags

For Mori-like behavior, the CLI invocation needs at least:

- model
- effort
- system prompt
- tool allowlist
- MCP config
- settings/safety hooks
- fallback model when supported
- resume session id
- working directory
- timeout and cancellation
- typed stream event handling

Mori also uses `--bare`, but Roko's current `ClaudeCliAgent` says `--bare` was removed from
Claude CLI and skips it. Do not blindly copy Mori flags without checking current CLI support.

### 2. `dispatch_direct.rs` is the wrong place to grow

`dispatch_direct.rs` currently has raw direct paths for Claude CLI, Anthropic API, and
OpenAI-compatible APIs. If it absorbs prompt assembly, tool policy, MCP, history, streaming,
timeouts, slash-command state, and provider-specific tool loops, it becomes another
duplicated runtime.

The better short path is a small `ChatAgentSession` in the CLI layer that delegates:

- Claude CLI turns to `roko_agent::claude_cli_agent::ClaudeCliAgent` or a very thin wrapper
  over it.
- API turns to existing provider adapters or `ModelCallService`, not handwritten JSON in
  `dispatch_direct.rs`.
- Later, when `ModelCallService` supports resume and streaming, the session wrapper can become
  a thin client of `ModelCallService`.

### 3. Do not hand-roll API tool loops

`solution-ACTUAL.md` says Batch 2 should build API tool loops. Current source shows tool
loops already exist under provider adapters:

- Anthropic tool-loop coverage is tested around
  `crates/roko-agent/src/provider/anthropic_api.rs:376-432`.
- OpenAI-compatible tool-loop coverage is tested around
  `crates/roko-agent/src/provider/openai_compat.rs:1018-1048`.

The shortest path is to route chat API providers through those existing adapters and fill
missing request/session state, not reimplement provider tool loops in the CLI.

### 4. Security cannot be ignored if no-args `roko` starts `serve`

The interactive command starts background serve by default:

- `crates/roko-cli/src/unified.rs:45-64`

The server still has unsafe defaults:

- Auth disabled by default: `crates/roko-core/src/config/serve.rs:54-57`
- Terminal routes merged outside the API auth path: `crates/roko-serve/src/routes/mod.rs:140`
- Permissive CORS fallback: `crates/roko-serve/src/routes/middleware.rs:428`
- Public gist sharing: `crates/roko-cli/src/share.rs:82-87`

If background serve stays enabled for no-args chat, a minimal safety batch must happen before
calling the first milestone "done." If the goal is local chat parity only, the faster move is
to disable background serve by default until it is secured.

### 5. Streaming is not optional for Mori feel

Mori's perceived quality comes from streaming text, tool events, token usage, and a running
agent lifecycle. A non-streaming implementation with better flags is useful, but it will not
feel like Mori. This is why the first milestone should include Claude stream forwarding to
`StreamingState`.

## Recommended Scope Model

Use four milestones instead of trying to solve "full Mori" in one pass.

### M0: Interactive Claude CLI Parity

Goal: `roko` works like a local Claude Code/Mori-style session.

Must include:

- no-args `roko` starts an interactive session without requiring sidecar knowledge
- Claude CLI auth is the preferred happy path
- workspace-rooted system prompt
- role/tool allowlist
- MCP config passthrough
- session id capture and reuse
- streaming output
- visible tool output
- `/system`, `/model`, `/effort`, and `/reset` actually mutate the session
- subprocess timeout and Ctrl-C cancellation
- background serve either secured or disabled by default

This is the shortest path to "Roko works."

### M1: API Provider Context Parity

Goal: API-backed providers are no longer bare one-shot text pipes.

Must include:

- shared client/config per session
- system prompt
- conversation history
- provider adapters or `ModelCallService` instead of raw JSON
- existing tool-loop agents for providers that support tools
- streaming if the provider path supports it

This is important, but it should not block M0 if Claude CLI is the assumed primary path.

### M2: One-Shot, Run, Share, And Server Coherence

Goal: the adjacent product surfaces stop contradicting the interactive path.

Must include:

- `roko "prompt"` and `roko run "prompt"` use the same prompt/session/provider policy where
  possible
- `--share` produces real run data and does not publish secrets by default
- server terminal/auth/CORS are safe
- StateHub/SSE receive meaningful runtime events

This is where the previous converge-runner audit becomes relevant.

### M3: Full Runtime/Learning Convergence

Goal: the larger Roko architecture becomes the single truth.

Must include:

- `WorkflowEngine` and runner paths use the same model-call, prompt, gate, feedback, and event
  contracts
- knowledge and learning are in the live path
- legacy dispatch and `orchestrate.rs` are retired or quarantined
- Mori parity matrix rows get proof links

This is real work, but it is not the shortest path to the first working Mori-like experience.

## Best Possible Solution

The best possible solution is a refined version of `solution-ACTUAL`:

> Build a narrow interactive `ChatAgentSession` that wires existing prompt, tool, MCP, Claude
> CLI, provider, and streaming components into no-args `roko`, prove Mori-style local chat,
> then expand the same session policy to API providers and adjacent run/share/server surfaces.

This is better than all prior alternatives because it:

- preserves the product-first insight from `solution-ACTUAL`
- avoids the over-engineering of the gateway/cell/graph proposals
- avoids increasing the long-term damage inside `dispatch_direct.rs`
- uses existing working code instead of rebuilding tool loops
- creates a clean seam that can later delegate to `ModelCallService`

## What Not To Do First

- Do not build a new `roko-gateway` crate before M0.
- Do not start the cell/graph engine before M0.
- Do not retire `orchestrate.rs` before product parity proof.
- Do not reimplement API tool loops in `dispatch_direct.rs`.
- Do not fix all 90 binary issues before making interactive chat work.
- Do not claim Mori parity because modules exist. Claim it only after the no-args `roko`
  path proves the behavior.

## First Proof That Matters

The first milestone is done only when this sequence works from a fresh terminal:

1. `roko` enters chat.
2. User asks: "What files are in this repository root?"
3. Agent uses tools, and tool output is visible.
4. User asks a follow-up that depends on the previous answer.
5. The next Claude turn uses `--resume` or equivalent session continuity.
6. User runs `/system <new instruction>` and the next answer reflects it.
7. User runs `/effort high` and the next Claude invocation reflects it.
8. Long answers stream incrementally.
9. Ctrl-C cancels an in-flight turn without leaving a child process.
10. If background serve is active, it is not exposing unauthenticated terminal access.

That proof is the shortest path to "as good as Mori" in the way users will actually feel.
