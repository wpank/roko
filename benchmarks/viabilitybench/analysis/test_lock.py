"""Tests for the pre-registration lock (lock.py) and blinding (blind.py), in a throwaway git repository that holds a
copy of the benchmark tree the lock pins; nothing here calls a model (task 3341).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_lock.py -q
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

import blind
import holm
import lock

VB_ROOT = lock.VB_ROOT
REPO_ROOT = VB_ROOT.parents[1]
COPIED = ("analysis", "audit", "streams", "families")
IGNORED = shutil.ignore_patterns("__pycache__", ".pytest_cache", ".venv")
GIT_ENV = {"GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull, "GIT_AUTHOR_NAME": "vb-test",
           "GIT_AUTHOR_EMAIL": "vb@test.invalid", "GIT_COMMITTER_NAME": "vb-test",
           "GIT_COMMITTER_EMAIL": "vb@test.invalid", "GIT_AUTHOR_DATE": "2026-10-03T08:00:00+00:00",
           "GIT_COMMITTER_DATE": "2026-10-03T08:00:00+00:00"}
PLAN = {"schema_version": "vb.prereg_lock/1", "prereg_id": "s09-v1-…", "spec_sha256": "…", "analysis_commit": "…",
        "price_snapshot_id": "prices-2026-09-28", "suite_hash": "…", "harness_sha": "…", "locked_at": "…",
        "alpha_fw": 0.05, "multiplicity": holm.MULTIPLICITY,
        "primaries": {"H1": {"p": "p_l1", "level_p": "max(p_R_j,p_C_j)", "X": 0.90, "Y": 0.30, "report": "E*"}},
        "exploratory": {"status": "preregistered_exploratory", "in_holm": False}}


def git(repo: Path, *args: str) -> str:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")} | GIT_ENV
    return subprocess.run(["git", "-C", str(repo), *args], env=env, capture_output=True, text=True,
                          check=True).stdout.strip()


def spec_file(path: Path, plan: dict = PLAN, version: str = "1.6") -> Path:
    """An S09 stand-in: a status line and §5's lock block, as the real spec writes them."""
    path.write_text(f"# S09\n\nStatus: draft v{version} (2026-10-02)\n\n## 5. Interfaces and schemas\n\n```json\n"
                    f"{json.dumps(plan)}\n```\n", encoding="utf-8")
    return path


@pytest.fixture
def repo(tmp_path: Path) -> dict[str, Path]:
    """A git repository with a copy of the benchmark tree and the price snapshots, committed, and an S09."""
    root = tmp_path / "repo"
    bench = root / "benchmarks" / "viabilitybench"
    for name in COPIED:
        shutil.copytree(VB_ROOT / name, bench / name, ignore=IGNORED)
    shutil.copy2(VB_ROOT / "requirements-analysis.lock", bench / "requirements-analysis.lock")
    (bench / "experiments").mkdir()
    shutil.copytree(REPO_ROOT / "config" / "prices", root / "config" / "prices")
    git(root, "init", "-q", "-b", "main")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "the benchmark tree")
    return {"root": root, "bench": bench, "lock": bench / "experiments" / "prereg.lock.json",
            "spec": spec_file(tmp_path / "S09.md")}


def test_the_lock_builds_deterministically_and_check_finds_every_drift(repo):
    built = lock.build(repo["spec"], repo["lock"])
    assert built == lock.build(repo["spec"], repo["lock"])  # same commit, same spec: the same lock
    head = git(repo["root"], "rev-parse", "HEAD")
    assert (built["analysis_commit"], built["harness_sha"], built["locked_at"]) == (head, head,
                                                                                   "2026-10-03T08:00:00+00:00")
    assert built["schema_version"] == "vb.prereg_lock/1"
    assert built["prereg_id"] == f"s09-v1.6-{built['spec_sha256'][:8]}"
    assert built["price_snapshot_id"] == "prices-2026-09-28" and built["multiplicity"] == holm.MULTIPLICITY
    assert built["primaries"] == PLAN["primaries"] and built["alpha_fw"] == 0.05
    hashes = built["hashes"]
    assert "analysis/lock.py" in hashes and "audit/estimate.py" in hashes and "streams/p1_core.toml" in hashes
    assert "requirements-analysis.lock" in hashes and "config/prices/2026-09-28.toml" in hashes
    assert not any("__pycache__" in name for name in hashes)
    lock.write(built, repo["lock"])
    first = repo["lock"].read_bytes()
    lock.write(lock.build(repo["spec"], repo["lock"]), repo["lock"])
    assert repo["lock"].read_bytes() == first and lock.check(repo["lock"], repo["spec"]) == []
    # Drift: an analysis file edited, a stream added, a family changed, another spec text.
    (repo["bench"] / "analysis" / "holm.py").write_text("# edited after the lock\n", encoding="utf-8")
    (repo["bench"] / "streams" / "extra.toml").write_text("x = 1\n", encoding="utf-8")
    (repo["bench"] / "families" / "common" / "canary.py").write_text("# edited\n", encoding="utf-8")
    drift = lock.check(repo["lock"], spec_file(repo["spec"], {**PLAN, "alpha_fw": 0.1}))
    assert "analysis/holm.py: changed since the lock" in drift and "streams/extra.toml: added since the lock" in drift
    assert "suite_hash: the families changed since the lock" in drift
    assert any(item.startswith("spec_sha256") for item in drift) and any(item.startswith("alpha_fw") for item in drift)


def test_the_lock_refuses_a_dirty_tree_and_a_plan_that_is_not_holms(repo):
    (repo["bench"] / "analysis" / "envelope.py").write_text("# uncommitted\n", encoding="utf-8")
    with pytest.raises(lock.LockError, match="uncommitted changes"):
        lock.build(repo["spec"], repo["lock"])
    assert lock.build(repo["spec"], repo["lock"], allow_dirty=True)["schema_version"] == "vb.prereg_lock/1"
    git(repo["root"], "checkout", "-q", "--", ".")
    other = spec_file(repo["spec"], {**PLAN, "multiplicity": {**holm.MULTIPLICITY, "recycle": "none"}})
    with pytest.raises(lock.LockError, match="holm.MULTIPLICITY"):
        lock.build(other, repo["lock"])
    with pytest.raises(lock.LockError, match="no §5 lock block"):
        lock.build(spec_file(repo["spec"], {"schema_version": "vb.deviation/1"}), repo["lock"])


def test_a_lock_may_be_used_only_once_committed_and_clean(repo):
    with pytest.raises(lock.LockError, match="no pre-registration lock"):
        lock.require(repo["lock"], repo["spec"])
    lock.write(lock.build(repo["spec"], repo["lock"]), repo["lock"])
    assert lock.committed(repo["lock"]) is None
    with pytest.raises(lock.LockError, match="not committed"):
        lock.require(repo["lock"], repo["spec"])
    git(repo["root"], "add", "-A")
    git(repo["root"], "commit", "-q", "-m", "take the lock")
    assert lock.committed(repo["lock"]) == "2026-10-03T08:00:00+00:00"
    assert lock.require(repo["lock"], repo["spec"])["prereg_id"].startswith("s09-v1.6-")
    assert lock.main(["--check", str(repo["lock"]), "--spec", str(repo["spec"])]) == 0
    (repo["bench"] / "audit" / "estimate.py").write_text("# edited\n", encoding="utf-8")
    with pytest.raises(lock.LockError, match="fails --check"):
        lock.require(repo["lock"], repo["spec"])
    assert lock.main(["--check", str(repo["lock"]), "--spec", str(repo["spec"])]) == 1


def test_blinded_labels_unblind_only_under_the_lock(repo, tmp_path):
    salt = blind.new_salt(tmp_path / "config" / "blind-salt")
    assert salt.stat().st_mode & 0o777 == 0o600
    with pytest.raises(blind.BlindError):
        blind.new_salt(salt)  # a salt is never replaced
    blinder = blind.Blinder.from_file(salt)
    labels = {arm: blinder.label(arm) for arm in blind.ARMS}
    assert len(set(labels.values())) == len(blind.ARMS) and all(label.startswith("arm-") for label in labels.values())
    assert blinder.label("roko_full") == blind.Blinder.from_file(salt).label("roko_full")
    assert blind.Blinder(b"another salt, 16+").label("roko_full") != labels["roko_full"]
    assert "salt=<32 bytes>" in repr(blinder)
    with pytest.raises(blind.BlindError, match="waits for the pre-registration lock"):
        blinder.unblind(labels["roko_full"], lock_path=repo["lock"], spec=repo["spec"])
    lock.write(lock.build(repo["spec"], repo["lock"]), repo["lock"])
    git(repo["root"], "add", "-A")
    git(repo["root"], "commit", "-q", "-m", "take the lock")
    assert blinder.unblind(labels["roko_full"], lock_path=repo["lock"], spec=repo["spec"]) == "roko_full"
    assert blinder.key(lock_path=repo["lock"], spec=repo["spec"]) == {label: arm for arm, label in labels.items()}
    salt.chmod(0o644)
    with pytest.raises(blind.BlindError, match="mode 0600"):
        blind.Blinder.from_file(salt)
