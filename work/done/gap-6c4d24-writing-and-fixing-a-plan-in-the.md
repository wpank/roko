+++
id = "gap-6c4d24"
kind = "gap"
title = "Writing and fixing a plan in the portal speaks in raw errors: a generate failure prints the planner's whole message twice with absolute paths, and validation shows rule ids, config keys and one missing check as two errors; show writing on glass, a failure as one sentence with the raw error folded, and each problem once, in plain words, on its task"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "33c33e738"
source = "round v8 portal-v8 audit (tmp/hermes-roko-v8/design/portal/before, main bd93521ee), the lead's approved proposal (tmp/hermes-roko-v8/design/portal/README.md, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, bd93521ee)"
anchors = ["apps/portal/src/components/stage/PromptPanel.tsx::PromptPanel", "apps/portal/src/components/stage/ValidationBadge.tsx::ValidationBadge", "apps/portal/src/lib/apiErrors.ts::describeRequestError", "apps/portal/src/lib/alerts.ts::pickAlert"]
links = { depends_on = ["gap-60f6b6"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF 'a generate failure is one sentence' apps/portal/src && (cd apps/portal && npx vitest run src/components/plan/authoring.v8.accept.test.tsx)"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T18:40:00Z"
commit = "33c33e738"
by = "portal-v8 (round v8)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
size = "M"
claimed_at = "2026-11-12T17:20:00Z"
forced = false
evidence = "Writing: the time on glass, the request quoted, what happens next. A failed write or revision is one sentence by what failed (lib/writeFailure.ts) with roko serve's message folded and its paths relative; the notice band repeats neither it nor a plan's validation errors. A plan that can't run: pill needs you, one line per problem in plain words on its task with show (lib/planProblems.ts; PLAN_035 and PLAN_037 for one missing check read as one line), rule ids folded, its pad outlined red ('no check'), keys revise and the orange edit plan. Esc closes the open box or editor before anything else. Not done, for want of a server part: a stop key while roko writes (no operation cancel route), SPENT while it writes (cost comes only with the result), and a 'writing the plan' row in the plan list (the plan doesn't exist until written). At 33c33e738's tree in roko-work-gap-6c4d24: tsc clean, vitest 725/725 (authoring.v8.accept 6 cases: 'a generate failure is one sentence', 'one problem, one line'), orphans none, build:export OK, roko plan validate over every portal plan without errors, the CI browser check PASS 37 of 37. Shots: tmp/hermes-roko-v8/design/portal/item4/ (writing, write-failed, problems)."
+++

## Problem

Writing and fixing a plan in the portal speaks in raw errors and engine words.

- **Generate failing** (`tmp/hermes-roko-v8/design/portal/before/00c-generate-result.png`): the planner's whole
  message twice (the alert band and an inline notice): "plan generation failed for <slug>: Plan generation failed after
  retries: no valid tasks.toml was produced. Last error: no TOML block found in agent output … ```toml block … The
  rejected drafts are in /Users/…/.roko/plan-drafts/<slug>." with an info glyph in red.
- **While roko writes**, a spinner and "Generating plan…" with a time, and nothing about what happens next.
- **Validation** (`13-validation-open.png`): `PLAN_035 · — · schema validation failed: T2: missing 'verify'
  (required for role 'implementer')` and `PLAN_037 · T2 · task 'T2' has no verify steps: … [meta] allow_unverified =
  true`: one problem reported twice, in rule ids and config keys, in a popover over the tasks; "2 validation errors"
  again in an alert band, and "Fix 2 validation errors first" beside the disabled Run.
- **Revise** is the same box with "What should change?"; Esc closes the alert instead of the open box.

## Why it matters

Goal visibility. Asking for a plan and fixing one are the first things a new user does, and the design system's rule
is one sentence for the reason, the raw error folded (§4.13, §6.4).

## Where

- `apps/portal/src/components/stage/PromptPanel.tsx` (generate and revise), `ValidationBadge.tsx`,
  `SourceEditor.tsx`, `RevisionDiff.tsx`.
- `apps/portal/src/lib/apiErrors.ts` (`describeRequestError`), `src/lib/alerts.ts` (`pickAlert`), `src/lib/operation.ts`.

## Plan

1. **Writing a plan:** the plan's row and page say "writing the plan" (live), with `TIME` and `SPENT` on glass, the
   request quoted, and "You'll see the plan here before anything runs."; `stop` ends it.
2. **A failure is one sentence,** chosen by what failed ("roko couldn't write a plan for this request: the model's
   answer had no plan in it. Try again, or say more about what you want."), with the raw message in a fold and
   paths shown relative to the project. It shows once, in the page, never also in a band.
3. **A plan that can't run:** pill "needs you", "This plan can't run yet." and one line per problem, in plain words,
   each with `show` (selects its task's pad, outlined red). Rule ids and keys move into the fold. Two diagnostics of
   one task's one problem (PLAN_035 and PLAN_037 for a missing check) show as one line. Keys: `revise` and the
   orange **`edit plan`**.
4. **Revise** opens the same box as a new plan ("What should change in <plan>?"); the revision's diff stays.
5. **The editor** (tasks.toml) restyled, behind `edit plan`; Esc closes the open box before anything else.

## Done when

- CI's portal job passes; `src/components/plan/authoring.v8.accept.test.tsx` passes ("a generate failure is one
  sentence", "one problem, one line").
- Screenshots, day and night, match the mock (`design/portal/after/new-*`, `writing-*`, `problems-*`).

## Notes

- Depends on gap-60f6b6 (the plan page); keep the P-3 validate contract.
