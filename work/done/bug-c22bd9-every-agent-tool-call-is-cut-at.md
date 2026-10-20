+++
id = "bug-c22bd9"
kind = "bug"
title = "Every agent tool call is cut at 60 s: ToolLoopAgent's tool timeout defaults to 60 s and is never set from [timeouts] tool_execution_ms, and bash ignores its advertised timeout_ms, so an agent can't finish a build longer than a minute"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/tool-loop"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "b77db15c4"
source = "round-v7-hermes-pilot#C-p0-run1"
discovered_from = "agent:hermes-pilot (round v7 phase 0, run 6988f0b8, T3 try 2)"
anchors = ["crates/roko-agent/src/tool_loop/agent_wrapper.rs::ToolLoopAgent", "crates/roko-agent/src/tool_loop/context_factory.rs::ToolExecutionContextFactory", "crates/roko-std/src/tool/builtin/bash.rs::run", "crates/roko-core/src/config/timeouts.rs::tool_execution"]
links = { depends_on = [], blocks = [], related = ["bug-872885", "gap-5c3347", "bug-d758f6", "gap-770cb5", "gap-6754ab"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_bash_call_runs_past_sixty_seconds_when_asked' crates/ && cargo test -p roko-agent -- a_bash_call_runs_past_sixty_seconds_when_asked"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T14:46:21Z"
commit = "b77db15c4"
size = "S"
claimed_at = "2026-10-19T14:02:35Z"
forced = false
evidence = "ToolContext::call_timeout gives a call to a tool that takes timeout_ms (bash, run_tests) what it asks for, up to the new [timeouts] tool_execution_max_ms (default 600000 ms), else [timeouts] tool_execution_ms (now wired: create_agent_for_model fills AgentOptions tool_timeout/tool_timeout_max from config and every roko tool-loop adapter applies them; the hard-coded 60 s defaults in ToolLoopAgent and the context factory are now 120 s / 600 s). bash and run_tests enforce it themselves; the dispatcher gives such tools the call budget plus 5 s grace instead of min(context, definition); a longer ask is cut to the cap and a call that then runs out says so. Verify: cargo test -p roko-agent -- a_bash_call_runs_past_sixty_seconds_when_asked passes (a real dispatcher plus the real bash tool run sleep 70 with timeout_ms 90000 to completion in 70.04 s); a_bash_call_asking_past_the_cap_is_cut_at_it_and_told, roko-core call_timeout_honours_the_ask_up_to_the_cap and the roko-std bash/run_tests tests pass (123 roko-agent, 9 roko-core, 19 roko-std lib tests); clippy --no-deps -D warnings clean on roko-core, roko-std, roko-agent, roko-dreams, roko-serve, roko-cli; nightly fmt clean."
+++

## Problem

Every tool call an agent makes is cut at 60 seconds, whatever the call or the config asks for.

Seen in round v7, Undertow phase 0, run `6988f0b8` (roko 3b34aed92, the plan's T3: the JUCE plugin):
- The agent ran `cmake --build --preset plugin` through bash. A cold JUCE plugin build takes more than 60 s
  (about 77 s at 4 jobs). Every call came back `error: timed out after 60001 ms`, about 10 times in try 2, including
  calls that asked for `"timeout_ms": 600000`.
- The agent tried `nohup … &`, `( … ) &`, `timeout 120 …` and raw `make -C build/plugin`, which bypasses the round's
  cmake shim. It never saw its build finish, and used all 45 of its turns on mid (1,246 s, $0.23).
- The gate's own build then failed on a compile error (`juce::Optional<double>::value_or`) that a single finished
  build would have shown the agent.
- Try 3 (strong, gpt-5.4-mini) handed off after 11 turns without a finished build and failed on 3 compile errors.
  The task ended "failed on its top ladder rung … split or replan it".

## Why it matters

Any compiled project with a build longer than a minute is affected: JUCE, a fresh Rust crate, a large TypeScript
build. The agent can't see its own compile errors, so each one costs a whole try and pushes a climb. The cheap rungs
take the blame for a harness limit, which defeats cheap-first.

## Where

- `crates/roko-agent/src/tool_loop/agent_wrapper.rs`: `ToolLoopAgent` defaults `timeout: Duration::from_secs(60)`
  (about line 81). `with_timeout` (about line 165) overrides it, but no production caller uses it.
  `context_factory()` passes the timeout to `ToolExecutionContextFactory`, whose own default is also 60 s
  (`crates/roko-agent/src/tool_loop/context_factory.rs`, about line 67).
- `crates/roko-core/src/config/timeouts.rs`: `[timeouts] tool_execution_ms` (default 120000) is read only by tests.
  `tool_execution()` has no production caller.
- `crates/roko-std/src/tool/builtin/bash.rs::run` uses `ctx.timeout` and ignores the call's `timeout_ms`, which its
  schema advertises: "Optional wall-clock timeout in milliseconds (default: 120000)".

## Plan

1. Build the attempt's agent with `with_timeout(config.timeouts.tool_execution())`.
2. Have bash honour `timeout_ms`, capped by `[timeouts] tool_execution_ms` (or a separate cap such as
   `bash_max_ms`) and by the time the attempt has left.
3. Make the schema's description state the real default and cap.

## Done when

An agent's bash call that asks for `timeout_ms: 300000` runs until its command ends, or until 300 s pass, and no
longer stops at 60 s. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p0-run1/` (attempts.jsonl and the T3 try 2 tool stream).
- Related:
  - bug-d758f6: planners strip bash;
  - gap-770cb5: no CMake build or check tool;
  - gap-6754ab: cold worktree builds.
