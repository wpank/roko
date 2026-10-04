"""Tests for the closure replays X1-X4 and closure 4 (replay_closure.py, task 3358), over a LOG1-shaped fixture.
No model is called, and no real roko-learn evaluator or roko binary is needed: the h5/h6 replay documents and
the loop census are built by hand, the shapes those mechanisms write (the module docstring).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_closure.py -q
"""

from __future__ import annotations

import json
import random
import sys
from pathlib import Path

import pytest

_DRIVER_DIR = Path(__file__).resolve().parents[1] / "driver"
if str(_DRIVER_DIR) not in sys.path:  # campaign.py, for closure_4's census_report (replay_closure.py's own path)
    sys.path.insert(0, str(_DRIVER_DIR))

import campaign  # noqa: E402
import replay  # noqa: E402
import replay_closure  # noqa: E402
import replay_h6  # noqa: E402
import replay_runner  # noqa: E402
from test_analysis import run_record, write_run  # noqa: E402
from test_replay_h5 import EXPERIMENT, log1_tree  # noqa: E402


def write_h5(root: Path, results: Path) -> Path:
    doc = replay_runner.run(results, [EXPERIMENT], "h5", seed=5)
    return replay_runner.export(doc, root / "h5.json")


def write_h6(root: Path, results: Path, rows: list[dict]) -> Path:
    table = root / "h6_table.jsonl"
    table.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")
    doc = replay_runner.run(results, [EXPERIMENT], "h6", seed=1, params={"table": table})
    return replay_runner.export(doc, root / "h6.json")


def fake_census(live: list[str]) -> object:
    """A monkeypatch replacement for `campaign.census_report`, reporting every name in `live` LIVE."""
    def fake(repo=None, roko_bin=None):
        return {"schema": campaign.LOOPS_SCHEMA, "harness_sha": "cafef00d",
                "rows": [{"loop": name, "state": "live"} for name in live]}
    return fake


def test_closure_replays_emit_their_preregistered_estimands(tmp_path, monkeypatch):
    """X1 is evaluated once --h5 is given (a real delta_brier with a bootstrap CI); X2, X3 and X4 are not, each
    with its own reason (3357's H6 adapter is not wired yet, no block E stream to follow X3's step, no LOG1
    policy cells); closure 4 reports the three loops' LIVE status from a faked census."""
    results, _false_keys = log1_tree(tmp_path)
    h5_path = write_h5(tmp_path, results)
    h6_path = write_h6(tmp_path, results, [{"arm": "A0", "disturbance": "provider_fault",
                                             "iae": {"mean": 0.1, "low": 0.05, "high": 0.15, "n": 10},
                                             "changes": 0.0, "holds": 0.0, "note": None}])
    monkeypatch.setattr(campaign, "census_report", fake_census(["L-M1", "L-audit"]))  # L-route-trust missing

    first = replay_runner.run(results, [EXPERIMENT], "closure", seed=9, params={"h5": h5_path, "h6": h6_path})
    again = replay_runner.run(results, [EXPERIMENT], "closure", seed=9, params={"h5": h5_path, "h6": h6_path})
    assert replay_runner.canonical(first) == replay_runner.canonical(again)
    found = first["estimates"]

    x1 = found["x1"]
    assert x1["evaluated"] and x1["n_units"] == 200 and x1["theta_census"] == 0.15
    low, high = x1["ci95"]
    assert low <= x1["delta_brier"] <= high
    assert abs(x1["delta_brier"]) < 0.05  # the uniform lottery's hajek estimate is close to theta_census here

    x2 = found["x2"]
    assert x2["n_evaluated"] == 0
    for kind, cell in x2["kinds"].items():
        assert not cell["evaluated"] and "A3, A4, A3-mis not evaluated" in cell["reason"]

    assert not found["x3"]["evaluated"] and "block E" in found["x3"]["reason"]
    assert not found["x4"]["evaluated"] and "policies" in found["x4"]["reason"]

    closure = found["closure_4"]
    assert closure["harness_sha"] == "cafef00d"
    assert closure["loops"] == {"L-M1": True, "L-audit": True, "L-route-trust": False}
    assert closure["all_live"] is False


def test_x1_and_x2_are_not_evaluated_without_their_replay_documents(tmp_path, monkeypatch):
    results, _ = log1_tree(tmp_path)
    monkeypatch.setattr(campaign, "census_report", fake_census(list(replay_closure.CLOSURE_4_LOOPS)))
    found = replay_runner.run(results, [EXPERIMENT], "closure", seed=1)["estimates"]
    assert not found["x1"]["evaluated"] and "--h5" in found["x1"]["reason"]
    assert not found["x2"]["evaluated"] and "--h6" in found["x2"]["reason"]
    assert found["closure_4"]["all_live"] is True


def step_tree(root: Path, before: int = 60, after: int = 120) -> Path:
    """Block A and E-shaped records for X3's false-green step: roko_fixed on `before` p1_core tasks, every one a true
    green, then on `after` F8 honeypots (stream log1_f8_honeypots), every one green and every other a false green."""
    results = root / "results"
    blocks = (("roko-a", "p1_core", "F1", before), ("roko-e", "log1_f8_honeypots", "F8", after))
    for run_id, stream, family, count in blocks:
        records = []
        for position in range(1, count + 1):
            instance = f"{family}-l{position % 5 + 1}-{position:04d}"
            false_green = stream == "log1_f8_honeypots" and position % 2 == 0
            record = run_record(instance, 1, run_id=run_id, arm="roko_fixed", label=int(not false_green),
                                visible=True, verdict="passed", honeypot=stream == "log1_f8_honeypots")
            record["stream"].update(id=stream, position=position)
            records.append(record)
        write_run(results / EXPERIMENT, records)
    return results


def test_x3_evaluates_from_a_false_green_step_replay(tmp_path, monkeypatch):
    """X3 replays one draw order per lottery at S05's 5% floor and at M1's 2x audit boost: after the step the
    boost confirms S06's E3 breach sooner, with a CI excluding 0, while its audits stay within SC6's 12%."""
    assert replay_closure.beta_tail(0.10, 1, 9) == pytest.approx(0.9 ** 9)
    results = step_tree(tmp_path)
    monkeypatch.setattr(campaign, "census_report", fake_census(list(replay_closure.CLOSURE_4_LOOPS)))
    first = replay_runner.run(results, [EXPERIMENT], "closure", seed=3, params={"reps": 200})
    again = replay_runner.run(results, [EXPERIMENT], "closure", seed=3, params={"reps": 200})
    assert replay_runner.canonical(first) == replay_runner.canonical(again)

    x3 = first["estimates"]["x3"]
    assert x3["evaluated"], x3
    assert (x3["step"], x3["n_units"], x3["post_step_units"], x3["lotteries"]) == (40, 160, 120, 200)
    assert x3["theta_post"] == 0.5 and x3["rates"] == {"floor": 0.05, "boost": 0.10}
    assert x3["median_delay"]["boost"] < x3["median_delay"]["floor"]
    low, high = x3["ci95"]
    assert 0 < low <= x3["delta_median_delay"] <= high
    assert x3["audit_share"]["boost"] <= replay_closure.X3_SHARE_MAX and x3["guard_passes"]
    assert x3["pays"] and x3["rule"] == "ci95_excludes_0"

    # With fewer block A units than the step needs, X3 is not evaluated, and says why.
    short = step_tree(tmp_path / "short", before=30)
    matrix = replay_runner.matrix(replay.load(short, [EXPERIMENT]))
    found = replay_closure.x3_from_matrix(matrix, random.Random(1), 5)
    assert not found["evaluated"] and "30 green block A" in found["reason"]


def test_delta_iae_x2_evaluates_once_every_arm_of_a_kind_is_live():
    """A kind whose A3, A4 and A3-mis cells are all evaluated (replay_h6.py's grid) gets a real delta_iae and
    guard; the decision rule itself stays not evaluated (the module docstring: no paired bootstrap draws)."""
    cells = [{"arm": arm, "kind": kind, "evaluated": False, "reason": "x"}
            for arm in replay_h6.ARMS for kind in replay_h6.KINDS]
    by_key = {(cell["arm"], cell["kind"]): cell for cell in cells}
    for arm, mean in (("A3", 0.20), ("A4", 0.10), ("A3-mis", 0.08)):  # mis's ub95 0.10 <= 1.10 x A4's 0.10: guard ok
        by_key[(arm, "provider_fault")].update(evaluated=True, iae_mean=mean, iae_ci95=[mean - 0.02, mean + 0.02],
                                                n=1000, changes=0.0, holds=0.0)

    found = replay_closure.delta_iae_x2({"cells": cells})
    assert found["n_evaluated"] == 1
    cell = found["kinds"]["provider_fault"]
    assert cell["evaluated"] and cell["delta_iae"] == pytest.approx(0.10 - 0.20)
    assert cell["guard_passes"] and not cell["rule_evaluated"]
    for kind in replay_h6.KINDS[1:]:
        assert not found["kinds"][kind]["evaluated"]
