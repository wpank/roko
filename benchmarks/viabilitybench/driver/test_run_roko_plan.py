"""Offline test of the roko_plan arm runner (3319): a fake `claude` writes a fixed multi-task plan, and a fake
`roko` runs it on the ladder, with one task that passes first try, one that retries on the same model and one that
escalates to another, then the whole-plan gate. Every cost class (plan, execute, retry, escalate, integrate)
should be recorded.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko_plan.py -q
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

import agent_env
import caps
import harness
import layout
import ledger
import provider
import run_roko_plan
import secret
import vb
from common import canary, hmac_seed
from test_run_cli import FAKE_CLAUDE

PLAN_MODEL = "claude-opus-5-5"
CHEAP, MID = "gpt-oss-120b", "glm-4.7"
SLUG_KEY = "PL01-0001.s1"
PLANNER_RESULT = {
    "type": "result", "subtype": "success", "is_error": False, "duration_ms": 9000, "duration_api_ms": 8000,
    "num_turns": 3, "result": "Wrote the plan.", "session_id": "fake-plan-session", "total_cost_usd": 0.42,
    "usage": {"input_tokens": 500, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0,
             "output_tokens": 900, "server_tool_use": {"web_search_requests": 0, "web_fetch_requests": 0},
             "service_tier": "standard", "cache_creation": {"ephemeral_1h_input_tokens": 0}},
    "modelUsage": {PLAN_MODEL: {"inputTokens": 500, "outputTokens": 900, "cacheReadInputTokens": 0,
                                "cacheCreationInputTokens": 0, "canonicalModel": PLAN_MODEL}},
    "permission_denials": [], "uuid": "fake-plan-uuid",
}
PLANNER_U_PRIME = (500 * 4.00 + 900 * 20.00) / 1e6  # claude-opus-5-5's rates (config/prices/2026-09-28.toml)

# A stand-in for roko's `plan run` on a multi-task plan (3319): T01 passes first try (execute), T02 fails once then
# passes on the same model (execute, retry) and T03 fails on the cheap rung then passes on the mid one (execute,
# escalate). The whole-plan gate (gap-60233f) then passes, so the checkpoint succeeds.
FAKE_ROKO = r'''#!__PYTHON__
import datetime, json, sys
from pathlib import Path

args = sys.argv[1:]
if args == ["--version"]:
    print("roko 0.1.0 (git 0fa4e0fa4e)")
    sys.exit(0)
if "validate" in args:
    sys.exit(0)
repo, slug = Path(args[args.index("--repo") + 1]), Path(args[args.index("run") + 1]).name
roko = repo / ".roko"
run_id = "graph-%s-1" % slug
(roko / "state" / "graph" / slug).mkdir(parents=True)
now = datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")
plan = [  # (task_id, [(model, success), ...])
    ("T01", [("__CHEAP__", True)]),
    ("T02", [("__CHEAP__", False), ("__CHEAP__", True)]),
    ("T03", [("__CHEAP__", False), ("__MID__", True)]),
]
episodes = []
for task_id, attempts in plan:
    for number, (model, passed) in enumerate(attempts, 1):
        key = "%s:%s:%s:%d" % (run_id, slug, task_id, number)
        episodes.append({"task_id": task_id, "model": model, "backend": "cerebras" if model == "__CHEAP__" else "zai",
                         "success": passed, "turns": 1, "completed_at": now,
                         "failure_reason": None if passed else "verify: 1/1 verify step(s) failed",
                         "extra": {"plan_id": slug, "attempt_key": key, "outcome": "passed" if passed else "gate_failed"}})
(roko / "episodes.jsonl").write_text("".join(json.dumps(e) + "\n" for e in episodes))
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps({"plan_id": slug, "status": "succeeded"}))
sys.exit(0)
'''


@pytest.fixture
def places(tmp_path: Path, monkeypatch) -> dict[str, Path]:
    secret_file = secret.create(tmp_path / "private-config" / "secret")
    fake_bin = tmp_path / "fake-bin"
    fake_bin.mkdir()
    monkeypatch.setenv("PATH", os.pathsep.join([str(fake_bin), *agent_env.SYSTEM_PATH]))
    monkeypatch.delenv(secret.ENV_NAME, raising=False)
    return {"tmp": tmp_path, "bin": fake_bin, "results": tmp_path / "results", "work": tmp_path / "work",
           "secret": secret_file}


def fake_claude(places: dict[str, Path], slug: str, plan_text: str) -> Path:
    """The fake `claude` on PATH, scripted to write `plans/<slug>/tasks.toml` and settle with PLANNER_RESULT."""
    log, config = places["tmp"] / "claude.jsonl", places["tmp"] / "claude.json"
    needles = [*secret.load(places["secret"]).needles, canary.RELEASE_CANARY]
    config.write_text(json.dumps({"scenario": "solve", "log": str(log), "result": PLANNER_RESULT,
                                  "needles": needles, "files": {f"plans/{slug}/tasks.toml": plan_text},
                                  "hidden_url": "", "visible": ""}))
    program = places["bin"] / "claude"
    program.write_text(FAKE_CLAUDE.replace("__CONFIG__", repr(str(config))))
    program.chmod(0o755)
    return program


def fake_roko(places: dict[str, Path]) -> Path:
    binary = places["bin"] / "roko"
    binary.write_text(FAKE_ROKO.replace("__PYTHON__", sys.executable).replace("__CHEAP__", CHEAP)
                      .replace("__MID__", MID))
    binary.chmod(0o755)
    return binary


def task_context(places: dict[str, Path], arm: dict, binary: Path) -> harness.TaskContext:
    arm = {**arm, "roko": {**arm["roko"], "binary": str(binary)}}
    snapshot = ledger.load_snapshot()
    run_dir = places["results"] / "TEST-PLAN" / "run-1"
    run_dir.mkdir(parents=True)
    work = places["work"] / SLUG_KEY
    work.mkdir(parents=True)
    subprocess.run(["git", "init", "-q"], cwd=work, check=True)
    subprocess.run(["git", "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "--allow-empty", "-qm", "x"],
                   cwd=work, check=True)
    book = ledger.Ledger(run_dir / "ledger.jsonl", line=arm["arm"]["line"], experiment_id="TEST-PLAN",
                         run_id="run-1", price_snapshot_id=snapshot.id)
    return harness.TaskContext(
        experiment_id="TEST-PLAN", run_id="run-1", arm=arm, model=CHEAP,
        endpoint=provider.Endpoint("cerebras", "http://127.0.0.1:9/cerebras"), provider=None,
        snapshot=snapshot, caps=caps.Caps.from_table(arm["caps"]), ledger=book, billed=True,
        instance_id="PL01-0001", seed=1, key=SLUG_KEY, workdir=work,
        spec_text="# Feature\n\nAdd a widget with three parts.\n", files_in_scope=("a.py",), visible_verify=("true",),
        agent_env=agent_env.build(home=places["work"] / "_home"))


def test_roko_plan_runner_records_planner_and_executor_costs(places):
    arm = vb.load_arm("roko_plan")
    slug = run_roko_plan.planemit.plan_slug(SLUG_KEY)
    plan_text = (
        f'[meta]\nplan = "{slug}"\ntotal = 3\ndone = 0\nstatus = "ready"\nmax_parallel = 3\n\n'
        '[[meta.verify]]\nphase = "test"\ncommand = "true"\nfail_msg = "whole-feature check failed"\n\n'
        '[[task]]\nid = "T01"\ntitle = "A"\ndescription = "Do A."\nstatus = "ready"\nrole = "implementer"\n'
        'files = ["a.py"]\ndepends_on = []\nmax_retries = 2\n'
        '[[task.verify]]\nphase = "test"\ncommand = "true"\nfail_msg = "x"\n\n'
        '[[task]]\nid = "T02"\ntitle = "B"\ndescription = "Do B."\nstatus = "ready"\nrole = "implementer"\n'
        'files = ["b.py"]\ndepends_on = []\nmax_retries = 2\n'
        '[[task.verify]]\nphase = "test"\ncommand = "true"\nfail_msg = "x"\n\n'
        '[[task]]\nid = "T03"\ntitle = "C"\ndescription = "Do C."\nstatus = "ready"\nrole = "implementer"\n'
        'files = ["c.py"]\ndepends_on = []\nmax_retries = 2\n'
        '[[task.verify]]\nphase = "test"\ncommand = "true"\nfail_msg = "x"\n')
    fake_claude(places, slug, plan_text)
    ctx = task_context(places, arm, fake_roko(places))
    outcome = run_roko_plan.run_task(ctx)
    assert outcome.status == "completed", outcome.reason
    classes = [attempt.cost_class for attempt in outcome.attempts]
    assert classes.count("plan") == 1 and classes.count("integrate") == 1
    assert classes.count("execute") == 3 and classes.count("retry") == 1 and classes.count("escalate") == 1
    assert {"plan", "execute", "retry", "escalate", "integrate"} == set(classes)
    plan_attempt = outcome.attempts[0]
    assert plan_attempt.cost_class == "plan" and plan_attempt.model_requested == PLAN_MODEL
    assert plan_attempt.cost.api_equiv_usd == pytest.approx(PLANNER_U_PRIME) and plan_attempt.cost.source == "cli_usage"
    assert plan_attempt.vendor_usd == pytest.approx(0.42)
    integrate = outcome.attempts[-1]
    assert integrate.cost_class == "integrate" and integrate.cost.api_equiv_usd == 0.0
    by_task = {}
    for attempt in outcome.attempts:
        if attempt.task_id:
            by_task.setdefault(attempt.task_id, []).append(attempt)
    assert [a.cost_class for a in by_task["T01"]] == ["execute"]
    assert [a.cost_class for a in by_task["T02"]] == ["execute", "retry"]
    assert [a.cost_class for a in by_task["T03"]] == ["execute", "escalate"]
    assert by_task["T03"][1].model_dispatched == MID
    rows = [json.loads(line) for line in (places["results"] / "TEST-PLAN" / "run-1" / "ledger.jsonl")
           .read_text().splitlines() if line.strip()]
    assert len(rows) == len(outcome.attempts)
    assert all(not row["billed_usd"] for row in rows if row["attempt_key"] == plan_attempt.attempt_key)
