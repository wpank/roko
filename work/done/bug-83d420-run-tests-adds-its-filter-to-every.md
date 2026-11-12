+++
id = "bug-83d420"
kind = "bug"
title = "run_tests adds its filter to every runner's command as one more word: go reads it as a package, make as a target, pytest as a path, forge does not filter by it, and cargo flags in it reach cargo as one argument"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-std/tools"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "ea2bd8669"
source = "bug-1f8db7 fix; golden-path live runs 2026-10-19 (gap-f30b8e)"
discovered_from = "bug-1f8db7"
anchors = ["crates/roko-std/src/tool/builtin/run_tests.rs::run"]
links = { depends_on = [], blocks = [], related = ["bug-1f8db7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_filter_reaches_each_runner_as_its_own_option' crates/roko-std/src && cargo test -p roko-std --lib a_filter_reaches_each_runner_as_its_own_option"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T03:12:29Z"
commit = "ea2bd8669"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-21T01:31:03Z"
forced = false
evidence = "ea2bd8669. Cause: run() appended the filter to Runner::args as one more word for every runner but node and unittest, so go read it as a package, make as a second target, pytest as a path from the root, forge as no filter, and cargo flags reached cargo as one word. A scratch two-crate workspace reproduces golden-path run 3's error (cargo test --workspace '-p alpha --test accept_word_count': invalid character ` ` in package name) and shows cargo ignores -p beside --workspace (both crates' tests ran). Fix (crates/roko-std/src/tool/builtin/run_tests.rs::runner_args): cargo takes a test name, or the filter's words when every flag is on CARGO_FLAGS (package, target, feature and profile selection) or after -- on HARNESS_FLAGS; any other flag (--config, --manifest-path, --target-dir, -Z, ...) is refused with what filter takes, since a run_tests call has no command for the bash policy to screen; --workspace is dropped when the filter picks packages. go gets -run=<filter>, or a ./package in place of ./...; forge --match-test=<filter>, or --match-path=<filter> for a .sol file; pytest a test file, directory or node id inside the checkout as given, a test file's bare name resolved from the scan (test_textutil.py::test_shout runs py/test_textutil.py::test_shout), else -k <filter>, refusing a filter that begins with - or a path outside the checkout; npm runs npm test -- <filter> and refuses one that begins with -; make gets TEST=<filter> when the Makefile reads $(TEST) and the filter is a plain name, else an error naming the runners that fit. The filter description in tool_def says what each runner does with it. Checked through heavy.sh on ea2bd8669's roko-std: cargo test -p roko-std --lib run_tests, 15 passed, among them the [[verify]] test a_filter_reaches_each_runner_as_its_own_option (15 runner and filter cases recorded by stand-ins on a test PATH, 7 refused filters that run nothing, make without a TEST variable); roko-std builtin_handlers 33 and golden_tools 17 passed; cargo clippy -p roko-std --no-deps -- -D warnings and cargo +nightly fmt -p roko-std -- --check clean. Filed bug-f831b3 (read_counts reads forge's summary lines as unittest's; pytest and go results carry no counts)."
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
