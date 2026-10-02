"""Tests for the paired bootstrap (analysis/bootstrap.py), on synthetic rows; nothing here calls a model.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_bootstrap.py -q
"""

from __future__ import annotations

from random import Random

import pytest

import bootstrap


def _row(family: str, level: int, task_num: int, seed: int, arm: str, label: float) -> dict:
    return {"task": {"family": family, "ladder": level, "instance_id": f"{family}-l{level}-{task_num:04d}"},
            "seed": seed, "arm": arm, "label": label}


def _diff(rows: list[dict]) -> float:
    """mean(A's label) - mean(B's label); 0.0 when a replicate drew neither arm for some reason."""
    a = [row["label"] for row in rows if row["arm"] == "A"]
    b = [row["label"] for row in rows if row["arm"] == "B"]
    if not a or not b:
        return 0.0
    return sum(a) / len(a) - sum(b) / len(b)


def _ratio(rows: list[dict]) -> float:
    """sum(A's cost) / sum(A's vs): a $/VS-shaped ratio statistic, skewed enough to need BCa."""
    costs = [row["cost"] for row in rows if row["arm"] == "A"]
    vs = [row["vs"] for row in rows if row["arm"] == "A"]
    total_vs = sum(vs)
    if not total_vs:
        return 0.0
    return sum(costs) / total_vs


def _planted_rows(gap: float, *, families: tuple[str, ...] = ("F1", "F2"), levels: tuple[int, ...] = (1, 2, 3),
                  tasks_per_stratum: int = 8, seeds: int = 3, base_rate: float = 0.75, seed: int = 2026,
                  ) -> list[dict]:
    """Rows for arms A and B, paired by (task, seed): A's label is Bernoulli(base_rate), B's is
    Bernoulli(base_rate - gap), drawn from a local, seeded RNG, so the data is fixed across test runs."""
    rng = Random(seed)
    rows = []
    for family in families:
        for level in levels:
            for task_num in range(tasks_per_stratum):
                for run in range(seeds):
                    rows.append(_row(family, level, task_num, run, "A", float(rng.random() < base_rate)))
                    rows.append(_row(family, level, task_num, run, "B", float(rng.random() < base_rate - gap)))
    return rows


def test_reference_values_on_a_small_fixed_set():
    """One stratum, 4 tasks, 2 seeds: A's label is always 1, B's is 1 at seed 1 and 0 at seed 2. Resampling draws
    whole (task, seed) units, so every replicate's A-mean is exactly 1 and its B-mean is the drawn share of seed-1
    units; both are hand-computable, and so is the point estimate (never resampled)."""
    rows = [_row("F1", 1, task_num, seed, arm, label) for task_num in range(4) for seed in (1, 2)
           for arm, label in (("A", 1.0), ("B", 1.0 if seed == 1 else 0.0))]
    result = bootstrap.paired_bootstrap(rows, _diff, b=500, seed=3)
    assert result.estimate == pytest.approx(0.5)  # mean(A) 1.0 - mean(B) 0.5 on the untouched data
    assert result.b == 500 and len(result.replicates) == 500
    assert all(0.0 <= value <= 1.0 for value in result.replicates)
    assert result.low <= result.estimate <= result.high


def test_stratified_bootstrap_recovers_a_planted_difference():
    rows = _planted_rows(gap=0.15)
    result = bootstrap.paired_bootstrap(rows, _diff, b=3000, seed=7)
    assert result.low > 0, f"a planted 0.15 gap's interval should exclude 0, got [{result.low}, {result.high}]"
    assert 0.0 < result.estimate < 0.3


def test_null_difference_interval_covers_zero():
    rows = _planted_rows(gap=0.0)
    result = bootstrap.paired_bootstrap(rows, _diff, b=3000, seed=7)
    assert result.low <= 0 <= result.high, f"a null gap's interval should cover 0, got [{result.low}, {result.high}]"


def test_percentile_bootstrap_is_deterministic_for_a_seed():
    rows = _planted_rows(gap=0.1, seed=11)
    first = bootstrap.paired_bootstrap(rows, _diff, b=400, seed=42)
    second = bootstrap.paired_bootstrap(rows, _diff, b=400, seed=42)
    assert first == second


def test_a_different_seed_can_give_different_bounds():
    rows = _planted_rows(gap=0.1, seed=11)
    first = bootstrap.paired_bootstrap(rows, _diff, b=400, seed=1)
    second = bootstrap.paired_bootstrap(rows, _diff, b=400, seed=2)
    assert first.replicates != second.replicates


def test_bca_bounds_for_a_cost_ratio_statistic():
    rng = Random(5)
    rows = []
    for level in (1, 2, 3):
        for task_num in range(10):
            for seed in range(3):
                vs = 1.0 if rng.random() < 0.6 else 0.0
                cost = round(rng.uniform(0.05, 0.4), 3)
                rows.append(_row("F1", level, task_num, seed, "A", 0.0) | {"cost": cost, "vs": vs})
    result = bootstrap.paired_bootstrap(rows, _ratio, b=2000, seed=9, method="bca")
    assert result.method == "bca"
    assert result.low <= result.estimate <= result.high
    assert result.low > 0  # every cost and every included row's vs sum is positive


def test_percentile_and_bca_agree_closely_on_a_symmetric_statistic():
    rows = _planted_rows(gap=0.15, seed=4)
    percentile = bootstrap.paired_bootstrap(rows, _diff, b=4000, seed=1, method="percentile")
    bca = bootstrap.paired_bootstrap(rows, _diff, b=4000, seed=1, method="bca")
    assert percentile.estimate == bca.estimate
    assert abs(percentile.low - bca.low) < 0.05
    assert abs(percentile.high - bca.high) < 0.05


def test_rejects_no_rows():
    with pytest.raises(bootstrap.BootstrapError):
        bootstrap.paired_bootstrap([], _diff)


def test_rejects_an_unknown_method():
    rows = _planted_rows(gap=0.1, tasks_per_stratum=2, seeds=1, levels=(1,), families=("F1",))
    with pytest.raises(bootstrap.BootstrapError):
        bootstrap.paired_bootstrap(rows, _diff, b=10, method="bootstrap-ville")


def test_rejects_a_row_missing_its_stratum_field():
    rows = [{"task": {"family": "F1"}, "seed": 1, "arm": "A", "label": 1.0}]  # no "ladder"
    with pytest.raises(bootstrap.BootstrapError):
        bootstrap.paired_bootstrap(rows, _diff, b=10)


def test_task_key_follows_metrics_convention():
    row = {"task": {"instance_id": "F1-l1-0001", "spec_variant": "precise"}}
    assert bootstrap.task_key(row) == ("F1-l1-0001", "precise")
    assert bootstrap.task_key({"task": {"instance_id": "F1-l1-0002"}}) == ("F1-l1-0002", None)
