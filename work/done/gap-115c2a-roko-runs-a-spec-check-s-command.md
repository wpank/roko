+++
id = "gap-115c2a"
kind = "gap"
title = "roko runs a spec check's command exactly as written (bug-56a806), but neither the planner prompts nor any guidance say the command must be self-contained: a check that needs a build must include it, since a build the planner puts before it is dropped"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "d41788629"
source = "review:roko-planner-2 of bug-56a806 (e87ec04e3)"
discovered_from = "agent:roko-planner-2 (round v7 review)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::planner_block", "crates/roko-cli/src/plan_generate/source_checks.rs::regeneration_prompt"]
links = { depends_on = [], blocks = [], related = ["bug-56a806"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'must be self-contained' crates/roko-cli/src/plan_generate/source_checks.rs && grep -qw 'fn the_planner_is_told_a_check_command_runs_as_written' crates/roko-cli/src/plan_generate/source_checks.rs && cargo test -p roko-cli --lib -- the_planner_is_told_a_check_command_runs_as_written"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T19:05:24Z"
commit = "d41788629"
by = "roko-exact (round v7)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
forced = false
evidence = "d41788629: source_checks::planner_block (when a check has its own command) and regeneration_prompt now say that a source check's command must be self-contained and runs exactly as written, so a check that needs a build includes the build in its own command (a build put before it in a step is dropped): one const, SELF_CONTAINED, used by both. No guidance for people on writing a spec's checks exists in roko (docs/v3/28-CLI.md plan generate, roko explain plans, the plan-authoring help), so nothing else changed. Option (b), keeping build steps before a spec command, not done (the lead's call). Verify: the_planner_is_told_a_check_command_runs_as_written passes (the planner block and the regeneration prompt carry the sentence for a check with a command; prose checks do not bring it). cargo test -p roko-cli --lib plan_generate: 161 passed, 0 failed; clippy -p roko-cli -D warnings clean; nightly fmt clean. Also filed gap-d99366 (p3), not built."
+++

## Problem

Since bug-56a806 (e87ec04e3), a verify step that runs a source check's command runs that command alone, exactly
as the source writes it: anything the planner puts around it is dropped. roko-planner-2's review: a check whose
command assumes a built tree (the spec's run is `ctest --preset core -R lfo`, the planner wrote `cmake --build
--preset core && ctest --preset core -R lfo`) loses the build, and fails on correct work. Neither the planner block,
the regeneration prompt, nor any guidance for spec authors says that a check's command must be self-contained.

## Why it matters

The person's acceptance checks are run as written; a check that cannot run alone fails the task whatever the work
does, as the planner's guards did in bug-56a806. Undertow's specs are safe (`tools/spec.sh` builds itself), but
another spec may not be. Related: bug-56a806.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs::planner_block`: the planner's list of source checks and how
  to keep them.
- `crates/roko-cli/src/plan_generate/source_checks.rs::regeneration_prompt`: the prompt that asks again for checks
  not kept.
- Guidance for people: none found. `docs/v3/28-CLI.md` (`roko plan generate`), `roko explain plans` and the
  plan-authoring help say nothing about how to write a spec's checks.

## Current state

The planner block says a `run:` command is its step's whole command; nothing says what the command itself must be.

## Plan

1. Both prompts say: a source check's command must be self-contained and runs exactly as written, so a check that
   needs a build includes the build in its own command (a build put before it in a step is dropped).
2. The same sentence wherever roko tells people how to write a spec's checks: there is no such place today.
3. Not option (b), keeping build steps before a spec command: the lead's call (2026-10-20), unless a real spec needs
   it.

## Done when

- The planner block and the regeneration prompt say it when a check has its own command, and the `[[verify]]` test
  holds both to it.

## Notes

- Decided by the lead (round v7, 2026-10-20) after roko-planner-2's review of bug-56a806.
