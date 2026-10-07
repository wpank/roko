"""Tests for R-H6 (replay_h6.py, task 3357): S06's controller replay over a LOG1-shaped fixture. No model is
called, and no real roko-learn evaluator run is needed: the table is built by hand, the shape
`roko_learn::homeostasis::replay::ArmReport` writes (the module docstring).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_h6.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import replay_h6
import replay_runner
from test_analysis import EXPERIMENT, run_record, write_run

INSTANCES = [f"F1-l1-{n:04d}" for n in range(1, 4)]  # 3 tasks: enough for a valid campaign, irrelevant to R-H6


def log1_tree(root: Path) -> Path:
    """roko_fixed on 3 tasks: R-H6 reads none of the matrix (the module docstring), so only a valid campaign is
    needed for `replay_runner.run` to load."""
    records = []
    for position, instance in enumerate(INSTANCES, 1):
        record = run_record(instance, 1, run_id="roko", arm="roko_fixed", label=1, verdict="passed")
        record["stream"].update(id="p1_core", position=position)
        records.append(record)
    write_run(root / "results" / EXPERIMENT, records)
    return root / "results"


def write_table(path: Path, rows: list[dict]) -> Path:
    path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")
    return path


def test_r_h6_replay_runs_every_controller_and_hook(tmp_path):
    """Every one of the 8 arms x 6 kinds appears exactly once. Without --table, every cell is unevaluated with
    `replay_h6.NOT_WIRED`; the replay is still deterministic seed for seed."""
    results = log1_tree(tmp_path)
    bare = replay_runner.run(results, [EXPERIMENT], "h6", seed=1)
    again = replay_runner.run(results, [EXPERIMENT], "h6", seed=1)
    assert replay_runner.canonical(bare) == replay_runner.canonical(again)
    found = bare["estimates"]
    assert found["arms"] == list(replay_h6.ARMS) and found["kinds"] == list(replay_h6.KINDS)
    cells = found["cells"]
    assert len(cells) == len(replay_h6.ARMS) * len(replay_h6.KINDS) == 48
    assert {(cell["arm"], cell["kind"]) for cell in cells} == {(arm, kind) for arm in replay_h6.ARMS
                                                               for kind in replay_h6.KINDS}
    assert found["n_evaluated"] == 0 and found["table"] is None
    assert all(not cell["evaluated"] and cell["reason"] == replay_h6.NOT_WIRED for cell in cells)


def test_r_h6_replay_reads_evaluated_and_noted_rows_from_the_table(tmp_path):
    """A table naming one evaluated (arm, kind) and one with a `note` (no predictions, A3-gated/A3-mis's own
    case) reports exactly those two cells accordingly; every other cell of the 48 stays unevaluated, for its own
    reason (missing from the table, not "not wired")."""
    results = log1_tree(tmp_path)
    table = write_table(tmp_path / "h6_table.jsonl", [
        {"arm": "A0", "disturbance": "provider_fault",
         "iae": {"mean": 0.12, "low": 0.08, "high": 0.17, "n": 1000}, "changes": 0.0, "holds": 2.5, "note": None},
        {"arm": "A3-gated", "disturbance": "model_swap", "iae": None, "changes": 0.0, "holds": 0.0,
         "note": "no predictions: M3 is not wired to the replay yet"},
    ])
    found = replay_runner.run(results, [EXPERIMENT], "h6", seed=1, params={"table": table})["estimates"]
    assert found["n_evaluated"] == 1 and found["table"] == str(table)
    by_key = {(cell["arm"], cell["kind"]): cell for cell in found["cells"]}
    good = by_key[("A0", "provider_fault")]
    assert good == {"arm": "A0", "kind": "provider_fault", "evaluated": True, "iae_mean": 0.12,
                    "iae_ci95": [0.08, 0.17], "n": 1000, "changes": 0.0, "holds": 2.5}
    noted = by_key[("A3-gated", "model_swap")]
    assert noted == {"arm": "A3-gated", "kind": "model_swap", "evaluated": False,
                     "reason": "no predictions: M3 is not wired to the replay yet"}
    others = [cell for key, cell in by_key.items() if key not in {("A0", "provider_fault"),
                                                                   ("A3-gated", "model_swap")}]
    assert len(others) == 46
    assert all(not cell["evaluated"] and cell["reason"] == "no row in --table for this (arm, kind)"
              for cell in others)


def test_r_h6_reads_rows_as_the_evaluator_cli_writes_them(tmp_path):
    """`roko learn homeostasis replay --evaluate` writes whole `ArmReport`s, `recovery` scorecard included
    (gap-1a8ee7): R-H6 reads the fields it knows and ignores the rest, so such a row's cell is evaluated."""
    results = log1_tree(tmp_path)
    table = write_table(tmp_path / "h6_table.jsonl", [
        {"arm": "A3", "disturbance": "budget_cut", "recovery": {"iae": 0.3, "recovered": True},
         "iae": {"mean": 0.25, "low": 0.2, "high": 0.31, "n": 20}, "changes": 1.5, "holds": 0.0, "note": None},
    ])
    found = replay_runner.run(results, [EXPERIMENT], "h6", seed=1, params={"table": table})["estimates"]
    assert found["n_evaluated"] == 1
    cell = next(cell for cell in found["cells"] if (cell["arm"], cell["kind"]) == ("A3", "budget_cut"))
    assert cell == {"arm": "A3", "kind": "budget_cut", "evaluated": True, "iae_mean": 0.25,
                    "iae_ci95": [0.2, 0.31], "n": 20, "changes": 1.5, "holds": 0.0}


def test_load_table_rejects_an_unknown_arm_or_kind(tmp_path):
    bad = write_table(tmp_path / "bad.jsonl", [{"arm": "A9", "disturbance": "provider_fault"}])
    with pytest.raises(ValueError, match="not an ArmReport row"):
        replay_h6.load_table(bad)
