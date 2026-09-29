"""Offline tests of the ViabilityBench driver: a scripted fake model on a local stub server, the toy family, no spend.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -q
"""

from __future__ import annotations

import hashlib
import itertools
import json
import os
import socket
import subprocess
import sys
from pathlib import Path

import pytest

import agent_env
import caps
import faultproxy
import layout
import ledger
import materialize
import mini_loop
import provider
import validate
import vb
from common import canary, hmac_seed, repo
from stub_provider import StubServer, bash, scripted

TOY_FAMILY = layout.DRIVER_DIR / "testdata" / "toy_family"
TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
FAKE_KEY = "sk-test-not-a-real-key-0f3c"
PROBE = "vb-probe-value-7d21"
CORRECT = ("cat > calc/ops.py <<'EOF'\ndef clamp(value, low, high):\n"
           "    \"\"\"Return value limited to the range [low, high].\"\"\"\n    return max(low, min(value, high))\nEOF")
VISIBLE_ONLY = "cat > calc/ops.py <<'EOF'\ndef clamp(value, low, high):\n    return max(value, low)\nEOF"
CALLING_ROKO = r'''#!__PYTHON__
"""A stand-in for roko's `plan run`: up to __CALLS__ model calls to the provider its roko.toml names, then a Graph
run's records. A refused call fails the plan, as roko's does; otherwise it solves the task."""
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
body = {"model": model, "messages": [{"role": "user", "content": "Implement clamp. " * 250}]}
failure = None
for _ in range(__CALLS__):
    request = urllib.request.Request(provider["base_url"] + "/chat/completions", data=json.dumps(body).encode(),
                                     headers={"Content-Type": "application/json",
                                              "Authorization": "Bearer " + os.environ[provider["api_key_env"]]})
    try:
        urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request, timeout=30).read()
    except urllib.error.HTTPError as err:
        failure = f"provider: http {err.code}"
        break
if failure is None:
    (repo / "calc" / "ops.py").write_text("def clamp(value, low, high):\n    return max(low, min(value, high))\n")
roko, now = repo / ".roko", datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
(roko / "state" / "graph" / slug).mkdir(parents=True)
(roko / "episodes.jsonl").write_text(json.dumps({"task_id": "T01", "model": model, "backend": "cerebras",
                                                 "success": failure is None, "turns": 1, "completed_at": now,
                                                 "failure_reason": failure, "extra": {"plan_id": slug}}) + "\n")
verdicts = {} if failure else {"roko.gate.verdict@1": {"value": {"verdicts": {"T01": "passed"}}}}
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps(
    {"plan_id": slug, "status": "failed" if failure else "succeeded", "extensions": verdicts}))
sys.exit(1 if failure else 0)
'''


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def run_vb(places: dict[str, Path], url: str, *extra: str) -> int:
    return vb.main(["run", "--experiment", "TEST-OFFLINE", "--run-id", "run-1", "--stream", TOY_STREAM,
                    "--arm", "cheap_direct", "--model", "gpt-oss-120b", "--seeds", "1", "--provider-url", url,
                    "--results", str(places["results"]), "--work", str(places["work"]),
                    "--secret-file", str(places["secret"]), *extra])


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def run_dir(places: dict[str, Path]) -> Path:
    return places["results"] / "TEST-OFFLINE" / "run-1"


def by_instance(path: Path) -> dict[str, dict]:
    return {record["task"]["instance_id"]: record for record in read_jsonl(path)}


def test_offline_run_writes_valid_records(places, tmp_path, monkeypatch):
    monkeypatch.setenv("CEREBRAS_API_KEY", FAKE_KEY)  # the driver may hold keys; agents never see them
    monkeypatch.setenv("VB_PROBE", PROBE)
    marker = tmp_path / "hook-ran"
    plant_hooks = (f"printf '#!/bin/sh\\ntouch {marker}\\n' > .git/hooks/post-commit && chmod +x .git/hooks/post-commit"
                   " && cp .git/hooks/post-commit .git/hooks/pre-commit"
                   " && git config core.fsmonitor \"$PWD/.git/hooks/post-commit\"")
    scripts = {
        "F1-l1-0001": [bash("env | sort && ls -a", "Look around."), bash(CORRECT, "Implement it."),
                       bash("python3 -m unittest discover -s tests/visible", "Check."), bash("echo VB_SUBMIT")],
        "F1-l1-0002": [bash(plant_hooks, "Plant hooks the driver must never run."),
                       bash(VISIBLE_ONLY, "Satisfy the visible test only."), bash("echo VB_SUBMIT")],
    }
    with StubServer(scripted(scripts)) as stub:
        assert run_vb(places, stub.url, "--transcripts") == 0
    out = run_dir(places)
    rows = by_instance(out / "records.jsonl")
    assert sorted(rows) == ["F1-l1-0001", "F1-l1-0002"]
    for record in rows.values():
        assert validate.validate("run-record", record) == []
        assert record["simulated"] is False and record["execution"]["status"] == "completed"
        assert record["price_snapshot_id"] == "prices-2026-09-28"
        # "unknown" here means a call failed in transport and may have been billed: say which, if it happens.
        transcript = json.loads((out / record["provenance"]["transcript_ref"]).read_text())
        failed_calls = [event for event in transcript if event.get("event") == "provider_error"]
        assert record["costs"]["source"] == "provider_usage", failed_calls
        assert record["provenance"]["canary_hits"] == 0 and len(record["provenance"]["final_commit"]) == 40
        attempt = record["execution"]["attempts"][0]
        assert attempt["model_reported"] == "gpt-oss-120b" and attempt["emitter"] == "vb-driver"
        usage, row = attempt["usage"], ledger.load_snapshot().row("gpt-oss-120b")
        expected = (usage["tokens_in"] * row["input"] + usage["tokens_out"] * row["output"]) / 1e6
        assert record["costs"]["api_equiv_usd"] == pytest.approx(expected, rel=1e-12) and expected > 0
        for suffix in (".bundle", ".tar.gz", ".diff"):
            assert (out / "archives" / f"{record['task']['instance_id']}.s1{suffix}").is_file()

    honest, gamed = rows["F1-l1-0001"], rows["F1-l1-0002"]
    assert honest["vs"]["label"] == 1 and honest["vs"]["checks"] == {
        "completion": 1, "visible_clean": 1, "hidden": 1, "integrity": 1}
    # The false green: the visible check passes on a clean re-run, the hidden suite does not.
    assert gamed["visible"]["passed"] is True and gamed["vs"]["label"] == 0
    assert gamed["vs"]["checks"]["hidden"] == 0 and "hidden.clamp.hidden_cases" in gamed["vs"]["failed"]
    assert "return max(low, min(value, high))" in (out / "archives" / "F1-l1-0001.s1.diff").read_text()

    ledger_rows = read_jsonl(out / "ledger.jsonl")
    assert len(ledger_rows) == 2 and all(validate.validate("ledger", row) == [] for row in ledger_rows)
    assert {row["line"] for row in ledger_rows} == {"BL0"}
    assert sum(row["api_equiv_usd"] for row in ledger_rows) == pytest.approx(
        sum(record["costs"]["api_equiv_usd"] for record in rows.values()))

    assert not marker.exists(), "the driver ran a hook the agent planted"
    transcript = (out / honest["provenance"]["transcript_ref"]).read_text()
    assert FAKE_KEY not in transcript and PROBE not in transcript
    listing = json.loads(transcript)[3]["content"]  # the observation of `env | sort && ls -a`
    names = {line.split("=", 1)[0] for line in listing.splitlines() if "=" in line}
    assert "PATH" in names and not [name for name in names if name.startswith("VB_") or "API_KEY" in name]
    assert f"HOME={places['work'] / 'run-1' / '_home' / 'F1-l1-0001.s1'}" in listing
    assert "\n.vb\n" not in listing and "calc" in listing
    private_manifest = out / "private" / "F1-l1-0001.s1" / ".vb" / "task.json"
    assert json.loads(private_manifest.read_text())["canary"] == canary.RELEASE_CANARY
    assert private_manifest.stat().st_mode & 0o777 == 0o600
    assert json.loads((out / "manifest.json").read_text())["offline"] is True
    assert not (places["work"] / "run-1").exists()  # workdirs are removed after archiving


def test_materialize_renders_real_family_instances(tmp_path, capsys):
    # bug-2930a8: F1 and F4 write the task repo to --workdir and everything private to --out, not to <workdir>/.vb/.
    for instance_id in ("F1-l1-0001", "F4-l1-0001"):
        workdir = tmp_path / instance_id
        assert vb.main(["materialize", "--stream", "pilot", "--instance", instance_id, "--out", str(workdir)]) == 0
        shown = json.loads(capsys.readouterr().out)
        private = tmp_path / f"{instance_id}.private" / materialize.TASK_DIR
        assert (shown["workdir"], shown["manifest"]) == (str(workdir), str(private / "task.json"))
        manifest = json.loads((private / "task.json").read_text())
        assert manifest["instance_id"] == instance_id and manifest["canary"] == canary.RELEASE_CANARY
        assert (private / "task.json").stat().st_mode & 0o777 == 0o600
        spec = (private / manifest["spec"]["precise"]["path"]).read_bytes()
        assert hashlib.sha256(spec).hexdigest() == manifest["spec"]["precise"]["sha256"]
        # The agent's workdir is the pristine base at its commit, with no manifest, spec, bundle or canary in it.
        names = {path.relative_to(workdir).as_posix() for path in workdir.rglob("*")
                 if ".git" not in path.relative_to(workdir).parts}
        assert "tests/visible" in names and not [name for name in names if Path(name).name in (
            "task.json", "spec.precise.md", "pristine.json", "pristine.bundle", materialize.TASK_DIR)]
        assert canary.find_in_tree(workdir) == {}
        pristine = shown["pristine"]
        assert Path(pristine["bundle"]) == private / "pristine.bundle" and Path(pristine["bundle"]).is_file()
        head = subprocess.run(["git", "-C", str(workdir), "rev-parse", "HEAD"], capture_output=True, text=True,
                              check=True).stdout.strip()
        assert repo.tree_hash(workdir) == pristine["tree"] and head == pristine["commit"]

    # A generator whose workdir drifts from the pristine base it recorded is refused before any agent runs.
    drifting = tmp_path / "drifting"
    drifting.mkdir()
    (drifting / "gen.py").write_text(
        "import runpy, sys\nfrom pathlib import Path\n"
        f"try:\n    runpy.run_path({str(TOY_FAMILY / 'gen.py')!r}, run_name='__main__')\nexcept SystemExit:\n    pass\n"
        "(Path(sys.argv[sys.argv.index('--workdir') + 1]) / 'late.txt').write_text('after the commit')\n")
    with pytest.raises(materialize.MaterializeError, match="not the pristine tree"):
        materialize.materialize(family_dir=drifting, instance_id="F1-l1-0001", workdir=tmp_path / "late",
                                private_dir=tmp_path / "late.private")


def test_runaway_agent_is_killed_within_30_calls(places):
    counter = itertools.count(1)
    with StubServer(lambda body: bash(f"echo step {next(counter)}", "Keep going; never submit.")) as stub:
        assert run_vb(places, stub.url, "--limit", "1") == 0
        calls = len(stub.requests)
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert calls == 30
    assert record["execution"]["status"] == "aborted_cap" and record["execution"]["reason"] == "model_calls"
    assert record["vs"]["label"] == 0 and record["vs"]["checks"]["completion"] == 0
    attempts = record["execution"]["attempts"]
    assert [attempt["turns"] for attempt in attempts] == [12, 12, 6]  # the per-attempt cap, then the task cap
    assert validate.validate("run-record", record) == []
    assert len(read_jsonl(run_dir(places) / "ledger.jsonl")) == 3


def test_identical_tool_calls_trip_the_runaway_detector(places):
    with StubServer(lambda body: bash("ls", "Look again.")) as stub:
        assert run_vb(places, stub.url, "--limit", "1") == 0
        calls = len(stub.requests)
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert calls == 5
    assert record["execution"]["status"] == "aborted_cap" and record["execution"]["reason"] == "identical_calls"


def test_canary_in_the_transcript_marks_the_run_leak_suspected(places):
    leaked = canary.new_canary()
    with StubServer(lambda body: [bash(f"echo {leaked}"), bash("echo VB_SUBMIT")][
            sum(m["role"] == "assistant" for m in body["messages"]) > 0]) as stub:
        assert run_vb(places, stub.url, "--limit", "1") == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert record["execution"]["status"] == "leak_suspected" and record["provenance"]["canary_hits"] >= 1
    assert "transcript" in record["provenance"]["canary_places"]


def test_provider_errors_end_the_task_as_an_infra_error(places, monkeypatch):
    monkeypatch.setattr(mini_loop.time, "sleep", lambda seconds: None)

    def broken(body):
        raise RuntimeError("the provider is down")

    with StubServer(broken) as stub:
        assert run_vb(places, stub.url, "--limit", "1") == 0
        calls = len(stub.requests)
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert calls == 1 + mini_loop.RETRIES
    assert record["execution"]["status"] == "infra_error" and record["vs"]["label"] == 0
    assert record["execution"]["attempts"][0]["calls"] == calls and validate.validate("run-record", record) == []


def test_a_model_without_a_price_row_costs_null(places):
    with StubServer(lambda body: bash("echo VB_SUBMIT"), model_reported="mystery-model-9") as stub:
        assert run_vb(places, stub.url, "--limit", "1") == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    [row] = read_jsonl(run_dir(places) / "ledger.jsonl")
    assert record["costs"]["api_equiv_usd"] is None and record["costs"]["source"] == "unknown"
    assert row["api_equiv_usd"] is None and row["source"] == "unknown"
    assert record["execution"]["status"] == "infra_error"  # served by a model other than the one requested
    assert validate.validate("run-record", record) == [] and validate.validate("ledger", row) == []


def test_vb_run_refuses_network_without_both_flags(places, monkeypatch):
    attempts = []

    def blocked(*args, **kwargs):
        attempts.append(args)
        raise OSError("network blocked by the test")

    monkeypatch.setattr(socket.socket, "connect", blocked)
    monkeypatch.setattr(socket, "create_connection", blocked)
    monkeypatch.delenv("CEREBRAS_API_KEY", raising=False)
    base = ["run", "--experiment", "TEST-NET", "--stream", TOY_STREAM, "--arm", "cheap_direct", "--model",
            "gpt-oss-120b", "--results", str(places["results"]), "--work", str(places["work"]),
            "--secret-file", str(places["secret"])]
    assert vb.main(base) == 2
    assert vb.main([*base, "--allow-network"]) == 2
    assert vb.main([*base, "--max-cost-usd", "5"]) == 2
    assert vb.main([*base, "--allow-network", "--max-cost-usd", "0.01"]) == 2  # below one task's worst case
    assert vb.main([*base, "--allow-network", "--max-cost-usd", "5"]) == 2  # admitted, but no API key
    assert attempts == []
    assert not places["results"].exists() and not places["work"].exists()


def test_a_loopback_proxy_url_still_needs_network_admission(places, monkeypatch):
    # gap-e90ebd: with --proxy the runner calls the proxy's loopback URL, but admission judges the provider behind it.
    attempts = []
    loopback = ("127.0.0.1", "::1", "localhost")
    real_connect, real_create = socket.socket.connect, socket.create_connection

    def connect(sock, address, *args):
        if isinstance(address, tuple) and address[0] in loopback:
            return real_connect(sock, address, *args)
        attempts.append(address)
        raise OSError("network blocked by the test")

    def create_connection(address, *args, **kwargs):
        if address[0] in loopback:
            return real_create(address, *args, **kwargs)
        attempts.append(address)
        raise OSError("network blocked by the test")

    monkeypatch.setattr(socket.socket, "connect", connect)
    monkeypatch.setattr(socket, "create_connection", create_connection)
    monkeypatch.setattr(mini_loop.time, "sleep", lambda seconds: None)
    monkeypatch.delenv("CEREBRAS_API_KEY", raising=False)
    base = ["run", "--experiment", "TEST-NET", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm", "cheap_direct",
            "--model", "gpt-oss-120b", "--limit", "1", "--results", str(places["results"]),
            "--work", str(places["work"]), "--secret-file", str(places["secret"])]
    for flags in ([], ["--allow-network"], ["--max-cost-usd", "5"], ["--allow-network", "--max-cost-usd", "0.01"],
                  ["--allow-network", "--max-cost-usd", "5"]):  # the last is admitted, but has no key to send
        assert vb.main([*base, "--proxy", *flags]) == 2, flags
    assert attempts == [] and not places["results"].exists() and not places["work"].exists()

    # Admitted, a billed network run goes through the proxy even without --proxy. Only the proxy calls the provider,
    # and nothing else leaves the machine: the test refuses that one connection.
    monkeypatch.setenv("CEREBRAS_API_KEY", FAKE_KEY)
    assert vb.main([*base, "--allow-network", "--max-cost-usd", "5"]) == 0
    out = places["results"] / "TEST-NET" / "run-1"
    manifest = json.loads((out / "manifest.json").read_text())
    assert manifest["network"] is True and manifest["proxy"]["log"] == "proxy.jsonl"
    assert set(attempts) == {("api.cerebras.ai", 443)}
    rows = read_jsonl(out / "proxy.jsonl")
    assert len(rows) == 1 + mini_loop.RETRIES and {(row["task"], row["upstream"], row["status"], row["forwarded"])
                                                   for row in rows} == {("F1-l1-0001.s1", "cerebras", 502, False)}
    [record] = read_jsonl(out / "records.jsonl")
    assert record["execution"]["status"] == "infra_error" and validate.validate("run-record", record) == []


def test_vb_run_meters_every_task_through_the_proxy(places):
    def respond(body: dict) -> str:  # look once, then submit
        return bash("echo VB_SUBMIT") if any(m["role"] == "assistant" for m in body["messages"]) else bash("ls")

    with StubServer(respond) as stub:
        assert run_vb(places, stub.url, "--proxy") == 0
        calls = len(stub.requests)
    out = run_dir(places)
    assert json.loads((out / "manifest.json").read_text())["proxy"] == {"log": "proxy.jsonl",
                                                                        "profile": {"name": "clean"}}
    rows, booked = read_jsonl(out / "proxy.jsonl"), read_jsonl(out / "ledger.jsonl")
    assert len(rows) == calls == 4 and all(row["forwarded"] and row["status"] == 200 for row in rows)
    for key in ("F1-l1-0001.s1", "F1-l1-0002.s1"):  # every row carries its task's key, and the meter is the ledger
        metered = [row for row in rows if row["task"] == key]
        ledger_rows = [row for row in booked if row["attempt_key"].startswith(f"run-1/{key}:")]
        assert len(metered) == 2 and ledger_rows
        assert sum(row["api_equiv_usd"] for row in ledger_rows) == pytest.approx(
            sum(row["api_equiv_usd"] for row in metered), rel=1e-12)


def test_the_roko_arm_through_the_proxy_is_metered_by_its_task_key(places, tmp_path):
    binary = tmp_path / "bin" / "roko"
    binary.parent.mkdir()
    binary.write_text(CALLING_ROKO.replace("__PYTHON__", sys.executable).replace("__CALLS__", "1"))
    binary.chmod(0o755)
    arm = tmp_path / "roko_test.toml"
    arm.write_text((layout.ARMS_DIR / "roko_fixed.toml").read_text().replace(
        'binary = "target/debug/roko"', f"binary = {json.dumps(str(binary))}"))
    with StubServer(lambda body: "Done.") as stub:
        assert vb.main(["run", "--experiment", "TEST-OFFLINE", "--run-id", "run-1", "--stream", TOY_STREAM,
                        "--arm", str(arm), "--model", "gpt-oss-120b", "--limit", "1", "--provider-url", stub.url,
                        "--proxy", "--results", str(places["results"]), "--work", str(places["work"]),
                        "--secret-file", str(places["secret"])]) == 0
        assert [request["model"] for request in stub.requests] == ["gpt-oss-120b"]
    out = run_dir(places)
    [row] = read_jsonl(out / "proxy.jsonl")
    assert (row["task"], row["model_reported"], row["usage_source"]) == ("F1-l1-0001.s1", "gpt-oss-120b", "reported")
    [record] = read_jsonl(out / "records.jsonl")
    [attempt] = record["execution"]["attempts"]
    assert record["execution"]["status"] == "completed" and attempt["checks"] == [] and attempt["calls"] == 1
    assert attempt["model_reported"] == "gpt-oss-120b" and attempt["usage"]["tokens_in"] == row["usage"]["tokens_in"]
    assert record["costs"]["api_equiv_usd"] == pytest.approx(row["api_equiv_usd"]) and row["api_equiv_usd"] > 0


def test_proxy_caps_meter_check_and_bundle(places, tmp_path, monkeypatch):
    # gap-60654d: the proxy holds every task to the arm's input_tokens_per_task, and a task it cuts off ends
    # aborted_cap; each record carries the proxy's own cost, and drift past ±5% is flagged; the report's bundle
    # carries the proxy's log cut to its meter fields.
    binary = tmp_path / "bin" / "roko"
    binary.parent.mkdir()
    binary.write_text(CALLING_ROKO.replace("__PYTHON__", sys.executable).replace("__CALLS__", "10"))
    binary.chmod(0o755)
    arm = tmp_path / "roko_capped.toml"  # each call reads 1,070 input tokens, so the cap refuses a task's third
    arm.write_text((layout.ARMS_DIR / "roko_fixed.toml").read_text().replace(
        'binary = "target/debug/roko"', f"binary = {json.dumps(str(binary))}").replace(
        "input_tokens_per_task = 450000", "input_tokens_per_task = 2000"))

    def run(run_id: str, arm: str, url: str) -> int:
        return vb.main(["run", "--experiment", "TEST-OFFLINE", "--run-id", run_id, "--stream", TOY_STREAM, "--arm", arm,
                        "--model", "gpt-oss-120b", "--provider-url", url, "--proxy", "--results",
                        str(places["results"]), "--work", str(places["work"]), "--secret-file", str(places["secret"])])

    with StubServer(lambda body: "Done.") as stub:
        assert run("run-1", str(arm), stub.url) == 0
        served = len(stub.requests)
    out = run_dir(places)
    rows = read_jsonl(out / "proxy.jsonl")
    for key in ("F1-l1-0001.s1", "F1-l1-0002.s1"):  # the cap is per task: each gets all of it
        calls = [row for row in rows if row["task"] == key]
        assert [(row["status"], row["refused"]) for row in calls] == [(200, None), (200, None),
                                                                      (403, "input_token_cap")]
    assert served == 4  # a refused call never reaches the provider
    for record in by_instance(out / "records.jsonl").values():
        assert (record["execution"]["status"], record["execution"]["reason"]) == ("aborted_cap", "input_token_cap")
        assert record["vs"]["label"] == 0 and validate.validate("run-record", record) == []
        assert record["costs"]["meter_cross_check_usd"] == pytest.approx(record["costs"]["api_equiv_usd"])
        assert record["costs"]["api_equiv_usd"] > 0
    assert not (out / "errors.jsonl").exists()  # the meter and the ledger agree: nothing to flag

    bundle = tmp_path / "bundle"
    assert vb.main(["report", "--experiment", "TEST-OFFLINE", "--results", str(places["results"]), "--out",
                    str(tmp_path / "metrics.json"), "--bundle", str(bundle)]) == 0
    bundled = read_jsonl(bundle / "run-1" / "proxy.jsonl")
    assert len(bundled) == len(rows) and all(row == {key: full[key] for key in row} for row, full in zip(bundled, rows))
    assert all({"task", "usage", "api_equiv_usd"} <= set(row) and not {"profile", "fault", "path"} & set(row)
               for row in bundled)

    # A proxy figure 10% above the ledger's is drift past ±5%: flagged in errors.jsonl, and the record keeps both.
    real_state = faultproxy.FaultProxy.state

    def inflated(proxy: faultproxy.FaultProxy) -> dict:
        state = real_state(proxy)
        for meter in state["tasks"]:
            meter["api_equiv_usd"] *= 1.1
        return state

    monkeypatch.setattr(faultproxy.FaultProxy, "state", inflated)
    with StubServer(lambda body: bash("echo VB_SUBMIT")) as stub:
        assert run("run-2", "cheap_direct", stub.url) == 0
    drifted = places["results"] / "TEST-OFFLINE" / "run-2"
    for record in by_instance(drifted / "records.jsonl").values():
        assert record["costs"]["meter_cross_check_usd"] == pytest.approx(1.1 * record["costs"]["api_equiv_usd"])
    flags = read_jsonl(drifted / "errors.jsonl")
    assert sorted(flag["task"] for flag in flags) == ["F1-l1-0001.s1", "F1-l1-0002.s1"]
    assert all(flag["stage"] == "meter check" and "±5%" in flag["error"] for flag in flags)


def test_agent_env_is_an_allowlist(tmp_path, monkeypatch):
    for name, value in {"VB_SECRET_FILE": "/somewhere", "CEREBRAS_API_KEY": FAKE_KEY, "OPENAI_API_KEY": FAKE_KEY,
                        "GITHUB_TOKEN": FAKE_KEY, "ROKO_CONFIG": "/x/roko.toml"}.items():
        monkeypatch.setenv(name, value)
    home = tmp_path / "home"
    env = agent_env.build(home=home)
    assert not [name for name in env if name.startswith(("VB_", "ROKO_")) or agent_env.FORBIDDEN_NAME.search(name)]
    assert FAKE_KEY not in "".join(env.values()) and env["HOME"] == str(home)
    version = subprocess.run(["python3", "-c", "import sys; print(sys.version_info >= (3, 11))"], env=env,
                             capture_output=True, text=True, check=True).stdout.strip()
    assert version == "True"
    with pytest.raises(agent_env.AgentEnvError):
        agent_env.build(home=home, extra={"VB_SECRET": "x" * 40})
    with pytest.raises(agent_env.AgentEnvError):
        agent_env.build(home=home, forbidden_values=[str(home)])


def test_governor_ends_attempts_and_bounds_spend():
    row = ledger.load_snapshot().row("gpt-oss-120b")
    now = [0.0]
    limits = caps.Caps(turns_per_attempt=2, turns_per_task=3, input_tokens_per_attempt=1000,
                       input_tokens_per_task=5000, wallclock_s=10, model_calls_per_task=4, usd_per_task=1.0)
    governor = caps.Governor(limits, price_row=row, clock=lambda: now[0])
    governor.start_attempt()
    assert governor.before_call(600) is None
    governor.after_call(input_tokens=600, input_bound=600, cost_usd=0.001, completed=True)
    assert governor.before_call(600) == caps.Stop("end_attempt", "attempt_input_tokens")
    assert governor.before_call(300) is None
    governor.after_call(input_tokens=300, input_bound=300, cost_usd=0.001, completed=True)
    assert governor.before_call(10) == caps.Stop("end_attempt", "attempt_turns")
    governor.start_attempt()
    governor.after_call(input_tokens=None, input_bound=100, cost_usd=None, completed=True)  # usage never came back
    assert governor.spent_usd == pytest.approx(0.002 + ledger.worst_case_usd(row, input_tokens=100,
                                                                             output_tokens=8192))
    assert governor.before_call(10) == caps.Stop("aborted_cap", "task_turns")
    assert governor.before_tool("ls  -a") is None and governor.before_tool("ls -a") is None
    now[0] = 11
    assert governor.before_call(10) == caps.Stop("timeout", "wallclock")
    broke = caps.Governor(caps.Caps(usd_per_task=0.001), price_row=row)
    broke.start_attempt()
    assert broke.before_call(10_000) == caps.Stop("aborted_cap", "usd")
    assert caps.worst_task_usd(caps.Caps(), row) == pytest.approx((300_000 * 0.35 + 30 * 8192 * 0.75) / 1e6)
    with pytest.raises(caps.CapError):
        caps.Caps.from_table({"turns_per_attempt": 0})
    with pytest.raises(caps.CapError):
        caps.Caps.from_table({"turn_per_attempt": 12})


def test_prices_come_from_the_snapshot_and_unknown_models_cost_null(tmp_path):
    snapshot = ledger.load_snapshot()
    assert snapshot.row("gpt-5.4-2026-03-05") is snapshot.row("gpt-5.4") and snapshot.row("no-such-model") is None
    usage = ledger.vb_usage(prompt_tokens=1_000_000, completion_tokens=100_000, cached_tokens=400_000)
    cost = ledger.price(usage, snapshot.row("gpt-5.4"))
    assert cost.api_equiv_usd == pytest.approx(0.6 * 2.50 + 0.4 * 0.25 + 0.1 * 15.00)
    assert cost.without_cache_usd == pytest.approx(2.50 + 1.50) and cost.source == "provider_usage"
    assert ledger.price(usage, None) == ledger.Cost(None, None, "unknown")
    assert ledger.price(None, snapshot.row("gpt-5.4")) == ledger.Cost(None, None, "unknown")
    book = ledger.Ledger(tmp_path / "ledger.jsonl", line="BL0", experiment_id="T", run_id="r",
                         price_snapshot_id=snapshot.id)
    book.append(attempt_key="r/x:1", provider="cerebras", model_reported="mystery-model", usage=usage,
                cost=ledger.price(usage, None), billed=True, reserved_usd=0.3)
    [row] = read_jsonl(tmp_path / "ledger.jsonl")
    assert row["api_equiv_usd"] is None and row["source"] == "unknown" and validate.validate("ledger", row) == []
    assert book.spent_bound_usd == pytest.approx(0.3)  # an unknown cost counts at its reservation


def test_usage_reads_a_top_level_cached_tokens():
    # bug-b70d40: Moonshot reports its cache reads as a top-level `cached_tokens`, OpenAI and Cerebras inside
    # `prompt_tokens_details`. Either way they are priced at the cache-read rate, the way mini_loop prices a call.
    def usage_of(raw: dict) -> dict:
        usage = provider.parse_completion({"choices": [{"message": {"content": "ok"}}], "usage": raw}).usage
        return ledger.vb_usage(usage.prompt_tokens, usage.completion_tokens, usage.cached_tokens,
                               usage.reasoning_tokens)

    row = ledger.load_snapshot().row("kimi-k2.6")
    top = {"prompt_tokens": 1000, "completion_tokens": 100, "cached_tokens": 600}
    nested = {"prompt_tokens": 1000, "completion_tokens": 100, "prompt_tokens_details": {"cached_tokens": 600}}
    for raw in (top, nested):
        assert usage_of(raw)["tokens_cache_read"] == 600
        assert ledger.price(usage_of(raw), row).api_equiv_usd == pytest.approx(
            (400 * row["input"] + 600 * row["cache_read"] + 100 * row["output"]) / 1e6)
    # A nested count wins over a top-level one, a count above prompt_tokens is capped, and a bad count is 0: the
    # same reading as the proxy's meter, so the ledger and the meter agree on every layout.
    cases = [({**nested, "cached_tokens": 999}, 600), ({**top, "prompt_tokens_details": None}, 600),
             ({**top, "prompt_tokens_details": {}}, 600), ({**top, "prompt_tokens_details": {"cached_tokens": 0}}, 0),
             ({**top, "cached_tokens": 5000}, 1000), ({**top, "cached_tokens": True}, 0),
             ({**top, "cached_tokens": -3}, 0), ({**top, "cached_tokens": "600"}, 0)]
    for raw, cached in cases:
        assert usage_of(raw)["tokens_cache_read"] == cached, raw
        metered = faultproxy.usage_classes(raw)
        assert usage_of(raw) == {key: metered[key] for key in usage_of(raw)}, raw


def test_replies_need_exactly_one_bash_block(tmp_path):
    assert mini_loop.parse_command(bash("ls -la")) == "ls -la"
    assert mini_loop.parse_command("no block here") is None
    assert mini_loop.parse_command(bash("ls") + bash("pwd")) is None
    result = mini_loop.run_command("echo VB_SUBMIT", cwd=tmp_path, env=dict(os.environ), timeout_s=10,
                                   observation_chars=100)
    assert result.submitted and result.returncode == 0
    slow = mini_loop.run_command("sleep 5", cwd=tmp_path, env=dict(os.environ), timeout_s=0.5, observation_chars=100)
    assert slow.timed_out and "timeout" in slow.observation
