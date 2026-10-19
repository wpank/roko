+++
id = "bug-0f30c7"
kind = "bug"
title = "A successful run_tests call never reaches the model: the production filter prefixes [external:run_tests] to its structured JSON, and the immune screen withholds the malformed JSON as an anomaly (0.9)"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-agent/safety"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-roko-speed#gap-770cb5"
discovered_from = "agent:roko-speed (round v7, testing gap-770cb5, 2026-10-19)"
anchors = ["crates/roko-agent/src/dispatcher/mod.rs::apply_production_result_filter", "crates/roko-agent/src/safety/result_filter.rs::sanitize"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-770cb5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_external_tool_s_structured_success_reaches_the_model' crates/roko-agent/src && cargo test -p roko-agent --lib an_external_tool_s_structured_success_reaches_the_model"
+++

## Problem

A successful `run_tests` call never reaches the model under the production safety chain. The agent sees "permission
denied: tool result denied by immune boundary", and the boundary records an incident against the local builtin tools.

Here is the sequence:
1. `run_tests` returns a structured result: `ToolResult::structured` with a JSON payload.
2. The production result filter (`ResultFilter::sanitize`, stage 9) prefixes the output of the tools in
   `EXTERNAL_OUTPUT_TOOLS` (`web_fetch`, `web_search`, `bash`, `run_tests`) with `[external:<tool>] `. The payload is
   now `[external:run_tests] {"status":"ok",...}`, yet the result still says `is_structured: true`.
3. The immune screen (`tool_immune::detect_tool_result_anomaly`) parses a structured result as JSON. It fails, and it
   scores `invalid_structured_result` 0.9, over the 0.8 threshold. The result is quarantined and withheld, and the
   vault entry is marked escalated.

An error result is unaffected, because the screen doesn't parse errors. So an agent only ever sees `run_tests` fail.

Found while testing gap-770cb5 (`run_tests` with `build: "checks"`). Its scripted agent's second, passing call came
back as the immune denial. The quarantine record says `invalid_structured_result: 0.9`.

## Why it matters

Agents get no confirmation that their tests or checks pass, and a passing run looks like a refusal. The boundary also
builds incidents against `builtin:local`, which can lead to response controls on a healthy tool.

## Where

- `crates/roko-agent/src/dispatcher/mod.rs`: `apply_production_result_filter`.
- `crates/roko-agent/src/safety/result_filter.rs`: `sanitize` and `EXTERNAL_OUTPUT_TOOLS`.

## Plan

After the filter, a result is structured only if every text block still parses as JSON. A noted or cut payload becomes
text, the screen reads it as text, and the model sees it with its note.

## Done when

A structured success of an external tool passes the production chain and the immune screen, and reaches the model with
its note. Checked by the test in `[[verify]]`.

## Notes

- Found and fixed by roko-speed on `work/gap-770cb5`.
