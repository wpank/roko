+++
id = "bug-1f8db7"
kind = "bug"
title = "run_tests returned nothing usable in all 30 calls of the 2026-10-19 golden-path live runs: the immune boundary scored a result invalid_structured_result and isolated the tool for the run, so 23 calls were refused or withheld; the rest failed to spawn (npm) or reported real test failures as tool failures"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/safety"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "gap-f30b8e golden-path rerun, live runs 1-3 (2026-10-19)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-std/src/tool/builtin/run_tests.rs", "crates/roko-agent/src/safety/result_filter.rs", "crates/roko-agent/src/tool_immune.rs::detect_tool_result_anomaly"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "bug-ba0c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_passing_run_tests_result_is_not_an_immune_anomaly' crates/roko-agent/src && cargo test -p roko-agent --lib a_passing_run_tests_result_is_not_an_immune_anomaly"
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
