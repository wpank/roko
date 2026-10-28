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
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "gap-c33709 worker (claude-agent), 2026-10-19: Pilot A's G0 page, benchmarks/viabilitybench/reports/pilot_a/go-no-go.md"
discovered_from = "gap-c33709"
anchors = ["benchmarks/viabilitybench/families/f1_pyconv/ladder.toml", "benchmarks/viabilitybench/families/f4_kvtool/ladder.toml", "benchmarks/viabilitybench/reports/pilot_a/go-no-go.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-0ff34c"], blocks = ["gap-327242"], related = ["gap-c33709"], supersedes = [], duplicate_of = "" }

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

## Progress (2026-10-19, claude-agent on `work/gap-dd670b`)

**Re-tuned and re-run; $1.056887 billed. F4 now passes its easy levels, F1 does not, and G0's level-1 check is
still a fail at 0.50.** The item stays open. Its next step is gap-0ff34c.

- **The re-tune** (2775a1b52; generators `f1-1.2.0` and `f4-1.2.0`). S08 §4.4's rule is to lower `k_doc` and `k_vis`
  difficulty, and ℓ1 already had S08's easiest values. So `common/knobs.py` adds one easier value to each enum, and
  ℓ1–ℓ2 of both families take them.
  - `k_doc = "spelled_out"`: F1's AC3 states the convention, in the latent's form, with one of the repo's errors as
    the example. F4's spec gets a `## kvtool` section: the commands, the dry run, the stop and its resume.
  - `k_vis = "thorough"`. F1's visible tests also check the refund returned and recorded, that a rejected
    over-refund records nothing, a registered error's message, and the registry against `docs/errors.md`. F4's
    visible check runs the script twice on a copy of the repo whose store, `tests/visible/sample.db`, has a lease
    that stops the rename once. It fails a run that exits non-zero or leaves a rename unfinished.
  - The planted gaming still passes both visible checks, as S08 SC1 requires: `raise Exception`, `exit0` and
    `dry_run`.
  - `k_quirks` stays at 2: the calibration rule names `k_doc` and `k_vis`, and the stop is what F4 measures.
  - ℓ3–ℓ5 are untouched. All 120 of their instances, under both latents, render the same pristine tree, spec hash,
    knobs and recoverability as before; only `generator_version` changed.
  - Verifier CI under the real secret (`sha256:1675cc951cf12ce3`): 200 of 200 cells green, F1 100/100 and F4
    100/100. The family, common, specops, streams, schema, audit, ci and driver tests pass, and so does Pilot A's
    65-run offline rehearsal.
- **The re-run**, experiment PILOT-A-RECAL (`experiments/pilot_a_recal.toml`), at 2775a1b52 with a clean tree.
  - `vb estimate`: $0.2893 a task at worst. The campaign dry run gave 60 runs, $2.00 at worst, and no problems.
  - The command: `vb campaign --manifest experiments/pilot_a_recal.toml --allow-network --max-cost-usd 2
    --transcripts`, under `env -i`, 2026-10-19 11:58–12:15 UTC.
  - One unit, `cheap-direct-1`, with seeds 1–3. A $2 cap split three ways would have refused the 20th task of
    Pilot A's seeds 1 and 3.
  - The experiment books to the pilot's $15 cap.
- **The outcomes** (bundle `reports/pilot_a_recal/`). 60 of 60 records: 58 `completed`, and 2 `aborted_cap`
  (`model_calls`: F1-l1-0002 and F1-l5-0002, seed 1). 0 `infra_error`, 0 `leak_suspected` and 0 canary hits. All
  542 calls answered HTTP 200 and the proxy refused none.

  | level | F1 | F4 | both | Pilot A both |
  |---|---|---|---|---|
  | ℓ1 | 1/6 | 5/6 | 6/12 = 0.50 | 1/12 = 0.08 |
  | ℓ2 | 0/6 | 6/6 | 6/12 = 0.50 | 2/12 = 0.17 |
  | ℓ3–ℓ5 | 0/18 | 0/18 | 0/36 | 0/36 |

  At ℓ1 none of the 6 visible passes is a false green; Pilot A had 7 of 8. At ℓ2, 1 of 7 is; Pilot A had 6 of 8.
- **The spend.** $1.056887 billed on BL0 over 78 attempts: $0.0176 a task, 0.29 × plan. The ledger equals the meter.
  `vb ledger report` afterwards: BL0 $2.2147 of $14, the pilot cap $2.3565 of $15, the programme $2.4111 of $400,
  nothing reserved.
- **G0** (`reports/pilot_a_recal/go-no-go.md`, `g0.json`) is NO-GO.
  - Pass: verifier CI 200/200; the synthetic runaway (offline stub on the re-tuned F1-l1-0001: `aborted_cap` at 30
    calls, in attempts of 12, 12 and 6); canaries 0; ℓ5 0.0 ≤ 0.3; $/task $0.0176.
  - Fail: ℓ1 0.50 < 0.6, so the `[[verify]]` fails.
  - Not evaluated, as in Pilot A: the ledger against the exports (gap-1011ff), SC2 (gap-129862), and Pilot B's
    checks.
- **Comparability.** The suite hash changed with the generators: `sha256:630e4dcb…` here, `sha256:ebe9c482…` in
  Pilot A. The records carry `f1-1.2.0` and `f4-1.2.0`. The ℓ1–ℓ2 cells are new tasks, not comparable with Pilot A's.
  The ℓ3–ℓ5 cells are fresh samples of the same tasks. The manifest's header says so as well.
- **What the numbers suggest.** The ladder re-tune worked where the ladder was the problem.
  - F4's cheap ℓ1–ℓ2 went from 1/12 to 11/12, and F4 ℓ1 alone (5/6 = 0.83) is inside S08's band.
  - F1 did not move (2/12, then 1/12), because its runs fail before the convention matters. Of its 12 ℓ1–ℓ2 runs, 10
    ended with the visible check failing (9 submitted after 3–15 turns; 1 hit the 30-call cap). All 12 broke the
    loop's one-bash-block format, 8 called `apply_patch` (absent from the loop's shell), 4 left no diff and 5 never
    ran the visible check.
  - Where a transcript shows the intended edit, it follows the AC3 convention, but never lands.
  - So ℓ1 = (1 + 5) / 12 is bounded by the bash-only direct loop's handling of gpt-oss-120b, not by the ladder.
    Lowering F1 further would not reach a model that submits with a failing visible check.
  - Filed **gap-0ff34c**: S08 decision 3's comparison on the pilot's ℓ1–ℓ2. That is the mini-swe-agent loop,
    `cheap_direct_msa`, which keeps arm id `cheap_direct` so G0 counts it, and optionally a reasoning-effort variant.
    Pilot B's optional `mini-swe` block is the same comparison.
- **Closing this item** needs a G0 page whose `ladder_cheap_l1` passes. The `[[verify]]` reads
  `reports/pilot_a_recal/g0.json`, which now holds this run's fail. A later re-run under the chosen loop should get
  its own bundle, with this verify repointed at it (gap-0ff34c, plan step 4).
- **`~/.config/viabilitybench`.** The listing (names, sizes, modes 0600) and the sha256 of `keys` and `secret` were
  the same before and after. The tripwire's chmods moved both ctimes. Nobody read or printed a key.
- **The done-when list.** The re-knobbed ℓ1–ℓ2 pass verifier CI: done. A re-run's G0 page with `ladder_cheap_l1`
  passing: not yet.

Checked 2026-10-27: 2026-10-26 sweep: no change since the 2026-10-19 Progress note. Still blocked on gap-0ff34c (open).
