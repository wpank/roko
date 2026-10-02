"""The egress proxy (gap-0bd49a, 3305): a CONNECT-only proxy on the loopback that admits an allowlist of targets.

Claude Code must reach Anthropic's API, so the loopback-only rule that holds the Roko arm cannot hold the fd_claude
arm as it is. Its process tree runs under `sandbox.command(network="loopback:<this proxy's port>")` instead, with
`HTTPS_PROXY` and `HTTP_PROXY` naming this proxy (`agent_env.proxy_env`, `run_cli`): the CLI's API calls go through
it, a shell `curl` to any other host is refused at its CONNECT, and a direct socket is denied by the sandbox. The same
proxy can serve another CLI arm (Codex) later.

- **What passes.** Only `CONNECT host:port` for a target on the allowlist (exact matches, the host compared without
  case), and then the tunnel's bytes both ways, unread: TLS stays end to end between the client and the host. Every
  other request, a CONNECT to another target or a plain-HTTP proxy request (`GET http://…`), gets a 403, and the proxy
  neither connects nor resolves anything for it.
- **The log.** Every request is one row, kept in memory and appended to `log_path` (JSONL, one write per row): `ts`
  (to the microsecond), `task` (set by `configure` before each task), `method`, `target`, `admitted` and, for an
  admitted target the proxy could not reach, `error`. `summary(task)` gives a task's allowlist, its admitted count and
  its refused requests, which the run record keeps.
- **What it cannot tell apart.** The CLI and the shell commands it runs share the proxy, so a shell `curl` to an
  allowlisted host passes as the CLI's own calls do. That reaches only the API itself, without the CLI's login.
- **Where.** 127.0.0.1 on an ephemeral port, in the driver's process, on daemon threads. A request's head must arrive
  within `HEAD_TIMEOUT_S` and fit in `HEAD_MAX_BYTES`; `close` also ends the tunnels still open.

API:
    EgressProxy(allow, *, log_path=None, connect_timeout_s=CONNECT_TIMEOUT_S)    # .start() -> self; a context manager
    EgressProxy.port; .url; .configure(*, task); .summary(task) -> dict; .rows; .close()
    parse_allow(entries) -> tuple[str, ...]              # "host:port" entries, normalised; raises EgressError
    DEFAULT_ALLOW, EgressError
"""

from __future__ import annotations

import datetime as dt
import json
import os
import re
import socket
import socketserver
import threading
import time
import urllib.parse
from pathlib import Path

DEFAULT_ALLOW = ("api.anthropic.com:443",)  # Claude Code's API host (3305's option (1))
TARGET = re.compile(r"(\[[0-9A-Fa-f:.]+\]|[A-Za-z0-9](?:[A-Za-z0-9.-]{0,251}[A-Za-z0-9])?):([0-9]{1,5})")
HEAD_MAX_BYTES = 16_384
HEAD_TIMEOUT_S = 10.0
CONNECT_TIMEOUT_S = 30.0
CHUNK = 65_536


class EgressError(ValueError):
    """An allowlist entry is not a `host:port` target."""


def parse_allow(entries: list[str] | tuple[str, ...]) -> tuple[str, ...]:
    """The allowlist's entries as `host:port`, the host in lower case. Raises EgressError for anything else."""
    if not isinstance(entries, (list, tuple)):
        raise EgressError(f"the egress allowlist is a list of host:port entries, not {type(entries).__name__}")
    found = []
    for entry in entries:
        target = _target(entry) if isinstance(entry, str) else None
        if target is None:
            raise EgressError(f"egress allowlist entry {entry!r} is not host:port with a port in 1-65535")
        found.append(target)
    return tuple(dict.fromkeys(found))


class EgressProxy:
    def __init__(self, allow: list[str] | tuple[str, ...], *, log_path: Path | None = None,
                 connect_timeout_s: float = CONNECT_TIMEOUT_S) -> None:
        self.allow = parse_allow(allow)
        self.log_path = Path(log_path) if log_path else None
        self.connect_timeout_s = connect_timeout_s
        self.rows: list[dict] = []
        self._task: str | None = None
        self._lock = threading.Lock()
        self._open: set[socket.socket] = set()  # every client and upstream socket of a request still being served
        self._server = _Server(("127.0.0.1", 0), self._handler())
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)

    @property
    def port(self) -> int:
        return self._server.server_address[1]

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    def start(self) -> EgressProxy:
        self._thread.start()
        return self

    def close(self) -> None:
        """Stop accepting, and end every tunnel still open."""
        if self._thread.is_alive():
            self._server.shutdown()
        with self._lock:
            open_sockets = list(self._open)
        for sock in open_sockets:
            _shut(sock)
        self._server.server_close()

    def __enter__(self) -> EgressProxy:
        return self.start()

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    def configure(self, *, task: str | None) -> None:
        """The task key the rows of later requests carry."""
        with self._lock:
            self._task = task

    def summary(self, task: str | None) -> dict:
        """What the run record keeps of one task's requests: the allowlist, how many were admitted, and each refused
        one."""
        with self._lock:
            rows = [row for row in self.rows if row["task"] == task]
        return {"allow": list(self.allow), "admitted": sum(1 for row in rows if row["admitted"]),
                "refused": [{key: row[key] for key in ("ts", "method", "target")} for row in rows
                            if not row["admitted"]]}

    def _handler(self) -> type[socketserver.BaseRequestHandler]:
        proxy = self

        class Handler(socketserver.BaseRequestHandler):
            def handle(self) -> None:
                proxy._serve(self.request)

        return Handler

    def _serve(self, client: socket.socket) -> None:
        with self._lock:
            self._open.add(client)
        try:
            self._answer(client)
        except OSError:
            pass
        finally:
            with self._lock:
                self._open.discard(client)

    def _answer(self, client: socket.socket) -> None:
        """Answer one request: refuse it, or connect its allowlisted target and relay until the tunnel ends."""
        head, rest = _read_head(client)
        if head is None:
            self._log("?", "", admitted=False)
            _reply(client, 400, "vb egress: no complete request head")
            return
        method, raw = _request_line(head)
        target = _target(raw) if method == "CONNECT" else None
        if target is None or target not in self.allow:
            label = target or _label(raw)
            self._log(method, label, admitted=False)
            _reply(client, 403, f"vb egress: {method} {label} is not on this run's allowlist")
            return
        host, _, port = target.rpartition(":")
        try:
            upstream = socket.create_connection((host.strip("[]"), int(port)), timeout=self.connect_timeout_s)
        except OSError as err:
            self._log(method, target, admitted=True, error=type(err).__name__)
            _reply(client, 502, f"vb egress: {target} cannot be reached")
            return
        with self._lock:
            self._open.add(upstream)
        try:
            self._log(method, target, admitted=True)
            upstream.settimeout(None)
            client.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            if rest:
                upstream.sendall(rest)
            _relay(client, upstream)
        finally:
            with self._lock:
                self._open.discard(upstream)
            upstream.close()

    def _log(self, method: str, target: str, *, admitted: bool, error: str | None = None) -> None:
        row = {"ts": dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ"), "task": None, "method": method[:16],
               "target": target[:255], "admitted": admitted}
        if error:
            row["error"] = error
        with self._lock:
            row["task"] = self._task
            self.rows.append(row)
            if self.log_path is not None:
                fd = os.open(self.log_path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
                try:
                    os.write(fd, (json.dumps(row, sort_keys=True) + "\n").encode("utf-8"))
                finally:
                    os.close(fd)


class _Server(socketserver.ThreadingTCPServer):
    daemon_threads = True
    block_on_close = False  # `close` ends the tunnels itself; a stuck one must not hold it
    allow_reuse_address = True


def _target(text: str) -> str | None:
    """`host:port` with the host in lower case, or None when `text` is not one (or the port is out of range)."""
    match = TARGET.fullmatch(text.strip())
    if not match or not 1 <= int(match[2]) <= 65535:
        return None
    return f"{match[1].lower()}:{int(match[2])}"


def _label(raw: str) -> str:
    """How the log names the target of a request that is not a CONNECT: its URL's host and port, else the target."""
    parts = urllib.parse.urlsplit(raw)
    try:
        port = parts.port or {"http": 80, "https": 443}.get(parts.scheme)
    except ValueError:
        port = None
    return f"{parts.hostname}:{port}" if parts.hostname and port else raw[:255]


def _read_head(client: socket.socket) -> tuple[str | None, bytes]:
    """The request head (up to its blank line) and any bytes after it; None when it did not arrive whole within
    HEAD_TIMEOUT_S or is longer than HEAD_MAX_BYTES."""
    deadline = time.monotonic() + HEAD_TIMEOUT_S
    data = b""
    while b"\r\n\r\n" not in data:
        left = deadline - time.monotonic()
        if len(data) > HEAD_MAX_BYTES or left <= 0:
            return None, b""
        client.settimeout(left)
        try:
            chunk = client.recv(4096)
        except OSError:  # TimeoutError included
            return None, b""
        if not chunk:
            return None, b""
        data += chunk
    head, _, rest = data.partition(b"\r\n\r\n")
    client.settimeout(None)
    return head.decode("latin-1"), rest


def _request_line(head: str) -> tuple[str, str]:
    """The method and the target of the head's request line."""
    parts = head.split("\r\n", 1)[0].split(" ")
    return (parts[0].upper(), parts[1]) if len(parts) == 3 else ("?", head.split("\r\n", 1)[0][:255])


def _reply(client: socket.socket, status: int, text: str) -> None:
    reason = {400: "Bad Request", 403: "Forbidden", 502: "Bad Gateway"}[status]
    body = (text + "\n").encode("utf-8")
    client.sendall(f"HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\n"
                   f"Content-Length: {len(body)}\r\nConnection: close\r\n\r\n".encode("ascii") + body)


def _relay(client: socket.socket, upstream: socket.socket) -> None:
    """Copy bytes both ways until both directions have ended."""
    back = threading.Thread(target=_pipe, args=(upstream, client), daemon=True)
    back.start()
    _pipe(client, upstream)
    back.join()


def _pipe(source: socket.socket, sink: socket.socket) -> None:
    try:
        while True:
            data = source.recv(CHUNK)
            if not data:
                break
            sink.sendall(data)
    except OSError:
        pass
    finally:
        try:
            sink.shutdown(socket.SHUT_WR)
        except OSError:
            pass


def _shut(sock: socket.socket) -> None:
    try:
        sock.shutdown(socket.SHUT_RDWR)
    except OSError:
        pass
