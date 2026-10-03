"""Tests for R-H4 and R-M3 (replay_h4.py, task 3355): S04's prequential routing replay over a LOG1-shaped
fixture. No model is called, and no real `roko learn self-model replay` or self-model run is needed: traces and
forecasts are built by hand, the same shape those mechanisms write (the module docstring).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_h4.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import replay_h4
import replay_runner
from test_analysis import EXPERIMENT, run_record, write_run
from test_econ import trace

INSTANCES = [f"F1-l{level}-{n:04d}" for level in range(1, 4) for n in range(1, 4)]  # 9 tasks


def log1_tree(root: Path) -> Path:
    """roko_fixed and cheap_direct on the same 9 tasks: roko_fixed passes 6 of 9, cheap_direct 3 of 9, so there
    is a real cost/VS spread for the policies and the best-static-arm comparison to work with."""
    results = root / "results"
    for run_id, arm, pass_every in (("roko", "roko_fixed", 3), ("cheap", "cheap_direct", 2)):
        records = []
        for position, instance in enumerate(INSTANCES, 1):
            label = int(position % pass_every != 0)
            gated = {"verdict": "passed"} if arm == "roko_fixed" and label else {}
            record = run_record(instance, 1, run_id=run_id, arm=arm, label=label, cost=0.05 if arm == "roko_fixed"
                                else 0.01, **gated)
            record["stream"].update(id="p1_core", position=position)
            records.append(record)
        write_run(results / EXPERIMENT, records)
    return results


def write_traces(path: Path) -> Path:
    """Two orderings of H4-B1 (passes like cheap_direct, at roko_fixed's cost) and of lcb_aci (passes like
    roko_fixed, at cheap_direct's cost): a calibrated policy should beat the uncalibrated cascade on CPR here."""
    lines = []
    for ordering in (0, 1):
        for position, instance in enumerate(INSTANCES, 1):
            lines.append(trace("H4-B1", ordering, instance, ["roko_fixed"], position % 2 != 0, 0.05))
            lines.append(trace("lcb_aci", ordering, instance, ["cheap_direct"], position % 3 != 0, 0.01))
    path.write_text("\n".join(json.dumps(line) for line in lines) + "\n", encoding="utf-8")
    return path


def write_forecasts(path: Path, *, calibrated: bool) -> Path:
    """20 {attempt_key, forecast, outcome} rows. Calibrated: the forecast equals the empirical frequency in each
    of two bins (10 at 0.2 with 2 positives, 10 at 0.8 with 8 positives) -- ECE 0, Brier better than the 0.5 base
    rate. Miscalibrated: every forecast is 0.5, regardless of the same outcomes -- ECE is the full gap."""
    rows = []
    for index in range(20):
        outcome = 1 if (index < 2 or 10 <= index < 18) else 0
        forecast = (0.2 if index < 10 else 0.8) if calibrated else 0.5
        rows.append({"attempt_key": f"a{index}", "forecast": forecast, "outcome": outcome})
    path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")
    return path


def test_r_h4_replay_is_deterministic(tmp_path):
    results = log1_tree(tmp_path)
    traces = write_traces(tmp_path / "traces.jsonl")
    forecasts = write_forecasts(tmp_path / "forecasts.jsonl", calibrated=True)
    params = {"traces": traces, "forecasts": forecasts}
    first = replay_runner.run(results, [EXPERIMENT], "h4", seed=3, params=params)
    again = replay_runner.run(results, [EXPERIMENT], "h4", seed=3, params=params)
    assert replay_runner.canonical(first) == replay_runner.canonical(again)
    found = first["estimates"]
    assert found["r_h4"]["evaluated"] is True and found["r_m3"]["evaluated"] is True


def test_r_h4_is_not_evaluated_without_traces(tmp_path):
    results = log1_tree(tmp_path)
    found = replay_runner.run(results, [EXPERIMENT], "h4", seed=1)["estimates"]
    assert found["r_h4"] == {"evaluated": False,
                             "reason": "no traces: roko learn self-model replay has not run on this matrix"}
    assert found["r_m3"]["evaluated"] is False


def test_r_h4_compares_policy_cpr_against_the_best_static_arm_and_h4_b1():
    econ_report = {
        "arms": {"roko_fixed": {"cpr_usd": 0.10, "vs_rate": 0.6}, "cheap_direct": {"cpr_usd": 0.05, "vs_rate": 0.3}},
        "policies": {"H4-B1": {"cpr_usd": 0.09, "vs_rate": 0.55},
                    "lcb_aci": {"cpr_usd": 0.03, "vs_rate": 0.58},  # <= 0.85*0.05 and <= 0.09: passes
                    "cascade": {"cpr_usd": 0.06, "vs_rate": 0.40}},  # > 0.85*0.05 (0.0425): fails
    }
    result = replay_h4.r_h4(econ_report)
    assert result["evaluated"] and result["best_static_cpr"] == 0.05
    assert result["bar_cpr"] == pytest.approx(0.0425)
    assert result["policies"]["lcb_aci"]["passes_sc2"] is True
    assert result["policies"]["cascade"]["passes_sc2"] is False
    assert result["any_passes_sc2"] is True
    assert "H4-B1" not in result["policies"]  # the cascade itself is the bar, not a candidate


def test_r_h4_not_evaluated_with_no_priced_arm():
    result = replay_h4.r_h4({"arms": {"x": {"cpr_usd": None, "vs_rate": 0.5}}, "policies": {}})
    assert result == {"evaluated": False, "reason": "no arm in the matrix has a known CPR"}


def test_brier_ece_and_auroc_on_a_perfectly_calibrated_forecaster():
    pairs = [(0.2, 0), (0.2, 0), (0.2, 0), (0.2, 0), (0.2, 1),  # 1/5 positive at p = 0.2
            (0.8, 1), (0.8, 1), (0.8, 1), (0.8, 1), (0.8, 0)]  # 4/5 positive at p = 0.8
    assert replay_h4.ece(pairs, bins=2) == pytest.approx(0.0, abs=1e-12)  # 2 bins: one per forecast value
    assert 0.0 < replay_h4.brier_score(pairs) < 0.25  # better than a coin flip's 0.25
    area = replay_h4.auroc(pairs)
    assert area is not None and 0.0 <= area <= 1.0


def test_ece_is_the_full_gap_when_every_forecast_is_the_same_wrong_number():
    pairs = [(0.5, 1)] * 8 + [(0.5, 0)] * 2  # base rate 0.8, forecast stuck at 0.5
    assert replay_h4.ece(pairs, bins=1) == pytest.approx(0.3, abs=1e-12)  # one bin: the whole population


def test_auroc_is_none_without_both_outcome_classes():
    assert replay_h4.auroc([(0.5, 1), (0.9, 1)]) is None
    assert replay_h4.auroc([(0.5, 0), (0.9, 0)]) is None


def test_r_m3_reports_calibrated_vs_miscalibrated_forecasters(tmp_path):
    """With the default 10 bins over only 20 rows, each forecast value's own bins see just 2 draws, too few to
    show calibration over sampling noise alone, so this compares them at bins=2 -- one per forecast value --
    where a calibrated forecaster's bins match their own observed frequency exactly."""
    calibrated = replay_h4.load_forecasts(write_forecasts(tmp_path / "cal.jsonl", calibrated=True))
    miscalibrated = replay_h4.load_forecasts(write_forecasts(tmp_path / "miscal.jsonl", calibrated=False))
    cal_pairs = [(forecast, outcome) for _key, forecast, outcome in calibrated]
    bad_pairs = [(forecast, outcome) for _key, forecast, outcome in miscalibrated]
    assert replay_h4.ece(cal_pairs, bins=2) == pytest.approx(0.0, abs=1e-12)
    assert replay_h4.ece(bad_pairs, bins=2) == pytest.approx(0.3, abs=1e-12)
    cal, bad = replay_h4.r_m3(cal_pairs), replay_h4.r_m3(bad_pairs)
    assert cal["ece"] < bad["ece"]  # even at the default 10 bins, the calibrated one is still the better of the two
    assert len(cal["calibration_bins"]) == 10  # 20 rows over 10 bins: every bin gets 2, none empty


def test_load_forecasts_rejects_a_malformed_row(tmp_path):
    path = tmp_path / "bad.jsonl"
    path.write_text('{"attempt_key": "a", "forecast": 0.5}\n', encoding="utf-8")  # no outcome
    with pytest.raises(ValueError, match="attempt_key, forecast, outcome"):
        replay_h4.load_forecasts(path)
