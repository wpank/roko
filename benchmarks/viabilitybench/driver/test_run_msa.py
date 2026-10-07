"""Offline tests of the mini-swe-agent runner (3317): it runs the fake `mini` CLI as a subprocess, pointed at the
metering proxy, and settles one attempt from the proxy's meter and the trajectory file's exit status.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_msa.py -q
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

import layout
import validate
import vb
from common import hmac_seed
from stub_provider import StubServer

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
ARM = layout.ARMS_DIR / "cheap_direct_msa.toml"
# A stand-in for the real `mini` CLI (driver/run_msa.py never imports the real one): it reads the model, the proxy's
# api_base and the placeholder key straight out of run_msa.py's own config file (no real YAML parser needed, since
# this is not real YAML consumption), makes `calls` chat calls through it, and writes a trajectory file in the shape
# run_msa.py reads back.
FAKE_MINI = r'''#!__PYTHON__
import json, os, re, sys, urllib.request
from pathlib import Path

args = sys.argv[1:]


def opt(flag):
    return args[args.index(flag) + 1] if flag in args else None


config = Path(opt("-c")).read_text(encoding="utf-8")


def field(name):
    match = re.search(name + r':\s*"((?:[^"\\]|\\.)*)"', config)
    return json.loads('"' + match.group(1) + '"') if match else None


model = field("model_name")
bare_model = model.split("/", 1)[1] if model and "/" in model else model
behaviour = json.loads(os.environ.get("VB_MSA_FAKE_BEHAVIOUR") or "{}")
calls, exit_status = behaviour.get("calls", 1), behaviour.get("exit_status", "Submitted")
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
for _ in range(calls):
    request = urllib.request.Request(field("api_base") + "/chat/completions", data=json.dumps(
        {"model": bare_model, "messages": [{"role": "user", "content": "Implement it."}]}).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + field("api_key")})
    opener.open(request, timeout=30).read()
output = Path(opt("-o"))
output.write_text(json.dumps({"info": {"model_stats": {"instance_cost": 0.0, "api_calls": calls},
                                       "exit_status": exit_status, "submission": "" if calls else None},
                             "messages": [], "trajectory_format": 1}))
sys.exit(0)
'''


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def fake_mini(tmp_path: Path) -> Path:
    binary = tmp_path / "bin" / "mini"
    binary.parent.mkdir()
    binary.write_text(FAKE_MINI.replace("__PYTHON__", sys.executable))
    binary.chmod(0o755)
    return binary


def arm_with(tmp_path: Path, binary: Path) -> Path:
    text = ARM.read_text()
    old = 'binary = ".venv-msa/bin/mini"'
    assert old in text
    arm = tmp_path / "msa_test.toml"
    arm.write_text(text.replace(old, f"binary = {json.dumps(str(binary))}"))
    return arm


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def test_msa_runner_meters_through_the_proxy(places, tmp_path):
    arm = arm_with(tmp_path, fake_mini(tmp_path))
    with StubServer(lambda body: "Done.") as stub:
        assert vb.main(["run", "--experiment", "TEST-MSA", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm",
                        str(arm), "--model", "gpt-oss-120b", "--seeds", "1", "--limit", "1", "--provider-url",
                        stub.url, "--proxy", "--results", str(places["results"]), "--work", str(places["work"]),
                        "--secret-file", str(places["secret"])]) == 0
        assert [request["model"] for request in stub.requests] == ["gpt-oss-120b"]
    out = places["results"] / "TEST-MSA" / "run-1"
    [row] = read_jsonl(out / "proxy.jsonl")
    assert (row["task"], row["model_reported"]) == ("F1-l1-0001.s1", "gpt-oss-120b") and row["api_equiv_usd"] > 0
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["arm"] == "cheap_direct"  # the same nominal arm as mini_loop.py's (module docstring)
    assert record["provenance"]["network_policy"]["harness"] == "mini-swe-agent"
    assert record["execution"]["status"] == "completed" and record["execution"]["reason"] == "submitted"
    [attempt] = record["execution"]["attempts"]
    assert attempt["model_requested"] == "gpt-oss-120b" and attempt["model_reported"] == "gpt-oss-120b"
    assert attempt["calls"] == 1 and attempt["turns"] == 1
    assert record["costs"]["api_equiv_usd"] == pytest.approx(row["api_equiv_usd"]) and row["api_equiv_usd"] > 0
    rows = read_jsonl(out / "ledger.jsonl")
    assert len(rows) == 1 and validate.validate("ledger", rows[0]) == []
