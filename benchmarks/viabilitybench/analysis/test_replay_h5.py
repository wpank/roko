"""Tests for R-H5 (replay_h5.py, task 3356): S05's lottery replay over a LOG1-shaped fixture with a known θ.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_h5.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import blind
import replay
import replay_h5
import replay_runner
from test_analysis import EXPERIMENT, run_record, write_run

THETA = 0.15  # 30 false greens among 200 green units


def log1_tree(root: Path) -> tuple[Path, list[str]]:
    """Block A-shaped records: roko_fixed on 200 p1_core tasks, every one green (gate verdict `passed`), 30 of them
    false greens (VS 0); and cheap_direct on the same tasks, which R-H5 leaves out. Returns the results root and the
    false greens' attempt keys."""
    results = root / "results"
    instances = [f"F{family}-l{level}-{n:04d}" for family in (1, 2, 3, 4) for level in range(1, 6)
                 for n in range(1, 11)]
    false_greens = set(instances[::6][:30])
    assert len(false_greens) == 30
    for run_id, arm in (("roko", "roko_fixed"), ("cheap", "cheap_direct")):
        records = []
        for position, instance in enumerate(instances, 1):
            gated = {"verdict": "passed"} if arm == "roko_fixed" else {}
            record = run_record(instance, 1, run_id=run_id, arm=arm, label=int(instance not in false_greens),
                                visible=True, **gated)
            record["stream"].update(id="p1_core", position=position)
            records.append(record)
        write_run(results / EXPERIMENT, records)
    keys = [f"roko/{instance}.s1:1" for instance in sorted(false_greens)]
    return results, keys


def test_r_h5_replay_covers_at_nominal_rate(tmp_path):
    results, false_keys = log1_tree(tmp_path)
    blinder = blind.Blinder(bytes(range(32)))
    roko = blinder.label("roko_fixed")
    uniform = replay_runner.run(results, [EXPERIMENT], "h5", seed=5, blind=blinder.label)
    again = replay_runner.run(results, [EXPERIMENT], "h5", seed=5, blind=blinder.label)
    assert replay_runner.canonical(uniform) == replay_runner.canonical(again)  # deterministic, seed for seed
    found = uniform["estimates"]
    assert (found["n_units"], found["false_greens"], found["theta_census"]) == (200, 30, THETA)
    assert found["lotteries"] == 1000 and len(found["cells"]) == 9
    for cell in found["cells"]:
        if cell["selection"] == "uniform":  # Hájek with Wilson at Kish n_eff covers θ at about the nominal rate
            assert cell["evaluated"] and cell["targets_met"], cell
            assert cell["coverage"] >= 0.93 and abs(cell["bias_hajek"]) <= 0.01 and abs(cell["theta_ratio"] - 1) < 0.07
        else:
            assert not cell["evaluated"] and cell["reason"], cell
    assert found["primary"] == {"rho": 0.15, "selection": "tilted", "verdict": "not evaluated"}  # no M3 risk yet

    # With M3's risk (false greens look riskier) and λ, the tilted cells and the primary one are evaluated.
    risk = tmp_path / "risk.jsonl"
    risk.write_text("".join(json.dumps({"attempt_key": f"roko/{record_key}", "r": r}) + "\n" for record_key, r in (
        *((key.removeprefix("roko/"), 0.6) for key in false_keys),
        *((f"F{f}-l{level}-{n:04d}.s1:1", 0.1) for f in (1, 2, 3, 4) for level in range(1, 6) for n in range(1, 11)
          if f"roko/F{f}-l{level}-{n:04d}.s1:1" not in false_keys))))
    tilted = replay_runner.run(results, [EXPERIMENT], "h5", seed=5, blind=blinder.label,
                               params={"risk": risk, "lam": 0.8, "arm": roko})
    cells = {(cell["rho"], cell["selection"]): cell for cell in tilted["estimates"]["cells"]}
    primary = cells[(0.15, "tilted")]
    assert primary["primary"] and primary["evaluated"] and primary["mean_n_eff"] < primary["mean_audited"]
    assert primary["coverage"] >= 0.93 and abs(primary["bias_hajek"]) <= 0.01, primary
    assert tilted["estimates"]["primary"]["verdict"] == "pass" and tilted["params"]["arm"] == roko
    assert not cells[(0.15, "ai")]["evaluated"]

    # The replay runs on arm-hashed data, and its A/A check passes on the same fixture.
    assert uniform["blinded"] and "roko_fixed" not in replay_runner.canonical(uniform)
    aa = replay_runner.run(results, [EXPERIMENT], "aa", seed=5, blind=blinder.label, params={"arm": roko})
    assert aa["estimates"]["passed"] and aa["matrix"] == uniform["matrix"]
    assert len(replay_h5.units(replay_runner.matrix(replay.load(results, [EXPERIMENT])))) == 200  # unblinded too
