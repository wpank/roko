+++
id = "find-70edcb"
kind = "finding"
title = "DF-0926 R-1/R-5: Plan generation/validation does not flag weak verify gates"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/plan_policy"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-1. Plan 01 shipped a regression its own gate could not see"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-1. Plan 01 shipped a regression its own gate could not see"
anchors = ["plan_policy.rs", "plan_generate.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
A verify scoped to the file the task wrote let a crate regression through; negative greps punished correct code; vacuous verifies passed on an untouched repo. Validator/generator should warn on these patterns and prefer whole-crate gates.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-1. Plan 01 shipped a regression its own gate could not see`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-5. Two verify gates that the CORRECT implementation would fail`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Changes applied in this session`
- `tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#Fix 12: Sandbox permission bypass for Restrict level`

How to verify: Check validator lint rules for verify-gate quality.
