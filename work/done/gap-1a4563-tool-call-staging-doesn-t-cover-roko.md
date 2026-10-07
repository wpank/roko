+++
id = "gap-1a4563"
kind = "gap"
title = "Tool-call staging doesn't cover roko graph run, chat-host-direct plan submission, or CLI-agent backends"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-cli/graph", "roko-cli/task-parser", "roko-agent/dispatcher"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "e37dd2c5e"
source = "wave-7 follow-up reports 2026-10-03 (PK76 gap-99c9ae)"
discovered_from = "gap-99c9ae"
anchors = ["crates/roko-cli/src/commands/graph.rs", "crates/roko-cli/src/task_parser.rs::TaskDef", "crates/roko-agent/src/dispatcher/production_safety_chain.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn direct_plan_submission_from_a_chat_host_sets_stage' crates/roko-cli/ && cargo test -p roko-cli direct_plan_submission_from_a_chat_host_sets_stage"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T14:16:01Z"
commit = "e37dd2c5e"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T12:50:29Z"
forced = false
evidence = "Gate 18 (merged e37dd2c5e): verify direct_plan_submission_from_a_chat_host_sets_stage passes, plus a_run_floor_raises_allow_and_keeps_deny, graph_run_refuses_agent_work_it_cannot_grant_and_says_why and agents_that_run_their_own_tools_cannot_take_a_staged_contract. Chat-host plans get an outbound floor of stage; roko graph run refuses agent work it cannot grant; own-tool agents (CLIs, ACP, Hermes, OpenClaw) refuse a stage/deny contract so failover moves the task to an API provider or fails closed (a behavior change for CLI-only workspaces, reported to Will)."
+++

## Problem

Three places where roko's "hold a tool call for approval" staging (the `[meta] outbound = "stage"` policy and
the production safety chain's 9-stage pipeline, `crates/roko-agent/src/dispatcher/production_safety_chain.rs`)
doesn't actually cover a real entry point, surfaced by PK76 (gap-99c9ae, done):

1. **`roko graph run` grants no llm capability by default.** Confirmed: `crates/roko-cli/src/commands/graph.rs`'s
   own test `graph_policy_missing_capabilities_defaults_to_empty` (~line 553) and
   `graph_policy_capabilities_round_trip_through_toml` (~524, checking for `roko_core::Capability::Llm`) show a
   graph policy with no explicit capabilities grants none — so `agent.task` and `plan.run` (roko-std tool
   definitions that need the Llm capability to dispatch an agent) refuse when `roko graph run` is used directly,
   without a policy TOML explicitly granting `Capability::Llm`.
2. **A plan run a chat host starts directly (not through `roko run`) gets no stage policy.** `task_parser.rs`'s
   own doc comment on `TaskDef.outbound` (~line 78): "A chat host's `roko run` sets `stage`. Unset, each task's
   domain decides." — `stage` is set specifically by the `roko run` code path. A plan submitted directly (e.g.
   via `roko plan run <dir>` on a hand-written or externally-generated `tasks.toml`, or a server route that
   doesn't go through `roko run`'s own plan-authoring step) never gets this field set, so its tasks fall through
   to "each task's domain decides" instead of the chat-host-safe `stage` default.
3. **CLI agents (Claude Code, Codex) run their own tools, so staging can't hold their calls.** This is an
   architectural limitation, not new: Claude Code's and Codex's own tool loops execute outside roko's dispatcher
   entirely (Codex specifically runs under its own "operation broker," per earlier work on gap-843aef/bug-248670),
   so the production safety chain's stages (which intercept tool calls at roko's own dispatch layer) never see a
   CLI agent's tool calls to stage them. Recorded here as a known gap in staging's actual coverage, since PK76
   flagged it alongside (1) and (2) as part of the same "where does staging actually apply" audit.

## Why it matters

Goal: release/safety. The whole point of `stage` is "a person reviews a tool call that acts on the outside world
before it runs." If (1) refuses the dispatch outright (fails closed — safe, but breaks a legitimate `roko graph
run` user who didn't know to grant the capability), (2) silently falls through to a less-safe default for a
chat-host-originated plan that *should* have gotten the safe default, and (3) means staging never even sees
CLI-agent-executed tool calls at all — together these mean "staging holds outbound tool calls" is true only on
one specific path (`roko run`, through roko's own tool loop, not a CLI agent's), not universally.

## Where

- `crates/roko-cli/src/commands/graph.rs` (graph policy capabilities).
- `crates/roko-cli/src/task_parser.rs::TaskDef.outbound` (where `stage` is set, and by what).
- `crates/roko-agent/src/dispatcher/production_safety_chain.rs` (the 9-stage pipeline itself, which (3) never
  reaches for CLI-agent tool calls).

## Current state

All three confirmed as described; none fixed.

## Plan

1. For (1): either make `roko graph run` warn loudly when a graph needs `agent.task`/`plan.run` but no policy
   grants `Capability::Llm` (rather than a bare refusal), or default-grant it for interactive use with an
   explicit opt-out for locked-down deployments — a design choice, not just a code change.
2. For (2): make whichever entry point submits a plan directly (not through `roko run`) set `stage` the same way
   `roko run` does when the request's origin is a chat host (`RunOrigin::Mcp`/similar), so the safe default
   follows the origin, not just the specific CLI command used.
3. For (3): no code fix proposed here — this is an architectural limitation of CLI-agent backends needing its own
   decision (a different confinement layer per-CLI, since roko's own staging can't reach inside their tool
   loops). Record it so it's visible rather than assumed covered.

## Done when

- `roko graph run` either grants or clearly explains the missing `Capability::Llm`, instead of a bare refusal
  with no hint.
- A plan submitted directly by a chat host gets the same `stage` default a `roko run`-originated one would.
- The `[[verify]]` command passes (covering 1 and 2; 3 is a recorded limitation, not something this item's
  verify can close).

## Notes

- (3) may need its own follow-up item once a design for CLI-agent-side confinement exists; this item just
  records the gap as PK76 found it.

## Decision

- 2026-10-05, Will: keep it failing safely. Chat-started (and ops-domain) work needs an API provider, whose tool
  loop can hold outward actions for approval; own-tool CLI agents refuse a stage/deny contract. Terminal use is
  unaffected.
