"""Offline tests of the Roko arm: plan emission, the per-attempt model check, and `vb run` through a fake roko.

The fake roko (`FAKE_ROKO`) stands in for the binary. It answers `--version`, passes `plan validate`, and on
`plan run` writes the records a Graph run leaves in `.roko/` (episodes, cost rows and the checkpoint) for the models
a test scripts. The two `real_roko` tests drive the prebuilt binary instead, when there is one
(`target/debug/roko`, or `$VB_TEST_ROKO_BIN`). They run it against a loopback fake provider, so no call is paid.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -q
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
import tomllib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

import agent_env
import layout
import ledger
import materialize
import planemit
import run_roko
import validate
import vb
from common import canary, hmac_seed
from stub_provider import StubServer

TOY_FAMILY = layout.DRIVER_DIR / "testdata" / "toy_family"
TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
ARM = layout.ARMS_DIR / "roko_fixed.toml"
PIN = "gpt-oss-120b"
SOLUTION = "def clamp(value, low, high):\n    return max(low, min(value, high))\n"
REAL_ROKO = Path(os.environ.get("VB_TEST_ROKO_BIN") or layout.REPO_ROOT / "target" / "debug" / "roko")
real_roko = pytest.mark.skipif(not os.access(REAL_ROKO, os.X_OK), reason=f"no roko binary at {REAL_ROKO}")

FAKE_ROKO = r'''#!__PYTHON__
"""A stand-in for the roko binary: it records its argv and environment and writes a Graph run's records."""
import json, os, sys
from pathlib import Path

behaviour = json.loads(Path(__BEHAVIOUR__).read_text())
args = sys.argv[1:]
with open(behaviour["log"], "a") as log:
    log.write(json.dumps({"argv": args, "env": dict(os.environ)}) + "\n")
if args == ["--version"]:
    print("roko 0.1.0 (rustc 1.96.1, aarch64-apple-darwin, git 0fa4e0fa4e)")
    sys.exit(0)
repo = Path(args[args.index("--repo") + 1])
if "validate" in args:
    print("0 diagnostics in 1 plan")
    sys.exit(0)
slug = Path(args[args.index("run") + 1]).name
roko = repo / ".roko"
(roko / "learn").mkdir(parents=True, exist_ok=True)
models, succeeded = behaviour["models"], behaviour["status"] == "succeeded"
completed = behaviour.get("completed") or [f"2026-09-29T15:00:0{number}Z" for number in range(len(models))]
for number, model in enumerate(models):
    passed = succeeded and number == len(models) - 1
    usage = {"input_tokens": 1200, "output_tokens": 80, "cache_read_tokens": 0, "cache_write_tokens": 0,
             "cost_usd": 0.0005}
    episode = {"task_id": "T01", "model": model, "backend": "cerebras", "success": passed, "turns": 1,
               "usage": usage, "completed_at": completed[number], "extra": {"plan_id": slug},
               "failure_reason": None if passed else "verify: 1/1 verify step(s) failed"}
    if "durations" in behaviour:  # the Graph path's dispatch time, in seconds
        episode["duration_secs"] = behaviour["durations"][number]
    with open(roko / "episodes.jsonl", "a") as handle:
        handle.write(json.dumps(episode) + "\n")
    with open(roko / "learn" / "costs.jsonl", "a") as handle:
        handle.write(json.dumps({"model": model, "provider": "cerebras", "plan_id": slug, "task_id": "T01"}) + "\n")
if behaviour.get("proxy_log"):  # what the metering proxy would have logged for these calls
    with open(behaviour["proxy_log"], "a") as handle:
        handle.writelines(json.dumps(row) + "\n" for row in behaviour["proxy"])
if behaviour["solve"]:
    (repo / "calc" / "ops.py").write_text(behaviour["solution"])
extensions = {"roko.gate.verdict@1": {"value": {"verdicts": {"T01": "passed"}}}} if succeeded else {}
(roko / "state" / "graph" / slug).mkdir(parents=True)
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps(
    {"schema_version": 3, "plan_id": slug, "status": behaviour["status"], "extensions": extensions}))
sys.exit(0 if succeeded else 1)
'''

RETRYING_ROKO = r'''#!__PYTHON__
"""A stand-in for roko's `plan run` that retries as roko does: each of its three attempts is one conversation with the
provider its roko.toml names, extended call by call until the metering proxy refuses a call, then a failed episode."""
import datetime, json, os, sys, tomllib, urllib.error, urllib.request
from pathlib import Path

args = sys.argv[1:]
if args == ["--version"]:
    print("roko 0.1.0 (git 0fa4e0fa4e)")
    sys.exit(0)
if "validate" in args:
    sys.exit(0)
repo, slug = Path(args[args.index("--repo") + 1]), Path(args[args.index("run") + 1]).name
config = tomllib.loads(Path(os.environ["ROKO_CONFIG"]).read_text())
[provider], [model] = config["providers"].values(), config["models"]
roko = repo / ".roko"
(roko / "state" / "graph" / slug).mkdir(parents=True)
for attempt in range(3):  # every attempt starts over from the same first prompt
    messages, failure = [{"role": "system", "content": "You implement tasks. " * 20},
                         {"role": "user", "content": "Fix clamp. " * 40}], None
    for _ in range(10):
        request = urllib.request.Request(provider["base_url"] + "/chat/completions", data=json.dumps(
            {"model": model, "messages": messages}).encode(), headers={
            "Content-Type": "application/json", "Authorization": "Bearer " + os.environ[provider["api_key_env"]]})
        try:
            reply = json.loads(urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request).read())
        except urllib.error.HTTPError as err:
            failure = f"provider: http {err.code}"
            break
        messages += [reply["choices"][0]["message"], {"role": "user", "content": "tool output " * 170}]
    now = datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")
    with open(roko / "episodes.jsonl", "a") as handle:
        handle.write(json.dumps({"task_id": "T01", "model": model, "backend": "cerebras", "success": False,
                                 "turns": 1, "completed_at": now, "failure_reason": failure,
                                 "extra": {"plan_id": slug}}) + "\n")
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps(
    {"plan_id": slug, "status": "failed", "extensions": {}}))
sys.exit(1)
'''


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def toy_task(tmp_path: Path, instance_id: str = "F1-l1-0001") -> materialize.Materialized:
    workdir, private = tmp_path / "ws" / instance_id, tmp_path / "private" / instance_id
    return materialize.materialize(family_dir=TOY_FAMILY, instance_id=instance_id, workdir=workdir, private_dir=private)


def plan_spec(task: materialize.Materialized, **changes: object) -> planemit.PlanSpec:
    fields = {"key": f"{task.instance_id}.s1", "spec_text": task.spec_text,
              "files": tuple(task.manifest["files_in_scope"]), "visible": tuple(task.manifest["visible_verify"]),
              "model": PIN, "provider": "cerebras", "base_url": "http://127.0.0.1:9/v1",
              "api_key_env": "CEREBRAS_API_KEY", "price_row": ledger.load_snapshot().row(PIN), "usd_cap": 0.43}
    return planemit.PlanSpec(**{**fields, **changes})


def fake_roko(tmp_path: Path, models: list[str], *, status: str = "succeeded", **extra: object) -> tuple[Path, Path]:
    """Write the fake roko and its behaviour (`extra` adds episode `completed` times and `durations`, and `proxy` rows
    to append to `proxy_log`); return the binary and the log of its calls."""
    log, behaviour = tmp_path / "fake-roko.jsonl", tmp_path / "fake-roko.json"
    behaviour.write_text(json.dumps({"models": models, "status": status, "solve": True, "solution": SOLUTION,
                                     "log": str(log), **extra}))
    binary = tmp_path / "bin" / "roko"
    binary.parent.mkdir()
    binary.write_text(FAKE_ROKO.replace("__PYTHON__", sys.executable).replace("__BEHAVIOUR__", repr(str(behaviour))))
    binary.chmod(0o755)
    return binary, log


def arm_with(tmp_path: Path, binary: Path) -> Path:
    text = ARM.read_text()
    assert 'binary = "target/debug/roko"' in text
    arm = tmp_path / "roko_test.toml"
    arm.write_text(text.replace('binary = "target/debug/roko"', f"binary = {json.dumps(str(binary))}"))
    return arm


def run_vb(places: dict[str, Path], arm: Path, url: str = "http://127.0.0.1:9/v1", *extra: str) -> int:
    return vb.main(["run", "--experiment", "TEST-ROKO", "--run-id", "run-1", "--stream", TOY_STREAM,
                    "--arm", str(arm), "--model", PIN, "--seeds", "1", "--limit", "1", "--provider-url", url,
                    "--results", str(places["results"]), "--work", str(places["work"]),
                    "--secret-file", str(places["secret"]), *extra])


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def evidence(models: list[str], **changes: object) -> run_roko.Evidence:
    episodes = [{"task_id": "T01", "model": model, "backend": "cerebras", "success": False, "turns": 1,
                 "completed_at": f"2026-09-29T15:00:0{number}Z", "extra": {"plan_id": "vb-x"}}
                for number, model in enumerate(models)]
    return run_roko.Evidence(**{"episodes": episodes, "cost_rows": [], "efficiency": [], "verdicts": [],
                                "checkpoint": None, "proxy_rows": None, **changes})


def settle(found: run_roko.Evidence) -> tuple[list[run_roko.RokoAttempt], list[str]]:
    return run_roko.settle(found, chain_key="run-1/F1-l1-0001.s1", model=PIN, provider="cerebras",
                           snapshot=ledger.load_snapshot(), reserved_usd=0.1, max_attempts=3)


def test_emitted_plan_has_explicit_rungs_and_no_hidden_checks(tmp_path):
    task = toy_task(tmp_path)
    emitted = planemit.emit(plan_spec(task), task.workdir)
    assert emitted.tasks_path.read_text() == emitted.tasks_text
    assert (task.workdir / "roko.toml").read_text() == emitted.config_text
    plan, config = tomllib.loads(emitted.tasks_text), tomllib.loads(emitted.config_text)
    [task_table] = plan["task"]
    assert {key: plan["meta"][key] for key in ("total", "max_parallel", "skip_enrichment")} == {
        "total": 1, "max_parallel": 1, "skip_enrichment": True}
    assert (task_table["role"], task_table["files"], task_table["max_retries"], task_table["model_hint"]) == (
        "implementer", ["calc/ops.py"], 2, PIN)
    assert task_table["description"].strip() == task.spec_text.strip() and "context" not in task_table
    visible = task.manifest["visible_verify"]
    assert [step["command"] for step in task_table["verify"]] == visible
    # Explicit rungs: exactly the visible check, so no path that reads rungs falls back to `cargo check`.
    assert [(rung["command"], rung["required"]) for rung in config["gates"]["rungs"]] == [(visible[0], True)]
    assert list(config["providers"]) == ["cerebras"] and list(config["models"]) == [PIN]
    assert config["models"][PIN]["slug"] == PIN and config["routing"]["fallback_models"] == []
    assert config["gates"]["max_review_cycles"] == 0 and config["pipeline"]["focused"]["max_turns"] == 12
    assert config["models"][PIN]["cost_input_per_m"] == ledger.load_snapshot().row(PIN)["input"]
    budget = config["budget"]  # Roko's invariants: 0 < max_turn_usd <= max_plan_usd, and room for every retry
    assert (budget["max_plan_usd"], budget["max_turn_usd"], config["conductor"]["max_agents"]) == (0.43, 0.043, 1)
    # No hidden check: nothing of the private manifest beyond the spec and the visible check reaches Roko.
    text = emitted.tasks_text + emitted.config_text
    manifest = task.manifest
    assert canary.find(text) == [] and manifest["canary"] not in text
    for private in (manifest["truth_suite"]["id"], *manifest["planted_gaming"], "hidden.py", str(task.private_dir)):
        assert private not in text
    assert emitted.slug == planemit.plan_slug("F1-l1-0001.s1") and "F1" not in emitted.slug
    assert "Implement `clamp`" in task_table["title"]

    with pytest.raises(planemit.PlanEmitError, match="already has"):  # Roko's files must be Roko's alone
        planemit.emit(plan_spec(task), task.workdir)
    other = toy_task(tmp_path, "F1-l1-0002")
    for bad in ({"spec_text": other.spec_text + canary.RELEASE_CANARY}, {"files": ("tests/*.py",)},
                {"files": ("plans/x.py",)}, {"visible": ()}):
        with pytest.raises(planemit.PlanEmitError):
            planemit.emit(plan_spec(other, **bad), other.workdir)
    assert not (other.workdir / "roko.toml").exists() and not (other.workdir / "plans").exists()
    nasty = 'Say "hi" to C:\\temp\n"""triple""" and a tab\t and \x01 end'
    tricky = planemit.emit(plan_spec(other, spec_text=nasty, visible=("true", "echo ok")), other.workdir)
    [task_table] = tomllib.loads(tricky.tasks_text)["task"]
    assert task_table["description"].strip() == nasty.strip()
    assert [step["command"] for step in task_table["verify"]] == ["( true ) && ( echo ok )"]


def test_model_mismatch_marks_attempt_infra_error(places, tmp_path):
    # The unit: a fixture episode naming another model marks that attempt, and the run, infra_error.
    attempts, problems = settle(evidence([PIN, "glm-4.7"]))
    assert [attempt.checks for attempt in attempts] == [[], ["model_mismatch"]]
    assert attempts[1].ended_by == "model_mismatch" and attempts[1].model_dispatched == "glm-4.7"
    assert any("attempt 2" in problem and "glm-4.7" in problem for problem in problems)
    ran = run_roko.Ran(argv=[], returncode=0, stdout="", stderr="", timed_out=False, seconds=1.0)
    assert run_roko._status(ran, evidence([PIN, "glm-4.7"]), problems)[0] == "infra_error"
    # A substitution recorded anywhere else counts too: a cost row, an efficiency row, or an S01 verdict.
    for changes in ({"cost_rows": [{"model": "kimi-k2.6", "provider": "cerebras"}]},
                    {"efficiency": [{"kind": "model_call", "model": PIN, "provider": "groq"}]},
                    {"verdicts": [{"attempt": 1, "executed": {"model_requested": PIN, "failover_chain": ["x"]}}]}):
        assert "model_mismatch" in settle(evidence([PIN], **changes))[1][0]
    # No evidence is no proof: an unnamed model or no attempt at all is unverified.
    assert "model_unverified" in settle(evidence([""]))[1][0] and "model_unverified" in settle(evidence([]))[1][0]

    # End to end: Roko reports a pass and the tree is right, but attempt 2 ran glm-4.7. It never counts as a success.
    binary, _ = fake_roko(tmp_path, [PIN, "glm-4.7"])
    assert run_vb(places, arm_with(tmp_path, binary), "http://127.0.0.1:9/v1", "--transcripts") == 0
    out = places["results"] / "TEST-ROKO" / "run-1"
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["execution"]["status"] == "infra_error" and "model_mismatch" in record["execution"]["reason"]
    first, second = record["execution"]["attempts"]
    assert (first["model_dispatched"], first["checks"], first["gate_verdict"]) == (PIN, [], None)
    assert (second["model_dispatched"], second["checks"], second["ended_by"]) == ("glm-4.7", ["model_mismatch"],
                                                                                  "model_mismatch")
    assert record["vs"]["label"] == 0 and record["vs"]["checks"]["completion"] == 0
    assert record["vs"]["checks"]["hidden"] == 1  # the tree is right: only the model check excludes the run
    rows = read_jsonl(out / "ledger.jsonl")
    assert len(rows) == 2 and all(validate.validate("ledger", row) == [] for row in rows)


def test_pinned_run_records_attempts_and_leaves_only_the_agents_tree(places, tmp_path, monkeypatch):
    monkeypatch.setattr(vb.secret, "KEYS_IN_ENV_OK", True)  # the tests' escape hatch (bug-979a06) for the next line
    monkeypatch.setenv("CEREBRAS_API_KEY", "sk-driver-only-9d1e")  # an offline run never hands Roko the real key
    monkeypatch.setenv("ROKO_CONFIG", "/elsewhere/roko.toml")
    binary, log = fake_roko(tmp_path, [PIN, PIN])
    assert run_vb(places, arm_with(tmp_path, binary)) == 0
    out = places["results"] / "TEST-ROKO" / "run-1"
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert (record["execution"]["status"], record["execution"]["reason"]) == ("completed", "gate_passed")
    assert record["vs"]["label"] == 1 and [a["model_dispatched"] for a in record["execution"]["attempts"]] == [PIN] * 2
    last = record["execution"]["attempts"][-1]
    assert (last["gate_verdict"], last["roko_build"], last["model_reported"], last["usage"]) == (
        "passed", "0fa4e0fa4e", None, None)
    # Roko records neither the served model nor its auxiliary calls, so without the proxy the cost is unknown.
    assert record["costs"]["api_equiv_usd"] is None and record["costs"]["source"] == "unknown"
    assert last["roko_usage"]["input_tokens"] == 1200
    assert all(row["api_equiv_usd"] is None for row in read_jsonl(out / "ledger.jsonl"))
    # Each attempt Roko may make (max_retries 2) was reserved before `plan run`; a row or a release freed each.
    keys = [f"run-1/F1-l1-0001.s1:{number}" for number in (1, 2, 3)]
    events = [(event["event"], event["attempt_key"]) for event in read_jsonl(out / "reservations.jsonl")]
    assert events == [("reserve", key) for key in keys] + [("release", keys[2])]
    assert ledger.read_books(places["results"]).reservations == []
    diff = (out / "archives" / "F1-l1-0001.s1.diff").read_text()
    assert "return max(low, min(value, high))" in diff
    assert "roko.toml" not in diff and "plans/" not in diff and ".roko/" not in diff
    assert read_jsonl(out / "s01" / "F1-l1-0001.s1" / "episodes.jsonl")[0]["model"] == PIN
    assert record["provenance"]["s01_run_dir"] == "s01/F1-l1-0001.s1"  # the record points at Roko's own records

    calls = read_jsonl(log)
    assert [call["argv"][-1] for call in calls[:1]] == ["--version"]
    validate_argv, run_argv = calls[1]["argv"], calls[2]["argv"]
    assert validate_argv[validate_argv.index("plan"):][:4] == ["plan", "validate", "--strict", "--dag"]
    assert run_argv[run_argv.index("plan"):][:2] == ["plan", "run"] and "--no-tui" in run_argv
    for argv in (validate_argv, run_argv):
        assert argv[argv.index("--model") + 1] == PIN and "--no-serve" in argv
    env = calls[2]["env"]
    extra = set(env) - set(agent_env.FIXED) - set(agent_env.PASSTHROUGH) - {"HOME", "TMPDIR", "PATH", "USER",
                                                                           "LOGNAME"}
    assert extra <= {"ROKO_CONFIG", "CEREBRAS_API_KEY", "__CF_USER_TEXT_ENCODING"}
    assert env["CEREBRAS_API_KEY"] == run_roko.OFFLINE_KEY and env["ROKO_CONFIG"].endswith("/roko.toml")
    assert env["ROKO_CONFIG"] != "/elsewhere/roko.toml" and env["HOME"].startswith(str(places["work"]))


SLUG = "vb-86da757b9e"
CHAIN = f"graph-{SLUG}-38dae924-f80f-44f4-b0a1-91c1a85589c3:{SLUG}:T01"  # Roko's chain key: <run>:<plan>:<task>


def model_truth_records(workspace: Path, order: tuple[int, ...] = (1, 2, 3), helpers: tuple[int, ...] = (2, 2, 0),
                        changes: dict | None = None) -> run_roko.Evidence:
    """Write Roko's records for three attempts, two failed gates and a pass, in the shape Roko writes since
    bug-31438d (b0ede92d7, read off a real run against the ToolStub), and read them back. Episodes go to the file in
    `order`. `changes` maps (file, attempt, role) to fields to change in that row: file is episode, cost, efficiency
    or verdict, and role is implementer or helper."""
    changes = changes or {}

    def row(file: str, number: int, role: str, fields: dict) -> dict:
        return {**fields, **changes.get((file, number, role), {})}

    episodes, costs, efficiency, verdicts = {}, [], [], []
    for number, helper_count in zip((1, 2, 3), helpers):
        key, passed = f"{CHAIN}:{number}", number == 3
        extra = {"plan_id": SLUG, "attempt_key": key, "model_reported": None, "model_mismatch": False,
                 "outcome": "passed" if passed else "gate_failed", "learning_label": int(passed),
                 "initial_model": PIN, "successful_model": PIN}
        if helper_count:
            extra.update(helper_calls=helper_count, helper_cost_usd=0.000775, helper_tokens_in=2000,
                         helper_tokens_out=100)
        episodes[number] = row("episode", number, "implementer", {
            "task_id": "T01", "model": PIN, "backend": "cerebras", "success": passed, "turns": 3,
            "completed_at": f"2026-09-30T07:46:0{number}.730513Z", "duration_secs": 0.057,
            "failure_reason": None if passed else "verify: 1/1 verify step(s) failed", "extra": extra})
        efficiency += [{"kind": "model_call", "role": "dispatch_v2", "model": PIN, "provider": "cerebras"}] * 3
        for index in range(1, helper_count + 1):
            served = {"attempt_key": key, "model": PIN, "role": "helper", "model_reported": PIN,
                      "model_mismatch": False}
            costs.append(row("cost", number, "helper", {**served, "provider": "cerebras", "plan_id": SLUG,
                                                        "task_id": "T01", "success": True}))
            efficiency.append(row("efficiency", number, "helper", {
                **served, "attempt_id": f"{key}/helper-{index}", "resolved_model": PIN, "backend": "cerebras"}))
        costs.append(row("cost", number, "implementer", {
            "attempt_key": key, "outcome": extra["outcome"], "learning_label": int(passed), "model": PIN,
            "provider": "cerebras", "role": "implementer", "plan_id": SLUG, "task_id": "T01", "success": passed,
            "model_reported": None, "model_mismatch": False}))
        efficiency.append(row("efficiency", number, "implementer", {
            "attempt_key": key, "attempt_id": key, "role": "implementer", "model": PIN, "resolved_model": PIN,
            "backend": "cerebras", "model_reported": None, "model_mismatch": False, "turn_number": 3}))
        verdicts.append(row("verdict", number, "implementer", {
            "schema_version": run_roko.VERDICT_SCHEMA, "plan_id": SLUG, "task_id": "T01", "attempt": number,
            "attempt_key": key, "inv": None, "outcome": extra["outcome"],
            "executed": {"provider": "cerebras", "model_requested": PIN, "model_dispatched": PIN,
                         "model_reported": None, "models_reported": [], "model_mismatch": False,
                         "failover_chain": [], "failover_reason": None, "turns": 3},
            **({"helpers": {"calls": helper_count, "tokens_in": 2000, "tokens_out": 100}} if helper_count else {})}))
    roko = workspace / ".roko"
    for relpath, rows in (("episodes.jsonl", [episodes[number] for number in order]), ("learn/costs.jsonl", costs),
                          ("learn/efficiency.jsonl", efficiency),
                          (f"runs/graph-{SLUG}-38dae924/attempts.jsonl", verdicts)):
        (roko / relpath).parent.mkdir(parents=True, exist_ok=True)
        (roko / relpath).write_text("".join(json.dumps(line) + "\n" for line in rows))
    return run_roko.read_evidence(workspace, SLUG)


def test_run_roko_reads_the_model_truth_fields(tmp_path):
    # gap-dad97b: since bug-31438d, Roko's records name each attempt by its S01 attempt key, report the served model,
    # a failover and an unreported turn count, and record helper calls as rows of their own. The runner numbers
    # attempts by key, whatever order the rows are in, counts turns plus helper calls, and checks every model field.
    attempts, problems = settle(model_truth_records(tmp_path / "clean", order=(2, 3, 1)))
    assert problems == []
    assert [(a.number, a.gate_verdict, a.turns, a.helper_calls, a.roko_calls, a.calls, a.calls_known)
            for a in attempts] == [(1, None, 3, 2, 5, 5, True), (2, None, 3, 2, 5, 5, True),
                                   (3, "passed", 3, 0, 3, 3, True)]
    assert [a.attempt_key for a in attempts] == [f"run-1/F1-l1-0001.s1:{number}" for number in (1, 2, 3)]
    assert attempts[0].as_record()["roko_calls"] == 5 and attempts[2].as_record()["helper_calls"] == 0

    # A turn count the agent never reported is unknown, not 0 turns; so is that attempt's call count.
    unknown = {("episode", 2, "implementer"): {"turns": 0, "extra": {
        "plan_id": SLUG, "attempt_key": f"{CHAIN}:2", "turns_unknown": True, "helper_calls": 2}}}
    attempts, problems = settle(model_truth_records(tmp_path / "unknown", changes=unknown))
    assert problems == [] and [(a.turns, a.roko_calls, a.as_record()["calls"]) for a in attempts] == [
        (3, 5, 5), (None, None, None), (3, 3, 3)]

    # Every served-model field counts, on the attempt its key names: the episode's report and Roko's own mismatch mark,
    # a helper call's cost row, a failover on an efficiency row, and the models a verdict saw.
    cases = [  # (the attempt flagged, the change)
        (2, {("episode", 2, "implementer"): {"extra": {"plan_id": SLUG, "attempt_key": f"{CHAIN}:2",
                                                       "model_reported": "glm-4.7", "model_mismatch": True}}}),
        (1, {("cost", 1, "helper"): {"model_reported": "glm-4.7", "model_mismatch": True}}),
        (3, {("efficiency", 3, "implementer"): {"substituted_from": "qwen-3-235b",
                                                 "substitution_reason": "rate limited"}}),
        # this verdict's `attempt` says 3, but its key names attempt 1, and the key decides
        (1, {("verdict", 1, "implementer"): {"attempt": 3, "executed": {
            "provider": "cerebras", "model_requested": PIN, "model_dispatched": PIN, "model_reported": "glm-4.7",
            "models_reported": [PIN, "glm-4.7"], "model_mismatch": True, "turns": 3}}}),
    ]
    for case, (flagged, change) in enumerate(cases):
        attempts, problems = settle(model_truth_records(tmp_path / f"case{case}", changes=change))
        assert problems and all(problem.startswith(f"model_mismatch: attempt {flagged}:") for problem in problems)
        assert [a.checks for a in attempts] == [["model_mismatch"] if a.number == flagged else [] for a in attempts]
    # A dated snapshot of the pin is the pin, and a null report is no evidence either way.
    dated = {("cost", 2, "helper"): {"model_reported": f"{PIN}-2025-08-05"}}
    assert settle(model_truth_records(tmp_path / "dated", changes=dated))[1] == []

    # With the proxy metering, its count of each attempt's calls is the meter; Roko's own stays beside it.
    usage = {"tokens_in": 1000, "tokens_cache_read": 0, "tokens_out": 50, "tokens_reasoning": 0}
    found = model_truth_records(tmp_path / "proxied")
    found.proxy_rows = [{"task": "F1-l1-0001.s1", "ordinal": ordinal, "ts": f"2026-09-30T07:46:0{second}.5Z",
                         "model_requested": PIN, "model_reported": PIN, "usage": usage, "usage_source": "reported"}
                        for ordinal, second in enumerate([1] * 5 + [2] * 4 + [3] * 3, 1)]
    attempts, problems = settle(found)
    assert problems == [] and [(a.calls, a.roko_calls) for a in attempts] == [(5, 5), (4, 5), (3, 3)]

    # Attempt keys that repeat, or name only some attempts, cannot say which record is whose.
    twice = {("episode", 3, "implementer"): {"extra": {"plan_id": SLUG, "attempt_key": f"{CHAIN}:2"}}}
    assert "model_unverified" in settle(model_truth_records(tmp_path / "twice", changes=twice))[1][0]


def test_attempt_reservations_are_all_or_nothing(tmp_path):
    budget = tmp_path / "budget.toml"
    budget.write_text('schema_version = "vb.budget/1"\nsource = "test"\n[programme]\ntotal_usd = 10\n'
                      'never_allocated_min_usd = 1\nstop_usd = 5\n[[line]]\nid = "BL0"\ngate = "G0"\n'
                      'content = "test"\nplanned_usd = 0.1\ncap_usd = 0.25\n')
    run_dir = tmp_path / "results" / "T" / "r"
    run_dir.mkdir(parents=True)
    book = ledger.Ledger(run_dir / "ledger.jsonl", line="BL0", experiment_id="T", run_id="r",
                         price_snapshot_id=ledger.DEFAULT_SNAPSHOT, budget=ledger.load_budget(budget))
    keys = ["r/x:1", "r/x:2", "r/x:3"]
    with pytest.raises(ledger.BudgetError):  # two attempts fit under $0.25, the third does not
        run_roko._reserve(book, keys, 0.1)
    events = [(event["event"], event["attempt_key"]) for event in read_jsonl(run_dir / ledger.RESERVATIONS)]
    assert events == [("reserve", "r/x:1"), ("reserve", "r/x:2"), ("release", "r/x:1"), ("release", "r/x:2")]
    assert ledger.read_books(tmp_path / "results").reservations == []
    run_roko._reserve(book, keys[:2], 0.1)  # nothing is left held, so two attempts fit again
    assert len(ledger.read_books(tmp_path / "results").reservations) == 2


def test_roko_failures_timeouts_and_missing_records(places, tmp_path):
    binary, _ = fake_roko(tmp_path, [PIN, PIN, PIN], status="failed")
    assert run_vb(places, arm_with(tmp_path, binary)) == 0
    [record] = read_jsonl(places["results"] / "TEST-ROKO" / "run-1" / "records.jsonl")
    assert (record["execution"]["status"], record["execution"]["reason"]) == ("failed", "gate_failed")
    assert len(record["execution"]["attempts"]) == 3 and record["vs"]["label"] == 0

    attempts, problems = settle(evidence([PIN] * 4))
    assert "extra_attempts" in problems[-1]
    ran = run_roko.Ran(argv=[], returncode=None, stdout="", stderr="", timed_out=True, seconds=1200.0)
    assert run_roko._status(ran, evidence([]), settle(evidence([]))[1]) == ("timeout", "wallclock")
    # A substituted model outranks the timeout: that run is excluded, not counted as censoring.
    assert run_roko._status(ran, evidence(["glm-4.7"]), settle(evidence(["glm-4.7"]))[1])[0] == "infra_error"


def test_proxy_meters_attempts_and_flags_silent_ones():
    # Rows as faultproxy.py writes them: whole-second ts, run-record usage, and usage_source none when unbilled.
    usage = {"tokens_in": 800, "tokens_cache_read": 200, "tokens_cache_write_5m": 0, "tokens_cache_write_1h": 0,
             "tokens_out": 100, "tokens_reasoning": 20}
    base = {"task": "F1-l1-0001.s1", "model_requested": PIN, "model_reported": PIN, "usage": usage,
            "usage_source": "reported"}
    proxied = [{**base, "ordinal": 1, "ts": "2026-09-29T14:59:59Z"},
               {**base, "ordinal": 2, "ts": "2026-09-29T15:00:00Z"},
               {**base, "ordinal": 3, "ts": "2026-09-29T15:00:00Z", "usage": None, "usage_source": "none",
                "model_reported": None, "fault_injected": "http_5xx"}]
    attempts, problems = settle(evidence([PIN, PIN], proxy_rows=proxied))
    assert problems == ["no_proxy_traffic: attempt 2: the metering proxy saw no request"]
    first = attempts[0]
    assert (first.calls, first.model_reported, first.usage["tokens_in"], first.usage["tokens_cache_read"]) == (
        3, PIN, 1600, 400)
    assert first.cost.api_equiv_usd == pytest.approx(
        ledger.price(first.usage, ledger.load_snapshot().row(PIN)).api_equiv_usd) and first.cost.source == \
        "provider_usage"
    assert attempts[1].checks == ["no_proxy_traffic"] and attempts[1].cost.api_equiv_usd is None
    # Two attempts that ended in the same second cannot be told apart, so the second's empty window is not flagged.
    close = evidence([PIN, PIN], proxy_rows=proxied[:2])
    close.episodes[0]["completed_at"], close.episodes[1]["completed_at"] = (
        "2026-09-29T15:00:00.100000Z", "2026-09-29T15:00:00.900000Z")
    assert settle(close)[1] == []
    assert "no_proxy_traffic" in settle(evidence([PIN], proxy_rows=[]))[1][0]  # routed around the proxy
    swapped = [{**proxied[0], "model_requested": "llama-3.3-70b", "model_reported": "llama-3.3-70b"}]
    assert any("model_requested 'llama-3.3-70b'" in problem
               for problem in settle(evidence([PIN], proxy_rows=swapped))[1])



def test_attempts_ending_in_the_same_second_get_their_own_usage():
    # bug-09fac4: the proxy stamps requests to the microsecond, as roko stamps its episodes (these three ends are real
    # roko's, from one capped task), so three attempts that end within one second each keep their own requests. The
    # second attempt's only call was refused at the input cap: it billed nothing, so it costs $0, not unknown.
    usage = {"tokens_in": 900, "tokens_cache_read": 100, "tokens_cache_write_5m": 0, "tokens_cache_write_1h": 0,
             "tokens_out": 50, "tokens_reasoning": 0}
    base = {"task": "F1-l1-0001.s1", "model_requested": PIN, "model_reported": PIN, "usage": usage,
            "usage_source": "reported", "status": 200}
    refused = {**base, "usage": None, "usage_source": "none", "model_reported": None, "status": 403,
               "refused": "input_token_cap"}
    stamps = ["19:49:04.100512", "19:49:04.150034", "19:49:04.240871", "19:49:04.290006"]
    rows = [{**row, "ordinal": ordinal, "ts": f"2026-09-29T{stamp}Z"}
            for ordinal, (row, stamp) in enumerate(zip([base, base, refused, base], stamps), 1)]

    def with_ends(proxy_rows: list[dict]) -> run_roko.Evidence:
        found = evidence([PIN, PIN, PIN], proxy_rows=proxy_rows)
        for episode, end in zip(found.episodes, ("19:49:04.216594", "19:49:04.266200", "19:49:04.311739")):
            episode["completed_at"] = f"2026-09-29T{end}Z"
        return found

    attempts, problems = settle(with_ends(rows))
    assert problems == []
    assert [(attempt.calls, attempt.usage_unknown) for attempt in attempts] == [(2, False), (1, False), (1, False)]
    assert [attempt.usage["tokens_in"] for attempt in attempts] == [1800, 0, 900]
    row = ledger.load_snapshot().row(PIN)
    assert [attempt.cost.api_equiv_usd for attempt in attempts] == pytest.approx(
        [ledger.price(attempts[0].usage, row).api_equiv_usd, 0.0, ledger.price(attempts[2].usage, row).api_equiv_usd])
    assert {attempt.cost.source for attempt in attempts} == {"provider_usage"}
    # The proxy's older whole-second stamps could not tell these attempts apart: the first took every request.
    attempts, problems = settle(with_ends([{**row, "ts": row["ts"][:19] + "Z"} for row in rows]))
    assert problems == [] and [(attempt.calls_known, attempt.usage_unknown) for attempt in attempts] == [
        (True, False), (False, True), (False, True)]


def test_the_roko_arm_enforces_the_per_attempt_input_cap(places, tmp_path):
    # gap-806e37: Roko has no per-attempt input cap, so the metering proxy holds each attempt to the arm's
    # input_tokens_per_attempt. It sees no attempts, so it draws them as conversations: calls that extend each other.
    # This roko's three attempts each extend one conversation from the same first prompt until a call could pass the
    # cap; the proxy refuses that call, and the next attempt, sent afresh, gets a budget of its own.
    binary = tmp_path / "bin" / "roko"
    binary.parent.mkdir()
    binary.write_text(RETRYING_ROKO.replace("__PYTHON__", sys.executable))
    binary.chmod(0o755)
    arm = arm_with(tmp_path, binary)
    arm.write_text(arm.read_text().replace("input_tokens_per_attempt = 150000", "input_tokens_per_attempt = 3000"))
    with StubServer(lambda body: "Done.") as stub:
        assert run_vb(places, arm, stub.url, "--proxy") == 0
        served = len(stub.requests)
    out = places["results"] / "TEST-ROKO" / "run-1"
    rows = read_jsonl(out / "proxy.jsonl")
    assert [(row["conversation"], row["refused"]) for row in rows] == [
        (conversation, refused) for conversation in (1, 2, 3) for refused in (None, None, "attempt_input_cap")]
    assert served == 6  # a refused call never reaches the provider
    for conversation in (1, 2, 3):  # no attempt's metered input passes the cap
        billed = [row["usage"]["tokens_in"] + row["usage"]["tokens_cache_read"] for row in rows
                  if row["conversation"] == conversation and row["usage"]]
        assert 0 < sum(billed) <= 3000
    [record] = read_jsonl(out / "records.jsonl")
    attempts = record["execution"]["attempts"]
    assert [(attempt["calls"], attempt["ended_by"]) for attempt in attempts] == [(3, "attempt_input_tokens")] * 3
    assert record["execution"]["status"] == "failed" and validate.validate("run-record", record) == []

def test_plan_slice_records_carry_queue_waits_and_class_costs(places, tmp_path):
    # gap-04e8e2: the fake roko records two attempts with their dispatch times, and the metering proxy's log holds
    # their calls, one of them rate limited. The record carries the queue wait, each attempt's class and busy time,
    # and the cost per class; the schema accepts them; and the plan-level report prints them.
    out = places["results"] / "TEST-ROKO" / "run-1"
    usage = {"tokens_in": 900, "tokens_cache_read": 0, "tokens_out": 120, "tokens_reasoning": 0}
    call = {"task": "F1-l1-0001.s1", "model_requested": PIN, "model_reported": PIN, "usage": usage,
            "usage_source": "reported", "status": 200, "elapsed_ms": 800.0}
    limited = {**call, "usage": None, "usage_source": "none", "model_reported": None, "status": 429,
               "fault_injected": "rate_limit", "elapsed_ms": 5.0}
    calls = [{**row, "ordinal": ordinal, "ts": f"2026-09-29T15:00:{second:02d}Z"}
             for ordinal, (row, second) in enumerate(((call, 1), (limited, 2), (call, 4), (call, 11), (call, 13)), 1)]
    binary, _ = fake_roko(tmp_path, [PIN, PIN], completed=["2026-09-29T15:00:06Z", "2026-09-29T15:00:14Z"],
                          durations=[5.5, 4.5], proxy=calls, proxy_log=str(out / "proxy.jsonl"))
    assert run_vb(places, arm_with(tmp_path, binary)) == 0
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    first, second = record["execution"]["attempts"]
    assert (first["task_id"], first["cost_class"], second["task_id"], second["cost_class"]) == (
        "T01", "execute", "T01", "retry")
    assert (first["started_at"], first["finished_at"]) == ("2026-09-29T15:00:00.500Z", "2026-09-29T15:00:06.000Z")
    assert (second["started_at"], second["finished_at"]) == ("2026-09-29T15:00:09.500Z", "2026-09-29T15:00:14.000Z")
    assert (first["queue_wait_s"], second["queue_wait_s"]) == (1.995, 0.0)  # 2 s after the 429, less its 5 ms
    assert record["execution"]["queue_wait_s"] == pytest.approx(1.995)
    each = ledger.price(ledger.add_usage(usage, usage), ledger.load_snapshot().row(PIN)).api_equiv_usd
    assert (first["api_equiv_usd"], second["api_equiv_usd"]) == (pytest.approx(each), pytest.approx(each))
    assert record["costs"]["by_class"] == {"plan": 0.0, "execute": pytest.approx(each), "retry": pytest.approx(each),
                                           "escalate": 0.0, "integrate": 0.0}
    assert record["costs"]["api_equiv_usd"] == pytest.approx(2 * each)
    # The schema holds the classes to the total and the record's wait to its attempts' waits.
    for path, value, error in (("costs.by_class.retry", 0.0, "add up to"),
                               ("execution.queue_wait_s", 5.0, "add up to"),
                               ("execution.attempts.0.queue_wait_s", -1.0, "below 0")):
        broken = json.loads(json.dumps(record))
        *parents, last = path.split(".")
        target = broken
        for key in parents:
            target = target[int(key)] if isinstance(target, list) else target[key]
        target[last] = value
        assert any(error in problem for problem in validate.validate("run-record", broken)), path

    # The plan-level report, fed the same record as a roko_plan feature of 20 s, prints what it carries.
    sys.path.insert(0, str(layout.VB_ROOT / "analysis"))
    import metrics
    import report
    feature = json.loads(json.dumps(record))
    feature.update(arm="roko_plan")
    feature["task"].update(family="PL", instance_id="PL01-0001", ladder=None)
    feature["execution"].update(started_at="2026-09-29T15:00:00Z", finished_at="2026-09-29T15:00:20Z")
    written, found = report.build([feature], "TEST-ROKO", ks=(3,), analysis_commit="x", computed_at="y")
    section = written["plan_slice"]
    assert section["table"][0]["arms"]["roko_plan"]["queue_wait_s"] == pytest.approx(1.995)
    got = section["arms"]["roko_plan"]
    assert got["queue_wait_median_s"] == pytest.approx(1.995) and got["queue_wait_recorded"] == 1
    assert got["process"] == {"planner_share": 0.0, "realized_parallelism_median": pytest.approx(10 / 20),
                              "tasks_escalated_share": 0.0, "integrations_rejected": 0}
    assert "queue waits" not in section["not_recorded"] and "planner share" not in section["not_recorded"]
    assert {m.metric for m in found} >= {"pl_queue_wait_median_s", "pl_planner_share", "pl_tasks_escalated_share"}
    printed = report.render(written, found)
    assert "median queue wait 2 s (1 of 1 features)" in printed
    assert "planner's share of the cost 0.0%; realized parallelism 0.50 (median); tasks escalated 0%" in printed
    assert "PL01-0001 roko_plan: VF 1, cost $" in printed and "queue wait 2 s, run run-1" in printed
    assert metrics.process_measures([feature])["planner_share"][0] == 0.0


@real_roko
def test_real_roko_validates_the_emitted_plan(tmp_path):
    task = toy_task(tmp_path)
    emitted = planemit.emit(plan_spec(task), task.workdir)
    env = {**agent_env.build(home=tmp_path / "home"), "ROKO_CONFIG": str(emitted.config_path),
           "CEREBRAS_API_KEY": run_roko.OFFLINE_KEY}
    checked = subprocess.run([str(REAL_ROKO), "--repo", str(task.workdir), "--model", PIN, "--no-serve", "plan",
                              "validate", "--strict", "--dag", str(task.workdir / "plans")], cwd=task.workdir,
                             env=env, capture_output=True, text=True, timeout=120, check=False)
    assert checked.returncode == 0, checked.stdout + checked.stderr
    assert "0 diagnostics in 1 plan" in checked.stdout and "Tasks: 1" in checked.stdout


class ToolStub:
    """A loopback chat-completions server that plays Roko's implementer: read the file, write the fix, finish."""

    def __init__(self) -> None:
        stub = self
        self.requests: list[dict] = []

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self) -> None:  # noqa: N802 (the stdlib's name)
                body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
                stub.requests.append(body)
                message = stub.reply(body.get("messages", []))
                usage = {"prompt_tokens": 1000, "completion_tokens": 50, "total_tokens": 1050}
                finish = "tool_calls" if message.get("tool_calls") else "stop"
                if body.get("stream"):
                    chunks = [{"choices": [{"index": 0, "delta": message, "finish_reason": None}]},
                              {"choices": [{"index": 0, "delta": {}, "finish_reason": finish}], "usage": usage}]
                    data = "".join(f"data: {json.dumps({'id': 'stub', 'object': 'chat.completion.chunk',
                                                          'model': body['model'], **chunk})}\n\n" for chunk in chunks)
                    self._send("text/event-stream", (data + "data: [DONE]\n\n").encode())
                else:
                    self._send("application/json", json.dumps({"id": "stub", "object": "chat.completion",
                                                               "model": body["model"], "usage": usage, "choices": [
                        {"index": 0, "message": message, "finish_reason": finish}]}).encode())

            def _send(self, kind: str, data: bytes) -> None:
                self.send_response(200)
                self.send_header("Content-Type", kind)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def log_message(self, *args: object) -> None:
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.server.server_address[1]}/v1"

    @staticmethod
    def reply(messages: list[dict]) -> dict:
        system = " ".join(str(m.get("content") or "") for m in messages if m.get("role") == "system")
        turn = sum(1 for m in messages if m.get("role") == "assistant")
        if "role_identity" not in system or turn >= 2:
            return {"role": "assistant", "content": "Done." if "role_identity" in system else "0.5"}
        name, arguments = ("read_file", {"path": "calc/ops.py"}) if turn == 0 else (
            "write_file", {"path": "calc/ops.py", "content": SOLUTION})
        return {"role": "assistant", "content": None, "tool_calls": [
            {"id": f"call_{turn}", "type": "function", "function": {"name": name, "arguments": json.dumps(arguments)}}]}


@real_roko
def test_real_roko_run_against_a_fake_provider(places, tmp_path):
    stub = ToolStub()
    try:
        assert run_vb(places, arm_with(tmp_path, REAL_ROKO), stub.url) == 0
    finally:
        stub.server.shutdown()
    [record] = read_jsonl(places["results"] / "TEST-ROKO" / "run-1" / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert (record["execution"]["status"], record["vs"]["label"]) == ("completed", 1), record["execution"]["reason"]
    assert {request["model"] for request in stub.requests} == {PIN}
    [attempt] = record["execution"]["attempts"]
    assert attempt["model_dispatched"] == PIN and attempt["checks"] == [] and attempt["roko_build"]


FLAKY = 'schema_version = "vb.disturbance/1"\n\n[[disturbance]]\nkind = "flaky_verify"\nparams = { p = 1.0 }\n'


def test_emitted_plan_runs_its_check_through_the_verify_wrapper(tmp_path):
    # gap-4e8795: in a run with flaky_verify, the verify step and the gate rung both run the visible check through
    # the visible-verify wrapper, and nothing else changes.
    task = toy_task(tmp_path)
    wrapper = "/runs/_home/F1-l1-0001.s1/.vb-bin/vb-verify"
    emitted = planemit.emit(plan_spec(task, verify_wrapper=wrapper), task.workdir)
    command = f"{wrapper} 'python3 -m unittest discover -s tests/visible'"
    assert emitted.visible_command == command
    [step] = tomllib.loads(emitted.tasks_text)["task"][0]["verify"]
    [rung] = tomllib.loads(emitted.config_text)["gates"]["rungs"]
    assert step["command"] == rung["command"] == command


@real_roko
def test_real_roko_gate_meets_the_flaky_verify_wrapper(places, tmp_path):
    # gap-4e8795: with p = 1, every run of the visible check through the wrapper fails as a killed check, so each of
    # Roko's attempts fails its gate. The record lists the flakes, and the census's own rerun still passes the fix.
    spec = tmp_path / "flaky.toml"
    spec.write_text(FLAKY)
    stub = ToolStub()
    try:
        assert run_vb(places, arm_with(tmp_path, REAL_ROKO), stub.url, "--disturbance", str(spec),
                      "--transcripts") == 0
    finally:
        stub.server.shutdown()
    out = places["results"] / "TEST-ROKO" / "run-1"
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    [emitted] = [event for event in json.loads((out / record["provenance"]["transcript_ref"]).read_text())
                 if event.get("event") == "emit"]
    # Named by PATH, so no host path enters Roko's prompt, which shows its verify command (A4).
    assert tomllib.loads(emitted["tasks_toml"])["task"][0]["verify"][0]["command"] == (
        "vb-verify 'python3 -m unittest discover -s tests/visible'")
    assert (record["execution"]["status"], record["execution"]["reason"]) == ("failed", "gate_failed")
    assert len(record["execution"]["attempts"]) == 3  # max_retries = 2: every attempt met a flake
    visible = record["visible"]
    assert visible["flake_injected"] and visible["verify_runs"] >= 3
    assert [flake["run"] for flake in visible["flakes"]] == list(range(1, visible["verify_runs"] + 1))
    assert visible["passed"] and record["stream"]["perturbations_active"] == ["flaky_verify"]
