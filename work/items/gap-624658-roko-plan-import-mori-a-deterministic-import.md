+++
id = "gap-624658"
kind = "gap"
title = "roko plan import --mori: a deterministic import of a mori plan dir (tasks, verify-tasks, reviews, scribe tasks, rubric) into a roko plan that validates"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "L"
subsystem = ["roko-cli/plan_import"]
created = 2026-10-16
updated = 2026-10-16
source = "tmp/hermes-roko-v6/findings/plans-research.md (plans-research, 2026-10-16), proposed item 1"
discovered_from = "hermes-roko-v6 round: mori-style plans in roko"
anchors = ["crates/roko-cli/src/commands/plan.rs::PlanCmd", "crates/roko-cli/src/plan_validate.rs::validate_plans_dir_with_workdir"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_import_mori_writes_a_plan_that_validates' crates/roko-cli/ && cargo test -p roko-cli --lib plan_import_mori_writes_a_plan_that_validates"
+++

## Problem

Will's mori plans (`/Users/will/dev/uniswap/bardo/.mori/plans/<plan>/`, 156 with a `tasks.toml`) are specs with
acceptance checks, review checks and a rubric. roko cannot run them as they are. It parses every mori `[[task]]` key,
yet an unmodified mori `tasks.toml` fails validation:
- tasks have no `role`;
- tasks have no verify steps;
- cross-plan `depends_on` entries such as `08e:T9` are unknown.

roko never reads `verify-tasks.toml`, `review-tasks.toml`, `scribe-tasks.toml`, `rubric.md` or `verify.sh`.

plans-research's prototype converter (`tmp/hermes-roko-v6/plans-research/mori2roko.py`) gets 132 of the 156 plans
through `plan validate`, and 19 under `--strict` (`tmp/hermes-roko-v6/findings/plans-research.md` §2).

## Why it matters

Round v6, deliverable 2: roko accepts and runs mori-style plans. A deterministic import costs nothing and needs no
model. Its plan is reviewable, and its acceptance criteria stay runnable. `plan generate --from-file` does not keep
them: it kept 0 of 8, 0 of 8 and 4 of 14 of the source's checks in three live runs.

## Where

- `crates/roko-cli/src/commands/plan.rs::PlanCmd`: the new `import` subcommand.
- A new module, `crates/roko-cli/src/plan_import.rs`, which does the import.
- `crates/roko-cli/src/plan_validate.rs::validate_plans_dir_with_workdir`: the import validates what it writes.
- `crates/roko-cli/src/plan_policy.rs::TierSizeLimits`: the tier limits a task must fit (read, not changed).

## Plan

1. Add `roko plan import --mori <dir>`, taking a plan dir, or a directory of them (the `.mori/plans` root). Port
   the prototype's mapping:
   - roles (`category = docs` gives a scribe, anything else an implementer), and tiers from `complexity_band`;
   - checks, from three sources:
     - verify-tasks entries whose `test_fns` the task's acceptance names, guarded by `grep -rqw 'fn <name>'`;
     - acceptance "Checkpoint: `cmd`" and "`cmd` exits 0" lines;
     - "File exists" lines;
   - the rest of verify-tasks, and the rubric's commands, become `[[meta.verify]]`;
   - `review-tasks.toml` becomes an advisory quick-reviewer task, and `scribe-tasks.toml` a docs task;
   - `rubric.md`, `testing-backlog.md` and the plan doc are copied beside `tasks.toml`;
   - `verify.sh` steps are mapped where they are commands, and noted otherwise;
   - writers of the same `exclusive_files` are serialized in the DAG;
   - `workspace_rungs = false` is set when the tasks carry their own checks;
   - cross-plan dependencies (`08e:T9`) become `depends_on_plan` and are reported, never dropped silently;
   - a task over roko's tier limits is raised to the tier it fits, split by files, or flagged with the reason.
2. Print an import report, and write it with `--json`: per plan, what was mapped, what is carried as notes, what was
   dropped, and the validation result.
3. Tests:
   - always on: a fixture mori plan imports into a plan that validates;
   - a corpus test over the bardo plans (`#[ignore]`, reading `ROKO_MORI_CORPUS`) reports how many validate, both
     plain and under `--strict`.

## Done when

`plan_import_mori_writes_a_plan_that_validates` passes (the `[[verify]]` command).
