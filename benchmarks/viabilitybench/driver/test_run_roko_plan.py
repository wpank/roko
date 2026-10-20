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
import re
import subprocess
import sys
import tomllib
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
from common import canary, hmac_seed, sandbox
from test_run_cli import FAKE_CLAUDE, HIDDEN_URL
from test_sandbox_net import Listener, macos_only

PLAN_MODEL = "claude-opus-5-5"
CHEAP, MID = "gpt-oss-120b", "glm-4.7"
KEY = "PL01-0001.s1"
ALIAS = "5d0c1f9e3a7b2468"  # the task's alias, which names its workdir and its plan (bug-3c921e)
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


def fake_claude(places: dict[str, Path], slug: str, plan_text: str, scenario: str = "solve",
                **extra: object) -> Path:
    """The fake `claude` on PATH, scripted to write `plans/<slug>/tasks.toml` and settle with PLANNER_RESULT; `extra`
    joins its config (test_run_cli.FAKE_CLAUDE's scenarios)."""
    log, config = places["tmp"] / "claude.jsonl", places["tmp"] / "claude.json"
    needles = [*secret.load(places["secret"]).needles, canary.RELEASE_CANARY]
    config.write_text(json.dumps({"scenario": scenario, "log": str(log), "result": PLANNER_RESULT,
                                  "needles": needles, "files": {f"plans/{slug}/tasks.toml": plan_text},
                                  "hidden_url": "", "visible": "", **extra}))
    program = places["bin"] / "claude"
    program.write_text(FAKE_CLAUDE.replace("__CONFIG__", repr(str(config))))
    program.chmod(0o755)
    return program


def fake_roko(places: dict[str, Path], shell: str = "", shell_log: Path | None = None) -> Path:
    """The fake `roko` on PATH. With `shell`, its `plan run` first runs that command as an agent's bash tool would,
    inside its own process tree, and appends what it printed to `shell_log`."""
    binary = places["bin"] / "roko"
    fake = FAKE_ROKO.replace("__PYTHON__", sys.executable).replace("__CHEAP__", CHEAP).replace("__MID__", MID)
    if shell:
        anchor = 'roko = repo / ".roko"\n'
        assert anchor in fake
        fake = fake.replace(anchor, anchor + (
            "import subprocess\n"
            f"_ran = subprocess.run(['bash', '-c', {shell!r}], cwd=repo, capture_output=True, text=True, timeout=60)\n"
            f"open({str(shell_log)!r}, 'a').write(_ran.stdout + _ran.stderr)\n"), 1)
    binary.write_text(fake)
    binary.chmod(0o755)
    return binary


def three_task_plan(slug: str) -> str:
    """The planner's tasks.toml: three tasks whose checks pass, and a whole-plan gate that passes."""
    return (
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


def read_log(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def task_context(places: dict[str, Path], arm: dict, binary: Path) -> harness.TaskContext:
    arm = {**arm, "roko": {**arm["roko"], "binary": str(binary)}}
    snapshot = ledger.load_snapshot()
    run_dir = places["results"] / "TEST-PLAN" / "run-1"
    run_dir.mkdir(parents=True)
    work = places["work"] / ALIAS
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
        instance_id="PL01-0001", seed=1, key=KEY, alias=ALIAS, workdir=work,
        spec_text="# Feature\n\nAdd a widget with three parts.\n", files_in_scope=("a.py",), visible_verify=("true",),
        agent_env=agent_env.build(home=places["work"] / "_home"))


def test_roko_plan_runner_records_planner_and_executor_costs(places):
    arm = vb.load_arm("roko_plan")
    slug = run_roko_plan.planemit.plan_slug(ALIAS)
    fake_claude(places, slug, three_task_plan(slug))
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
    # gap-0bd49a: the record names the ladder's network rule and, under `planner`, the planner session's own.
    host = sandbox.kind((), sandbox.NETWORK_NONE)  # "sandbox-exec+net" on macOS, "none" where no rule applies
    policy = outcome.network_policy
    assert (policy["network"], policy["sandbox"], policy["unix_sockets"]) == ("loopback:9", host, "workspace")
    planner = policy["planner"]
    assert re.fullmatch(r"loopback:\d+", planner["network"]) and planner["sandbox"] == host
    assert planner["egress"] == {"allow": ["api.anthropic.com:443"], "admitted": 0, "refused": []}


def test_the_plan_slice_arm_records_its_prompt(places, monkeypatch):
    """bug-b19ce8: `vb run` records roko_plan's prompt (`vb.runner_prompt`, the config's `prompt`, and so its
    `config_hash`) as `run_roko_plan.arm_prompt` reports it: a version and the sha256 of every text the runner writes
    for the arm's agents. Changing any of them by one more newline (monkeypatched) changes the recorded sha256 and
    keeps the version: the planner's prompt, roko.toml's templates (each of which changes the roko.toml the runner
    writes, which still parses) and the planner session's fixed invocation (`run_cli.PROMPT_SHA256`)."""
    arm = vb.load_arm("roko_plan")
    runner = vb.load_runner(arm)
    recorded = vb.runner_prompt(runner, arm)
    assert runner is run_roko_plan and recorded["version"] == "run-roko-plan-1", recorded
    assert re.fullmatch("[0-9a-f]{64}", recorded["sha256"] or ""), recorded
    ctx = task_context(places, arm, fake_roko(places))
    settings = ctx.arm["roko"]
    written = run_roko_plan._roko_toml(ctx, settings)
    assert tomllib.loads(written)["routing"]["ladder"]["rungs"][0]["model"] == CHEAP
    texts = [(run_roko_plan, "PLANNER_PROMPT", False), (run_roko_plan, "ROKO_TOML_TAIL", True),
             (run_roko_plan, "RUNG_LINE", True), (run_roko_plan.planemit, "LADDER_HEAD", True),
             (run_roko_plan.planemit, "PROVIDER_TABLE", True), (run_roko_plan.planemit, "MODEL_TABLE", True),
             (run_roko_plan.run_cli, "PROMPT_SHA256", False)]
    for module, name, in_roko_toml in texts:
        with monkeypatch.context() as patched:
            patched.setattr(module, name, getattr(module, name) + "\n")
            changed = vb.runner_prompt(runner, arm)
            assert changed["version"] == recorded["version"] and changed["sha256"] != recorded["sha256"], name
            if in_roko_toml:
                rewritten = run_roko_plan._roko_toml(ctx, settings)
                assert rewritten != written and tomllib.loads(rewritten), name
    assert vb.runner_prompt(runner, arm) == recorded  # each text put back, the same prompt


@macos_only
def test_roko_plan_planner_and_ladder_cannot_reach_the_network(places):
    """gap-0bd49a: the roko_plan arm's two halves run in the agents' sandbox. The planner's Bash tool reaches the
    network only through its egress proxy, which refuses a truth suite's host, and cannot open a socket to a local
    listener; the ladder's roko processes, whose tools run the agent's commands, cannot reach the listener either."""
    arm = vb.load_arm("roko_plan")
    slug = run_roko_plan.planemit.plan_slug(ALIAS)
    ladder_log = places["tmp"] / "ladder-shell.log"
    with Listener() as other:
        local = f"curl -sS --max-time 5 http://127.0.0.1:{other.port}/; echo exit=$?"
        fake_claude(places, slug, three_task_plan(slug), "egress", api=["127.0.0.1", 9],
                    shell={"local_port": local, "fetch_suite": f"curl -sS --max-time 5 {HIDDEN_URL}; echo exit=$?"})
        outcome = run_roko_plan.run_task(task_context(places, arm, fake_roko(places, local, ladder_log)))
        assert outcome.status == "completed", outcome.reason
        assert other.connections == 0
    [tried] = [line["egress"] for line in read_log(places["tmp"] / "claude.jsonl") if "egress" in line]
    outcome_of = {name: re.search(r"exit=\d+", text)[0] for name, text in tried["shell"].items()}
    assert outcome_of == {"local_port": "exit=7", "fetch_suite": "exit=56"}, tried["shell"]
    assert tried["api"].startswith("failed")  # the CONNECT to a host off the allowlist was refused
    assert re.search(r"exit=\d+", ladder_log.read_text())[0] == "exit=7"
    planner = outcome.network_policy["planner"]
    assert planner["sandbox"] == outcome.network_policy["sandbox"] == "sandbox-exec+net"
    assert [(row["method"], row["target"]) for row in planner["egress"]["refused"]] == [
        ("CONNECT", "127.0.0.1:9"), ("CONNECT", "raw.githubusercontent.com:443")]
