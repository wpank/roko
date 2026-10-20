+++
id = "bug-83d420"
kind = "bug"
title = "run_tests adds its filter to every runner's command as one more word: go reads it as a package, make as a target, pytest as a path, forge does not filter by it, and cargo flags in it reach cargo as one argument"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-std/tools"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "bug-1f8db7 fix; golden-path live runs 2026-10-19 (gap-f30b8e)"
discovered_from = "bug-1f8db7"
anchors = ["crates/roko-std/src/tool/builtin/run_tests.rs::run"]
links = { depends_on = [], blocks = [], related = ["bug-1f8db7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_filter_reaches_each_runner_as_its_own_option' crates/roko-std/src && cargo test -p roko-std --lib a_filter_reaches_each_runner_as_its_own_option"
+++

## Problem

`run_tests` adds its `filter` argument to the runner's command as one more positional word, whatever the runner:

- `go`: `go test ./... <filter>` reads the filter as a package path, so `filter = "TestShout"` fails with a package
  error instead of running `-run TestShout`;
- `make`: `make test <filter>` reads it as a second target to build;
- `forge`: forge filters tests with `--match-test`; a bare word after `forge test` is not a test-name filter;
- `pytest`: `python3 -m pytest <filter>` reads it as a path, so a test name or a file name outside the root
  (`test_accept_word_list.py`, which lives in `py/`) is "file or directory not found";
- `cargo`: agents put cargo flags in the filter, and they reach cargo as one word. In the 2026-10-19 golden-path run 3,
  T3 sent `filter = "-p alpha --test accept_word_count"` and cargo answered `invalid character ' ' in package name`.

bug-1f8db7 gave `node` (a test file path fragment, else `--test-name-pattern`) and `unittest` (`-k`) filters of their
own; the other runners still take the filter as it comes.

## Why it matters

A filter is how an agent runs the one test it is fixing. When the filter breaks the command, the agent gets a runner
error instead of its test's result, spends a turn on it, and tends to fall back to `bash`.

## Where

- `crates/roko-std/src/tool/builtin/run_tests.rs::run`: the arguments of every runner but `node` and `unittest`
  (`Runner::args` then the filter).
- The `filter` description in `tool_def`.

## Current state

Since bug-1f8db7 a failing run is a structured result, so the runner's complaint at least reaches the agent in
`output`. Nothing maps the filter per runner beyond `node` and `unittest`.

## Plan

1. Map the filter per runner: `go test ./... -run <filter>`, `forge test --match-test <filter>`, pytest: a path or
   node id (`py/test_x.py::test_y`) as given, else `-k <filter>`; `make`: refuse a filter plainly (or pass it as
   `TEST=<filter>` if the project documents one).
2. For cargo, either split a filter that starts with `-` into words, or refuse it with a message that says `filter`
   is a test name and how to run one package's tests.
3. Say in the `filter` description what each runner does with it.

## Done when

- [ ] The `[[verify]]` test passes: each runner's stand-in (a script on a test `PATH`, as bug-1f8db7's tests use)
  records the filter where that runner expects it.

## Notes

- Found while fixing bug-1f8db7 (evidence: `tmp/status-quo/2026-10-16-live/evidence/gp3/golden-path/run3/workspace/tool_audit.jsonl`
  in the main checkout, gitignored).
