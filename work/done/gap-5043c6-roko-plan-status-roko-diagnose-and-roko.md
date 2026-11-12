+++
id = "gap-5043c6"
kind = "gap"
title = "roko plan status, roko diagnose and roko plan validate don't read like the rest of roko's output: absolute paths, engine words, no single next step, and a bug in each (bug-5013e8, bug-649557, bug-81993d)"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands", "roko-cli/plan_validate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "337914fb3"
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan_dir_status", "crates/roko-cli/src/commands/diagnose.rs::render_text", "crates/roko-cli/src/plan_validate.rs::render_text"]
links = { depends_on = ["gap-cf3940", "bug-5013e8", "bug-649557", "bug-81993d"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn validate_reads_as_one_list_per_task' crates/roko-cli/src && grep -rqw 'fn diagnose_shows_each_try_and_the_check_output_once' crates/roko-cli/src && cargo test -p roko-cli --lib -- validate_reads_as_one_list_per_task diagnose_shows_each_try_and_the_check_output_once"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T17:13:45Z"
commit = "337914fb3"
by = "roko-recovery"
executor = "claude-agent"
size = "M"
claimed_at = "2026-11-12T14:39:18Z"
forced = false
evidence = "337914fb3 on work/gap-5043c6: roko plan validate, roko plan status and roko diagnose in roko's design system, like the run view and roko status: a header, the same glyphs and words, and one Next command to paste. validate lists each problem once under its task, then Ready to run or what stops it, the workspace checks and Next; plan status and diagnose show the run view's own table, read from the run; diagnose adds each failed task's check and command, a line per try and the check's output once, and ends with the command that runs the plan again. --json unchanged apart from bug-81993d's and bug-649557's fixes. CLAUDE.md and docs/v3/28-CLI.md updated from the captures. Branch checks: nightly fmt; roko-cli lib 4129 passed (8 ignored; validate_reads_as_one_list_per_task and diagnose_shows_each_try_and_the_check_output_once among them), bin 484 passed; tests/plan_validation 40, plan_validate 27 and cli_fallback 27 passed; workspace clippy -D warnings clean; the binary builds. $0 captures of round v8's story workspace (roko-recovery/runs/item5b: validate, plan status, diagnose, status and a refused run, at 80x24, 100x32 and 120x40, with NO_COLOR and in a pipe): no line reaches the last column, and no escape codes under NO_COLOR or in a pipe."
+++

## Problem

`roko plan status <dir>`, `roko diagnose <plan>` and `roko plan validate <dir>` don't read like the rest of round
v8's redesigned output:
- absolute paths;
- engine words ("verify[0:imports]", "rung[compile]", "diagnostics");
- no single next step.

Each also has a bug, filed on its own: bug-5013e8, bug-649557 and bug-81993d. Captures:
`tmp/hermes-roko-v8/design/terminal/before/{plan-status,plan-status-broken,diagnose,validate-*}.*`.

## Why it matters

Goal tooling. These are the commands a person uses after a run. The proposal and mocks:
`tmp/hermes-roko-v8/design/terminal/README.md`, `after/`.

## Where

- `crates/roko-cli/src/commands/plan.rs::cmd_plan_dir_status`.
- `crates/roko-cli/src/commands/diagnose.rs`: `render_text`, `render_task`.
- `crates/roko-cli/src/plan_validate.rs::render_text`, and its caller `commands/plan.rs::cmd_plan_validate`.

## Plan

1. **plan status:**
   - the header with state, age, time and spend;
   - task rows from the run (state, tries, time, spend), and the plan check;
   - where the work went;
   - Next.
2. **diagnose:**
   - the header;
   - per failed task: the failing check and its command, then one line per try (time, spend, model, the error's
     last line);
   - the check's output once, from the last try;
   - numbered next steps with commands the person can paste.
3. **validate:**
   - the header;
   - problems grouped by task, with one line each, the rule id dim;
   - a result line (Ready to run, or N problems);
   - the workspace check;
   - Next.
4. `--json` is unchanged everywhere.
5. Update CLAUDE.md's CLI table and `docs/v3/28-CLI.md`.

## Done when

- The three bugs are closed.
- The exact-text tests pass, and `tests/plan_validate.rs`'s text assertions are updated.
- The `[[verify]]` passes.
