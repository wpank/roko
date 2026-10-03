"""Tests for `vb replay` (replay_runner.py, task 3354) on a small LOG1-shaped results tree; nothing here calls a model.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_runner.py -q
"""

from __future__ import annotations

import io
import json
import os
import random
import sys
from contextlib import redirect_stdout
from pathlib import Path

import blind
import replay_runner
from test_analysis import EXPERIMENT, run_record, write_run

DRIVER = Path(__file__).resolve().parents[1] / "driver"


def campaign_tree(root: Path) -> Path:
    """roko_fixed and cheap_direct on 60 tasks (F1-F4 x levels 1-5 x 3) x 2 seeds, VS drawn per task from a
    level-dependent rate, so tasks differ and a task's two seeds agree more often than chance."""
    rng = random.Random(20261003)
    results = root / "results"
    for run_id, arm in (("roko", "roko_fixed"), ("cheap", "cheap_direct")):
        records = []
        for family in ("F1", "F2", "F3", "F4"):
            for level in range(1, 6):
                for n in (1, 2, 3):
                    ease = rng.random() * (1.1 - level / 5)
                    for seed in (1, 2):
                        record = run_record(f"{family}-l{level}-000{n}", seed, run_id=run_id, arm=arm,
                                            label=int(rng.random() < ease))
                        record["stream"]["position"] = len(records) + 1
                        records.append(record)
        write_run(results / EXPERIMENT, records)
    return results


def salt_file(path: Path) -> Path:
    path.write_text(bytes(range(32)).hex() + "\n")  # blind.new_salt's form: the salt as hex text
    os.chmod(path, 0o600)
    return path


def test_replay_is_byte_identical_and_aa_shows_no_effect(tmp_path):
    results = campaign_tree(tmp_path)
    blinder = blind.Blinder(bytes(range(32)))
    first = replay_runner.run(results, [EXPERIMENT], "aa", seed=7, blind=blinder.label, params={"reps": 400})
    again = replay_runner.run(results, [EXPERIMENT], "aa", seed=7, blind=blinder.label, params={"reps": 400})
    assert replay_runner.canonical(first) == replay_runner.canonical(again)  # byte-identical, seed for seed
    one = replay_runner.export(first, tmp_path / "out" / "a.json").read_bytes()
    assert one == replay_runner.export(again, tmp_path / "out" / "b.json").read_bytes()
    other = replay_runner.run(results, [EXPERIMENT], "aa", seed=8, blind=blinder.label, params={"reps": 400})
    assert other["estimates"]["first_splits"] != first["estimates"]["first_splits"]
    assert other["matrix"] == first["matrix"]  # the same data, another seed

    # The replay ran on arm-hashed data: labels, never arm ids, and the record says so.
    text = replay_runner.canonical(first)
    assert first["blinded"] and "roko_fixed" not in text and "cheap_direct" not in text
    assert first["matrix"]["rows"] == 240 and first["estimates"]["arm"].startswith(blind.PREFIX)

    # A/A: two halves of one arm's tasks differ by chance alone, so the 95% interval covers 0 at about 95%.
    aa = first["estimates"]
    assert aa["reps"] == 400 and aa["tasks"] == [30, 30] and aa["passed"], aa
    assert aa["band"][0] <= aa["coverage"] <= aa["band"][1] and abs(aa["mean_diff"]) < 0.05

    # The descriptive adapter, and the command: blinded by the salt file; unblinding refused before the lock.
    rates = replay_runner.run(results, [EXPERIMENT], "vs_rates", seed=7, blind=blinder.label)
    assert sorted(rates["estimates"]["arms"]) == sorted(blinder.label(arm) for arm in ("roko_fixed", "cheap_direct"))
    for row in rates["estimates"]["arms"].values():
        assert row["n"] == 120 and row["wilson95"][0] <= row["rate"] <= row["wilson95"][1]
    salt = salt_file(tmp_path / "salt")
    args = ["--experiment", EXPERIMENT, "--results", str(results), "--adapter", "aa", "--seed", "7", "--reps",
            "400", "--salt", str(salt), "--out", str(tmp_path / "out" / "cli.json")]
    shown = io.StringIO()
    with redirect_stdout(shown):
        assert replay_runner.main(args) == 0
    assert shown.getvalue() == text + "\n" == (tmp_path / "out" / "cli.json").read_text()
    assert replay_runner.main([*args[:6], "--unblinded"]) == 2  # no committed lock in a test tree

    # `vb replay` is the same command.
    sys.path.insert(0, str(DRIVER))
    import vb

    through_vb = io.StringIO()
    with redirect_stdout(through_vb):
        assert vb.main(["replay", *args]) == 0
    assert through_vb.getvalue() == shown.getvalue()
    assert json.loads(through_vb.getvalue())["adapter"] == "aa"
