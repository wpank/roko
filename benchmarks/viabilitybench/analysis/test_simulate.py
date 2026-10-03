"""Tests for the synthetic campaign simulator (simulate.py, task 3339): a fast version of the full run, with a wider
band; nothing here calls a model.

The full run, 1,000 campaigns at B = 10,000, is heavy in pure Python and runs once, off-CI (simulate.py's module
docstring); this one, 100 campaigns at B = 200, takes seconds.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_simulate.py -q
"""

from __future__ import annotations

import json
from random import Random

import pytest

import simulate

# The fast run's bands: the criteria's own, widened by about two Monte Carlo SEs at 100 campaigns.
FAST_BANDS = {"bootstrap_rate_coverage": (0.88, 1.0), "bootstrap_diff_coverage": (0.88, 1.0),
              "bootstrap_ratio_coverage": (0.88, 1.0), "cuped_coverage": (0.88, 1.0),
              "cs_anytime_coverage": (0.90, 1.0), "envelope_fwer_r": (0.0, 0.10), "envelope_fwer_c": (0.0, 0.10)}


def test_simulator_reports_coverage_in_band():
    report = simulate.simulate(planted=0.15, sims=100, b=200, seed=33)
    assert report["schema_version"] == "vb.simulation/1" and report["sims"] == 100 and report["planted"] == 0.15
    assert report["resamples"] == {"b_per_interval": 200, "intervals_per_campaign": 7,
                                   "total_replicates": 100 * 7 * 200}
    assert set(report["criteria"]) == set(simulate.CRITERIA) == set(FAST_BANDS)
    for name, (low, high) in FAST_BANDS.items():
        entry = report["criteria"][name]
        assert low <= entry["value"] <= high, (name, entry)
        assert entry["band"] == list(simulate.CRITERIA[name])
        assert entry["in_band"] == (entry["band"][0] <= entry["value"] <= entry["band"][1])
    assert report["all_in_band"] == all(entry["in_band"] for entry in report["criteria"].values())
    json.dumps(report)  # the report is plain JSON


def test_a_report_does_not_depend_on_the_number_of_workers():
    one = simulate.simulate(sims=6, b=50, seed=7, workers=1)
    two = simulate.simulate(sims=6, b=50, seed=7, workers=2)
    assert {**one, "seconds": 0} == {**two, "seconds": 0}


def test_the_generator_plants_the_difference_and_keeps_p1_cores_shape():
    rows = simulate.campaign(Random(1), planted=0.15)
    assert len(rows) == 6 * 5 * 4 * 3 * 2 and {row["arm"] for row in rows} == {"A", "B"}
    assert len({row["task"]["instance_id"] for row in rows}) == 120
    assert {(row["task"]["family"], row["task"]["ladder"]) for row in rows} == {
        (family, level) for family in simulate.FAMILIES for level in simulate.LEVEL_BANDS}
    # Over many campaigns, A's runs beat B's by the planted 0.15 on average, and the population figures agree
    # (14,400 runs per arm: the Monte Carlo SE of the difference is about 0.006).
    rng = Random(2)
    pooled = {"A": [0, 0], "B": [0, 0]}
    for _ in range(40):
        for row in simulate.campaign(rng, planted=0.15):
            pooled[row["arm"]][0] += row["_vs"]
            pooled[row["arm"]][1] += 1
    rate_a, rate_b = (pooled[arm][0] / pooled[arm][1] for arm in ("A", "B"))
    assert rate_a - rate_b == pytest.approx(0.15, abs=0.02)
    assert rate_a == pytest.approx(simulate.population_rate(0.15), abs=0.02)
    assert simulate.population_ratio(0.15) == pytest.approx(simulate.population_rate(0.15)
                                                            / simulate.population_rate(0.0))
    with pytest.raises(ValueError):
        simulate.campaign(Random(3), planted=0.3, spread=0.1)  # some task's p_A would pass 1
    with pytest.raises(SystemExit):
        simulate.main(["--sims", "1", "--b", "10", "--planted", "0.4"])
