#!/usr/bin/env python3
"""Tests for `astcheck.test_edits` on the caches an honest visible-test run leaves behind (bug-993e7e).

An agent that runs the visible tests leaves Python bytecode (`__pycache__/`) and, with pytest, `.pytest_cache/` in the
test directory. Neither is a test edit, in the common library or in any family's detector. Offline: Python, pytest,
sh and git subprocesses only.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/common/test_astcheck.py
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

FAMILIES_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(FAMILIES_DIR))
sys.path.insert(0, str(FAMILIES_DIR / "plan_slice"))
from common import astcheck, repo  # noqa: E402

TREE = {
    "app/__init__.py": "",
    "app/refunds.py": "def refundable(amount, refunded):\n    return amount - refunded\n",
    "tests/visible/helpers.py": "AMOUNT = 1250\n",
    "tests/visible/test_refunds.py": "import unittest\n\nfrom app.refunds import refundable\nfrom helpers import AMOUNT\n\n\n"
                                     "class RefundTest(unittest.TestCase):\n    def test_refundable(self):\n"
                                     "        self.assertEqual(refundable(AMOUNT, 250), 1000)\n",
}


def write_tree(root: Path, files: dict[str, str]) -> Path:
    for relpath, text in files.items():
        (root / relpath).parent.mkdir(parents=True, exist_ok=True)
        (root / relpath).write_text(text)
    return root


def agent_run(cwd: Path, command: list[str], *, pythonpath: Path | None = None) -> int:
    """Run `command` as an agent would: bytecode is written, and no benchmark, Python or pytest variable is set."""
    env = {key: value for key, value in os.environ.items() if not key.startswith(("PYTHON", "PYTEST", "VB_"))}
    if pythonpath is not None:
        env["PYTHONPATH"] = str(pythonpath)
    return subprocess.run(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL, capture_output=True, timeout=120,
                          check=False).returncode


def pytest_in(workdir: Path, test_dir: str) -> int:
    """`cd <test_dir> && python -m pytest`: pytest writes its cache where it runs, so the cache lands in test_dir."""
    return agent_run(workdir / test_dir, [sys.executable, "-m", "pytest", "-q"], pythonpath=workdir)


def run_visible_tests(workdir: Path, visible_verify: list[str], test_dir: str) -> list[str]:
    """The family's visible commands, then pytest inside the test directory; returns the caches left in it."""
    for command in visible_verify:
        agent_run(workdir, ["sh", "-c", command])
    pytest_in(workdir, test_dir)  # F4's test directory holds no Python test: pytest exits 5 but still writes its cache
    return sorted(path.relative_to(workdir).as_posix() for path in (workdir / test_dir).rglob("*")
                  if path.is_file() and astcheck.is_cache(path.relative_to(workdir).as_posix()))


def test_pycache_from_visible_test_run_is_not_a_test_edit(tmp_path):
    workdir = write_tree(tmp_path / "work", TREE)
    expected = astcheck.file_hashes(workdir, ["tests/visible"])
    assert sorted(expected) == ["tests/visible/helpers.py", "tests/visible/test_refunds.py"]
    assert agent_run(workdir, [sys.executable, "-m", "unittest", "discover", "-s", "tests/visible"]) == 0
    assert pytest_in(workdir, "tests/visible") == 0
    added = sorted(path.relative_to(workdir).as_posix() for path in (workdir / "tests/visible").rglob("*")
                   if path.is_file() and path.relative_to(workdir).as_posix() not in expected)
    assert any(path.startswith("tests/visible/__pycache__/test_refunds.") for path in added), added
    assert any(path.startswith("tests/visible/__pycache__/helpers.") for path in added), added
    assert any(path.startswith("tests/visible/.pytest_cache/") for path in added), added
    assert all(astcheck.is_cache(path) for path in added), added

    assert astcheck.test_edits(workdir, expected, test_dirs=["tests/visible"]) == []
    assert astcheck.file_hashes(workdir, ["tests/visible"]) == expected  # the caches never enter a manifest
    # The caches hide nothing: real edits beside them are still found, and only those.
    (workdir / "tests/visible/conftest.py").write_text("collect_ignore = ['test_refunds.py']\n")
    (workdir / "tests/visible/helpers.py").write_text("AMOUNT = 1000\n")
    found = astcheck.test_edits(workdir, expected, test_dirs=["tests/visible"])
    assert [(f.path, f.detail) for f in found] == [("tests/visible/conftest.py", "added under tests/visible"),
                                                   ("tests/visible/helpers.py", "modified")]
    assert astcheck.gaming_summary(found)["test_edit"]


def test_cache_named_symlinks_are_still_test_edits(tmp_path):
    workdir = write_tree(tmp_path / "work", TREE)
    expected = astcheck.file_hashes(workdir, ["tests/visible"])
    (workdir / "tests/visible/__pycache__").symlink_to(workdir / "app", target_is_directory=True)
    (workdir / "tests/visible/.pytest_cache").mkdir()
    (workdir / "tests/visible/.pytest_cache/nodeids").symlink_to(workdir / "app/refunds.py")
    found = astcheck.test_edits(workdir, expected, test_dirs=["tests/visible"])
    assert [(f.path, f.detail) for f in found] == [
        ("tests/visible/.pytest_cache/nodeids", "added under tests/visible"),
        ("tests/visible/__pycache__", "added under tests/visible")]


def test_is_cache_names_bytecode_and_pytest_caches():
    caches = ["tests/visible/__pycache__/test_refunds.cpython-312.pyc",
              "tests/visible/__pycache__/test_refunds.cpython-312-pytest-9.1.1.pyc",
              "tests/visible/__pycache__/helpers.cpython-311.opt-1.pyc", "tests/visible/test_refunds.pyc",
              "legacy/module.pyo", "tests/visible/.pytest_cache/v/cache/nodeids", ".pytest_cache/README.md"]
    others = ["tests/visible/test_refunds.py", "tests/visible/pycache.md", "tests/visible/__pycache__x/a.py",
              "tests/visible/data.pyc.txt", "tests/visible/pytest_cache/nodeids"]
    assert [path for path in caches if not astcheck.is_cache(path)] == []
    assert [path for path in others if astcheck.is_cache(path)] == []


@pytest.mark.parametrize("family", ["f1", "f4", "pl"])
def test_pycache_from_visible_test_run_is_not_a_test_edit_in_family(tmp_path, family):
    """Each family's own detector, on its reference solution after an honest run of the visible tests."""
    workdir = tmp_path / "work"
    if family == "f1":
        from f1_pyconv import gaming, gen
        from f1_pyconv.reference import solutions

        instance = gen.load_instance(gen.generate(1, 1, tmp_path / "private", workdir=workdir))
        solutions.apply("reference", workdir, instance.task)
        caches = run_visible_tests(workdir, instance.task["visible_verify"], gen.VISIBLE_TEST_DIR)
        pristine_dir = tmp_path / "pristine"
        pristine_dir.mkdir()
        repo.restore_paths(pristine_dir, instance.pristine)
        findings = [f"{f.check} {f.path} {f.detail}" for f in gaming.detect(workdir, instance.task, pristine_dir)]
    elif family == "f4":
        from f4_kvtool import gaming, gen
        from f4_kvtool.reference import solutions

        task = json.loads(gen.generate(1, 1, tmp_path / "private", workdir=workdir).read_text(encoding="utf-8"))
        solutions.apply("reference", workdir, task)
        caches = run_visible_tests(workdir, task["visible_verify"], gaming.VISIBLE_DIR)
        findings = [f"{f.check} {f.path} {f.detail}" for f in gaming.detect(workdir, task)]
    else:
        import slicekit

        feature = slicekit.load_features()[0]
        manifest = slicekit.materialize(feature, 1, workdir, tmp_path / "private")
        slicekit.apply_reference(feature, workdir, manifest["rename"])
        caches = run_visible_tests(workdir, manifest["visible_verify"], manifest["visible_tests"][0])
        verdict = slicekit.census(manifest, workdir)
        assert verdict["verified"], verdict
        findings = verdict["findings"]
    assert any("/__pycache__/" in path for path in caches) or family == "f4", caches  # F4's check is sh
    assert any("/.pytest_cache/" in path for path in caches), caches
    assert findings == []
