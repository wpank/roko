"""Tests for `vb capture-errors` (error_capture.py, gap-db8dab), offline: a stub upstream answers the probe as a
provider would, and the capture goes through the metering proxy. No provider is called."""

from __future__ import annotations

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

import error_capture
import faultproxy
import layout
import secret
import vb

REQUEST_ID = "req_8f3e2b1c9d0a4e5f6a7b8c9d0e1f2a3b"
ORGANIZATION = "user-a1b2c3d4e5f6g7h8"
COOKIE = ("__cf_bm=Q2t9vWzLx3pK8mN1rT5yB7cD0fG4hJ6kL9aS2dF5gH8-1792260000.1234567-1.0.1.1-aB3dE5fG7hJ9kL1mN3pQ5r; "
          "HttpOnly; Secure; Path=/; Expires=Sat, 17 Oct 2026 19:30:00 GMT")
HEADERS = [("x-request-id", REQUEST_ID), ("openai-organization", ORGANIZATION), ("openai-version", "2020-10-01"),
           ("x-ratelimit-limit-requests", "500"), ("set-cookie", COOKIE), ("cf-ray", "8f3e2b1c9d0a4e5f-SJC"),
           ("strict-transport-security", "max-age=31536000; includeSubDomains; preload")]
BODY = json.dumps({"error": {"message": "The model `nonexistent-model` does not exist or you do not have access to it.",
                             "type": "invalid_request_error", "param": None, "code": "model_not_found",
                             "request_id": REQUEST_ID}}, indent=4) + "\n"
SECRETS = (REQUEST_ID, ORGANIZATION, "Q2t9vWzLx3pK8mN1rT5yB7cD0fG4hJ6kL9aS2dF5gH8", "8f3e2b1c9d0a4e5f")


class ErrorUpstream:
    """A provider that refuses every chat completion with a 404 in OpenAI's indented shape, with `HEADERS`.
    `requests` holds each request's headers (names in lower case) and body."""

    def __init__(self) -> None:
        self.requests: list[tuple[dict, dict]] = []
        stub = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def do_POST(self) -> None:  # noqa: N802 (the stdlib's name)
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                stub.requests.append(({k.lower(): v for k, v in self.headers.items()}, body))
                data = BODY.encode()
                self.send_response(404)
                self.send_header("Content-Type", "application/json; charset=utf-8")
                self.send_header("Content-Length", str(len(data)))
                for name, value in HEADERS:
                    self.send_header(name, value)
                self.end_headers()
                self.wfile.write(data)

            def log_message(self, *args: object) -> None:
                pass

        self._server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self._server.server_address[1]}/v1"

    def __enter__(self) -> ErrorUpstream:
        self._thread.start()
        return self

    def __exit__(self, *exc_info: object) -> None:
        self._server.shutdown()
        self._server.server_close()


def test_mask_draws_every_id_anew_in_its_form_and_follows_it_into_the_body():
    headers, body, masked = error_capture.mask(HEADERS, BODY.encode())
    values = dict(headers)
    assert masked == ["x-request-id", "openai-organization", "set-cookie", "cf-ray"]
    assert [name for name, _ in headers] == [name for name, _ in HEADERS]
    for name in ("openai-version", "x-ratelimit-limit-requests", "strict-transport-security"):
        assert values[name] == dict(HEADERS)[name]  # not an id: kept
    new_id = values["x-request-id"]
    assert new_id != REQUEST_ID and len(new_id) == len(REQUEST_ID) and new_id.startswith("req_")
    assert values["openai-organization"].startswith("user-") and values["openai-organization"] != ORGANIZATION
    assert values["set-cookie"].startswith("__cf_bm=") and values["set-cookie"].endswith(COOKIE.split(";", 1)[1])
    text = body.decode()
    assert json.loads(text)["error"]["request_id"] == new_id and text.endswith("}\n")
    assert not any(secret_ in text or secret_ in json.dumps(headers) for secret_ in SECRETS)


def test_body_format_names_the_layout():
    value = {"error": {"message": "x", "code": None}}
    assert error_capture.body_format(json.dumps(value, separators=(",", ":")))["layout"] == "compact"
    assert error_capture.body_format(json.dumps(value))["layout"] == "spaced"
    assert error_capture.body_format(json.dumps(value, indent=4) + "\n") == {"layout": "indent:4", "newline": True}
    assert error_capture.body_format("<html>no</html>")["layout"] == "other"


def capture_args(tmp_path: Path, url: str, *extra: str) -> list[str]:
    return ["capture-errors", "--provider", "openai", "--provider-url", f"openai={url}", "--out",
            str(tmp_path / "fixtures"), "--results", str(tmp_path / "results"), "--run-id", "c1", *extra]


def test_vb_capture_errors_writes_a_masked_fixture_and_books_zero(tmp_path, capsys):
    with ErrorUpstream() as stub:
        assert vb.main(capture_args(tmp_path, stub.url)) == 0
    assert len(stub.requests) == 1
    sent_headers, sent = stub.requests[0]
    assert sent == {"model": error_capture.PROBE_MODEL, "messages": error_capture.PROBE_MESSAGES,
                    "max_completion_tokens": 1}
    assert sent_headers["user-agent"] == error_capture.USER_AGENT and "authorization" not in sent_headers
    text = (tmp_path / "fixtures" / "openai.json").read_text()
    fixture = json.loads(text)
    assert (fixture["schema_version"], fixture["provider"], fixture["status"], fixture["kind"]) == (
        error_capture.SCHEMA, "openai", 404, "unknown_model")
    assert fixture["content_type"] == "application/json; charset=utf-8"
    assert fixture["format"] == {"layout": "indent:4", "newline": True}
    assert [name for name, _ in fixture["headers"]] == [name for name, _ in HEADERS]  # NOT_MIRRORED left out
    assert fixture["masked"] == sorted(["x-request-id", "openai-organization", "set-cookie", "cf-ray"])
    assert json.loads(fixture["body"])["error"]["code"] == "model_not_found"
    assert not any(secret_ in text for secret_ in SECRETS) and "429" in fixture["why"]
    run_dir = tmp_path / "results" / "ERROR-CAPTURE" / "c1"
    [row] = [json.loads(line) for line in (run_dir / "ledger.jsonl").read_text().splitlines()]
    assert (row["line"], row["api_equiv_usd"], row["billed_usd"], row["source"]) == ("BL0", 0.0, 0.0, "provider_usage")
    [call] = [json.loads(line) for line in (run_dir / "proxy.jsonl").read_text().splitlines()]
    assert (call["status"], call["forwarded"], call["cost_source"], call["task"]) == (404, True, "not_billed",
                                                                                      "c1/openai:1")
    stored = "".join(path.read_text() for path in run_dir.iterdir())
    assert not any(secret_ in stored for secret_ in SECRETS)  # the run keeps no header or body


def test_only_the_proxy_sends_the_key(tmp_path, monkeypatch):
    key = "sk-test-" + "k" * 32
    key_file = secret.create_keys(tmp_path / "config" / "keys", {"OPENAI_API_KEY": key})
    with ErrorUpstream() as stub:
        arms = tmp_path / "arms"
        arms.mkdir()
        text = (layout.ARMS_DIR / "cheap_direct_mini.toml").read_text()
        (arms / "cheap_direct_mini.toml").write_text(text.replace('"https://api.openai.com/v1"', f'"{stub.url}"'))
        monkeypatch.setattr(layout, "ARMS_DIR", arms)
        assert vb.main(["capture-errors", "--provider", "openai", "--key-file", str(key_file), "--out",
                        str(tmp_path / "fixtures"), "--results", str(tmp_path / "results"), "--run-id", "k1"]) == 0
    assert stub.requests[0][0]["authorization"] == f"Bearer {key}"
    assert key not in (tmp_path / "fixtures" / "openai.json").read_text()
    assert all(key not in path.read_text() for path in (tmp_path / "results" / "ERROR-CAPTURE" / "k1").iterdir())


def test_vb_capture_errors_admits_the_network_only_with_both_flags(tmp_path, capsys):
    results = tmp_path / "results"
    base = ["capture-errors", "--results", str(results), "--out", str(tmp_path / "fixtures")]
    for extra, words in (([], "--allow-network"), (["--allow-network"], "--max-cost-usd"),
                         (["--allow-network", "--max-cost-usd", "0.000001"], "more than --max-cost-usd")):
        assert vb.main([*base, *extra]) == 2
        assert words in capsys.readouterr().err
    assert not results.exists() and not (tmp_path / "fixtures").exists()


@pytest.mark.parametrize("name", error_capture.PROVIDERS)
def test_every_provider_has_an_arm_endpoint_with_a_key_and_its_docs(name):
    snapshot = vb.ledger.load_snapshot()
    endpoint = error_capture._endpoint(vb, snapshot, name, None)
    assert endpoint.provider == name and endpoint.api_key_env and not endpoint.offline
    assert error_capture.DOCS[name].startswith("https://")
    assert name in faultproxy.UPSTREAM_ERRORS
