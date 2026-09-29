"""Offline tests of the metering and fault proxy: local stub upstreams only, no provider and no spend.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -q
"""

from __future__ import annotations

import http.client
import json
import threading
import time
import urllib.parse
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

import faultproxy
import layout
import ledger
import vb
from common import hmac_seed
from stub_provider import StubServer, bash

MODEL = "gpt-5.4"  # its cache reads cost less than its input, so mixing up the token classes changes the cost
REQUEST = {"model": MODEL, "messages": [{"role": "user", "content": "Say hello."}]}
STREAM = {**REQUEST, "stream": True}
TOOLS = [{"type": "function", "function": {"name": "bash", "parameters": {"type": "object"}}}]
TOOL_CALL = {"id": "call_1", "type": "function", "function": {"name": "bash", "arguments": "{\"command\": \"ls\"}"}}
TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")


def raw_usage(n: int) -> dict:
    """The usage the stub reports for its n-th request: varied counts, with cached tokens reported two ways."""
    prompt, completion = 120 + 37 * n, 15 + 11 * n
    usage = {"prompt_tokens": prompt, "completion_tokens": completion, "total_tokens": prompt + completion,
             "completion_tokens_details": {"reasoning_tokens": n % 7}}
    cached = (29 * n) % prompt if n % 3 else 0
    if n % 4 == 1:
        usage["cached_tokens"] = cached  # a top-level count, S01 §4.4's "provider's equivalent"
    else:
        usage["prompt_tokens_details"] = {"cached_tokens": cached}
    return usage


def expected_classes(raw: dict) -> dict:
    """S01 §4.4's row for OpenAI-style usage, written out here independently of `faultproxy.usage_classes`."""
    cached = raw.get("prompt_tokens_details", {}).get("cached_tokens", raw.get("cached_tokens", 0))
    return {"tokens_in": raw["prompt_tokens"] - cached, "tokens_cache_read": cached, "tokens_cache_write_5m": 0,
            "tokens_cache_write_1h": 0, "tokens_out": raw["completion_tokens"],
            "tokens_reasoning": raw["completion_tokens_details"]["reasoning_tokens"]}


class StubUpstream:
    """A local OpenAI-compatible model: a JSON body, or server-sent events in a chunked body, with `raw_usage`.

    With `stream_usage=False` it plays a provider that ignores `stream_options.include_usage`; with `redirect_to` it
    answers every request with a 307 there. `requests` holds every request as (lower-cased headers, body),
    `reported` every usage object sent, in order, and `ports` the client ports the requests came from.
    """

    def __init__(self, *, stream_usage: bool = True, redirect_to: str | None = None) -> None:
        self.stream_usage = stream_usage
        self.redirect_to = redirect_to
        self.requests: list[tuple[dict, dict]] = []
        self.reported: list[dict] = []
        self.ports: set[int] = set()
        self._lock = threading.Lock()
        self._server = ThreadingHTTPServer(("127.0.0.1", 0), self._handler())
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self._server.server_address[1]}/v1"

    def __enter__(self) -> StubUpstream:
        self._thread.start()
        return self

    def __exit__(self, *exc_info: object) -> None:
        self._server.shutdown()
        self._server.server_close()

    def _handler(self) -> type[BaseHTTPRequestHandler]:
        stub = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def do_POST(self) -> None:  # noqa: N802 (the stdlib's name)
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                with stub._lock:
                    stub.requests.append(({k.lower(): v for k, v in self.headers.items()}, body))
                    stub.ports.add(self.client_address[1])
                    n = len(stub.requests)
                if stub.redirect_to:
                    self.send_response(307)
                    self.send_header("Location", stub.redirect_to)
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                usage = raw_usage(n)
                if body.get("stream"):
                    self._stream(body, usage)
                    return
                message = {"role": "assistant", "content": f"Hello {n}."}
                if body.get("tools"):
                    message["tool_calls"] = [TOOL_CALL]
                with stub._lock:
                    stub.reported.append(usage)
                data = json.dumps({"id": f"stub-{n}", "object": "chat.completion", "model": body["model"],
                                   "choices": [{"index": 0, "message": message, "finish_reason": "stop"}],
                                   "usage": usage}).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def _stream(self, body: dict, usage: dict) -> None:
                base = {"id": "stub", "object": "chat.completion.chunk", "model": body["model"]}
                deltas = [{"role": "assistant"}, {"content": "Hello"}, {"content": " there."}]
                if body.get("tools"):
                    deltas.append({"tool_calls": [{"index": 0, **TOOL_CALL}]})
                events = [{**base, "choices": [{"index": 0, "delta": delta, "finish_reason": None}]}
                          for delta in deltas]
                events.append({**base, "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]})
                if stub.stream_usage and (body.get("stream_options") or {}).get("include_usage") is True:
                    events.append({**base, "choices": [], "usage": usage})
                    with stub._lock:
                        stub.reported.append(usage)
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Transfer-Encoding", "chunked")
                self.end_headers()
                for data in [b"data: " + json.dumps(event).encode() + b"\n\n" for event in events] + [
                        b"data: [DONE]\n\n"]:
                    self.wfile.write(b"%x\r\n%s\r\n" % (len(data), data))
                self.wfile.write(b"0\r\n\r\n")

            def log_message(self, *args: object) -> None:
                pass

        return Handler


@dataclass
class Reply:
    status: int | None  # None when no response came
    headers: dict
    body: bytes
    error: str | None  # disconnected | incomplete | timeout

    def json(self) -> dict:
        return json.loads(self.body)

    def events(self) -> list:
        """The data of each server-sent event: a parsed JSON object, or the text (such as "[DONE]")."""
        out = []
        for block in self.body.split(b"\n\n"):
            data = b"\n".join(line[6:] for line in block.splitlines() if line.startswith(b"data: "))
            if data:
                out.append(json.loads(data) if data.startswith(b"{") else data.decode())
        return out

    def chunks(self) -> list[dict]:
        return [event for event in self.events() if isinstance(event, dict)]


def exchange(connection: http.client.HTTPConnection, method: str, path: str, body: dict | None,
             headers: dict) -> Reply:
    """One request, with a dropped connection or a cut body reported rather than raised."""
    connection.request(method, path, body=None if body is None else json.dumps(body),
                       headers={"Content-Type": "application/json", **headers})
    try:
        response = connection.getresponse()
    except http.client.RemoteDisconnected:
        return Reply(None, {}, b"", "disconnected")
    except TimeoutError:
        return Reply(None, {}, b"", "timeout")
    received = dict(response.getheaders())
    try:
        return Reply(response.status, received, response.read(), None)
    except http.client.IncompleteRead as err:
        return Reply(response.status, received, err.partial, "incomplete")


def call(method: str, url: str, body: dict | None = None, *, timeout: float = 10.0,
         headers: dict | None = None) -> Reply:
    """One request on a fresh connection."""
    parts = urllib.parse.urlsplit(url)
    connection = http.client.HTTPConnection(parts.hostname, parts.port, timeout=timeout)
    try:
        return exchange(connection, method, parts.path, body, headers or {})
    finally:
        connection.close()


class Session:
    """Requests to one URL over a kept-alive connection, opened again after a fault drops it. Thousands of fresh
    connections would exhaust the loopback's ephemeral ports while their closed sockets wait out TIME_WAIT."""

    def __init__(self, url: str) -> None:
        self.parts = urllib.parse.urlsplit(url)
        self.connection: http.client.HTTPConnection | None = None
        self.opened = 0

    def post(self, body: dict) -> Reply:
        if self.connection is None:
            self.connection = http.client.HTTPConnection(self.parts.hostname, self.parts.port, timeout=10.0)
            self.opened += 1
        reply = exchange(self.connection, "POST", self.parts.path, body, {})
        if reply.error:
            self.close()
        return reply

    def close(self) -> None:
        if self.connection is not None:
            self.connection.close()
            self.connection = None


def post(url: str, body: dict, **kwargs: object) -> Reply:
    return call("POST", url, body, **kwargs)


def completions(proxy: faultproxy.FaultProxy, name: str = "stub") -> str:
    return proxy.base_url(name) + "/chat/completions"


def read_log(path: Path, task: str | None) -> list[dict]:
    lines = [json.loads(line) for line in path.read_text().splitlines()]
    return sorted((line for line in lines if line["task"] == task), key=lambda line: line["ordinal"])


@pytest.fixture
def upstream():
    with StubUpstream() as stub:
        yield stub


@pytest.fixture
def log(tmp_path: Path) -> Path:
    return tmp_path / "proxy.jsonl"


@pytest.fixture
def proxy(upstream: StubUpstream, log: Path):
    with faultproxy.FaultProxy([faultproxy.Upstream("stub", upstream.url)], log_path=log) as running:
        yield running


# Each profile with what the client sees when its fault fires; latency is checked in the log, since timing a few
# milliseconds from the client would be flaky. Two rates (0.12, 0.33) are not whole per block of 20 requests.
RATE_PROFILES = [
    ({"name": "http_5xx", "p": 0.10, "seed": 1}, lambda reply: reply.status == 500),
    ({"name": "rate_limit", "p": 0.20, "retry_after": 7, "seed": 2},
     lambda reply: reply.status == 429 and reply.headers.get("Retry-After") == "7"),
    ({"name": "hang", "p": 0.05, "s": 0.01, "seed": 3}, lambda reply: reply.error == "disconnected"),
    ({"name": "truncate", "p": 0.12, "bytes": 64, "seed": 4},
     lambda reply: reply.error == "incomplete" and len(reply.body) == 64),
    ({"name": "latency", "ms": 3, "jitter": 2, "p": 0.30, "seed": 5}, None),
    ({"name": "schema_drift", "p": 0.33, "seed": 6},
     lambda reply: "usage" not in reply.json() and "stop_reason" in reply.json()["choices"][0]),
]
REQUESTS = 1000


def test_fault_rates_within_two_points(proxy, upstream, log):
    session, dropped = Session(completions(proxy)), 0
    for spec, seen in RATE_PROFILES:
        task, forwarded_before = f"rates/{spec['name']}", len(upstream.requests)
        proxy.configure(task=task, profile=spec)
        replies = [session.post(REQUEST) for _ in range(REQUESTS)]
        dropped += sum(reply.error is not None for reply in replies)
        lines = read_log(log, task)
        assert [line["ordinal"] for line in lines] == list(range(1, REQUESTS + 1))
        assert {line["fault_injected"] for line in lines} <= {None, spec["name"]}
        injected = [line["fault_injected"] is not None for line in lines]
        if seen is None:
            delays = [line["fault"]["delay_ms"] for line in lines if line["fault_injected"]]
            assert all(1 <= delay <= 5 for delay in delays)
            assert all(line["elapsed_ms"] >= line["fault"]["delay_ms"] - 0.1
                       for line in lines if line["fault_injected"])
        else:
            # Every logged fault reached the client, at the request the log names, and no other request was faulted.
            assert [seen(reply) for reply in replies] == injected
        assert all(reply.status == 200 and reply.error is None and reply.json()["usage"]
                   for reply, fired in zip(replies, injected) if not fired or spec["name"] == "latency")
        # Faults the proxy answers itself never reach the upstream; the others are forwarded.
        assert len(upstream.requests) - forwarded_before == sum(line["forwarded"] for line in lines)
        if spec["name"] in ("http_5xx", "rate_limit", "hang"):
            assert [line["forwarded"] for line in lines] == [not fired for fired in injected]
        rate = sum(injected) / REQUESTS
        assert abs(rate - spec["p"]) <= 0.02, f"{spec['name']}: injected {rate:.3f}, target {spec['p']}"
    session.close()
    # Kept-alive connections both ways: only faults that drop the client's connection open new ones.
    assert session.opened <= 1 + dropped and len(upstream.ports) <= faultproxy.IDLE_CONNECTIONS


def test_meter_equals_upstream_usage(upstream, log):
    with StubUpstream(stream_usage=False) as legacy, faultproxy.FaultProxy(
            [faultproxy.Upstream("stub", upstream.url), faultproxy.Upstream("legacy", legacy.url, stream_usage=False)],
            log_path=log) as proxy:
        url = completions(proxy)
        proxy.configure(task="meter")
        for _ in range(6):
            reply = post(url, REQUEST)
            assert reply.status == 200 and reply.json()["usage"] == upstream.reported[-1]
        for _ in range(6):  # the proxy asks for the usage chunk, and a client that did not ask never sees it
            reply = post(url, STREAM)
            assert reply.error is None and reply.events()[-1] == "[DONE]"
            assert not [chunk for chunk in reply.chunks() if "usage" in chunk]
        for _ in range(4):  # a client that asks gets it unchanged
            reply = post(url, {**STREAM, "stream_options": {"include_usage": True}})
            [last] = [chunk for chunk in reply.chunks() if chunk.get("usage")]
            assert last["usage"] == upstream.reported[-1] and last["choices"] == []
        # Faults that keep the usage from the client do not keep it from the meter.
        proxy.configure(profile={"name": "schema_drift", "p": 1.0, "kinds": ["drop_usage"]})
        assert "usage" not in post(url, REQUEST).json()
        asked = post(url, {**STREAM, "stream_options": {"include_usage": True}})
        assert not [chunk for chunk in asked.chunks() if "usage" in chunk]
        proxy.configure(profile={"name": "truncate", "p": 1.0, "bytes": 30})
        assert post(url, REQUEST).error == "incomplete" and post(url, STREAM).error == "incomplete"
        # A provider that ignores include_usage: its streams' usage is missing, so their cost is unknown, never 0.
        proxy.configure(task="legacy", profile="clean")
        for _ in range(3):
            assert post(completions(proxy, "legacy"), STREAM).events()[-1] == "[DONE]"
        totals = {row["task"]: row for row in proxy.state()["tasks"]}

    row = ledger.load_snapshot().row(MODEL)
    assert row["cache_read"] < row["input"]
    expected = [expected_classes(raw) for raw in upstream.reported]
    meter = totals["meter"]
    assert len(expected) == 20 == meter["requests"] == meter["forwarded"]
    assert meter["usage"] == {key: sum(usage[key] for usage in expected) for key in expected[0]}
    assert sum(usage["tokens_cache_read"] for usage in expected) > 0  # both ways of reporting cached tokens counted
    cost = sum(usage["tokens_in"] * row["input"] + usage["tokens_cache_read"] * row["cache_read"]
               + usage["tokens_out"] * row["output"] for usage in expected) / 1e6
    assert meter["api_equiv_usd"] == pytest.approx(cost, rel=1e-12) and cost > 0
    assert meter["cost_unknown"] == meter["usage_missing"] == 0
    assert meter["input_tokens"] == sum(usage["tokens_in"] + usage["tokens_cache_read"] for usage in expected)
    lines = read_log(log, "meter")
    assert [line["usage"] for line in lines] == expected  # call by call, not only in total
    assert {(line["usage_source"], line["cost_source"], line["model_reported"]) for line in lines} == {
        ("reported", "provider_usage", MODEL)}
    streams = [body for _, body in upstream.requests if body.get("stream")]
    assert len(streams) == 12 and all(body["stream_options"] == {"include_usage": True} for body in streams)

    legacy_lines = read_log(log, "legacy")
    assert [(line["usage_source"], line["usage"], line["api_equiv_usd"], line["cost_source"])
            for line in legacy_lines] == [("missing", None, None, "unknown")] * 3
    assert totals["legacy"]["usage_missing"] == totals["legacy"]["cost_unknown"] == 3
    assert totals["legacy"]["usage"] == {} and totals["legacy"]["api_equiv_usd"] == 0.0
    assert totals["legacy"]["input_tokens"] == 3 * len(json.dumps(STREAM).encode())  # bytes bound the unknown input
    assert all("stream_options" not in body for _, body in legacy.requests)


def test_usage_classes_follow_s01():
    raw = {"prompt_tokens": 1000, "completion_tokens": 300, "prompt_tokens_details": {"cached_tokens": 400},
           "completion_tokens_details": {"reasoning_tokens": 120}}
    assert faultproxy.usage_classes(raw) == {"tokens_in": 600, "tokens_cache_read": 400, "tokens_cache_write_5m": 0,
                                             "tokens_cache_write_1h": 0, "tokens_out": 300, "tokens_reasoning": 120}
    moonshot = {"prompt_tokens": 1000, "completion_tokens": 300, "cached_tokens": 250}
    assert faultproxy.usage_classes(moonshot)["tokens_in"] == 750
    assert faultproxy.usage_classes({**moonshot, "cached_tokens": 5000})["tokens_in"] == 0  # never negative
    for broken in (None, {}, {"prompt_tokens": 10}, {"prompt_tokens": -1, "completion_tokens": 2},
                   {"prompt_tokens": True, "completion_tokens": 2}):
        assert faultproxy.usage_classes(broken) is None
    row = ledger.load_snapshot().row(MODEL)
    cost = ledger.price(faultproxy.usage_classes(raw), row)
    assert cost.api_equiv_usd == pytest.approx((600 * 2.50 + 400 * 0.25 + 300 * 15.00) / 1e6)  # cached input once


def test_fault_draws_depend_only_on_seed_task_and_request_number():
    profile = faultproxy.Profile.parse({"name": "http_5xx", "p": 0.25, "seed": 7})
    fired = [k for k in range(1, 41) if profile.draw("golden", k) is not None]
    assert fired == [5, 6, 9, 13, 14, 27, 29, 34, 35, 37]  # pinned: the same on every host and Python version
    assert fired != [k for k in range(1, 41) if profile.draw("other-task", k) is not None]
    reseeded = faultproxy.Profile.parse({"name": "http_5xx", "p": 0.25, "seed": 8})
    assert fired != [k for k in range(1, 41) if reseeded.draw("golden", k) is not None]
    latency = faultproxy.Profile.parse({"name": "latency", "ms": 100, "jitter": 20, "seed": 7})  # p defaults to 1
    assert [latency.draw("golden", k)["delay_ms"] for k in range(1, 4)] == [80.257, 109.403, 92.219]
    assert all(80 <= latency.draw("golden", k)["delay_ms"] <= 120 for k in range(1, 201))
    # The rate is a property of every seed, not of the seeds the other tests picked.
    for p in (0.05, 0.12, 0.33):
        for seed in range(40):
            drawn = faultproxy.Profile(name="truncate", p=p, bytes=1, seed=seed)
            rate = sum(drawn.draw("any", k) is not None for k in range(1, REQUESTS + 1)) / REQUESTS
            assert abs(rate - p) <= 0.02
    for bad in ("chaos", {"name": "rate_limit", "p": 0.1}, {"name": "hang", "p": 0.1, "s": 1, "bytes": 3},
                {"name": "truncate", "p": 1.5, "bytes": 3}, {"name": "http_5xx", "p": 0.1, "status": 404},
                {"name": "schema_drift", "p": 0.1, "kinds": ["drop_everything"]}, {"name": "latency", "jitter": 3}):
        with pytest.raises(faultproxy.ProxyError):
            faultproxy.Profile.parse(bad)


def test_schema_drift_rewrites_bodies_and_streams(proxy):
    proxy.configure(task="drift", profile={"name": "schema_drift", "p": 1.0,
                                           "kinds": ["rename_finish_reason", "stringify_tool_args"]})
    body = post(completions(proxy), {**REQUEST, "tools": TOOLS}).json()
    [choice] = body["choices"]
    assert "finish_reason" not in choice and choice["stop_reason"] == "stop" and body["usage"]  # usage kept
    arguments = choice["message"]["tool_calls"][0]["function"]["arguments"]
    assert json.loads(arguments) == TOOL_CALL["function"]["arguments"]  # a JSON string holding the JSON string
    chunks = post(completions(proxy), {**STREAM, "tools": TOOLS}).chunks()
    choices = [choice for chunk in chunks for choice in chunk["choices"]]
    assert not [c for c in choices if "finish_reason" in c] and [c for c in choices if c.get("stop_reason") == "stop"]
    [call_delta] = [c["delta"]["tool_calls"][0] for c in choices if c.get("delta", {}).get("tool_calls")]
    assert json.loads(call_delta["function"]["arguments"]) == TOOL_CALL["function"]["arguments"]


def test_truncate_never_completes_a_response(proxy, upstream, log):
    proxy.configure(task="cut", profile={"name": "truncate", "p": 1.0, "bytes": 100_000})
    whole = post(completions(proxy), REQUEST)  # a cut larger than the body still stops one byte short
    assert whole.error == "incomplete" and len(whole.body) == int(whole.headers["Content-Length"]) - 1
    proxy.configure(profile={"name": "truncate", "p": 1.0, "bytes": 90})
    stream = post(completions(proxy), STREAM)
    assert stream.error == "incomplete" and len(stream.body) == 90 and b"[DONE]" not in stream.body
    lines = read_log(log, "cut")
    assert [line["status"] for line in lines] == [200, 200] and all(line["usage"] for line in lines)
    assert [line["usage"] for line in lines] == [expected_classes(raw) for raw in upstream.reported]


def test_input_token_cap_refuses_further_calls(proxy, upstream, log):
    proxy.configure(task="capped", input_token_cap=raw_usage(1)["prompt_tokens"] + 1)
    assert [post(completions(proxy), REQUEST).status for _ in range(2)] == [200, 200]  # the cap is reached
    refused = post(completions(proxy), REQUEST)
    assert refused.status == 403 and refused.json()["error"]["type"] == "vb_cap_exceeded"
    assert len(upstream.requests) == 2  # a refused call never reaches the provider
    assert post(completions(proxy), REQUEST).status == 403
    proxy.configure(task="next")
    assert post(completions(proxy), REQUEST).status == 200  # the cap counts per task
    lines = read_log(log, "capped")
    assert [line["refused"] for line in lines] == [None, None, "input_token_cap", "input_token_cap"]
    assert [(line["usage_source"], line["api_equiv_usd"], line["cost_source"]) for line in lines[2:]] == [
        ("none", 0.0, "not_billed")] * 2
    [capped] = [row for row in proxy.state()["tasks"] if row["task"] == "capped"]
    assert capped["refused"] == 2 and capped["forwarded"] == 2 and capped["requests"] == 4


def test_control_endpoint_needs_the_token(proxy, upstream):
    control = proxy.url + faultproxy.CONTROL_PATH
    assert call("GET", control).status == 401
    assert post(control, {"profile": "clean"}, headers={"Authorization": "Bearer wrong"}).status == 401
    auth = {"Authorization": f"Bearer {proxy.token}"}
    reply = post(control, {"task": "t1", "profile": {"name": "http_5xx", "p": 1.0, "status": 503}}, headers=auth)
    assert reply.status == 200 and reply.json()["task"] == "t1" and reply.json()["profile"]["status"] == 503
    assert post(completions(proxy), REQUEST).status == 503
    assert post(control, {"profile": {"name": "hang", "p": 2, "s": 1}}, headers=auth).status == 400
    assert post(control, {"profile": "clean", "extra": 1}, headers=auth).status == 400
    state = call("GET", control, headers=auth).json()
    assert state["profile"]["name"] == "http_5xx"  # the refused changes changed nothing
    assert [(row["task"], row["faults"]) for row in state["tasks"]] == [("t1", 1)]
    assert post(proxy.url + "/nowhere/chat/completions", REQUEST).status == 404  # only named upstreams
    assert call("GET", completions(proxy)).status == 404  # only POST is forwarded
    assert upstream.requests == []


def test_upstreams_are_allowlisted_and_get_the_proxys_key(tmp_path, monkeypatch, upstream):
    assert {"api.cerebras.ai", "api.openai.com"} <= faultproxy.arm_hosts()
    for base_url in ("https://example.com/v1", "http://api.openai.com/v1", "ftp://127.0.0.1/v1"):
        with pytest.raises(faultproxy.ProxyError):
            faultproxy.FaultProxy([faultproxy.Upstream("x", base_url)], log_path=tmp_path / "refused.jsonl")
    monkeypatch.delenv("VB_TEST_UNSET_KEY", raising=False)
    with pytest.raises(faultproxy.ProxyError):
        faultproxy.FaultProxy([faultproxy.Upstream("openai", "https://api.openai.com/v1", "VB_TEST_UNSET_KEY")],
                              log_path=tmp_path / "refused.jsonl")
    monkeypatch.setenv("VB_TEST_UPSTREAM_KEY", "sk-upstream-test-7c1d")
    faultproxy.FaultProxy([faultproxy.Upstream("openai", "https://api.openai.com/v1", "VB_TEST_UPSTREAM_KEY")],
                          log_path=tmp_path / "allowed.jsonl").close()  # an arm's host is allowed; nothing is sent
    with StubUpstream(redirect_to="http://127.0.0.1:9/elsewhere") as redirecting, faultproxy.FaultProxy(
            [faultproxy.Upstream("stub", upstream.url, "VB_TEST_UPSTREAM_KEY"), faultproxy.Upstream(
                "moved", redirecting.url)], log_path=tmp_path / "proxy.jsonl") as proxy:
        assert post(completions(proxy), REQUEST, headers={"Authorization": "Bearer client-secret"}).status == 200
        moved = post(completions(proxy, "moved"), REQUEST)
    assert upstream.requests[0][0]["authorization"] == "Bearer sk-upstream-test-7c1d"
    assert moved.status == 307 and moved.headers["Location"] == "http://127.0.0.1:9/elsewhere"  # not followed
    assert "authorization" not in redirecting.requests[0][0]


def test_a_hang_outlasts_a_short_client_timeout(proxy, upstream, log):
    proxy.configure(task="hang", profile={"name": "hang", "p": 1.0, "s": 5})
    started = time.monotonic()
    assert post(completions(proxy), REQUEST, timeout=0.3).error == "timeout"
    deadline = time.monotonic() + 3
    while not read_log(log, "hang") and time.monotonic() < deadline:
        time.sleep(0.02)
    [line] = read_log(log, "hang")
    assert time.monotonic() - started < 3  # the proxy stops holding once the client gives up
    assert line["fault_injected"] == "hang" and line["status"] is None and line["forwarded"] is False
    assert line["usage_source"] == "none" and upstream.requests == []


def test_a_driver_run_through_the_proxy_matches_its_ledger(tmp_path, log):
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")

    def respond(body: dict) -> str:  # look once, then submit
        return bash("echo VB_SUBMIT") if any(m["role"] == "assistant" for m in body["messages"]) else bash("ls")

    with StubServer(respond) as stub, faultproxy.FaultProxy([faultproxy.Upstream("cerebras", stub.url)],
                                                            log_path=log) as proxy:
        proxy.configure(task="toy")
        code = vb.main(["run", "--experiment", "TEST-PROXY", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm",
                        "cheap_direct", "--model", "gpt-oss-120b", "--seeds", "1", "--provider-url",
                        proxy.base_url("cerebras"), "--results", str(tmp_path / "results"), "--work",
                        str(tmp_path / "work"), "--secret-file", str(secret)])
        calls = len(stub.requests)
        [meter] = proxy.state()["tasks"]
    assert code == 0 and calls == 4
    rows = [json.loads(line) for line in (tmp_path / "results" / "TEST-PROXY" / "run-1" / "ledger.jsonl")
            .read_text().splitlines()]
    assert meter["task"] == "toy" and meter["requests"] == meter["forwarded"] == calls
    assert sum(row["api_equiv_usd"] for row in rows) == pytest.approx(meter["api_equiv_usd"], rel=1e-12)
    for key in ("tokens_in", "tokens_cache_read", "tokens_out", "tokens_reasoning"):
        assert sum(row["usage"][key] for row in rows) == meter["usage"][key]
