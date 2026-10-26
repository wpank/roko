+++
id = "gap-dbf1bd"
kind = "gap"
title = "plan validate doesn't flag a non-goal that forbids changing a file the task owns (phase 5's 'Do not change the renderer implementation itself.' was right for T4, but phase 5.1 gives the renderer to the demo task; outright bans only, scoped bans and other verbs pass)"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-hermes-pilot#R-p5-1f677933"
discovered_from = "agent:hermes-pilot (round v7, phase 5 plan p5-ship and run 1f677933; plancheck rule 12, the lead's ask)"
anchors = ["crates/roko-cli/src/plan_validate.rs", "crates/roko-cli/src/task_parser.rs"]
links = { depends_on = [], blocks = [], related = ["bug-3b3bb6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_non_goal_that_forbids_the_task_s_own_file_is_flagged' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_non_goal_that_forbids_the_task_s_own_file_is_flagged"
+++

## Problem

`roko plan validate` doesn't flag a task whose `non_goals` forbid changing a file the task itself owns. A person
reviewing the plan without a separate checker would miss it, and the task would then carry a check that its own files
are forbidden from satisfying.

**The case:** Undertow phase 5 (plan `p5-ship`, written by gpt-5.4-mini on roko e90161ec4, run `1f677933` on
74c3594bc). The planner gave T4, the demo-render script, the non-goal **"Do not change the renderer implementation
itself."**
- That was right for T4, since the renderer (`tools/render/main.cpp`) was T3's file.
- But T4's check P5.06 failed twice on the renderer's output level. T4 couldn't change the renderer, and its third try,
  on gpt-5.4, gamed the check (bug-3b3bb6).
- Phase 5.1 moves the renderer into the demo task's own files. A planner that reuses a non-goal like that one, as
  phase 5's did, would forbid the task's own file outright. Nothing in roko would say so.

The round's reviewer caught this class with a separate checker (`tmp/hermes-roko-v7/pilot/tooling/plancheck.py`,
rule 12). This item asks roko's own validator to do the same.

## Rule

This is the scope hermes-pilot's rule 12 settled on, calibrated on round v7's plans.

- **Flag** a non-goal of the form `(do not|don't|never|must not) (change|modify|edit|touch|alter|rewrite) …` that names
  a word from the task's own `files`.
  - The words are a file's base name or stem, or a directory name, for example `render` for
    `tools/render/main.cpp`.
  - Generic words are ignored: `main`, `index`, `src`, `include`, `core`, `plugin`, `tools`, `ui`, `spec`, `build`,
    `docs`, `CMakeLists`, and words of three letters or fewer.
- **Pass scoped bans:** after the verb, any of "beyond", "unrelated", "API", "public", "path", "format", "only",
  "except", "other than" or "signature". Examples:
  - "Do not change the filter's drive path or its public API."
  - "Do not change the sequencer algorithm beyond what is needed to wire it into the engine."
- **Pass other verbs**, which aren't bans on the task's own files: "add", "install", "implement". Example: "Do not
  add JUCE dependencies to the renderer."
- **Calibration:** on Undertow's phase 3, 4, 4.1, 4.2, 4.3 and 5 plans (24 tasks, 53 non-goals), the rule flags
  nothing. On a task owning `tools/render/main.cpp`, it flags "Do not change the renderer implementation itself." and
  passes "Do not change the renderer's public API.".

## Where

- `crates/roko-cli/src/plan_validate.rs`: the plan diagnostics (rule ids such as `PLAN_TIER_SIZE`). This needs a new
  rule, for example `PLAN_NON_GOAL_OWN_FILE`.
- `crates/roko-cli/src/task_parser.rs`: `non_goals` on the task spec.
- Optionally, the planner's prompt rules in `crates/roko-cli/src/plan_generate/`: never write a non-goal that forbids
  the task's own files.

## Plan

- Add the rule as a warning that names the task, the non-goal and the matched word.
- A new warning fails `--strict` on every plan it fires on, so check its noise on the repo's `plans/` and on the
  round's Undertow plans before shipping. The scope above is what kept the noise at zero there.

## Done when

The test in `[[verify]]` passes:
- A task owning `tools/render/main.cpp` with the non-goal "Do not change the renderer implementation itself." gets the
  diagnostic.
- The same task with "Do not change the renderer's public API." or "Do not add JUCE dependencies to the renderer." gets
  none.
