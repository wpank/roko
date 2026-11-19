+++
id = "bug-7d13f0"
kind = "bug"
title = "A provider CLI that exits before reading its prompt is misreported by timing: the prompt write's broken pipe fails the run as 'stdin write failed', so an empty answer is no empty_response and an exit code and stderr are lost (immune_boundary's factory test flakes on Linux)"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-agent/exec"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "a3e0039d6"
source = "round v8 lead: release PR #89's Test + Clippy run 44574043691 (attempt 1) on Linux"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-agent/src/exec.rs::ExecAgent", "crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent", "crates/roko-cli/src/chat_session.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-d85634"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn write_prompt' crates/roko-agent/src/process/stdin.rs && cargo test -p roko-agent --lib -- write_prompt exits_before_reading"

[closed]
at = 2026-11-19
at_ts = "2026-11-19T18:33:05Z"
commit = "a3e0039d6"
forced = false
evidence = "a3e0039d6 on work/bug-7d13f0. PR #89 Test + Clippy on Linux (run 44574043691 attempt 1) failed immune_boundary canonical_provider_factory_installs_boundary_automatically (provider_error None, not empty_response). Cause, a product race: ExecAgent::run (and ClaudeCliAgent::run, and roko chat CLI turn) failed the run as \"stdin write failed\" whenever the prompt write broke the pipe, before looking at the agent exit; an agent that exits before reading its prompt breaks the pipe only if its exit comes first, else the prompt waits in the pipe and the run is judged by exit status and output (an empty answer, which the boundary makes empty_response; an exit code with stderr). Fix: roko_agent::process::write_prompt counts a broken pipe as written (the child stopped reading; its exit decides the run), other write errors still fail; the three spawns use it. bug-62b77a does not touch this path (provider CLIs get the credential scrub). Red first, deterministic with a 4 MiB prompt to an agent that never reads it: the factory exit-0 agent came back \"stdin write failed: Broken pipe (os error 32)\" with provider_error None (the CI failure), ExecAgent exit 3 with stderr lost its \"exit 3: unknown flag: --frobnicate\", a fake Claude Code exiting 1 with \"Invalid API key\" lost it too. Green: those 3 and write_prompt own test pass; roko-agent lib 2442 passed; the once-flaky factory test 20 of 20 run 4 at a time; roko-cli chat_session tests 64; workspace clippy clean."
+++

## Problem

`ExecAgent::run` and `ClaudeCliAgent::run` spawn the agent, write the prompt to its stdin and then wait for it. If the
write fails, they kill the agent and fail the run with "stdin write failed: <error>", before they look at the agent's
exit status or output.

An agent that exits before it reads its whole prompt (an instant crash, an argument or auth error, a script that
ignores its input) makes the write fail with a broken pipe (EPIPE), but only when the exit comes first:

- **The write wins:** a prompt that fits in the pipe's buffer is written into it, and the run goes on. The agent's exit
  status and output decide it, so an empty answer becomes the boundary's `empty_response`, and an exit 1 becomes
  "exit 1: <stderr>".
- **The exit wins:** the run fails as "stdin write failed: Broken pipe". That carries no `provider_error` tag, and the
  agent's own exit code and stderr are lost. A prompt bigger than the pipe's buffer always ends this way.

Seen on 2026-11-19 in release PR #89's Test + Clippy on Linux (run 44574043691, attempt 1).
`immune_boundary::tests::canonical_provider_factory_installs_boundary_automatically` failed at `immune_boundary.rs:2356`
with left None, right Some("empty_response"). Its fallback agent is `/bin/sh -c "exit 0"`, and it exited before the
prompt was written. The test passed on Linux at 16aa66348, and it passes on macOS.

## Why it matters

Goal core. How a run fails depends on timing, so retries, failover and the ladder get a different error for the same
agent behaviour. A provider CLI that dies at startup reports a broken pipe instead of its own error: an auth failure
reads as a pipe error. In CI it is a flake.

## Where

- `crates/roko-agent/src/exec.rs::ExecAgent::run`: the stdin write.
- `crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent::run`: the same write.
- `crates/roko-cli/src/chat_session.rs`: the chat's CLI turn, the same write.

## Plan

1. `roko_agent::process::write_prompt`: write the prompt to a child's stdin and close it. A broken pipe means the
   child stopped reading, which is not a failure in itself: the run goes on, and the child's exit status and output
   decide it. Other write errors still fail the run.
2. The three spawns use it.
3. Red first, deterministically: a prompt bigger than any pipe buffer, given to an agent that never reads it, always
   breaks the pipe. With today's code, a silent `exit 0` agent is not an `empty_response`, and a CLI that prints an
   error and exits 1 is reported as "stdin write failed".

## Done when

An agent that exits before reading its prompt is judged by its exit status and output, whichever way the timing goes,
and the `[[verify]]` passes.
