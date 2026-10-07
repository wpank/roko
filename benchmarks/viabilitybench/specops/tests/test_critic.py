"""Tests for the LLM critic and the ambiguity probe (S07.5, §4.2; task 3237): both are off by default, both are
advisory, and neither ever makes a network call on its own. Run:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_critic.py
"""

from __future__ import annotations

import socket
import sys
import tomllib
from pathlib import Path

import pytest

SPECOPS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SPECOPS_DIR.parent))
sys.path.insert(0, str(SPECOPS_DIR.parent / "speclint"))
import speclint  # noqa: E402
from specops import ambiguity, critic  # noqa: E402

FIXTURES = SPECOPS_DIR / "fixtures"


def load(name: str) -> dict:
    return tomllib.loads((FIXTURES / f"{name}.toml").read_text(encoding="utf-8"))["task"][0]


def _no_network(monkeypatch):
    """Fail the test the instant anything opens a socket, so "no network call" is enforced, not assumed."""
    def forbidden(*args, **kwargs):
        raise AssertionError("a socket was opened; the default run must make no network call")

    monkeypatch.setattr(socket, "socket", forbidden)
    monkeypatch.setattr(socket, "create_connection", forbidden)


@pytest.mark.parametrize("name", ["f1_pyconv_errors", "f2_api_pager"])
def test_default_run_makes_no_network_call(name, monkeypatch, tmp_path):
    """speclint.score_task, without --critic or --ambiguity-probe, never touches the network and always scores
    critic and ambiguity null -- the CLI flags do not exist as a code path that could call out by accident."""
    _no_network(monkeypatch)
    spec = load(name)
    ctx = speclint.PlanContext(workspace=speclint.Workspace(tmp_path), plan_id="t", plan_path="x.toml",
                               archived=False, tasks_by_id={spec["id"]: spec}, plan_outputs={})
    record = speclint.score_task(spec, ctx)
    assert record["critic"] is None and record["ambiguity"] is None
    score_again = speclint.score_task(spec, ctx)["score"]
    assert score_again == record["score"]  # nothing about scoring depends on the (unset) flags


def test_critic_and_ambiguity_flags_refuse_without_a_wired_model(tmp_path):
    """The CLI surface exists (specops' Where), but a live model is S09 block C, not this task: passing either
    flag refuses clearly instead of guessing an answer or attempting a call."""
    plan = tmp_path / "tasks.toml"
    plan.write_text('[[task]]\nid = "T-1"\ntitle = "x"\n', encoding="utf-8")
    for flag in ("--critic", "--ambiguity-probe"):
        with pytest.raises(SystemExit) as excinfo:
            speclint.main([str(plan), flag])
        assert excinfo.value.code == 2


def _always(answer: bool):
    def model(spec: dict, question: str) -> tuple[bool, float]:
        return answer, 0.0005
    return model


def test_critique_asks_the_fixed_four_questions_and_scores_the_fraction_of_yes():
    spec = load("f1_pyconv_errors")
    unchanged = dict(spec)
    result = critic.critique(spec, _always(True))
    assert (result.critic_score, result.n_questions) == (1.0, 4)
    assert result.answers == [True, True, True, True]
    assert result.cost_usd == pytest.approx(0.002, abs=1e-9)  # S07.5's "about $0.002 per task"
    assert spec == unchanged

    mixed_model = iter([True, False, True, False])

    def alternating(spec: dict, question: str) -> tuple[bool, float]:
        return next(mixed_model), 0.0005

    result = critic.critique(spec, alternating)
    assert result.critic_score == 0.5
    assert result.as_json() == {"critic_score": 0.5, "n_questions": 4}


def test_critique_asks_every_question_once_in_the_fixed_order():
    asked = []

    def recording(spec: dict, question: str) -> tuple[bool, float]:
        asked.append(question)
        return True, 0.0

    critic.critique(load("f2_api_pager"), recording)
    assert asked == list(critic.QUESTIONS)
    assert len(critic.QUESTIONS) == 4


def test_ambiguity_is_zero_when_every_sample_agrees():
    spec = load("f2_api_pager")
    unchanged = dict(spec)

    def consistent(spec: dict, sample_index: int) -> tuple[list[str], float]:
        return ["customer_id", "currency"], 0.0007

    result = ambiguity.probe(spec, consistent)
    assert result.ambiguity == 0.0
    assert result.cost_usd == pytest.approx(0.0021, abs=1e-9)
    assert result.as_json() == {"ambiguity": 0.0}
    assert spec == unchanged


def test_ambiguity_is_higher_when_samples_disagree():
    guesses = [["customer_id"], ["account_number"], ["user_id", "currency"]]

    def inconsistent(spec: dict, sample_index: int) -> tuple[list[str], float]:
        return guesses[sample_index], 0.0

    result = ambiguity.probe(load("f1_pyconv_errors"), inconsistent)
    assert 0.0 < result.ambiguity <= 1.0
    assert result.samples == guesses

    def disjoint(spec: dict, sample_index: int) -> tuple[list[str], float]:
        return [f"field_{sample_index}"], 0.0

    assert ambiguity.probe(load("f1_pyconv_errors"), disjoint).ambiguity == 1.0


def test_ambiguity_caps_each_sample_at_max_inputs_and_needs_at_least_one_sample():
    def six_fields(spec: dict, sample_index: int) -> tuple[list[str], float]:
        return [f"field_{i}" for i in range(6)], 0.0

    result = ambiguity.probe(load("f1_pyconv_errors"), six_fields, max_inputs=5)
    assert all(len(sample) == 5 for sample in result.samples)

    def one_field(spec: dict, sample_index: int) -> tuple[list[str], float]:
        return ["x"], 0.0

    assert ambiguity.probe(load("f1_pyconv_errors"), one_field, k=1).ambiguity == 0.0
    with pytest.raises(ValueError):
        ambiguity.probe(load("f1_pyconv_errors"), one_field, k=0)


def test_neither_feature_changes_score_or_band():
    spec = load("f4_kvtool")
    ctx = speclint.PlanContext(workspace=speclint.Workspace(Path(".")), plan_id="t", plan_path="x.toml",
                               archived=False, tasks_by_id={spec["id"]: spec}, plan_outputs={})
    before = speclint.score_task(spec, ctx)
    critic.critique(spec, _always(False))
    ambiguity.probe(spec, lambda spec, i: (["x"], 0.0))
    after = speclint.score_task(spec, ctx)
    assert (before["score"], before["band"]) == (after["score"], after["band"])
