"""Tests of planemit's ladder mode (3310): the cheap-model ladder of decision 3302 instead of one pinned model, with the
pinned mode unchanged byte for byte. The `real_roko` test runs `plan validate --strict --dag` on the emitted ladder
plan when there is a binary (`target/debug/roko`, or `$VB_TEST_ROKO_BIN`).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -q -rs
"""

from __future__ import annotations

import dataclasses
import subprocess
import tomllib
from pathlib import Path

import pytest

import agent_env
import layout
import ledger
import materialize
import planemit
from test_run_roko import REAL_ROKO, real_roko

SNAPSHOT = ledger.load_snapshot()
GOLDEN = layout.DRIVER_DIR / "testdata" / "planemit"
CASCADE = (("cheap", "gpt-oss-120b", "cerebras", "CEREBRAS_API_KEY"), ("mid", "glm-4.7", "zai", "ZAI_API_KEY"),
           ("strong", "gpt-5.4-mini", "openai", "OPENAI_API_KEY"))  # D11's three cheap rungs
ALLOW = tuple(model for _, model, _, _ in CASCADE)


def rungs(cascade=CASCADE) -> tuple[planemit.Rung, ...]:
    return tuple(planemit.Rung(name=name, model=model, provider=provider, base_url=f"http://127.0.0.1:9/{provider}",
                               api_key_env=key, price_row=SNAPSHOT.row(model)) for name, model, provider, key in cascade)


def pinned_spec() -> planemit.PlanSpec:
    """The spec of the golden files in testdata/planemit/, emitted before ladder mode existed."""
    return planemit.PlanSpec(
        key="F1-l1-0001.s1", spec_text=("# Implement `clamp`\n\nToy instance: F1-l1-0001\n\nKeep \"quotes\" and \\ "
                                        "backslashes.\n"),
        files=("calc/ops.py",), visible=("python3 -m unittest discover -s tests/visible",), model="gpt-oss-120b",
        provider="cerebras", base_url="http://127.0.0.1:9/v1", api_key_env="CEREBRAS_API_KEY",
        price_row=SNAPSHOT.row("gpt-oss-120b"), usd_cap=0.43)


def ladder_spec(**changes: object) -> planemit.PlanSpec:
    return dataclasses.replace(pinned_spec(), **{"rungs": rungs(), "start": "cheap", "allow": ALLOW, "usd_cap": 0.6,
                                                 **changes})


def workspace(tmp_path: Path, name: str = "ws") -> Path:
    path = tmp_path / name
    path.mkdir()
    return path


def test_ladder_mode_emits_rungs_without_fallbacks(tmp_path):
    emitted = planemit.emit(ladder_spec(), workspace(tmp_path))
    config = tomllib.loads(emitted.config_path.read_text())
    plan = tomllib.loads(emitted.tasks_path.read_text())
    assert list(config["providers"]) == ["cerebras", "zai", "openai"]
    assert {name: table["base_url"] for name, table in config["providers"].items()} == {
        "cerebras": "http://127.0.0.1:9/cerebras", "zai": "http://127.0.0.1:9/zai",
        "openai": "http://127.0.0.1:9/openai"}
    assert list(config["models"]) == list(ALLOW)  # each model's key is its slug: "glm-4.7" stays one key
    for model, table in config["models"].items():
        row = SNAPSHOT.row(model)
        assert (table["slug"], table["cost_input_per_m"], table["cost_output_per_m"]) == (model, row["input"],
                                                                                          row["output"])
    assert config["routing"] == {"fallback_models": [], "ladder": {
        "enabled": True, "rungs": [{"name": name, "model": model} for name, model, _, _ in CASCADE],
        "start": dict.fromkeys(planemit.TIERS, "cheap")}}
    assert not set(config["models"]) & planemit.FRONTIER_MODELS and config["agent"]["default_model"] == "gpt-oss-120b"
    [task] = plan["task"]
    assert "model_hint" not in task and task["tier"] == "focused"  # the ladder picks the model; PLAN_041 has no pin
    # Everything else is the pinned mode's: one visible check, the shared tree, learning off, the dollar cap.
    assert [rung["command"] for rung in config["gates"]["rungs"]] == [step["command"] for step in task["verify"]] == [
        "python3 -m unittest discover -s tests/visible"]
    assert config["runner"] == {"worktree_per_task": False} and config["budget"]["max_plan_usd"] == 0.6
    assert config["conductor"] == {"max_agents": 1}
    assert planemit.LADDER_VERSION in emitted.config_text and planemit.LADDER_TEMPLATE_SHA256 != \
        planemit.TEMPLATE_SHA256
    # A start further up the ladder is the arm's choice too.
    mid = planemit.emit(ladder_spec(start="mid", model="glm-4.7"), workspace(tmp_path, "mid"))
    assert tomllib.loads(mid.config_text)["routing"]["ladder"]["start"] == dict.fromkeys(planemit.TIERS, "mid")


def test_pinned_mode_is_byte_identical_to_before_ladder_mode(tmp_path):
    emitted = planemit.emit(pinned_spec(), workspace(tmp_path))
    assert emitted.tasks_text == (GOLDEN / "pinned.tasks.toml").read_text()
    assert emitted.config_text == (GOLDEN / "pinned.roko.toml").read_text()
    assert planemit.TEMPLATE_VERSION == "planemit-4"
    assert planemit.TEMPLATE_SHA256 == "24d5db33665a3537449708ff920e53ec0e182d9fd77c411c1feefd1eae4b8007"


@pytest.mark.parametrize(("changes", "error"), [
    ({"rungs": rungs((*CASCADE, ("top", "claude-sonnet-5", "anthropic", "ANTHROPIC_API_KEY")))},
     "no frontier model, not claude-sonnet-5"),
    ({"rungs": rungs((CASCADE[0], ("strong", "gpt-5.4", "openai", "OPENAI_API_KEY")))}, "not gpt-5.4"),
    ({"rungs": rungs((CASCADE[1], CASCADE[0]))}, "not in the order of the arm's models_allow"),
    ({"allow": ALLOW[:2]}, "gpt-5.4-mini are not in the arm's models_allow"),
    ({"rungs": rungs((CASCADE[0], CASCADE[0]))}, "a model and a name of its own"),
    ({"start": "top"}, "the start rung 'top' must be a rung"),
    ({"start": "mid"}, "and its model 'gpt-oss-120b'"),
    ({"rungs": rungs((("cheap", "gpt-oss-120b", "cere bras", "CEREBRAS_API_KEY"),))}, "cannot be a roko.toml key"),
], ids=["frontier-rung", "frontier-gpt-5.4", "out-of-order", "outside-allowlist", "repeated-rung", "unknown-start",
        "start-model-mismatch", "bad-key"])
def test_ladder_mode_refuses_a_frontier_rung_and_a_bad_ladder(tmp_path, changes, error):
    with pytest.raises(planemit.PlanEmitError, match=error):
        planemit.emit(ladder_spec(**changes), workspace(tmp_path))
    assert not (tmp_path / "ws" / "roko.toml").exists()  # refused before anything is written


@real_roko
def test_real_roko_validates_the_ladder_plan(tmp_path):
    task = materialize.materialize(family_dir=layout.DRIVER_DIR / "testdata" / "toy_family", instance_id="F1-l1-0001",
                                   workdir=tmp_path / "ws", private_dir=tmp_path / "private")
    spec = ladder_spec(key="F1-l1-0001.s1", spec_text=task.spec_text, files=tuple(task.manifest["files_in_scope"]),
                       visible=tuple(task.manifest["visible_verify"]))
    emitted = planemit.emit(spec, task.workdir)
    env = {**agent_env.build(home=tmp_path / "home"), "ROKO_CONFIG": str(emitted.config_path),
           **{key: "vb-offline-placeholder" for _, _, _, key in CASCADE}}
    checked = subprocess.run([str(REAL_ROKO), "--repo", str(task.workdir), "--no-serve", "plan", "validate", "--strict",
                              "--dag", str(task.workdir / "plans")], cwd=task.workdir, env=env, capture_output=True,
                             text=True, timeout=120, check=False)
    assert checked.returncode == 0, checked.stdout + checked.stderr
    assert "0 diagnostics in 1 plan" in checked.stdout and "Tasks: 1" in checked.stdout
