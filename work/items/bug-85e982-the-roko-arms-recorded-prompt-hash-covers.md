+++
id = "bug-85e982"
kind = "bug"
title = "The Roko arms' recorded prompt hash covers only planemit's pinned openai_compat templates: the ladder, Claude CLI and overlay templates that roko_ladder, roko_full and fr_claude emit from never reach config.prompt or config_hash"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-c1b04b worker (claude-agent), 2026-10-17 (run_roko.PROMPT_VERSION/PROMPT_SHA256 = planemit.TEMPLATE_VERSION/TEMPLATE_SHA256; LADDER_TEMPLATE_SHA256, CLAUDE_CLI_TEMPLATE_SHA256 and _overlay_text reach no record; LADDER_VERSION was only ever in the emitted roko.toml's header)"
discovered_from = "bug-c1b04b"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/planemit.py::LADDER_TEMPLATE_SHA256"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-c1b04b", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_roko_arms_prompt_hash_covers_every_template' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_the_roko_arms_prompt_hash_covers_every_template -q"
+++

## Problem

`vb run` records each runner's prompt as `config.prompt = {version: runner.PROMPT_VERSION, sha256:
runner.PROMPT_SHA256}`, and it enters the run's `config_hash`. The Roko runner sets both from planemit's pinned
openai_compat templates alone (`run_roko.PROMPT_VERSION = planemit.TEMPLATE_VERSION`, `PROMPT_SHA256 =
planemit.TEMPLATE_SHA256 = sha256(TASKS_TEMPLATE + CONFIG_TEMPLATE)`). The Roko arms emit from other templates too:
fr_claude from `CLAUDE_CLI_CONFIG_HEAD` (`CLAUDE_CLI_TEMPLATE_SHA256`), roko_ladder and roko_full from the ladder's
(`LADDER_TASKS_TEMPLATE`, `LADDER_HEAD`, `PROVIDER_TABLE`, `MODEL_TABLE`: `LADDER_TEMPLATE_SHA256`, `LADDER_VERSION`)
and roko_full also from `_overlay_text`. None of those reaches `config.prompt`, so a change to them leaves those arms'
`config_hash`, and the metric records' `config_hashes`, as they were; only `harness_sha` tells the runs apart. Until
bug-c1b04b, `LADDER_VERSION` was at least in each emitted roko.toml's header comment, and so in the transcript's
`emit` event; it is now in no record.

To see it (2026-10-17, at bug-c1b04b's branch): `run_roko.PROMPT_SHA256 == planemit.TEMPLATE_SHA256`, and
`planemit.LADDER_TEMPLATE_SHA256` and `CLAUDE_CLI_TEMPLATE_SHA256` appear in no runner's prompt and in no record.

## Why it matters

Goal `proof`. A run's `config_hash` is its configuration's identity in `vb report` (`analysis/report.py`, a run's
records must share one), the metric records' provenance (`config_hashes`) and the showcase bundle. For the ladder
arms (H2's routed Roko), roko_full and fr_claude it misses the very templates their agents run under. Before the
pre-registration lock (gap-394f28), so that the locked configuration identifies every arm's emitted plan.

## Where

- `benchmarks/viabilitybench/driver/run_roko.py`: `PROMPT_VERSION`, `PROMPT_SHA256` (module constants, one per
  runner, which `vb.py::cmd_run` reads with `getattr`).
- `benchmarks/viabilitybench/driver/planemit.py`: `TEMPLATE_SHA256`, `LADDER_TEMPLATE_SHA256`,
  `CLAUDE_CLI_TEMPLATE_SHA256`, `_overlay_text`.

## Current state

Since ladder mode (3310), the Claude CLI kind (3318) and the overlay (3360) were added beside the pinned templates.

## Plan

1. Make the Roko runner's prompt hash cover every template it can emit: for example `PROMPT_SHA256` over
   `TEMPLATE_SHA256`, `LADDER_TEMPLATE_SHA256`, `CLAUDE_CLI_TEMPLATE_SHA256` and the overlay's tables, and a
   `PROMPT_VERSION` that names both versions; or let a runner report its prompt per arm (a function `vb.py` calls).
2. Add `test_the_roko_arms_prompt_hash_covers_every_template` to `driver/test_run_roko.py`: changing any of the
   ladder, Claude CLI or overlay templates (monkeypatched) changes the hash `vb run` records for the arm that uses it.

## Done when

- [ ] Every Roko arm's `config.prompt` changes when a template it emits from changes.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-c1b04b worker, 2026-10-17, while removing the emitted files' headers, which held the version.
