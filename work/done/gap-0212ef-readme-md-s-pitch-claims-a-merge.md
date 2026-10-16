+++
id = "gap-0212ef"
kind = "gap"
title = "README.md's pitch claims a merge queue with back-out, automatic split and replan, a controller that retunes itself and routing from what has passed, beyond what the code and both papers describe"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["docs/readme"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "da707bd99"
source = "status-quo-2026-10-15#finding-33"
anchors = ["README.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-d76b1b", "q-6b7cca", "gap-ac1aed", "gap-ebd656", "bug-700d7c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -Eq 'merges through a queue|merged through a queue|merge queue|is backed out|then split or replan the task|retunes itself when they drift|comes from what has actually passed' README.md && grep -q 'run branch' README.md"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T14:10:09Z"
commit = "da707bd99"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:07:02Z"
forced = false
evidence = "README.md at da707bd99, each claim checked against the code at 5e32264ce. Delivery: finished plans merge into one run branch, which keeps a plan only if the whole-plan check passes on the merge (Why Roko, comparison table, Verify > Whole-plan checks; the crate table says 'plan delivery'), as graph_execution/batch.rs::deliver and delivery.rs::git_merge/run_regression do; the merge queue was deleted (runner/merge.rs). Escalation: a task that still fails at the top of its ladder goes to a person, who decides whether to split or replan it (graph_task_dispatch/ladder.rs, ladder_exhausted; ReplanController has no production caller). Routing: the forecaster learns from verified results (shadow by default, config/self_model.rs), each task starts on its tier's ladder rung and climbs only on failure, and the forecaster picks the start rung once calibrated and switched on (graph_task_dispatch/self_model.rs); the same claim is fixed in the comparison table, How it works and Providers (dispatch/model_routing.rs precedence). Self-regulation: the four essential variables are tracked against the policy file, and a move applies one notch at a time only once a person switches the controller on (config/homeostasis.rs, shadow by default). Verify: exit 1 before, exit 0 at da707bd99. Filed bug-700d7c (59a083690): delivery moves the run branch before the check and resets it on failure, which the papers rule out."
+++

## Problem

The README's pitch ("Why Roko", "Roko and Claude Code") and its feature list describe four things neither the code
nor the two papers do (status-quo assessment 2026-10-15 at 7b6a5af44, finding 33;
`tmp/status-quo/2026-10-15/papers/F-paper-vs-paper.md` section 3, rows P1-P4 and P6):

- **A merge queue** (README:49 "finished work merges through a queue", :65 "merged through a queue", the crate
  table at :374 "merge queue"). Graph runs build each plan on its own branch and deliver plans to the run branch one
  at a time (WP 04:65-74; RP 07:51-62); FRAMING §7.7 says not to describe the old file-locked merge queue.
- **Back-out** (README:101-102 "a merge that breaks it is backed out"). The whole-plan check runs on the merge in a
  separate checkout and "the run branch moves only once it passes" (WP 04:69-71); nothing is merged and then reverted.
- **Automatic split and replan** (README:88-89 "Repeated failures escalate the model, then split or replan the
  task"). Failure on the top rung goes to a person (WP 04:48; RP 06:47); `ReplanController` has no production caller.
- **A controller that "retunes itself when they drift"** (README:115-116). The papers' bounded controller starts in
  shadow mode until a person switches it on (WP 05:72-73; RP 09:82-84); `HomeostasisMode::Shadow` is the default.
- **Routing that "comes from what has actually passed"** (README:37-39, :111-112). By the papers' precedence a
  default run routes by the model ladder, and the learned router's pick is logged as a shadow choice (RP 08:55-59).

## Why it matters

Goal `whitepaper`: the README is the public pitch and is already on `origin/main`. Will's rules for it (memory
`readme_pitch_framing.md`, 2026-10-02) are present tense, the whole vision, no gaps or caveats, lead with vendor
neutrality and measured trust, but it must not claim mechanisms the design rules out.

## Where

`README.md` lines 37-39, 49, 65, 88-89, 101-102, 111-112, 115-118 and 374.

## Current state

Will's 2026-10-15 decision 2 turns the cybernetic layer on in safe stages (PlanCompleted, M1 in shadow by default,
the self-model in shadow by default, loop benefit β, playbooks on plan runs, error patterns keyed by plan, audits
at 10% after q-394b7f's floor) and keeps the papers as written. No turn-on covers the merge queue, back-out or
split/replan.

## Plan

Edit the README only; keep the present tense and the pitch's order.

1. **Fix now (no turn-on covers them):** describe delivery as it is: each plan builds on its own branch, the merged
   result is checked as a whole in its own checkout, and the run branch moves only once that check passes (:49, :65,
   :101-102, and "plan delivery" for "merge queue" at :374). For :88-89: repeated failures climb the model ladder,
   and a task that fails on the top rung goes to a person, who decides whether to split or replan it.
2. **Word so it becomes true as the turn-on lands:** routing (:37-39, :111-112): each task starts on the cheapest rung
   of your model ladder and moves up only when it fails, while a router and a forecaster learn from your verified
   results which model and vendor pass which kinds of task. Self-regulation (:115-116): Roko tracks pass rate, cost
   per verified task, false-pass rate and latency against the bounds in its policy file, proposes the setting changes
   that would bring them back, and applies them once you switch the controller on.
3. Leave the audit and loop-audit sentences (:43-47, :103-104, :117-118) as they are: the papers say the same, and
   the turn-on (gap-d76b1b and the β estimate) makes them hold.

## Done when

The `[[verify]]` command passes and the edited lines match the papers' descriptions (WP 04, 05; RP 06-09).

## Notes

Don't edit the papers (decision 2 keeps them as written). Follow memory `readme_pitch_framing.md`: no "not yet",
no status tags, never lead with "a frontier model plans, cheap models execute". The verify's `grep -q 'run branch'`
expects the new delivery wording to name the run branch, as both papers do; adjust it if the final wording differs.

2026-10-16 (closing worker): the code does take a failed plan back out. `GitDeliveryBackend::git_merge` moves the run
branch to the merge before the whole-plan check runs, and `BatchIntegration::deliver` resets it when the check fails
(`graph_execution/batch.rs:288-316`), so the old "backed out" wording described the code, and the papers' "the run
branch moves only once it passes" does not. The new wording ("the run branch keeps a plan only if that check passes")
holds under either order; bug-700d7c tracks making the code match the papers. The README's comparison table
("routed per task by what has passed before"), "How it works" and "Providers" repeated the routing claim, so they
were fixed too. CLAUDE.md's key-crates table still says "merge queue" for roko-cli (outside this item's README scope).
