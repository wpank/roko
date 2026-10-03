"""R-H4 and R-M3: S04's prequential routing replay over LOG1 (S09 §4.3 "Replays"; SC2, SC1; task 3355).

    vb.py replay --experiment LOG1 --adapter h4 [--seed N] [--traces FILE] [--forecasts FILE]

An adapter under `replay_runner` (3354), so it runs deterministically on arm-hashed data like every replay.

**R-H4** (SC2): each routing policy's VS rate, CPR and cost per task, from `roko learn self-model replay`'s own
trace lines (S04 §4.9's `TraceLine`: one JSON line per policy, ordering and task). This adapter never
recomputes any of that itself: it hands LOG1's own run records and the traces to `analysis.econ.build`, the
same Python economics report S04's live comparison reads (decision 6103 (b)), and reads policy (a) or (b)'s CPR
back out of it against SC2's bar (≤ 0.85 x the best static cheap arm's CPR, and ≤ H4-B1's). The mechanism that
writes the traces -- `roko learn self-model replay --matrix ... --policies H4-B0,H4-B1,H4-B2,H4-B3,lcb_aci,
cascade --orderings 50 --seed N` -- comes from another slice (S04.T06/T07b); without `--traces` (it has not run
on this matrix) R-H4 is not evaluated, the same as R-H5's cells with no `--risk` file.

**R-M3** (SC1): Brier skill (against the base rate), ECE (10 equal-mass bins) and AUROC of S04's forecaster,
from `{"attempt_key", "forecast", "outcome"}` rows -- S04's own `self_model.forecast`/`self_model.outcome` pair,
joined by attempt. Without `--forecasts` (S04's self-model has not scored this matrix) R-M3 is not evaluated
either. `calibration_bins` is `fig_f6_reliability.py`'s own input shape (`calibration_bin_mean`,
`calibration_bin_freq`, each bin's n), so the figure needs no separate scorer.

**Which records.** LOG1's rows carry every block; `econ.build` itself excludes the plan-level slice and
`infra_error`/`leak_suspected` runs (`metrics.EXCLUDED`), so this adapter passes every record through unfiltered.

API:
    SC2_CPR_RATIO; ECE_BINS; H4_POLICIES; H4_B1
    brier_score(pairs: Sequence[tuple[float, int]]) -> float
    calibration_bins(pairs: Sequence[tuple[float, int]], bins: int = ECE_BINS) -> list[dict]
    ece(pairs: Sequence[tuple[float, int]], bins: int = ECE_BINS) -> float
    auroc(pairs: Sequence[tuple[float, int]]) -> float | None
    load_forecasts(path: Path) -> list[tuple[str, float, int]]    # (attempt_key, forecast, outcome)
    r_h4(econ_report: dict) -> dict
    r_m3(pairs: Sequence[tuple[float, int]]) -> dict
    estimate(matrix, rng, traces=None, forecasts=None, b=econ.B) -> dict
"""

from __future__ import annotations

import json
import math
import random
import sys
from collections.abc import Sequence
from pathlib import Path

if str(Path(__file__).resolve().parent) not in sys.path:  # econ.py, report.py and friends live beside this file
    sys.path.insert(0, str(Path(__file__).resolve().parent))
if str(Path(__file__).resolve().parents[1]) not in sys.path:  # the benchmark directory, for replay_runner
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import econ  # noqa: E402
import replay_runner  # noqa: E402

SC2_CPR_RATIO = 0.85  # S04 SC2: policy (a) or (b)'s CPR <= this share of the best static cheap arm's
ECE_BINS = 10  # S04 S4.7, SC1: 10 equal-mass bins
H4_POLICIES = ("H4-B0", "H4-B1", "H4-B2", "H4-B3", "lcb_aci", "cascade")  # S04 S4.9's --policies
H4_B1 = "H4-B1"  # the uncalibrated cascade SC2 also compares against


def brier_score(pairs: Sequence[tuple[float, int]]) -> float:
    """mean((forecast - outcome)^2), S04 SC1's forecaster score."""
    return math.fsum((p - y) ** 2 for p, y in pairs) / len(pairs)


def calibration_bins(pairs: Sequence[tuple[float, int]], bins: int = ECE_BINS) -> list[dict]:
    """`bins` equal-mass bins of `pairs` by forecast, each with its mean forecast, observed frequency and n
    (`fig_f6_reliability.py`'s own `calibration_bin_mean`/`calibration_bin_freq` shape)."""
    ordered = sorted(pairs, key=lambda pair: pair[0])
    n = len(ordered)
    out = []
    for index in range(bins):
        start, end = index * n // bins, (index + 1) * n // bins
        chunk = ordered[start:end]
        if not chunk:
            continue
        out.append({"bin": index, "n": len(chunk), "calibration_bin_mean": math.fsum(p for p, _ in chunk) / len(chunk),
                    "calibration_bin_freq": math.fsum(y for _, y in chunk) / len(chunk)})
    return out


def ece(pairs: Sequence[tuple[float, int]], bins: int = ECE_BINS) -> float:
    """Expected calibration error: the n-weighted mean |bin mean forecast - bin observed frequency|."""
    rows = calibration_bins(pairs, bins)
    total = sum(row["n"] for row in rows)
    return math.fsum(row["n"] * abs(row["calibration_bin_mean"] - row["calibration_bin_freq"]) for row in rows) / total


def auroc(pairs: Sequence[tuple[float, int]]) -> float | None:
    """The rank-based AUROC of `pairs`' forecasts against their 0/1 outcomes; None with no positive or no
    negative outcome (it is undefined there)."""
    positive = [p for p, y in pairs if y == 1]
    negative = [p for p, y in pairs if y == 0]
    if not positive or not negative:
        return None
    wins = sum((p > n) + 0.5 * (p == n) for p in positive for n in negative)
    return wins / (len(positive) * len(negative))


def load_forecasts(path: Path) -> list[tuple[str, float, int]]:
    """`{"attempt_key", "forecast", "outcome"}` rows from `path` (one per line), as (attempt_key, forecast,
    outcome) triples; `outcome` is VS- (0 or 1)."""
    triples = []
    for number, text in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
        if not text.strip():
            continue
        row = json.loads(text)
        if not (isinstance(row, dict) and isinstance(row.get("attempt_key"), str)
                and isinstance(row.get("forecast"), (int, float)) and row.get("outcome") in (0, 1)):
            raise ValueError(f"{path}:{number}: not an {{attempt_key, forecast, outcome}} row")
        triples.append((row["attempt_key"], float(row["forecast"]), int(row["outcome"])))
    return triples


def r_h4(econ_report: dict) -> dict:
    """SC2 over `econ.build`'s own `policies`/`arms`: policy (a) or (b)'s CPR (`cpr_usd`, S04 §4.9's default that
    counts audit spend) against the best static cheap arm's and H4-B1's."""
    policies, arms = econ_report["policies"], econ_report["arms"]
    cheap_arms = {name: cell for name, cell in arms.items() if cell.get("cpr_usd") is not None}
    if not cheap_arms:
        return {"evaluated": False, "reason": "no arm in the matrix has a known CPR"}
    best_static = min(cheap_arms.values(), key=lambda cell: cell["cpr_usd"])
    bar = best_static["cpr_usd"] * SC2_CPR_RATIO
    b1 = policies.get(H4_B1, {}).get("cpr_usd")
    candidates = {name: cell for name, cell in policies.items()
                 if name not in (H4_B1,) and cell.get("cpr_usd") is not None}
    passing = {name: cell["cpr_usd"] <= bar and (b1 is None or cell["cpr_usd"] <= b1)
              for name, cell in candidates.items()}
    return {"evaluated": True, "best_static_cpr": best_static["cpr_usd"], "bar_cpr": bar, "h4_b1_cpr": b1,
            "policies": {name: {"cpr_usd": candidates[name]["cpr_usd"], "vs_rate": candidates[name]["vs_rate"],
                               "passes_sc2": passing[name]} for name in sorted(candidates)},
            "any_passes_sc2": any(passing.values())}


def r_m3(pairs: Sequence[tuple[float, int]]) -> dict:
    """SC1 over `pairs`: Brier, the Brier skill score against the base rate, ECE and AUROC, with the bins."""
    base_rate = math.fsum(y for _, y in pairs) / len(pairs)
    score = brier_score(pairs)
    base_brier = base_rate * (1 - base_rate)
    bss = 1 - score / base_brier if base_brier else None
    return {"evaluated": True, "n": len(pairs), "brier": score, "bss": bss, "ece": ece(pairs),
           "ece_bound": 0.08, "ece_within_bound": ece(pairs) <= 0.08, "auroc": auroc(pairs),
           "calibration_bins": calibration_bins(pairs)}


def estimate(found: replay_runner.OutcomeMatrix, rng: random.Random, traces: str | None = None,
            forecasts: str | None = None, b: int = econ.B) -> dict:
    """R-H4 and R-M3 over the matrix (module docstring)."""
    del rng  # econ.build and the Brier/ECE/AUROC formulas here draw nothing random; b only sizes the bootstrap
    if traces:
        trace_rows = econ.load_traces([Path(traces)])
        report = econ.build(list(found.records), traces=trace_rows, b=int(b))
        h4 = r_h4(report)
        h4["econ_report_sha256"] = replay_runner.canonical(report)
    else:
        h4 = {"evaluated": False, "reason": "no traces: roko learn self-model replay has not run on this matrix"}
    if forecasts:
        pairs = [(forecast, outcome) for _attempt_key, forecast, outcome in load_forecasts(Path(forecasts))]
        m3 = r_m3(pairs)
    else:
        m3 = {"evaluated": False, "reason": "no forecasts: S04's self-model has not scored this matrix"}
    return {"r_h4": h4, "r_m3": m3}


replay_runner.register(replay_runner.Adapter("h4", "r-h4-1", estimate, params=("traces", "forecasts", "b")))
