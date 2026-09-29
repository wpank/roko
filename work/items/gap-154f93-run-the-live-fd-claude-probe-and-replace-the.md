+++
id = "gap-154f93"
kind = "gap"
title = "Run the live fd_claude probe and replace the invented modelUsage fixture with its saved output"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-ccarm's report on gap-c4f364, merged in 157f1d434)"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/test_run_cli.py", "benchmarks/viabilitybench/arms/fd_claude.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-c4f364", "gap-8be530", "gap-f253cf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/driver/testdata/fd_claude_probe.json && grep -qw 'def test_the_recorded_probe_parses' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_the_recorded_probe_parses -q"
+++

## Problem

gap-c4f364 built the `fd_claude` arm (`driver/run_cli.py`, merged in 157f1d434) and was closed without its live probe. Its notes say: "Not done: the live probe. No real `claude` ran in this push." Until the probe runs, two things rest on guesses:

- **The cost fixture.** The `result` event in `driver/test_run_cli.py` (:38 onward) was written by hand in "Claude Code 2.1.282's stream-json shape". The U′ headline sums `modelUsage` (run_cli.py:44, :415), so a wrong field name would null or miscount every fd_claude cost.
- **The per-task keychain login.** run_cli.py points `CLAUDE_CONFIG_DIR` at a per-task directory and sets `CLAUDE_SECURESTORAGE_CONFIG_DIR` to empty (:315), to keep the default keychain entry name. The variable is undocumented (:32), so only a live session shows whether a per-task config directory still finds the subscription login.

## Why it matters

Pilot benchmark (epic spec-567e52): fd_claude is the frontier baseline of H1 and H2. Its cost and its isolation must be observed, not assumed.

## Where

`driver/run_cli.py`: the `probe` subcommand (:445), the environment (:117, :315) and the `modelUsage` parser (:415). The fixture is in `driver/test_run_cli.py`.

## Current state

At e43d3a033 the probe exists and has never run against a real `claude`. Every fd_claude test uses the invented fixture and a fake `claude`.

## Plan

1. Off-hours, run `benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/driver/run_cli.py probe --arm fd_claude --allow-network`. It costs a few cents under the subscription (D42).
2. Keep its JSON: the `system`/`init` event and the `result` event, with the CLI version. Scrub account and session identifiers, and save the result as `driver/testdata/fd_claude_probe.json`.
3. Add `test_the_recorded_probe_parses`. The parser reads the saved `modelUsage`, and the init event shows the pinned model and no MCP servers. Drop the invented fixture, or keep it only where the saved one can't serve.
4. Record whether the per-task config directory found the login, and change the environment if it didn't.

## Done when

- [ ] A real probe's output is saved in the repo, and the fd_claude tests parse it.
- [ ] The `[[verify]]` command passes.

## Notes

- This makes one real call on the subscription. Confirm D42 (subscription terms) first.
- The probe is also where to confirm that the arm has no web tools (gap-f253cf).
