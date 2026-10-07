"""Tests for the audit lottery, the false-green estimators, the replay and the vs.label row (S05 §4.2, §4.5, §5, §7.1).

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_estimate.py
"""

from __future__ import annotations

import copy
import hashlib
import hmac
import json
import math
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

import pytest

VB_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(VB_ROOT))
import audit  # noqa: E402
import validate  # noqa: E402  (schema/validate.py, on sys.path through audit)
from audit import estimate, lottery, replay  # noqa: E402
from common.hmac_seed import Stream  # noqa: E402  (families/common, on sys.path through audit)

RUNS = 1000
# An S05 window of 200 green units, as (M3 risk r, units, false greens): each group is exactly r false green, so the
# census rate is known, θ = 26/200. The units are fixed; only the lotteries vary.
WINDOW = ((0.05, 80, 4), (0.10, 60, 6), (0.20, 40, 8), (0.40, 20, 8))
THETA = 26 / 200
GROUP_SHARES = tuple((risk, units / 200) for risk, units, _ in WINDOW)  # a stationary stream of the same mix


def window_units() -> list[replay.Unit]:
    units = []
    for risk, count, false_greens in WINDOW:
        for k in range(count):
            i = len(units)
            units.append(replay.Unit(f"run-w:plan:T{i:03d}:1", "run-w", f"T{i:03d}", f"{i:040x}",
                                     int(k < false_greens), "passed", False, risk))
    return units


def census_row(index: int = 0, *, false_green: bool = False, **changes) -> dict:
    """A valid census vs.label row (S05 §5's example plus its verdict); a false green fails 3 hidden tests."""
    row = {"ev": "vs.label", "run_id": "run-1", "attempt_key": f"run-1:plan:F4-l3-{index:04d}:1",
           "task_id": f"F4-l3-{index:04d}", "seed": 2, "arm": "roko_full", "prediction_id": None,
           "vs_source": "census", "pi": 1.0, "verdict": "passed", "final_commit": f"{index:040x}", "completion": True,
           "visible_clean": True, "hidden": {"suite": "truth:F4@v1", "n": 14, "failed": 3 if false_green else 0},
           "integrity": {"g": 0, "findings": []}, "honeypot": None, "vs": int(not false_green),
           "vs_lenient": int(not false_green), "unknown": False,
           "cost_usd": {"impl": 0.041, "escalation": 0.0, "spec_refine": 0.0, "audit": 0.004}, "label_rule": "vs-1"}
    row.update(changes)
    return row


# --- the replay: SC1 --------------------------------------------------------------------------------------------


def test_wilson_interval_meets_coverage_in_every_cell():
    """1,000 keyed lotteries over the window at each ρ, uniform and tilted: coverage ≥ 0.93, |bias| ≤ 0.01, HT
    unbiased, π ≥ the floor, in every cell (gap-a499aa).

    Uniform selection is S05's first slice (no M3); tilted selection uses M3's risk at λ_max, and ρ = 0.15 tilted is
    SC1's primary cell. The interval is Wilson's at Kish n_eff in every cell. Under S05 §4.5's first rule, Wald on v̂
    once n_eff ≥ 30 with 5 events, the tilted ρ = 0.30 cell (n_eff about 45) covered 0.911 on this window and missed
    SC1's 0.93; Wilson covers it at 0.994. The uniform cells cover 0.972 to 0.988, so the target is no coin flip;
    HT's bias is checked against its own Monte Carlo error.
    """
    units = window_units()
    uniform = replay.replay(units, rhos=replay.RHOS, runs=RUNS, seed="test-uniform")
    tilted = replay.replay(units, rhos=replay.RHOS, runs=RUNS, seed="test-tilted", lam=lottery.LAMBDA_MAX)
    assert uniform["theta_census"] == tilted["theta_census"] == THETA
    for report in (uniform, tilted):
        assert [cell["rho"] for cell in report["cells"]] == [0.10, 0.15, 0.30]
        for cell in report["cells"]:
            where = f"{report['selection']} rho={cell['rho']}: {cell}"
            assert cell["coverage"] >= replay.TARGET_COVERAGE, where
            assert abs(cell["bias_hajek"]) <= replay.TARGET_BIAS, where
            assert abs(cell["bias_ht"]) <= 4 * cell["ht_mc_se"], where  # unbiased within Monte Carlo error
            assert cell["pi_min"] >= lottery.EPS_FLOOR, where
            assert cell["targets_met"], where
            assert cell["empty"] == 0 and cell["mean_n_eff"] > 0, where
        text = replay.format_report(report)
        assert "theta_census = 0.1300" in text and "n_eff" in text and text.count(" met") == 3
        assert "NOT MET" not in text
    assert uniform["cells"][0]["mean_audited"] == pytest.approx(0.10 * 200, rel=0.05)
    primary = tilted["cells"][1]
    assert primary["mean_n_eff"] < uniform["cells"][1]["mean_n_eff"]  # the tilt costs effective sample size
    assert tilted["cells"][2]["mean_n_eff"] > 30  # past the first rule's n_eff ≥ 30: the cell Wald used to decide


def test_betting_sequence_holds_at_every_stopping_time():
    """On stationary streams (each unit a false green with probability θ = 0.13 given the past), the sequence covers θ
    at every one of 200 times in at least 95% of 1,000 runs, at each ρ, uniform and tilted."""
    secret = hashlib.sha256(b"test-betting").digest()
    for lam in (0.0, lottery.LAMBDA_MAX):
        for rho in replay.RHOS:
            pis = {risk: lottery.inclusion_probability(rho, risk=risk, mean_risk=THETA, lam=lam)
                   for risk, _ in GROUP_SHARES}
            held = 0
            for run in range(RUNS):
                stream = Stream(hashlib.sha256(f"stream/{lam}/{rho}/{run}".encode()).digest())
                key = lottery.run_key(secret, f"run-{lam}-{rho}-{run}")
                z = []
                for i in range(200):
                    risk = _group(stream.random())
                    y = int(stream.random() < risk)
                    x = lottery.draw(key, "run", f"T{i}", f"run:T{i}:1", f"c{i}")
                    z.append(estimate.audit_z(lottery.is_selected(x, pis[risk]), y, pis[risk]))
                held += estimate.sequence_holds(z, THETA)
            assert held / RUNS >= 0.95, (lam, rho, held)


def _group(u: float) -> float:
    for risk, share in GROUP_SHARES:
        if u < share:
            return risk
        u -= share
    return GROUP_SHARES[-1][0]


def test_betting_cs_contains_the_exact_sequence_and_narrows():
    secret = hashlib.sha256(b"test-cs").digest()
    key = lottery.run_key(secret, "run-cs")
    z = []
    for i in range(150):
        y = int(i % 8 in (0, 1, 3, 5, 6))  # 5 in 8 false greens
        x = lottery.draw(key, "run-cs", f"T{i}", "1", f"c{i}")
        z.append(estimate.audit_z(lottery.is_selected(x, 0.5), y, 0.5))
    z_max = lottery.EPS_FLOOR / 0.5
    sequence = estimate.betting_cs(z, z_max=z_max, grid=200)
    assert len(sequence) == len(z) and None not in sequence
    for earlier, later in zip(sequence, sequence[1:]):  # a running intersection only narrows
        assert earlier[0] <= later[0] and later[1] <= earlier[1]
    assert sequence[-1][0] > 0.0 and sequence[-1][1] < 1.0
    for theta in (0.0, 0.2, 0.5, 0.625, 0.8, 1.0):  # a θ the exact process keeps is inside every interval
        if estimate.sequence_holds(z, theta, z_max=z_max):
            assert all(low <= theta <= high for low, high in sequence)
    assert estimate.sequence_holds(z, 0.625, z_max=z_max)
    assert not estimate.sequence_holds(z, 0.0, z_max=z_max)
    with pytest.raises(ValueError, match="outside"):
        estimate.betting_cs([0.5], z_max=0.1)
    with pytest.raises(ValueError, match="z_max"):
        estimate.betting_cs([0.0], z_max=0.01)
    assert estimate.hedged_capital([], 0.1) == []


# --- the lottery ------------------------------------------------------------------------------------------------


def test_pi_never_drops_below_the_floor():
    for rho in (0.05, 0.10, 0.15, 0.30, 0.5):
        for lam in (0.0, 0.3, 0.8, 1.0):
            for alpha in (0.5, 1.0, 2.0):
                for risk in (None, 0.0, 1e-12, 0.01, 0.2, 1.0):
                    for mean_risk in (None, 0.0, 1e-9, 0.01, 0.5, 1.0):
                        for pi_max in (lottery.EPS_FLOOR, 0.6, 1.0):
                            pi = lottery.inclusion_probability(rho, risk=risk, mean_risk=mean_risk, lam=lam,
                                                               alpha=alpha, pi_max=pi_max)
                            assert lottery.EPS_FLOOR <= pi <= pi_max
    assert lottery.inclusion_probability() == lottery.DEFAULT_RHO == 0.10  # real work (decision 7103)
    assert lottery.inclusion_probability(0.15, risk=0.0, mean_risk=0.1, lam=1.0) == lottery.EPS_FLOOR
    for bad in ({"rho": 0.04}, {"rho": 0.6}, {"rho": 0.15, "pi_max": 0.04}, {"rho": 0.15, "lam": 1.5},
                {"rho": 0.15, "risk": 1.2, "mean_risk": 0.1, "lam": 0.5}, {"rho": 0.15, "alpha": 0.0}):
        with pytest.raises(ValueError):
            lottery.inclusion_probability(**bad)
    with pytest.raises(ValueError, match="floor"):
        lottery.select(bytes(32), "r", "t", "a", "c", 0.01)


def test_tilt_waits_for_fifty_calibrated_labels():
    assert lottery.tilt(None, 500) == 0.0
    assert lottery.tilt(0.0, lottery.MIN_TILT_LABELS - 1) == 0.0
    assert lottery.tilt(0.0, lottery.MIN_TILT_LABELS) == lottery.LAMBDA_MAX
    assert lottery.tilt(0.05, 100) == pytest.approx(lottery.LAMBDA_MAX / 2)
    assert lottery.tilt(lottery.ECE_REF, 100) == lottery.tilt(0.4, 100) == 0.0  # miscalibrated: uniform
    with pytest.raises(ValueError):
        lottery.tilt(1.5, 100)


def test_revealed_key_reproduces_every_draw_and_its_commitment():
    secret = bytes(range(32, 64))
    run_id = "run-reveal"
    key = lottery.run_key(secret, run_id)
    assert key == lottery.run_key(secret, run_id) != lottery.run_key(secret, "run-other")
    committed = lottery.commitment(key, run_id)  # logged at run start; the key itself only at run end
    assert committed == "sha256:" + hashlib.sha256(key + run_id.encode()).hexdigest()
    pis = (lottery.EPS_FLOOR, 0.10, 0.15, 0.30, 1.0)
    ledger = [lottery.select(key, run_id, f"T{i}", f"{run_id}:plan:T{i}:1", f"tree-{i}", pis[i % 5]).record()
              for i in range(100)]
    assert 0 < sum(row["selected"] for row in ledger) < 100
    assert all(row["selected"] for row in ledger if row["pi"] == 1.0)  # census strata are always audited
    assert lottery.verify_reveal(key, run_id, committed, ledger) == []
    assert lottery.verify_reveal(lottery.run_key(secret, "run-other"), run_id, committed, ledger)[0].startswith(
        "commitment")
    for field, value, fragment in (("selected", None, "selected"), ("prf_u", "0x0000000000000000", "prf_u"),
                                   ("accepted_commit", "tree-forged", "prf_u"), ("pi", 0.01, "floor"),
                                   ("run_id", "run-other", "run_id"), ("task_id", None, "cannot recompute")):
        forged = copy.deepcopy(ledger)
        row = forged[7]
        row[field] = (not row["selected"]) if value is None and field == "selected" else value
        problems = lottery.verify_reveal(key, run_id, committed, forged)
        assert problems and all("selection 7" in problem for problem in problems), (field, problems)
        assert any(fragment in problem for problem in problems), (field, problems)


def test_draw_is_hmac_over_hmac_seeds_length_prefixed_fields():
    key = bytes(range(1, 33))
    fields = (lottery.DRAW_TAG, "run", "tâche", "attempt", "commit")
    message = b"".join(len(data).to_bytes(4, "big") + data for data in (field.encode() for field in fields))
    x = int.from_bytes(hmac.digest(key, message, "sha256")[:8], "big")
    assert lottery.draw(key, *fields[1:]) == x
    assert lottery.prf_hex(x) == f"0x{x:016x}" and len(lottery.prf_hex(0)) == 18
    assert lottery.u_value(x) == x / 2 ** 64
    assert lottery.draw(key, "ab", "c", "", "") != lottery.draw(key, "a", "bc", "", "")  # the prefixes separate fields
    secret = bytes(32)
    run_key = lottery.run_key(secret, "r")
    keyed = b"".join(len(data).to_bytes(4, "big") + data for data in (lottery.KEY_TAG.encode(), b"r"))
    assert run_key == hmac.digest(secret, keyed, "sha256")
    with pytest.raises(ValueError):
        lottery.run_key(bytes(16), "r")
    with pytest.raises(ValueError):
        lottery.draw(bytes(31), "r", "t", "a", "c")


def test_selection_is_an_exact_comparison_of_the_draw_with_pi():
    for pi in (lottery.EPS_FLOOR, 0.15, 0.3, 0.5):
        edge = int(pi * 2 ** 64)
        assert Fraction(edge, 2 ** 64) == Fraction(pi)  # π·2^64 is an integer for every π above the floor
        assert lottery.is_selected(edge - 1, pi) and not lottery.is_selected(edge, pi)
    assert lottery.is_selected(0, lottery.EPS_FLOOR)
    assert lottery.is_selected(2 ** 64 - 1, 1.0)  # π = 1 always selects, even where x / 2^64 rounds up to 1.0
    assert lottery.u_value(2 ** 64 - 1) == 1.0
    for bad in (-1, 2 ** 64, True, 1.5):
        with pytest.raises(ValueError):
            lottery.is_selected(bad, 0.5)
    for pi in (0.0, 1.1, float("nan")):
        with pytest.raises(ValueError):
            lottery.is_selected(1, pi)


# --- the estimators ---------------------------------------------------------------------------------------------


def test_estimators_match_a_hand_computation():
    audited = [(0.5, 1), (0.5, 0), (0.25, 1)]  # w = 2, 2, 4: N̂ = 8, Σ w·Y = 6
    est = estimate.estimate(audited, 10)
    assert (est.n_green, est.n_audited, est.events) == (10, 3, 2)
    assert est.n_hat == 8.0 and est.theta_ht == 0.6 and est.theta_hajek == 0.75
    # v̂ = (0.5·4·0.0625 + 0.5·4·0.5625 + 0.75·16·0.0625) / 64 = 2 / 64
    assert est.variance == pytest.approx(2 / 64, abs=1e-15)
    assert est.n_eff == pytest.approx(64 / 24, abs=1e-15)
    assert est.ci_method == "wilson_eff" and est.ci == estimate.wilson(0.75, 64 / 24)
    record = est.record()
    assert record["ci"] == list(est.ci) and json.loads(json.dumps(record)) == record


def test_the_interval_is_wilson_at_n_eff_however_large_the_sample():
    """gap-a499aa: no Wald branch. The samples S05 §4.5's first rule gave to Wald (n_eff ≥ 30, 5 events each way)
    get Wilson at n_eff like every other; v̂ is still reported."""
    large = estimate.estimate([(0.3, 1)] * 12 + [(0.3, 0)] * 48, 200)  # N̂ = 200, n_eff = 60, 12 events
    assert large.theta_hajek == pytest.approx(0.2) and large.n_eff == pytest.approx(60)
    assert large.ci_method == estimate.CI_METHOD == "wilson_eff" and large.ci == estimate.wilson(0.2, large.n_eff)
    # v̂ = 0.7 · (1/0.09) · (12·0.64 + 48·0.04) / 200²
    assert large.variance == pytest.approx(0.7 / 0.09 * 9.6 / 200 ** 2, abs=1e-15)
    for units, n_green in (([(0.3, 1)] * 4 + [(0.3, 0)] * 56, 200), ([(0.3, 1)] * 56 + [(0.3, 0)] * 4, 200),
                           ([(0.3, 1)] * 6 + [(0.3, 0)] * 23, 200), ([(1.0, 1)] * 6 + [(1.0, 0)] * 34, 40)):
        est = estimate.estimate(units, n_green)
        assert est.ci_method == "wilson_eff" and est.ci == estimate.wilson(est.theta_hajek, est.n_eff), units
    census = estimate.estimate([(1.0, 1)] * 6 + [(1.0, 0)] * 34, 40)  # π = 1: n_eff is the window's 40 units
    assert census.n_eff == 40 and census.theta_ht == census.theta_hajek == 0.15
    assert census.ci[0] < 0.15 < census.ci[1]


def test_wilson_matches_published_values_and_handles_the_ends():
    # 2 of 10 at 95%: the Wilson score interval is 0.0567 to 0.5098
    low, high = estimate.wilson(0.2, 10)
    assert (round(low, 4), round(high, 4)) == (0.0567, 0.5098)
    assert estimate.wilson(0.0, 6)[0] == 0.0 and estimate.wilson(1.0, 6)[1] == 1.0
    assert estimate.wilson(0.3, 0) == (0.0, 1.0)
    assert estimate.z_value(0.05) == estimate.Z95
    assert estimate.z_value(0.10) == pytest.approx(1.6448536269514722)
    with pytest.raises(ValueError):
        estimate.wilson(1.2, 5)


def test_empty_samples_and_null_labels():
    empty = estimate.estimate([], 50)
    assert (empty.theta_ht, empty.theta_hajek, empty.variance, empty.ci) == (0.0, None, None, (0.0, 1.0))
    low, high = estimate.null_bounds([(0.15, 1), (0.15, None), (0.3, 0), (0.05, None), (1.0, 0)], 80)
    assert low.events == 1 and high.events == 3
    assert low.theta_hajek < high.theta_hajek
    assert low.theta_hajek == pytest.approx((1 / 0.15) / (2 / 0.15 + 1 / 0.3 + 1 / 0.05 + 1))
    for bad_units, n_green in (([(0.0, 1)], 5), ([(1.2, 0)], 5), ([(0.5, 2)], 5), ([(0.5, None)], 5),
                               ([(0.5, True)], 5), ([(0.5, 1)] * 3, 2), ([], 0)):
        with pytest.raises(ValueError):
            estimate.estimate(bad_units, n_green)
    with pytest.raises(ValueError):
        estimate.audit_z(True, None, 0.5)
    assert estimate.audit_z(False, None, 0.5) == 0.0 and estimate.audit_z(True, 1, 0.05) == 1.0


def test_the_rust_fixture_matches_the_reference():
    """`fixtures/estimators.json` is what roko-gate's audit module must reproduce (7114): it must equal the code's."""
    stored = json.loads((audit.FIXTURES_DIR / "estimators.json").read_text(encoding="utf-8"))
    _assert_close(stored, replay.build_fixture(), "$")
    assert stored["version"] == replay.FIXTURE_VERSION
    methods = {case["expected"]["ci_method"] for case in stored["estimates"]}
    assert methods == {"wilson_eff"} and stored["constants"]["ci_method"] == "wilson_eff"  # gap-a499aa: no Wald
    assert any(case["cs"][-1] != [0.0, 1.0] for case in stored["betting"])  # some sequence narrows


def _assert_close(stored, computed, path: str) -> None:
    if isinstance(stored, float) or isinstance(computed, float):
        assert isinstance(stored, (int, float)) and isinstance(computed, (int, float)), path
        assert math.isclose(stored, computed, rel_tol=1e-12, abs_tol=1e-12), (path, stored, computed)
    elif isinstance(stored, dict):
        assert isinstance(computed, dict) and stored.keys() == computed.keys(), path
        for name in stored:
            _assert_close(stored[name], computed[name], f"{path}.{name}")
    elif isinstance(stored, list):
        computed = list(computed) if isinstance(computed, tuple) else computed
        assert isinstance(computed, list) and len(stored) == len(computed), path
        for index, (left, right) in enumerate(zip(stored, computed)):
            _assert_close(left, right, f"{path}[{index}]")
    else:
        assert stored == computed and type(stored) is type(computed), (path, stored, computed)


# --- the vs.label row -------------------------------------------------------------------------------------------


def test_the_schema_uses_only_what_validate_implements():
    schema = json.loads(audit.LABEL_SCHEMA.read_text(encoding="utf-8"))
    validate.check_schema(schema)
    assert schema["additionalProperties"] is False
    s05 = ("attempt_key", "run_id", "task_id", "seed", "arm", "prediction_id", "vs_source", "pi", "final_commit",
           "completion", "visible_clean", "hidden", "integrity", "honeypot", "vs", "vs_lenient", "unknown", "cost_usd",
           "label_rule")
    assert set(s05) | {"ev", "verdict"} == set(schema["required"])
    assert schema["properties"]["verdict"]["enum"][:-1] == list(audit.GREEN_VERDICTS)


VALID = [
    ("S05's example row", {}),
    ("a false green", {"hidden": {"suite": "truth:F4@v1", "n": 14, "failed": 3}, "vs": 0, "vs_lenient": 0}),
    ("gamed but correct: lenient VS only", {"integrity": {"g": 1, "findings": [{"kind": "test_edit", "file": "t.py"}]},
                                           "vs": 0}),
    ("an unknown label", {"hidden": None, "vs": 0, "vs_lenient": 0, "unknown": True}),
    ("an F8 honeypot", {"honeypot": {"spec_correct": True, "conflict_flagged": True}}),
    ("an attempt with no pass", {"verdict": None, "completion": False, "visible_clean": False, "vs": 0,
                                 "vs_lenient": 0}),
    ("an audit at pi_i x pi_B", {"vs_source": "audit", "pi": 0.025, "seed": None, "arm": "prod",
                                 "prediction_id": "m3-1", "verdict": "passed_with_preexisting_failures"}),
    ("gold, with battery columns", {"gold": True, "battery": {"a1": "pass", "a2": "pass", "b1": None}}),
    ("unknown costs", {"cost_usd": {"impl": None, "escalation": None, "spec_refine": None, "audit": None}}),
]

INVALID = [
    ("ev", "vs.labels", '$.ev: must be "vs.label"'),
    ("verdict", "satisfied", "$.verdict: \"satisfied\" is not one of"),
    ("verdict", "DELETE", "missing required field 'verdict'"),
    ("vs", True, "$.vs: true is not one of"),
    ("vs", 0, "$.vs: the checks give 1, not 0"),
    ("unknown", True, "$.unknown: the checks give false, not true"),
    ("pi", 0.5, "a census row labels every attempt, so pi is 1.0"),
    ("pi", "1.0", "$.pi: expected number, got string"),
    ("seed", 2.5, "$.seed: expected integer or null"),
    ("extra", 1, "$.extra: unexpected field"),
    ("cost_usd", {"impl": 0.1, "escalation": 0.0, "spec_refine": 0.0, "audit": -0.01}, "a cost cannot be below 0"),
    ("cost_usd", {"impl": 0.1, "escalation": 0.0, "spec_refine": 0.0, "audit": 0.0, "judge": 0.1},
     "$.cost_usd.judge: unexpected field"),
    ("label_rule", "vs-2", "is not a rule this package knows"),
    ("hidden", {"suite": "truth:F4@v1", "n": 14, "failed": 15}, "15 failed of 14 tests"),
    ("hidden", None, "$.unknown: the checks give true"),
    ("final_commit", None, "a completed attempt left a final commit"),
    ("integrity", {"g": 1, "findings": []}, "g is 1, so it names at least one finding"),
    ("integrity", {"g": 0, "findings": [{"kind": "test_edit"}]}, "any finding makes g 1"),
    ("integrity", {"g": 0, "findings": [{"file": "t.py"}]}, "missing required field 'kind'"),
    ("honeypot", {"spec_correct": True, "conflict_flagged": False}, "$.vs: the checks give 0, not 1"),
]


@pytest.mark.parametrize(("name", "changes"), VALID, ids=[name for name, _ in VALID])
def test_valid_vs_label_rows(name, changes):
    assert audit.label_errors(census_row(**changes)) == []


@pytest.mark.parametrize(("field", "value", "expected"), INVALID,
                         ids=[f"{field}={value!r}" for field, value, _ in INVALID])
def test_vs_label_mutants_fail(field, value, expected):
    row = census_row()
    if value == "DELETE":
        del row[field]
    else:
        row[field] = value
    errors = audit.label_errors(row)
    assert any(expected in error for error in errors), errors


def test_an_audit_row_needs_a_green_verdict_and_false_green_reads_unknown_both_ways():
    errors = audit.label_errors(census_row(vs_source="audit", pi=0.15, verdict=None))
    assert any("an audit labels a green unit" in error for error in errors)
    assert audit.label_errors(census_row(vs_source="audit", pi=0.0)) != []
    unknown = census_row(hidden=None, vs=0, vs_lenient=0, unknown=True)
    assert audit.false_green(unknown) == 1 and audit.false_green(unknown, unknown_vs=1) == 0
    assert audit.false_green(census_row(false_green=True)) == 1 and audit.false_green(census_row()) == 0


# --- the replay's input and command line ------------------------------------------------------------------------


def write_labels(path: Path, rows: list[dict]) -> Path:
    path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
    return path


def slice_rows() -> list[dict]:
    """A 60-unit first slice: 12 false greens, one unknown label, plus rows the replay must leave out."""
    rows = [census_row(i, false_green=i % 5 == 0) for i in range(59)]
    rows.append(census_row(59, hidden=None, vs=0, vs_lenient=0, unknown=True))
    rows.append(census_row(60, verdict=None, completion=False, visible_clean=False, vs=0, vs_lenient=0))
    rows.append(census_row(61, false_green=True, gold=True))
    rows.append(census_row(62, verdict="unverified"))
    return rows


def test_units_from_labels_keep_green_non_gold_census_rows(tmp_path):
    rows = audit.load_labels(write_labels(tmp_path / "labels.jsonl", slice_rows()))
    units = replay.units_from_labels(rows)
    assert len(units) == 61 and {unit.verdict for unit in units} == {"passed", "unverified"}
    report = replay.replay(units, runs=20)
    assert report["n_units"] == 60 and report["false_greens"] == 13  # 12 failed hidden suites, 1 unknown label
    assert report["theta_census"] == pytest.approx(13 / 60) and report["unknown_labels"] == 1
    assert report["theta_census_unknown_as_vs"] == pytest.approx(12 / 60)
    assert report["census_strata"]["unverified"] == {"units": 1, "false_greens": 0}
    with pytest.raises(replay.ReplayError, match="census labels"):
        replay.units_from_labels([census_row(vs_source="audit")])
    with pytest.raises(replay.ReplayError, match="risk"):
        replay.replay(units, runs=1, lam=0.5)


def test_load_labels_names_every_bad_line(tmp_path):
    rows = [census_row(1), census_row(2, vs=0), census_row(1)]
    path = write_labels(tmp_path / "labels.jsonl", rows)
    path.write_text(path.read_text() + "{not json\n\n", encoding="utf-8")
    with pytest.raises(audit.LabelError) as caught:
        audit.load_labels(path)
    message = str(caught.value)
    assert ":2: $.vs" in message and ":3: attempt_key" in message and "repeats line 1" in message
    assert ":4: not JSON" in message


def test_replay_cli_prints_a_row_per_rho_and_runs_as_a_script(tmp_path, capsys):
    labels = write_labels(tmp_path / "labels.jsonl", slice_rows())
    assert replay.main(["--labels", str(labels), "--runs", "40"]) == 0
    out = capsys.readouterr().out
    assert "theta_census = 0.2167" in out and "n_eff" in out
    assert [line.split()[0] for line in out.splitlines() if line.startswith(" 0.")] == ["0.10", "0.15", "0.30"]
    assert replay.main(["--labels", str(labels), "--runs", "10", "--rho", "0.2", "--json"]) == 0
    report = json.loads(capsys.readouterr().out)
    assert [cell["rho"] for cell in report["cells"]] == [0.2] and report["runs"] == 10
    risk = tmp_path / "risk.jsonl"
    risk.write_text("".join(json.dumps({"attempt_key": row["attempt_key"], "r": 0.3 if row["vs"] else 0.6}) + "\n"
                            for row in slice_rows()), encoding="utf-8")
    assert replay.main(["--labels", str(labels), "--runs", "10", "--risk", str(risk), "--lam", "0.8", "--json"]) == 0
    assert json.loads(capsys.readouterr().out)["selection"] == "tilted"
    bad = write_labels(tmp_path / "bad.jsonl", [census_row(1), census_row(1)])
    assert replay.main(["--labels", str(bad)]) == 2
    assert "repeats line 1" in capsys.readouterr().err
    assert replay.main(["--labels", str(labels), "--rho", "0.01"]) == 2
    script = subprocess.run([sys.executable, str(VB_ROOT / "audit" / "replay.py"), "--labels", str(labels), "--runs",
                             "5"], capture_output=True, text=True, cwd=tmp_path, check=False)
    assert script.returncode == 0, script.stderr
    assert "theta_census" in script.stdout
