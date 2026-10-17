"""Tests of planemit's ladder mode (3310): the cheap-model ladder of decision 3302 instead of one pinned model, with the
pinned mode unchanged byte for byte. The `real_roko` tests run `plan validate --strict --dag` on the emitted ladder
plan, and `run_roko.preflight` on every Roko arm file's plan, when there is a binary (`target/debug/roko`, or
`$VB_TEST_ROKO_BIN`).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -q -rs
"""

from __future__ import annotations

import dataclasses
import json
import re
import subprocess
import tomllib
from pathlib import Path

import pytest

import agent_env
import campaign
import caps
import layout
import ledger
import materialize
import planemit
import run_roko
import vb
from test_run_roko import REAL_ROKO, fake_roko, real_roko, roko_arms

SNAPSHOT = ledger.load_snapshot()
GOLDEN = layout.DRIVER_DIR / "testdata" / "planemit"
CASCADE = (("cheap", "gpt-oss-120b", "cerebras", "CEREBRAS_API_KEY"), ("mid", "glm-4.7", "zai", "ZAI_API_KEY"),
           ("strong", "gpt-5.4-mini", "openai", "OPENAI_API_KEY"))  # D11's three cheap rungs
ALLOW = tuple(model for _, model, _, _ in CASCADE)


def rungs(cascade=CASCADE) -> tuple[planemit.Rung, ...]:
    return tuple(planemit.Rung(name=name, model=model, provider=provider, base_url=f"http://127.0.0.1:9/{provider}",
                               api_key_env=key, price_row=SNAPSHOT.row(model)) for name, model, provider, key in cascade)


def pinned_spec() -> planemit.PlanSpec:
    """The spec of the golden files in testdata/planemit/, emitted before ladder mode existed. Its alias is the task
    key they were emitted with; since bug-3c921e `vb run` hands planemit a random alias, never a key."""
    return planemit.PlanSpec(
        alias="F1-l1-0001.s1", spec_text=("# Implement `clamp`\n\nToy instance: F1-l1-0001\n\nKeep \"quotes\" and \\ "
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
    assert planemit.LADDER_TEMPLATE_SHA256 != planemit.TEMPLATE_SHA256  # its version is in no file (bug-c1b04b)
    # A start further up the ladder is the arm's choice too.
    mid = planemit.emit(ladder_spec(start="mid", model="glm-4.7"), workspace(tmp_path, "mid"))
    assert tomllib.loads(mid.config_text)["routing"]["ladder"]["start"] == dict.fromkeys(planemit.TIERS, "mid")


def test_pinned_mode_is_byte_identical_to_before_ladder_mode(tmp_path):
    emitted = planemit.emit(pinned_spec(), workspace(tmp_path))
    assert emitted.tasks_text == (GOLDEN / "pinned.tasks.toml").read_text()
    assert emitted.config_text == (GOLDEN / "pinned.roko.toml").read_text()
    assert planemit.TEMPLATE_VERSION == "planemit-6"  # bug-c1b04b: 4's files had a header; bug-d68575: 5's `vb-` slug
    assert planemit.TEMPLATE_SHA256 == "392ef78bd0db0e9e65256a77aaee191a339e0c12a326021249322d855892c5f8"


def test_pinned_mode_emits_a_model_whose_slug_has_a_dot(tmp_path):
    """bug-562ee6: pinned mode writes roko.toml's model and provider keys as ladder mode does (`_key`): quoted when the
    slug is not a bare TOML key, so `[models."glm-4.7"]` is one key, where `[models.glm-4.7]` was `models` -> `glm-4`
    -> `7` and `_check` refused the file. Every pinned Roko arm file (roko_fixed on each of LOG1 block A's four models,
    and fr_claude), through the spec `run_roko._plan_spec` builds for it on the endpoint `vb.make_plan` builds, emits
    a roko.toml whose `models` table has the pinned slug as its one key, and `run_roko.preflight`, which `vb run` calls
    before any task, no longer refuses the arm ("the arm cannot emit a plan"). A slug that is a bare key stays bare,
    so the golden files and TEMPLATE_SHA256 do not change (test_pinned_mode_is_byte_identical_to_before_ladder_mode)."""
    snapshot = ledger.load_snapshot()
    pinned = roko_arms(pinned_only=True)
    assert {"roko_fixed", "roko_fixed_glm", "roko_fixed_kimi", "roko_fixed_mini", "fr_claude"} <= set(pinned)
    binary, log = fake_roko(tmp_path, [])  # it accepts every plan: what this test checks is that the arm emits one
    dotted = []
    for name, arm in pinned.items():
        [model] = arm["arm"]["models_allow"]
        endpoint, limits = vb._endpoint(arm, snapshot.row(model), None), caps.Caps.from_table(arm["caps"])
        spec = run_roko._plan_spec(arm, model, endpoint, limits, snapshot.row(model), limits.usd_per_task,
                                   alias=name, spec_text="# T\n\nTest.\n", files=("f.py",), visible=("true",),
                                   snapshot=snapshot)
        emitted = planemit.emit(spec, workspace(tmp_path, name))
        config = tomllib.loads(emitted.config_text)
        assert list(config["models"]) == [model] and config["models"][model]["slug"] == model, name
        assert list(config["providers"]) == [endpoint.provider] and config["agent"]["default_model"] == model, name
        assert tomllib.loads(emitted.tasks_text)["task"][0]["model_hint"] == model, name
        assert f"\n[models.{json.dumps(model) if '.' in model else model}]\n" in emitted.config_text, name
        dotted += [model] if "." in model else []
        run_roko.preflight({**arm, "roko": {**arm["roko"], "binary": str(binary)}}, model, endpoint, limits, snapshot)
    assert sorted(dotted) == ["glm-4.7", "gpt-5.4-mini", "kimi-k2.6"]  # three of LOG1 block A's four Roko models
    validated = [json.loads(line)["argv"] for line in log.read_text().splitlines() if line.strip()]
    assert len(validated) == len(pinned) and all("validate" in argv for argv in validated)


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


ROKO_FULL_OVERLAY = {"gate_mode": "enforce", "audit_floor": 0.05, "routing_mode": "active",
                     "routing_policy": "lcb_aci", "holdout": 0.10, "homeostasis": "if_live"}


def full_spec(**changes: object) -> planemit.PlanSpec:
    return dataclasses.replace(ladder_spec(), overlay=ROKO_FULL_OVERLAY, **changes)


def fake_census(live: bool) -> object:
    """A monkeypatch replacement for `campaign.census_report`, reporting `planemit.HOMEOSTASIS_LOOP` as `live`."""
    def fake(repo=None, roko_bin=None):
        state = "live" if live else "flagged"
        return {"schema": campaign.LOOPS_SCHEMA, "harness_sha": "f1x3d5eed",
                "rows": [{"loop": planemit.HOMEOSTASIS_LOOP, "state": state}]}
    return fake


def test_roko_full_overlay_turns_on_gate_routing_audits_and_holdout(tmp_path, monkeypatch):
    """3360: every mechanism the overlay names lands in roko.toml, each in its own table, and S06's
    homeostasis mode follows L-M1's own loop census (faked here, never a real roko binary)."""
    monkeypatch.setattr(campaign, "census_report", fake_census(live=True))
    emitted = planemit.emit(full_spec(), workspace(tmp_path))
    config = tomllib.loads(emitted.config_text)
    assert config["spec_quality"] == {"mode": "enforce"}
    assert config["audit"] == {"enabled": True, "eps_floor": 0.05}
    assert config["self_model"] == {"mode": "active", "policy": "lcb_aci"}
    assert config["homeostasis"] == {"holdout": 0.10, "mode": "on"}  # L-M1 is LIVE

    monkeypatch.setattr(campaign, "census_report", fake_census(live=False))
    shadow = planemit.emit(full_spec(), workspace(tmp_path, "shadow"))
    assert tomllib.loads(shadow.config_text)["homeostasis"] == {"holdout": 0.10, "mode": "shadow"}


def test_roko_fixed_and_roko_ladder_emit_no_overlay_table(tmp_path):
    """3360's Plan step 3: the overlay's absence in roko_fixed's and roko_ladder's configs."""
    for spec, name in ((pinned_spec(), "fixed"), (ladder_spec(), "ladder")):
        emitted = planemit.emit(spec, workspace(tmp_path, name))
        config = tomllib.loads(emitted.config_text)
        assert not {"spec_quality", "audit", "self_model", "homeostasis"} & set(config)


BENCHMARK_WORDS = re.compile(r"viabilitybench|benchmark|hidden|planemit", re.IGNORECASE)


def test_no_emitted_plan_names_the_benchmark(tmp_path, monkeypatch):
    """bug-c1b04b: the two files planemit writes into a Roko arm's workspace, which its agents can read while Roko
    runs, name neither the benchmark, nor its hidden checks, nor the driver's templates (`planemit-*` versions): they
    hold no comment at all. In every mode the arms use (pinned, Claude CLI, ladder and roko_full's overlay), with and
    without the flaky_verify wrapper, the emitted tasks.toml and roko.toml match none of those words, whatever their
    case. The direct, mini-swe-agent, Claude Code and Codex arms' agents are given no such text either."""
    monkeypatch.setattr(campaign, "census_report", fake_census(live=True))
    claude_cli = dataclasses.replace(pinned_spec(), provider_kind="claude_cli", model="claude-opus-5-5",
                                     tier="architectural")
    modes = {"pinned": pinned_spec(), "claude-cli": claude_cli, "ladder": ladder_spec(), "overlay": full_spec()}
    for name, spec in list(modes.items()):
        modes[f"{name}-wrapped"] = dataclasses.replace(spec, verify_wrapper="/home/agent/bin/wrapper")
    for name, spec in modes.items():
        emitted = planemit.emit(spec, workspace(tmp_path, name))
        for path in (emitted.tasks_path, emitted.config_path):
            text = path.read_text(encoding="utf-8")
            assert BENCHMARK_WORDS.search(text) is None, (name, path.name, BENCHMARK_WORDS.search(text))
            assert [line for line in text.splitlines() if line.lstrip().startswith("#")] == [], (name, path.name)
            assert tomllib.loads(text)  # the files hold only TOML that Roko reads
    # The modes cover both provider kinds, both of planemit's modes, the overlay and the wrapper.
    assert {spec.provider_kind for spec in modes.values()} == {"openai_compat", "claude_cli"}
    assert {bool(spec.rungs) for spec in modes.values()} == {True, False} and modes["overlay"].overlay
    assert sum(spec.verify_wrapper is not None for spec in modes.values()) == 4


def test_resolve_overlay_refuses_an_unknown_mechanism():
    with pytest.raises(planemit.PlanEmitError, match="no mechanism for overlay key"):
        planemit.resolve_overlay({"nonexistent_mechanism": True})


def test_emitted_config_freezes_learning_for_pinned_arms_not_ladder_mode(tmp_path):
    """gap-b001ca: [learning].frozen is true for a pinned-mode arm (roko_fixed's openai_compat shape, and
    fr_claude's claude_cli one), false for a routed, ladder-mode one (roko_ladder's shape); learning_frozen
    overrides either default explicitly, in both directions."""
    pinned = planemit.emit(pinned_spec(), workspace(tmp_path, "pinned"))
    assert tomllib.loads(pinned.config_text)["learning"]["frozen"] is True

    claude_cli = dataclasses.replace(pinned_spec(), provider_kind="claude_cli", model="claude-opus-5-5",
                                     tier="architectural")
    emitted = planemit.emit(claude_cli, workspace(tmp_path, "claude-cli"))
    assert tomllib.loads(emitted.config_text)["learning"]["frozen"] is True

    ladder = planemit.emit(ladder_spec(), workspace(tmp_path, "ladder"))
    assert tomllib.loads(ladder.config_text)["learning"]["frozen"] is False

    unfrozen_pinned = planemit.emit(dataclasses.replace(pinned_spec(), learning_frozen=False),
                                    workspace(tmp_path, "unfrozen-pinned"))
    assert tomllib.loads(unfrozen_pinned.config_text)["learning"]["frozen"] is False
    frozen_ladder = planemit.emit(dataclasses.replace(ladder_spec(), learning_frozen=True),
                                  workspace(tmp_path, "frozen-ladder"))
    assert tomllib.loads(frozen_ladder.config_text)["learning"]["frozen"] is True

    assert planemit.is_frozen(pinned_spec()) is True and planemit.is_frozen(ladder_spec()) is False


@real_roko
def test_real_roko_validates_the_ladder_plan(tmp_path):
    task = materialize.materialize(family_dir=layout.DRIVER_DIR / "testdata" / "toy_family", instance_id="F1-l1-0001",
                                   workdir=tmp_path / "ws", private_dir=tmp_path / "private")
    spec = ladder_spec(alias="5d0c1f9e3a7b2468", spec_text=task.spec_text, files=tuple(task.manifest["files_in_scope"]),
                       visible=tuple(task.manifest["visible_verify"]))
    emitted = planemit.emit(spec, task.workdir)
    env = {**agent_env.build(home=tmp_path / "home"), "ROKO_CONFIG": str(emitted.config_path),
           **{key: "placeholder" for _, _, _, key in CASCADE}}  # run_roko.OFFLINE_KEY
    checked = subprocess.run([str(REAL_ROKO), "--repo", str(task.workdir), "--no-serve", "plan", "validate", "--strict",
                              "--dag", str(task.workdir / "plans")], cwd=task.workdir, env=env, capture_output=True,
                             text=True, timeout=120, check=False)
    assert checked.returncode == 0, checked.stdout + checked.stderr
    assert "0 diagnostics in 1 plan" in checked.stdout and "Tasks: 1" in checked.stdout


@real_roko
def test_real_roko_accepts_every_roko_arms_preflight_plan(monkeypatch):
    """bug-562ee6: `run_roko.preflight`, the `plan validate --strict --dag` that `vb run` runs on the plan an arm
    emits before any task, passes with a real binary for every arm file run_roko runs, on every model the arm may
    start on: the pinned arms' quoted keys (`[models."glm-4.7"]`) as well as the ladder's. roko_full's loop census
    is faked, as in the overlay tests."""
    monkeypatch.setattr(campaign, "census_report", fake_census(live=True))
    snapshot = ledger.load_snapshot()
    checked = []
    for name, arm in roko_arms().items():
        pointed = {**arm, "roko": {**arm["roko"], "binary": str(REAL_ROKO)}}
        for model in arm["arm"]["models_allow"]:
            endpoint = vb._endpoint(arm, snapshot.row(model), None)
            run_roko.preflight(pointed, model, endpoint, caps.Caps.from_table(arm["caps"]), snapshot)
            checked.append((name, model))
    assert {("roko_fixed", "gpt-oss-120b"), ("roko_fixed_glm", "glm-4.7"), ("roko_fixed_kimi", "kimi-k2.6"),
            ("roko_fixed_mini", "gpt-5.4-mini"), ("fr_claude", "claude-opus-5-5"), ("roko_ladder", "glm-4.7"),
            ("roko_full", "gpt-oss-120b")} <= set(checked)
