+++
id = "bug-09cc5f"
kind = "bug"
title = "roko plan generate runs its planner under the Claude CLI's default 10-turn cap, so a request that needs more repository reading fails"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "787041389"
source = "gap-e1fdb3 live check, 2026-10-16 (5 of 18 planner calls capped; the serve-route request failed 3/3)"
discovered_from = "gap-e1fdb3"
anchors = ["crates/roko-cli/src/agent_exec.rs::run_agent_capture_impl", "crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-e1fdb3", "gap-11d870"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn planner_call_gets_a_turn_cap_above_the_default' crates/roko-cli/src && cargo test -p roko-cli --lib -- planner_call_gets_a_turn_cap_above_the_default"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T20:19:35Z"
commit = "787041389"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:20Z"
forced = false
evidence = "787041389: every planner call runs under PLANNER_MAX_TURNS = OperatingFrequency::Delta.turn_limit() (30), passed AgentExecOpts.max_turns -> SpawnAgentSpec.max_turns -> AgentOptions.max_turns (agent_exec.rs, agent_spawn.rs). generate_plan builds the planner's options once, so its first call, crash retry, validation retry and both regenerations carry the cap (plan_generate/pipeline.rs); revise_plan_source gets it too (plan_authoring.rs). classify_agent_crash names a turn-cap hit (detect_turn_cap) AgentCrashClass::TurnCapReached, before the status-code substrings; it is not retried, and the error says the planner ran out of turns where it said \"Agent crashed for an unknown reason\". Checked: cargo test -p roko-cli --lib planner_call_gets_a_turn_cap_above_the_default passes (a fake claude_cli is started with --max-turns 30 on the first call and on the validation retry; before, the CLI got its default --max-turns 10), as do a_planner_out_of_turns_is_not_asked_again (1 call, the error quotes \"agent turn cap reached (turns=31, cap=30)\") and classify_agent_crash_turn_cap; the plan_generate::, plan_authoring::, agent_exec:: and agent_spawn:: lib tests pass (129); cargo clippy -p roko-cli --no-deps -- -D warnings is clean; nightly fmt. Not run: the live serve-route rerun in Done when (no paid calls in this round)."
+++

## Problem

`roko plan generate` runs its planner (`[authoring] planner_model`, claude-opus via claude_cli) with no turn cap of
its own. The Claude CLI agent then keeps its built-in default, `--max-turns 10` (`OperatingFrequency::Theta`). An
opus planner that reads a large repository with Read/Grep/Glob reaches that cap before it writes the plan. The CLI
ends with `error_max_turns`, roko classes it `Unknown`, retries the same prompt twice, and the generation fails.

Seen in gap-e1fdb3's live check (2026-10-16, roko built from main 5e32264ce, scratch copy of main, Max login):
- 5 of 18 planner calls ended `agent turn cap reached (turns=11, cap=10)` after 48–68 s each.
- The serve-route request ("Add a read-only HTTP route to roko serve that returns the workspace's active price
  snapshot ...") hit the cap on all 3 calls: `error: plan generation agent failed after 3 attempts ... The planner
  said: agent turn cap reached (turns=11, cap=10)`. It produced a plan only when it was rewritten as a short spec
  naming its files, and even then 1 of 3 calls hit the cap.
- The CLI-flag request lost 1 of its 3 calls to the cap.

## Why it matters

Plan generation is the golden path's first step (`roko run --plan`, `roko plan generate`). Requests that need more
reading than ten tool turns, such as a route that spans roko-serve and roko-cli, fail outright. Each capped call
costs a full opus call: $0.39–0.60 API-equivalent here, and about a minute of subscription time.

## Where

- `crates/roko-cli/src/agent_exec.rs::run_agent_capture_impl`: its `SpawnAgentSpec` carries no turn cap, so
  `AgentOptions::max_turns` is None.
- `crates/roko-agent/src/claude_cli_agent.rs`: `ClaudeCliAgent::new` defaults `max_turns` to
  `OperatingFrequency::Theta.turn_limit()` (10). `provider/claude_cli.rs` overrides it only when the options carry one.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the planner call, its crash retries (a cap hit
  counts as `Unknown`, retried twice with the same prompt) and the spec-quality regeneration.

## Current state

No config key reaches this cap: `[pipeline.<tier>] max_turns` applies to Graph tasks only. gap-11d870 (done) made a
timed-out planner call report as a timeout. Turn-cap hits still read "Agent crashed for an unknown reason".

## Plan

1. Give the planner call an explicit turn cap that fits planning a large repository (for example
   `OperatingFrequency::Delta`, 30, or an `[authoring] planner_max_turns` key), passed through `AgentExecOpts` into
   `SpawnAgentSpec`.
2. Classify `error_max_turns` as its own crash class, never `Unknown`, so the error says the planner ran out of turns.
   A retry with the same cap is unlikely to finish.
3. Test it: the planner call's options carry the cap.

## Done when

- [ ] `planner_call_gets_a_turn_cap_above_the_default` passes (the `[[verify]]` command).
- [ ] A live `roko plan generate` of the serve-route request above, given as a prompt, writes a plan.

## Notes

- Evidence: `tmp/status-quo/2026-10-16-live/small-live.md` (item 3) and the scratch run's `.roko/roko.log`.
