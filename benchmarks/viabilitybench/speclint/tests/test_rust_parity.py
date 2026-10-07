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


def _dynamic_payload(edit=None) -> dict:
    """What ``plan validate --dynamic --json`` prints for the dynamic fixture: its expected results."""
    tasks, checks = [], []
    for key, want in rust_parity.expected_dynamic().items():
        plan_path, task_id = key.rsplit(" ", 1)
        hard_fail = ["HF3"] if want["HF3"] else []
        tasks.append({"plan_path": plan_path, "task_id": task_id, "red_on_base": want["red_on_base"], "rules": {"SQ06": float(want["SQ06"])}, "hard_fail": hard_fail})
        checks.append({"plan_path": plan_path, "task_id": task_id, "outcome": want["outcome"]})
    if edit:
        edit(tasks, checks)
    return {
        "plans": [],
        "totals": {},
        "spec_quality": {"linter": speclint.LINTER, "tasks": tasks, "parse_errors": []},
        "red_on_base": {"cargo": "run", "checks": checks},
    }


def _workspace_with_stub_roko(tmp_path: Path, edit=None, dynamic_edit=None) -> tuple[Path, Path]:
    """A workspace with one plan, the vendored fixtures, and a roko that prints speclint's records,
    or the dynamic fixture's expected results when called with ``--dynamic``."""
    root = tmp_path.resolve()
    tasks = root / "plans" / "p" / "tasks.toml"
    tasks.parent.mkdir(parents=True)
    tasks.write_text((SPECLINT_DIR / "fixtures" / "sq04-verify-strength" / "tasks.toml").read_text())
    shutil.copytree(ROOT / rust_parity.VENDORED_FIXTURES, root / rust_parity.VENDORED_FIXTURES)
    records, errors = speclint.lint_files([tasks], root)
    assert errors == []
    if edit:
        edit(records)
    payload = {"plans": [], "totals": {}, "spec_quality": {"linter": speclint.LINTER, "tasks": records, "parse_errors": []}}
    dynamic = _dynamic_payload(dynamic_edit)
    stub = root / "roko"
    stub.write_text(
        "#!/bin/sh\n"
        'case " $* " in\n'
        f'  *" --dynamic "*) cat <<\'JSON\'\n{json.dumps(dynamic)}\nJSON\n  ;;\n'
        f"  *) cat <<'JSON'\n{json.dumps(payload)}\nJSON\n  ;;\n"
        "esac\n"
    )
    stub.chmod(stub.stat().st_mode | stat.S_IEXEC)
    return root, stub


def test_main_passes_when_roko_matches_speclint(tmp_path, capsys):
    root, stub = _workspace_with_stub_roko(tmp_path)
    assert rust_parity.main(["--root", str(root), "--roko", str(stub), "plans"]) == 0
    out = capsys.readouterr().out
    assert "1 plan validate runs, 1 files, 7 tasks compared" in out
    assert "dynamic fixture (red on base): 8 tasks, 0 differences" in out
    assert out.rstrip().endswith("PASS")


def test_main_fails_on_a_red_on_base_difference(tmp_path, capsys):
    """3214: roko's red-on-base outcome for a dynamic fixture task must match expected.json."""

    def green_is_red(tasks, checks):
        for task, check in zip(tasks, checks):
            if task["task_id"] == "T2" and task["plan_path"] == "tasks.toml":
                task["red_on_base"], task["hard_fail"], check["outcome"] = "fail", [], "fail"

    root, stub = _workspace_with_stub_roko(tmp_path, dynamic_edit=green_is_red)
    assert rust_parity.main(["--root", str(root), "--roko", str(stub), "plans"]) == 1
    out = capsys.readouterr().out
    assert "tasks.toml T2: red_on_base 'fail' vs 'pass'" in out
    assert "tasks.toml T2: outcome 'fail' vs 'pass'" in out
    assert "tasks.toml T2: HF3 False vs True" in out
    assert out.rstrip().endswith("FAIL")


def test_compare_dynamic_reports_missing_and_extra_tasks():
    want = {"tasks.toml T1": {"red_on_base": "fail", "outcome": "fail", "SQ06": 1, "HF3": False}}
    got = {"tasks.toml T9": {"red_on_base": "pass", "outcome": "pass", "SQ06": 0.0, "HF3": True}}
    assert rust_parity.compare_dynamic(got, want) == [
        "tasks.toml T1: missing from roko's records",
        "tasks.toml T9: not in expected.json",
    ]
    got = {"tasks.toml T1": {"red_on_base": "fail", "outcome": "fail", "SQ06": 1.0, "HF3": False}}
    assert rust_parity.compare_dynamic(got, want) == []


def test_main_without_red_on_base_in_the_json_exits_2(tmp_path, capsys):
    """A roko built before 3214 prints no red_on_base for --dynamic: a tool failure, not a pass."""
    root, stub = _workspace_with_stub_roko(tmp_path)
    text = stub.read_text().replace('"red_on_base": {"cargo": "run", "checks": [', '"no_red_on_base": {"checks": [', 1)
    stub.write_text(text)
    assert rust_parity.main(["--root", str(root), "--roko", str(stub), "plans"]) == 2
    assert "no red_on_base in the JSON" in capsys.readouterr().err


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
