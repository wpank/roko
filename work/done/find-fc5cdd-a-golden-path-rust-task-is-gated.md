+++
id = "find-fc5cdd"
kind = "finding"
title = "A golden-path Rust task is gated only by its accept test at verify depth V0, though its spec asks for a cargo fmt clean, clippy-free crate with a working doc example: T4 left an unformatted crate or a doc example that cannot compile in 3 of the last 6 live runs, and only T6's later edit kept the plan green"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["self-hosting"]
created = 2026-10-19
updated = 2026-10-28
last_verified = 2026-10-28
last_verified_rev = "b03ef0808"
source = "gap-f30b8e golden-path rerun reviews, live runs 1-2 (2026-10-19)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-cli/tests/fixtures/golden_path/plan/tasks.toml", "crates/roko-cli/src/graph_task_dispatch/verify_depth.rs"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "find-5bcba7", "bug-1f8db7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rust_task_checks_refuse_an_unformatted_change' crates/roko-cli/src && cargo test -p roko-cli --lib rust_task_checks_refuse_an_unformatted_change"

[closed]
at = 2026-10-28
at_ts = "2026-10-28T10:38:37Z"
commit = "e51053465"
by = "work-next 2026-10-28 1059"
executor = "claude-session"
via = "work-next"
size = "S"
claimed_at = "2026-10-28T09:59:58Z"
forced = false
evidence = "Option 1 of Will's 2026-10-28 decision. e51053465 adds the task's Rust checks (crates/roko-cli/src/graph_task_dispatch/rust_checks.rs): for the crates an attempt changed, cargo fmt --check (counting only the files the attempt changed), clippy with warnings denied, and the doc tests of the changed crates that have a library, at every verify depth once the attempt's own checks passed, on the compile lock, failing fast; rows rust:fmt, rust:clippy, rust:doc in the verdict. Not in a FAST run, and not with [gates] clippy_enabled = false (which Graph runs read again, so it is no longer reported inert); V1 does not rerun clippy once they passed it. rust_task_checks_refuse_an_unformatted_change runs the real dispatcher in a temp Cargo workspace: an unformatted change is refused by rust:fmt (rows rust:fmt false, rust:clippy null, rust:doc null), the same change formatted passes (true, true, true), and clippy_enabled = false runs none. Also passing on the branch: roko-cli lib graph_task_dispatch (518), clippy -p roko-cli -D warnings, every CI canary target (honest_verdicts, secrets_and_git_guard, plan_branch_integration, attempt_diff, scheduler, supervision, tier_ladder) and golden_path_acceptance."
+++

## Finding

A Rust task of the golden-path fixture is gated only by its pinned accept test, at verify depth V0, though its
spec also asks for a `cargo fmt` clean crate, free of clippy warnings, with a `///` comment; nothing at the task
level runs fmt, clippy or the crate's doc tests. So an attempt can leave its crate unformatted or with a doc
example that does not compile, and still be accepted onto the plan branch. T4 (`beta::farewell`) shows it
most:

- 2026-10-19 run 1 (`graph-golden-path-31d6a6fb-…`, `4535b3914`): T4's accepted commit `215eb99` was not
  `cargo fmt` clean (`crates/beta/src/lib.rs:32`, a blank line before `mod tests`' closing brace) and its
  doc example calls `farewell` unqualified, so the doctest cannot compile (E0425). Its verdict has one
  `verify:0/test` step at depth V0. T6 (`9cd9467`) fixed both lines later, unasked.
- 2026-10-19 run 2: the same broken example (`4c2dab7`); T4's own `run_tests` reported "1 tests failed"
  before it handed off (`tool_audit.jsonl:61`), and T6 fixed it (`6a80855`).
- 2026-10-18 run 3: the same (`27a3265`), fixed by T6 (find-5bcba7's note).

In each case the delivered plan was green only because a later task that edits the same file repaired it.
Had it not, the whole-plan check would have failed; since find-5bcba7 (`11bbb817f`) that failure goes back to
the task that last changed the file, which may be the later task, not T4. Meanwhile every tree built on T4's
work carries the defect, which is how the audit charged it to T5 in runs 1 and 2 (bug-2504b3).

## Why it matters

The golden path's claim is that cheap rungs are held to the spec by checks. Here the spec's own requirements
(fmt, clippy, a working doc example) are checked only at the plan's end, so the defect travels between tasks,
costs a later task's turns, and can be blamed on the wrong task. In 3 of the last 6 live runs T4 left it.

## Options (decided 2026-10-28: option 1)

1. **Rust task checks for the crates an attempt changed:** `cargo fmt --check -p <crate>`, clippy with
   warnings denied, and `cargo test -p <crate> --doc`, as a task-level step whatever the plan authors. (V1's
   clippy already does part of this; the depth ladder starts at V0 in a fresh vault.)
2. **Operator config:** `[[gates.rungs]]` command rungs with the same steps (find-5bcba7's option 2). Changes
   no plan text; also runs for the Python and TypeScript tasks unless scoped.
3. **Fixture:** give T3–T7 an authored fmt and doc-test verify step. Changes the frozen plan again.
4. **Leave it to the whole-plan check and its hand-back.** Cheapest; the defect still travels until the plan
   is checked.

## Decision (2026-10-28, Will)

Option 1: **task-level Rust checks for the crates an attempt changed.** For every Rust crate an attempt's diff
touches, the task's checks run `cargo fmt --check -p <crate>`, `cargo clippy -p <crate> -- -D warnings` and
`cargo test -p <crate> --doc`, whatever the plan authors, so a T4 that leaves its crate unformatted, with a clippy
warning or a doc example that does not compile is refused at its own task instead of being repaired by a later one.
Options 2 to 4 are not taken. Mind FAST (one authored verify per task, no Cargo in the agent's turn) and the
build-time cost: scope the checks to the changed crates only, and say in the verdict which step failed.

## Evidence

- `work/history/golden-path-live-2026-10-19.md`; in the main checkout (gitignored)
  `tmp/status-quo/2026-10-16-live/evidence/gp3/golden-path/run{1,2}/review.md` and the run records.
