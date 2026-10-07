+++
id = "gap-051a34"
kind = "gap"
title = "Hajek estimator bias exceeds SC1's +/-0.01 target on a tilted, ~60-unit first audit slice"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK28 gap-c06ff3)"
discovered_from = "gap-c06ff3 (backlog task 7107); independently reproduced, see body"
anchors = ["benchmarks/viabilitybench/audit/estimate.py::estimate", "benchmarks/viabilitybench/audit/lottery.py::tilt", "benchmarks/viabilitybench/audit/tests/test_estimate.py::test_hajek_ci_covers_theta_in_1000_replays"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_hajek_bias_is_flagged_on_a_tilted_60_unit_window' benchmarks/viabilitybench/audit/tests/test_estimate.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_estimate.py -k test_hajek_bias_is_flagged_on_a_tilted_60_unit_window -q"
+++

## Problem

The Hájek false-green estimator (`benchmarks/viabilitybench/audit/estimate.py::estimate`) is meant to keep
`|bias| <= 0.01` at every audited window (SC1, enforced today only at a fixed 200-unit window by
`test_hajek_ci_covers_theta_in_1000_replays`). Under **tilted** selection (`lam=lottery.LAMBDA_MAX`, i.e. M3's risk
weighting) on a small, realistic *first* audit slice, the bias runs noticeably higher than on a uniform lottery and
can approach or cross the 0.01 target, while uniform selection stays well inside it at the same size.

I reproduced this independently (not just citing PK28's number) with the real production code
(`audit.replay.replay`, 3000-5000 replays per cell, several seeds), scaling `test_estimate.py`'s own `WINDOW` shape
down to smaller totals:

| n   | rho  | selection | bias_hajek (mean of 3 seeds) | coverage |
|-----|------|-----------|-------------------------------|----------|
| 60  | 0.10 | uniform   | ~ -0.002                      | ~0.97    |
| 60  | 0.10 | tilted (lam=LAMBDA_MAX) | +0.0093 to +0.0121  | 0.95-0.96 |
| 120 | 0.10 | tilted    | +0.0033 to +0.0062            | ~0.97    |
| 60  | 0.15 | tilted    | +0.0047 to +0.0066            | ~0.97    |

PK28's own figures (+0.02 at 60 units, +0.0075 at 120, presumably at a different rho/window shape or more replays)
are larger than what I measured, but the direction and the risk are the same: at n=60, rho=0.10 (the exact
scenario `replay.py`'s own module docstring already flags as risky — "about 60 units at rho=0.10 ... the Wilson
fallback dominates coverage"), two of my three seeds land at or above +0.01, i.e. **outside** SC1's own acceptance
bound, under tilted selection only. Uniform selection is comfortably unbiased at the same size.

## Why it matters

SC1 ("Estimator validity") is M4's first deep-audit claim (`tmp/cybernetic-harness/specs/S05-deep-audits.md`,
§7 acceptance criteria) and gates the pre-registration lock for the audit pipeline (goal `cybernetic`,
spec-fef7c5). `audit.estimate`/`audit.replay` are also the reference implementation the Rust `roko-gate::audit`
module (S05 task 4, not yet built) must port bit-for-bit via `fixtures/estimators.json` — if the Python reference
silently tolerates an out-of-target bias on a realistic first slice, the Rust port inherits the same blind spot.
The existing test only ever exercises a 200-unit window, so this is currently invisible to CI.

## Where

- `benchmarks/viabilitybench/audit/estimate.py::estimate` (the Hájek ratio estimator; ratio estimators are only
  asymptotically unbiased, with O(1/n_eff) bias in general even when `pi_i` is exactly correct — tilting
  concentrates weight and lowers `n_eff` for the same `n`, which is the likely mechanical cause).
- `benchmarks/viabilitybench/audit/lottery.py::tilt` / `LAMBDA_MAX` (the M3 risk-weighted selection that produces
  the tilt).
- `benchmarks/viabilitybench/audit/tests/test_estimate.py::test_hajek_ci_covers_theta_in_1000_replays` (the only
  current bias/coverage test, fixed at 200 units; its sibling `WINDOW`/`window_units()` would need a
  size parameter to cover smaller first slices).
- `benchmarks/viabilitybench/audit/replay.py` (the CLI/library the real "first slice run" — S05 task 3 — would use
  to certify a live window; it has no size-dependent warning today).

## Current state

Confirmed open: the only bias assertion in the repo runs at a single, fixed 200-unit window (where bias stays
small, +0.0028 to +0.0061 at rho 0.10-0.30 in my run). No test or guard in `estimate.py`/`replay.py` flags a
smaller, tilted first slice as higher-risk beyond the prose note in task 7107's own "Notes" section
("With about 60 units at rho=0.10 ... the Wilson fallback dominates coverage: print n_eff next to it").

## Plan

Pick one (or combine):
1. **Raise the floor.** Require a minimum `n_eff` (or minimum window size at the worst supported tilt) before a
   window's Hájek estimate is reported as "targets met" / usable for a pre-registration decision; below it, report
   only the wider Wilson-at-n_eff interval and flag the estimate as provisional. `interval()` already distinguishes
   Wald vs Wilson by `n_eff`; a similar `bias_risk` or `n_eff < MIN_RELIABLE_N_EFF_UNDER_TILT` flag on `Estimate`
   would make this visible to every caller instead of only to someone who reads the docstring.
2. **Document and test the boundary.** At minimum, add a regression test at the specific risky cell (n~60, rho=0.10,
   lam=LAMBDA_MAX) that pins down today's actual bias (whatever it measures to be) so a future change can't silently
   make it worse, and extend `replay.py --write-fixture`/`format_report` to print whether the window is below the
   safe-tilt threshold.
3. Re-run a larger sweep (closer to PK28's 20,000-replay-per-cell scale, which the `estimate.py` module docstring
   already describes for *coverage* at 60-400 units) specifically for *bias*, across rho in {0.10, 0.15, 0.30} and
   window sizes in {60, 120, 200, 400}, to pin the real worst case precisely before choosing a threshold in (1).

## Done when

- A new or updated test demonstrates the chosen guard/behavior at the risky cell (small n, tilted selection,
  rho=0.10), and the `[[verify]]` command passes.
- `## Problem`'s table scenario (n=60, rho=0.10, tilted) is either kept within SC1's `|bias| <= 0.01` by design
  (a real algorithmic fix) or is visibly flagged as not meeting it (a guard/warning), instead of silently reporting
  `targets_met = True`-equivalent behavior.

## Notes

- No LLM spend; everything here is reproducible with `benchmarks/viabilitybench/.venv/bin/python` and the existing
  `audit` package, no new dependencies.
- EPS_FLOOR (0.05) is locked by decision 7102 ("the author changes it, never a config or a learning loop") — don't
  touch it as part of this fix.
- 2026-10-02 (filer-grpD): filed from backlog wave reports (PK28 gap-c06ff3); the bias magnitudes above are my own
  independent reproduction, not a re-quote of PK28's numbers, since my scaled-window construction differs slightly
  from whatever PK28 used — the qualitative finding (tilted > uniform bias, risk of crossing 0.01 at a ~60-unit
  rho=0.10 first slice) is what's load-bearing here, not the exact decimal.
