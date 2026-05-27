# Final Solution: Shortest Path To Mori Parity

Date: 2026-04-28 (third pass)

This supersedes `solution-ACTUAL.md` by incorporating the review agent's corrections and the
demo run failure analysis.

Update after source verification: the demo PRD/plan failure did not come from the raw
`dispatch_direct.rs` no-args chat path. It came from the direct PRD/plan agent-exec paths, which
already route through shared agent creation and `ClaudeCliAgent` for Claude CLI. The broader
interactive-chat diagnosis is still valid, but the PRD/plan demo failure needs its own grounding
and validation rail.

Update after E2E dogfood: a clean Rust run in `/private/tmp/roko-e2e-rust-yW9yhe` proved a more
basic prerequisite. Before the two-rail M0 below is trustworthy, Roko needs an **M0-0 execution
contract repair**: one model/provider selection path, generated plan validation before execution,
truthful gates, truthful telemetry, and consistent resume/state paths. Full evidence is in
`E2E-ROKO-DOGFOOD-AUDIT.md`.

Update after demo-app workflow audit: `demo/demo-app` is not yet a reliable proof surface. It
mixes live state with fallback data, runs product workflows through terminal scraping, has bench
and share API contract drift, starts terminals in ambiguous workdirs, and can show fake running or
connected states after failures. Full evidence is in `DEMO-APP-WORKFLOW-AUDIT.md`. This does not
change M0-0; it makes truthful demo proof a required product rail after the execution contract
repair.

## What The Demo Run Proved

On 2026-04-28, `roko prd draft new system-prompt-wiring` + `roko prd plan system-prompt-wiring`
ran successfully in `/tmp/roko-demo-1777396797076`. Findings:

1. **Plan generated greenfield crates that already exist.** The plan creates `roko-prompt` and
   `roko-orchestrate`. The demo workspace appears to be a temp workspace rather than the actual
   Roko source tree, so the immediate issue is context-root ambiguity: if this was meant to modify
   Roko, the run lacked the Roko repo; if it was meant to demo a blank project, the artifact should
   have said that clearly instead of producing a confident Roko-internal architecture. In the real
   Roko source tree, the relevant pieces already live under `roko-compose`, `roko-agent`,
   `roko-runtime`, `roko-cli/src/runner`, and legacy/donor `orchestrate.rs`.

2. **Cost tracking shows $0.00 for all episodes.** Two Claude Opus sessions ran for 247s and
   273s respectively, both recording 0 input/output tokens and $0.00 cost. The episode logger
   never extracts token/cost metadata from the Claude CLI response stream. This is systemic
   problem S4 (silent errors — cost data is lost, not reported as missing).

3. **Cascade router has 0 observations.** The role_table is statically seeded but never updated.
   `total_observations: 0` means the learning loop is not receiving feedback from dispatches.

4. **PRD quality is high, relevance is low.** The generated PRD has academic citations,
   architecture diagrams, 20 requirements, and a 20-task plan — all describing functionality
   that already exists under different names. The agent was maximally capable and minimally
   informed.

## What The E2E Dogfood Added

On 2026-04-28, a full fresh-repo test covered `init`, `prd idea`, `prd draft new`, `prd plan`,
`plan validate`, `plan regenerate`, `plan run`, `run`, `run --engine legacy`, positional prompts,
provider tests, route explanations, `status`, `plan list`, `plan show`, and `resume`.

New findings:

1. **Model selection is not a contract.** `prd plan` and `plan regenerate` ignored
   `--model claude-haiku-4-5` and used Opus. `run` ignored model flags and selected Anthropic API
   Sonnet without an Anthropic key. Positional prompts ignored `--model gpt-4o`, `--model glm-5-1`,
   and `--model llama32`, and used GLM. Provider tests ignored `--model` in at least the Ollama
   case. `config models route <model>` selected Sonnet for every requested model tested.

2. **Generated plans are not self-executable.** `prd plan greet-function` generated a plausible
   but invalid `tasks.toml`: missing required `role` fields and using unconfigured `sonnet`/`haiku`
   aliases. `plan regenerate` did not repair those fields. `plan run` then failed immediately on
   provider resolution.

3. **The runner can work only after manual repair.** After manually adding roles and replacing
   model aliases with `claude-sonnet-4-6`, the runner implemented the requested Rust function and
   tests. It still failed final gates due a doctest import and a temp-package clippy name issue, and
   auto-fix did not resolve either.

4. **Gate truth is mixed.** Several gates are recorded as passing stubs (`symbol`,
   `generated_test`, `verify_chain`, `fact_check`, `llm_judge`, `integration`). Legacy `run` passed
   on `shell:true` even when Claude Code's internal `cargo check` was blocked by a hook.

5. **Telemetry is both missing and duplicated.** PRD/plan agent-exec episodes still record zero
   cost even when Claude stream result events include cost. Runner costs are double-counted in
   `.roko/learn/costs.jsonl` because attempts are logged once as success and again as gate failure.
   Model is sometimes recorded as `unknown-model`.

6. **State views disagree.** `resume` hardcodes `./plans` and fails for `.roko/plans`.
   `status`, `plan list`, and `.roko/state/run-state.json` disagree about task completion.

This does not invalidate the two-rail solution. It means the first batch must make execution
selection, artifact acceptance, gates, telemetry, and state truthful before the product can be
called Mori-like.

## Root Cause (Refined)

There are two related but distinct first-mile problems:

1. **Interactive chat dispatch is too thin.**
   The no-args chat path still sends bare prompt text through `dispatch_direct.rs` and does not
   give Claude CLI the system prompt, tool policy, MCP config, resume id, or streaming lifecycle
   that Mori relies on.

2. **PRD/plan artifact generation is not grounded or validated enough.**
   The demo path uses the PRD/plan command handlers and `agent_exec`, not the no-args chat path.
   It asks the agent to search the codebase, but Roko accepts the generated markdown/TOML without
   proving that the current directory is the intended repo, that the artifact names real crates,
   cites existing files, avoids duplicate greenfield crates, or maps onto the current runner/prompt
   architecture.

3. **The demo UI is not a proof harness yet.**
   Demo routes can hide broken endpoints behind fallback data, scripted scenarios can run stale CLI
   commands, and UI state can be inferred from terminal text instead of structured run results.
   The visible demo can therefore look alive while the actual workflow failed, did nothing, or ran
   in the wrong workspace.

The shared failure pattern is "model output accepted without enough repository contract." Chat
needs a real session contract. PRD/plan generation needs a repository-grounding and artifact
validation contract.

## The Correct Solution (Refined)

### M0-0: Execution Contract Repair

Prepend this before ChatAgentSession and PRD/plan grounding work:

- Implement one `EffectiveModelSelection` or equivalent result shared by positional prompt,
  `run`, legacy run, `prd draft`, `prd plan`, `plan generate`, `plan regenerate`, `plan run`, and
  provider tests.
- Make explicit CLI `--model` a hard command override unless a command documents and prints a
  stronger planner policy.
- Thread `--model`, `--effort`, `--repo`, `--workdir`, and `--resume` through every agent-starting
  command.
- Make generated `tasks.toml` validate before accepting a PRD/plan generation result.
- Reject or normalize model aliases such as `sonnet` and `haiku` before execution.
- Make `roko init` emit schema v2 provider/model tables, or make first run block on an
  explicit `config migrate`/preflight path with non-interactive confirmation support.
- Mark stub gates as `skipped/not_wired`, not pass.
- Replace generated `shell:true` with real profile gates for `roko init --profile rust`.
- Treat migrated no-op gates as still unproven; migration should not preserve `true` as the
  only success proof for a coding workflow.
- Parse Claude CLI result usage/cost and do not store unknown usage as numeric zero.
- Emit one cost event per agent attempt, not one success plus one failure cost event for the same
  attempt.
- Fix `roko resume` to use the same `.roko/plans` root as PRD-generated plans.

### M0-1: Truthful Demo Proof Surface

Do this after M0-0 starts landing, before claiming demo parity:

- Remove silent fallback from mutation paths and make live/mock mode explicit.
- Align bench and share API contracts between React and `roko-serve`.
- Ensure unmatched `/api/*` routes return JSON errors, not the SPA HTML shell.
- Make terminal/workflow workdirs explicit and block UI actions until setup is complete.
- Fix demo terminal handle readiness before scenario play.
- Move product workflow truth from terminal scraping to typed server events.
- Make every demo scenario end in `completed`, `failed`, or `skipped` with artifact links.
- Hide or clearly label placeholder surfaces such as Mirage and chain anchoring until they have
  real backend workflows.

### Architecture: ChatAgentSession → Existing Adapters

Do NOT grow `dispatch_direct.rs`. Create a small `ChatAgentSession` that delegates to existing
working code:

```
ChatAgentSession
├── Claude CLI turns → ClaudeCliAgent (crates/roko-agent/src/claude_cli_agent.rs)
│   Already has: model, effort, system prompt, tools, MCP, resume, timeout, fallback
├── API turns → Provider adapters (anthropic_api.rs, openai_compat.rs)
│   Already have: tool loops, system prompt, history
└── Later → ModelCallService (when it supports resume + streaming)
    Already has: budget, cache, feedback, events
```

### What ClaudeCliAgent Already Does Right

```rust
// Already builds the correct command:
cmd.arg("--print");
cmd.arg("--verbose");
cmd.arg("--output-format").arg("stream-json");
cmd.arg("--model").arg(&model);
cmd.arg("--effort").arg(&effort);
cmd.arg("--settings").arg(&settings_json);

if dangerously_skip_permissions { cmd.arg("--dangerously-skip-permissions"); }
if let Some(max_turns) = max_turns { cmd.arg("--max-turns").arg(max_turns.to_string()); }
if let Some(fallback) = fallback_model { cmd.arg("--fallback-model").arg(fallback); }
if let Some(system_prompt) = system_prompt { cmd.arg("--append-system-prompt").arg(system_prompt); }
if let Some(tools) = allowed_tools { cmd.arg("--tools").arg(tools); }
if let Some(mcp) = mcp_config { cmd.arg("--mcp-config").arg(mcp).arg("--strict-mcp-config"); }
if let Some(session_id) = resume { cmd.arg("--resume").arg(session_id); }
```

### What dispatch_direct.rs Does Wrong (lines 140-143)

```rust
// Missing everything:
cmd.arg("--print");
cmd.arg("--output-format");
cmd.arg("stream-json");
// No model, no effort, no system prompt, no tools, no MCP, no resume
```

### Implementation Strategy

**Batch 0: Safety posture** (0.5 day)
- Disable background `serve` for no-args `roko` by default
- OR: enable auth when binding publicly, restrict terminal routes, localhost CORS

**Batch 0.5: Demo truth repair** (0.5-1 day)
- Stop fake success in the demo UI: explicit mock/live mode, no mutation fallback, aligned bench
  and share routes, explicit workdirs, and scenario readiness checks.
- Make `/api/*` misses return JSON 404 instead of the SPA shell.
- Keep terminal transcript visible, but stop using terminal scraping as the source of workflow
  truth.

**Batch 1: ChatAgentSession** (0.5-1 day)
- Small struct holding: auth, workdir, model, effort, system_prompt, tools, MCP config,
  session_id, history, HTTP client
- Owns session state that slash commands mutate
- Lives in `roko-cli`, replaces ad-hoc state in `chat_inline.rs`

**Batch 2: Route Claude CLI through ClaudeCliAgent** (1 day)
- Chat turns invoke `ClaudeCliAgent` or its command builder
- Pass all session fields: model, effort, system prompt, tools, MCP, resume, timeout
- Extract and store session_id from response
- Keep tool output visible

**Batch 3: Streaming** (1 day)
- Parse stream-json lines as they arrive (not after subprocess exit)
- Feed `StreamingState` with text/tool deltas
- Ctrl-C kills child process
- Final metadata (session_id, tokens, cost) captured from result event

**Batch 4: Grounded PRD/plan generation** (1-1.5 days)
- Build a small repository context pack before `prd draft new`, `prd draft edit`, `prd plan`,
  and `plan generate`
- Verify the intended repository root before generation; stop or explicitly enter new-project
  mode when the current root has no matching source tree
- Require generated PRDs to include a `Repository Grounding` section
- Validate plans against existing files and root `Cargo.toml`
- Reject greenfield duplicate-crate plans unless explicitly allowed
- Store artifact validation status separately from subprocess status
- Withhold positive learning and knowledge seeds when artifact validation fails

**Batch 5: API provider context** (deferred to M1)
- Route through existing provider adapters and `ModelCallService`
- System prompt + history instead of bare prompt
- Existing tool loops for providers that support them

**Batch 6: Slash commands + proof** (1 day)
- `/system`, `/model`, `/effort`, `/reset` mutate `ChatAgentSession`
- Observable effect in next request (testable)
- Proof suite: interactive chat plus grounded PRD/plan acceptance tests

### Cost Tracking Fix (Cross-cutting)

The demo proved cost tracking is broken. Fix during Batch 2:

- Claude CLI stream JSON can expose result/usage metadata depending on CLI version and flags.
- `ClaudeCliAgent` currently parses stream JSON for progress and final text, but returns
  `AgentResult` usage with `wall_ms` only.
- The fix is to parse available result usage into `roko_agent::usage::Usage`.
- If usage is unavailable, record it as unknown/null rather than numeric zero.
- Routing chat through `ClaudeCliAgent` is still right, but it does not by itself fix cost
  accounting.

### Workspace Context Fix (Cross-cutting)

The demo proved generated artifacts have insufficient workspace awareness. Fix this in both
Batch 1 and Batch 4:

- `ChatAgentSession.system_prompt` should include a workspace inventory section
- PRD/plan generation should verify the intended repository root instead of assuming the temp
  workspace is the target codebase
- PRD/plan generation should receive a bounded repo context pack before the model runs
- Generated PRDs should include a repository-grounding section
- Generated plans should fail validation if they create duplicate crates or cite no existing files
- `orchestrate.rs` can remain a donor/reference for legacy behavior, but new first-mile work
  should not target it as the primary place for new runtime logic

## Acceptance Criteria

Interactive proof from a fresh terminal:

1. `roko` enters chat
2. User asks: "What files are in this repository root?"
3. Agent uses tools, tool output is visible
4. User asks a follow-up that depends on the previous answer
5. Next Claude turn uses `--resume` (session continuity)
6. `/system <new instruction>` changes the next answer
7. `/effort high` changes the next Claude invocation
8. Long answers stream incrementally
9. Ctrl-C cancels an in-flight turn without orphaned child processes
10. No unauthenticated terminal access is exposed

Artifact proof from a fresh temporary repo:

1. `roko prd draft new <feature>` generates a PRD with `Repository Grounding`
2. The grounding section names existing crates and files
3. `roko prd plan <feature>` generates tasks that modify existing files where appropriate
4. A plan that says "greenfield" for an existing workspace fails validation
5. A plan that creates a new workspace crate without explicit permission fails validation
6. A Roko-internal feature requested from a non-Roko temp workspace fails with a context-root error
   or asks for the intended repo
7. Learning records distinguish `process_success` from `artifact_validation`
8. Positive knowledge seeds are not emitted for failed artifact validation

## What Not To Do

- Do not build new crates (`roko-prompt`, `roko-orchestrate`, `roko-gateway`)
- Do not implement provider tool loops in `dispatch_direct.rs`
- Do not design a cell/graph engine before interactive chat works
- Do not fix all 90 binary issues before making the default path work
- Do not add more architectural scaffolding until the product proof passes

## Time Estimate

5-7 focused engineering days for M0 if it includes both Claude CLI interactive parity and the
first grounded PRD/plan validation pass. Chat-only M0 is still closer to 4-5 days.

## File Quick Reference

| What | Where |
|---|---|
| Current broken dispatch | `crates/roko-cli/src/dispatch_direct.rs:140-143` |
| Correct CLI command builder | `crates/roko-agent/src/claude_cli_agent.rs:305-341` |
| Existing system prompt builder | `crates/roko-compose/src/system_prompt_builder.rs` |
| Current chat REPL state | `crates/roko-cli/src/chat_inline.rs:740-764` |
| Background serve start | `crates/roko-cli/src/unified.rs:45-64` |
| Auth disabled default | `crates/roko-core/src/config/serve.rs:54-57` |
| Anthropic tool loop | `crates/roko-agent/src/provider/anthropic_api.rs:52-64` |
| OpenAI tool loop | `crates/roko-agent/src/provider/openai_compat.rs:360-381` |
| Provider-neutral seam | `crates/roko-cli/src/dispatch_v2.rs:203-229` |
| ModelCallService | `crates/roko-agent/src/model_call_service.rs` |
| PRD prompt | `crates/roko-cli/src/prd_prompt.rs` |
| PRD command handlers | `crates/roko-cli/src/commands/prd.rs` |
| Plan generator prompt | `crates/roko-cli/src/plan_generate.rs` |
| Artifact audit | `tmp/solutions/DEMO-RUN-AUDIT.md` |
| Revised two-rail plan | `tmp/solutions/REVISED-BEST-SOLUTION-AFTER-DEMO.md` |
| Mori reference dispatch | `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2444-2620` |

## Decision Answers (From CLARIFYING-QUESTIONS.md)

1. **First milestone scope**: No-args interactive `roko` plus a narrow PRD/plan grounding and
   validation pass if the demo UI is in scope. `roko run` and full `plan run` follow.
2. **Default provider**: Claude CLI preferred even when API keys are present.
3. **Background serve**: Disabled by default for M0. Re-enable after server auth is hardened.
4. **API tool use**: Not required for M0. System prompt + history enough if Claude CLI works.
5. **Default role**: Read-oriented until edit/write intent is clear. Conservative tool allowlist.
6. **`--dangerously-skip-permissions`**: Only for local workspaces after explicit config.
7. **Mandatory UI features**: Streaming text, tool output, session continuity, status bar.
8. **Canonical plan**: This document (`FINAL-SOLUTION.md`) + `MORI-PARITY-BATCH-PLAN.md`.
