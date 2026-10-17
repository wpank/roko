+++
id = "bug-c1b04b"
kind = "bug"
title = "Roko's emitted tasks.toml and roko.toml open with 'Emitted by the ViabilityBench driver ... for one benchmark task ... the benchmark's hidden checks never enter it', so only the Roko arms' agents can read that they face a benchmark with hidden checks"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-3c921e worker (claude-agent), 2026-10-17 (planemit.py's TASKS_TEMPLATE, CONFIG_HEAD, CLAUDE_CLI_CONFIG_HEAD and LADDER_HEAD; driver/testdata/planemit/pinned.tasks.toml line 1)"
discovered_from = "bug-3c921e"
anchors = ["benchmarks/viabilitybench/driver/planemit.py::TASKS_TEMPLATE", "benchmarks/viabilitybench/driver/planemit.py::CONFIG_HEAD", "benchmarks/viabilitybench/driver/planemit.py::CLAUDE_CLI_CONFIG_HEAD", "benchmarks/viabilitybench/driver/planemit.py::LADDER_HEAD"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-3c921e", "bug-b5eab4", "bug-d7ff47", "gap-394f28", "bug-562ee6", "bug-85e982"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_emitted_plan_names_the_benchmark' benchmarks/viabilitybench/driver/test_planemit.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -k test_no_emitted_plan_names_the_benchmark -q"
+++

## Problem

Each file that `planemit.emit` writes into a Roko arm's task workdir opens with a comment that names the benchmark
and its hidden checks:

- `plans/<slug>/tasks.toml` (`TASKS_TEMPLATE`): "# Emitted by the ViabilityBench driver (planemit-4) for one benchmark
  task. Roko's agent sees this file and its verify command; the benchmark's hidden checks never enter it.";
- `roko.toml` (`CONFIG_HEAD`, `CLAUDE_CLI_CONFIG_HEAD`, `LADDER_HEAD`): "# Emitted by the ViabilityBench driver
  (planemit-4) for one benchmark run: ...".

`driver/testdata/planemit/pinned.tasks.toml` and `pinned.roko.toml` show them. The files sit in the workdir while Roko
runs (`run_roko` removes them only afterwards), so an agent of the roko_fixed, roko_full or ladder arms that lists or
reads its workspace learns that it is on a ViabilityBench task with hidden checks. The direct, mini-swe-agent, Claude
Code and Codex arms' agents are given no such text.

## Why it matters

Goal `proof`. The arms do not run under the same conditions: H1 compares Roko arms with the direct arm, and only the
Roko side can read "benchmark" and "hidden checks" (evaluation awareness), as bug-d7ff47 found for four families of
eight. "ViabilityBench" is also a search key into the public repository, bug-b5eab4's concern. Under Will's decision 9
the coordinator decided on 2026-10-17 that no benchmark-internal identifier reaches an agent (bug-3c921e's Decision).
New headers change `TEMPLATE_SHA256` and the golden files, so before the pre-registration lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/planemit.py::TASKS_TEMPLATE`, `::CONFIG_HEAD`, `::CLAUDE_CLI_CONFIG_HEAD` and
  `::LADDER_HEAD`: each one's leading comment lines.
- `benchmarks/viabilitybench/driver/testdata/planemit/pinned.tasks.toml` and `pinned.roko.toml`, the golden files
  `test_pinned_mode_is_byte_identical_to_before_ladder_mode` compares with.

## Current state

The headers came with planemit (T11). bug-3c921e named the plan by the task's alias; the headers did not change.
`run_roko_plan`'s own roko.toml (`_roko_toml`) has no such header.

## Plan

1. Drop the headers, or reword them without the benchmark (for example "Generated for this workspace."). Bump
   `TEMPLATE_VERSION` and `LADDER_VERSION`, update the pinned SHA-256s and regenerate the golden files.
2. Add `test_no_emitted_plan_names_the_benchmark` to `driver/test_planemit.py`: in every mode (pinned, ladder, Claude
   CLI, roko_full's overlay), the emitted tasks.toml and roko.toml match none of `viabilitybench`, `benchmark` and
   `hidden` (case-insensitive).

## Done when

- [x] No file planemit writes into a workspace names the benchmark or its hidden checks.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-3c921e worker, 2026-10-17, while renaming `PlanSpec.key` to `alias`.
- The plan-slice arm's planner prompt (`run_roko_plan.PLANNER_PROMPT`) speaks of "a hidden command" in Roko's sense
  (a command a person could not read); it was left as it is.
- The headers are dropped, not reworded: a comment that says nothing is no use to the operator, who has the module's
  docstring, and any wording would be one more thing for an agent to read. The files' TOML is what it was, so Roko
  reads the same configuration and plan. The plan slug's `vb-` prefix is bug-d68575's.
- Found while comparing every Roko arm's files (2026-10-17): main's planemit cannot emit roko_fixed_glm,
  roko_fixed_kimi or roko_fixed_mini at all, since pinned mode writes `[models.glm-4.7]` unquoted (bug-562ee6, p1:
  LOG1 block A); and the Roko runner's recorded prompt hash covers only the pinned templates, so the ladder version,
  which the header showed, is now in no record (bug-85e982).

## Decision (2026-10-17)

Whether the files Roko's agents can read may say that they were emitted for a benchmark task with hidden checks had
no recorded default. Under Will's decision 9 of 2026-10-15 (technical questions with no recorded default take the
coordinator's recommendation, the conservative option, recorded on the item with the reasoning; Will can veto it
later), the coordinator decided that no benchmark-internal identifier reaches an agent: the emitted tasks.toml and
roko.toml carry no header, so neither names the benchmark, its hidden checks or the driver's template versions. The
change lands before the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. Only the Roko arms' agents could read that
they face a benchmark task with hidden checks, so the arms H1 compares did not run under the same conditions
(evaluation awareness), and "ViabilityBench" and `planemit-4` are search keys into the public repository, where the
driver, the generators and the truth suites are. Dropping the comments is the conservative option: TOML ignores them,
so Roko reads the same configuration and plan, and nothing an agent is graded on changes.
