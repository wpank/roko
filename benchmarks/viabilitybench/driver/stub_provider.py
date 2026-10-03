"""A scripted fake model behind a local OpenAI-compatible server, for offline tests and dry runs of the driver, and
for the shakedown suite's fault injection against the real Roko binary (3314).

`StubServer(respond)` serves `POST /v1/chat/completions` on 127.0.0.1, on an ephemeral port, from a daemon thread.
`respond(body)` gets the parsed request and returns the assistant's reply, in one of five shapes:
- a plain `str`: the reply text (unchanged from before 3314);
- a `dict`: one delta/message as is (`content`, or `tool_calls`, e.g. a single tool call with no streamed split);
  unlike a `str`, it is never wrapped as `{"content": ...}`;
- `Blank()`: a reply with no content and no tool call (D1's trigger for a High-severity immune isolation, pre-fix);
- `Chunks(deltas, finish_reason="stop")`: explicit streamed deltas, each a message-shape dict (`content`, or
  `tool_calls` with a `function` whose `name`/`arguments` may be split across entries) — sent one per SSE chunk
  under a streaming request, so a parser that keeps only one field per chunk loses the others (D2); merged into one
  message under a non-streaming request;
- `ErrorStatus(status=500, auth=False)`: an HTTP error instead of a completion, `auth` marking it a 401 (D7).

A streaming request (`"stream": true` in the body) gets Server-Sent Events, one chunk per delta, a closing chunk
carrying `finish_reason`, and `usage` on that chunk only when the request's own `stream_options.include_usage` was
true (D6) — never otherwise, as a real provider would. A non-streaming request gets one JSON response, always with
`usage`. The response's `model` is the requested one (or `model_reported`); non-fault replies price deterministically
(about four characters per token, `usage_for`). `requests` keeps every request body, in order, and `headers` each
request's headers (names in lower case). Nothing here touches the network: `vb run --provider-url <server.url>` is
offline, and so is a real Roko binary pointed at it (3314's shakedown suite).

`scripted(scripts)` builds a `respond` that plays one script per task. It finds the task by a key that occurs in the
request's first user message, and returns step k of that script on the k-th call of an attempt (k = the number of
assistant messages in the request). A script that runs out repeats its last step. A step may be any of the four
reply shapes above.

API:
    StubServer(respond, *, model_reported=None)     # a context manager; .url, .requests, .headers
    scripted(scripts: Mapping[str, Sequence[str | Blank | Chunks | ErrorStatus]]) -> Callable[[dict], object]
    Blank(); Chunks(deltas: tuple[dict, ...], finish_reason: str = "stop"); ErrorStatus(status=500, auth=False)
    bash(command: str, thought: str = "Next step.") -> str      # a reply holding one bash block
    usage_for(messages: Sequence[dict], reply: str) -> dict
"""

from __future__ import annotations

import json
import threading
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REASONING_TOKENS = 8
Reply = "str | Blank | Chunks | ErrorStatus"


@dataclass(frozen=True)
class Blank:
    """3314, D1: a reply with no content and no tool call — the pre-fix trigger for a High-severity immune
    isolation instead of a retryable `empty_response`."""


@dataclass(frozen=True)
class ErrorStatus:
    """3314, D3/D7: an HTTP error instead of a completion. `auth` marks it a 401 (definitive: a retry must not
    climb past it as if it were a generic provider failure); otherwise it is a retryable server error."""

    status: int = 500
    auth: bool = False


@dataclass(frozen=True)
class Chunks:
    """3314, D2: explicit streamed deltas (module docstring), sent one per SSE chunk under a streaming request so
    every field of every chunk is checked, not just the first one's."""

    deltas: tuple[dict, ...]
    finish_reason: str = "stop"


def bash(command: str, thought: str = "Next step.") -> str:
    """A reply in the direct loop's format: a thought, then one bash block."""
    return f"THOUGHT: {thought}\n\n```bash\n{command}\n```\n"


def usage_for(messages: Sequence[dict], reply: str) -> dict:
    """Deterministic chat-completions usage: about four characters per token, and a few reasoning tokens."""
    prompt = sum(len(str(message.get("content", ""))) for message in messages) // 4 + 8 * len(messages)
    completion = len(reply) // 4 + 1 + REASONING_TOKENS
    return {"prompt_tokens": prompt, "completion_tokens": completion, "total_tokens": prompt + completion,
            "prompt_tokens_details": {"cached_tokens": 0},
            "completion_tokens_details": {"reasoning_tokens": REASONING_TOKENS}}


def scripted(scripts: Mapping[str, Sequence[object]]) -> Callable[[dict], object]:
    """A `respond` that plays the script whose key occurs in the first user message. A step may be a plain string
    or one of `Blank`/`Chunks`/`ErrorStatus` (module docstring)."""

    def respond(body: dict) -> object:
        messages = body.get("messages", [])
        first_user = next((str(m.get("content", "")) for m in messages if m.get("role") == "user"), "")
        matches = [key for key in scripts if key in first_user]
        if len(matches) != 1:
            raise LookupError(f"expected exactly one script key in the task message, found {matches}")
        steps = scripts[matches[0]]
        step = sum(1 for message in messages if message.get("role") == "assistant")
        return steps[min(step, len(steps) - 1)]

    return respond


def _deltas_of(reply: object) -> list[dict]:
    if isinstance(reply, Blank):
        return [{"content": ""}]
    if isinstance(reply, Chunks):
        return list(reply.deltas)
    if isinstance(reply, dict):  # already one delta (e.g. a single tool call): used as is, not wrapped as content
        return [reply]
    return [{"content": reply}]


def _merge_deltas(deltas: Sequence[dict]) -> dict:
    """What a correct streaming client reconstructs from `deltas` (module docstring, D2): every content fragment
    concatenated, and every tool call's `name` and `arguments` merged by its `index`, arguments concatenated."""
    content = "".join(str(delta["content"]) for delta in deltas if delta.get("content"))
    merged: dict = {"role": "assistant", "content": content or None}
    calls: dict[int, dict] = {}
    for delta in deltas:
        for call in delta.get("tool_calls") or []:
            index = call.get("index", 0)
            slot = calls.setdefault(index, {"id": None, "type": "function",
                                            "function": {"name": None, "arguments": ""}})
            if call.get("id"):
                slot["id"] = call["id"]
            function = call.get("function") or {}
            if function.get("name"):
                slot["function"]["name"] = function["name"]
            if function.get("arguments"):
                slot["function"]["arguments"] += str(function["arguments"])
    if calls:
        merged["tool_calls"] = [calls[index] for index in sorted(calls)]
        merged["content"] = None
    return merged


def _finish_of(reply: object, merged: dict) -> str:
    if isinstance(reply, Chunks):
        return reply.finish_reason
    return "tool_calls" if merged.get("tool_calls") else "stop"


class StubServer:
    def __init__(self, respond: Callable[[dict], object], *, model_reported: str | None = None) -> None:
        self.respond = respond
        self.model_reported = model_reported
        self.requests: list[dict] = []
        self.headers: list[dict[str, str]] = []
        self._lock = threading.Lock()
        self._server = ThreadingHTTPServer(("127.0.0.1", 0), self._handler())
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self._server.server_address[1]}/v1"

    def __enter__(self) -> StubServer:
        self._thread.start()
        return self

    def __exit__(self, *exc_info: object) -> None:
        self._server.shutdown()
        self._server.server_close()

    def _handler(self) -> type[BaseHTTPRequestHandler]:
        stub = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self) -> None:  # noqa: N802 (the stdlib's name)
                if self.path.rstrip("/") != "/v1/chat/completions":
                    self._reply(404, "application/json", json.dumps({"error": {"message":
                                                                               f"no route {self.path}"}}).encode())
                    return
                body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
                with stub._lock:
                    stub.requests.append(body)
                    stub.headers.append({name.lower(): value for name, value in self.headers.items()})
                    try:
                        reply = stub.respond(body)
                    except Exception as err:  # a broken script is a server error, never a hang
                        self._reply(500, "application/json",
                                    json.dumps({"error": {"message": f"{type(err).__name__}: {err}"}}).encode())
                        return
                if isinstance(reply, ErrorStatus):
                    kind = "authentication_error" if reply.auth else "server_error"
                    self._reply(reply.status, "application/json",
                                json.dumps({"error": {"message": kind, "type": kind}}).encode())
                    return
                deltas = _deltas_of(reply)
                merged = _merge_deltas(deltas)
                finish = _finish_of(reply, merged)
                model = stub.model_reported or body.get("model")
                usage = usage_for(body.get("messages", []), merged.get("content") or "")
                call_id = f"stub-{len(stub.requests)}"
                if body.get("stream"):
                    wants_usage = bool((body.get("stream_options") or {}).get("include_usage"))
                    self._reply(200, "text/event-stream",
                                _stream_bytes(call_id, model, deltas, finish, usage if wants_usage else None))
                else:
                    self._reply(200, "application/json", json.dumps({
                        "id": call_id, "object": "chat.completion", "model": model,
                        "choices": [{"index": 0, "finish_reason": finish, "message": merged}],
                        "usage": usage,
                    }).encode())

            def _reply(self, status: int, content_type: str, data: bytes) -> None:
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def log_message(self, *args: object) -> None:
                pass

        return Handler


def _stream_bytes(call_id: str, model: str, deltas: Sequence[dict], finish: str, usage: dict | None) -> bytes:
    """One SSE event per delta, a closing event carrying `finish_reason` (and `usage` only when asked, D6), then
    `[DONE]`."""

    def event(choices: list[dict], **extra: object) -> str:
        chunk = {"id": call_id, "object": "chat.completion.chunk", "model": model, "choices": choices, **extra}
        return f"data: {json.dumps(chunk)}\n\n"

    body = "".join(event([{"index": 0, "delta": delta, "finish_reason": None}]) for delta in deltas)
    body += event([{"index": 0, "delta": {}, "finish_reason": finish}], **({"usage": usage} if usage else {}))
    return (body + "data: [DONE]\n\n").encode()
