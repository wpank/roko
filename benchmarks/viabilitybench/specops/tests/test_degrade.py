"""Tests for the spec-degradation operator D-v1 (3233, S07 §4.5).

They run on the golden precise specs of S07 §4.6 (F4, F2, F7, F1) under ``fixtures/``. Run from the repo root
with the benchmark venv:
``benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_degrade.py``.
"""

from __future__ import annotations

import copy
import re
import sys
import tomllib
from pathlib import Path

import pytest

SPECOPS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SPECOPS_DIR.parent))
from specops import LEVELS, OPERATOR, VAGUE, VERSION, degrade  # noqa: E402

FIXTURES = SPECOPS_DIR / "fixtures"
NAMES = ["f4_kvtool", "f2_api_pager", "f7_rust_iter", "f1_pyconv_errors"]
PROSE = ("title", "goal", "description", "prompt", "acceptance", "non_goals")


def load(name: str) -> dict:
    return tomllib.loads((FIXTURES / f"{name}.toml").read_text(encoding="utf-8"))["task"][0]


def prose(spec: dict) -> str:
    parts = []
    for name in PROSE:
        value = spec.get(name)
        parts.extend(value if isinstance(value, list) else [value] if isinstance(value, str) else [])
    return "\n".join(parts)


@pytest.mark.parametrize("name", NAMES)
def test_degrade_is_idempotent(name):
    once, _ = degrade(load(name), VAGUE, seed=7)
    twice, manifest = degrade(once, VAGUE, seed=7)
    assert twice == once
    assert not manifest.changed(), manifest.record()


@pytest.mark.parametrize("name", NAMES)
def test_degrade_is_deterministic_per_seed(name):
    spec = load(name)
    first, first_manifest = degrade(spec, VAGUE, seed=7)
    again, again_manifest = degrade(spec, VAGUE, seed=7)
    assert again == first
    assert again_manifest.record() == first_manifest.record()
    other, _ = degrade(spec, VAGUE, seed=8)
    assert other != first, "a different seed draws different lexicon terms or positions"
    assert spec == load(name), "the input spec is never modified"


@pytest.mark.parametrize("name", NAMES)
def test_manifest_lists_every_removed_or_replaced_item(name):
    spec = load(name)
    vague, manifest = degrade(spec, VAGUE, seed=7)

    def items(level: str, field: str) -> list:
        return [entry["item"] for entry in manifest.removed if entry["level"] == level and entry["field"] == field]

    # L1: every verify step is gone from the spec and kept for scoring.
    assert "verify" not in vague
    assert items("L1", "verify") == spec["verify"]
    assert manifest.hidden_verify == spec["verify"]
    # L2: every acceptance item, and the sentences that name the criteria.
    assert "acceptance" not in vague
    assert items("L2", "acceptance") == spec["acceptance"]
    assert any("AC ids refer" in item for item in items("L2", "description"))
    assert not re.search(r"\bAC\d*\b", prose(vague))
    # L3: every context entry and convention pointer.
    context = spec.get("context", {})
    assert items("L3", "context.read_files") == context.get("read_files", [])
    assert items("L3", "context.symbols") == context.get("symbols", [])
    assert "context" not in vague
    assert not re.search(r"\bsee\b|--help|CONTRIBUTING", prose(vague), re.IGNORECASE)
    # L4: every anchor that survived L1-L3 is replaced, and none is left.
    after_l3, _ = degrade(spec, ["L1", "L2", "L3"], seed=7)
    anchors = re.findall(r"`[^`]+`", prose(after_l3))
    assert sorted(entry["before"] for entry in manifest.replaced if entry["before"].startswith("`")) == sorted(anchors)
    assert "`" not in prose(vague) and "::" not in prose(vague)
    # L5: k = 3 lexicon terms, each listed with its position.
    assert len(manifest.inserted) == 3
    words = vague["description"].split()
    for entry in manifest.inserted:
        assert entry["level"] == "L5" and entry["field"] == "description"
        assert " ".join(words[entry["position"] : entry["position"] + len(entry["term"].split())]) == entry["term"]


def test_dose_response_levels_apply_in_order():
    # F7 keeps anchors past L3, so every level changes it.
    spec = load("f7_rust_iter")
    previous = spec
    for count in range(1, len(VAGUE) + 1):
        levels = list(VAGUE[:count])
        degraded, manifest = degrade(spec, list(reversed(levels)), seed=3)
        assert manifest.levels == levels
        assert {entry["level"] for entry in manifest.removed + manifest.replaced + manifest.inserted} == set(levels)
        assert degraded != previous
        previous = degraded


def test_manifest_record_names_the_operator():
    _, manifest = degrade(load("f7_rust_iter"), VAGUE, seed=11)
    record = manifest.record()
    assert record["ev"] == "spec.degraded"
    assert (record["operator"], record["operator_version"]) == (OPERATOR, VERSION) == ("D-v1", "1")
    assert record["levels"] == list(VAGUE)
    assert (record["seed"], record["task_id"]) == (11, "F7-0003")


def test_l6_is_off_and_unknown_levels_are_refused():
    assert "L6" in LEVELS
    with pytest.raises(ValueError, match="L6"):
        degrade(load("f1_pyconv_errors"), ["L1", "L6"], seed=1)
    with pytest.raises(ValueError, match="unknown level"):
        degrade(load("f1_pyconv_errors"), ["L9"], seed=1)


def test_hidden_suite_and_scope_are_left_for_scoring():
    spec = load("f2_api_pager")
    vague, _ = degrade(spec, VAGUE, seed=5)
    assert vague["hidden"] == spec["hidden"]
    assert vague["files"] == spec["files"]
    assert copy.deepcopy(vague) == vague
