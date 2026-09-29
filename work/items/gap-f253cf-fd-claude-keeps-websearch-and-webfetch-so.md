+++
id = "gap-f253cf"
kind = "gap"
title = "fd_claude keeps WebSearch and WebFetch, so once the repo is public an agent can fetch the hidden suites"
status = "open"
triage = "verified"
last_verified = 2026-09-29
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-ccarm's report on gap-c4f364, merged in 157f1d434)"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/census.py", "benchmarks/viabilitybench/arms/fd_claude.toml", "benchmarks/viabilitybench/families/f1_pyconv/hidden.py", "benchmarks/viabilitybench/families/f4_kvtool/hidden.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-c4f364", "gap-8c3752", "gap-308373", "gap-154f93"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_fd_claude_cannot_fetch_the_hidden_suites' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_fd_claude_cannot_fetch_the_hidden_suites -q"
+++

## Problem

`run_cli.py`'s fixed flags (:111-112) isolate settings and MCP, but restrict no built-in tool. The fd_claude agent therefore keeps WebSearch and WebFetch; the driver even counts `webSearchRequests` (:429).

The repository is to be public, and the truth suites live in it: `families/f1_pyconv/hidden.py`, `families/f4_kvtool/hidden.py` and the plan-slice suites. An agent could search for the benchmark and fetch a hidden suite.

The census would catch that only if the canary reached the transcript:

- Each `hidden.py` carries the canary GUID on line 2 (`vb-canary-f6c9…`), and the census counts every `vb-canary-…` string in the transcript (census.py:20-24).
- But Claude Code's WebFetch returns a model-processed answer to its prompt, not the raw page, so the canary line may never reach the transcript. A search snippet may miss it as well.

Nobody has checked either path.

## Why it matters

Pilot benchmark (epic spec-567e52), SC4: a fetched suite inflates the frontier arm's VS rate, and nothing would show it.

## Where

- `driver/run_cli.py`: the fixed flags and the CLI argv.
- `driver/census.py`: the canary scan.
- `arms/fd_claude.toml`: the arm definition.

## Current state

At e43d3a033 no flag disables the web tools, and no test covers a web fetch.

## Plan

1. **Preferred: remove the tools.** Disable web tools for the arm (`--disallowedTools WebFetch WebSearch`, or an explicit `--tools` allowlist). Check in the live probe (gap-154f93) that the init event lists no web tools.
2. **If the arm must keep web access** (check S08 §4.9's definition of frontier-direct), prove that the census catches a fetch, and add a rule that doesn't depend on the canary: any fetch or search that names `families/*/hidden.py`, a plan-slice suite or the repository's benchmark path marks the run `leak_suspected`.
3. Apply the same rule to every other CLI arm with web tools, for example fd_codex.
4. Add `test_fd_claude_cannot_fetch_the_hidden_suites`.

## Done when

- [x] fd_claude can't reach a hidden suite through the web, or such a fetch always makes the run `leak_suspected`.
- [x] The `[[verify]]` command passes.

## Notes

- Until this lands, don't make the repository public while fd_claude runs are pending, or run them before publishing.
- **Done 2026-09-29 (wk-bench-fix2): the tools are removed (plan step 1).** S08 §4.9 defines fd_claude by
  `ClaudeCliAgent`'s flags minus Roko's prompt, and that adapter already supports `--disallowed-tools`. S08 §4.9
  also sanctions deny rules for isolation, and the tasks need no web (S08 §4.2 (3)). S09 §4.8's "at their defaults"
  therefore still holds for every other tool.
  - `run_cli.py` passes `--disallowed-tools WebFetch,WebSearch` and adds deny rules for both to `--settings`, which
    also bind subagents.
  - It kills a session whose `init` event still offers a `Web…` tool before the first turn (`infra_error`, reason
    `web_tools`), and the probe checks `no_web_tools`.
  - As a backstop, the census marks `leak_suspected` (place `web`) for any web tool call or positive web-request count
    in the transcript.
  - Not covered: any arm's shell can still reach the network (`curl`, `git clone`). Only a network sandbox closes
    that (S08 decision 4).
