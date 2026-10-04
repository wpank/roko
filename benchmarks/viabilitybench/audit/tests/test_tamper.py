"""Tests for A1, the tamper diff (audit/tamper.py), a deliberate Python port of roko-gate's attempt_diff.rs.
audit/tests/test_battery.py exercises every A1 kind together from a real archived fixture
(audit/fixtures/tamper.json); this file adds a focused test per rule that fixture does not reach on its own.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_tamper.py
"""

from __future__ import annotations

import sys
from pathlib import Path

VB_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(VB_ROOT))
import audit  # noqa: E402
from audit import tamper  # noqa: E402


def _tree(root: Path, files: dict[str, str]) -> Path:
    for relpath, text in files.items():
        path = root / relpath
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return root


def test_rung_file_edited_in_the_same_diff_is_tamper(tmp_path):
    """gap-146d77, porting bug-d5d55f: an attempt that loosens the schema a workspace rung checks artefacts
    against, in the same diff as the artefact, tampers with that check, even when the task's files name the
    directory that holds both. Only a task whose files name the schema file itself may write it (mirrors
    attempt_diff.rs's own schema_file_edited_in_the_same_diff_is_tamper)."""
    base = _tree(tmp_path / "base", {"reports/report.schema": '{"required": ["title"]}\n'})
    final = _tree(tmp_path / "final", {"reports/q3.json": "{}\n", "reports/report.schema": "{}\n"})

    def tampering(files_in_scope: list[str]) -> list[tamper.Finding]:
        policy = tamper.Policy.from_manifest({"files_in_scope": files_in_scope,
                                              "rung_files": ["./reports/report.schema"]})  # named_path strips ./
        return tamper.check_trees(base, final, policy)

    for scope in (["reports/"], ["reports/*.json"], []):
        findings = tampering(scope)
        assert [finding.kind for finding in findings] == ["rung_file_edited"], (scope, findings)
        assert findings[0].path == "reports/report.schema"
        assert findings[0].detail == "a file a workspace rung checks the task's artefacts against was edited"

    # A task whose files name the schema file itself, not just its directory, may write it.
    assert tampering(["reports/q3.json", "reports/report.schema"]) == []


def test_rung_file_edited_does_not_also_trigger_an_unrelated_timeout_finding(tmp_path):
    """A rung-file finding must not flip `check_config` for the same change: `_check_findings` keeps the two
    concerns apart, so a non-test rung file whose text happens to set a timeout gets exactly the one finding, not a
    spurious timeout_edited beside it (`rung_file_edited` sits outside the verify-script/tasks-toml/gate-config
    group it is written next to, unlike those three, which is what `check_config` was gating)."""
    base = _tree(tmp_path / "base", {"reports/report.schema": "timeout = 30\n"})
    final = _tree(tmp_path / "final", {"reports/report.schema": "timeout = 9999\n"})
    findings = tamper.check_trees(base, final, tamper.Policy(rung_files=("reports/report.schema",)))
    assert [finding.kind for finding in findings] == ["rung_file_edited"]


def test_rung_files_defaults_to_empty_so_every_caller_before_gap_146d77_is_unaffected():
    """`Policy()` and `Policy.from_manifest({})` both default `rung_files` to `()`, so a manifest that names none
    (every `vb.task/1` task today) never gets a rung_file_edited finding, and `Policy.from_manifest`'s existing
    fields parse exactly as before."""
    assert tamper.Policy().rung_files == ()
    assert tamper.Policy.from_manifest({}).rung_files == ()
    assert tamper.Policy.from_manifest({"files_in_scope": ["a.py"]}) == tamper.Policy(files_in_scope=("a.py",))


def test_edited_rung_file_matches_the_old_path_of_a_rename_too():
    """`edited_rung_file` checks both `path` and `old_path` (attempt_diff.rs's own signature), so renaming a rung
    file away is caught at its old path, not silently missed because the new path is not one of `rung_files`."""
    policy = tamper.Policy(rung_files=("reports/report.schema",))
    assert policy.edited_rung_file("reports/report.schema", "reports/report.schema") == "reports/report.schema"
    assert policy.edited_rung_file("reports/renamed.schema", "reports/report.schema") == "reports/report.schema"
    assert policy.edited_rung_file("reports/other.json", "reports/other.json") is None
    assert policy.names_exactly("reports/report.schema") is False
    exempted = tamper.Policy(files_in_scope=("reports/report.schema",), rung_files=("reports/report.schema",))
    assert exempted.names_exactly("reports/report.schema") is True
    assert exempted.edited_rung_file("reports/report.schema", "reports/report.schema") is None
