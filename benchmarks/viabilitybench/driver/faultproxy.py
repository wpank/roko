"""The metering and fault proxy (S08 §4.11, T13): a local OpenAI-compatible reverse proxy that meters every model call
independently of the client, and injects the provider faults of S08 §4.6's `provider_fault` hook.

**Routing.** `FaultProxy` listens on 127.0.0.1 (an ephemeral port, daemon threads) and serves each `Upstream` under
its own prefix: a client whose base URL is `proxy.base_url("cerebras")` has `POST /cerebras/chat/completions`
forwarded to `<the upstream's base_url>/chat/completions`. Nothing else is forwarded anywhere. An upstream's host
must be one an arm file names (`arm_hosts`) or a loopback address, and a network upstream must use https; a client
cannot name a target; only POST is forwarded; redirects go back to the client and are never followed.

The client's credentials are dropped, and the proxy sends the upstream's key instead: the entry of `keys` that the
upstream's `api_key_env` names. The keys come from the driver-only key file (`secret.load_keys`), never from an
environment, where agents could read them (bug-979a06). A client routed through the proxy therefore needs no key
(`vb.py` gives an override URL none), and the proxy is the only holder of a provider key. Connections to an upstream
are kept alive and reused, as a provider's own client would, so routing a call through the proxy adds no connection or
TLS setup to it.

**Metering** (always on). Each response's `usage` is mapped onto S01 §4.4's disjoint token classes (`usage_classes`:
`tokens_in` is uncached input only; cache reads and cache writes are classes of their own) and priced from the price
snapshot by the model the response reports (`ledger.price`). A streaming request gets
`stream_options.include_usage = true` when its upstream supports it (`Upstream.stream_usage`), and the usage-only
chunk this adds is not relayed to a client that did not ask for it. The proxy reads every upstream response to its
end, even when a fault cuts or rewrites what the client gets, so the meter holds what the provider billed. Each call's
`usage_source` is one of:
- `reported`: the provider reported usage (its cost is still unknown, null, for a model without a price row);
- `missing`: the request reached the provider but no usage came back, so its cost is unknown;
- `none`: nothing was billed, because the proxy answered itself (a fault or a refusal) or the upstream answered an
  error status.

Each task's input tokens (every input class) are counted against the `input_token_cap`. A call that could take them
past it is refused with a 403 (`vb_cap_exceeded`) and never forwarded: the proxy adds an upper bound on the call's
input to what the task has counted and what its calls in flight may still count (bug-c30764). The bound is the
request's bytes plus `PREAMBLE_TOKENS`, since a token is never shorter than a byte and a provider may put a header of
its own before the first message (as `mini_loop` bounds its calls). When the request extends one the task sent
before, with the same settings and that request's messages first, the bound is that request's reported input plus
the bytes of the messages it adds, so a long conversation's bound stays close to its real size. A call whose usage is
missing counts its bound. The bound holds for text only: an image, audio or a file costs tokens that are no function
of its bytes (an image a request only links to can cost more tokens than its URL has bytes). So while a cap is set,
a request with a part that is not text is refused with a 403 (`vb_unbounded_input`) before it is forwarded
(bug-d34a29): the pilot's arms send text only, and a cap that such a request could cross would be no cap.

**Faults.** One profile is active at a time (`Profile`; the names and parameters are S08 §4.11's):
- `clean`: no faults;
- `http_5xx(p, status=500)`: answer `status` without forwarding;
- `rate_limit(p, retry_after)`: answer 429 with `Retry-After: <retry_after>` without forwarding;
- `hang(p, s)`: hold the request for s seconds without an answer, or until the client gives up, then drop the
  connection;
- `truncate(p, bytes)`: forward, relay the status, the headers and at most `bytes` bytes of the body, then drop the
  connection before the response is complete;
- `latency(ms, jitter=0, p=1)`: wait ms ± jitter milliseconds (uniform), then forward;
- `schema_drift(p, kinds)`: forward, then rewrite the response (a stream chunk by chunk). `drop_usage` removes
  `usage`; `rename_finish_reason` renames each choice's `finish_reason` to `stop_reason`; `stringify_tool_args`
  JSON-encodes each tool call's `function.arguments` once more. `kinds` defaults to all three.

Whether a task's k-th request gets the fault, and the latency drawn, come from `hmac_seed.surface_stream` keyed by
(profile seed, task, k): the same profile, task and k give the same fault on every host, whatever ran before. k counts
the task's requests from 1, refused ones included. Faults are balanced within blocks of `FAULT_BLOCK` requests: a block
gets p × FAULT_BLOCK of them (rounded up or down at random when that is not whole), at random positions. Each request
still has probability p, but the rate over n requests stays close to p. Independent draws would miss ±2 points over
1,000 requests one time in seven at p = 0.3 (S08 T13's acceptance).

**Control.** `POST /_vb/control` with `Authorization: Bearer <proxy.token>` and a JSON object sets any of `task` (the
active task id), `profile` (a name, or a table with `name` and parameters) and `input_token_cap` (null for none).
`GET` returns the state with each task's meter totals. In the driver's own process, `configure` and `state` do the
same. The token is random per proxy, so an agent that finds the port can neither switch faults off nor move its calls
to another task.

**The log.** Every request to an upstream is metered and logged before the client can see the end of its response, so
a caller whose call has returned finds it in the log and in `state`. Each appends one JSON line to `log_path`, flushed
but not fsynced: `ts` (when the request arrived, in UTC to the microsecond, so a runner can tell apart attempts that
end within one second: bug-09fac4), `task`, `ordinal`, `profile`, `fault_injected` (the profile's name when its fault
fired, else null), `fault` (the fault's detail), `upstream`, `path`, `model_requested`, `model_reported`, `stream`,
`status` (what the client got; null when it got none), `forwarded`, `usage_source`, `usage` (the classes; null when
none came back), `api_equiv_usd`, `without_cache_usd`, `cost_source` (`provider_usage`, `unknown` or `not_billed`),
`price_snapshot_id`, `refused` (`input_token_cap` or `unbounded_input` when the proxy refused the call, else null),
`input_bound` (the most input the call could have been billed for; null for one that is not text only) and
`elapsed_ms`.

API:
    Upstream(name, base_url, api_key_env=None, stream_usage=True); Upstream.from_endpoint(endpoint) -> Upstream
    Profile(name="clean", p=0.0, seed=0, ...); Profile.parse(spec: str | Mapping | Profile) -> Profile
    Profile.draw(task, ordinal) -> dict | None; Profile.as_json() -> dict
    FaultProxy(upstreams, *, log_path, snapshot=None, allowed_hosts=None, input_token_cap=None, token=None, keys=None)
    FaultProxy: a context manager (.start(), .close()); .url, .token, .base_url(name) -> str
    FaultProxy.endpoint(endpoint: provider.Endpoint) -> provider.Endpoint      # the same provider, through the proxy
    FaultProxy.configure(*, task=..., profile=..., input_token_cap=...) -> dict; .state() -> dict
    usage_classes(raw) -> dict | None; arm_hosts() -> frozenset[str]
    ProxyError
"""

from __future__ import annotations

import datetime as dt
import hashlib
import hmac
import http.client
import json
import math
import os
import re
import secrets
import select
import socket
import threading
import time
import tomllib
import urllib.parse
from collections.abc import Iterable, Mapping
from dataclasses import asdict, dataclass, field, replace
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

import layout  # also puts families/ on sys.path for common
import ledger
import provider
from common import hmac_seed

CONTROL_PATH = "/_vb/control"
DRIFT_KINDS = ("drop_usage", "rename_finish_reason", "stringify_tool_args")
PROFILE_PARAMS = {"clean": (), "http_5xx": ("p", "status"), "rate_limit": ("p", "retry_after"), "hang": ("p", "s"),
                  "truncate": ("p", "bytes"), "latency": ("ms", "jitter", "p"), "schema_drift": ("p", "kinds")}
REQUIRED_PARAMS = {"http_5xx": ("p",), "rate_limit": ("p", "retry_after"), "hang": ("p", "s"),
                   "truncate": ("p", "bytes"), "latency": ("ms",), "schema_drift": ("p",)}
FAULT_BLOCK = 20  # requests over which faults are balanced (`Profile.draw`)
UPSTREAM_NAME = re.compile(r"[a-z0-9][a-z0-9_-]{0,63}")
MAX_BODY_BYTES = 32 << 20
UPSTREAM_TIMEOUT_S = 600.0
IDLE_CONNECTIONS = 8  # kept-alive upstream connections held per upstream
NOT_RELAYED = frozenset({"connection", "keep-alive", "proxy-authenticate", "proxy-authorization", "te", "trailer",
                         "transfer-encoding", "upgrade", "content-length", "date", "server"})
EVENT_END = re.compile(rb"\r\n\r\n|\n\n|\r\r")
PREAMBLE_TOKENS = 256  # what a provider may put before the first message (a harmony system header), as in mini_loop
TEXT_PARTS = frozenset({"text", "refusal", "input_text", "output_text"})  # content parts whose tokens bytes bound
# Keys that carry an image, audio or a file in any request shape (chat content parts, the Responses API's input).
MEDIA_KEYS = frozenset({"image_url", "input_image", "input_audio", "file", "input_file", "file_data", "file_id",
                        "file_url", "video_url"})
# Request fields that set how a model samples, not what its prompt holds: a change to one keeps a conversation's prefix.
NOT_PROMPT = frozenset({"stream", "stream_options", "max_tokens", "max_completion_tokens", "temperature", "top_p",
                        "seed", "n", "stop", "user", "metadata", "logprobs", "top_logprobs", "presence_penalty",
                        "frequency_penalty", "logit_bias"})
_UNSET: Any = object()


class ProxyError(ValueError):
    """An upstream, a profile or a control value is not allowed or not valid."""


def _number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _whole(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def _kinds(value: object) -> bool:
    return (isinstance(value, tuple) and all(isinstance(kind, str) for kind in value)
            and 0 < len(value) == len(set(value)) and set(value) <= set(DRIFT_KINDS))


_CHECKS = {
    "p": (lambda value: _number(value) and 0 <= value <= 1, "a number from 0 to 1"),
    "seed": (lambda value: isinstance(value, int) and not isinstance(value, bool), "an integer"),
    "status": (lambda value: _whole(value) and 500 <= value <= 599, "an HTTP status from 500 to 599"),
    "retry_after": (_whole, "a whole number of seconds"),
    "s": (lambda value: _number(value) and value > 0, "a number of seconds above 0"),
    "bytes": (_whole, "a whole number of bytes"),
    "ms": (lambda value: _number(value) and value >= 0, "a number of milliseconds"),
    "jitter": (lambda value: _number(value) and value >= 0, "a number of milliseconds"),
    "kinds": (_kinds, f"a non-empty list of {', '.join(DRIFT_KINDS)} without repeats"),
}


@dataclass(frozen=True)
class Upstream:
    name: str  # served under /<name>/
    base_url: str  # e.g. https://api.cerebras.ai/v1
    api_key_env: str | None = None
    stream_usage: bool = True  # the provider honours stream_options.include_usage

    @classmethod
    def from_endpoint(cls, endpoint: provider.Endpoint) -> Upstream:
        return cls(name=endpoint.provider, base_url=endpoint.base_url, api_key_env=endpoint.api_key_env)


@dataclass(frozen=True)
class Profile:
    """One fault profile. The constructor checks every value; `parse` also checks which parameters a name takes."""

    name: str = "clean"
    p: float = 0.0
    seed: int = 0
    status: int = 500  # http_5xx
    retry_after: int = 1  # rate_limit, seconds
    s: float = 30.0  # hang, seconds
    bytes: int = 0  # truncate: the most body bytes the client gets
    ms: float = 0.0  # latency
    jitter: float = 0.0  # latency
    kinds: tuple[str, ...] = DRIFT_KINDS  # schema_drift

    def __post_init__(self) -> None:
        if self.name not in PROFILE_PARAMS:
            raise ProxyError(f"unknown fault profile {self.name!r}; the profiles are {', '.join(PROFILE_PARAMS)}")
        if isinstance(self.kinds, list):
            object.__setattr__(self, "kinds", tuple(self.kinds))
        for key, (valid, wanted) in _CHECKS.items():
            if not valid(getattr(self, key)):
                raise ProxyError(f"profile {self.name}: {key} must be {wanted}, not {getattr(self, key)!r}")

    @classmethod
    def parse(cls, spec: str | Mapping | Profile) -> Profile:
        """A profile from its name, or from a table with `name`, its parameters and an optional `seed`."""
        if isinstance(spec, Profile):
            return spec
        if not isinstance(spec, (str, Mapping)):
            raise ProxyError(f"a profile is a name or a table, not {spec!r}")
        table = {"name": spec} if isinstance(spec, str) else dict(spec)
        name = table.pop("name", None)
        if name not in PROFILE_PARAMS:
            raise ProxyError(f"unknown fault profile {name!r}; the profiles are {', '.join(PROFILE_PARAMS)}")
        unknown = sorted(set(table) - {*PROFILE_PARAMS[name], "seed"})
        if unknown:
            raise ProxyError(f"profile {name} takes no {', '.join(unknown)}")
        missing = [key for key in REQUIRED_PARAMS.get(name, ()) if key not in table]
        if missing:
            raise ProxyError(f"profile {name} needs {', '.join(missing)}")
        if name == "latency":
            table.setdefault("p", 1.0)
        return cls(name=name, **table)

    def draw(self, task: str | None, ordinal: int) -> dict | None:
        """The fault of request `ordinal` of `task`: its detail when this profile's fault fires, else None."""
        if self.name == "clean":
            return None
        block, position = divmod(ordinal - 1, FAULT_BLOCK)
        stream = hmac_seed.surface_stream("vb.faultproxy/1", json.dumps([self.seed, task, block]))
        faults = self.p * FAULT_BLOCK
        faults = int(faults) + (stream.random() < faults - int(faults))
        if position not in stream.sample(range(FAULT_BLOCK), faults):
            return None
        if self.name == "latency":
            spread = 2 * stream.child(f"latency/{position}").random() - 1
            return {"delay_ms": round(max(self.ms + spread * self.jitter, 0.0), 3)}
        if self.name == "schema_drift":
            return {"kinds": list(self.kinds)}
        return {key: getattr(self, key) for key in PROFILE_PARAMS[self.name] if key != "p"}

    def as_json(self) -> dict:
        table = {"name": self.name, **{key: getattr(self, key) for key in PROFILE_PARAMS[self.name]}}
        if "kinds" in table:
            table["kinds"] = list(self.kinds)
        return table if self.name == "clean" else {**table, "seed": self.seed}


def usage_classes(raw: object) -> dict | None:
    """OpenAI-style `usage` on S01 §4.4's disjoint token classes; None without whole prompt and completion counts.

    `prompt_tokens` includes the cached tokens (`prompt_tokens_details.cached_tokens`, or a top-level `cached_tokens`
    where the provider reports it there), so `tokens_in` is the difference. Cache writes are not billed apart by these
    providers: any reported write count stays inside `tokens_in`, and both write classes are 0.
    """
    if not isinstance(raw, dict) or not (_whole(raw.get("prompt_tokens")) and _whole(raw.get("completion_tokens"))):
        return None
    prompt, completion = raw["prompt_tokens"], raw["completion_tokens"]
    details = raw.get("prompt_tokens_details")
    cached = details.get("cached_tokens") if isinstance(details, dict) else None
    if cached is None:
        cached = raw.get("cached_tokens")
    output = raw.get("completion_tokens_details")
    reasoning = output.get("reasoning_tokens") if isinstance(output, dict) else None
    usage = ledger.vb_usage(prompt, completion, min(cached, prompt) if _whole(cached) else 0,
                            reasoning if _whole(reasoning) else 0)
    return {**usage, "tokens_cache_write_5m": 0, "tokens_cache_write_1h": 0}


def arm_hosts() -> frozenset[str]:
    """The provider hosts the arm files name (`[providers.*] base_url`): the proxy's default allowlist."""
    hosts = set()
    for path in sorted(layout.ARMS_DIR.glob("*.toml")):
        try:
            with path.open("rb") as handle:
                doc = tomllib.load(handle)
        except (OSError, tomllib.TOMLDecodeError):
            continue  # vb.load_arm reports a broken arm file; it adds no host here
        for table in doc.get("providers", {}).values():
            if isinstance(table, dict) and isinstance(table.get("base_url"), str):
                host = urllib.parse.urlsplit(table["base_url"]).hostname
                if host:
                    hosts.add(host.lower())
    return frozenset(hosts)


@dataclass
class _Meter:
    """One task's totals. `api_equiv_usd` sums the known costs; `cost_unknown` counts the calls without one."""

    requests: int = 0
    forwarded: int = 0
    faults: int = 0
    refused: int = 0  # calls refused at the task's input cap
    unbounded: int = 0  # calls refused because the proxy cannot bound their input
    usage_missing: int = 0
    cost_unknown: int = 0
    input_tokens: int = 0  # every input class, plus the input bound of each call whose usage is missing
    reserved: int = 0  # the input bounds of the calls in flight
    usage: dict = field(default_factory=dict)
    api_equiv_usd: float = 0.0


class FaultProxy:
    def __init__(self, upstreams: Iterable[Upstream], *, log_path: Path, snapshot: ledger.Snapshot | None = None,
                 allowed_hosts: Iterable[str] | None = None, input_token_cap: int | None = None,
                 token: str | None = None, upstream_timeout_s: float = UPSTREAM_TIMEOUT_S,
                 keys: Mapping[str, str] | None = None) -> None:
        allowed = frozenset(host.lower() for host in (arm_hosts() if allowed_hosts is None else allowed_hosts))
        self.upstreams: dict[str, Upstream] = {}
        for upstream in upstreams:
            _check_upstream(upstream, allowed)
            if upstream.name in self.upstreams:
                raise ProxyError(f"two upstreams are named {upstream.name}")
            self.upstreams[upstream.name] = upstream
        if not self.upstreams:
            raise ProxyError("the proxy needs at least one upstream")
        self._keys: Mapping[str, str] = keys if keys is not None else {}
        missing = [upstream.api_key_env for upstream in self.upstreams.values()
                   if upstream.api_key_env and not self._keys.get(upstream.api_key_env)]
        if missing:
            raise ProxyError(f"no key for {', '.join(missing)}: the proxy's upstreams need them in the key file")
        self.snapshot = snapshot or ledger.load_snapshot()
        self.token = token or secrets.token_urlsafe(24)
        self.upstream_timeout_s = upstream_timeout_s
        self._lock = threading.Lock()
        self._task: str | None = None
        self._profile = Profile()
        self._cap = _cap(input_token_cap)
        self._meters: dict[str | None, _Meter] = {}
        self._prompts: dict[str | None, dict[str, int]] = {}  # per task: a prompt's digest -> its reported input
        self._idle: dict[str, list[http.client.HTTPConnection]] = {}
        self._closed = False
        self._log = os.fdopen(os.open(log_path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600), "a", encoding="utf-8")
        self._server = _Server(self)
        self._thread = threading.Thread(target=self._server.serve_forever, name="vb-faultproxy", daemon=True)

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self._server.server_address[1]}"

    def base_url(self, name: str) -> str:
        """The base URL a client uses for upstream `name`."""
        if name not in self.upstreams:
            raise ProxyError(f"no upstream named {name!r}")
        return f"{self.url}/{name}"

    def endpoint(self, endpoint: provider.Endpoint) -> provider.Endpoint:
        """`endpoint` routed through the proxy: the same provider and limits, the proxy's URL, and no key."""
        return replace(endpoint, base_url=self.base_url(endpoint.provider), api_key_env=None)

    def start(self) -> FaultProxy:
        self._thread.start()
        return self

    def close(self) -> None:
        if self._thread.is_alive():
            self._server.shutdown()
        self._server.server_close()
        with self._lock:
            self._closed = True
            self._log.close()
            idle = [connection for connections in self._idle.values() for connection in connections]
            self._idle.clear()
        for connection in idle:
            connection.close()

    def __enter__(self) -> FaultProxy:
        return self.start()

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    def configure(self, *, task: str | None = _UNSET, profile: str | Mapping | Profile = _UNSET,
                  input_token_cap: int | None = _UNSET) -> dict:
        """Set the active task id, the fault profile and the input-token cap; returns the new state."""
        if task is not _UNSET and task is not None and not isinstance(task, str):
            raise ProxyError(f"task must be a string or null, not {task!r}")
        profile = profile if profile is _UNSET else Profile.parse(profile)
        cap = input_token_cap if input_token_cap is _UNSET else _cap(input_token_cap)
        with self._lock:
            self._task = self._task if task is _UNSET else task
            self._profile = self._profile if profile is _UNSET else profile
            self._cap = self._cap if cap is _UNSET else cap
        return self.state()

    def state(self) -> dict:
        with self._lock:
            return {"task": self._task, "profile": self._profile.as_json(), "input_token_cap": self._cap,
                    "price_snapshot_id": self.snapshot.id,
                    "tasks": [{"task": task, **asdict(meter)} for task, meter in self._meters.items()]}

    def _serve(self, handler: _Handler, upstream: Upstream, path: str, body: bytes) -> None:
        """One call to an upstream: refuse it, fault it or forward it; it is metered and logged before it ends."""
        started = time.monotonic()
        request = _json_object(body)
        digests, sizes = _prompt_digests(request)
        media = _unbounded_part(request)
        with self._lock:
            task, profile, cap = self._task, self._profile, self._cap
            meter = self._meters.setdefault(task, _Meter())
            meter.requests += 1
            ordinal = meter.requests
            bound = self._input_bound(task, digests, sizes, len(body))  # a text bound: none holds for media
            counted = meter.input_tokens + meter.reserved
            refusal = None if cap is None else "unbounded_input" if media else \
                "input_token_cap" if counted + bound > cap else None
            if refusal is None:
                meter.reserved += bound
        call = _Call(self, handler, meter, bound, None if refusal else digests[-1], started, {
            "ts": _now(), "task": task, "ordinal": ordinal, "profile": profile.as_json(),
            "fault_injected": None, "fault": None, "upstream": upstream.name, "path": path,
            "model_requested": _text(request.get("model")), "model_reported": None,
            "stream": request.get("stream") is True, "status": None, "forwarded": False, "usage_source": "none",
            "usage": None, "refused": refusal, "input_bound": None if media else bound})
        try:
            if refusal == "unbounded_input":
                call.answer(403, _error("vb_unbounded_input", f"task {task!r} has an input-token cap, and the proxy "
                                        f"cannot bound this request's input: it holds a {media!r} part",
                                        "unbounded_input"))
                return
            if refusal:
                call.answer(403, _error("vb_cap_exceeded", f"task {task!r} could pass its input-token cap of {cap}: "
                                        f"{counted} counted, and this call may add up to {bound}", "input_token_cap"))
                return
            detail = profile.draw(task, ordinal)
            fault = None if detail is None else profile.name
            call.entry["fault_injected"], call.entry["fault"] = fault, detail
            if fault == "http_5xx":
                call.answer(profile.status, _error("server_error", "injected upstream error"))
            elif fault == "rate_limit":
                call.answer(429, _error("rate_limit_exceeded", "injected rate limit"),
                            {"Retry-After": str(profile.retry_after)})
            elif fault == "hang":
                handler.hold(profile.s)
                call.settle()
                handler.drop()
            else:
                if fault == "latency":
                    time.sleep(detail["delay_ms"] / 1000)
                relay = _Relay(call, cut=profile.bytes if fault == "truncate" else None,
                               drift=profile.kinds if fault == "schema_drift" else None)
                self._forward(call, relay, upstream, path, body, request)
        finally:
            call.settle()  # a no-op unless an error skipped it

    def _forward(self, call: _Call, relay: _Relay, upstream: Upstream, path: str, body: bytes,
                 request: dict) -> None:
        handler, injected = call.handler, False
        if call.entry["stream"] and upstream.stream_usage:
            options = request.get("stream_options") if isinstance(request.get("stream_options"), dict) else {}
            if options.get("include_usage") is not True:
                body = json.dumps({**request, "stream_options": {**options, "include_usage": True}}).encode("utf-8")
                injected = True
        target = urllib.parse.urlsplit(upstream.base_url)
        headers = {"Content-Type": "application/json", "Accept": handler.headers.get("Accept") or "application/json"}
        if handler.headers.get("User-Agent"):
            headers["User-Agent"] = handler.headers["User-Agent"]
        key = self._keys.get(upstream.api_key_env) if upstream.api_key_env else None
        if key:
            headers["Authorization"] = f"Bearer {key}"
        connection, reusable = self._connection(upstream), False
        try:
            try:
                connection.request("POST", target.path.rstrip("/") + path + (f"?{handler.query}" if handler.query
                                                                             else ""), body=body, headers=headers)
            except (OSError, http.client.HTTPException) as err:  # an incomplete request is never processed
                call.answer(502, _error("upstream_unreachable", f"{upstream.name}: {err}"))
                return
            call.entry["forwarded"] = True
            try:
                response = connection.getresponse()
            except (OSError, http.client.HTTPException) as err:
                call.entry["usage_source"] = "missing"
                call.answer(502, _error("upstream_failed", f"{upstream.name}: {type(err).__name__}: {err}"))
                return
            call.entry["status"] = response.status
            if "text/event-stream" in (response.getheader("Content-Type") or "").lower():
                reusable = self._relay_stream(call, response, relay, injected)
            else:
                reusable = self._relay_body(call, response, relay)
            reusable = reusable and not response.will_close
        finally:
            self._release(upstream, connection, reusable)

    def _connection(self, upstream: Upstream) -> http.client.HTTPConnection:
        """A kept-alive connection to `upstream` that its server has not closed, or a new one."""
        with self._lock:
            idle = self._idle.get(upstream.name, [])
            while idle:
                connection = idle.pop()
                if connection.sock is not None and not select.select([connection.sock], [], [], 0)[0]:
                    return connection
                connection.close()  # readable while idle: the server closed it
        target = urllib.parse.urlsplit(upstream.base_url)
        kind = http.client.HTTPSConnection if target.scheme == "https" else http.client.HTTPConnection
        return kind(target.hostname, target.port, timeout=self.upstream_timeout_s)

    def _release(self, upstream: Upstream, connection: http.client.HTTPConnection, reusable: bool) -> None:
        """Keep a connection whose response was read to its end for the next call; close any other."""
        with self._lock:
            idle = self._idle.setdefault(upstream.name, [])
            if reusable and not self._closed and len(idle) < IDLE_CONNECTIONS:
                idle.append(connection)
                return
        connection.close()

    def _relay_body(self, call: _Call, response: http.client.HTTPResponse, relay: _Relay) -> bool:
        """Relay a whole body; returns whether it was read to its end."""
        billed = 200 <= response.status < 300
        try:
            data = response.read()
        except (OSError, http.client.HTTPException) as err:
            call.entry["usage_source"] = "missing" if billed else "none"
            call.answer(502, _error("upstream_failed", f"{type(err).__name__}: {err}"))
            return False
        payload = _json_object(data)
        call.entry["model_reported"] = _text(payload.get("model"))
        if billed:
            usage = usage_classes(_usage_of(payload))
            call.entry["usage_source"], call.entry["usage"] = ("reported", usage) if usage else ("missing", None)
        if relay.drift and payload:
            data = json.dumps(_drift(payload, relay.drift)).encode("utf-8")
        relay.head(response, len(data))
        relay.body(data)
        return True

    def _relay_stream(self, call: _Call, response: http.client.HTTPResponse, relay: _Relay, injected: bool) -> bool:
        """Relay server-sent events one by one, reading the usage (and the model) before any rewrite; returns
        whether the stream was read to its end."""
        relay.head(response, None)
        buffer, usage, model, complete = b"", None, None, True
        while True:
            try:
                data = response.read1(65536)
            except (OSError, http.client.HTTPException):
                complete = False
                break
            if not data:
                break
            buffer += data
            while match := EVENT_END.search(buffer):
                event, buffer = buffer[:match.end()], buffer[match.end():]
                chunk = _event_json(event)
                if chunk is not None:
                    usage = _usage_of(chunk) or usage
                    model = model or _text(chunk.get("model"))
                    if injected and isinstance(chunk.get("usage"), dict) and chunk.get("choices") == []:
                        continue  # the usage-only chunk the client did not ask for
                    if relay.drift:
                        event = b"data: " + json.dumps(_drift(chunk, relay.drift)).encode("utf-8") + b"\n\n"
                relay.chunk(event)
        call.entry["model_reported"] = model
        if 200 <= response.status < 300:
            classes = usage_classes(usage)
            call.entry["usage_source"], call.entry["usage"] = ("reported", classes) if classes else ("missing", None)
        relay.chunk(buffer)  # an unterminated last event
        relay.end(complete)
        return complete

    def _input_bound(self, task: str | None, digests: list[str], sizes: list[int], body_bytes: int) -> int:
        """The most input tokens a request can be billed for (module docstring); call it holding the lock."""
        bound = body_bytes + PREAMBLE_TOKENS
        seen = self._prompts.get(task, {})
        for index in range(len(digests) - 1, -1, -1):  # the longest earlier prompt this one extends
            if digests[index] in seen:
                return min(bound, seen[digests[index]] + sum(sizes[index:]))
        return bound

    def _settle(self, call: _Call) -> None:
        entry, meter, started = call.entry, call.meter, call.started
        usage, source = entry["usage"], entry["usage_source"]
        if source == "reported":
            cost = ledger.price(usage, self.snapshot.row(entry["model_reported"]))
            api_equiv, without_cache, cost_source = cost.api_equiv_usd, cost.without_cache_usd, cost.source
        elif source == "missing":
            api_equiv, without_cache, cost_source = None, None, "unknown"
        else:
            api_equiv, without_cache, cost_source = 0.0, 0.0, "not_billed"
        entry.update(api_equiv_usd=api_equiv, without_cache_usd=without_cache, cost_source=cost_source,
                     price_snapshot_id=self.snapshot.id, elapsed_ms=round((time.monotonic() - started) * 1000, 1))
        if usage:
            counted = sum(usage[key] for key in ("tokens_in", "tokens_cache_read", "tokens_cache_write_5m",
                                                 "tokens_cache_write_1h"))
        else:
            counted = call.bound if source == "missing" else 0
        line = json.dumps(entry, sort_keys=True) + "\n"
        with self._lock:
            if call.digest is not None:  # it held a reservation, and a reported input makes its prompt a known prefix
                meter.reserved -= call.bound
                if usage:
                    self._prompts.setdefault(entry["task"], {})[call.digest] = counted
            meter.forwarded += entry["forwarded"]
            meter.faults += entry["fault_injected"] is not None
            meter.refused += entry["refused"] == "input_token_cap"
            meter.unbounded += entry["refused"] == "unbounded_input"
            meter.usage_missing += source == "missing"
            meter.cost_unknown += api_equiv is None
            meter.input_tokens += counted
            if usage:
                meter.usage = ledger.add_usage(meter.usage, usage)
            if api_equiv is not None:
                meter.api_equiv_usd += api_equiv
            if not self._log.closed:
                self._log.write(line)
                self._log.flush()

    def _control(self, handler: _Handler, body: bytes | None) -> None:
        supplied = handler.headers.get("Authorization", "").encode("utf-8", "replace")
        if not hmac.compare_digest(supplied, f"Bearer {self.token}".encode()):
            handler.send_json(401, _error("unauthorized", "the control endpoint needs the proxy's token"),
                              {"WWW-Authenticate": "Bearer"})
            return
        if body is None:
            handler.send_json(200, self.state())
            return
        try:
            table = json.loads(body)
        except ValueError:
            table = None
        if not isinstance(table, dict) or set(table) - {"task", "profile", "input_token_cap"}:
            handler.send_json(400, _error("invalid_request",
                                          "send a JSON object with task, profile or input_token_cap"))
            return
        try:
            handler.send_json(200, self.configure(**table))
        except ProxyError as err:
            handler.send_json(400, _error("invalid_request", str(err)))


class _Call:
    """One call's log entry. It is metered and logged once, just before the client can see the end of its response,
    so a caller that got its response finds the call in the log and in `state`."""

    def __init__(self, proxy: FaultProxy, handler: _Handler, meter: _Meter, bound: int, digest: str | None,
                 started: float, entry: dict) -> None:
        self.proxy = proxy
        self.handler = handler
        self.meter = meter
        self.bound = bound  # the call's input bound, reserved on the meter until it settles
        self.digest = digest  # its prompt's digest; None for a refused call, which reserved nothing
        self.started = started
        self.entry = entry
        self.settled = False

    def settle(self) -> None:
        if not self.settled:
            self.settled = True
            self.proxy._settle(self)

    def answer(self, status: int, payload: dict, headers: Mapping[str, str] | None = None) -> None:
        """Answer the client from the proxy itself."""
        self.entry["status"] = status
        self.settle()
        self.handler.send_json(status, payload, headers)


class _Relay:
    """Writes one upstream response to the client, cut short after `cut` body bytes when a truncate fault fired.

    A cut stream stalls rather than ending at once: the connection drops only after the proxy has read the upstream
    to its end and logged the call.
    """

    def __init__(self, call: _Call, *, cut: int | None, drift: tuple[str, ...] | None) -> None:
        self.call = call
        self.handler = call.handler
        self.cut = cut
        self.drift = drift
        self.sent = 0  # body bytes, without the chunk framing
        self.open = True  # false once the client is gone, or its body was cut

    def head(self, response: http.client.HTTPResponse, length: int | None) -> None:
        """The status line and the headers; a body without a length is sent chunked."""
        handler = self.handler
        try:
            handler.send_response(response.status, response.reason)
            for name, value in response.getheaders():
                if name.lower() not in NOT_RELAYED:
                    handler.send_header(name, value)
            handler.send_header(*(("Transfer-Encoding", "chunked") if length is None else
                                  ("Content-Length", str(length))))
            handler.end_headers()
        except OSError:
            self.open = False

    def body(self, data: bytes) -> None:
        """A whole body. A cut one stops short of its last byte, even when `cut` would reach it."""
        if self.cut is None:
            self.call.settle()
            self._send(data)
            return
        self._send(data[:min(self.cut, max(len(data) - 1, 0))])
        self.call.settle()
        self.handler.drop()

    def chunk(self, data: bytes) -> None:
        """Part of a chunked body, up to the cut."""
        if not data or not self.open:
            return
        if self.cut is not None and self.sent + len(data) > self.cut:
            self._send(_chunked(data[:self.cut - self.sent]))
            self.open = False
            return
        self._send(_chunked(data))
        self.sent += len(data)

    def end(self, complete: bool) -> None:
        """End a chunked body; one that was cut, or whose upstream broke off, ends by dropping the connection."""
        self.call.settle()
        if complete and self.cut is None:
            self._send(b"0\r\n\r\n")
        else:
            self.handler.drop()

    def _send(self, data: bytes) -> None:
        if not self.open or not data:
            return
        try:
            self.handler.wfile.write(data)
        except OSError:  # the client went away; the proxy still reads the upstream to the end to meter it
            self.open = False


class _Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    timeout = 120  # seconds an idle keep-alive connection may hold its thread
    server: _Server
    query = ""

    def do_GET(self) -> None:  # noqa: N802 (the stdlib's name)
        self._dispatch(has_body=False)

    def do_POST(self) -> None:  # noqa: N802 (the stdlib's name)
        self._dispatch(has_body=True)

    def send_json(self, status: int, payload: dict, headers: Mapping[str, str] | None = None) -> int:
        data = json.dumps(payload).encode("utf-8")
        try:
            self.send_response(status)
            for name, value in (headers or {}).items():
                self.send_header(name, value)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        except OSError:
            self.close_connection = True
        return status

    def hold(self, seconds: float) -> None:
        """Wait `seconds` without answering, or less if the client gives up first."""
        deadline = time.monotonic() + seconds
        readable, _, _ = select.select([self.connection], [], [], seconds)
        if readable:
            try:
                if self.connection.recv(1, socket.MSG_PEEK):  # more data, not a close: keep holding
                    time.sleep(max(deadline - time.monotonic(), 0.0))
            except OSError:
                pass

    def drop(self) -> None:
        """End the connection without finishing the response."""
        self.close_connection = True
        try:
            self.connection.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass

    def log_message(self, *args: object) -> None:
        pass

    def _dispatch(self, *, has_body: bool) -> None:
        path, _, self.query = self.path.partition("?")
        body = self._body() if has_body else b""
        if body is None:
            return
        proxy = self.server.proxy
        if path == CONTROL_PATH:
            proxy._control(self, body if has_body else None)
            return
        name, _, rest = path.lstrip("/").partition("/")
        upstream = proxy.upstreams.get(name)
        if not has_body or upstream is None or not rest:
            self.send_json(404, _error("not_found", f"no route {self.command} {path}"))
            return
        proxy._serve(self, upstream, "/" + rest, body)

    def _body(self) -> bytes | None:
        """The request body; None after answering a request whose body cannot be read."""
        length = self.headers.get("Content-Length")
        if length is None and self.headers.get("Transfer-Encoding"):
            self.send_json(411, _error("invalid_request", "send the body with a Content-Length"))
        elif length is not None and (not length.isdigit() or int(length) > MAX_BODY_BYTES):
            self.send_json(413, _error("invalid_request", f"the body must be at most {MAX_BODY_BYTES} bytes"))
        else:
            return self.rfile.read(int(length)) if length else b""
        self.drop()
        return None


class _Server(ThreadingHTTPServer):
    daemon_threads = True
    request_queue_size = 64

    def __init__(self, proxy: FaultProxy) -> None:
        super().__init__(("127.0.0.1", 0), _Handler)
        self.proxy = proxy


def _check_upstream(upstream: Upstream, allowed: frozenset[str]) -> None:
    if not UPSTREAM_NAME.fullmatch(upstream.name):
        raise ProxyError(f"upstream name {upstream.name!r} must match {UPSTREAM_NAME.pattern}")
    parts = urllib.parse.urlsplit(upstream.base_url)
    if parts.scheme not in ("http", "https") or not parts.hostname or parts.query or parts.fragment or parts.username:
        raise ProxyError(f"upstream {upstream.name}: {upstream.base_url!r} is not an http(s) base URL")
    if provider.is_loopback(upstream.base_url):
        return
    if parts.hostname.lower() not in allowed:
        raise ProxyError(f"upstream {upstream.name}: {parts.hostname} is not an allowed provider host "
                         f"({', '.join(sorted(allowed)) or 'none'})")
    if parts.scheme != "https":
        raise ProxyError(f"upstream {upstream.name}: a network provider must use https")


def _cap(value: object) -> int | None:
    if value is None or (_whole(value) and value > 0):
        return value
    raise ProxyError(f"input_token_cap must be a positive integer or null, not {value!r}")


def _error(kind: str, message: str, code: str | None = None) -> dict:
    """An error body in the OpenAI shape."""
    return {"error": {"message": message, "type": kind, "code": code}}


def _prompt_digests(request: dict) -> tuple[list[str], list[int]]:
    """Digests of a request's prompt through each of its messages, and each message's size in canonical JSON bytes.
    The first digest, before any message, covers the settings that shape a prompt: the other fields but `NOT_PROMPT`."""
    digest = hashlib.sha256(_canonical({key: value for key, value in request.items()
                                        if key != "messages" and key not in NOT_PROMPT}))
    digests, sizes = [digest.hexdigest()], []
    messages = request.get("messages")
    for message in messages if isinstance(messages, list) else []:
        data = _canonical(message)
        digest.update(b"\n" + data)
        digests.append(digest.hexdigest())
        sizes.append(len(data))
    return digests, sizes


def _unbounded_part(request: dict) -> str | None:
    """What makes a request's input unboundable by its bytes: the type of a message's first content part that is not
    text, or a key that carries media anywhere in the messages or a Responses API `input` (`MEDIA_KEYS`; tool schemas
    are not searched, where `file` can name a parameter); None for a text-only request."""
    messages = request.get("messages")
    for message in messages if isinstance(messages, list) else []:
        content = message.get("content") if isinstance(message, dict) else None
        for part in content if isinstance(content, list) else []:
            kind = part.get("type") if isinstance(part, dict) else None
            if kind not in TEXT_PARTS:
                return str(kind)
    pending: list[object] = [messages, request.get("input")]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            found = next((key for key in value if key in MEDIA_KEYS), None)
            if found:
                return found
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    return None


def _now() -> str:
    """The current UTC time in ISO 8601, to the microsecond."""
    return dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def _json_object(data: bytes) -> dict:
    try:
        value = json.loads(data)
    except ValueError:
        return {}
    return value if isinstance(value, dict) else {}


def _text(value: object) -> str | None:
    return value if isinstance(value, str) and value else None


def _choices(payload: dict) -> list[dict]:
    choices = payload.get("choices")
    return [choice for choice in choices if isinstance(choice, dict)] if isinstance(choices, list) else []


def _usage_of(payload: dict) -> dict | None:
    """A response's or a chunk's usage: at the top level, or inside a choice (Moonshot's streams put it there)."""
    if isinstance(payload.get("usage"), dict):
        return payload["usage"]
    return next((choice["usage"] for choice in _choices(payload) if isinstance(choice.get("usage"), dict)), None)


def _event_json(event: bytes) -> dict | None:
    """The JSON object a server-sent event carries in its data lines; None for `[DONE]` and everything else."""
    lines = [line[5:].removeprefix(b" ") for line in event.splitlines() if line.startswith(b"data:")]
    if not lines:
        return None
    try:
        value = json.loads(b"\n".join(lines))
    except ValueError:
        return None
    return value if isinstance(value, dict) else None


def _drift(payload: dict, kinds: tuple[str, ...]) -> dict:
    """Rewrite a response or a chunk in place with the schema_drift kinds; returns it."""
    if "drop_usage" in kinds:
        payload.pop("usage", None)
    for choice in _choices(payload):
        if "drop_usage" in kinds:
            choice.pop("usage", None)
        if "rename_finish_reason" in kinds and "finish_reason" in choice:
            choice["stop_reason"] = choice.pop("finish_reason")
        if "stringify_tool_args" in kinds:
            for part in (choice.get("message"), choice.get("delta")):
                calls = part.get("tool_calls") if isinstance(part, dict) else None
                for call in calls if isinstance(calls, list) else []:
                    function = call.get("function") if isinstance(call, dict) else None
                    if isinstance(function, dict) and "arguments" in function:
                        function["arguments"] = json.dumps(function["arguments"])
    return payload


def _chunked(data: bytes) -> bytes:
    """`data` as one chunk of a chunked body; nothing for no data, since an empty chunk ends the body."""
    return b"%x\r\n%s\r\n" % (len(data), data) if data else b""
