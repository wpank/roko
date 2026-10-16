+++
id = "bug-72e919"
kind = "bug"
title = "plan validate --dynamic counts a check that cannot run (exit 127, command not found) as red on the base"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "089ccb340"
source = "tmp/hermes-roko-v6/findings/plans-research.md (plans-research, 2026-10-16), proposed item 6"
discovered_from = "hermes-roko-v6 round: mori-style plans in roko"
anchors = ["crates/roko-cli/src/spec_red_on_base.rs::classify", "crates/roko-cli/src/spec_red_on_base.rs::Outcome"]
links = { depends_on = [], blocks = [], related = ["bug-6fa025"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn red_on_base_treats_a_missing_command_as_unknown' crates/roko-cli/ && cargo test -p roko-cli --lib red_on_base_treats_a_missing_command_as_unknown"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T21:16:30Z"
commit = "089ccb340"
by = "commit trailer"
executor = "claude-agent"
forced = false
evidence = "Closed by 089ccb340. Verify passed (cargo test -p roko-cli --lib red_on_base_treats_a_missing_command_as_unknown); roko-cli lib tests 3734 passed (gist test skipped); clippy -p roko-cli -D warnings and nightly fmt clean. Live, plan validate --spec-quality --dynamic --fixture on a scratch plan checked by pytest (not installed), ./bin/tool and test -f: before (roko-main 47b3cdd20) all three tasks were red with SQ06 = 1; after, T1 is unknown/cannot_run (`pytest` is not installed, exit 127) with SQ06 = 0 and the other two stay red. speclint dynamic.py has no cannot_run outcome yet."
+++

## Problem

`roko plan validate --dynamic` runs each implementer task's checks on the base commit, to prove them red there
(SQ06 = 1) and to catch checks that already pass (HF3). `spec_red_on_base.rs::classify` counts any non-zero exit as
red. A check whose command is not installed exits 127 (`bash: line 1: pytest: command not found`). It can never pass,
so it says nothing about the work, yet the task scores SQ06 = 1, "red on base".

Seen 2026-10-16 by plans-research. A generated pocket-synth plan was checked by `pytest -q tests/test_osc.py` in a
workspace without pytest. It scored 100 A with 0 diagnostics under `--strict --spec-quality --dynamic`
(`tmp/hermes-roko-v6/findings/plans-research.md` §3.2, run G3a).

## Why it matters

The red-on-base proof is how roko tells a spec whose checks can turn green from one whose checks never will. Today a
check that cannot run gets full credit, so a plan that can never succeed scores as well as a sound one.

## Where

- `crates/roko-cli/src/spec_red_on_base.rs::classify`: every non-zero exit is `Fail`.
- `crates/roko-cli/src/spec_red_on_base.rs::Outcome`: the outcomes the summary lists.

## Plan

1. A step that exits 127 while bash reports `<name>: command not found` for a bare command name (no `/`) cannot run.
   Its task becomes `unknown`, with a new outcome, `cannot_run`, that names the missing command. A missing path
   (`./bin/tool: No such file or directory`, a program the task has not built yet) stays red.
2. Test: a task checked by a command that is not installed is `unknown`/`cannot_run`; a task checked by a program it
   builds is still red.

## Done when

`red_on_base_treats_a_missing_command_as_unknown` passes (the `[[verify]]` command). Related: bug-6fa025, the same exit
at run time.
