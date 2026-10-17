+++
id = "bug-ce2967"
kind = "bug"
title = "A plan has three titles: the plan list reads plan.md's first heading and ignores [meta].title, run events read [meta].title and ignore plan.md, and /api/workflows title-cases the slug"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/plan"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "feca5b4d5"
source = "tmp/hermes-roko-v6/findings/dogfood-will.md (dogfood-will Run 3, U8), via plugin-runs' triage"
anchors = ["crates/roko-cli/src/plan.rs::plan_title", "crates/roko-cli/src/graph_execution/plan_runner.rs::plan_title", "crates/roko-serve/src/routes/workflows.rs::parse_plan_dir"]
links = { depends_on = [], blocks = [], related = ["bug-1602d4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_title_reads_meta_title_then_plan_md_then_slug' crates/roko-cli/ && cargo test -p roko-cli --lib plan_title_reads_meta_title_then_plan_md_then_slug"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T17:30:47Z"
commit = "f23c4689e"
forced = false
evidence = "One rule for a plan title, roko_core::task::plan_title: [meta].title, else the first H1 of plan.md after its front matter (else the front matter title:), else the slug. roko plan list and show, GET /api/plans and /api/plans/{id}/tasks, the Graph runner plan_set_loaded event and GET /api/workflows all use it. Live at zero cost (three plans): before, the list and /api/plans said pocket-synth while /api/workflows said pocket-synth: oscillator, filter, envelope, render, and workflows title-cased the slugs (Bare, From Doc); after, every route says pocket-synth: oscillator, filter, envelope, render, Written for people and bare. On feca5b4d5 (main 08d4fe558 and gap-6605f7 merged in): the verify, the 2 roko-core plan_title and markdown_title tests, the 2 plan_set_entries tests, gap-6605f7 verify, 3 roko-learn task_estimate tests, 68 roko-serve routes::plans tests, clippy -D warnings (roko-core, roko-learn, roko-serve, roko-cli) and nightly fmt pass."
+++

## Problem

A plan's title depends on who asks:

- The plan list (`roko plan list`, `GET /api/plans`) and the plan API's task list use `plan.rs::plan_title`: the first
  heading of `plan.md`, else the plan id. They ignore `[meta].title` in `tasks.toml`.
- Run events (`plan_set_loaded`, and with it a run summary's title) use `plan_runner.rs::plan_title`: `[meta].title`,
  else the `[meta] plan` id. They ignore `plan.md`.
- `GET /api/workflows` (`workflows.rs::parse_plan_dir`) uses `[meta].title`, else the title-cased `[meta] plan`. It
  never reads `plan.md`'s heading while `[meta] plan` is set, and that is almost always.

Seen in dogfood-will's Run 3 (U8, `tmp/hermes-roko-v6/findings/dogfood-will.md`): `tasks.toml` has
`title = "pocket-synth: oscillator, filter, envelope, render"`, and the run card shows it. The Plans tab lists
"pocket-synth / pocket-synth", because `plan.md`'s heading is `# pocket-synth`. bug-1602d4 works around this for the
overview `plan.md` that serve writes, by retitling that file too.

## Why it matters

One plan shows two names: the Plans tab and the run card disagree. A title the planner or the person gives in
`[meta].title` never reaches the list.

## Where

- `crates/roko-cli/src/plan.rs::plan_title`: the list and the plan API.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::plan_title`: run events.
- `crates/roko-serve/src/routes/workflows.rs::parse_plan_dir`: the workflows view.
- `crates/roko-core/src/task.rs`: where one rule can live, since roko-cli and roko-serve both depend on roko-core.

## Plan

1. One rule in roko-core, `plan_title(meta_title, plan_md, slug)`: `[meta].title`, else the first `# ` heading of
   `plan.md` after its front matter (else the front matter's `title:`), else the slug.
2. The plan list, the plan API, run events and the workflows view all use it. bug-1602d4's retitling stays, so
   `plan.md` agrees with the title.
3. Tests: a plan with all three sources, with no `[meta].title`, and with neither.

## Done when

`plan_title_reads_meta_title_then_plan_md_then_slug` passes (the `[[verify]]` command).
