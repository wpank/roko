+++
id = "bug-b19ce8"
kind = "bug"
title = "The roko_plan arm records no prompt: run_roko_plan has no PROMPT_VERSION, PROMPT_SHA256 or arm_prompt, so its planner prompt and roko.toml text never reach config.prompt or config_hash"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "91fee0fa7"
source = "bug-85e982 worker (claude-agent), 2026-10-17 (vb.runner_prompt(run_roko_plan, vb.load_arm('roko_plan')) is {'version': None, 'sha256': None}: run_roko_plan.py has PLANNER_PROMPT and _roko_toml's inline roko.toml, and no PROMPT_VERSION, PROMPT_SHA256 or arm_prompt)"
discovered_from = "bug-85e982"
anchors = ["benchmarks/viabilitybench/driver/run_roko_plan.py::PLANNER_PROMPT", "benchmarks/viabilitybench/driver/run_roko_plan.py::_roko_toml"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-85e982", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_plan_slice_arm_records_its_prompt' benchmarks/viabilitybench/driver/test_run_roko_plan.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko_plan.py -k test_the_plan_slice_arm_records_its_prompt -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T19:20:11Z"
commit = "91fee0fa7"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T17:26:15Z"
forced = false
evidence = "91fee0fa7: run_roko_plan.arm_prompt(arm), which vb.runner_prompt prefers and vb run records as config.prompt (so in config_hash), reports PROMPT_VERSION run-roko-plan-1 and the sha256 (planemit._sha256) of every text the runner writes or formats for the arm's agents, read on each call: PLANNER_PROMPT; roko.toml's templates, planemit's LADDER_HEAD (the head _roko_toml wrote inline was its text, byte for byte), PROVIDER_TABLE and MODEL_TABLE and the new ROKO_TOML_TAIL (gates, tier turn cap, conductor, runner, budget, learning) and RUNG_LINE; and run_cli.PROMPT_SHA256, since the planner's session runs under run_cli's fixed flags, settings and environment. _roko_toml now formats LADDER_HEAD + ROKO_TOML_TAIL. The verify passes: test_the_plan_slice_arm_records_its_prompt (driver/test_run_roko_plan.py), 1 passed: roko_plan's recorded prompt names run-roko-plan-1 and a sha256, and one more newline in PLANNER_PROMPT, ROKO_TOML_TAIL, RUNG_LINE, LADDER_HEAD, PROVIDER_TABLE, MODEL_TABLE or run_cli.PROMPT_SHA256 (monkeypatched) changes the sha256 and keeps the version, each roko.toml template changing the roko.toml _roko_toml writes. On main's code it fails: {'version': None, 'sha256': None}. The instrument is otherwise the same: _roko_toml writes main's text byte for byte for the roko_plan arm on each of its 3 models, 4 [roko] settings and 2 turn caps (24 texts), and every other arm file records the prompt it did (roko_plan: null to run-roko-plan-1 243199c803afccec...). Bench pytest: 822 passed, 17 skipped (14 with no roko binary in the worktree, the analysis stack, two opt-in sweeps)."
+++

## Problem

`vb run` records each runner's prompt in the run's config (`config.prompt`, through `vb.runner_prompt` since
bug-85e982), and it enters the run's `config_hash`. The plan-slice arm's runner, `run_roko_plan` (roko_plan, 3319),
defines no `PROMPT_VERSION`, `PROMPT_SHA256` or `arm_prompt`, so every roko_plan run records
`config.prompt = {"version": null, "sha256": null}`. Its agents run under text this module writes: `PLANNER_PROMPT`
(the frontier planner's authoring prompt, which also states the plan's rules), the roko.toml that `_roko_toml` builds
inline (its gates, budget, conductor and learning tables) and planemit's `PROVIDER_TABLE` and `MODEL_TABLE`. A change
to any of them leaves roko_plan's `config_hash` as it was; only `harness_sha` tells the runs apart.

To see it (2026-10-17): `arm = vb.load_arm("roko_plan")`, then `vb.runner_prompt(vb.load_runner(arm), arm)` returns
`{"version": None, "sha256": None}`; `grep -n PROMPT benchmarks/viabilitybench/driver/run_roko_plan.py` finds
nothing.

## Why it matters

Goal `proof`. As bug-85e982 found for run_roko's arms: a run's `config_hash` is its configuration's identity in
`vb report` (a run's records must share one), the metric records' `config_hashes` and the showcase bundle, and the
pre-registration lock (gap-394f28) should identify every arm's emitted text. roko_plan is the plan-level slice's arm
(S09 §4.9/E12, budget line BL13), exploratory and gated after G0, hence p3.

## Where

- `benchmarks/viabilitybench/driver/run_roko_plan.py`: `PLANNER_PROMPT`, and `_roko_toml`, which writes roko.toml
  inline rather than from a template constant.
- `benchmarks/viabilitybench/driver/vb.py::runner_prompt`: what `vb run` records, a runner's `arm_prompt(arm)` or its
  `PROMPT_VERSION` and `PROMPT_SHA256`.
- `benchmarks/viabilitybench/driver/planemit.py`: `PROVIDER_TABLE`, `MODEL_TABLE`, which `_roko_toml` formats.

## Current state

Since run_roko_plan was written (3319). bug-85e982 made run_roko's arms record the planemit templates they emit from
(`run_roko.arm_prompt`, `planemit.template_identity`); run_roko_plan, a separate runner, was outside it.

## Plan

1. Move `_roko_toml`'s fixed text into template constants, so that it can be hashed.
2. Give run_roko_plan a prompt: a version (e.g. `run-roko-plan-1`) and a sha256 over `PLANNER_PROMPT`, those
   templates and planemit's `PROVIDER_TABLE` and `MODEL_TABLE`, read when `vb run` asks for it (`arm_prompt(arm)`,
   as run_roko's), so that a changed template changes it.
3. Add `test_the_plan_slice_arm_records_its_prompt` to `driver/test_run_roko_plan.py`: the prompt `vb.runner_prompt`
   gives roko_plan names a version and a sha256, and changing `PLANNER_PROMPT`, a roko.toml template or either
   planemit table (monkeypatched) changes it.

## Done when

- [x] roko_plan's `config.prompt` names a version and a sha256, and changes when its planner prompt or a template of
  its roko.toml changes.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-85e982 worker, 2026-10-17, checking the prompt every Roko arm file records.
- Fixed 2026-10-17 (the Plan as written). `run_roko_plan.arm_prompt(arm)`, which `vb.runner_prompt` prefers, reports
  `PROMPT_VERSION` (`run-roko-plan-1`) and the sha256 (`planemit._sha256`) of `PLANNER_PROMPT`, roko.toml's templates
  and the planner session's fixed invocation, read on each call: planemit's `LADDER_HEAD` (the head `_roko_toml` wrote
  inline was its text, byte for byte), `PROVIDER_TABLE` and `MODEL_TABLE`, this module's new `ROKO_TOML_TAIL` (the
  gates, the tier's turn cap, the conductor, the runner, the budget and the learning tables) and `RUNG_LINE`, and
  `run_cli.PROMPT_SHA256`, since the planner runs under run_cli's flags, settings and environment. `_roko_toml` now
  formats `LADDER_HEAD + ROKO_TOML_TAIL` and writes the same roko.toml as before. roko_plan records
  `run-roko-plan-1` / `243199c803afccec...` where it recorded nulls; every other arm records what it did.
