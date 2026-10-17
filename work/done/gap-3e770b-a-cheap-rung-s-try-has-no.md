+++
id = "gap-3e770b"
kind = "gap"
title = "A cheap rung's try has no bound of its own: one gpt-oss-120b attempt looped 30 turns ($0.159) until the plan cap stopped it, blamed on the harness, so the ladder never climbed"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-core/config"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "69361cb50"
source = "tmp/hermes-roko-v6/findings/plans-research.md §3.3 (R1 pocket-synth run, 2026-10-16); round v6 brief"
discovered_from = "plans-research proposed item 7"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::dispatch", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::record_attempt_ladder", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched", "crates/roko-core/src/config/routing.rs::LadderConfig"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-460230", "bug-4fabd0", "gap-ca1c92", "gap-e14344"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_turn_bounded_try_climbs_the_ladder' crates/roko-cli/src && grep -rqw 'fn a_spend_bounded_try_climbs_the_ladder' crates/roko-cli/src && cargo test -p roko-cli --lib bounded_try_climbs_the_ladder"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T11:16:36Z"
commit = "b0b57f6dd"
forced = false
evidence = "b0b57f6dd: a try on a rung the task can still climb from is bounded. [routing.ladder] max_turns and max_usd set a rung's allowance by name; by default each rung below the highest one the task can reach gets half the turns of the rung above it, from the tier's cap (focused 60: cheap 15, mid 30), never fewer than 12, and the highest rung and a pinned model keep the tier's cap. The bound applies once the attempt is routed, so a retry on the rung keeps it, with the resume note. A try's calls draw on the plan's hold first, then the rung's max_usd; a rung stop (roko's tool loop, the streamed-spend watch, or the Claude CLI at --max-budget-usd) settles turn_cap, blamed on the agent (reason rung_allowance), and is retried with a resume note; a plan stop stays budget_exhausted. a_turn_bounded_try_climbs_the_ladder and a_spend_bounded_try_climbs_the_ladder pass. Live run (pocket-synth from the plans-research showcase, binary 69361cb50, roko init --profile python defaults, no [pipeline.focused] override, --budget-override 0.60): ENV passed on cheap gpt-oss-120b (11 turns, $0.036); OSC hit its 15-turn bound ($0.069), then passed on its second cheap try (5 turns, $0.011); SVF hit the 15-turn bound twice on cheap ($0.057, $0.066) and climbed to mid glm-4.7 (route_decided ladder_reason climb, ladder_step 1), where the immune boundary denied its try after 136 s (provider_stream_limit_exceeded, R3-1); the run was stopped by SIGTERM during the next mid try, $0.2442 recorded. Before (R1, plans-research §3.3): $0.2477 under a $0.25 cap; SVF ran one 30-turn attempt ($0.159) until the plan cap stopped it as budget_exhausted, blamed on the harness, and never climbed. Checks at 69361cb50 (the stack rebuilt on main 9df2f5948): fmt --check, clippy -D warnings (roko-core, roko-cli, roko-serve), lib tests roko-core 2038 passed, roko-serve 1052 passed, roko-cli 3769 passed with one known load flake (plan_check_publishes_start_heartbeat_and_finish, bug-8d8f98, passes alone); the [[verify]] passes."
+++

## Problem

A try on a cheap rung has no bound of its own, so the ladder never gets to climb. Seen live on 2026-10-16
(`tmp/hermes-roko-v6/findings/plans-research.md` §3.3, run R1 on the pocket-synth showcase, `--budget-override 0.25`,
`max_task_usd = 0.12`): task SVF started on the cheap rung (gpt-oss-120b). Its filter narrowly missed two passband
checks (`test_lowpass_passes_lows` 0.7988 vs > 0.8, `test_bandpass_mode` 0.386 vs > 0.4). The model ran the tests
itself and kept editing for 30 turns ($0.159) inside its one attempt, until the plan cap stopped it. The attempt
settled `budget_exhausted`, blamed on the harness, so it did not count against the rung, and the run ended with no
climb and RENDER skipped.

Three things let that happen:
- a focused task's attempt gets the tier's full turn cap (`[pipeline.focused] max_turns = 60`) on every rung;
- `max_task_usd` is checked only when an attempt opens (`admit_task_budget`), never inside one;
- a stop for the budget settles as `BudgetExhausted` (`Blame::Harness`), which `note_ladder_outcome` does not count.

Expected: a cheap rung gets a bounded try. A try that uses up its rung's turns or spend counts as a failed try on
that rung (blame: the model), and two such tries climb under roko's existing rule (`FAILURES_PER_RUNG = 2`). The
run's cap still ends everything and has the last word.

## Why it matters

This is the pitch (round v6 brief): cheap models first, escalate when the acceptance checks fail, and learn. Today a
cheap model that loops spends the run's budget on one try, and the stronger rung never runs.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::dispatch`: computes the attempt's `max_turns` from the tier, routes
  the attempt (`plan_dispatch`, `record_attempt_ladder`), builds the request and its `allowance`, and settles a
  failed provider result (`budget_stop`, the turn-cap branch).
- `crates/roko-cli/src/graph_task_dispatch/streaming.rs`: the streaming path, the same steps.
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::record_attempt_ladder`: the attempt's rung and step; `top`
  says whether the task can still climb.
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched`: stops an agent that runs its own loop once what
  it streamed reaches what the plan can pay for (gap-ca1c92).
- `crates/roko-core/src/config/routing.rs::LadderConfig`: `[routing.ladder]`.

## Current state

The plan cap holds inside an attempt (bug-4fabd0, gap-ca1c92, gap-e14344): roko's tool loop asks the plan's
`AttemptAllowance` before each model call, a CLI agent is priced as it streams, and the Claude CLI also stops itself
at `--max-budget-usd`. A turn-cap stop is already `TurnCap`, blamed on the agent, and counts toward a climb; the next
try then gets half again as many turns and a resume note (`raised_turn_cap`, `turn_cap_resume_note`).

## Plan

1. `[routing.ladder]` gains two per-rung maps, by rung name: `max_turns = { cheap = 12 }` (turns per try) and
   `max_usd = { cheap = 0.06 }` (spend per try, USD).
2. A try on a rung the task can still climb from gets fewer turns by default: half the turns of the rung above it,
   from the tier's cap on the highest rung it can reach (at most `MAX_ESCALATIONS` up), and never fewer than 12. The
   top rung, and a pinned model, keep the tier's cap as today. A rung named in `max_turns` gets exactly that.
3. Apply the rung's turns once the attempt is routed, as `min(max_turns, rung turns)`, so a second try on the same
   rung keeps the same allowance (the turn-cap resume note stays). A turn-cap stop is already `TurnCap` (agent).
4. Wrap the plan's `AttemptAllowance` and the rung's spend allowance in one allowance for the try. The plan is asked
   first, so its cap has the last word. A stop by the rung's allowance (roko's tool loop refusing the next call, the
   streamed-spend watch, or the Claude CLI stopping at the lower limit it was given) settles as a failed try on the
   rung (`TurnCap`, agent-blamed, reason `rung_allowance: …`) and is retried; a stop by the plan stays
   `BudgetExhausted`.
5. Both dispatch paths.

## Done when

- A try that uses up its rung's turns, or its rung's spend, fails on that rung, and the second such try moves the
  task up one rung: `a_turn_bounded_try_climbs_the_ladder` and `a_spend_bounded_try_climbs_the_ladder` pass.
- The plan cap still stops a try first when it is lower.
- A live pocket-synth run under a $0.60 cap: SVF climbs instead of looping.
- The `[[verify]]` command passes.

## Notes

- Stay off `budget.rs::record_task_spend` and `GraphTaskSpendLedger` (bug-1bc222) and `dispatch/model_routing.rs`
  (bug-3e2d76): read the ladder through `RoutingLadder`'s public methods and the config.
- `max_task_usd` still applies only when an attempt opens; with a bounded try per rung one try can overrun it by at
  most the rung's allowance. A task ceiling below two cheap tries plus one higher try stops the climb at admission:
  size `max_task_usd` for the ladder, or leave it unset.
- Related: proposed item 7 of plans-research §5, gap-460230 (the ladder), bug-4fabd0 (the plan cap in an attempt).
