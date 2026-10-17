+++
id = "bug-85e982"
kind = "bug"
title = "The Roko arms' recorded prompt hash covers only planemit's pinned openai_compat templates: the ladder, Claude CLI and overlay templates that roko_ladder, roko_full and fr_claude emit from never reach config.prompt or config_hash"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "23e068324"
source = "bug-c1b04b worker (claude-agent), 2026-10-17 (run_roko.PROMPT_VERSION/PROMPT_SHA256 = planemit.TEMPLATE_VERSION/TEMPLATE_SHA256; LADDER_TEMPLATE_SHA256, CLAUDE_CLI_TEMPLATE_SHA256 and _overlay_text reach no record; LADDER_VERSION was only ever in the emitted roko.toml's header)"
discovered_from = "bug-c1b04b"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/planemit.py::LADDER_TEMPLATE_SHA256"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-c1b04b", "gap-394f28", "bug-b19ce8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_roko_arms_prompt_hash_covers_every_template' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_the_roko_arms_prompt_hash_covers_every_template -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T12:44:19Z"
commit = "23e068324"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T11:04:34Z"
forced = false
evidence = "23e068324: a runner may report its prompt per arm, and the Roko runner does (the item's Plan option 2). vb.runner_prompt(runner, arm), which cmd_run now records as config.prompt (and so in config_hash), is the runner's arm_prompt(arm) when it has one, else PROMPT_VERSION and PROMPT_SHA256 as before (harness.py states both). run_roko.arm_prompt calls planemit.template_identity for the templates emit writes the arm's files from, in the mode run_roko._plan_spec gives the arm (_routed, now shared by the two): pinned openai_compat (TEMPLATE_VERSION planemit-6, TEMPLATE_SHA256), pinned claude_cli (planemit-6, CLAUDE_CLI_TEMPLATE_SHA256), ladder (LADDER_VERSION planemit-ladder-3, LADDER_TEMPLATE_SHA256), and, with an overlay, also the overlay's tables as _overlay_text writes them for every supported key (OVERLAY_PROBE, whose keys are SUPPORTED_OVERLAY_KEYS), the version then naming OVERLAY_VERSION planemit-overlay-1 (new) after the mode's. template_identity reads the module's templates on every call, so a changed template changes the recorded prompt; the three sha256 constants keep their values (one helper, _sha256). run_roko's PROMPT_VERSION and PROMPT_SHA256, which were only roko_fixed's, are gone. Recorded now: roko_fixed on all four models planemit-6 392ef78bd0db0e9e... (as before, so their config_hash is unchanged), fr_claude planemit-6 5fd8ec4f4505c049..., roko_ladder planemit-ladder-3 9cfdc3b2ecbff6a2..., roko_full planemit-ladder-3+planemit-overlay-1 19610df569d94ca9...; the other runners' prompts are unchanged (mini-loop-2, run-msa-1, claude-code-1, codex-cli-1). The verify passes: test_the_roko_arms_prompt_hash_covers_every_template (driver/test_run_roko.py), 1 passed. For each of planemit's seven templates (TASKS_TEMPLATE, CONFIG_TEMPLATE, CLAUDE_CLI_CONFIG_TEMPLATE, LADDER_TASKS_TEMPLATE, LADDER_CONFIG_TEMPLATE, PROVIDER_TABLE, MODEL_TABLE) and _overlay_text, changed in turn by one more newline (monkeypatched), the arms whose recorded prompt changes are exactly the arms whose emitted tasks.toml and roko.toml change, with every run_roko arm file emitted through run_roko._plan_spec: ladder mode's templates roko_ladder and roko_full, the Claude CLI config fr_claude, the overlay roko_full, CONFIG_TEMPLATE roko_fixed on its four models, TASKS_TEMPLATE those and fr_claude. A vb run through the fake roko records roko_fixed's prompt in its manifest's config. It fails before the fix (vb has no runner_prompt), and with arm_prompt put back to the old constants it fails on fr_claude's prompt (TEMPLATE_SHA256 where CLAUDE_CLI_TEMPLATE_SHA256 is due). real_arm_spec now takes the arm's first model by default and builds the endpoint as vb.make_plan does (vb._endpoint), so it covers fr_claude, whose provider names no key. Bench pytest: 811 passed, 17 skipped (14 with no roko binary in the worktree, the analysis stack, two opt-in sweeps); with VB_TEST_ROKO_BIN at a roko built at f9318c6a0, the Roko and other runners' tests (test_planemit, test_run_roko, test_run_roko_routed, test_fr_claude, test_run_roko_plan, test_run_roko_sandbox, test_run_cli, test_run_codex, test_run_msa) 86 passed. Built on work/bug-562ee6 (the test emits roko_fixed_glm, roko_fixed_kimi and roko_fixed_mini). Filed bug-b19ce8 (p3): roko_plan's runner (run_roko_plan) reports no prompt at all, so its config.prompt is null."
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

- [x] Every Roko arm's `config.prompt` changes when a template it emits from changes.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-c1b04b worker, 2026-10-17, while removing the emitted files' headers, which held the version.
