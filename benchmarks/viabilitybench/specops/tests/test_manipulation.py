"""Tests for the manipulation check (S07.3; SC2; task 3234): a vague twin must score at least 30 SQS points
below its precise twin, scored under the same suite and workspace.

Runs on the four 3233 fixtures (F1, F2, F4, F7); the full 48-instance H3 report needs a converter from each
family's own generator into this module's `[[task]]` shape, not part of this change (the module docstring).
Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_manipulation.py
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

import pytest

SPECOPS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SPECOPS_DIR.parent))
sys.path.insert(0, str(SPECOPS_DIR.parent / "speclint"))
import speclint  # noqa: E402
from specops import VAGUE, manipulation_check  # noqa: E402

FIXTURES = SPECOPS_DIR / "fixtures"
NAMES = ["f4_kvtool", "f2_api_pager", "f7_rust_iter", "f1_pyconv_errors"]
VB_ROOT = SPECOPS_DIR.parent


def load(name: str) -> dict:
    return tomllib.loads((FIXTURES / f"{name}.toml").read_text(encoding="utf-8"))["task"][0]


def fixtures() -> dict[str, dict]:
    return {load(name)["id"]: load(name) for name in NAMES}


@pytest.mark.parametrize("name", NAMES)
def test_vague_variants_score_30_below_precise(name):
    spec = load(name)
    report = manipulation_check.check_pair(spec["id"], spec, seed=7, workspace_root=VB_ROOT)
    assert report.gap >= manipulation_check.MIN_GAP, report.as_json()
    assert report.hashes_match
    assert report.ok
    assert report.precise_score > report.vague_score
    assert report.linter == speclint.LINTER
    assert report.manifest["levels"] == list(VAGUE)
    assert spec == load(name), "the precise spec passed in is never modified"


def test_check_many_is_ok_over_the_whole_fixture_set():
    report = manipulation_check.check_many(fixtures(), seed=7, workspace_root=VB_ROOT)
    assert report.ok and not report.failures
    assert len(report.pairs) == len(NAMES)
    assert {pair.instance_id for pair in report.pairs} == {spec["id"] for spec in fixtures().values()}
    assert report.as_json()["ok"] is True


def test_a_gap_below_the_bar_is_reported_and_fails():
    # A pair whose two halves happen to score close together (far from the 48 real instances, which the other
    # tests show clear the bar by a wide margin): PairReport is built directly to force the near-miss.
    report = manipulation_check.PairReport(
        instance_id="F1-0011", precise_score=50.0, vague_score=49.0, linter=speclint.LINTER, manifest={},
        precise_suite_hash="a", vague_suite_hash="a", precise_env_hash="b", vague_env_hash="b")
    assert report.gap == 1.0 and not report.ok
    many = manipulation_check.Report([report])
    assert not many.ok
    assert many.failures == ["F1-0011: gap 1.0 < 30"]


def test_mismatched_suites_or_workspaces_fail_even_with_a_big_gap():
    report = manipulation_check.PairReport(
        instance_id="X-1", precise_score=90.0, vague_score=10.0, linter=speclint.LINTER, manifest={},
        precise_suite_hash="a", vague_suite_hash="different", precise_env_hash="b", vague_env_hash="b")
    assert report.gap == 80.0 and not report.hashes_match and not report.ok
    assert manipulation_check.Report([report]).failures == [
        "X-1: precise and vague were scored under different suites or workspaces"]


def test_suite_hash_changes_with_the_linter_and_env_hash_with_the_workspace(tmp_path):
    assert manipulation_check.suite_hash("sq-2") != manipulation_check.suite_hash("sq-3")
    assert manipulation_check.suite_hash("sq-3") == manipulation_check.suite_hash("sq-3")
    other = tmp_path / "elsewhere"
    other.mkdir()
    assert manipulation_check.env_hash(VB_ROOT) != manipulation_check.env_hash(other)


def test_an_empty_report_is_not_ok():
    assert manipulation_check.Report([]).ok is False
