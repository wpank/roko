+++
id = "gap-0b5ebf"
kind = "gap"
title = "A generated task whose non-goal forbids changing the file its check's failure comes from says nothing of the Blocked: route, so a try may still game the check rather than hand it back (phase 5's T4 and the renderer; bug-3b3bb6's plan item 5)"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 roko-exact (split from bug-3b3bb6, plan item 5)"
discovered_from = "agent:hermes-pilot (round v7 phase 5, run 1f677933)"
anchors = ["crates/roko-cli/src/plan_generate/"]
links = { depends_on = [], blocks = [], related = ["bug-3b3bb6", "gap-4dd77d"], supersedes = [], duplicate_of = "" }
+++

## Problem

A generated plan can give a task a check it cannot pass inside its own files, along with a non-goal that forbids
the real fix. Nothing in the plan says what the task does then.

Phase 5's plan (Undertow run 1f677933, `p5-ship`) did this with T4, "Script the demo renders and commit the showcase
audio":
- Its files were `tools/render-demos.sh` and the four showcase WAVs.
- It had the non-goal "Do not change the renderer implementation itself".
- Its check, P5.06, measures the renders' level.

The level comes from T3's renderer. Both T4 tries said so, and the stronger model's try then gamed the check (see
bug-3b3bb6).

Since bug-3b3bb6, a try can name such a cause with a `Blocked: <path> — <why>` line, and roko hands the failure to
the task that owns the path. The plan still says nothing about that route. The non-goal reads as a flat ban, so a
model may not see `Blocked:` as the allowed way out.

## Why it matters

bug-3b3bb6's plan item (5). A plan that names the route makes it the expected move rather than an escape.

## Where

- `crates/roko-cli/src/plan_generate/`: the planner's prompt, and the repairs after a draft.
- A task's `non_goals` and `verify` steps.

## Plan

Choose one, or both:
1. **The planner's prompt.** When a non-goal forbids changing a file that another task changes and whose output
   this task's check measures, the planner words the route into the non-goal: "… if a check fails because of
   `<file>`, report `Blocked: <file> — <why>` rather than work around it".
2. **A repair after a draft.** Such a non-goal gets the sentence appended.

## Done when

On phase 5's request, a generated T4-like task whose non-goal forbids the renderer names the `Blocked:` route in it.
A test pins the wording.

## Notes

- Split from bug-3b3bb6 (plan item 5) when its roko side and template rule landed.
- Related: bug-3b3bb6, gap-4dd77d (draft repairs).
