"""Offline tests of the mini-swe-agent runner (3317): it runs the fake `mini` CLI as a subprocess, pointed at the
metering proxy, and settles one attempt from the proxy's meter and the trajectory file's exit status.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_msa.py -q
"""

from __future__ import annotations

import json
import os
import shlex
import subprocess
import sys
import tomllib
from pathlib import Path

import pytest

import agent_env
import layout
import run_msa
import validate
import vb
from common import hmac_seed
from stub_provider import StubServer
from test_driver import BENCHMARK_MARK, host_paths_left_out
from test_secret import PROCARGS

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
# bug-e16450, test_no_msa_parent_names_the_benchmark: a fake `mini` as pip installs the real one, a console script whose
# first line names its venv's python. It makes one call through the proxy, as FAKE_MINI does, runs one command as
# mini-swe-agent 2.4.6's LocalEnvironment runs each of its agent's commands, in a shell that is its child
# (`Popen(command, shell=True, start_new_session=True)`), and logs that command's output with its own argv and
# executable. The command is __COMMAND__, PROCARGS on the shell's parent.
PARENT_MINI = r'''#!__PYTHON__
import json, os, re, subprocess, sys, urllib.request
from pathlib import Path

args = sys.argv[1:]
config = Path(args[args.index("-c") + 1]).read_text(encoding="utf-8")


def field(name):
    return json.loads('"' + re.search(name + r':\s*"((?:[^"\\]|\\.)*)"', config).group(1) + '"')


opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
body = {"model": field("model_name").split("/", 1)[-1], "messages": [{"role": "user", "content": "Implement it."}]}
opener.open(urllib.request.Request(field("api_base") + "/chat/completions", data=json.dumps(body).encode(),
    headers={"Content-Type": "application/json", "Authorization": "Bearer " + field("api_key")}), timeout=30).read()
shell = subprocess.Popen(__COMMAND__, shell=True, text=True, env=os.environ, stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT, start_new_session=True)
with open(__LOG__, "w", encoding="utf-8") as log:
    json.dump({"argv": sys.argv, "orig_argv": sys.orig_argv, "executable": sys.executable,
               "parent": shell.communicate(timeout=60)[0]}, log)
Path(args[args.index("-o") + 1]).write_text(json.dumps({
    "info": {"model_stats": {"instance_cost": 0.0, "api_calls": 1}, "exit_status": "Submitted", "submission": ""},
    "messages": [], "trajectory_format": 1}))
'''


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def fake_mini(tmp_path: Path) -> Path:
    """FAKE_MINI, run by the interpreter agents run: one inside the repository, such as this venv's, is refused."""
    binary = tmp_path / "bin" / "mini"
    binary.parent.mkdir()
    binary.write_text(FAKE_MINI.replace("__PYTHON__", str(agent_env._base_python())))
    binary.chmod(0o755)
    return binary


def arm_with(tmp_path: Path, binary: Path | str, name: str = "msa_test.toml") -> Path:
    text = ARM.read_text()
    old = f'binary = "{run_msa.DEFAULT_BINARY}"'
    assert old in text
    arm = tmp_path / name
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


def test_no_msa_parent_names_the_benchmark(places, tmp_path, monkeypatch, capsys):
    """bug-e16450: mini-swe-agent runs each of its agent's commands in a shell that is its child, and from inside the
    sandbox that shell reads the `mini` process's argv and start-up environment (`sysctl(KERN_PROCARGS2)`). So `mini`
    runs from a venv outside the repository whose path names no benchmark: the arm file's default, under the
    operator's home, here a temporary one. The fake `mini` is installed there as pip installs the real one, its first
    line naming the venv's python, and through `vb run`, with the arm file as it is, it logs its argv and executable
    and has its shell read its parent with PROCARGS: none of it names the benchmark, the repository, the experiment,
    the run or the results root, while the probe did read the `mini` process (the control). A binary inside the
    repository (the old `.venv-msa/` default), one whose path names the benchmark, a neutral link into the repository
    and one whose first line names an interpreter inside it are refused before anything runs."""
    named = Path(tomllib.loads(ARM.read_text())["msa"]["binary"])
    assert named.is_absolute() or str(named).startswith("~"), named  # a relative path lies in the benchmark tree
    home = tmp_path / "home"
    monkeypatch.setenv("HOME", str(home))
    binary = named.expanduser()
    assert not layout.within(binary, layout.REPO_ROOT) and not BENCHMARK_MARK.search(str(named)), binary
    assert binary.parent.name == "bin"
    venv = binary.parent.parent  # the arm's venv, made as the README says, from the interpreter agents run
    subprocess.run([str(agent_env._base_python()), "-m", "venv", "--without-pip", str(venv)], check=True, timeout=120)
    log = tmp_path / "mini.log"
    binary.write_text(PARENT_MINI.replace("__PYTHON__", str(venv / "bin" / "python")).replace(
        "__COMMAND__", repr(f"python3 -c {shlex.quote(PROCARGS)} $PPID")).replace("__LOG__", repr(str(log))))
    binary.chmod(0o755)
    assert run_msa.binary_path(vb.load_arm(str(ARM))) == binary

    def run(arm: Path, run_id: str) -> int:
        return vb.main(["run", "--experiment", "TEST-MSA", "--run-id", run_id, "--stream", TOY_STREAM, "--arm",
                        str(arm), "--model", "gpt-oss-120b", "--seeds", "1", "--limit", "1", "--provider-url",
                        stub.url, "--proxy", "--results", str(places["results"]), "--work", str(places["work"]),
                        "--secret-file", str(places["secret"])])

    with StubServer(lambda body: "Done.") as stub:
        assert run(ARM, "e-oss-1") == 0
        # Refused before anything runs: the old default inside the benchmark tree, a path that names the benchmark,
        # a link with a neutral name into the repository, and a copy whose first line names the test venv's python.
        link = home / "tools" / "mini"
        link.parent.mkdir()
        link.symlink_to(layout.DRIVER_DIR / "vb.py")
        copy = home / "copied" / "mini"
        copy.parent.mkdir()
        copy.write_text(binary.read_text().replace(str(venv / "bin" / "python"), sys.executable, 1))
        copy.chmod(0o755)
        capsys.readouterr()
        for number, (wrong, why) in enumerate(((".venv-msa/bin/mini", "lies inside the repository"),
                                               (home / "vb-tools" / "mini", "names the benchmark ('vb' in"),
                                               (link, "lies inside the repository"),
                                               (copy, f"the interpreter {sys.executable} that the mini binary"))):
            assert run(arm_with(tmp_path, wrong, f"wrong-{number}.toml"), f"wrong-{number}") == 2, wrong
            err = capsys.readouterr().err
            assert why in err and "bug-e16450" in err, (wrong, err)
            assert not (places["results"] / "TEST-MSA" / f"wrong-{number}").exists()
        requests = len(stub.requests)
    assert requests == 1  # the one run's one call
    [record] = read_jsonl(places["results"] / "TEST-MSA" / "e-oss-1" / "records.jsonl")
    assert record["execution"]["status"] == "completed" and record["provenance"]["network_policy"]["harness"] == \
        "mini-swe-agent"
    seen = json.loads(log.read_text())
    assert seen["executable"] == str(venv / "bin" / "python") and seen["orig_argv"][1] == str(binary)
    texts = {"argv": " ".join(seen["orig_argv"]), "executable": seen["executable"], "parent": seen["parent"]}
    results = sorted({str(places["results"]), os.path.realpath(places["results"])})
    for name, text in texts.items():
        mentioned = [mark for mark in ("TEST-MSA", "e-oss-1", str(layout.REPO_ROOT), *results) if mark in text]
        marks = sorted({match.group(0) for match in BENCHMARK_MARK.finditer(host_paths_left_out(tmp_path)(text))})
        assert mentioned == [] and marks == [], (name, mentioned, marks, text)
    # The control: the probe read the mini process, its interpreter's argv with the binary and the model.
    assert str(binary) in seen["parent"] and "-m openai/gpt-oss-120b" in seen["parent"], seen["parent"]
