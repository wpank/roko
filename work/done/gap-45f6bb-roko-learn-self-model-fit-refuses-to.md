+++
id = "gap-45f6bb"
kind = "gap"
title = "roko learn self-model fit refuses to run without --offline, its only mode, prints two Brier skills over different windows without naming either (0.079 and -0.024 on the roko repo), and scores today's predictor over the logs while saying nothing of the forecasts the runs made (v7's live shadow forecasts: skill -0.19)"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/commands", "roko-learn/self-model"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "508b7a5ff"
source = "round-v8 roko-learnguard survey ($V8/roko-learnguard/survey, roko a0089c19c)"
discovered_from = "agent:roko-learnguard (round v8)"
anchors = ["crates/roko-cli/src/commands/learn_self_model.rs", "crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate"]
links = { depends_on = [], blocks = [], related = ["gap-00e982"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn self_model_fit_runs_offline_by_default' crates/roko-cli/src && cargo test -p roko-cli self_model_fit_runs_offline_by_default"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T13:08:33Z"
commit = "508b7a5ff"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-11-12T12:00:45Z"
forced = false
evidence = "508b7a5ff. crates/roko-cli learn_self_model.rs: the fit runs offline without --offline (the flag stays accepted); its lines name their windows (prequential, over all N units; calibration, over the gate's last W outcomes); a live-forecasts line scores the runs' shadow forecasts for the arm that ran against their verdicts, with the base-rate Brier beside it (crates/roko-learn self_model/ingest.rs score_live_forecasts, FitReport::live_forecasts in the JSON). Red first on 57f0fdf6f: self_model_fit_runs_offline_by_default failed (the fit bailed without --offline); green: it and the_fit_names_its_windows_and_scores_the_live_forecasts pass with the learn_self_model tests; lib suites roko-cli 4109/0 (bins 10/0, 483/0), roko-learn 1413/0, roko-serve 1091/0, the known flake bug-5c77d6 passing alone and in the rerun; clippy -D warnings and nightly fmt clean (tmp/hermes-roko-v8/roko-learnguard/logs/45f6bb-2e6afb-chain.log, 45f6bb-2e6afb-rerun.log)."
+++

## Problem

The self-model's offline fit is hard to start and easy to misread.

- **It needs a flag that is its only mode.** `roko learn self-model fit` exits 2 with "error: only `--offline` fits
  exist: the fit reads the logs and dispatches nothing". `--offline` is the only fit there is, so the command should
  run it.
- **It prints two Brier skills over different windows, and names neither.** On the roko repo (2026-11-12):

  ```
  prequential (n = 132): Brier 0.082, base rate 0.902, Brier skill 0.079, …
  calibration: not_eligible (ECE above 0.08: 0.102; Brier skill below 0.05: -0.024)
  ```

  The first line scores every unit, prequentially. The second is the calibration gate over its window, the last
  `[self_model] n_min` outcomes (`CalibrationGate::evaluate`). A reader sees a skill of 0.079 and a skill of −0.024
  for one model.
- **It reads like a measure of live routing, but isn't one.** The fit scores today's predictor over the logs. It
  doesn't score the forecasts the self-model made at the time. On v7's Undertow workspace the fit reports AUROC 0.84
  and Brier skill 0.32, while the shadow forecasts v7's runs logged had a skill of −0.19 (Brier 0.288 against a
  base-rate 0.242, 93 tries with a verdict, `runs/*/predictions.jsonl`). Both numbers are true, but they answer
  different questions.

## Why it matters

- The roko-for-Hermes guide reports what the self-model learned. The command has to run as written, and its numbers
  have to say what they cover.
- Goal: learning.

## Where

- `crates/roko-cli/src/commands/learn_self_model.rs`: the fit's arguments and its printout.
- `crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate`: the gate's window.

## Plan

1. **No flag needed:** `fit` runs offline without `--offline`, and the flag stays accepted.
2. **Name both windows:**
   - "prequential over all 132 units";
   - "the gate's last W outcomes: ECE …, Brier skill …".
3. **Add the live forecasts:** one line scoring what the runs' shadow forecasts said for the arm that ran, against
   its verdict: n, Brier, base-rate Brier and skill. That line answers how well the self-model predicted at the time.
4. **Test:** `self_model_fit_runs_offline_by_default` covers no flag, both windows named, and the live-forecast line.

## Done when

- The `[[verify]]` passes.
- `roko learn self-model fit` on a copy of Undertow's workspace prints the prequential, gate and live-forecast lines,
  each saying what it covers.

## Notes

- Found by roko-learnguard's round v8 survey (`$V8/findings/roko-learnguard.md`).
- Related: gap-00e982, which wiped the self-model's state at phase 5.4 when the price snapshot changed.
