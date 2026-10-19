+++
id = "gap-dd670b"
kind = "gap"
title = "Pilot A's cheap ladder misses S08's bands: gpt-oss-120b solves 1 of 12 level-1 runs where G0 wants 0.6, so F1 and F4 need re-knobbing before Pilot B"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "gap-c33709 worker (claude-agent), 2026-10-19: Pilot A's G0 page, benchmarks/viabilitybench/reports/pilot_a/go-no-go.md"
discovered_from = "gap-c33709"
anchors = ["benchmarks/viabilitybench/families/f1_pyconv/ladder.toml", "benchmarks/viabilitybench/families/f4_kvtool/ladder.toml", "benchmarks/viabilitybench/reports/pilot_a/go-no-go.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = ["gap-327242"], related = ["gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot_a_recal/g0.json && python3 -c \"import json, sys; checks = {c['id']: c['verdict'] for c in json.load(open('benchmarks/viabilitybench/reports/pilot_a_recal/g0.json'))['checks']}; sys.exit(0 if checks.get('ladder_cheap_l1') == 'pass' else 1)\""
+++

## Problem

Pilot A (gap-c33709, 2026-10-19; bundle `benchmarks/viabilitybench/reports/pilot_a/`) ran `cheap_direct` on
gpt-oss-120b over the pilot slice: F1 and F4, levels ℓ1–ℓ5, two instances each, seeds 1–3, 60 runs. G0's ladder
check fails at level 1 (`reports/pilot_a/go-no-go.md`): gpt-oss-120b solves 1 of 12 runs at ℓ1, where G0 wants
VS ≥ 0.6 and S08 §4.4's design band is ≥ 0.8.

| level | cheap_direct gpt-oss-120b VS | S08 §4.4 band (cheap direct) | fd_api gpt-5.4 VS |
|---|---|---|---|
| ℓ1 | 1/12 = 0.08 (F1 0/6, F4 1/6) | ≥ 0.8; G0 ≥ 0.6 | 1/1 |
| ℓ2 | 2/12 = 0.17 (F1 2/6, F4 0/6) | 0.6–0.8 | 1/1 |
| ℓ3 | 0/12 | 0.4–0.6 | 1/1 |
| ℓ4 | 0/12 | 0.2–0.4 | 1/1 |
| ℓ5 | 0/12 | ≤ 0.2; G0 ≤ 0.3 (passes) | 1/1 |

Per census check over the 60 runs: completion 59, visible_clean 40, hidden 3, integrity 60. 37 of the 40 visible
passes are false greens.

What the records and transcripts show:
- **F4, every level:** 28 of 30 runs fail the same three hidden checks, `keys_renamed`, `none_lost` and
  `partial_resumed`, while 29 of 30 pass the visible check. The scripts do not resume a rename that `bin/kvtool`
  stops part-way (AC3). The visible check runs the script once on the sample store, and in F4-l1-0002 seed 1 that
  run renamed all 8 keys in one call, so it never meets a stop. That run's script also calls `kvtool resume TOKEN`,
  a command kvtool does not have (`kvtool --help` lists `rename --resume TOKEN`), and the model never opened
  `docs/kvtool.md`.
- **F1:** 27 of 30 runs fail all five error-convention checks (`error.subclasses_app_error`, `code_registered`,
  `code_unique`, `docs_row`, `message_format`), and only 11 of 30 pass the visible check. Some runs rewrite modules
  outside the task: F1-l1-0002 seed 1 replaced `app/checkout/charges.py` and `refunds.py`, so every hidden check
  failed while its visible test passed. Five F1 runs submitted within two turns.

The instrument checked out on the same runs. Verifier CI was green on 200 of 200 cells under the real secret. fd_api
gpt-5.4 solved all five of its tasks (ℓ1–ℓ5, F1 and F4) through the same census, in 3–6 turns. The runaway detector
stopped a synthetic runaway and a real one (F1-l3-0001 seed 3) at 30 calls, and there were 0 canary hits. So the
level-1 result measures the tasks and the model, not a broken label.

## Why it matters

- G0 stays NO-GO until this check passes, and G0 gates LOG1's $152 (S09 §4.7).
- The ladder is the x-axis of H1's envelope. With cheap ℓ1 at 0.08, no level is cheap-viable, and the
  envelope has nothing to measure.
- gap-c33709's plan (step 5) asks for this re-tuning before Pilot B (gap-327242), whose `roko_fixed` arm runs the
  same 20 instances.

## Where

- `benchmarks/viabilitybench/families/f1_pyconv/ladder.toml` and `families/f4_kvtool/ladder.toml`: each level's
  knobs. Each family's README ("Ladder") and `gen.py` explain them.
- S08 §4.4's calibration rule: "if cheap ℓ1 < 0.6, lower `k_doc`/`k_vis` difficulty".
- Evidence: `benchmarks/viabilitybench/reports/pilot_a/` (records, `metrics.json`, `go-no-go.md`). The transcripts
  and archives are in `~/.roko-bench/viability/PILOT-A/<run>/`, which is not committed.

## Current state

- At ℓ1–ℓ2, F4 has `k_quirks = 2`, where S08 has 1, and F1 has `k_files = 3`, where S08 has 1 and 1–2. Both
  ladder files document these deviations.
- The visible checks at ℓ1 cover the main path only. F1 has `k_vis = "main_path"`. F4's visible check runs the
  script once on the sample store.

## Plan

1. **Re-knob ℓ1 and ℓ2 of both families**, using S08 §4.4's rule: lower `k_doc`/`k_vis` difficulty. ℓ5 stays as it
   is, since it passes. Candidates:
   - F4: `k_quirks` 2 → 1 at ℓ1. Either give the sample store a rename that kvtool stops part-way, so the visible
     check exercises AC3, or name the stop in ℓ1's spec.
   - F1: add a visible test that checks the convention (an AppError subclass, an `ERROR_CODES` entry and a docs
     row), or state the convention in ℓ1's spec.
2. **Bump the generator versions.** Then rerun verifier CI (`ci/verify_verifiers.py --families f1,f4`) and check
   that the pilot instances still materialize.
3. **Re-run the re-knobbed ℓ1–ℓ2 instances.** Run `cheap_direct` gpt-oss-120b on them × 3 seeds: 8 instances, 24
   runs, about $0.50 at Pilot A's $0.019 per task. Run it as experiment PILOT-A-RECAL on BL0, and only with Will's
   go-ahead. Bundle it in `reports/pilot_a_recal/` and compute its G0 page with `analysis/gates.py G0`.
4. **One cheap side check:** whether the direct loop, rather than the ladder, holds gpt-oss back. Two candidates are
   S08 decision 3's mini-swe-agent (Pilot B's optional `mini-swe` block) and the arm's reasoning effort, which
   the loop leaves at the provider's default.

## Done when

- [ ] F1's and F4's re-knobbed ℓ1–ℓ2 pass verifier CI.
- [ ] A re-run's G0 page shows `ladder_cheap_l1` passing: the `[[verify]]` command.

## Notes

- **Step 3 spends money,** so it needs Will's go-ahead. After Pilot A, BL0 has $12.84 left and the pilot cap
  $13.70; Pilot B plans $6.08 of that.
- **The suite hash changes** with the generators, so Pilot A's ℓ1–ℓ2 cells are not comparable with the re-run's.
  Say so in the bundle.
- **One runner at a time,** never through roko.

## Decision (2026-10-19, Will)

Re-tune the easy levels: make F1/F4's ℓ1–ℓ2 easier as S09's calibration intends (G0 needs cheap-arm VS ≥ 0.6 at ℓ1), then rerun Pilot A's cheap_direct arm (about $1.20) before Pilot B.
