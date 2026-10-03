"""Tests for replay IO (replay.py, task 3340), on a small results tree; nothing here calls a model.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay.py -q
"""

from __future__ import annotations

import hashlib
import json
import os
import stat
from pathlib import Path

import pytest

import figlib
import metrics
import replay
import report
from metrics import Clause
from test_analysis import run_record

S01_KEY = "F1-l1-0001.s1"


def record(experiment: str, run_id: str, instance: str, seed: int, position: int, *, arm: str = "cheap_direct",
           s01: bool = False) -> dict:
    row = run_record(instance, seed, run_id=run_id, arm=arm)
    row["experiment_id"] = experiment
    row["stream"]["position"] = position
    if s01:
        row["provenance"]["s01_run_dir"] = f"s01/{S01_KEY}"
    return row


def write_run(directory: Path, rows: list[dict]) -> Path:
    directory.mkdir(parents=True)
    (directory / "manifest.json").write_text("{}\n")
    (directory / "records.jsonl").write_text("".join(json.dumps(row) + "\n" for row in rows))
    return directory


def results_tree(root: Path) -> Path:
    """Two experiments, written out of order, a Roko run with its S01 copy, and two directories that are not runs."""
    results = root / "results"
    write_run(results / "LOG1" / "run-b", [record("LOG1", "run-b", f"F2-l1-000{n}", 1, n) for n in (3, 1, 2)])
    roko = write_run(results / "LOG1" / "run-a", [record("LOG1", "run-a", "F1-l1-0001", seed, 1, arm="roko_fixed",
                                                         s01=seed == 1) for seed in (2, 1)])
    s01 = roko / "s01" / S01_KEY
    (s01 / "learn").mkdir(parents=True)
    (s01 / "runs" / "plan-x").mkdir(parents=True)
    (s01 / "episodes.jsonl").write_text('{"task_id": "t", "attempt": 2}\n\n{"task_id": "t", "attempt": 1}\n')
    (s01 / "learn" / "costs.jsonl").write_text('{"usd": 0.01}\n')
    (s01 / "runs" / "plan-x" / "attempts.jsonl").write_text('{"schema_version": "verdict", "attempt": 1}\n')
    write_run(results / "E-P1-live" / "run-c", [record("E-P1-live", "run-c", "F3-l2-0001", 1, 1, arm="roko_full")])
    (results / "LOG1" / ".campaign").mkdir()  # campaign.py's derived streams: not a run
    write_run(results / "LOG1.abandoned" / "run-z", [record("LOG1", "run-z", "F9-l1-0001", 1, 1)])
    return results


def snapshot(root: Path) -> dict[str, tuple]:
    return {path.relative_to(root).as_posix(): (path.stat().st_mtime_ns, hashlib.sha256(path.read_bytes()).hexdigest())
            for path in sorted(root.rglob("*")) if path.is_file()}


def test_replay_io_reads_records_deterministically(tmp_path):
    results = results_tree(tmp_path)
    before = snapshot(results)
    first, second = replay.load(results), replay.load(results)
    assert first == second and not first.problems and not first.blinded
    assert [(row["experiment_id"], row["run_id"], row["record_id"]) for row in first.records] == sorted(
        (row["experiment_id"], row["run_id"], row["record_id"]) for row in first.records)
    assert [row["run_id"] for row in first.records] == ["run-c", "run-a", "run-a", "run-b", "run-b", "run-b"]
    with_s01 = [entry for entry in first.entries if entry.s01]
    assert len(with_s01) == 1 and with_s01[0].record["seed"] == 1
    assert with_s01[0].s01 == {"episodes.jsonl": ({"task_id": "t", "attempt": 2}, {"task_id": "t", "attempt": 1}),
                               "learn/costs.jsonl": ({"usd": 0.01},),
                               "runs/plan-x/attempts.jsonl": ({"schema_version": "verdict", "attempt": 1},)}
    assert replay.load(results, ["LOG1"]).records == [row for row in first.records if row["experiment_id"] == "LOG1"]
    # The lines' order on disk does not matter, and nothing under results changed: replays only read.
    records_file = results / "LOG1" / "run-b" / "records.jsonl"
    lines = records_file.read_text().splitlines()
    records_file.write_text("\n".join(reversed(lines)) + "\n")
    assert replay.load(results) == first
    records_file.write_text("\n".join(lines) + "\n")
    os.utime(records_file, ns=(before["LOG1/run-b/records.jsonl"][0],) * 2)
    assert snapshot(results) == before
    for directory in [results, *(path for path in results.rglob("*") if path.is_dir())]:
        directory.chmod(stat.S_IRUSR | stat.S_IXUSR)  # a read-only tree loads all the same
    try:
        assert replay.load(results) == first
    finally:
        for directory in [results, *(path for path in results.rglob("*") if path.is_dir())]:
            directory.chmod(stat.S_IRWXU)


def test_a_corrupt_row_is_reported_not_skipped(tmp_path):
    results = results_tree(tmp_path)
    bad_record = dict(record("LOG1", "run-b", "F2-l1-0009", 1, 9))
    del bad_record["vs"]
    with (results / "LOG1" / "run-b" / "records.jsonl").open("a") as handle:
        handle.write("{not json\n" + json.dumps(bad_record) + "\n")
    with (results / "LOG1" / "run-a" / "s01" / S01_KEY / "learn" / "costs.jsonl").open("a") as handle:
        handle.write("[1, 2]\n")
    misfiled = write_run(results / "E-P1-live" / "run-d", [record("LOG1", "run-x", "F3-l2-0002", 1, 1)])
    with pytest.raises(replay.ReplayError) as caught:
        replay.load(results)
    message = str(caught.value)
    assert "run-b/records.jsonl:4: not JSON" in message and "run-b/records.jsonl:5:" in message
    assert f"s01/{S01_KEY}/learn/costs.jsonl:2: not a JSON object" in message
    assert "filed under E-P1-live/run-d" in message and str(misfiled) in message
    lenient = replay.load(results, strict=False)
    assert len(lenient.problems) == 4 and {problem.line for problem in lenient.problems} == {1, 2, 4, 5}
    assert len(lenient.records) == 6  # the sound rows, every one of them, and none of the corrupt ones
    (results / "LOG1" / "run-a" / "s01" / S01_KEY / "episodes.jsonl").unlink()
    (results / "LOG1" / "run-a" / "s01" / S01_KEY / "episodes.jsonl").symlink_to(tmp_path / "elsewhere.jsonl")
    assert any(problem.why == "not a regular file, so it is not read"
               for problem in replay.load(results, strict=False).problems)


def test_blinding_hook_and_stream_order(tmp_path):
    results = results_tree(tmp_path)
    blinded = replay.load(results, blind=lambda arm: "arm-" + hashlib.sha256(b"salt" + arm.encode()).hexdigest()[:8])
    assert blinded.blinded and all(row["arm"].startswith("arm-") for row in blinded.records)
    assert len({row["arm"] for row in blinded.records}) == 3  # one label per arm
    streams = replay.load(results).streams()
    assert list(streams) == [("E-P1-live", "run-c", 1), ("LOG1", "run-a", 1), ("LOG1", "run-a", 2),
                             ("LOG1", "run-b", 1)]
    assert [entry.record["stream"]["position"] for entry in streams[("LOG1", "run-b", 1)]] == [1, 2, 3]
    assert [entry.record["task"]["instance_id"] for entry in streams[("LOG1", "run-b", 1)]] == [
        "F2-l1-0001", "F2-l1-0002", "F2-l1-0003"]
    write_run(results / "LOG1" / "run-e", [record("LOG1", "run-e", f"F4-l1-000{n}", 1, 1) for n in (1, 2)])
    with pytest.raises(replay.ReplayError):
        replay.load(results).streams()


def test_a_metric_at_a_stream_position_reads_back_in_the_figure_scripts():
    """3340's note (PK96): F9 reads `regret_cum` at stream position p from the clause `stream.position == p` in the
    MetricRecord's record_filter, through figlib. position_cut writes that clause."""
    rows = [record("R-H4", "run-a", f"F1-l1-000{n}", 1, n, arm="roko_full") for n in (1, 2, 3)]
    cut = replay.position_cut(rows, 3, [Clause("arm", "==", "roko_full")])
    assert [row["stream"]["position"] for row in cut.rows] == [3] and cut.filter.endswith("stream.position == 3")
    metric = metrics.Metric(metric="regret_cum", value=0.4, n=3, estimator="cumulative regret after 3 tasks",
                            cost_basis=None, cut=cut, cell="all", ci=(0.2, 0.6), ci_method="band_90_orderings")
    row = report.metric_record(metric, experiment_id="R-H4", snapshot="prices-2026-09-28", analysis_commit="abc",
                               computed_at="now")
    parsed = figlib.Rec(row, "id-1", figlib.parse_filter(row["record_filter"]))
    assert parsed.eq("stream.position") == 3
    with pytest.raises(ValueError):
        replay.position_cut(rows, 0)
