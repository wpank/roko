"""The Roko arm's process tree in the loopback-only network sandbox (gap-0bd49a, 3304).

`vb run` starts every roko process of a task under the rule `loopback:<port>`, the port of the endpoint Roko calls,
here the metering proxy's. A fake roko (no binary needed) calls the model through the proxy, tries another local
port, DNS and a Unix socket outside its workspace, and binds and reaches one inside it, as Roko's own inject socket.
The `real_roko` variant has real Roko's `bash` tool, which runs the agent's commands, try another port; it runs when
there is a binary (`target/debug/roko`, or `$VB_TEST_ROKO_BIN`). Off macOS there is no sandbox (gap-29ac83), and the
tests skip; `-rs` prints why.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest \
        benchmarks/viabilitybench/driver/test_run_roko_sandbox.py -q -rs
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import run_roko
import validate
from stub_provider import StubServer
from test_run_roko import (PIN, REAL_ROKO, SOLUTION, ToolStub, arm_with, places,  # noqa: F401 (a fixture)
                           read_jsonl, real_roko, run_vb)
from test_sandbox_net import Listener, macos_only, socket_dir  # noqa: F401 (fixtures)

NET_ROKO = r'''#!__PYTHON__
"""A stand-in for roko: on `plan run` it calls the model through the endpoint its roko.toml names, tries the network
and two Unix sockets, notes what each attempt did, and records one passed attempt as Roko does."""
import datetime, json, os, socket, sys, tomllib, urllib.parse, urllib.request
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


def attempt(name, action):
    try:
        action()
        seen[name] = "connected"
    except OSError as err:
        seen[name] = "refused %s" % (err.errno or type(err).__name__)


def unix(path):
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(5)
    client.connect(str(path))
    client.close()


seen = {"base_url": provider["base_url"]}
request = urllib.request.Request(provider["base_url"] + "/chat/completions", data=json.dumps(
    {"model": model, "messages": [{"role": "user", "content": "Implement clamp."}]}).encode(),
    headers={"Content-Type": "application/json", "Authorization": "Bearer " + os.environ[provider["api_key_env"]]})
served = json.loads(urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request, timeout=30).read())
attempt("other_port", lambda: socket.create_connection(("127.0.0.1", __OTHER__), timeout=5).close())
attempt("dns", lambda: socket.getaddrinfo("example.com", 443))
attempt("outside_socket", lambda: unix(__OUTSIDE__))
inject = repo / ".roko" / "runtime" / "inject"  # where Roko binds its per-run inject socket
inject.mkdir(parents=True)
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(str(inject / ("%d.sock" % os.getpid())))
server.listen(1)
attempt("workspace_socket", lambda: unix(inject / ("%d.sock" % os.getpid())))
server.close()
Path(__LOG__).write_text(json.dumps(seen))
(repo / "calc" / "ops.py").write_text(__SOLUTION__)
roko, now = repo / ".roko", datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")
(roko / "state" / "graph" / slug).mkdir(parents=True)
(roko / "episodes.jsonl").write_text(json.dumps({
    "task_id": "T01", "model": model, "backend": "cerebras", "success": True, "turns": 1, "completed_at": now,
    "extra": {"plan_id": slug, "attempt_key": "graph-%s-1:%s:T01:1" % (slug, slug),
              "model_reported": served["model"], "model_mismatch": served["model"] != model}}) + "\n")
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps(
    {"plan_id": slug, "status": "succeeded", "extensions": {"roko.gate.verdict@1": {"value": {"verdicts": {
        "T01": "passed"}}}}}))
'''


class NetworkToolStub(ToolStub):
    """Plays Roko's implementer: run a command that tries the other port with Roko's `bash` tool, then read the file,
    write the fix (Roko's implementer contract wants a read before an edit) and finish."""

    def __init__(self, port: int) -> None:
        self.command = (f"python3 -c 'import socket; socket.create_connection((\"127.0.0.1\", {port}), timeout=5)'; "
                        "echo socket_exit=$?")
        super().__init__()

    def reply(self, messages: list[dict]) -> dict:
        system = " ".join(str(m.get("content") or "") for m in messages if m.get("role") == "system")
        turn = sum(1 for m in messages if m.get("role") == "assistant")
        if "role_identity" not in system or turn >= 3:
            return {"role": "assistant", "content": "Done." if "role_identity" in system else "0.5"}
        name, arguments = [("bash", {"command": self.command}), ("read_file", {"path": "calc/ops.py"}),
                           ("write_file", {"path": "calc/ops.py", "content": SOLUTION})][turn]
        return {"role": "assistant", "content": None, "tool_calls": [
            {"id": f"call_{turn}", "type": "function", "function": {"name": name, "arguments": json.dumps(arguments)}}]}


@macos_only
def test_roko_arm_process_cannot_reach_the_network(places, tmp_path, socket_dir):
    """3304: the fake roko reaches the model through the metering proxy, and nothing else: another loopback port, DNS
    and a Unix socket outside its workspace all fail, while a socket it binds in its workspace works. The attempt
    still settles on the proxy's meter, and the record names the rule."""
    log = tmp_path / "net-roko.json"
    places = {**places, "work": socket_dir / "work"}  # Roko's socket paths in the workspace stay under 104 bytes
    with Listener() as other, Listener(socket_dir / "outside.sock") as outside, \
            StubServer(lambda body: "Done.") as stub:
        binary = tmp_path / "bin" / "roko"
        binary.parent.mkdir()
        binary.write_text(NET_ROKO.replace("__PYTHON__", sys.executable).replace("__OTHER__", str(other.port))
                          .replace("__OUTSIDE__", repr(outside.target)).replace("__LOG__", repr(str(log)))
                          .replace("__SOLUTION__", repr(SOLUTION)))
        binary.chmod(0o755)
        assert run_vb(places, arm_with(tmp_path, binary), stub.url, "--proxy", "--transcripts") == 0
        assert (other.connections, outside.connections) == (0, 0)
        model_calls = len(stub.requests)
    seen = json.loads(log.read_text())
    proxy_port = int(seen.pop("base_url").split(":")[2].split("/")[0])
    assert seen == {"other_port": "refused 1", "dns": "refused 8", "outside_socket": "refused 1",
                    "workspace_socket": "connected"}
    assert model_calls == 1 and proxy_port != other.port  # the one call went through the proxy, to the stub

    out = places["results"] / "TEST-ROKO" / "run-1"
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert (record["execution"]["status"], record["vs"]["label"]) == ("completed", 1), record["execution"]["reason"]
    assert record["provenance"]["network_policy"] == {"network": f"loopback:{proxy_port}",
                                                      "sandbox": "sandbox-exec+net", "unix_sockets": "workspace"}
    [attempt] = record["execution"]["attempts"]  # settled on the proxy's meter: one billed call, priced
    assert attempt["calls"] == 1 and attempt["checks"] == [] and record["costs"]["api_equiv_usd"] > 0
    assert [row["task"] for row in read_jsonl(out / "proxy.jsonl")] == ["F1-l1-0001.s1"]
    assert len(read_jsonl(out / "ledger.jsonl")) == 1
    # The transcript keeps roko's own command lines, not the sandbox's profile.
    transcript = json.loads((out / record["provenance"]["transcript_ref"]).read_text())
    assert [event["argv"][0] for event in transcript if "argv" in event] == [str(binary)] * 2
    assert run_roko.network_rule(run_roko.provider.Endpoint("cerebras", "http://127.0.0.1:8123/v1")) == \
        "loopback:8123"
    assert run_roko.network_rule(run_roko.provider.Endpoint("cerebras", "https://api.cerebras.ai/v1")) == "none"


@macos_only
@real_roko
def test_real_roko_runs_in_the_sandbox_and_its_bash_tool_cannot_reach_the_network(places, tmp_path):
    """3304 with the real binary: Roko solves the task through its loopback endpoint, and its `bash` tool, which runs
    the agent's commands, cannot open a connection to another local port."""
    with Listener() as other:
        stub = NetworkToolStub(other.port)
        try:
            assert run_vb(places, arm_with(tmp_path, REAL_ROKO), stub.url) == 0
        finally:
            stub.server.shutdown()
        assert other.connections == 0
    [record] = read_jsonl(places["results"] / "TEST-ROKO" / "run-1" / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert (record["execution"]["status"], record["vs"]["label"]) == ("completed", 1), record["execution"]["reason"]
    assert record["provenance"]["network_policy"] == {"network": f"loopback:{stub.url.split(':')[2].split('/')[0]}",
                                                      "sandbox": "sandbox-exec+net", "unix_sockets": "workspace"}
    # The tool ran the command inside Roko, and the model saw it fail.
    results = [json.dumps(request.get("messages")) for request in stub.requests]
    assert any("socket_exit=1" in text for text in results), results[-1]
    assert {request["model"] for request in stub.requests} == {PIN}
