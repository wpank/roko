+++
id = "bug-562ee6"
kind = "bug"
title = "planemit's pinned mode cannot emit a model whose slug holds a dot: [models.glm-4.7] parses as nested tables, so roko_fixed_glm, roko_fixed_kimi and roko_fixed_mini (LOG1 block A) fail emit and vb run refuses them at preflight"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "9858ef94b"
source = "bug-c1b04b worker (claude-agent), 2026-10-17 (planemit.emit on run_roko._plan_spec for each Roko arm file: roko_fixed_glm, roko_fixed_kimi and roko_fixed_mini raise PlanEmitError 'roko.toml must hold exactly the pinned provider and model', on main's planemit and the branch's)"
discovered_from = "bug-c1b04b"
anchors = ["benchmarks/viabilitybench/driver/planemit.py::emit"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-e9218e", "bug-c1b04b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_pinned_mode_emits_a_model_whose_slug_has_a_dot' benchmarks/viabilitybench/driver/test_planemit.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -k test_pinned_mode_emits_a_model_whose_slug_has_a_dot -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T11:53:44Z"
commit = "9858ef94b"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T11:04:22Z"
forced = false
evidence = "9858ef94b: planemit's pinned mode writes roko.toml's model and provider keys through _key, as ladder mode does (emit's common dict: model_key=_key(spec.model), provider_key=_key(spec.provider)), so a slug that is not a bare TOML key is quoted: [models.\"glm-4.7\"] is one key, where [models.glm-4.7] parsed as models -> glm-4 -> 7 and _check raised 'roko.toml must hold exactly the pinned provider and model, with no fallbacks and the routing ladder off'. A bare slug is written bare as before: testdata/planemit/ and TEMPLATE_SHA256 are unchanged (test_pinned_mode_is_byte_identical_to_before_ladder_mode passes). The verify passes: test_pinned_mode_emits_a_model_whose_slug_has_a_dot (driver/test_planemit.py), 1 passed: every pinned run_roko arm file (roko_fixed, roko_fixed_glm, roko_fixed_kimi, roko_fixed_mini, fr_claude), through run_roko._plan_spec on the endpoint vb.make_plan builds (vb._endpoint), emits a roko.toml whose models table has the pinned slug as its one key, written quoted for glm-4.7, kimi-k2.6 and gpt-5.4-mini, and run_roko.preflight, which vb run calls before any task, passes for each with a fake roko; with the old raw keys put back it fails with the item's PlanEmitError. Step 3: test_real_roko_accepts_every_roko_arms_preflight_plan (new, real_roko) runs run_roko.preflight with a real binary on every run_roko arm file and every model it may start on; with VB_TEST_ROKO_BIN at a roko built at f9318c6a0 (main's crates/ have changed since only in gate output checks, dashboard events and serve routes, nothing that config loading or plan validate reads) it passes, and so do all 11 arm and start-model pairs one by one (the five pinned arms, and roko_ladder and roko_full on each of their three rungs), where roko_fixed_glm, roko_fixed_kimi and roko_fixed_mini failed at emit before the fix. Each of those three also ran one toy task (F1-l1-0001, seed 1) end to end through vb run and that binary against test_run_roko's loopback ToolStub, offline and unbilled: completed, gate_passed, VS 1, its pin the only model dispatched and the only one the stub saw, no failed check. roko_plan's runner (run_roko_plan) writes its own roko.toml, already quoting its keys with planemit._key, and has no preflight. Bench pytest: 810 passed, 17 skipped (14 with no roko binary in the worktree, the new real_roko test among them, the analysis stack, two opt-in sweeps); with VB_TEST_ROKO_BIN set, test_planemit.py and test_run_roko.py 40 passed and the Roko driver tests (planemit, run_roko, routed, fr_claude, run_roko_plan, sandbox) 48 passed."
+++

## Problem

planemit's pinned mode writes roko.toml's `[models.{model_key}]` and `[providers.{provider_key}]` with the raw slug
(`emit`'s `common` dict: `model_key=spec.model`, `provider_key=spec.provider`), where ladder mode quotes each key
(`_key`: "so a slug's dots stay in one key (glm-4.7)"). A pinned model whose slug holds a dot then parses as nested
tables: `[models.glm-4.7]` is `models` -> `glm-4` -> `7`, so `_check`'s `list(config["models"]) != [spec.model]`
raises PlanEmitError ("roko.toml must hold exactly the pinned provider and model, with no fallbacks and the routing
ladder off"). Three of the four pinned roko_fixed arms have such a model: roko_fixed_glm (`glm-4.7`), roko_fixed_kimi
(`kimi-k2.6`) and roko_fixed_mini (`gpt-5.4-mini`). For each, `run_roko.preflight` emits the stand-in task's plan and
fails ("the arm cannot emit a plan"), so `vb run` exits 2 before any task; `run_task` would end every task
`infra_error`.

To see it (2026-10-17): `planemit.emit(run_roko._plan_spec(vb.load_arm("roko_fixed_glm"), "glm-4.7", ...), ws)`
raises that PlanEmitError, and the same for roko_fixed_kimi and roko_fixed_mini; roko_fixed (`gpt-oss-120b`) and
fr_claude (`claude-opus-5-5`) emit. The ladder arms quote their keys and emit.

## Why it matters

Goal `proof`. LOG1 block A (gap-e9218e, p1) runs roko_fixed on four cheap models, three of them these
(`experiments/log1.toml`), and later blocks use roko_fixed_glm too: their units would refuse to start, so the
headline H1 comparison would have one Roko model of four.

## Where

- `benchmarks/viabilitybench/driver/planemit.py::emit`: the pinned mode's `common` dict (`model_key`, `provider_key`).
- `benchmarks/viabilitybench/driver/test_planemit.py`: no test emits a pinned model with a dot; the golden pinned files
  use `gpt-oss-120b`.

## Current state

Since the pinned mode was written (T11); ladder mode has quoted its keys from the start (3310).

## Plan

1. Pass `model_key=_key(spec.model)` and `provider_key=_key(spec.provider)` in pinned mode, as ladder mode does. A
   slug without a dot is written bare as before, so the golden files and `TEMPLATE_SHA256` do not change.
2. Add `test_pinned_mode_emits_a_model_whose_slug_has_a_dot` to `driver/test_planemit.py`: every pinned Roko arm
   file's spec (`run_roko._plan_spec` on `arms/roko_fixed*.toml` and `fr_claude.toml`) emits, and roko.toml's
   `models` table has the pinned slug as its one key.
3. With a real binary (`VB_TEST_ROKO_BIN`), `plan validate --strict --dag` accepts the quoted key, as it does the
   ladder's.

## Done when

- [x] Every pinned Roko arm emits its plan.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-c1b04b worker, 2026-10-17, comparing every Roko arm's emitted files before and after removing
  their headers; main's planemit refuses these three arms the same way.
