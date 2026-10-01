+++
id = "find-70edcb"
kind = "finding"
title = "Plan generation/validation does not flag weak verify gates"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/plan_policy"]
created = 2026-09-26
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-1. Plan 01 shipped a regression its own gate could not see"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-1. Plan 01 shipped a regression its own gate could not see"
anchors = ["crates/roko-cli/src/plan_policy.rs::validate_plan_budgets", "crates/roko-cli/src/plan_validate.rs", "crates/roko-cli/src/plan_generate.rs:353"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
A verify scoped to the file the task wrote let a crate regression through; negative greps punished correct code; vacuous verifies passed on an untouched repo. Validator/generator should warn on these patterns and prefer whole-crate gates.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-1. Plan 01 shipped a regression its own gate could not see`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-5. Two verify gates that the CORRECT implementation would fail`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Changes applied in this session`
- `tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#Fix 12: Sandbox permission bypass for Restrict level`

How to verify: Check validator lint rules for verify-gate quality.

Verified 2026-09-28 (static check against 3d0ee4d02): No weak-verify lint exists: plan_validate.rs rules cover graph shape, roles/templates, models, schema (PLAN_035) and write capability (PLAN_036), and plan_policy.rs:44-114 only enforces verify-step counts and duplicates (max_verify_steps_per_task, require_one_verify, reject_duplicate_verify). Nothing flags file-scoped verifies, negative greps or verifies that pass on an untouched tree. The generator prompt (plan_generate.rs:352-366) still asks implementers for 'exactly one focused verify step' with no whole-crate gate guidance or warning about these patterns.

Rechecked 2026-09-29 at d9e79e9d8: still open. plan_validate.rs has no rule that flags file-scoped verifies, negative greps or verifies that pass on an untouched tree. plan_policy.rs::validate_plan_budgets (:189) still checks only verify-step counts and duplicates. The generator prompt's verify section has moved to plan_generate.rs:353-366.

## Notes

- 2026-10-01 (wk-specq): partial, on work/gap-404fdb; cargo verification deferred to the batch check.
  - Done: `roko plan validate` warns (PLAN_042) on a verify step that negates a grep (`! grep`, `! rg`), the R-5
    pattern. Test: `a_negative_grep_verify_step_is_a_plan_042_warning`.
  - Done: the generator prompt (`plan_generate.rs`) asks for verify steps that fail on the unchanged code,
    prefers the crate's own gate to a check of the edited file alone, and rules out negated greps and commands
    that cannot fail.
  - Left: default validation still does not flag vacuous or file-scoped verify steps. `plan validate
    --spec-quality` scores them (HF2, SQ04, SQ05). Making that a default warning needs a decision, since
    `--strict` rejects warnings and many plans and test fixtures use `echo ok` or `test -f` steps.
