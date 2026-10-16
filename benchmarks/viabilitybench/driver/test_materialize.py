"""Tests for materialize.py's vague spec variant (S08 §4.5, S07.3; task 3329): built from the precise spec on
first request, cached in the manifest, passing the manipulation check (task 3234) and the recoverability rule.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_materialize.py
"""

from __future__ import annotations

import json
import sys
import tomllib
from collections import Counter
from pathlib import Path

import pytest

DRIVER_DIR = Path(__file__).resolve().parent
VB_ROOT = DRIVER_DIR.parent
sys.path.insert(0, str(DRIVER_DIR))
sys.path.insert(0, str(VB_ROOT))
sys.path.insert(0, str(VB_ROOT / "speclint"))
import materialize  # noqa: E402
from specops import VAGUE, degrade, manipulation_check  # noqa: E402

H3_SAMPLE = [("f1_pyconv", "F1-l1-0001"), ("f1_pyconv", "F1-l5-0002"), ("f2_apipager", "F2-l1-0001"),
             ("f2_apipager", "F2-l5-0002"), ("f3_moneyround", "F3-l1-0001"), ("f3_moneyround", "F3-l5-0002"),
             ("f4_kvtool", "F4-l1-0001"), ("f4_kvtool", "F4-l3-0005"), ("f5_sqlmigrate", "F5-l1-0001"),
             ("f5_sqlmigrate", "F5-l5-0002"), ("f7_rustiter", "F7-l1-0001"),
             ("f7_rustiter", "F7-l5-0002")]  # one low and one high level of each H3 family
H3_VAGUE_STREAM = VB_ROOT / "streams" / "log1_p1_h3_vague.toml"  # LOG1 block C (S09 §4.3), all 48 H3 instances


def _materialize(tmp_path, family: str, instance_id: str, *, spec_variant: str = "precise", tag: str = ""):
    root = tmp_path / f"{family}-{instance_id}{tag}"
    return materialize.materialize(family_dir=VB_ROOT / "families" / family, instance_id=instance_id,
                                   workdir=root / "work", private_dir=root / "private", spec_variant=spec_variant)


@pytest.mark.parametrize("family,instance_id", H3_SAMPLE)
def test_h3_variants_pass_the_manipulation_check(tmp_path, family, instance_id):
    """Both variants exist, and the vague one scores at least 30 SQS points below the precise one (S07.3)."""
    precise = _materialize(tmp_path, family, instance_id, spec_variant="precise")
    task = materialize.task_dict(precise.manifest, precise.spec_text)
    report = manipulation_check.check_pair(instance_id, task, seed=precise.manifest["seed"], workspace_root=VB_ROOT)
    assert report.ok, report.as_json()
    assert report.gap >= manipulation_check.MIN_GAP

    vague = _materialize(tmp_path, family, instance_id, spec_variant="vague", tag="-vague")
    assert vague.spec_variant == "vague" and vague.spec_text  # the agent-facing text actually renders
    entry = vague.manifest["spec"]["vague"]
    assert (entry["operator"], entry["operator_version"]) == ("D-v1", "1")
    assert entry["levels"] == ["L1", "L2", "L3", "L4", "L5"]


@pytest.mark.parametrize("family,instance_id", H3_SAMPLE)
def test_the_vague_variant_is_cached_in_the_manifest(tmp_path, family, instance_id):
    root = tmp_path / f"{family}-{instance_id}"
    vague = materialize.materialize(family_dir=VB_ROOT / "families" / family, instance_id=instance_id,
                                    workdir=root / "work", private_dir=root / "private", spec_variant="vague")
    on_disk = json.loads(vague.manifest_path.read_text(encoding="utf-8"))
    assert on_disk["spec"]["vague"] == vague.manifest["spec"]["vague"]
    # A second materialize() of a *different* workdir, same private_dir's task.json, reads the cached file rather
    # than writing (and so refusing to overwrite) spec.vague.md again.
    reloaded = json.loads(vague.manifest_path.read_text(encoding="utf-8"))
    assert reloaded["spec"]["vague"]["sha256"] == vague.manifest["spec"]["vague"]["sha256"]


def test_every_h3_vague_instance_materializes(tmp_path):
    """bug-f90bd0: LOG1 block C runs all 48 H3 instances (8 each of F1-F5 and F7) under their vague spec. Every one
    materializes with spec_variant="vague"; its cached spec.vague.md is D-v1's full VAGUE composition of its own
    precise spec; no requirement the vague spec drops is left without evidence in the workdir (the recoverability
    rule, S08 §4.5); and the vague twin scores at least MIN_GAP below the precise spec (S07.3)."""
    stream = tomllib.loads(H3_VAGUE_STREAM.read_text(encoding="utf-8"))["stream"]
    instances = stream["instances"]
    assert stream["spec_variant"] == "vague" and len(instances) == 48
    assert Counter(instance_id.split("-")[0] for instance_id in instances) == dict.fromkeys(
        materialize.VAGUE_FAMILIES, 8)
    problems: list[str] = []
    for instance_id in instances:
        family_dir = VB_ROOT / stream["families"][instance_id.split("-")[0]]
        root = tmp_path / instance_id
        try:
            vague = materialize.materialize(family_dir=family_dir, instance_id=instance_id, workdir=root / "work",
                                            private_dir=root / "private", spec_variant="vague")
        except materialize.MaterializeError as err:
            problems.append(str(err))
            continue
        manifest = vague.manifest
        entry = manifest["spec"]["vague"]
        if (vague.spec_variant, entry["operator"], entry["operator_version"]) != ("vague", "D-v1", "1") \
                or entry["levels"] != list(VAGUE):
            problems.append(f"{instance_id}: the vague entry is {entry}")
        precise_text = (vague.manifest_path.parent / manifest["spec"]["precise"]["path"]).read_text(encoding="utf-8")
        task = materialize.task_dict(manifest, precise_text)
        if not (task["title"] and task["goal"] and task["acceptance"] and task["context"] and task["non_goals"]):
            problems.append(f"{instance_id}: the precise spec's sections did not parse: {task}")
        degraded, _manifest = degrade(task, VAGUE, manifest["seed"])
        if materialize.render_vague_markdown(degraded) != vague.spec_text:
            problems.append(f"{instance_id}: spec.vague.md is not D-v1's output for the precise spec")
        problems += [f"{instance_id}: {violation}" for violation in
                     materialize.recoverability_violations(manifest, degraded)]
        for requirement in manifest["recoverability"]:
            in_repo = [item for item in requirement["evidence"] if not item.startswith("spec.precise.md#")]
            missing = [item for item in in_repo if not (vague.workdir / item.split(" ")[0]).exists()]
            if not in_repo or missing:
                problems.append(f"{instance_id}: {requirement['req']!r} has no evidence in the workdir: {missing}")
        report = manipulation_check.check_pair(instance_id, task, seed=manifest["seed"], workspace_root=VB_ROOT)
        if not report.ok:
            problems.append(f"{instance_id}: manipulation check failed: gap {report.gap}")
    assert not problems, "\n".join(problems)


def test_a_family_with_no_vague_support_refuses_clearly(tmp_path):
    # A minimal manifest, not a real materialize() call: only H3's families have a vague variant, and F6 is in no
    # H3 stream, so this checks VAGUE_FAMILIES directly, the first thing _add_vague_spec does.
    manifest = {"instance_id": "F6-l1-0001", "family": "F6", "spec": {"precise": {"path": "x", "sha256": "x"}}}
    with pytest.raises(materialize.MaterializeError, match="F6 has no vague variant"):
        materialize._add_vague_spec(manifest, tmp_path / "task.json", tmp_path)


def test_task_dict_pulls_files_and_verify_from_the_manifest_not_the_prose():
    precise = {
        "instance_id": "F1-l1-0001", "files_in_scope": ["app/errors.py", "app/invoicing/refunds.py"],
        "visible_verify": ["python3 -m unittest discover -s tests/visible"],
    }
    text = ("# A title\n\n**Goal.** Fix the thing.\n\nSome more detail about it.\n\n## Acceptance criteria\n\n"
           "- AC1: it works\n\n## Context\n\n- `app/errors.py`: the base errors\n\n## Non-goals\n\n"
           "- Do not touch tests\n")
    task = materialize.task_dict(precise, text)
    assert task["files"] == precise["files_in_scope"]
    assert task["verify"] == [{"phase": "test", "command": precise["visible_verify"][0]}]
    assert task["title"] == "A title" and task["goal"] == "Fix the thing." and task["description"] == "Some more detail about it."
    assert task["acceptance"] == ["AC1: it works"]
    assert task["context"]["read_files"] == [{"path": "app/errors.py", "why": "the base errors"}]
    assert task["non_goals"] == ["Do not touch tests"]


def test_render_vague_markdown_omits_a_section_once_degrade_empties_its_field():
    full = materialize.render_vague_markdown({"title": "T", "goal": "G", "description": "D",
                                              "acceptance": ["AC1: x"], "verify": [{"command": "run"}],
                                              "context": {"read_files": [{"path": "a", "why": "w"}]},
                                              "non_goals": ["n"], "files": ["a"]})
    assert "## Acceptance criteria" in full and "## Verify" in full and "## Context" in full
    stripped = materialize.render_vague_markdown({"title": "T", "goal": "G", "files": ["a"]})
    assert "## Acceptance criteria" not in stripped and "## Verify" not in stripped and "## Context" not in stripped
    assert "## Scope" in stripped and "`a`" in stripped


def test_recoverability_violations_flags_only_spec_only_evidence_for_a_dropped_ac():
    manifest = {"recoverability": [
        {"req": "r1", "evidence": ["spec.precise.md#AC1"]},
        {"req": "r2", "evidence": ["spec.precise.md#AC2", "app/errors.py"]},
        {"req": "r3", "evidence": ["spec.precise.md#AC3"]},
    ]}
    vague_keeping_ac3 = {"acceptance": ["AC3: still here"]}
    problems = materialize.recoverability_violations(manifest, vague_keeping_ac3)
    assert len(problems) == 1 and problems[0].startswith("'r1'")

    vague_dropping_everything = {"acceptance": []}
    problems = materialize.recoverability_violations(manifest, vague_dropping_everything)
    assert {p.split("'")[1] for p in problems} == {"r1", "r3"}  # r2 has file evidence too, so it is never flagged
