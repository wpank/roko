"""Tests for the P1-ext selection and contamination probe (S08 §4.8; task 3332). Offline: a synthetic fixture
dataset stands in for SWE-bench Verified (`select.fetch_dataset`, the only network call in this package, is
never called here); the probe runs against stub models, never a live one (task 3353).

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/external/swebench/test_swebench.py
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import probe  # noqa: E402

select_mod = probe.select_mod  # select.py, loaded by path in probe.py (its own name clashes with the stdlib)

PATCH_ONE_FILE = ("diff --git a/pkg/a.py b/pkg/a.py\n--- a/pkg/a.py\n+++ b/pkg/a.py\n@@ -1,2 +1,3 @@\n"
                 " line one\n+line two\n line three\n")
PATCH_TWO_FILES = ("diff --git a/pkg/a.py b/pkg/a.py\n--- a/pkg/a.py\n+++ b/pkg/a.py\n@@ -1 +1 @@\n-x\n+y\n"
                   "diff --git a/pkg/b.py b/pkg/b.py\n--- a/pkg/b.py\n+++ b/pkg/b.py\n@@ -1 +1 @@\n-x\n+y\n")
PATCH_TOO_BIG = "diff --git a/pkg/a.py b/pkg/a.py\n--- a/pkg/a.py\n+++ b/pkg/a.py\n@@ -1,50 +1,0 @@\n" + \
    "".join(f"-line {i}\n" for i in range(50))
PATCH_TOO_MANY_FILES = "".join(f"diff --git a/pkg/f{i}.py b/pkg/f{i}.py\n--- a/pkg/f{i}.py\n+++ b/pkg/f{i}.py\n"
                               f"@@ -1 +1 @@\n-x\n+y\n" for i in range(3))

REPOS = [f"org/repo-{i}" for i in range(6)]  # >= MIN_REPOS, each with more than MAX_PER_REPO eligible


def _record(repo: str, index: int, *, difficulty="<15 min fix", fail_to_pass=None, patch=PATCH_ONE_FILE) -> dict:
    return {"instance_id": f"{repo}__{index}", "repo": repo, "difficulty": difficulty,
           "FAIL_TO_PASS": fail_to_pass if fail_to_pass is not None else ["tests/test_a.py::test_one"],
           "problem_statement": f"Issue {index} in {repo}: something is broken in pkg/a.py.", "patch": patch}


def _fixture_dataset() -> list[dict]:
    records = []
    for repo in REPOS:
        for index in range(15):  # 15 > MAX_PER_REPO (12), so the cap actually bites
            records.append(_record(repo, index))
    return records


def test_patch_files_and_changed_lines():
    assert select_mod.patch_files(PATCH_ONE_FILE) == ["pkg/a.py"]
    assert select_mod.patch_files(PATCH_TWO_FILES) == ["pkg/a.py", "pkg/b.py"]
    assert select_mod.patch_changed_lines(PATCH_ONE_FILE) == 1  # one "+line two"
    assert select_mod.patch_changed_lines(PATCH_TWO_FILES) == 4


def test_eligible_filters_difficulty_fail_to_pass_and_patch_shape():
    base = _record("org/x", 0)
    assert select_mod.eligible(base)
    assert not select_mod.eligible({**base, "difficulty": "1-4 hours"})
    assert not select_mod.eligible({**base, "FAIL_TO_PASS": [f"t{i}" for i in range(6)]})
    assert not select_mod.eligible({**base, "FAIL_TO_PASS": []})
    assert not select_mod.eligible({**base, "patch": PATCH_TOO_BIG})
    assert not select_mod.eligible({**base, "patch": PATCH_TOO_MANY_FILES})
    assert select_mod.eligible({**base, "patch": PATCH_TWO_FILES})  # 2 files is the cap, still eligible


def test_select_is_reproducible_and_caps_tasks_per_repo():
    records = _fixture_dataset()
    result = select_mod.select(records, seed=1, n=60, max_per_repo=12, min_repos=5)
    assert len(result.instances) == 60 and len(set(result.instance_ids)) == 60
    counts: dict[str, int] = {}
    for record in result.instances:
        counts[record["repo"]] = counts.get(record["repo"], 0) + 1
    assert all(count <= 12 for count in counts.values())
    assert len(counts) >= 5

    again = select_mod.select(records, seed=1, n=60, max_per_repo=12, min_repos=5)
    assert again.instance_ids == result.instance_ids
    different = select_mod.select(records, seed=2, n=60, max_per_repo=12, min_repos=5)
    assert different.instance_ids != result.instance_ids

    # The log covers every eligible record, not only the 60 selected, so the sample is auditable.
    assert len(result.log) == len(records)  # every fixture record here is eligible
    assert sum(1 for entry in result.log if entry["selected"]) == 60


def test_select_refuses_a_pool_with_too_few_repositories():
    records = [record for record in _fixture_dataset() if record["repo"] in REPOS[:3]]  # only 3 repos
    with pytest.raises(select_mod.SelectionError, match="at least 5"):
        select_mod.select(records, seed=1, n=60, max_per_repo=12, min_repos=5)


def test_select_refuses_a_pool_too_small_for_n():
    records = _fixture_dataset()[:10]
    with pytest.raises(select_mod.SelectionError):
        select_mod.select(records, seed=1, n=60, max_per_repo=12, min_repos=5)


def test_probe_never_sees_the_patch_test_patch_or_test_names():
    record = _record("org/x", 0)
    record["test_patch"] = "diff --git a/tests/test_a.py b/tests/test_a.py\n+++ secret test content"
    seen = []

    def recording_model(issue_text: str) -> list[str]:
        seen.append(issue_text)
        return ["pkg/a.py"]

    result = probe.probe_one(record, [recording_model])
    assert seen == [record["problem_statement"]]
    assert all("patch" not in text and "test_a.py::test_one" not in text for text in seen)
    assert result.agreeing == 1 and result.excluded is False  # 1 of 5 default agreement needs 2


def test_probe_excludes_at_the_agreement_threshold():
    record = _record("org/x", 0, patch=PATCH_ONE_FILE)  # gold files: {"pkg/a.py"}
    agreeing_model = lambda issue: ["pkg/a.py"]
    disagreeing_model = lambda issue: ["pkg/other.py"]

    one_agrees = probe.probe_one(record, [agreeing_model, disagreeing_model, disagreeing_model])
    assert one_agrees.agreeing == 1 and not one_agrees.excluded

    two_agree = probe.probe_one(record, [agreeing_model, agreeing_model, disagreeing_model])
    assert two_agree.agreeing == 2 and two_agree.excluded  # meets the default agreement of 2


def test_probe_many_reports_the_excluded_share():
    memorized = _record("org/x", 0, patch=PATCH_ONE_FILE)
    fresh = _record("org/x", 1, patch=PATCH_ONE_FILE)
    models = [lambda issue: ["pkg/a.py"], lambda issue: ["pkg/a.py"], lambda issue: []]
    # Both records get the same models here; a real probe draws per-task guesses, but the report's arithmetic
    # (the excluded share) does not care what made each ProbeResult true.
    report = probe.probe_many([memorized, fresh], models)
    assert report.excluded_share == 1.0  # both "excluded" under these always-agreeing-twice stub models
    assert report.as_json()["n"] == 2


def test_probe_main_refuses_without_a_live_model(tmp_path, capsys):
    import json

    record = _record("org/x", 0)
    dataset = tmp_path / "dataset.jsonl"
    dataset.write_text(json.dumps(record) + "\n", encoding="utf-8")
    selection = tmp_path / "selection.json"
    selection.write_text(json.dumps({"instance_ids": [record["instance_id"]]}), encoding="utf-8")
    assert probe.main(["--dataset", str(dataset), "--selection", str(selection)]) == 2
    assert "no live model is wired" in capsys.readouterr().err
