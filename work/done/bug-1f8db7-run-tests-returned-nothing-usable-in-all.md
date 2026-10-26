+++
id = "bug-1f8db7"
kind = "bug"
title = "run_tests returned nothing usable in all 30 calls of the 2026-10-19 golden-path live runs: the immune boundary scored a result invalid_structured_result and isolated the tool for the run, so 23 calls were refused or withheld; the rest failed to spawn (npm) or reported real test failures as tool failures"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/safety"]
created = 2026-10-19
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "1709a4e1a"
source = "gap-f30b8e golden-path rerun, live runs 1-3 (2026-10-19)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-std/src/tool/builtin/run_tests.rs", "crates/roko-agent/src/safety/result_filter.rs", "crates/roko-agent/src/tool_immune.rs::detect_tool_result_anomaly"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "bug-ba0c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_passing_run_tests_result_is_not_an_immune_anomaly' crates/roko-agent/src && cargo test -p roko-agent --lib a_passing_run_tests_result_is_not_an_immune_anomaly"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T15:51:01Z"
commit = "1709a4e1a"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-20T14:02:53Z"
forced = false
evidence = "1709a4e1a. Cause, from the live runs' tool_audit.jsonl (main checkout, tmp/status-quo/2026-10-16-live/evidence/gp3/golden-path/run{1,2,3}/workspace): of the 30 calls, the 6 passing ones (all cargo) were withheld as invalid_structured_result because the production filter's `[external:run_tests] ` note made their JSON unparseable, and the first isolated run_tests (builtin:local) for the run, so 16 later calls were refused; 4 were npm spawn failures (npm not on the harness PATH, no root package.json, the TypeScript tests run under node --test); 2 ran pytest without pytest; 1 failing cargo run came back as a tool error whose 400-character head showed only passing crates; 1 filter carried cargo flags (so 6 withheld and 4 failures, not the 7 and 3 the item counted). Fix: the production filter turns into text only JSON it broke (bug-0f30c7's demotion, landed after the runs, also hid a handler's own malformed structured payload from the screen), and truncate_result turns a cut structured payload into text, so invalid_structured_result now fires only for the handler's own malformed payload, which is still withheld and still isolates the tool. run_tests returns a failing run, and a failing task check, as a structured result with exit_code, counts and the output's last 60 lines; adds node (node --test, type stripping for TypeScript tests) and unittest (discover per test directory) runners; picks the runner the checkout's root fits when build is omitted; and names a missing runner (not on PATH, npm with no test script, pytest not installed) together with the runners the checkout fits. Checked through heavy.sh at 1709a4e1a: the [[verify]] command passes; roko-agent dispatcher::, truncate and tool_immune tests 143 passed (the whole lib 2384 passed before a test-helper fix); roko-std lib 243 passed and builtin_handlers, golden_tools and property_tests 33, 17 and 14; roko-cli a_cmake_task_can_run_its_checks_mid_try passed with the new checks result; clippy -p roko-std -p roko-agent --no-deps -D warnings and nightly rustfmt clean. Mutation checks: with the filter's demotion removed (the live runs' code) the verify test fails with 'tool result denied by immune boundary'; without the truncation change, a_cut_structured_result_reaches_the_model_as_text and a_cut_json_payload_is_text fail. Filed bug-83d420 (run_tests adds its filter to every runner as one more word) and gap-a2787f (nothing lists or releases a tool isolation control)."
+++

## Problem

In the three golden-path live runs of 2026-10-19 (gap-f30b8e, `4535b3914`), the agents' `run_tests` tool returned
no usable result in any of its 30 calls (11, 9 and 10 per run; every result `ok = false`):

- 16 were refused at once: "permission denied: tool source is isolated by the immune boundary";
- 7 had their result withheld: "permission denied: tool result denied by immune boundary (record …)";
- 4 failed to start: "spawn failed: No such file or directory" (`build = "npm"` in a project with no npm and
  no `package.json`, the TypeScript tests being `node --test`; and the like);
- 3 were real test failures, reported as tool failures.

The immune boundary judged a `run_tests` result an anomaly (`invalid_structured_result`, score 0.9), quarantined
it, and set the tool's control to `isolated` for the rest of the run (`.roko/immune/tool-controls.json`:
`tool_name = "run_tests"`, `source_scope = "builtin:local"`, `control.state = "isolated"`; quarantine evidence
in `.roko/immune/quarantine/evidence.json`, effects `delivery_denied`, `tool_isolation_activated`). From then on
every call is refused.

## Why it matters

`run_tests` is how an agent is meant to check its work before it hands off. Refused, the cheap models fell back to
`bash` cargo commands, and in run 1 that is how T7 built into `crates/alpha/target/`, which was then committed
and delivered (bug-ba0c40). It also hides real failures: a test run that failed reads as a tool failure.

A likely cause, not yet proven: a passing `run_tests` returns `ToolResult::structured(<JSON>)`
(`crates/roko-std/src/tool/builtin/run_tests.rs`), but `run_tests` is one of `EXTERNAL_OUTPUT_TOOLS`
(`crates/roko-agent/src/safety/result_filter.rs:22`), whose output the result filter prefixes with
`[external:run_tests] ` (and truncates with an appended note when long). If the immune check
(`crates/roko-agent/src/tool_immune.rs:210`: `is_structured` and the text does not parse as JSON) sees the
filtered text, every passing result is "invalid", and the first one isolates the tool. Failing runs return
`ToolResult::Err`, which that check skips, which fits the three real failures getting through.

## Where

- `crates/roko-std/src/tool/builtin/run_tests.rs` (the result, and a failed run returned as an error).
- `crates/roko-agent/src/safety/result_filter.rs` (the `[external:…]` prefix, truncation).
- `crates/roko-agent/src/tool_immune.rs::detect_tool_result_anomaly` (`invalid_structured_result`) and the
  tool control that isolates a source (records in `.roko/immune/`).

## Plan

1. Confirm which `run_tests` result the boundary scored `invalid_structured_result` and why, and make the two
   agree: check structure before the filter annotates, or do not annotate a structured result.
2. A failed test run is a result (`ok = true`, with its failures), not a tool failure.
3. Pick the runner from the project (`node --test` for a `.test.ts` file with no `package.json`), or say
   plainly which runner is missing.
4. A test: a `run_tests` call on a failing cargo test returns the failure, and the next call is not refused.

## Done when

- [ ] The `[[verify]]` test passes.

## Notes

- Evidence (gitignored, main checkout): `tmp/status-quo/2026-10-16-live/evidence/gp3/golden-path/run*/run-record/`
  and the reviews; the run workspaces' `.roko/tool_audit.jsonl` and `.roko/immune/` are in the worker's scratch.
