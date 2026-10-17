+++
id = "bug-562ee6"
kind = "bug"
title = "planemit's pinned mode cannot emit a model whose slug holds a dot: [models.glm-4.7] parses as nested tables, so roko_fixed_glm, roko_fixed_kimi and roko_fixed_mini (LOG1 block A) fail emit and vb run refuses them at preflight"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-c1b04b worker (claude-agent), 2026-10-17 (planemit.emit on run_roko._plan_spec for each Roko arm file: roko_fixed_glm, roko_fixed_kimi and roko_fixed_mini raise PlanEmitError 'roko.toml must hold exactly the pinned provider and model', on main's planemit and the branch's)"
discovered_from = "bug-c1b04b"
anchors = ["benchmarks/viabilitybench/driver/planemit.py::emit"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-e9218e", "bug-c1b04b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_pinned_mode_emits_a_model_whose_slug_has_a_dot' benchmarks/viabilitybench/driver/test_planemit.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -k test_pinned_mode_emits_a_model_whose_slug_has_a_dot -q"
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

- [ ] Every pinned Roko arm emits its plan.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-c1b04b worker, 2026-10-17, comparing every Roko arm's emitted files before and after removing
  their headers; main's planemit refuses these three arms the same way.
