"""Tests for rust_parity.py, the speclint check of ``roko plan validate --spec-quality`` (gap-46ab3f).

They need no roko binary: the end-to-end tests run a stand-in that prints speclint's own records.
Run from the repo root with the benchmark venv:
``benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint``.
"""

from __future__ import annotations

import json
import shutil
import stat
import sys
from pathlib import Path

import pytest

SPECLINT_DIR = Path(__file__).resolve().parents[1]
ROOT = SPECLINT_DIR.parents[2]

sys.path.insert(0, str(SPECLINT_DIR))
import rust_parity  # noqa: E402
import speclint  # noqa: E402


def test_vendored_fixtures_match_speclint():
    assert rust_parity.fixture_drift(ROOT) == []


def test_runs_cover_what_plan_validate_skips(tmp_path):
    for rel in ("a/tasks.toml", "archive/old/tasks.toml", "b/archived/x/tasks.toml", "b/c/tasks.toml", "b/notes.md"):
        path = tmp_path / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("")

    def rels(paths):
        return [path.relative_to(tmp_path).as_posix() for path in paths]

    assert rels(rust_parity.validated_files(tmp_path)) == ["a/tasks.toml", "b/c/tasks.toml"]
    assert rels(rust_parity.run_roots(tmp_path)[1:]) == ["archive", "b/archived"]
    assert rels(rust_parity.validated_files(tmp_path / "archive")) == ["archive/old/tasks.toml"]


def _record(task_id, score, hard=(), band="D"):
    return {
        "plan_path": "plans/p/tasks.toml",
        "task_id": task_id,
        "score": score,
        "band": band,
        "hard_fail": list(hard),
        "hard_fail_detail": {},
        "rules": {"SQ01": 0.0},
        "verify_classes": [],
    }


def test_compare_fails_on_scores_and_hard_fails_and_notes_the_rest():
    result = rust_parity.Comparison()
    ours = [_record("T1", 37.0), _record("T2", 17.2, ["HF2"]), _record("T3", 20.0, band="C"), _record("T4", 45.0)]
    theirs = [_record("T1", 37.4), _record("T2", 17.0), _record("T3", 20.0), _record("T4", 44.0)]
    rust_parity.compare(ours, theirs, 0.5, result)
    assert result.tasks == 4
    assert result.failures == [
        "plans/p/tasks.toml T2: hard fails ['HF2'] vs []",
        "plans/p/tasks.toml T4: score 45.0 vs 44.0",
    ]
    assert result.notes == ["plans/p/tasks.toml T3: band C vs D"]
    assert result.max_delta == pytest.approx(1.0)


def test_compare_reports_missing_tasks():
    result = rust_parity.Comparison()
    rust_parity.compare([_record("T1", 37.0)], [_record("T1", 37.0), _record("T2", 17.0)], 0.5, result)
    assert result.tasks == 0
    assert len(result.failures) == 1 and "task ids differ" in result.failures[0]


def _workspace_with_stub_roko(tmp_path: Path, edit=None) -> tuple[Path, Path]:
    """A workspace with one plan, the vendored fixtures, and a roko that prints speclint's records."""
    root = tmp_path.resolve()
    tasks = root / "plans" / "p" / "tasks.toml"
    tasks.parent.mkdir(parents=True)
    tasks.write_text((SPECLINT_DIR / "fixtures" / "sq04-verify-strength" / "tasks.toml").read_text())
    shutil.copytree(ROOT / rust_parity.VENDORED_FIXTURES, root / rust_parity.VENDORED_FIXTURES)
    records, errors = speclint.lint_files([tasks], root)
    assert errors == []
    if edit:
        edit(records)
    payload = {"plans": [], "totals": {}, "spec_quality": {"linter": "sq-1", "tasks": records, "parse_errors": []}}
    stub = root / "roko"
    stub.write_text(f"#!/bin/sh\ncat <<'JSON'\n{json.dumps(payload)}\nJSON\n")
    stub.chmod(stub.stat().st_mode | stat.S_IEXEC)
    return root, stub


def test_main_passes_when_roko_matches_speclint(tmp_path, capsys):
    root, stub = _workspace_with_stub_roko(tmp_path)
    assert rust_parity.main(["--root", str(root), "--roko", str(stub), "plans"]) == 0
    out = capsys.readouterr().out
    assert "1 plan validate runs, 1 files, 7 tasks compared" in out
    assert out.rstrip().endswith("PASS")


def test_main_fails_on_a_hard_fail_difference(tmp_path, capsys):
    def add_hard_fail(records):
        records[0]["hard_fail"] = ["HF1"]

    root, stub = _workspace_with_stub_roko(tmp_path, add_hard_fail)
    assert rust_parity.main(["--root", str(root), "--roko", str(stub), "plans"]) == 1
    out = capsys.readouterr().out
    assert "T1: hard fails ['HF1'] vs []" in out
    assert out.rstrip().endswith("FAIL")


def test_main_without_a_binary_exits_2(tmp_path):
    assert rust_parity.main(["--root", str(tmp_path), "--roko", str(tmp_path / "missing"), "plans"]) == 2
