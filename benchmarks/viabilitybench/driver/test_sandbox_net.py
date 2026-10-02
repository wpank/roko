"""The agents' network sandbox (gap-0bd49a, decision 3301's option (b)): on macOS, no agent process reaches a network
its arm does not need. `common.sandbox` builds the rules, and the direct loop runs every agent command without any.

The tests use local listeners only. A connection the sandbox let through would reach a listener on this host and be
counted there, so no test needs the internet or touches it. Off macOS there is no sandbox (`sandbox.KIND` "none",
gap-29ac83), and the tests that need one skip; `-rs` prints why. `Listener` is shared with the other arms' tests.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_sandbox_net.py -q -rs
"""

from __future__ import annotations

import json
import os
import re
import shutil
import socket
import stat
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

import pytest

import layout
import mini_loop
import secret
import validate
import vb
from common import hmac_seed, sandbox
from stub_provider import StubServer, bash, scripted

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
CORRECT = ("cat > calc/ops.py <<'EOF'\ndef clamp(value, low, high):\n"
           "    \"\"\"Return value limited to the range [low, high].\"\"\"\n    return max(low, min(value, high))\nEOF")
macos_only = pytest.mark.skipif(sandbox.KIND == "none", reason=(
    "no sandbox-exec on this host, so agent processes keep the network here (gap-29ac83 owns Linux)"))
# A raw socket and a curl, each printing its exit status; the listener counts any connection that got through.
CONNECT = ("python3 -c 'import socket, sys; socket.create_connection((\"127.0.0.1\", int(sys.argv[1])), timeout=5)' "
           "{port}; echo socket_exit=$?")
CURL = "curl -sS --max-time 5 http://127.0.0.1:{port}/hidden.py; echo curl_exit=$?"
PROBE = r'''
import socket, sys
for target in sys.argv[1:]:
    try:
        if target.startswith("/"):
            client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            client.settimeout(5)
            client.connect(target)
        else:
            client = socket.create_connection(("127.0.0.1", int(target)), timeout=5)
        client.close()
        print(target, "connected")
    except OSError as err:
        print(target, "refused", err.errno)
'''


class Listener:
    """A listener on the loopback, or on a Unix socket at `path`, that counts the connections it accepts."""

    SENTINEL = b"vb-listener-sentinel"

    def __init__(self, path: Path | None = None) -> None:
        self.path = path
        self.server = socket.socket(socket.AF_UNIX if path else socket.AF_INET, socket.SOCK_STREAM)
        self.server.bind(str(path) if path else ("127.0.0.1", 0))
        self.server.listen(16)
        self.port = 0 if path else self.server.getsockname()[1]
        self._taken: list[bool] = []  # per accepted connection, in order: whether it was a sentinel
        self._sentinels = 0
        self._thread = threading.Thread(target=self._accept, daemon=True)

    @property
    def target(self) -> str:
        """What `PROBE` takes: the socket's path, or the port."""
        return str(self.path) if self.path else str(self.port)

    @property
    def connections(self) -> int:
        """The connections that reached the listener so far. It sends a sentinel connection of its own and waits until
        the accept loop, which takes connections in the order they came, has taken it; sentinels do not count."""
        with socket.socket(self.server.family, socket.SOCK_STREAM) as client:
            client.connect(self.server.getsockname())
            client.sendall(self.SENTINEL)
        self._sentinels += 1
        deadline = time.monotonic() + 10
        while self._taken.count(True) < self._sentinels:
            assert time.monotonic() < deadline, "the listener stopped accepting"
            time.sleep(0.01)
        return self._taken.count(False)

    def __enter__(self) -> Listener:
        self._thread.start()
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.server.close()

    def _accept(self) -> None:
        while True:
            try:
                client, _ = self.server.accept()
            except OSError:
                return
            with client:
                client.settimeout(5)
                try:
                    first = client.recv(len(self.SENTINEL))
                except OSError:
                    first = b""
            self._taken.append(first == self.SENTINEL)


def probe(argv: list[str], *listeners: Listener) -> dict[str, str]:
    """Run PROBE as `argv` (a sandbox prefix or nothing) against the listeners: target -> "connected" or "refused"."""
    ran = subprocess.run([*argv, sys.executable, "-c", PROBE, *(listener.target for listener in listeners)],
                         capture_output=True, text=True, timeout=60, check=False)
    assert ran.returncode == 0, ran.stderr
    return {line.split()[0]: line.split()[1] for line in ran.stdout.splitlines()}


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


@pytest.fixture
def socket_dir():
    """A short directory for Unix sockets, whose paths must stay under 104 bytes."""
    path = Path(tempfile.mkdtemp(prefix="vbs-"))
    yield path
    shutil.rmtree(path, ignore_errors=True)


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def test_network_rules_build_the_profile_and_refuse_unknown_ones(tmp_path):
    # The rules and the profile, on every host.
    assert sandbox.ports(None) == () and sandbox.ports("none") == () and sandbox.ports("loopback:443,8080") == (443,
                                                                                                                 8080)
    assert sandbox.loopback(9, 65535) == "loopback:9,65535"
    for bad in ("", "all", "loopback:", "loopback:0", "loopback:65536", "loopback:80,x", "loopback: 80", 443):
        with pytest.raises(ValueError, match="network rule"):
            sandbox.ports(bad)
        with pytest.raises(ValueError):  # refused before anything runs, on every host
            sandbox.command(["true"], deny=(), network=bad)
    with pytest.raises(ValueError):
        sandbox.loopback()
    secret_file = tmp_path / "secret"
    none = sandbox.profile([secret_file], "none")
    assert none == (f'(version 1) (allow default) (deny file* (subpath "{os.path.realpath(secret_file)}"))'
                    " (deny network-outbound)")
    sockets = tmp_path / 'odd "dir"'
    loop = sandbox.profile([], sandbox.loopback(8080), [sockets])
    assert loop.endswith(' (deny network-outbound) (allow network-outbound (remote ip "localhost:8080"))'
                         f' (allow network-outbound (subpath "{os.path.realpath(tmp_path)}/odd \\"dir\\""))')
    assert sandbox.profile([secret_file]) == sandbox.profile([secret_file], None, [sockets])  # no rule, no network
    # `kind` names the confinement this host applies; "+net" only where a network rule does.
    assert sandbox.kind((), None) == "none" and sandbox.kind([secret_file]) == sandbox.KIND
    assert sandbox.command(["true"], deny=()) == ["true"]
    if sandbox.KIND == "none":  # nothing applies a rule here, and the record says so
        assert sandbox.kind((), "none") == "none" and sandbox.command(["true"], deny=(), network="none") == ["true"]
    else:
        assert sandbox.kind((), "none") == "sandbox-exec+net" == sandbox.kind([secret_file], "loopback:80")
        assert sandbox.command(["true"], deny=(), network="none") == [sandbox.SANDBOX_EXEC, "-p", sandbox.profile(
            [], "none"), "true"]


@macos_only
def test_loopback_rule_admits_only_its_ports_and_sockets(socket_dir):
    allowed_dir, other_dir = socket_dir / "ws", socket_dir / "elsewhere"
    allowed_dir.mkdir()
    other_dir.mkdir()
    with Listener() as proxy, Listener() as other, Listener(allowed_dir / "inject.sock") as inject, \
            Listener(other_dir / "other.sock") as elsewhere:
        everything = (proxy, other, inject, elsewhere)
        # Control: unconfined, every listener is reachable, so a refusal below is the sandbox's.
        assert set(probe([], *everything).values()) == {"connected"}
        loop = sandbox.command([], deny=(), network=sandbox.loopback(proxy.port), sockets=[allowed_dir])
        assert probe(loop, *everything) == {proxy.target: "connected", other.target: "refused",
                                            inject.target: "connected", elsewhere.target: "refused"}
        assert set(probe(sandbox.command([], deny=(), network="none"), *everything).values()) == {"refused"}
        assert sandbox.command([], deny=(), sockets=[allowed_dir]) == []  # sockets alone add no network rule
        assert [listener.connections for listener in everything] == [2, 1, 2, 1]


@macos_only
def test_direct_loop_shell_cannot_reach_the_network(places):
    """3303: every command of the direct loop's agent runs with no network on macOS. A stub agent tries a raw socket
    and a curl to a local listener, a curl to the model's own endpoint, and a look into the run's private task
    directories. All four fail, the task still completes with a correct clamp, and the record names the policy."""
    run_dir = places["results"] / "TEST-NET" / "run-1"
    with Listener() as target:
        stub: StubServer | None = None

        def respond(body: dict) -> str:
            steps = [bash(CONNECT.format(port=target.port), "Try a raw socket."),
                     bash(CURL.format(port=target.port), "Try curl."),
                     bash(f"curl -sS --max-time 5 -d '{{}}' {stub.url}/chat/completions; echo provider_exit=$?",
                          "Call the model myself."),
                     bash(f"ls {run_dir / 'private'}; echo private_exit=$?", "Look for the task files."),
                     bash(CORRECT, "Implement it."), bash("echo VB_SUBMIT")]
            return scripted({"": steps})(body)

        with StubServer(respond) as stub:
            assert vb.main(["run", "--experiment", "TEST-NET", "--run-id", "run-1", "--stream", TOY_STREAM,
                            "--arm", "cheap_direct", "--model", "gpt-oss-120b", "--seeds", "1", "--limit", "1",
                            "--provider-url", stub.url, "--results", str(places["results"]), "--work",
                            str(places["work"]), "--secret-file", str(places["secret"]), "--transcripts"]) == 0
            model_calls = len(stub.requests)
        assert target.connections == 0
        # Control: the same raw socket, unconfined, reaches the listener.
        subprocess.run(["bash", "-c", CONNECT.format(port=target.port)], capture_output=True, timeout=60, check=False)
        assert target.connections == 1
    assert model_calls == 6  # the driver's calls only: the agent's own request never reached the endpoint

    [record] = read_jsonl(run_dir / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["execution"]["status"] == "completed" and record["vs"]["label"] == 1
    assert record["provenance"]["network_policy"] == {"network": "none", "sandbox": "sandbox-exec+net"}
    assert record["provenance"]["canary_hits"] == 0
    transcript = json.loads((run_dir / record["provenance"]["transcript_ref"]).read_text())
    seen = [event for event in transcript if event.get("role") == "user" and "command" in event]

    def observed(start: str) -> str:
        [content] = [event["content"] for event in seen if event["command"].startswith(start)]
        return content

    assert "socket_exit=1" in observed("python3 -c") and "PermissionError" in observed("python3 -c")
    assert "curl_exit=7" in observed(f"curl -sS --max-time 5 http://127.0.0.1:{target.port}/")
    assert "provider_exit=7" in observed("curl -sS --max-time 5 -d")
    assert "private_exit=1" in observed("ls ") and "Operation not permitted" in observed("ls ")
    assert ".vb" not in observed("ls ") and "F1-l1-0001" not in observed("ls ")
    assert mini_loop.network_policy(()) == {"network": "none", "sandbox": "sandbox-exec+net"}


@macos_only
def test_agent_commands_cannot_touch_the_secret_or_the_key_file(places, tmp_path):
    """3303: the direct loop also denies its agent's commands every file the tripwire holds, so on macOS even the chmod
    that the tripwire exists to catch fails before it changes anything (test_secret.py has the tripwire's own tests,
    on an agent the sandbox leaves those files to)."""
    key = "csk-test-sandboxed-only-4b7e01d2"
    key_file = secret.create_keys(tmp_path / "key-config" / "keys", {"CEREBRAS_API_KEY": key})
    # The names are split, so no command names the secret file (census place `argv`).
    attempt = (f"for f in '{places['secret'].parent}/sec''ret' '{key_file.parent}/ke''ys'; do "
               'chmod 600 "$f"; cat "$f"; done 2>&1; echo tried')
    with StubServer(scripted({"": [bash(attempt, "Take the secret and the key."), bash(CORRECT, "Implement it."),
                                   bash("echo VB_SUBMIT")]})) as stub:
        arm_text, count = re.subn(r"(?m)^base_url = .*$", f'base_url = "{stub.url}"',
                                  (layout.ARMS_DIR / "cheap_direct.toml").read_text())
        assert count == 1
        (tmp_path / "keyed.toml").write_text(arm_text)  # an endpoint that names its key: the proxy sends it
        assert vb.main(["run", "--experiment", "TEST-NET", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm",
                        str(tmp_path / "keyed.toml"), "--model", "gpt-oss-120b", "--seeds", "1", "--limit", "1",
                        "--results", str(places["results"]), "--work", str(places["work"]), "--secret-file",
                        str(places["secret"]), "--key-file", str(key_file), "--proxy", "--transcripts"]) == 0
    run_dir = places["results"] / "TEST-NET" / "run-1"
    [record] = read_jsonl(run_dir / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["execution"]["status"] == "completed" and record["vs"]["label"] == 1
    assert record["provenance"]["canary_places"] == []  # no chmod reached either file, so the tripwire saw nothing
    transcript = json.loads((run_dir / record["provenance"]["transcript_ref"]).read_text())
    [tried] = [event["content"] for event in transcript if event.get("command") == attempt]
    assert tried.count("Operation not permitted") == 4 and "tried" in tried
    assert key not in tried and not secret.load(places["secret"]).find(tried)
    for path in (places["secret"], key_file):  # at rest again after the run
        assert stat.S_IMODE(path.stat().st_mode) == secret.REST_MODE
