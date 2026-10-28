+++
id = "gap-31e8a7"
kind = "gap"
title = "roko's own bash and run_tests tools, and the Codex, Cursor, Hermes and OpenClaw agents, set no variable that tells roko safety release it runs for an agent, so a script an agent writes can still release a control"
status = "done"
triage = "verified"
severity = "p3"
size = "M"
subsystem = ["roko-cli/commands", "roko-std/tool", "roko-agent/safety"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "983024e81"
source = "find-d05b5e fix (2026-10-26)"
discovered_from = "find-d05b5e"
anchors = ["crates/roko-cli/src/commands/safety.rs::roko_started_markers", "crates/roko-std/src/tool/builtin/bash.rs::run", "crates/roko-core/src/child_env.rs::apply_gate_env"]
links = { depends_on = [], blocks = [], related = ["find-d05b5e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_release_from_a_tool_command_is_refused' crates/ && cargo test -p roko-cli --lib a_release_from_a_tool_command_is_refused"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T20:31:41Z"
commit = "983024e81"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-26T16:06:45Z"
forced = false
evidence = "ROKO_AGENT_ID (roko_core::child_env::AGENT_ID_ENV via mark_agent_process; \"agent\" when no id is at hand) is set on the commands roko-std bash and run_tests run (beside apply_gate_env, from the ToolContext correlation agent id), on ACP bash (the call agent, else the session) and on every CLI agent spawn: runner::agent_stream (plan-path Claude and Gemini), ClaudeCliAgent, the roko chat streaming turn, ExecAgent (Codex, roko-agent Gemini CLI), CursorCliAgent, the harness ChildProcessRunner (Hermes one-shot, OpenClaw infer) and AcpStdioClient (Cursor, Hermes and OpenClaw ACP). roko_started_markers counts it, so roko safety release refuses under it (crates/roko-cli/src/commands/safety.rs). Option 3: claude_cli_guard.py denies a Bash call whose words run roko safety release. Option 2 left out. Checks: cargo test -p roko-cli --lib a_release_from_a_tool_command_is_refused passes (the real bash tool writes and runs a script whose roko stand-in records its environment, and the release refuses under it); safety_cli 5/5 with safety_release_refuses_under_an_agent_id; the mark test at each spawn site and settings_hook_denies_roko_safety_release pass; clippy --all-targets on roko-core, roko-std, roko-agent, roko-acp and roko-cli reports nothing on changed lines (its other errors predate the change); the built roko refuses under ROKO_AGENT_ID and runs the release without it. Merge waits on Will (coordinator, 2026-10-26)."
+++

## Problem

find-d05b5e made `roko safety release` refuse to run in a process roko started for an agent or a check. It tells
them by the variables roko already sets on them (`crates/roko-cli/src/commands/safety.rs::roko_started_markers`):
the Claude Code agents' `ISOLATION_ENV` pair, the `ROKO_GATE_*` rung variables, `ROKO_TOOL_ARGS`,
`ROKO_EFFECT_FILE`, `ROKO_CELL_INPUT`, and a provider config in a `roko-cli-provider-*` directory (Gemini).

Two kinds of agent process carry none of them:

- the commands roko's own `bash` and `run_tests` tools run, which every API-model agent uses (OpenAI-compatible
  providers such as glm and gpt-5.4-mini, the Anthropic API, the Gemini API, Cerebras). They get the gate
  environment (`roko_core::child_env::apply_gate_env`) and nothing that marks them; ACP's `bash` is the same;
- the Codex, Cursor, Hermes and OpenClaw CLI agents. Roko sets only `CARGO_INCREMENTAL` and `CARGO_BUILD_JOBS` on
  them, which a person's shell may hold too.

In their shells only the bash policy stops the release (`crates/roko-agent/src/safety/bash.rs`,
`DenyPattern::RokoSubcommand`), and it sees only what a command's shell words spell. A script the agent writes
and then runs, a name held in a variable, or an encoded command gets through. The release then lands, with `by`
taken from `$USER`. Its audit line's `via` does say `by_source = "user_env"` and `terminal = false`, so the line
is honest, but the control is gone.

## Why it matters

Only a person releases an immune control (find-d05b5e's decision). For the API-model agents, which run most of
roko's tasks, the in-environment refusal is missing, and the bash rule is a filter on spellings, not a boundary.

## Where

- `crates/roko-core/src/child_env.rs::apply_gate_env`: the environment `bash` and `run_tests` commands get.
- `crates/roko-std/src/tool/builtin/bash.rs::run` and `crates/roko-std/src/tool/builtin/run_tests.rs`: the tool
  handlers; `crates/roko-acp/src/builtin_tools.rs` for ACP's `bash`.
- `crates/roko-cli/src/runner/agent_stream.rs` (CLI agent spawns), `crates/roko-agent/src/exec.rs`,
  `crates/roko-agent/src/cursor_cli_agent.rs` and `crates/roko-agent/src/harness/`: the other agent spawns.
- `crates/roko-cli/src/commands/safety.rs::roko_started_markers`: the check to extend.

## Current state

find-d05b5e's coordinator call was to use only markers that already exist, so no new one was added. Unchanged
since then.

## Plan

Options, not exclusive; the first needs a decision, since it adds a marker:

1. Roko sets one variable on every agent process and on the commands its tools run, say `ROKO_AGENT_ID=<agent
   id>`: in `bash` and `run_tests` (next to `apply_gate_env`), ACP's `bash`, and every CLI agent spawn. Then
   `roko_started_markers` checks it. This also gives tests and tools a single way to tell an agent's process.
2. `roko safety release` requires a terminal on its standard input (a person typing), and leaves scripts to the
   serve route. An agent can still allocate a pseudo-terminal (`script`), so this only raises the bar.
3. The Claude CLI guard (`crates/roko-agent/src/claude_cli_guard.py`) refuses `roko safety release` as well, so a
   Claude agent's command is stopped before it runs. The marker already refuses it in a Claude agent's shell.

## Done when

- A `bash` tool command run for an API-model agent cannot release a control through a script it wrote, and a test
  shows it (`a_release_from_a_tool_command_is_refused`, run by the `[[verify]]`).

## Notes

- Do not invent the marker without a decision: find-d05b5e's call was existing markers only.
- `crates/roko-cli/tests/common/mod.rs::Isolated` drops every inherited `ROKO_*` variable from a test's `roko`. A
  `ROKO_*` marker would leave the CLI tests unaffected; a non-`ROKO_` one would need adding to
  `crates/roko-cli/tests/safety_cli.rs::roko`.
- Coordinator 2026-10-26: option 1 (ROKO_AGENT_ID) built on work/gap-31e8a7 for Will's review; merge waits on his decision.
