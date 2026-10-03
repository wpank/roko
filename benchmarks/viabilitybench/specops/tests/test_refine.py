"""Tests for the spec refiner R-v1 (S07 §4.3; task 3235): additive-only rounds against a stub model, over the
plan corpus and the 3233 fixtures. No model is called; `Proposal`/`Addition` are built by hand or by hypothesis.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_refine.py
"""

from __future__ import annotations

import copy
import sys
import tomllib
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

SPECOPS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SPECOPS_DIR.parent))
sys.path.insert(0, str(SPECOPS_DIR.parent / "speclint"))
import speclint  # noqa: E402
from specops import refine  # noqa: E402

FIXTURES = SPECOPS_DIR / "fixtures"
NAMES = ["f4_kvtool", "f2_api_pager", "f7_rust_iter", "f1_pyconv_errors"]


def load(name: str) -> dict:
    return tomllib.loads((FIXTURES / f"{name}.toml").read_text(encoding="utf-8"))["task"][0]


def context(spec: dict) -> speclint.PlanContext:
    return speclint.PlanContext(workspace=speclint.Workspace(Path(".")), plan_id="test", plan_path="x.toml",
                                archived=False, tasks_by_id={spec["id"]: spec}, plan_outputs={})


def stub(*rounds: list[refine.Addition]) -> refine.Model:
    """A model that proposes `rounds[0]` on round 1, `rounds[1]` on round 2, and nothing after."""
    def model(spec: dict, round_number: int) -> refine.Proposal:
        additions = rounds[round_number - 1] if round_number <= len(rounds) else []
        return refine.Proposal(model="stub", additions=additions, cost_usd=0.001 if additions else 0.0)
    return model


@pytest.mark.parametrize("name", NAMES)
def test_a_well_formed_round_only_adds_and_can_raise_the_score(name):
    spec = load(name)
    before = speclint.score_task(spec, context(spec))["score"]
    new_ac = f"AC{len(spec.get('acceptance', [])) + 1}: a bonus, independently checkable criterion"
    round_one = [
        refine.Addition("acceptance", new_ac, source="PRD line 1"),
        refine.Addition("verify", {"phase": "test", "command": "python3 -m unittest tests.visible.test_bonus",
                                   "covers": [f"AC{len(spec.get('acceptance', [])) + 1}"]}, source="PRD line 1"),
    ]
    result = refine.refine(spec, stub(round_one), context(spec), max_rounds=1)
    assert len(result.rounds) == 1 and result.stopped == "rounds_exhausted"
    assert result.spec["acceptance"][-1] == new_ac
    assert result.spec["verify"][: len(spec.get("verify") or [])] == (spec.get("verify") or [])  # untouched prefix
    assert result.rounds[0].after_score >= before
    assert result.cost_usd == pytest.approx(0.001)
    assert spec == load(name), "the input spec is never modified"


@pytest.mark.parametrize("name", NAMES)
def test_refiner_never_deletes_or_weakens_a_verify_step(name):
    spec = load(name)
    if not spec.get("verify"):
        pytest.skip(f"{name} has no verify step to protect")
    # A model that proposes a verify "addition" is still rejected if the result does not keep the old steps: the
    # guard is exercised directly, since `refine`'s own `_append` can only extend a list, never replace one of it.
    before_verify = spec["verify"]
    tampered = copy.deepcopy(spec)
    tampered["verify"] = tampered["verify"][1:]  # as if a step had been dropped
    assert refine._rejection(spec, tampered, []) == "deleted, reordered or edited an existing verify step"
    tampered = copy.deepcopy(spec)
    tampered["verify"][0]["command"] = "true"  # as if a step had been weakened in place
    assert refine._rejection(spec, tampered, []) == "deleted, reordered or edited an existing verify step"
    # And through the public API: a round that only appends leaves the existing steps exactly as they were.
    result = refine.refine(spec, stub([refine.Addition("acceptance", "AC-extra: ok", source="s")]), context(spec))
    final_verify = result.spec.get("verify") or []
    assert final_verify[:len(before_verify)] == before_verify


@pytest.mark.parametrize("name", NAMES)
def test_a_round_that_would_lower_the_score_is_rejected(name):
    spec = load(name)
    # An acceptance item with nothing covering it strictly lowers SQ03 (traceability)'s denominator-only growth.
    bad_round = [refine.Addition("acceptance", "AC-uncovered: nothing checks this", source="s")]
    result = refine.refine(spec, stub(bad_round), context(spec))
    assert result.rounds == [] and result.stopped.startswith("round 1 would lower the score")
    assert result.spec == spec


def test_a_round_naming_a_hidden_suite_path_is_rejected():
    spec = load("f2_api_pager")
    for bad_path in (".vb/task.json", "tests/hidden/secret.py", "task.json"):
        bad = [refine.Addition("context.read_files", {"path": bad_path, "why": "peek"}, source="s")]
        result = refine.refine(spec, stub(bad), context(spec))
        assert result.rounds == [] and "hidden-suite path" in result.stopped


def test_an_addition_with_no_source_is_rejected():
    spec = load("f1_pyconv_errors")
    bad = [refine.Addition("non_goals", "Do not do the unrelated thing", source="   ")]
    result = refine.refine(spec, stub(bad), context(spec))
    assert result.rounds == [] and "no source" in result.stopped


def test_an_addition_to_a_non_additive_field_is_rejected():
    spec = load("f1_pyconv_errors")
    bad = [refine.Addition("files", ["extra.py"], source="s")]
    result = refine.refine(spec, stub(bad), context(spec))
    assert result.rounds == [] and result.stopped == "changed files"


def test_refinement_stops_at_at_most_two_rounds():
    spec = load("f2_api_pager")  # has an uncovered AC4, so a legitimate covering step keeps raising the score
    forever = [refine.Addition("verify", {"phase": "test", "command": f"python3 -m unittest tests.visible.test_x{i}",
                                          "covers": ["AC4"]}, source=f"PRD line {i}") for i in range(1, 6)]

    def greedy_model(spec: dict, round_number: int) -> refine.Proposal:
        return refine.Proposal(model="greedy", additions=[forever[round_number - 1]], cost_usd=0.001)

    result = refine.refine(spec, greedy_model, context(spec), max_rounds=refine.MAX_ROUNDS)
    assert len(result.rounds) == refine.MAX_ROUNDS == 2
    assert result.stopped == "rounds_exhausted"
    assert [r.round for r in result.rounds] == [1, 2]


def test_no_proposal_stops_refinement_with_no_rounds():
    spec = load("f1_pyconv_errors")
    result = refine.refine(spec, stub([]), context(spec))
    assert result.rounds == [] and result.stopped == "no_proposal" and not result.refined


def test_spec_hash_is_stable_and_order_independent_of_dict_construction():
    spec = load("f1_pyconv_errors")
    assert refine.spec_hash(spec) == refine.spec_hash(copy.deepcopy(spec))
    assert refine.spec_hash(spec) == refine.spec_hash(dict(reversed(list(spec.items()))))


# --- the monotonicity property test (3235's Plan item 4) -----------------------------------------------------

_SOURCE = st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122), min_size=1, max_size=12)
_TEXT = st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122) | st.just(" "), min_size=1, max_size=40)
_PROSE_ADDITION = st.builds(refine.Addition, field=st.sampled_from(("acceptance", "non_goals", "assumptions")),
                           item=_TEXT, source=_SOURCE)


@settings(max_examples=60, deadline=None)
@given(name=st.sampled_from(NAMES), additions=st.lists(_PROSE_ADDITION, max_size=3))
def test_an_accepted_round_never_lowers_the_score(name, additions):
    """Over the 3233 fixtures and arbitrary additive, sourced proposals: whatever `refine` accepts, it accepts
    because the score did not drop, and whatever it rejects, `spec` comes back unchanged (S07 §4.3's
    monotonicity: refinement is a one-way ratchet on the score, never a way to launder a weaker spec)."""
    spec = load(name)
    before = speclint.score_task(spec, context(spec))["score"]
    result = refine.refine(spec, stub(additions), context(spec), max_rounds=1)
    if result.rounds:
        assert result.rounds[0].after_score >= before
        assert result.rounds[0].before_score == before
    else:
        assert result.spec == spec
