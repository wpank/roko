#!/usr/bin/env python3
"""Tests for the plan-level slice (gap-89f393). Offline; git and the venv's Python are the only tools they call.

The item's verify command runs the headline check, `test_plan_slice_reference_passes_and_stub_fails`, which lives in
`benchmarks/viabilitybench/tests/test_plan_slice.py`. This file checks the rest: every feature's shape, that each
one-task-short reference fails its hidden suite, a second seed, and the census's defences.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/plan_slice/test_slice.py
"""

from __future__ import annotations

import copy
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import slicekit  # noqa: E402
from slicekit import Task  # noqa: E402

sys.path.insert(0, str(HERE.parent))
from common import canary, repo  # noqa: E402

sys.path.insert(0, str(HERE.parents[1] / "schema"))
import validate  # noqa: E402

FEATURES = slicekit.load_features()
IDS = [feature.id for feature in FEATURES]


def instance(tmp_path: Path, feature: slicekit.Feature, seed: int = 1) -> tuple[dict, Path]:
    manifest = slicekit.materialize(feature, seed, tmp_path / "work", tmp_path / "private")
    return manifest, tmp_path / "work"


def visible_test_file(manifest: dict) -> str:
    return sorted(path for path in manifest["visible_test_hashes"] if Path(path).name.endswith("_smoke.py"))[0]


# --- shape ------------------------------------------------------------------------------------------------------


def test_the_slice_has_six_to_ten_valid_features():
    assert slicekit.FEATURES_MIN <= len(FEATURES) <= slicekit.FEATURES_MAX
    assert len(set(IDS)) == len(IDS)
    for feature in FEATURES:
        assert slicekit.shape_errors(feature) == [], feature.id
        assert slicekit.TASKS_MIN <= len(feature.tasks) <= slicekit.TASKS_MAX
        assert slicekit.width(feature.tasks) >= slicekit.WIDTH_MIN
        assert feature.source["suite"]["cross_family_review"] in ("pending", "done")


def test_width_is_the_largest_set_of_independent_tasks():
    def tasks(edges: dict[str, list[str]]) -> tuple[Task, ...]:
        return tuple(Task(id=name, title=name, files=(f"{name}.py",), depends_on=tuple(deps), covers=("R1",))
                     for name, deps in edges.items())
    assert slicekit.width(tasks({"a": [], "b": ["a"], "c": ["b"], "d": ["c"]})) == 1
    assert slicekit.width(tasks({"a": [], "b": ["a"], "c": ["a"], "d": ["b", "c"]})) == 2
    assert slicekit.width(tasks({"a": [], "b": [], "c": [], "d": ["a", "b", "c"]})) == 3
    # b depends on a only through c, so a and b are not independent
    assert slicekit.width(tasks({"a": [], "c": ["a"], "b": ["c"], "x": []})) == 2


def test_shape_errors_catch_a_narrow_or_overlapping_plan(tmp_path):
    feature = FEATURES[0]
    narrow = [Task(t.id, t.title, t.files, tuple(u.id for u in feature.tasks[:i]), t.covers)
              for i, t in enumerate(feature.tasks)]
    errors = slicekit.shape_errors(slicekit.Feature(feature.root, feature.source, tuple(narrow), feature.requirements))
    assert any("in parallel" in error for error in errors)
    shared = [Task(t.id, t.title, feature.tasks[0].files, t.depends_on, t.covers) for t in feature.tasks]
    errors = slicekit.shape_errors(slicekit.Feature(feature.root, feature.source, tuple(shared), feature.requirements))
    assert any("same file" in error for error in errors)


@pytest.mark.parametrize("feature", FEATURES, ids=IDS)
def test_run_record_task_values_make_valid_run_records(feature, tmp_path):
    manifest, _ = instance(tmp_path, feature, seed=1)
    assert manifest["run_record_task"] == slicekit.run_record_task(feature, 1)
    assert manifest["run_record_task"]["instance_id"] == manifest["instance_id"] == f"{feature.id}-0001"
    example = json.loads((HERE.parents[1] / "schema" / "examples" / "run-record.json").read_text())
    for arm in ("roko_plan", "fd_claude"):  # S09 §4.9's two arms
        record = copy.deepcopy(example) | {"arm": arm, "task": manifest["run_record_task"]}
        assert validate.validate("run-record", record) == [], arm


def test_run_record_tasks_carry_a_null_ladder():
    # A feature has no difficulty level: its rows say so with ladder = null, never with a placeholder level.
    for feature in FEATURES:
        task = slicekit.run_record_task(feature, 1)
        assert task["ladder"] is None and task["family"] == "PL" and slicekit.run_record_errors(task) == []
    assert len(slicekit.run_record_errors(task | {"ladder": "plan", "spec_variant": "plan-slice"})) == 2
    placeholder = slicekit.Feature(FEATURES[0].root, FEATURES[0].source | {"run_record": {"ladder": 5}},
                                   FEATURES[0].tasks, FEATURES[0].requirements)
    assert any("sets ladder" in error for error in slicekit.shape_errors(placeholder))


def test_every_requirement_is_stated_planned_and_tested():
    for feature in FEATURES:
        planned = {req for task in feature.tasks for req in task.covers}
        tested = {req for reqs in feature.hidden_tags().values() for req in reqs}
        assert set(feature.requirements) == planned == tested, feature.id


# --- verifier CI beyond the headline --------------------------------------------------------------------------


@pytest.mark.parametrize("feature", FEATURES, ids=IDS)
def test_one_task_short_fails_the_hidden_suite(feature):
    row = slicekit.selftest(feature, seed=1, partials=True)
    assert row["ok"], row
    assert sorted(row["one_task_short"]) == sorted(task.id for task in feature.tasks)
    assert not any(result["hidden"] for result in row["one_task_short"].values()), row


@pytest.mark.parametrize("feature", FEATURES, ids=IDS)
def test_a_second_seed_renames_the_package_and_still_verifies(feature, tmp_path):
    manifest, work = instance(tmp_path, feature, seed=2)
    original = feature.source["package"]
    assert manifest["package"] in feature.source["rename"][original]
    assert (work / manifest["package"]).is_dir() and not (work / original).exists()
    word = re.compile(rf"(?<![A-Za-z0-9_]){re.escape(original)}(?![A-Za-z0-9_])")
    assert not [path for path in work.rglob("*") if path.is_file() and ".git" not in path.parts
                and word.search(path.read_text(encoding="utf-8"))]
    assert word.search((tmp_path / "private" / "description.md").read_text()) is None
    slicekit.apply_reference(feature, work, manifest["rename"])
    assert slicekit.census(manifest, work)["verified"]


# --- isolation, canaries and gaming -----------------------------------------------------------------------------


def test_the_workdir_holds_no_canary_and_the_manifest_stays_private(tmp_path):
    manifest, work = instance(tmp_path, FEATURES[0])
    assert canary.find_in_tree(work) == {}
    assert json.loads((tmp_path / "private" / "feature.json").read_text())["canary"] == canary.RELEASE_CANARY
    names = {path.name for path in work.rglob("*")}
    assert not names & {"feature.json", "feature.toml", "plan.toml", "description.md", "pristine.bundle"}
    hidden = {path.name for path in (FEATURES[0].root / "hidden").iterdir() if path.name != "__init__.py"}
    assert hidden and not names & hidden
    assert manifest["pristine"]["bundle"].startswith(str(tmp_path / "private"))


def test_the_manifest_matches_its_schema(tmp_path):
    manifest, _ = instance(tmp_path, FEATURES[1])
    assert slicekit.manifest_errors(manifest) == []
    del manifest["canary"]
    manifest["family"] = "F1"
    assert len(slicekit.manifest_errors(manifest)) == 2


def test_a_private_directory_inside_the_workdir_is_refused(tmp_path):
    with pytest.raises(slicekit.SliceError):
        slicekit.materialize(FEATURES[0], 1, tmp_path / "work", tmp_path / "work" / "private")
    with pytest.raises(slicekit.SliceError):
        slicekit.materialize(FEATURES[0], 1, tmp_path / "private" / "work", tmp_path / "private")


def test_an_edited_visible_test_is_flagged_and_restored(tmp_path):
    feature = FEATURES[2]
    manifest, work = instance(tmp_path, feature)
    smoke = work / visible_test_file(manifest)
    smoke.write_text("import unittest\n\n\n@unittest.skip('later')\nclass Nothing(unittest.TestCase):\n"
                     "    def test_nothing(self):\n        pass\n")
    verdict = slicekit.census(manifest, work)
    assert verdict["gaming"]["test_edit"] and verdict["gaming"]["tests_skipped"]
    assert not verdict["visible"]["passed"] and not verdict["passed"] and not verdict["verified"]
    slicekit.apply_reference(feature, work, manifest["rename"])
    verdict = slicekit.census(manifest, work)
    assert verdict["passed"] and verdict["visible"]["passed"] and not verdict["verified"] and not verdict["vf"]
    assert repo.tree_hash(work) != manifest["pristine"]["tree"]


def test_a_regression_in_the_base_fails_vf_although_the_hidden_suite_passes(tmp_path):
    # S09 §4.9 (v1.3): the base's visible tests are part of the verified feature. Here the feature works, but the
    # base's existing `movements` command now miscounts, which only the base's own visible test notices.
    feature = FEATURES[0]
    manifest, work = instance(tmp_path, feature)
    slicekit.apply_reference(feature, work, manifest["rename"])
    cli = work / manifest["package"] / "cli.py"
    good = 'print(len(read_ledger(args.ledger.read_text(encoding="utf-8"))))'
    assert cli.read_text().count(good) == 1
    cli.write_text(cli.read_text().replace(good, good[:-1] + " + 1)"))
    verdict = slicekit.census(manifest, work)
    assert verdict["passed"] and not verdict["visible"]["passed"]
    assert not verdict["gaming"]["test_edit"] and not verdict["vf"] and not verdict["verified"]


def test_a_canary_in_the_workdir_blocks_verification(tmp_path):
    feature = FEATURES[3]
    manifest, work = instance(tmp_path, feature)
    slicekit.apply_reference(feature, work, manifest["rename"])
    (work / "NOTES.md").write_text(f"copied from the benchmark: {canary.RELEASE_CANARY}\n")
    verdict = slicekit.census(manifest, work)
    assert verdict["passed"] and verdict["canary_hits"] == {"NOTES.md": [canary.RELEASE_CANARY]}
    assert verdict["leak_suspected"] and verdict["vf"] and not verdict["verified"]


def test_planted_modules_cannot_take_over_the_runner(tmp_path):
    manifest, work = instance(tmp_path, FEATURES[4])
    always_pass = ("import sys\nclass TestResult:\n    pass\nsys.exit(0)\n")
    for name in ("unittest.py", "sitecustomize.py", "json.py"):
        (work / name).write_text(always_pass)
    verdict = slicekit.census(manifest, work)
    assert verdict["checks"] and not verdict["passed"] and not verdict["visible"]["passed"]


def test_the_census_is_repeatable(tmp_path):
    feature = FEATURES[5]
    manifest, work = instance(tmp_path, feature)
    slicekit.apply_reference(feature, work, manifest["rename"])
    first, second = slicekit.census(manifest, work), slicekit.census(manifest, work)
    assert first == second and first["verified"]
    assert {req for check in first["checks"] for req in check["reqs"]} == set(manifest["requirements"])


def test_the_same_seed_gives_the_same_task_repo(tmp_path):
    feature = FEATURES[0]
    one = slicekit.materialize(feature, 7, tmp_path / "a" / "work", tmp_path / "a" / "private")
    two = slicekit.materialize(feature, 7, tmp_path / "b" / "work", tmp_path / "b" / "private")
    assert (one["pristine"]["commit"], one["pristine"]["tree"]) == (two["pristine"]["commit"], two["pristine"]["tree"])
    assert one["description"]["sha256"] == two["description"]["sha256"]
    other = [slicekit.materialize(feature, seed, tmp_path / str(seed) / "work", tmp_path / str(seed) / "private")
             for seed in (8, 9, 10)]
    assert len({manifest["pristine"]["tree"] for manifest in [one, *other]}) > 1


def test_the_command_line(tmp_path):
    script = str(HERE / "slicekit.py")
    check = subprocess.run([sys.executable, script, "check"], capture_output=True, text=True)
    assert check.returncode == 0 and check.stdout.startswith(f"ok: {len(FEATURES)} features"), check.stdout
    made = subprocess.run([sys.executable, script, "materialize", "--feature", "PL02", "--seed", "3", "--workdir",
                           str(tmp_path / "work"), "--private", str(tmp_path / "private")],
                          capture_output=True, text=True)
    assert made.returncode == 0 and json.loads(made.stdout)["instance_id"] == "PL02-0003"
    census = subprocess.run([sys.executable, script, "census", "--manifest", str(tmp_path / "private" / "feature.json"),
                             "--workdir", str(tmp_path / "work")], capture_output=True, text=True)
    assert census.returncode == 1 and json.loads(census.stdout)["passed"] is False
    shutil.rmtree(tmp_path / "work")
