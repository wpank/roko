"""Tests for S09 §4.1's toolkit: confidence sequences (cs.py), McNemar's exact test (mcnemar.py) and CUPED (cuped.py),
on fixed synthetic data with reference values; nothing here calls a model (task 3335).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_toolkit.py -q
"""

from __future__ import annotations

import math
from fractions import Fraction
from random import Random

import pytest

import cs
import cuped
import mcnemar
import metrics
from audit import estimate, lottery  # on sys.path through cs
from test_analysis import run_record

ALPHA = 0.05


def bernoulli_stream(rng: Random, p: float, n: int) -> list[float]:
    return [1.0 if rng.random() < p else 0.0 for _ in range(n)]


# --- confidence sequences -----------------------------------------------------------------------------------------


def test_confidence_sequence_keeps_anytime_coverage():
    """Over 400 streams of 200 outcomes at each of three means, the CS excludes the true mean at some time in at most
    α of the streams (Monte Carlo margin included), so an analyst who stops the moment it excludes the truth (the
    worst stopping rule) is wrong at most that often. A 95% Wald interval re-checked after every task, the fixed-n
    interval misused, fails that test several times over."""
    rng = Random(3335)
    streams, length = 400, 200
    for mean in (0.1, 0.5, 0.8):
        data = [bernoulli_stream(rng, mean, length) for _ in range(streams)]
        missed = sum(not cs.covers(stream, mean, ALPHA) for stream in data)
        assert missed / streams <= ALPHA + 3 * math.sqrt(ALPHA * (1 - ALPHA) / streams), (mean, missed)
        peeked = sum(_wald_ever_misses(stream, mean) for stream in data)
        assert peeked / streams > 2 * ALPHA, (mean, peeked)  # optional stopping breaks the fixed-n interval
    # Stopped at a data-dependent time (the first t at which the running mean looks most extreme), the interval of
    # the grid CS at that time still covers the mean whenever `covers` says the whole sequence does.
    for stream in data[:20]:
        sequence = cs.confidence_sequence(stream, ALPHA, grid=200)
        stop = max(range(len(stream)), key=lambda t: abs(sum(stream[:t + 1]) / (t + 1) - 0.8))
        if cs.covers(stream, 0.8, ALPHA):
            assert sequence[stop][0] <= 0.8 <= sequence[stop][1]


def test_confidence_sequence_is_the_running_intersection_and_matches_s05s_sequence():
    rng = Random(7)
    stream = bernoulli_stream(rng, 0.3, 150)
    sequence = cs.confidence_sequence(stream, ALPHA, grid=100)
    assert len(sequence) == len(stream) and all(entry is not None for entry in sequence)
    for before, after in zip(sequence, sequence[1:]):  # a running intersection only ever narrows
        assert before[0] <= after[0] and after[1] <= before[1]
    assert sequence[-1][1] - sequence[-1][0] < 0.5 and sequence[-1][0] <= 0.3 <= sequence[-1][1]
    # S05's betting_cs on Z = ε·x with z_max = ε tests the means j/grid too: the same intervals, by its own loop.
    audit = estimate.betting_cs([value * lottery.EPS_FLOOR for value in stream], ALPHA, z_max=lottery.EPS_FLOOR,
                                grid=100)
    assert sequence == [tuple(entry) for entry in audit]
    # Bounds other than [0, 1] rescale: outcomes in [2, 4] give the [0, 1] sequence moved to [2, 4].
    shifted = cs.confidence_sequence([2 + 2 * value for value in stream], ALPHA, lo=2.0, hi=4.0, grid=100)
    assert shifted == [(2 + 2 * low, 2 + 2 * high) for low, high in sequence]


def test_anytime_p_is_one_over_the_running_peak_of_the_capital():
    rng = Random(11)
    null_stream = bernoulli_stream(rng, 0.5, 120)
    p = cs.anytime_p(null_stream, 0.5)
    capital = estimate.hedged_capital(null_stream, 0.5, ALPHA)
    assert p == [min(1.0, 1.0 / max(1.0, *capital[:t + 1])) for t in range(len(capital))]
    assert all(later <= earlier for earlier, later in zip(p, p[1:]))  # a running peak: p never rises
    assert (min(p) <= ALPHA) == (not cs.covers(null_stream, 0.5, ALPHA))
    shifted = bernoulli_stream(rng, 0.9, 120)
    assert cs.anytime_p(shifted, 0.5)[-1] < 0.01  # a real difference drives p down
    # H1's quality stop: roko_full's VS rate below half of fd_claude's shows as a CS for x − 0.5·y below 0.
    weak = bernoulli_stream(rng, 0.1, 200)
    strong = bernoulli_stream(rng, 0.9, 200)
    values, lo, hi = cs.paired_contrast(weak, strong, weight=0.5)
    assert (lo, hi) == (-0.5, 1.0)
    assert cs.confidence_sequence(values, ALPHA, lo=lo, hi=hi, grid=150)[-1][1] < 0
    with pytest.raises(ValueError):
        cs.paired_contrast([1.0], [1.0, 0.0])
    with pytest.raises(ValueError):
        cs.confidence_sequence([1.5], ALPHA)


def _wald_ever_misses(stream: list[float], mean: float, start: int = 10) -> bool:
    """Whether a 95% Wald interval, recomputed after every outcome from the 10th on, ever excludes `mean`."""
    total = 0.0
    for t, value in enumerate(stream, 1):
        total += value
        if t >= start:
            rate = total / t
            half = 1.959963984540054 * math.sqrt(max(rate * (1 - rate), 1e-12) / t)
            if not rate - half <= mean <= rate + half:
                return True
    return False


# --- McNemar --------------------------------------------------------------------------------------------------------


def test_mcnemar_exact_p_matches_hand_computed_binomial_tails():
    # b = 10, c = 2: 2·P(X ≤ 2), X ~ Bin(12, 1/2) = 2·(1 + 12 + 66)/4096
    assert mcnemar.exact_p(10, 2) == mcnemar.exact_p(2, 10) == float(Fraction(158, 4096))
    assert mcnemar.exact_p(9, 1) == float(Fraction(2 * 11, 1024))
    assert mcnemar.exact_p(6, 0) == 2 / 64
    assert mcnemar.exact_p(0, 0) == 1.0 and mcnemar.exact_p(5, 5) == 1.0  # 2·P(X ≤ 5) > 1 is capped
    found = mcnemar.compare([(1, 0)] * 10 + [(0, 1)] * 2 + [(1, 1)] * 30 + [(0, 0)] * 8)
    assert (found.b, found.c, found.n) == (10, 2, 50) and found.difference == pytest.approx(8 / 50)
    assert found.p == pytest.approx(0.03857421875, abs=1e-15)
    with pytest.raises(ValueError):
        mcnemar.compare([(1, 2)])


def test_mcnemar_pairs_come_from_seed_one_of_both_arms():
    records = [run_record("F1-l1-0001", 1, arm="roko_full", label=1),
               run_record("F1-l1-0001", 1, arm="fd_claude", label=0),
               run_record("F1-l1-0002", 1, arm="roko_full", label=0, unknown=True),
               run_record("F1-l1-0002", 1, arm="fd_claude", label=1),
               run_record("F1-l1-0002", 2, arm="roko_full", label=1),  # seed 2: not a seed-1 pair
               run_record("F1-l1-0003", 1, arm="roko_full", label=1, status="infra_error"),
               run_record("F1-l1-0003", 1, arm="fd_claude", label=1),
               run_record("F2-l2-0004", 1, arm="roko_full", label=1)]
    pairs, missing = mcnemar.seed_pairs(records, "roko_full", "fd_claude")
    variant = records[0]["task"]["spec_variant"]
    assert pairs == {("F1-l1-0001", variant): (1, 0), ("F1-l1-0002", variant): (0, 1)}  # unknown counts as 0
    assert missing == [("F1-l1-0003", variant), ("F2-l2-0004", variant)]
    with pytest.raises(metrics.MetricsError):
        mcnemar.seed_pairs(records + [run_record("F1-l1-0001", 1, arm="roko_full", run_id="run-b")], "roko_full",
                           "fd_claude")


# --- CUPED ----------------------------------------------------------------------------------------------------------


def test_cuped_reproduces_the_hand_computed_adjustment():
    first = [(1, 0.9), (1, 0.7), (0, 0.3), (1, 0.6), (1, 0.8)]
    second = [(0, 0.5), (1, 0.8), (0, 0.2), (0, 0.4), (1, 0.6)]
    found = cuped.adjusted_contrast(first, second)
    # Exact reference: θ = Σ(x − x̄)(y − ȳ) / Σ(x − x̄)² over all ten tasks, then each group's mean of y − θ(x − x̄).
    exact = [(Fraction(y), Fraction(str(x))) for y, x in first + second]
    x_bar = sum(x for _, x in exact) / len(exact)
    y_bar = sum(y for y, _ in exact) / len(exact)
    theta = sum((x - x_bar) * (y - y_bar) for y, x in exact) / sum((x - x_bar) ** 2 for _, x in exact)
    adjusted = [y - theta * (x - x_bar) for y, x in exact]
    groups = (adjusted[:5], adjusted[5:])
    means = [sum(group) / 5 for group in groups]
    variance = sum(sum((value - mean) ** 2 for value in group) / 4 / 5 for group, mean in zip(groups, means))
    assert found.theta == pytest.approx(float(theta), abs=1e-12)
    assert found.estimate == pytest.approx(float(means[0] - means[1]), abs=1e-12)
    assert found.se == pytest.approx(math.sqrt(variance), abs=1e-12)
    assert found.dim == pytest.approx(0.8 - 0.4, abs=1e-12)
    assert found.vrf == pytest.approx(found.dim_se ** 2 / found.se ** 2) and found.vrf > 1.3  # S03 C7's target
    assert found.ci[0] < found.estimate < found.ci[1] and (found.n_first, found.n_second) == (5, 5)
    with pytest.raises(ValueError):
        cuped.adjusted_contrast([(1, 0.5), (0, 0.5)], [(1, 0.5), (0, 0.5)])  # a constant covariate adjusts nothing


def test_cuped_keeps_coverage_and_narrows_the_interval():
    """300 simulated between-task contrasts with a true effect of 0.15 and a covariate that predicts the outcome:
    CUPED's 95% interval covers 0.15 about as often as DiM's, and is narrower."""
    rng = Random(42)
    covered = dim_covered = 0
    widths, dim_widths = [], []
    for _ in range(300):
        groups = []
        for effect in (0.15, 0.0):
            group = []
            for _ in range(60):
                x = rng.random()
                group.append((0.2 + 0.6 * x + effect + rng.gauss(0, 0.1), x))
            groups.append(group)
        found = cuped.adjusted_contrast(*groups)
        covered += found.ci[0] <= 0.15 <= found.ci[1]
        dim_covered += found.dim_ci[0] <= 0.15 <= found.dim_ci[1]
        widths.append(found.ci[1] - found.ci[0])
        dim_widths.append(found.dim_ci[1] - found.dim_ci[0])
    assert 0.92 <= covered / 300 <= 0.98 and 0.92 <= dim_covered / 300 <= 0.98
    assert sum(widths) < 0.6 * sum(dim_widths)


def test_log1_covariates_use_roko_fixed_gpt_oss_and_fall_back_to_the_stratum():
    log1 = [run_record("F1-l1-0001", seed, arm="roko_fixed", run_id="log1-a", label=label)
            for seed, label in ((1, 1), (2, 0), (3, 1))]
    log1 += [run_record("F1-l1-0002", seed, arm="roko_fixed", run_id="log1-a", label=0) for seed in (1, 2, 3)]
    log1 += [run_record("F1-l1-0003", 1, arm="roko_fixed", run_id="log1-glm", label=1, model="glm-4.7")]
    log1 += [run_record("F1-l1-0001", 1, arm="cheap_direct", run_id="log1-b", label=0)]
    tasks = [dict(record["task"]) for record in log1[:1]]
    tasks.append({**tasks[0], "instance_id": "F1-l1-0003"})  # LOG1 ran it only on glm-4.7: the stratum's mean
    variant = tasks[0]["spec_variant"]
    covariates, from_stratum = cuped.log1_covariates(log1, tasks)
    assert covariates == {("F1-l1-0001", variant): pytest.approx(2 / 3),
                          ("F1-l1-0003", variant): pytest.approx((2 / 3 + 0) / 2)}
    assert from_stratum == [("F1-l1-0003", variant)]
    with pytest.raises(metrics.MetricsError):
        cuped.log1_covariates(log1, [{**tasks[0], "instance_id": "F4-l3-0001", "family": "F4", "ladder": 3}])
