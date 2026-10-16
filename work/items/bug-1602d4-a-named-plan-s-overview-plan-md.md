+++
id = "bug-1602d4"
kind = "bug"
title = "A named plan's overview plan.md keeps the plan id as its title, and the plan-writing stage line shows no spend until the plan is written"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/plans", "roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/dogfood-will.md Run 2, R2-5 and R2-6 (2026-10-16)"
anchors = ["crates/roko-serve/src/routes/plans/authoring.rs::name_generated_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["gap-11d870", "gap-c2efe7", "bug-e0624e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_named_plan_retitles_its_overview' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_named_plan_retitles_its_overview"

[[verify]]
command = "grep -rqw 'fn plan_writing_reports_its_spend_as_each_call_ends' crates/roko-cli/src && cargo test -p roko-cli --lib -- plan_writing_reports_its_spend_as_each_call_ends"
+++

## Problem

Two findings of the release check (`tmp/hermes-roko-v5/findings/dogfood-will.md`, "Run 2", R2-5 and R2-6), both on a
plan a chat host asked for:

- **R2-5, the plan is titled by its id.** Hermes passed `title` "Word count demo" to `/mcp` `plan_generate`. serve
  wrote it into `tasks.toml` (`[meta] title = "Word count demo"`, `name_generated_plan`) after the plan was written,
  but the overview `plan.md` roko writes when the planner writes none (gap-11d870) already had `title:
  word-count-demo` and `# word-count-demo`: a chat host's request is not quoted, and the planner gave no title. Plans,
  the Run plan dialog and `/roko status` take a plan's title from `plan.md`'s first heading (`plan::plan_title`), so
  they showed `word-count-demo`, while the plan-writing card said "Word count demo".
- **R2-6, the stage line shows $0.00 throughout.** The operation's `progress.cost_usd` (gap-11d870) is read when a
  stage starts. A one-call plan's spend is first reported at `writing` and `recording`, a moment before the
  operation completes, so a host polling every few seconds saw $0.00 for the whole writing and the $0.0734 only once
  the plan was written.

## Why it matters

The person sees the plan's id instead of the name the plan was given, and the spend of plan writing only after the
fact (goal `hermes`).

## Where

- `crates/roko-serve/src/routes/plans/authoring.rs::name_generated_plan`: names a generated plan in `tasks.toml`
  only.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: when the stages are reported.

## Plan

1. When serve names a generated plan, also retitle its `plan.md` where that still carries the slug as its title: the
   frontmatter `title:` and the first `#` heading, which stays the body's first `#` line. A `plan.md` the planner
   titled itself is left alone.
2. Report the stage again as soon as each planner call ends, with the spend so far and the finished call counted.

## Done when

`[[verify]]` passes: a named plan's overview `plan.md` carries the name, and the stage reports a planner call's spend
when the call ends.
