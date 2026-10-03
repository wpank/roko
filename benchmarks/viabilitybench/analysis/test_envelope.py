"""Tests for H1's envelope (envelope.py) and S09's graphical Holm mapping (holm.py), on hand-built records and p-values;
nothing here calls a model (task 3338).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_envelope.py -q
"""

from __future__ import annotations

from fractions import Fraction
from random import Random

import pytest

import envelope
import holm
import report
import validate
from test_analysis import run_record

B = 400  # replicates per interval: enough for these well-separated cases, fast enough for CI
# P(VS) per (arm, level) for the chain case: levels 1-2 at parity, level 3 where roko_full collapses, level 4 where
# fd_claude does, so the cumulative R recovers there, and level 5 at parity again.
CHAIN_CASE = {"roko_full": {1: 0.97, 2: 0.95, 3: 0.2, 4: 1.0, 5: 0.9},
              "fd_claude": {1: 0.9, 2: 0.9, 3: 0.9, 4: 0.1, 5: 0.9}}
COSTS = {"roko_full": 0.01, "fd_claude": 0.5}


def records_for(case: dict, tasks_per_family: int = 10, seeds: int = 3, seed: int = 3338) -> list[dict]:
    """Both arms on the same tasks: families F1 and F2, levels 1-5, three seeds; labels drawn from `case`."""
    rng = Random(seed)
    rows = []
    for arm, rates in case.items():
        for level, rate in rates.items():
            for family in ("F1", "F2"):
                for task in range(1, tasks_per_family + 1):
                    for run in range(1, seeds + 1):
                        rows.append(run_record(f"{family}-l{level}-{task:04d}", run, arm=arm, run_id=f"run-{arm}",
                                               label=int(rng.random() < rate), cost=COSTS[arm],
                                               billed=arm != "fd_claude"))
    return rows


def test_envelope_stops_at_the_first_failed_level():
    """Levels 1 and 2 pass both bars; at level 3 roko_full's VS rate collapses, so R_3's lower bound falls below
    X = 0.90 and the chain stops: E* = 2. At level 4 fd_claude collapses instead, so the cumulative R_4 clears X
    again, but a fixed sequence never resumes after a failure."""
    records = records_for(CHAIN_CASE)
    found = envelope.envelope(records, b=B)
    assert found.e_star == 2 and [level.claimed for level in found.levels] == [True, True, False, False, False]
    first, second, third, fourth = found.levels[:4]
    assert first.r_low >= envelope.X and first.c_high <= envelope.Y and not first.why
    assert third.r_low < envelope.X and third.why.startswith("R's lower bound")
    assert fourth.r_low >= envelope.X and fourth.c_high <= envelope.Y
    assert fourth.why == "the chain stopped at an earlier level"
    assert (first.n_tasks, first.n_runs) == (20, 120) and found.levels[-1].n_tasks == 100
    # The p-values Holm reads agree with the bounds: a claimed level's p is at most alpha, and the failed one's not;
    # the chain's p never falls, and E* at the envelope's own alpha is E*.
    for level in found.levels:
        assert level.p_level == max(level.p_r, level.p_c)
    assert first.p_chain <= 0.05 and second.p_chain <= 0.05 and third.p_level > 0.05
    assert [level.p_chain for level in found.levels] == holm.chain([level.p_level for level in found.levels])
    assert found.e_star_at(0.05) == 2 and found.e_star_at(1e-9) == 0
    # The same records, analysed twice, give the same envelope (the bootstrap is seeded).
    assert envelope.envelope(records, b=B) == found


def test_envelope_cannot_claim_a_level_with_an_unknown_cost_or_no_vs():
    records = records_for(CHAIN_CASE)
    for record in records:
        if record["arm"] == "roko_full" and record["task"]["instance_id"] == "F2-l2-0003" and record["seed"] == 2:
            record["costs"]["api_equiv_usd"] = None
    found = envelope.envelope(records, b=B)
    assert found.e_star == 1 and found.levels[1].why == "a run's cost is unknown, so C is unmeasurable"
    assert found.levels[1].p_level == 1.0 and found.levels[1].r is None
    never = {"roko_full": {1: 0.0}, "fd_claude": {1: 0.9}}
    level_one = envelope.envelope(records_for(never), b=B).levels[0]
    assert not level_one.claimed and "roko_full has no VS" in level_one.why
    # Every run of both arms passes at level 1: R is 1 in every replicate and BCa's jackknife is flat, so the
    # (then exact) percentile interval stands in, and the level is claimed.
    perfect = envelope.envelope(records_for({"roko_full": {1: 1.0}, "fd_claude": {1: 1.0}}), b=B).levels[0]
    assert perfect.claimed and (perfect.r_low, perfect.r, perfect.r_high) == (1.0, 1.0, 1.0)


def test_envelope_metrics_carry_the_names_f3_and_t7_read():
    records = records_for(CHAIN_CASE, tasks_per_family=6)
    found = envelope.envelope(records, b=B)
    alone = envelope.level_ratios(records, b=B)
    assert [level.level for level in alone] == [1, 2, 3, 4, 5] and alone[3].r > 1  # level 4 alone: roko_full wins
    produced = envelope.envelope_metrics(found, records, alone=alone)
    names = [metric.metric for metric in produced]
    assert names.count("envelope_ratio_r") == names.count("envelope_ratio_c") == 5
    assert names.count("envelope_ratio_r_level") == names.count("envelope_ratio_c_level") == 5
    assert names[-1] == "envelope_level" and produced[-1].value == float(found.e_star) and produced[-1].ladder is None
    for metric in produced:
        row = report.metric_record(metric, experiment_id="LOG1", snapshot="prices-2026-09-28", analysis_commit="abc",
                                   computed_at="now")
        assert validate.validate("metric-record", row) == [], row
        assert row["arms"] == ["fd_claude", "roko_full"]
        if metric.metric.startswith("envelope_ratio"):
            assert row["ci_method"] == "paired_stratified_bootstrap_bca" and row["ladder"] == metric.ladder
            assert (row["cost_basis"] == "api_equiv_usd") == metric.metric.startswith("envelope_ratio_c")
    cumulative = next(metric for metric in produced if metric.metric == "envelope_ratio_r" and metric.ladder == 3)
    assert 'task.ladder in [1, 2, 3]' in cumulative.cut.filter
    table = envelope.level_table(records, "PILOT-T")
    assert {(row["cell"], row["level"]) for row in table} == {(arm, level) for arm in CHAIN_CASE
                                                              for level in range(1, 6)}
    assert all(set(row) == {"cell", "level", "pass_hat_3", "usd_per_vs", "vs_rate"} for row in table)


# --- graphical Holm -------------------------------------------------------------------------------------------------


def test_holm_mapping_helpers_follow_s09():
    assert holm.directional(0.02) == 0.04 and holm.directional(0.7) == 1.0
    assert holm.iut(0.01, 0.03) == 0.03 and holm.bonferroni(0.01, 0.2) == 0.02 and holm.bonferroni(0.6, 0.9) == 1.0
    assert holm.gated(0.001, passed=False) == 1.0 and holm.gated(0.001, passed=True) == 0.001
    assert holm.chain([0.001, 0.004, 0.02, 0.003, 0.5]) == [0.001, 0.004, 0.02, 0.02, 0.5]
    # H4's lock entry: p = 2*min(p_a, p_b), each p_policy = max(p_ni, p_cost, 2*min(p_cost_b1, p_lat_b1)).
    policy_a = holm.iut(0.004, 0.01, holm.bonferroni(0.02, 0.3))
    policy_b = holm.iut(0.2, 0.001, holm.bonferroni(0.5, 0.6))
    assert holm.bonferroni(policy_a, policy_b) == pytest.approx(0.08)
    with pytest.raises(ValueError):
        holm.directional(1.5)


def test_holm_graph_is_the_one_the_lock_records():
    graph = holm.s09_graph()
    assert sum(graph.weights.values()) == 1 and graph.weights["H1:l1"] == Fraction(1, 7)
    assert all(graph.weights[node] == 0 for node in holm.H1_LEVELS[1:])
    assert all(sum(targets.values()) == 1 for targets in graph.edges.values())
    assert graph.edges["H1:l3"] == {"H1:l4": 1} and graph.edges["H1:l5"] == dict.fromkeys(holm.PRIMARIES[1:],
                                                                                           Fraction(1, 6))
    assert graph.edges["H2"] == dict.fromkeys(["H1:l1", "H3", "H4", "H5", "H6", "H7"], Fraction(1, 6))
    assert holm.MULTIPLICITY == {  # S09 §5's multiplicity block, verbatim
        "method": "graphical_holm", "weights": "1/7 each", "recycle": "equal", "directional_p": "2*one_sided",
        "conjunction": "iut_max", "disjunction": "bonferroni_2min", "failed_gate_p": 1,
        "h1_chain": ["l1", "l2", "l3", "l4", "l5"], "h1_release_after": "l5"}


def test_holm_reduces_to_classic_holm_when_h1s_levels_share_one_p():
    """With every H1 level at one p, the chain acts as one hypothesis, and the graph is Holm's step-down over seven
    primaries: the adjusted p of the i-th smallest is max_{j <= i} min(1, (7 - j + 1)·p_(j))."""
    others = {"H2": 0.04, "H3": 0.003, "H4": 0.2, "H5": 0.012, "H6": 0.008, "H7": 0.9}
    result = holm.decide(holm.node_p([0.006] * 5, others))
    ordered = sorted([("H1", 0.006), *others.items()], key=lambda item: item[1])
    expected, running = {}, 0.0
    for rank, (name, p) in enumerate(ordered):
        running = max(running, min(1.0, (7 - rank) * p))
        expected[name] = running
    assert {name: result.adjusted["H1:l1" if name == "H1" else name] for name in expected} == pytest.approx(expected)
    assert all(result.adjusted[node] == result.adjusted["H1:l1"] for node in holm.H1_LEVELS)
    assert result.supported == {name: expected[name] <= 0.05 for name in holm.PRIMARIES}


def test_holm_recycles_h1s_weight_only_after_level_5_and_feeds_its_chain():
    others = {"H2": 0.008, "H3": 0.5, "H4": 0.5, "H5": 0.5, "H6": 0.5, "H7": 0.5}
    # H1 claims all five levels at 0.05/7, then its 1/7 splits over H2-H7: H2's 1/6 now covers p = 0.008 > 0.05/7.
    full = holm.decide(holm.node_p([0.001] * 5, others))
    assert full.e_star == 5 and full.supported["H2"] and full.adjusted["H2"] == pytest.approx(0.048)
    # A chain that stops at level 3 keeps H1's weight, so H2 is not rejected.
    stopped = holm.decide(holm.node_p([0.001, 0.002, 0.3, 0.001, 0.001], others))
    assert stopped.e_star == 2 and stopped.supported["H1"] and not stopped.supported["H2"]
    # Weight the others release flows to H1's next unclaimed level: with H2-H7 all rejected, level 3 is tested at
    # the whole alpha, and p = 0.03 is claimed, then levels 4 and 5.
    fed = holm.decide(holm.node_p([0.001, 0.002, 0.03, 0.001, 0.001], dict.fromkeys(others, 0.0001)))
    assert fed.e_star == 5 and all(fed.supported.values())
    # A failed gate sets p = 1, so H6 is never rejected, however small its test's p was.
    gated = holm.decide(holm.node_p([0.001] * 5, {**dict.fromkeys(others, 0.0001), "H6": holm.gated(1e-6, False)}))
    assert not gated.supported["H6"] and gated.adjusted["H6"] == 1.0 and gated.e_star == 5
    with pytest.raises(ValueError):
        holm.node_p([0.01] * 4, others)
