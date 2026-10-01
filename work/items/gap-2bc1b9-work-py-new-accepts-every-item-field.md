+++
id = "gap-2bc1b9"
kind = "gap"
title = "work.py new accepts every item field as a flag"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (§1, R3)"
anchors = ["tools/work.py::cmd_new", "tools/work.py::main", "tools/test_work.py"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = ["gap-130a3e"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_new_writes_every_field_from_flags' tools/test_work.py && grep -qw 'def test_new_refuses_an_unguarded_verify' tools/test_work.py && python3 tools/test_work.py -k test_new_writes_every_field_from_flags -k test_new_refuses_an_unguarded_verify"
+++

## Problem

`work.py new` takes only `--kind`, `--title`, `--source`, `--created`, `--root`, `--subsystem`, `--severity`,
`--status` and `--triage`. Everything else (goal, lane, parent, size, anchors, verify commands, links and the body) is
hand-edited into TOML afterwards (W1 §1). That is how malformed front matter and unguarded verify commands get in;
the agents drafting these epics wrote every file by hand for the same reason.

## Why it matters

Goal `tooling`, epic spec-1e1b45. Agents file new items while they work (README step 6), and the checklist import
(gap-25065c) creates about a hundred items through `new`.

## Where

`tools/work.py`: `cmd_new` and the `new` parser in `main`. Tests in `tools/test_work.py`.

## Current state

Checked at `41c7ffbd6`: `cmd_new` writes `anchors = []`, empty links and the body template.

## Plan

1. Add `--goal`, `--lane`, `--parent`, `--milestone`, `--size`, `--rank`, `--hold`, `--discovered-from`, `--doc`,
   `--anchor` and `--verify` (both repeatable), `--depends-on`, `--related` and `--blocks` (comma lists), and
   `--body-file`, which replaces the template.
2. Validate before writing: goal against `goals.toml`; lane, parent and milestone as gap-130a3e does; each verify
   through `lint_verify`; each anchor through `valid_anchor`.
3. Refuse a `gap`, `bug` or `regression` with no `--anchor`, or with no `--verify` unless `--no-verify-yet` is given.
4. Write the fields in the order of `work/README.md`'s example, so generated and hand-written items look alike.
5. `--dry-run` prints the file instead of writing it.

## Done when

- [ ] `new` with every flag writes a file that `check` accepts and `load` reads back with the same values.
- [ ] A `--verify` that `lint_verify` flags, such as an unguarded cargo test filter, is refused.
- [ ] The `[[verify]]` command passes.

## Notes

- Keep the id rule (`make_id`) unchanged.
- Waits for gap-130a3e.
- 2026-10-01 (wk-filer4): implemented on work/gap-2bc1b9. `new` now takes every item field as a flag (goal, lane, parent, milestone, size, rank, hold, discovered-from, doc, repeatable --anchor and --verify, the link lists, --body-file), checks them the way `check` does (plus `lint_verify` and `valid_anchor`), refuses a gap, bug or regression without an anchor, or without a verify unless --no-verify-yet, and prints the file with --dry-run. Tests: test_new_writes_every_field_from_flags and test_new_refuses_an_unguarded_verify; `python3 tools/test_work.py` passes (39 tests).
