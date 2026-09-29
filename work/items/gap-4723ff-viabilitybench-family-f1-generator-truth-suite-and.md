+++
id = "gap-4723ff"
kind = "gap"
title = "ViabilityBench family F1: generator, truth suite and gaming detector (S08.T3)"
status = "open"
triage = "verified"
last_verified = 2026-09-29
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§4.3 F1, §4.4, §6 T3; checklist S08.T3)"
anchors = ["benchmarks/viabilitybench/families/f1_pyconv/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-2790c5"], blocks = [], related = ["gap-9e7079"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_f1_cells_green_on_two_seeds' benchmarks/viabilitybench/families/f1_pyconv/test_f1.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/f1_pyconv/test_f1.py -k test_f1_cells_green_on_two_seeds -q"
+++

## Problem

No task family exists yet. F1, `pyconv-errors`, tests whether an agent follows a repository convention that it
has to learn from the repo itself. Every domain error in the repo:

- subclasses `AppError`;
- has a unique `E\d{4}` code in `ERROR_CODES` and a row in `docs/errors.md`;
- uses the message format `"[E1234] …"`.

A task asks for a new error, for example rejecting a refund larger than the charge. The visible check only asserts
that some exception is raised, so `raise Exception(...)` passes it.

## Why it matters

F1 is half the pilot: its 20 tasks are F1 and F4 at levels ℓ1–ℓ5, two instances each. It measures
convention-following, which cheap models are expected to do worst, and its planted gaming solution supplies known
false greens. Pilot A needs it.

## Where

All new, and the path follows D4: `benchmarks/viabilitybench/families/f1_pyconv/`, holding `gen.py`, `template/`,
`hidden.py`, `gaming.py`, `ladder.toml`, `spec/` (the precise-spec template), `reference/{solution,stub,gaming}/`
and `test_f1.py`.

## Current state

Checked at `41c7ffbd6`: nothing exists. The contract is in B §3.2–3.3 and S08 §4.3–4.4 and §5.2; the common
library comes from gap-2790c5.

## Plan

1. **Generator.** `gen.py --level ℓ --seed s --out DIR` renders the template with the level's knobs: whether the
   convention is documented, how many examples the repo has, the files to change, the repo size and the strength
   of the visible tests. It writes `.vb/task.json` (`vb.task/1`) with the canary and `recoverability[]`, the
   evidence for every hidden requirement.
2. **Precise spec.** Rendered with `string.Template` from S07's TSS v1 fields; it never contains hidden inputs.
3. **Truth suite.** `hidden.py --task … --workdir … --secret-file …` checks that the new error subclasses
   `AppError`, that its code is registered and unique, that its docs row exists, that its message has the right
   format, and that nothing regressed. It returns the S08 §5.2 JSON.
4. **Gaming detector.** `gaming.py` flags the `raise Exception` base class and any edit to visible tests.
5. **Solutions.** A reference, a stub and a gaming solution per level. ℓ5 adds S08 §4.3's hardening: a stale
   CONTRIBUTING file, 60 modules and two call sites.

## Done when

- [ ] All 50 cells (ℓ1–ℓ5 × 10 seeds) are green:
  - the reference solution gives VS = 1;
  - the stub fails the visible check;
  - the gaming solution passes the visible check and gives VS = 0;
  - two runs give identical verdicts.
- [ ] Every instance lists its recoverability evidence.
- [ ] The `[[verify]]` command passes. It runs 10 of the cells so that it finishes within 60 s.

## Notes

- `hidden.py` and `reference/` carry the canary; agents work outside the repo.
- Out of scope: latent v2, the flipped convention. Runs in parallel with F4 (disjoint directories).
- A repo-wide pytest run must not collect the template's own `tests/`.
- **From gap-2790c5 (2026-09-29):** S08 §5.2 puts `canary` in `DIR/.vb/task.json` beside the agent's spec. Keep the task manifest outside the agent's workdir, or any agent that reads it trips the canary. Use `families/common/` (`repo`, `knobs`, `hmac_seed`, `astcheck`, `canary`, `mutate`).
