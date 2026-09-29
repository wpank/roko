"""A scripted fake model behind a local OpenAI-compatible server, for offline tests and dry runs of the driver.

`StubServer(respond)` serves `POST /v1/chat/completions` on 127.0.0.1, on an ephemeral port, from a daemon thread.
`respond(body)` gets the parsed request and returns the assistant's reply text. The server wraps the reply in a
chat-completions response whose `model` is the requested one (or `model_reported`) and whose usage is a fixed
function of the text (about four characters per token, `usage_for`), so costs are reproducible. `requests` keeps
every request body, in order. Nothing here touches the network: `vb run --provider-url <server.url>` is offline.

`scripted(scripts)` builds a `respond` that plays one script per task. It finds the task by a key that occurs in the
request's first user message, and returns step k of that script on the k-th call of an attempt (k = the number of
assistant messages in the request). A script that runs out repeats its last step.

API:
    StubServer(respond, *, model_reported=None)     # a context manager; .url, .requests
    scripted(scripts: Mapping[str, Sequence[str]]) -> Callable[[dict], str]
    bash(command: str, thought: str = "Next step.") -> str      # a reply holding one bash block
    usage_for(messages: Sequence[dict], reply: str) -> dict
"""

from __future__ import annotations

import json
import threading
from collections.abc import Callable, Mapping, Sequence
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REASONING_TOKENS = 8


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


def scripted(scripts: Mapping[str, Sequence[str]]) -> Callable[[dict], str]:
    """A `respond` that plays the script whose key occurs in the first user message."""

    def respond(body: dict) -> str:
        messages = body.get("messages", [])
        first_user = next((str(m.get("content", "")) for m in messages if m.get("role") == "user"), "")
        matches = [key for key in scripts if key in first_user]
        if len(matches) != 1:
            raise LookupError(f"expected exactly one script key in the task message, found {matches}")
        steps = scripts[matches[0]]
        step = sum(1 for message in messages if message.get("role") == "assistant")
        return steps[min(step, len(steps) - 1)]

    return respond


class StubServer:
    def __init__(self, respond: Callable[[dict], str], *, model_reported: str | None = None) -> None:
        self.respond = respond
        self.model_reported = model_reported
        self.requests: list[dict] = []
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
                    self._send(404, {"error": {"message": f"no route {self.path}"}})
                    return
                body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
                with stub._lock:
                    stub.requests.append(body)
                    try:
                        reply = stub.respond(body)
                    except Exception as err:  # a broken script is a server error, never a hang
                        self._send(500, {"error": {"message": f"{type(err).__name__}: {err}"}})
                        return
                self._send(200, {
                    "id": f"stub-{len(stub.requests)}", "object": "chat.completion",
                    "model": stub.model_reported or body.get("model"),
                    "choices": [{"index": 0, "finish_reason": "stop",
                                 "message": {"role": "assistant", "content": reply}}],
                    "usage": usage_for(body.get("messages", []), reply),
                })

            def _send(self, status: int, payload: dict) -> None:
                data = json.dumps(payload).encode("utf-8")
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def log_message(self, *args: object) -> None:
                pass

        return Handler
