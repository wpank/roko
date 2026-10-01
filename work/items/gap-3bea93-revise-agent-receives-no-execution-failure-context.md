+++
id = "gap-3bea93"
kind = "gap"
title = "revise agent receives no execution-failure context"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "plan:portal-programme/04b-backend-plan-revision#T06"
discovered_from = "plan:portal-programme/04b-backend-plan-revision#T06"
anchors = ["crates/roko-serve/src/routes/plans.rs::revise_plan", "crates/roko-cli/src/plan_authoring.rs::revise_plan_source", "crates/roko-cli/src/plan_authoring.rs::build_revision_prompt"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/pub fn build_revision_prompt/,/^}/p' crates/roko-cli/src/plan_authoring.rs | grep -qiE 'failure|last_error'"
+++

When `POST /api/plans/{id}/revise` is called after a plan run has failed, the planning
agent receives only the current `tasks.toml` and the user's feedback text. It has no
information about which task failed, what error it produced, or what gate output caused
the failure.

A smarter revision experience would optionally include the last run's failure summary —
the failed task id, the gate output or agent error — so the agent can produce a more
targeted fix without the user having to copy-paste the failure message into the feedback
field manually.

This is a `p3` polish gap: the feature works correctly without it, and the user can
include failure context in the feedback field manually.

**A fix must:** pass execution-failure context (e.g. the last checkpoint's error or
`last_error` from `PlanSummary`) into the revision prompt alongside the feedback, when
the plan has a recorded failure and the caller does not supply explicit failure detail.

Re-checked 2026-09-29: unchanged. The prompt is built in crates/roko-cli/src/plan_authoring.rs::build_revision_prompt (:279) from plan id, current tasks.toml and feedback only. The existing verify is unsound: it matches last_error in the list/detail DTO tests of routes/plans.rs and passes while the gap exists.

## Notes

2026-10-01 (wk-runstate): implemented on work/find-8872ad; cargo verification deferred to the batch check. `build_revision_prompt` takes `last_failure: Option<&str>` and, when given, adds a "The plan's last run failed…" section between the feedback and the instructions. `revise_plan_source` fills it with the new `last_run_failure`. That function reads the report `roko diagnose` prints: `commands::diagnose::build_report` is now `pub(crate)`. If the last Graph run failed, it lists up to five failed tasks, each with its reason and last error, capped at 2,000 characters. Otherwise it returns `None`, and the prompt is unchanged. The recorded failure is always included; there is no request field to override it, so a user who also pastes the failure into the feedback sends it twice. Tests: `revision_prompt_carries_the_last_run_failure` and `a_plan_that_never_ran_has_no_last_run_failure` (plan_authoring.rs). The [[verify]] grep now fails at BASE and passes here.
