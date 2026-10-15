+++
id = "bug-e0624e"
kind = "bug"
title = "GET /api/plans/{id}/tasks gives the plan id as the plan's title, so Hermes' Run dialog and run label show demo-hello-world instead of plan.md's title"
status = "open"
triage = "unverified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-15
updated = 2026-10-15
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-cli/src/serve_runtime.rs::load_plan_tasks", "crates/roko-cli/src/plan.rs::extract_markdown_title", "crates/roko-serve/src/plan_types.rs::PlanTasksDto"]
links = { depends_on = [], blocks = [], related = ["bug-0c7f25", "gap-8f84d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_tasks_title_is_the_listing_title' crates/ && cargo test -p roko-cli plan_tasks_title_is_the_listing_title"
+++

## Problem

`GET /api/plans/{id}/tasks` returns `title` = `[meta].plan` from `tasks.toml`, which is the plan id. `GET /api/plans`
and `GET /api/plans/{id}` return the first heading of `plan.md`. Seen 2026-10-15 on roko-main a8e27e4b7, in a clone of
the roko repo:

- `curl :6761/api/plans/demo-hello-world` gives `"title": "Start here: hello world demo"`;
- `curl :6761/api/plans/demo-hello-world/tasks` gives `"title": "demo-hello-world"`.

The roko plugin for Hermes builds its plan detail from the `/tasks` route (`hermes_plugin_roko/tools.py::plan_detail`).
So the Plans tab's dialog reads `Run plan "demo-hello-world"` and the run is labelled `demo-hello-world`, while the list
above it says "Start here: hello world demo". `PlanTasksDto.title` is documented as a "Human-readable plan title";
`[meta].plan` is not one.

## Why it matters

The title is how a person finds the demo to start with (bug-0c7f25 renamed it for that reason). Hosts now see two names
for one plan, and they label runs with the id.

## Where

- `crates/roko-cli/src/serve_runtime.rs::load_plan_tasks`: builds `title` from `tasks_file.meta.plan`.
- `crates/roko-cli/src/plan.rs::extract_markdown_title`: the title the listing uses.
- `crates/roko-serve/src/plan_types.rs::PlanTasksDto`: the field's doc comment.

## Plan

1. Give `/tasks` the same title the listing gives (`plan.md`'s first heading), falling back to `[meta].plan`.
2. Test: for a plan whose `plan.md` has a heading, `load_plan_tasks` returns that heading as the title.

## Done when

`plan_tasks_title_is_the_listing_title` passes (the `[[verify]]` command).
